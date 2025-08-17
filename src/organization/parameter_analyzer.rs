use super::{
    MaintainabilityImpact, OrganizationAntiPattern, OrganizationDetector, Parameter,
    ParameterGroup, ParameterRefactoring,
};
use crate::common::SourceLocation;
use std::collections::HashMap;
use syn::{self, visit::Visit};

pub struct ParameterAnalyzer {
    max_parameters: usize,
}

impl Default for ParameterAnalyzer {
    fn default() -> Self {
        Self { max_parameters: 5 }
    }
}

impl ParameterAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    fn suggest_parameter_refactoring(
        &self,
        function: &FunctionInfo,
        data_clumps: &[ParameterGroup],
    ) -> ParameterRefactoring {
        if !data_clumps.is_empty() {
            ParameterRefactoring::ExtractStruct
        } else if function.parameters.len() > 8 {
            ParameterRefactoring::UseBuilder
        } else if self.has_many_boolean_parameters(function) {
            ParameterRefactoring::UseConfiguration
        } else {
            ParameterRefactoring::SplitFunction
        }
    }

    fn has_many_boolean_parameters(&self, function: &FunctionInfo) -> bool {
        let bool_count = function
            .parameters
            .iter()
            .filter(|p| p.type_name == "bool")
            .count();

        bool_count > 2
    }

    fn find_data_clumps(&self, parameters: &[Parameter]) -> Vec<ParameterGroup> {
        let mut clumps = Vec::new();

        // Group parameters by semantic similarity
        let groups = self.group_parameters_by_semantics(parameters);

        for (semantic_group, group_params) in groups {
            if group_params.len() >= 3 {
                clumps.push(ParameterGroup {
                    parameters: group_params,
                    group_name: semantic_group.clone(),
                    semantic_relationship: semantic_group,
                });
            }
        }

        clumps
    }

    fn group_parameters_by_semantics(
        &self,
        parameters: &[Parameter],
    ) -> HashMap<String, Vec<Parameter>> {
        let mut groups: HashMap<String, Vec<Parameter>> = HashMap::new();

        for param in parameters {
            let semantic_group = self.identify_semantic_group(param);
            groups
                .entry(semantic_group)
                .or_default()
                .push(param.clone());
        }

        groups
    }

    fn identify_semantic_group(&self, parameter: &Parameter) -> String {
        let name = parameter.name.to_lowercase();

        // Define semantic patterns
        const SEMANTIC_PATTERNS: &[(&str, &[&str])] = &[
            ("coordinate", &["x", "y", "z", "width", "height", "depth"]),
            ("time", &["start", "end", "duration", "timeout", "delay"]),
            ("user", &["user", "username", "userid", "email", "name"]),
            ("config", &["config", "setting", "option", "preference"]),
            ("network", &["host", "port", "url", "endpoint", "address"]),
            ("file", &["path", "filename", "directory", "extension"]),
            (
                "authentication",
                &["token", "key", "secret", "auth", "credential"],
            ),
            ("pagination", &["page", "limit", "offset", "size", "count"]),
        ];

        for (group_name, keywords) in SEMANTIC_PATTERNS {
            if keywords.iter().any(|keyword| name.contains(keyword)) {
                return group_name.to_string();
            }
        }

        // If no semantic pattern matches, group by type
        format!("type_{}", parameter.type_name)
    }

    fn count_clump_occurrences(&self, clump: &ParameterGroup, functions: &[FunctionInfo]) -> usize {
        functions
            .iter()
            .filter(|f| self.function_has_clump(f, clump))
            .count()
    }

    fn function_has_clump(&self, function: &FunctionInfo, clump: &ParameterGroup) -> bool {
        // Check if function contains the same parameter pattern
        let clump_names: Vec<_> = clump.parameters.iter().map(|p| &p.name).collect();
        let function_names: Vec<_> = function.parameters.iter().map(|p| &p.name).collect();

        clump_names.iter().all(|name| function_names.contains(name))
    }

    fn suggest_struct_name(&self, clump: &ParameterGroup) -> String {
        if !clump.group_name.is_empty() {
            format!("{}Config", capitalize_first(&clump.group_name))
        } else {
            "ConfigParameters".to_string()
        }
    }
}

impl OrganizationDetector for ParameterAnalyzer {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        let mut visitor = FunctionVisitor::new();
        visitor.visit_file(file);

