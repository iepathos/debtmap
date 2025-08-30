use super::{
    MaintainabilityImpact, OrganizationAntiPattern, OrganizationDetector, ResponsibilityGroup,
};
use crate::common::{capitalize_first, SourceLocation, UnifiedLocationExtractor};
use std::collections::HashMap;
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

    fn is_god_object(&self, analysis: &TypeAnalysis) -> bool {
        analysis.method_count > self.max_methods
            || analysis.field_count > self.max_fields
            || analysis.responsibilities.len() > self.max_responsibilities
            || analysis.trait_implementations > 10
    }

    fn suggest_responsibility_split(&self, analysis: &TypeAnalysis) -> Vec<ResponsibilityGroup> {
        let mut groups = Vec::new();

        // Group methods by prefix patterns
        let method_groups = self.group_methods_by_prefix(&analysis.methods);

        for (prefix, methods) in method_groups {
            let responsibility = self.infer_responsibility_name(&prefix);
            groups.push(ResponsibilityGroup {
                name: format!("{}Manager", responsibility.replace(' ', "")),
                methods,
                fields: Vec::new(), // Would need more sophisticated field grouping
                responsibility,
            });
        }

        // If no clear grouping, suggest a generic split
        if groups.is_empty() && analysis.method_count > self.max_methods {
            groups.push(ResponsibilityGroup {
                name: format!("{}Core", analysis.name),
                methods: analysis.methods.clone(),
                fields: analysis.fields.clone(),
                responsibility: "Core functionality".to_string(),
            });
        }

        groups
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
            } => {
                if *method_count > 30 || *field_count > 20 {
                    MaintainabilityImpact::Critical
                } else if *method_count > 20 || *field_count > 15 {
                    MaintainabilityImpact::High
                } else {
                    MaintainabilityImpact::Medium
                }
            }
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
    location_extractor: Option<UnifiedLocationExtractor>,
}

impl TypeVisitor {
    fn with_location_extractor(location_extractor: Option<UnifiedLocationExtractor>) -> Self {
        Self {
            types: HashMap::new(),
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
}
