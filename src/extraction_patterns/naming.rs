use super::{AccumulationOp, ExtractablePattern};

pub struct FunctionNameInferrer;

impl Default for FunctionNameInferrer {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionNameInferrer {
    pub fn new() -> Self {
        Self
    }

    pub fn infer_name(pattern: &ExtractablePattern, language: &str) -> String {
        let base_name = Self::generate_base_name(pattern);
        Self::format_for_language(base_name, language)
    }

    // Extract pure helper functions to reduce complexity
    fn accumulation_op_to_verb(operation: &AccumulationOp) -> &str {
        match operation {
            AccumulationOp::Sum => "sum",
            AccumulationOp::Product => "multiply",
            AccumulationOp::Concatenation => "concat",
            AccumulationOp::Collection => "collect",
            AccumulationOp::Custom(name) => name.as_str(),
        }
    }

    fn build_accumulation_name(
        operation: &AccumulationOp,
        filter: &Option<Box<super::Expression>>,
        transform: &Option<Box<super::Expression>>,
        iterator_binding: &str,
    ) -> String {
        let mut name_parts = vec![];

        if filter.is_some() {
            name_parts.push("filter");
        }

        if transform.is_some() {
            name_parts.push("map");
        }

        name_parts.push(Self::accumulation_op_to_verb(operation));
        name_parts.push(Self::pluralize(iterator_binding));

        name_parts.join("_")
    }

    fn build_guard_chain_name(checks_count: usize) -> String {
        if checks_count == 1 {
            "validate_precondition".to_string()
        } else {
            format!("validate_{}_preconditions", checks_count)
        }
    }

    fn build_transformation_name(
        input_binding: &str,
        output_type: &str,
        stages_count: usize,
    ) -> String {
        if stages_count == 1 {
            format!(
                "transform_{}_to_{}",
                Self::singularize(input_binding),
                Self::singularize(output_type)
            )
        } else {
            format!("process_{}_pipeline", Self::singularize(input_binding))
        }
    }

    fn build_branches_name(condition_var: &str) -> String {
        format!("handle_{}_cases", Self::singularize(condition_var))
    }

    fn build_nested_extraction_name(outer_scope: &str) -> String {
        format!("process_{}_block", Self::singularize(outer_scope))
    }

    fn generate_base_name(pattern: &ExtractablePattern) -> String {
        match pattern {
            ExtractablePattern::AccumulationLoop {
                iterator_binding,
                operation,
                filter,
                transform,
                ..
            } => Self::build_accumulation_name(operation, filter, transform, iterator_binding),

            ExtractablePattern::GuardChainSequence { checks, .. } => {
                Self::build_guard_chain_name(checks.len())
            }

            ExtractablePattern::TransformationPipeline {
                input_binding,
                output_type,
                stages,
                ..
            } => Self::build_transformation_name(input_binding, output_type, stages.len()),

            ExtractablePattern::SimilarBranches { condition_var, .. } => {
                Self::build_branches_name(condition_var)
            }

            ExtractablePattern::NestedExtraction { outer_scope, .. } => {
                Self::build_nested_extraction_name(outer_scope)
            }
        }
    }

    fn format_for_language(name: String, language: &str) -> String {
        match language {
            "rust" | "python" => name, // snake_case
            "javascript" | "typescript" => Self::to_camel_case(&name),
            _ => name,
        }
    }

