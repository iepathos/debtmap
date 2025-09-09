use super::{
    calculate_god_object_score, determine_confidence, group_methods_by_responsibility,
    GodObjectAnalysis, GodObjectConfidence, GodObjectThresholds, MaintainabilityImpact,
    OrganizationAntiPattern, OrganizationDetector, ResponsibilityGroup,
};
use crate::common::{capitalize_first, SourceLocation, UnifiedLocationExtractor};
use std::collections::HashMap;
use std::path::Path;
use syn::{self, visit::Visit};

pub struct GodObjectDetector {
    max_methods: usize,
    max_fields: usize,
    max_responsibilities: usize,
    location_extractor: Option<UnifiedLocationExtractor>,
}

impl Default for GodObjectDetector {
    fn default() -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_responsibilities: 3,
            location_extractor: None,
        }
    }
}

impl GodObjectDetector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_source_content(source_content: &str) -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_responsibilities: 3,
            location_extractor: Some(UnifiedLocationExtractor::new(source_content)),
        }
    }

    pub fn analyze_comprehensive(&self, path: &Path, ast: &syn::File) -> GodObjectAnalysis {
        let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
        visitor.visit_file(ast);

        // Get thresholds based on file extension
        let thresholds = if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            GodObjectThresholds::for_rust()
        } else if path.extension().and_then(|s| s.to_str()) == Some("py") {
            GodObjectThresholds::for_python()
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s == "js" || s == "ts")
            .unwrap_or(false)
        {
            GodObjectThresholds::for_javascript()
        } else {
            GodObjectThresholds::default()
        };

        // Aggregate metrics from all types in the file
        let mut total_methods = 0;
        let mut total_fields = 0;
        let mut all_methods = Vec::new();
        let mut total_complexity = 0u32;

        // Count methods from types (structs/impl blocks)
        for type_info in visitor.types.values() {
            total_methods += type_info.method_count;
            total_fields += type_info.field_count;
            all_methods.extend(type_info.methods.clone());
            // Estimate complexity (rough approximation)
            total_complexity += (type_info.method_count * 5) as u32;
        }

        // Also count standalone functions in the file
        let standalone_functions = visitor.standalone_functions.clone();
        total_methods += standalone_functions.len();
        all_methods.extend(standalone_functions.clone());
        total_complexity += (standalone_functions.len() * 5) as u32;

        // Count lines (simple approximation based on AST)
        let lines_of_code = ast.items.len() * 50; // Very rough estimate

        let responsibility_groups = group_methods_by_responsibility(&all_methods);
        let responsibility_count = responsibility_groups.len();

        let god_object_score = calculate_god_object_score(
            total_methods,
            total_fields,
            responsibility_count,
            lines_of_code,
            &thresholds,
        );

        let confidence = determine_confidence(
            total_methods,
            total_fields,
            responsibility_count,
            lines_of_code,
            total_complexity,
            &thresholds,
        );

        let is_god_object = confidence != GodObjectConfidence::NotGodObject;

        let recommended_splits = if is_god_object {
            let file_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module");
            crate::organization::recommend_module_splits(
                file_name,
                &all_methods,
                &responsibility_groups,
            )
        } else {
            Vec::new()
        };

        let responsibilities: Vec<String> = responsibility_groups.keys().cloned().collect();

        GodObjectAnalysis {
            is_god_object,
            method_count: total_methods,
            field_count: total_fields,
            responsibility_count,
            lines_of_code,
            complexity_sum: total_complexity,
            god_object_score,
            recommended_splits,
            confidence,
            responsibilities,
        }
    }

    #[allow(dead_code)]
    fn analyze_type(&self, item_struct: &syn::ItemStruct) -> TypeAnalysis {
        let location = if let Some(ref extractor) = self.location_extractor {
            extractor.extract_item_location(&syn::Item::Struct(item_struct.clone()))
        } else {
            SourceLocation::default()
        };

        TypeAnalysis {
            name: item_struct.ident.to_string(),
            method_count: 0,
            field_count: self.count_fields(&item_struct.fields),
            methods: Vec::new(),
            fields: self.extract_field_names(&item_struct.fields),
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location,
        }
    }

    #[allow(dead_code)]
    fn count_fields(&self, fields: &syn::Fields) -> usize {
        match fields {
            syn::Fields::Named(fields) => fields.named.len(),
            syn::Fields::Unnamed(fields) => fields.unnamed.len(),
            syn::Fields::Unit => 0,
        }
    }

    #[allow(dead_code)]
    fn extract_field_names(&self, fields: &syn::Fields) -> Vec<String> {
        match fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref().map(|id| id.to_string()))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Classify the maintainability impact based on method and field counts
    fn classify_god_object_impact(
        method_count: usize,
        field_count: usize,
    ) -> MaintainabilityImpact {
        match () {
            _ if method_count > 30 || field_count > 20 => MaintainabilityImpact::Critical,
            _ if method_count > 20 || field_count > 15 => MaintainabilityImpact::High,
            _ => MaintainabilityImpact::Medium,
        }
    }

    fn is_god_object(&self, analysis: &TypeAnalysis) -> bool {
        analysis.method_count > self.max_methods
            || analysis.field_count > self.max_fields
            || analysis.responsibilities.len() > self.max_responsibilities
            || analysis.trait_implementations > 10
    }

    fn suggest_responsibility_split(&self, analysis: &TypeAnalysis) -> Vec<ResponsibilityGroup> {
        let method_groups = self.group_methods_by_prefix(&analysis.methods);

        let groups: Vec<ResponsibilityGroup> = method_groups
            .into_iter()
            .map(|(prefix, methods)| self.create_responsibility_group(prefix, methods))
            .collect();

        // Return existing groups or create default if empty and exceeds threshold
        if groups.is_empty() && analysis.method_count > self.max_methods {
            vec![self.create_default_responsibility_group(analysis)]
        } else {
            groups
        }
    }

    /// Create a responsibility group from prefix and methods
    fn create_responsibility_group(
        &self,
        prefix: String,
        methods: Vec<String>,
    ) -> ResponsibilityGroup {
        let responsibility = self.infer_responsibility_name(&prefix);
        ResponsibilityGroup {
            name: format!("{}Manager", responsibility.replace(' ', "")),
            methods,
            fields: Vec::new(),
            responsibility,
        }
    }

    /// Create a default responsibility group for core functionality
    fn create_default_responsibility_group(&self, analysis: &TypeAnalysis) -> ResponsibilityGroup {
        ResponsibilityGroup {
            name: format!("{}Core", analysis.name),
            methods: analysis.methods.clone(),
            fields: analysis.fields.clone(),
            responsibility: "Core functionality".to_string(),
        }
    }

    fn group_methods_by_prefix(&self, methods: &[String]) -> HashMap<String, Vec<String>> {
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();

        for method in methods {
            let prefix = self.extract_method_prefix(method);
            groups.entry(prefix).or_default().push(method.clone());
        }

        groups
    }

    fn extract_method_prefix(&self, method_name: &str) -> String {
        Self::find_matching_prefix(method_name)
            .unwrap_or_else(|| Self::extract_first_word(method_name))
    }

    /// Pure function to find a matching prefix from the common list
    fn find_matching_prefix(method_name: &str) -> Option<String> {
        const COMMON_PREFIXES: &[&str] = &[
            "get",
            "set",
            "is",
            "has",
            "can",
            "should",
            "will",
            "create",
            "build",
            "make",
            "new",
            "init",
            "calculate",
            "compute",
            "process",
            "transform",
            "validate",
            "check",
            "verify",
            "ensure",
            "save",
            "load",
            "store",
            "retrieve",
            "fetch",
            "update",
            "modify",
            "change",
            "edit",
            "delete",
            "remove",
            "clear",
            "reset",
            "send",
            "receive",
            "handle",
            "manage",
        ];

        let lower_name = method_name.to_lowercase();
        COMMON_PREFIXES
            .iter()
            .find(|&&prefix| lower_name.starts_with(prefix))
            .map(|&s| s.to_string())
    }

    /// Pure function to extract the first word from a method name
    fn extract_first_word(method_name: &str) -> String {
        method_name
            .split('_')
            .next()
            .unwrap_or(method_name)
            .to_string()
    }

    fn infer_responsibility_name(&self, prefix: &str) -> String {
        Self::classify_responsibility(prefix)
    }

    /// Pure function to classify responsibility based on method prefix
    fn classify_responsibility(prefix: &str) -> String {
        match prefix {
            "get" | "set" => "Data Access".to_string(),
            "calculate" | "compute" => "Computation".to_string(),
            "validate" | "check" | "verify" | "ensure" => "Validation".to_string(),
            "save" | "load" | "store" | "retrieve" | "fetch" => "Persistence".to_string(),
            "create" | "build" | "new" | "make" | "init" => "Construction".to_string(),
            "send" | "receive" | "handle" | "manage" => "Communication".to_string(),
            "update" | "modify" | "change" | "edit" => "Modification".to_string(),
            "delete" | "remove" | "clear" | "reset" => "Deletion".to_string(),
            "is" | "has" | "can" | "should" | "will" => "State Query".to_string(),
            "process" | "transform" => "Processing".to_string(),
            _ => format!("{} Operations", capitalize_first(prefix)),
        }
    }
}