        let functions = visitor.functions;

        for function in &functions {
            // Check for long parameter lists
            if function.parameters.len() > self.max_parameters {
                let data_clumps = self.find_data_clumps(&function.parameters);
                let refactoring = self.suggest_parameter_refactoring(function, &data_clumps);

                patterns.push(OrganizationAntiPattern::LongParameterList {
                    function_name: function.name.clone(),
                    parameter_count: function.parameters.len(),
                    data_clumps,
                    suggested_refactoring: refactoring,
                    location: SourceLocation::default(), // TODO: Extract actual location
                });
            }

            // Check for data clumps even in shorter parameter lists
            let data_clumps = self.find_data_clumps(&function.parameters);
            for clump in data_clumps {
                if clump.parameters.len() >= 3 {
                    patterns.push(OrganizationAntiPattern::DataClump {
                        parameter_group: clump.clone(),
                        occurrence_count: self.count_clump_occurrences(&clump, &functions),
                        suggested_struct_name: self.suggest_struct_name(&clump),
                        locations: vec![SourceLocation::default()], // TODO: Extract actual locations
                    });
                }
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "ParameterAnalyzer"
    }

    fn estimate_maintainability_impact(
        &self,
        pattern: &OrganizationAntiPattern,
    ) -> MaintainabilityImpact {
        match pattern {
            OrganizationAntiPattern::LongParameterList {
                parameter_count, ..
            } => ParameterAnalyzer::classify_parameter_list_impact(*parameter_count),
            OrganizationAntiPattern::DataClump {
                occurrence_count, ..
            } => ParameterAnalyzer::classify_data_clump_impact(*occurrence_count),
            _ => MaintainabilityImpact::Low,
        }
    }
}

impl ParameterAnalyzer {
    /// Classify the maintainability impact based on parameter count
    fn classify_parameter_list_impact(parameter_count: usize) -> MaintainabilityImpact {
        match parameter_count {
            count if count > 10 => MaintainabilityImpact::High,
            count if count > 7 => MaintainabilityImpact::Medium,
            _ => MaintainabilityImpact::Low,
        }
    }

    /// Classify the maintainability impact based on data clump occurrences
    fn classify_data_clump_impact(occurrence_count: usize) -> MaintainabilityImpact {
        match occurrence_count {
            count if count > 5 => MaintainabilityImpact::Medium,
            _ => MaintainabilityImpact::Low,
        }
    }
}

struct FunctionInfo {
    name: String,
    parameters: Vec<Parameter>,
}

struct FunctionVisitor {
    functions: Vec<FunctionInfo>,
}

impl FunctionVisitor {
    fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }

    fn extract_parameters(
        &self,
        inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    ) -> Vec<Parameter> {
        let mut parameters = Vec::new();
        let mut position = 0;

        for input in inputs {
            if let syn::FnArg::Typed(pat_type) = input {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    let name = pat_ident.ident.to_string();
                    let type_name = self.extract_type_name(&pat_type.ty);

                    parameters.push(Parameter {
                        name,
                        type_name,
                        position,
                    });
                    position += 1;
                }
            }
        }

        parameters
    }

    #[allow(clippy::only_used_in_recursion)]
    fn extract_type_name(&self, ty: &syn::Type) -> String {
        match ty {
            syn::Type::Path(type_path) => type_path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            syn::Type::Reference(type_ref) => self.extract_type_name(&type_ref.elem),
            _ => "Unknown".to_string(),
        }
    }
}