    fn to_camel_case(snake_case: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for ch in snake_case.chars() {
            if ch == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(ch.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn pluralize(word: &str) -> &str {
        // Simple pluralization - could be enhanced
        // For now, just return the word as-is
        // TODO: Implement proper pluralization logic
        word
    }

    fn singularize(word: &str) -> &str {
        // Simple singularization - could be enhanced
        // For now, just return the word as-is
        // TODO: Implement proper singularization logic
        word
    }
}

pub struct NameVariantGenerator;

impl NameVariantGenerator {
    pub fn generate_variants(base_name: &str, context: &str) -> Vec<String> {
        let mut variants = vec![base_name.to_string()];

        // Add context-specific variants
        if !context.is_empty() {
            variants.push(format!("{}_{}", base_name, context));
            variants.push(format!("{}_{}", context, base_name));
        }

        // Add common prefixes
        variants.push(format!("get_{}", base_name));
        variants.push(format!("calculate_{}", base_name));
        variants.push(format!("compute_{}", base_name));
        variants.push(format!("extract_{}", base_name));

        variants
    }

    pub fn select_best_variant(variants: &[String], existing_names: &[String]) -> String {
        // Select variant that doesn't conflict with existing names
        for variant in variants {
            if !existing_names.contains(variant) {
                return variant.clone();
            }
        }

        // If all conflict, append a number
        let base = &variants[0];
        let mut counter = 2;
        loop {
            let numbered = format!("{}_{}", base, counter);
            if !existing_names.contains(&numbered) {
                return numbered;
            }
            counter += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulation_loop_naming() {
        let pattern = ExtractablePattern::AccumulationLoop {
            iterator_binding: "item".to_string(),
            accumulator: "total".to_string(),
            operation: AccumulationOp::Sum,
            filter: None,
            transform: None,
            start_line: 1,
            end_line: 5,
        };

        let name = FunctionNameInferrer::infer_name(&pattern, "rust");
        assert!(name.contains("sum"));
    }

    #[test]
    fn test_guard_chain_naming() {
        let pattern = ExtractablePattern::GuardChainSequence {
            checks: vec![],
            early_return: super::super::ReturnType {
                type_name: "bool".to_string(),
                is_early_return: true,
            },
            start_line: 1,
            end_line: 10,
        };

        let name = FunctionNameInferrer::infer_name(&pattern, "rust");
        assert!(name.contains("validate"));
    }

    #[test]
    fn test_camel_case_conversion() {
        let snake = "validate_preconditions";
        let camel = FunctionNameInferrer::to_camel_case(snake);
        assert_eq!(camel, "validatePreconditions");
    }

    #[test]
    fn test_variant_generation() {
        let variants = NameVariantGenerator::generate_variants("sum_values", "totals");
        assert!(variants.contains(&"sum_values".to_string()));
        assert!(variants.contains(&"sum_values_totals".to_string()));
        assert!(variants.contains(&"calculate_sum_values".to_string()));
    }

    #[test]
    fn test_accumulation_op_to_verb() {
        assert_eq!(FunctionNameInferrer::accumulation_op_to_verb(&AccumulationOp::Sum), "sum");
        assert_eq!(FunctionNameInferrer::accumulation_op_to_verb(&AccumulationOp::Product), "multiply");
        assert_eq!(FunctionNameInferrer::accumulation_op_to_verb(&AccumulationOp::Concatenation), "concat");
        assert_eq!(FunctionNameInferrer::accumulation_op_to_verb(&AccumulationOp::Collection), "collect");
        
        let custom_op = AccumulationOp::Custom("aggregate".to_string());
        assert_eq!(FunctionNameInferrer::accumulation_op_to_verb(&custom_op), "aggregate");
    }

    #[test]
    fn test_build_accumulation_name() {
        // Test without filter or transform
        let name = FunctionNameInferrer::build_accumulation_name(
            &AccumulationOp::Sum,
            &None,
            &None,
            "items"
        );
        assert_eq!(name, "sum_items");

        // Test with filter
        let filter = Some(Box::new(super::super::Expression {
            code: "x > 0".to_string(),
            variables: vec!["x".to_string()],
        }));
        let name = FunctionNameInferrer::build_accumulation_name(
            &AccumulationOp::Collection,
            &filter,
            &None,
            "values"
        );
        assert_eq!(name, "filter_collect_values");

        // Test with both filter and transform
        let transform = Some(Box::new(super::super::Expression {
            code: "x * 2".to_string(),
            variables: vec!["x".to_string()],
        }));
        let name = FunctionNameInferrer::build_accumulation_name(
            &AccumulationOp::Sum,
            &filter,
            &transform,
            "numbers"
        );
        assert_eq!(name, "filter_map_sum_numbers");
    }

    #[test]
    fn test_build_guard_chain_name() {
        // Single check
        assert_eq!(
            FunctionNameInferrer::build_guard_chain_name(1),
            "validate_precondition"
        );
        
        // Multiple checks
        assert_eq!(
            FunctionNameInferrer::build_guard_chain_name(3),
            "validate_3_preconditions"
        );
        
        // Zero checks edge case
        assert_eq!(
            FunctionNameInferrer::build_guard_chain_name(0),
            "validate_0_preconditions"
        );
    }

    #[test]
    fn test_build_transformation_name() {
        // Single stage transformation
        let name = FunctionNameInferrer::build_transformation_name(
            "string",
            "number",
            1
        );
        assert_eq!(name, "transform_string_to_number");

        // Multi-stage pipeline
        let name = FunctionNameInferrer::build_transformation_name(
            "data",
            "result",
            3
        );
        assert_eq!(name, "process_data_pipeline");
    }

    #[test]
    fn test_build_branches_name() {
        assert_eq!(
            FunctionNameInferrer::build_branches_name("status"),
            "handle_status_cases"
        );
        
        assert_eq!(
            FunctionNameInferrer::build_branches_name("error_type"),
            "handle_error_type_cases"
        );
    }

    #[test]
    fn test_build_nested_extraction_name() {
        assert_eq!(
            FunctionNameInferrer::build_nested_extraction_name("loop"),
            "process_loop_block"
        );
        
        assert_eq!(
            FunctionNameInferrer::build_nested_extraction_name("condition"),
            "process_condition_block"
        );
    }
}