impl OrganizationDetector for GodObjectDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
        visitor.visit_file(file);

        // Analyze each struct found
        for (_type_name, type_info) in visitor.types {
            if self.is_god_object(&type_info) {
                let suggested_split = self.suggest_responsibility_split(&type_info);

                patterns.push(OrganizationAntiPattern::GodObject {
                    type_name: type_info.name.clone(),
                    method_count: type_info.method_count,
                    field_count: type_info.field_count,
                    responsibility_count: suggested_split.len(),
                    suggested_split,
                    location: type_info.location,
                });
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "GodObjectDetector"
    }

    fn estimate_maintainability_impact(
        &self,
        pattern: &OrganizationAntiPattern,
    ) -> MaintainabilityImpact {
        match pattern {
            OrganizationAntiPattern::GodObject {
                method_count,
                field_count,
                ..
            } => GodObjectDetector::classify_god_object_impact(*method_count, *field_count),
            _ => MaintainabilityImpact::Low,
        }
    }
}

struct TypeAnalysis {
    name: String,
    method_count: usize,
    field_count: usize,
    methods: Vec<String>,
    fields: Vec<String>,
    responsibilities: Vec<Responsibility>,
    trait_implementations: usize,
    location: SourceLocation,
}

struct Responsibility {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    methods: Vec<String>,
    #[allow(dead_code)]
    fields: Vec<String>,
    #[allow(dead_code)]
    cohesion_score: f64,
}