impl<'ast> Visit<'ast> for FunctionVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let function = FunctionInfo {
            name: node.sig.ident.to_string(),
            parameters: self.extract_parameters(&node.sig.inputs),
        };
        self.functions.push(function);

        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let function = FunctionInfo {
            name: node.sig.ident.to_string(),
            parameters: self.extract_parameters(&node.sig.inputs),
        };
        self.functions.push(function);

        syn::visit::visit_impl_item_fn(self, node);
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::{
        OrganizationAntiPattern, ParameterGroup, ParameterRefactoring, ValueContext,
    };

    #[test]
    fn test_classify_parameter_list_impact_high() {
        // Test high impact for more than 10 parameters
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(11),
            MaintainabilityImpact::High
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(15),
            MaintainabilityImpact::High
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(100),
            MaintainabilityImpact::High
        );
    }

    #[test]
    fn test_classify_parameter_list_impact_medium() {
        // Test medium impact for 8-10 parameters
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(8),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(9),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(10),
            MaintainabilityImpact::Medium
        );
    }

    #[test]
    fn test_classify_parameter_list_impact_low() {
        // Test low impact for 7 or fewer parameters
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(0),
            MaintainabilityImpact::Low
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(5),
            MaintainabilityImpact::Low
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(7),
            MaintainabilityImpact::Low
        );
    }

    #[test]
    fn test_classify_data_clump_impact_medium() {
        // Test medium impact for more than 5 occurrences
        assert_eq!(
            ParameterAnalyzer::classify_data_clump_impact(6),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            ParameterAnalyzer::classify_data_clump_impact(10),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            ParameterAnalyzer::classify_data_clump_impact(100),
            MaintainabilityImpact::Medium
        );
    }

    #[test]
    fn test_classify_data_clump_impact_low() {
        // Test low impact for 5 or fewer occurrences
        assert_eq!(
            ParameterAnalyzer::classify_data_clump_impact(0),
            MaintainabilityImpact::Low
        );
        assert_eq!(
            ParameterAnalyzer::classify_data_clump_impact(3),
            MaintainabilityImpact::Low
        );
        assert_eq!(
            ParameterAnalyzer::classify_data_clump_impact(5),
            MaintainabilityImpact::Low
        );
    }

    #[test]
    fn test_estimate_maintainability_impact_long_parameter_list() {
        let analyzer = ParameterAnalyzer::new();

        // Test with high parameter count
        let pattern = OrganizationAntiPattern::LongParameterList {
            function_name: "test_function".to_string(),
            parameter_count: 12,
            data_clumps: vec![],
            suggested_refactoring: ParameterRefactoring::ExtractStruct,
        };
        assert_eq!(
            analyzer.estimate_maintainability_impact(&pattern),
            MaintainabilityImpact::High
        );

        // Test with medium parameter count
        let pattern = OrganizationAntiPattern::LongParameterList {
            function_name: "test_function".to_string(),
            parameter_count: 8,
            data_clumps: vec![],
            suggested_refactoring: ParameterRefactoring::ExtractStruct,
        };
        assert_eq!(
            analyzer.estimate_maintainability_impact(&pattern),
            MaintainabilityImpact::Medium
        );
    }

    #[test]
    fn test_estimate_maintainability_impact_data_clump() {
        let analyzer = ParameterAnalyzer::new();

        // Test with high occurrence count
        let pattern = OrganizationAntiPattern::DataClump {
            parameter_group: ParameterGroup {
                parameters: vec![],
                group_name: "test_group".to_string(),
                semantic_relationship: "related".to_string(),
            },
            occurrence_count: 7,
            suggested_struct_name: "TestStruct".to_string(),
        };
        assert_eq!(
            analyzer.estimate_maintainability_impact(&pattern),
            MaintainabilityImpact::Medium
        );

        // Test with low occurrence count
        let pattern = OrganizationAntiPattern::DataClump {
            parameter_group: ParameterGroup {
                parameters: vec![],
                group_name: "test_group".to_string(),
                semantic_relationship: "related".to_string(),
            },
            occurrence_count: 3,
            suggested_struct_name: "TestStruct".to_string(),
        };
        assert_eq!(
            analyzer.estimate_maintainability_impact(&pattern),
            MaintainabilityImpact::Low
        );
    }

    #[test]
    fn test_estimate_maintainability_impact_other_patterns() {
        let analyzer = ParameterAnalyzer::new();

        // Test with other pattern types (should return Low)
        let pattern = OrganizationAntiPattern::MagicValue {
            value_type: crate::organization::MagicValueType::NumericLiteral,
            value: "42".to_string(),
            occurrence_count: 5,
            suggested_constant_name: "ANSWER".to_string(),
            context: ValueContext::BusinessLogic,
        };
        assert_eq!(
            analyzer.estimate_maintainability_impact(&pattern),
            MaintainabilityImpact::Low
        );
    }

    #[test]
    fn test_parameter_list_boundary_values() {
        // Test boundary values for parameter list impact classification
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(7),
            MaintainabilityImpact::Low
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(8),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(10),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(11),
            MaintainabilityImpact::High
        );
    }
}
