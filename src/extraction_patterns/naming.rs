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

    fn generate_base_name(pattern: &ExtractablePattern) -> String {
        match pattern {
            ExtractablePattern::AccumulationLoop {
                iterator_binding,
                operation,
                filter,
                transform,
                ..
            } => {
                let op_verb = match operation {
                    AccumulationOp::Sum => "sum",
                    AccumulationOp::Product => "multiply",
                    AccumulationOp::Concatenation => "concat",
                    AccumulationOp::Collection => "collect",
                    AccumulationOp::Custom(name) => name.as_str(),
                };

                let mut name_parts = vec![];

                if filter.is_some() {
                    name_parts.push("filter");
                }

                if transform.is_some() {
                    name_parts.push("map");
                }

                name_parts.push(op_verb);
                name_parts.push(Self::pluralize(iterator_binding));

                name_parts.join("_")
            }

            ExtractablePattern::GuardChainSequence { checks, .. } => {
                if checks.len() == 1 {
                    "validate_precondition".to_string()
                } else {
                    format!("validate_{}_preconditions", checks.len())
                }
            }

            ExtractablePattern::TransformationPipeline {
                input_binding,
                output_type,
                stages,
                ..
            } => {
                if stages.len() == 1 {
                    format!(
                        "transform_{}_to_{}",
                        Self::singularize(input_binding),
                        Self::singularize(output_type)
                    )
                } else {
                    format!("process_{}_pipeline", Self::singularize(input_binding))
                }
            }

            ExtractablePattern::SimilarBranches { condition_var, .. } => {
                format!("handle_{}_cases", Self::singularize(condition_var))
            }

            ExtractablePattern::NestedExtraction { outer_scope, .. } => {
                format!("process_{}_block", Self::singularize(outer_scope))
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
}