struct TypeVisitor {
    types: HashMap<String, TypeAnalysis>,
    standalone_functions: Vec<String>,
    location_extractor: Option<UnifiedLocationExtractor>,
}

impl TypeVisitor {
    fn with_location_extractor(location_extractor: Option<UnifiedLocationExtractor>) -> Self {
        Self {
            types: HashMap::new(),
            standalone_functions: Vec::new(),
            location_extractor,
        }
    }
}

impl TypeVisitor {
    fn extract_type_name(self_ty: &syn::Type) -> Option<String> {
        match self_ty {
            syn::Type::Path(type_path) => type_path.path.get_ident().map(|id| id.to_string()),
            _ => None,
        }
    }

    fn count_impl_methods(items: &[syn::ImplItem]) -> (Vec<String>, usize) {
        let mut methods = Vec::new();
        let mut count = 0;

        for item in items {
            if let syn::ImplItem::Fn(method) = item {
                methods.push(method.sig.ident.to_string());
                count += 1;
            }
        }

        (methods, count)
    }

    fn update_type_info(&mut self, type_name: &str, node: &syn::ItemImpl) {
        if let Some(type_info) = self.types.get_mut(type_name) {
            let (methods, count) = Self::count_impl_methods(&node.items);

            type_info.methods.extend(methods);
            type_info.method_count += count;

            if node.trait_.is_some() {
                type_info.trait_implementations += 1;
            }
        }
    }
}

impl<'ast> Visit<'ast> for TypeVisitor {
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        let type_name = node.ident.to_string();
        let field_count = match &node.fields {
            syn::Fields::Named(fields) => fields.named.len(),
            syn::Fields::Unnamed(fields) => fields.unnamed.len(),
            syn::Fields::Unit => 0,
        };

