use super::{
    MagicValueType, MaintainabilityImpact, OrganizationAntiPattern, OrganizationDetector,
    ValueContext,
};
use std::collections::HashMap;
use syn::{self, visit::Visit};

pub struct MagicValueDetector {
    ignore_common_values: bool,
    min_occurrence_threshold: usize,
}

impl Default for MagicValueDetector {
    fn default() -> Self {
        Self {
            ignore_common_values: true,
            min_occurrence_threshold: 2,
        }
    }
}

impl MagicValueDetector {
    pub fn new() -> Self {
        Self::default()
    }

    fn should_ignore_numeric_value(&self, value: &str) -> bool {
        if !self.ignore_common_values {
            return false;
        }

        // Common values that are typically not magic numbers
        const COMMON_VALUES: &[&str] = &[
            "0", "1", "-1", "2", "10", "100", "1000", "0.0", "1.0", "-1.0", "0.5", "2.0",
        ];

        COMMON_VALUES.contains(&value)
    }

    fn should_ignore_string_value(&self, value: &str) -> bool {
        if !self.ignore_common_values {
            return false;
        }

        // Common strings that shouldn't be flagged
        value.is_empty() || value == " " || value == "\n" || value == "\t"
    }

    fn suggest_constant_name(&self, value: &str, context: &ValueContext) -> String {
        match context {
            ValueContext::Timeout => format!("TIMEOUT_{}_MS", self.value_to_identifier(value)),
            ValueContext::BufferSize => format!("BUFFER_SIZE_{}", self.value_to_identifier(value)),
            ValueContext::ArrayIndexing => format!("INDEX_{}", self.value_to_identifier(value)),
            ValueContext::BusinessLogic => {
                format!("BUSINESS_RULE_{}", self.value_to_identifier(value))
            }
            ValueContext::Calculation => format!("FACTOR_{}", self.value_to_identifier(value)),
            ValueContext::Comparison => format!("THRESHOLD_{}", self.value_to_identifier(value)),
        }
    }

    fn value_to_identifier(&self, value: &str) -> String {
        value
            .replace('.', "_DOT_")
            .replace('-', "NEG_")
            .replace(' ', "_")
            .replace('"', "")
            .to_uppercase()
    }

    fn infer_context(&self, usage_context: &str) -> ValueContext {
        let lower = usage_context.to_lowercase();

        if lower.contains("timeout") || lower.contains("delay") || lower.contains("duration") {
            ValueContext::Timeout
        } else if lower.contains("buffer") || lower.contains("capacity") || lower.contains("size") {
            ValueContext::BufferSize
        } else if lower.contains("index") || lower.contains("[") {
            ValueContext::ArrayIndexing
        } else if lower.contains("==")
            || lower.contains("!=")
            || lower.contains(">")
            || lower.contains("<")
        {
            ValueContext::Comparison
        } else if lower.contains("+")
            || lower.contains("-")
            || lower.contains("*")
            || lower.contains("/")
        {
            ValueContext::Calculation
        } else {
            ValueContext::BusinessLogic
        }
    }
}

impl OrganizationDetector for MagicValueDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        let mut visitor = LiteralVisitor::new();
        visitor.visit_file(file);

        // Analyze numeric literals
        let mut numeric_counts: HashMap<String, usize> = HashMap::new();
        for (value, _) in &visitor.numeric_literals {
            if !self.should_ignore_numeric_value(value) {
                *numeric_counts.entry(value.clone()).or_insert(0) += 1;
            }
        }

        for (value, count) in numeric_counts {
            if count >= self.min_occurrence_threshold {
                let context = self.infer_context(&value);
                patterns.push(OrganizationAntiPattern::MagicValue {
                    value_type: MagicValueType::NumericLiteral,
                    value: value.clone(),
                    occurrence_count: count,
                    suggested_constant_name: self.suggest_constant_name(&value, &context),
                    context,
                });
            }
        }

        // Analyze string literals
        let mut string_counts: HashMap<String, usize> = HashMap::new();
        for (value, _) in &visitor.string_literals {
            if !self.should_ignore_string_value(value) {
                *string_counts.entry(value.clone()).or_insert(0) += 1;
            }
        }

        for (value, count) in string_counts {
            if count >= self.min_occurrence_threshold {
                patterns.push(OrganizationAntiPattern::MagicValue {
                    value_type: MagicValueType::StringLiteral,
                    value: value.clone(),
                    occurrence_count: count,
                    suggested_constant_name: format!("STR_{}", self.value_to_identifier(&value)),
                    context: ValueContext::BusinessLogic,
                });
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "MagicValueDetector"
    }

    fn estimate_maintainability_impact(
        &self,
        pattern: &OrganizationAntiPattern,
    ) -> MaintainabilityImpact {
        match pattern {
            OrganizationAntiPattern::MagicValue {
                occurrence_count,
                context,
                ..
            } => match context {
                ValueContext::BusinessLogic | ValueContext::Timeout => {
                    if *occurrence_count > 5 {
                        MaintainabilityImpact::High
                    } else {
                        MaintainabilityImpact::Medium
                    }
                }
                _ => {
                    if *occurrence_count > 10 {
                        MaintainabilityImpact::Medium
                    } else {
                        MaintainabilityImpact::Low
                    }
                }
            },
            _ => MaintainabilityImpact::Low,
        }
    }
}

struct LiteralVisitor {
    numeric_literals: Vec<(String, String)>, // (value, context)
    string_literals: Vec<(String, String)>,  // (value, context)
}

impl LiteralVisitor {
    fn new() -> Self {
        Self {
            numeric_literals: Vec::new(),
            string_literals: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for LiteralVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        match node {
            syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Int(lit_int) => {
                    let value = lit_int.base10_digits().to_string();
                    self.numeric_literals.push((value, "numeric".to_string()));
                }
                syn::Lit::Float(lit_float) => {
                    let value = lit_float.base10_digits().to_string();
                    self.numeric_literals.push((value, "numeric".to_string()));
                }
                syn::Lit::Str(lit_str) => {
                    let value = lit_str.value();
                    self.string_literals.push((value, "string".to_string()));
                }
                _ => {}
            },
            _ => {}
        }

        syn::visit::visit_expr(self, node);
    }
}
