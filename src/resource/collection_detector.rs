use super::{
    BoundingStrategy, GrowthPattern, ResourceDetector, ResourceImpact, ResourceManagementIssue,
    SourceLocation,
};
use std::path::Path;
use syn::{visit::Visit, Expr, ExprMethodCall, Fields, ItemImpl, ItemStruct, Type};

pub struct UnboundedCollectionDetector {
    growth_analyzer: CollectionGrowthAnalyzer,
}

impl Default for UnboundedCollectionDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl UnboundedCollectionDetector {
    pub fn new() -> Self {
        Self {
            growth_analyzer: CollectionGrowthAnalyzer::new(),
        }
    }

    fn analyze_collection_usage(&self, visitor: &CollectionVisitor) -> Vec<CollectionUsage> {
        let mut usages = Vec::new();

        for field in &visitor.collection_fields {
            let growth_analysis = self.growth_analyzer.analyze_growth_pattern(
                field,
                &visitor.insertions,
                &visitor.removals,
            );

            if growth_analysis.has_unbounded_growth {
                usages.push(CollectionUsage {
                    name: field.name.clone(),
                    collection_type: field.type_name.clone(),
                    is_unbounded: true,
                    growth_pattern: growth_analysis.pattern,
                    insert_sites: growth_analysis.insert_sites,
                    remove_sites: growth_analysis.remove_sites,
                });
            }
        }

        usages
    }

    fn suggest_bounding_strategy(&self, usage: &CollectionUsage) -> BoundingStrategy {
        match usage.growth_pattern {
            GrowthPattern::UnboundedInsertion => {
                if usage.collection_type.contains("Cache") {
                    BoundingStrategy::LruEviction
                } else {
                    BoundingStrategy::SizeLimit
                }
            }
            GrowthPattern::NoEviction => BoundingStrategy::TimeBasedEviction,
            GrowthPattern::MemoryAccumulation => BoundingStrategy::CapacityCheck,
            GrowthPattern::RecursiveGrowth => BoundingStrategy::SizeLimit,
        }
    }
}

impl ResourceDetector for UnboundedCollectionDetector {
    fn detect_issues(&self, file: &syn::File, _path: &Path) -> Vec<ResourceManagementIssue> {
        let mut visitor = CollectionVisitor::new();
        visitor.visit_file(file);

        let collection_usage = self.analyze_collection_usage(&visitor);

        let mut issues = Vec::new();
        for usage in collection_usage {
            if usage.is_unbounded {
                let bounding_strategy = self.suggest_bounding_strategy(&usage);
                issues.push(ResourceManagementIssue::UnboundedCollection {
                    collection_name: usage.name,
                    collection_type: usage.collection_type,
                    growth_pattern: usage.growth_pattern,
                    bounding_strategy,
                });
            }
        }

        issues
    }

    fn detector_name(&self) -> &'static str {
        "UnboundedCollectionDetector"
    }

    fn assess_resource_impact(&self, issue: &ResourceManagementIssue) -> ResourceImpact {
        match issue {
            ResourceManagementIssue::UnboundedCollection { growth_pattern, .. } => {
                match growth_pattern {
                    GrowthPattern::RecursiveGrowth => ResourceImpact::Critical,
                    GrowthPattern::UnboundedInsertion => ResourceImpact::High,
                    GrowthPattern::NoEviction => ResourceImpact::High,
                    GrowthPattern::MemoryAccumulation => ResourceImpact::Medium,
                }
            }
            _ => ResourceImpact::Medium,
        }
    }
}

struct CollectionVisitor {
    collection_fields: Vec<FieldDefinition>,
    insertions: Vec<MethodCall>,
    removals: Vec<MethodCall>,
    current_struct: Option<String>,
}

impl CollectionVisitor {
    fn new() -> Self {
        Self {
            collection_fields: Vec::new(),
            insertions: Vec::new(),
            removals: Vec::new(),
            current_struct: None,
        }
    }

    fn is_collection_type(&self, type_name: &str) -> bool {
        COLLECTION_TYPES.iter().any(|ct| type_name.contains(ct))
    }
}

impl<'ast> Visit<'ast> for CollectionVisitor {
    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        self.current_struct = Some(node.ident.to_string());

        match &node.fields {
            Fields::Named(named) => {
                for field in &named.named {
                    if let Some(ident) = &field.ident {
                        let type_name = extract_type_name(&field.ty);
                        if self.is_collection_type(&type_name) {
                            self.collection_fields.push(FieldDefinition {
                                name: ident.to_string(),
                                type_name,
                                struct_name: self.current_struct.clone(),
                            });
                        }
                    }
                }
            }
            Fields::Unnamed(unnamed) => {
                for (idx, field) in unnamed.unnamed.iter().enumerate() {
                    let type_name = extract_type_name(&field.ty);
                    if self.is_collection_type(&type_name) {
                        self.collection_fields.push(FieldDefinition {
                            name: format!("{}", idx),
                            type_name,
                            struct_name: self.current_struct.clone(),
                        });
                    }
                }
            }
            _ => {}
        }

        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        if let Type::Path(type_path) = &*node.self_ty {
            if let Some(segment) = type_path.path.segments.last() {
                self.current_struct = Some(segment.ident.to_string());
            }
        }

        syn::visit::visit_item_impl(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();

        // Track insertions
        if INSERTION_METHODS.contains(&method_name.as_str()) {
            self.insertions.push(MethodCall {
                method_name: method_name.clone(),
                receiver: extract_receiver(&node.receiver),
                line: 0, // Would need span info for accurate line numbers
            });
        }

        // Track removals
        if REMOVAL_METHODS.contains(&method_name.as_str()) {
            self.removals.push(MethodCall {
                method_name,
                receiver: extract_receiver(&node.receiver),
                line: 0,
            });
        }

        syn::visit::visit_expr_method_call(self, node);
    }
}