        let fields = match &node.fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref().map(|id| id.to_string()))
                .collect(),
            _ => Vec::new(),
        };

        let location = if let Some(ref extractor) = self.location_extractor {
            extractor.extract_item_location(&syn::Item::Struct(node.clone()))
        } else {
            SourceLocation::default()
        };

        self.types.insert(
            type_name.clone(),
            TypeAnalysis {
                name: type_name,
                method_count: 0,
                field_count,
                methods: Vec::new(),
                fields,
                responsibilities: Vec::new(),
                trait_implementations: 0,
                location,
            },
        );
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if let Some(type_name) = Self::extract_type_name(&node.self_ty) {
            self.update_type_info(&type_name, node);
        }
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        // Track standalone functions
        self.standalone_functions.push(node.sig.ident.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, ItemImpl};

    #[test]
    fn test_find_matching_prefix_with_get() {
        assert_eq!(
            GodObjectDetector::find_matching_prefix("get_value"),
            Some("get".to_string())
        );
    }

    #[test]
    fn test_find_matching_prefix_with_set() {
        assert_eq!(
            GodObjectDetector::find_matching_prefix("setValue"),
            Some("set".to_string())
        );
    }

    #[test]
    fn test_find_matching_prefix_with_validate() {
        assert_eq!(
            GodObjectDetector::find_matching_prefix("validate_input"),
            Some("validate".to_string())
        );
    }

    #[test]
    fn test_find_matching_prefix_case_insensitive() {
        assert_eq!(
            GodObjectDetector::find_matching_prefix("CREATE_INSTANCE"),
            Some("create".to_string())
        );
    }

    #[test]
    fn test_find_matching_prefix_no_match() {
        assert_eq!(GodObjectDetector::find_matching_prefix("foo_bar"), None);
    }

    #[test]
    fn test_extract_first_word_with_underscore() {
        assert_eq!(
            GodObjectDetector::extract_first_word("custom_method_name"),
            "custom".to_string()
        );
    }

    #[test]
    fn test_extract_first_word_no_underscore() {
        assert_eq!(
            GodObjectDetector::extract_first_word("singleword"),
            "singleword".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_data_access() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("get"),
            "Data Access".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("set"),
            "Data Access".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_computation() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("calculate"),
            "Computation".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("compute"),
            "Computation".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_validation() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("validate"),
            "Validation".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("check"),
            "Validation".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("verify"),
            "Validation".to_string()
        );
    }

    #[test]
    fn test_classify_god_object_impact_critical() {
        // Critical: method_count > 30 or field_count > 20
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(31, 10),
            MaintainabilityImpact::Critical
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(15, 21),
            MaintainabilityImpact::Critical
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(35, 25),
            MaintainabilityImpact::Critical
        );
    }

    #[test]
    fn test_classify_god_object_impact_high() {
        // High: method_count > 20 or field_count > 15 (but not critical)
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(21, 10),
            MaintainabilityImpact::High
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(15, 16),
            MaintainabilityImpact::High
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(25, 14),
            MaintainabilityImpact::High
        );
    }

    #[test]
    fn test_classify_god_object_impact_medium() {
        // Medium: everything else
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(10, 10),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(20, 15),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(5, 5),
            MaintainabilityImpact::Medium
        );
    }

    #[test]
    fn test_classify_god_object_impact_boundary_conditions() {
        // Test exact boundary values
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(30, 20),
            MaintainabilityImpact::High
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(31, 20),
            MaintainabilityImpact::Critical
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(30, 21),
            MaintainabilityImpact::Critical
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(20, 15),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(21, 15),
            MaintainabilityImpact::High
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(20, 16),
            MaintainabilityImpact::High
        );
    }

    #[test]
    fn test_classify_responsibility_persistence() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("save"),
            "Persistence".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("load"),
            "Persistence".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("fetch"),
            "Persistence".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_construction() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("create"),
            "Construction".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("build"),
            "Construction".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("new"),
            "Construction".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_communication() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("send"),
            "Communication".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("receive"),
            "Communication".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("handle"),
            "Communication".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_modification() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("update"),
            "Modification".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("modify"),
            "Modification".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("change"),
            "Modification".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_deletion() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("delete"),
            "Deletion".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("remove"),
            "Deletion".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("clear"),
            "Deletion".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_state_query() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("is"),
            "State Query".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("has"),
            "State Query".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("can"),
            "State Query".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_processing() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("process"),
            "Processing".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("transform"),
            "Processing".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_default() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("custom"),
            "Custom Operations".to_string()
        );
    }

    #[test]
    fn test_extract_type_name_with_path_type() {
        let self_ty: syn::Type = parse_quote!(MyStruct);
        let result = TypeVisitor::extract_type_name(&self_ty);
        assert_eq!(result, Some("MyStruct".to_string()));
    }

    #[test]
    fn test_extract_type_name_with_complex_path() {
        let self_ty: syn::Type = parse_quote!(std::collections::HashMap);
        let result = TypeVisitor::extract_type_name(&self_ty);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_type_name_with_reference_type() {
        let self_ty: syn::Type = parse_quote!(&MyStruct);
        let result = TypeVisitor::extract_type_name(&self_ty);
        assert_eq!(result, None);
    }

    #[test]
    fn test_count_impl_methods_empty() {
        let items = vec![];
        let (methods, count) = TypeVisitor::count_impl_methods(&items);
        assert_eq!(methods.len(), 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_impl_methods_with_functions() {
        let impl_block: ItemImpl = parse_quote! {
            impl MyStruct {
                fn method1(&self) {}
                fn method2(&mut self) {}
                const CONSTANT: i32 = 42;
                fn method3() {}
            }
        };

        let (methods, count) = TypeVisitor::count_impl_methods(&impl_block.items);
        assert_eq!(count, 3);
        assert_eq!(methods.len(), 3);
        assert!(methods.contains(&"method1".to_string()));
        assert!(methods.contains(&"method2".to_string()));
        assert!(methods.contains(&"method3".to_string()));
    }

    #[test]
    fn test_count_impl_methods_mixed_items() {
        let impl_block: ItemImpl = parse_quote! {
            impl MyStruct {
                type Item = i32;
                fn method1(&self) {}
                const VALUE: i32 = 10;
                fn method2(&self) {}
            }
        };

        let (methods, count) = TypeVisitor::count_impl_methods(&impl_block.items);
        assert_eq!(count, 2);
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&"method1".to_string()));
        assert!(methods.contains(&"method2".to_string()));
    }

    #[test]
    fn test_update_type_info_with_methods() {
        let mut visitor = TypeVisitor::with_location_extractor(None);

        visitor.types.insert(
            "TestStruct".to_string(),
            TypeAnalysis {
                name: "TestStruct".to_string(),
                method_count: 0,
                field_count: 0,
                methods: Vec::new(),
                fields: Vec::new(),
                responsibilities: Vec::new(),
                trait_implementations: 0,
                location: SourceLocation::default(),
            },
        );

        let impl_block: ItemImpl = parse_quote! {
            impl TestStruct {
                fn method1(&self) {}
                fn method2(&self) {}
            }
        };

        visitor.update_type_info("TestStruct", &impl_block);

        let type_info = visitor.types.get("TestStruct").unwrap();
        assert_eq!(type_info.method_count, 2);
        assert_eq!(type_info.methods.len(), 2);
        assert!(type_info.methods.contains(&"method1".to_string()));
        assert!(type_info.methods.contains(&"method2".to_string()));
    }

    #[test]
    fn test_update_type_info_with_trait_impl() {
        let mut visitor = TypeVisitor::with_location_extractor(None);

        visitor.types.insert(
            "TestStruct".to_string(),
            TypeAnalysis {
                name: "TestStruct".to_string(),
                method_count: 0,
                field_count: 0,
                methods: Vec::new(),
                fields: Vec::new(),
                responsibilities: Vec::new(),
                trait_implementations: 0,
                location: SourceLocation::default(),
            },
        );

        let impl_block: ItemImpl = parse_quote! {
            impl Display for TestStruct {
                fn fmt(&self, f: &mut Formatter) -> Result {
                    Ok(())
                }
            }
        };

        visitor.update_type_info("TestStruct", &impl_block);

        let type_info = visitor.types.get("TestStruct").unwrap();
        assert_eq!(type_info.trait_implementations, 1);
        assert_eq!(type_info.method_count, 1);
        assert!(type_info.methods.contains(&"fmt".to_string()));
    }

    #[test]
    fn test_update_type_info_nonexistent_type() {
        let mut visitor = TypeVisitor::with_location_extractor(None);

        let impl_block: ItemImpl = parse_quote! {
            impl NonExistent {
                fn method(&self) {}
            }
        };

        visitor.update_type_info("NonExistent", &impl_block);

        assert!(!visitor.types.contains_key("NonExistent"));
    }

    #[test]
    fn test_visit_item_impl_integration() {
        use syn::visit::Visit;

        let mut visitor = TypeVisitor::with_location_extractor(None);

        visitor.types.insert(
            "MyStruct".to_string(),
            TypeAnalysis {
                name: "MyStruct".to_string(),
                method_count: 0,
                field_count: 0,
                methods: Vec::new(),
                fields: Vec::new(),
                responsibilities: Vec::new(),
                trait_implementations: 0,
                location: SourceLocation::default(),
            },
        );

        let impl_block: ItemImpl = parse_quote! {
            impl MyStruct {
                fn new() -> Self { MyStruct }
                fn process(&self) {}
            }
        };

        visitor.visit_item_impl(&impl_block);

        let type_info = visitor.types.get("MyStruct").unwrap();
        assert_eq!(type_info.method_count, 2);
        assert_eq!(type_info.methods.len(), 2);
    }

    #[test]
    fn test_create_responsibility_group() {
        let detector = GodObjectDetector::new();
        let methods = vec!["get_value".to_string(), "get_name".to_string()];

        let group = detector.create_responsibility_group("get".to_string(), methods.clone());

        assert_eq!(group.name, "DataAccessManager");
        assert_eq!(group.responsibility, "Data Access");
        assert_eq!(group.methods, methods);
        assert!(group.fields.is_empty());
    }

    #[test]
    fn test_create_responsibility_group_with_spaces() {
        let detector = GodObjectDetector::new();
        let methods = vec!["validate_input".to_string()];

        let group = detector.create_responsibility_group("validate".to_string(), methods.clone());

        assert_eq!(group.name, "ValidationManager");
        assert_eq!(group.responsibility, "Validation");
        assert_eq!(group.methods, methods);
    }

    #[test]
    fn test_create_default_responsibility_group() {
        let detector = GodObjectDetector::new();
        let analysis = TypeAnalysis {
            name: "TestClass".to_string(),
            method_count: 5,
            field_count: 3,
            methods: vec!["method1".to_string(), "method2".to_string()],
            fields: vec!["field1".to_string(), "field2".to_string()],
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location: SourceLocation::default(),
        };

        let group = detector.create_default_responsibility_group(&analysis);

        assert_eq!(group.name, "TestClassCore");
        assert_eq!(group.responsibility, "Core functionality");
        assert_eq!(group.methods, analysis.methods);
        assert_eq!(group.fields, analysis.fields);
    }

    #[test]
    fn test_suggest_responsibility_split_with_method_groups() {
        let detector = GodObjectDetector::new();
        let analysis = TypeAnalysis {
            name: "TestClass".to_string(),
            method_count: 8,
            field_count: 5,
            methods: vec![
                "get_value".to_string(),
                "get_name".to_string(),
                "set_value".to_string(),
                "validate_input".to_string(),
                "validate_output".to_string(),
                "save_data".to_string(),
            ],
            fields: Vec::new(),
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location: SourceLocation::default(),
        };

        let groups = detector.suggest_responsibility_split(&analysis);

        assert_eq!(groups.len(), 4); // get, set, validate, save

        // Verify that groups are properly created
        let group_names: Vec<String> = groups.iter().map(|g| g.name.clone()).collect();
        assert!(group_names.contains(&"DataAccessManager".to_string()));
        assert!(group_names.contains(&"ValidationManager".to_string()));
        assert!(group_names.contains(&"PersistenceManager".to_string()));
    }

    #[test]
    fn test_suggest_responsibility_split_with_no_groups_below_threshold() {
        let detector = GodObjectDetector::new();
        let analysis = TypeAnalysis {
            name: "SmallClass".to_string(),
            method_count: 10, // Below max_methods (15)
            field_count: 5,
            methods: vec!["custom_method".to_string()],
            fields: Vec::new(),
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location: SourceLocation::default(),
        };

        let groups = detector.suggest_responsibility_split(&analysis);

        // Should return the grouped method even if below threshold
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "CustomOperationsManager");
    }

    #[test]
    fn test_suggest_responsibility_split_with_no_groups_above_threshold() {
        let detector = GodObjectDetector::new();
        let analysis = TypeAnalysis {
            name: "LargeClass".to_string(),
            method_count: 20, // Above max_methods (15)
            field_count: 5,
            methods: vec!["custom_method".to_string()],
            fields: vec!["field1".to_string()],
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location: SourceLocation::default(),
        };

        let groups = detector.suggest_responsibility_split(&analysis);

        // Should still group by prefix even above threshold
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "CustomOperationsManager");
    }

    #[test]
    fn test_suggest_responsibility_split_empty_methods_above_threshold() {
        let detector = GodObjectDetector::new();
        let analysis = TypeAnalysis {
            name: "EmptyClass".to_string(),
            method_count: 20, // Above max_methods (15)
            field_count: 5,
            methods: Vec::new(), // No methods to group
            fields: vec!["field1".to_string()],
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location: SourceLocation::default(),
        };

        let groups = detector.suggest_responsibility_split(&analysis);

        // Should create default group when no methods but above threshold
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "EmptyClassCore");
        assert_eq!(groups[0].responsibility, "Core functionality");
    }
}