pub struct CollectionGrowthAnalyzer;

impl CollectionGrowthAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn analyze_growth_pattern(
        &self,
        field: &FieldDefinition,
        insertions: &[MethodCall],
        removals: &[MethodCall],
    ) -> GrowthAnalysis {
        let mut analysis = GrowthAnalysis::default();

        // Find insertions/removals for this specific field
        let field_insertions: Vec<_> = insertions
            .iter()
            .filter(|i| self.targets_field(&i.receiver, &field.name))
            .collect();

        let field_removals: Vec<_> = removals
            .iter()
            .filter(|r| self.targets_field(&r.receiver, &field.name))
            .collect();

        analysis.insert_sites = field_insertions
            .iter()
            .map(|i| SourceLocation {
                file: String::new(),
                line: i.line,
                column: 0,
            })
            .collect();

        analysis.remove_sites = field_removals
            .iter()
            .map(|r| SourceLocation {
                file: String::new(),
                line: r.line,
                column: 0,
            })
            .collect();

        // Determine if growth is bounded
        if !field_insertions.is_empty() && field_removals.is_empty() {
            analysis.has_unbounded_growth = true;
            analysis.pattern = GrowthPattern::NoEviction;
        } else if field_insertions.len() > field_removals.len() * 2 {
            // Significantly more insertions than removals
            analysis.has_unbounded_growth = true;
            analysis.pattern = GrowthPattern::UnboundedInsertion;
        } else if self.has_recursive_pattern(field) {
            analysis.has_unbounded_growth = true;
            analysis.pattern = GrowthPattern::RecursiveGrowth;
        } else if self.accumulates_without_bounds(&field_insertions, &field_removals) {
            analysis.has_unbounded_growth = true;
            analysis.pattern = GrowthPattern::MemoryAccumulation;
        }

        analysis
    }

    fn targets_field(&self, receiver: &str, field_name: &str) -> bool {
        receiver.contains(field_name) || receiver.contains("self") && receiver.contains(field_name)
    }

    fn has_recursive_pattern(&self, field: &FieldDefinition) -> bool {
        // Check if the collection type contains itself (recursive data structure)
        field.type_name.contains("Vec")
            && field
                .struct_name
                .as_ref()
                .is_some_and(|struct_name| field.type_name.contains(struct_name))
    }

    fn accumulates_without_bounds(
        &self,
        insertions: &[&MethodCall],
        removals: &[&MethodCall],
    ) -> bool {
        // Check if there's a pattern of accumulation without proper cleanup
        !insertions.is_empty() && removals.is_empty()
    }
}

fn extract_type_name(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => {
            type_path
                .path
                .segments
                .iter()
                .map(|s| {
                    let base = s.ident.to_string();
                    // Include generic parameters
                    if let syn::PathArguments::AngleBracketed(args) = &s.arguments {
                        let generics = args
                            .args
                            .iter()
                            .filter_map(|arg| {
                                if let syn::GenericArgument::Type(ty) = arg {
                                    Some(extract_type_name(ty))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        if !generics.is_empty() {
                            format!("{}<{}>", base, generics)
                        } else {
                            base
                        }
                    } else {
                        base
                    }
                })
                .collect::<Vec<_>>()
                .join("::")
        }
        Type::Reference(reference) => extract_type_name(&reference.elem),
        _ => "Unknown".to_string(),
    }
}

fn extract_receiver(expr: &Expr) -> String {
    match expr {
        Expr::Path(path) => path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        Expr::Field(field) => {
            format!(
                "{}.{}",
                extract_receiver(&field.base),
                match &field.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(index) => index.index.to_string(),
                }
            )
        }
        _ => "unknown".to_string(),
    }
}

#[derive(Debug, Clone)]
struct FieldDefinition {
    name: String,
    type_name: String,
    struct_name: Option<String>,
}

#[derive(Debug)]
struct CollectionUsage {
    name: String,
    collection_type: String,
    is_unbounded: bool,
    growth_pattern: GrowthPattern,
    #[allow(dead_code)]
    insert_sites: Vec<SourceLocation>,
    #[allow(dead_code)]
    remove_sites: Vec<SourceLocation>,
}

#[derive(Debug, Clone)]
struct MethodCall {
    #[allow(dead_code)]
    method_name: String,
    receiver: String,
    line: usize,
}

#[derive(Debug, Default)]
struct GrowthAnalysis {
    has_unbounded_growth: bool,
    pattern: GrowthPattern,
    insert_sites: Vec<SourceLocation>,
    remove_sites: Vec<SourceLocation>,
}

impl Default for GrowthPattern {
    fn default() -> Self {
        GrowthPattern::NoEviction
    }
}

const COLLECTION_TYPES: &[&str] = &[
    "Vec",
    "HashMap",
    "BTreeMap",
    "HashSet",
    "BTreeSet",
    "VecDeque",
    "LinkedList",
    "BinaryHeap",
    "Cache",
    "Buffer",
    "Queue",
    "Stack",
];

const INSERTION_METHODS: &[&str] = &[
    "push",
    "insert",
    "add",
    "put",
    "append",
    "extend",
    "push_back",
    "push_front",
];

const REMOVAL_METHODS: &[&str] = &[
    "pop",
    "remove",
    "clear",
    "drain",
    "take",
    "pop_back",
    "pop_front",
    "retain",
];
