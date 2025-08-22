use super::{
    MagicValueType, MaintainabilityImpact, OrganizationAntiPattern, OrganizationDetector,
    ValueContext,
};
use crate::common::SourceLocation;
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
                    locations: vec![SourceLocation::default()], // TODO: Extract actual locations
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
                    locations: vec![SourceLocation::default()], // TODO: Extract actual locations
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
        if let syn::Expr::Lit(expr_lit) = node {
            match &expr_lit.lit {
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
            }
        }

        syn::visit::visit_expr(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_should_ignore_common_numeric_values() {
        let detector = MagicValueDetector::new();

        // Common values should be ignored by default
        assert!(detector.should_ignore_numeric_value("0"));
        assert!(detector.should_ignore_numeric_value("1"));
        assert!(detector.should_ignore_numeric_value("-1"));
        assert!(detector.should_ignore_numeric_value("100"));
        assert!(detector.should_ignore_numeric_value("0.5"));

        // Non-common values should not be ignored
        assert!(!detector.should_ignore_numeric_value("42"));
        assert!(!detector.should_ignore_numeric_value("3.14"));
        assert!(!detector.should_ignore_numeric_value("256"));
        assert!(!detector.should_ignore_numeric_value("-42"));
    }

    #[test]
    fn test_should_not_ignore_when_disabled() {
        let mut detector = MagicValueDetector::new();
        detector.ignore_common_values = false;

        // Even common values should not be ignored when disabled
        assert!(!detector.should_ignore_numeric_value("0"));
        assert!(!detector.should_ignore_numeric_value("1"));
        assert!(!detector.should_ignore_numeric_value("100"));
    }

    #[test]
    fn test_should_ignore_common_string_values() {
        let detector = MagicValueDetector::new();

        // Common strings should be ignored
        assert!(detector.should_ignore_string_value(""));
        assert!(detector.should_ignore_string_value(" "));
        assert!(detector.should_ignore_string_value("\n"));
        assert!(detector.should_ignore_string_value("\t"));

        // Non-common strings should not be ignored
        assert!(!detector.should_ignore_string_value("hello"));
        assert!(!detector.should_ignore_string_value("config"));
        assert!(!detector.should_ignore_string_value("error message"));
    }

    #[test]
    fn test_value_to_identifier_conversion() {
        let detector = MagicValueDetector::new();

        assert_eq!(detector.value_to_identifier("42"), "42");
        assert_eq!(detector.value_to_identifier("3.14"), "3_DOT_14");
        assert_eq!(detector.value_to_identifier("-1"), "NEG_1");
        assert_eq!(detector.value_to_identifier("hello world"), "HELLO_WORLD");
        assert_eq!(detector.value_to_identifier("\"quoted\""), "QUOTED");
        assert_eq!(detector.value_to_identifier("-3.5"), "NEG_3_DOT_5");
    }

    #[test]
    fn test_infer_context_timeout() {
        let detector = MagicValueDetector::new();

        assert!(matches!(
            detector.infer_context("timeout_value"),
            ValueContext::Timeout
        ));
        assert!(matches!(
            detector.infer_context("delay_ms"),
            ValueContext::Timeout
        ));
        assert!(matches!(
            detector.infer_context("duration_seconds"),
            ValueContext::Timeout
        ));
    }

    #[test]
    fn test_infer_context_buffer_size() {
        let detector = MagicValueDetector::new();

        assert!(matches!(
            detector.infer_context("buffer_size"),
            ValueContext::BufferSize
        ));
        assert!(matches!(
            detector.infer_context("capacity"),
            ValueContext::BufferSize
        ));
        assert!(matches!(
            detector.infer_context("max_size"),
            ValueContext::BufferSize
        ));
    }

    #[test]
    fn test_infer_context_array_indexing() {
        let detector = MagicValueDetector::new();

        assert!(matches!(
            detector.infer_context("arr[3]"),
            ValueContext::ArrayIndexing
        ));
        assert!(matches!(
            detector.infer_context("get_index"),
            ValueContext::ArrayIndexing
        ));
    }

    #[test]
    fn test_infer_context_comparison() {
        let detector = MagicValueDetector::new();

        assert!(matches!(
            detector.infer_context("value == 42"),
            ValueContext::Comparison
        ));
        assert!(matches!(
            detector.infer_context("x > 100"),
            ValueContext::Comparison
        ));
        assert!(matches!(
            detector.infer_context("y != 0"),
            ValueContext::Comparison
        ));
    }

    #[test]
    fn test_infer_context_calculation() {
        let detector = MagicValueDetector::new();

        assert!(matches!(
            detector.infer_context("x + 5"),
            ValueContext::Calculation
        ));
        assert!(matches!(
            detector.infer_context("y * 2"),
            ValueContext::Calculation
        ));
        assert!(matches!(
            detector.infer_context("z / 10"),
            ValueContext::Calculation
        ));
    }

    #[test]
    fn test_suggest_constant_name() {
        let detector = MagicValueDetector::new();

        assert_eq!(
            detector.suggest_constant_name("5000", &ValueContext::Timeout),
            "TIMEOUT_5000_MS"
        );
        assert_eq!(
            detector.suggest_constant_name("1024", &ValueContext::BufferSize),
            "BUFFER_SIZE_1024"
        );
        assert_eq!(
            detector.suggest_constant_name("3", &ValueContext::ArrayIndexing),
            "INDEX_3"
        );
        assert_eq!(
            detector.suggest_constant_name("100", &ValueContext::Comparison),
            "THRESHOLD_100"
        );
        assert_eq!(
            detector.suggest_constant_name("0.5", &ValueContext::Calculation),
            "FACTOR_0_DOT_5"
        );
        assert_eq!(
            detector.suggest_constant_name("42", &ValueContext::BusinessLogic),
            "BUSINESS_RULE_42"
        );
    }

    #[test]
    fn test_estimate_maintainability_impact_high() {
        let detector = MagicValueDetector::new();

        let pattern = OrganizationAntiPattern::MagicValue {
            value_type: MagicValueType::NumericLiteral,
            value: "42".to_string(),
            occurrence_count: 6,
            suggested_constant_name: "CONSTANT_42".to_string(),
            context: ValueContext::BusinessLogic,
            locations: vec![],
        };

        assert!(matches!(
            detector.estimate_maintainability_impact(&pattern),
            MaintainabilityImpact::High
        ));
    }

    #[test]
    fn test_estimate_maintainability_impact_medium() {
        let detector = MagicValueDetector::new();

        let pattern = OrganizationAntiPattern::MagicValue {
            value_type: MagicValueType::NumericLiteral,
            value: "100".to_string(),
            occurrence_count: 3,
            suggested_constant_name: "TIMEOUT_100_MS".to_string(),
            context: ValueContext::Timeout,
            locations: vec![],
        };

        assert!(matches!(
            detector.estimate_maintainability_impact(&pattern),
            MaintainabilityImpact::Medium
        ));
    }

    #[test]
    fn test_estimate_maintainability_impact_low() {
        let detector = MagicValueDetector::new();

        let pattern = OrganizationAntiPattern::MagicValue {
            value_type: MagicValueType::NumericLiteral,
            value: "2".to_string(),
            occurrence_count: 3,
            suggested_constant_name: "FACTOR_2".to_string(),
            context: ValueContext::Calculation,
            locations: vec![],
        };

        assert!(matches!(
            detector.estimate_maintainability_impact(&pattern),
            MaintainabilityImpact::Low
        ));
    }

    #[test]
    fn test_detect_numeric_magic_values() {
        let detector = MagicValueDetector::new();

        let file: syn::File = parse_quote! {
            fn calculate() {
                let x = 42;
                let y = 42;
                let z = 42;
            }
        };

        let patterns = detector.detect_anti_patterns(&file);

        // Should detect 42 appearing 3 times
        assert!(!patterns.is_empty());
        let first_pattern = &patterns[0];

        if let OrganizationAntiPattern::MagicValue {
            value,
            occurrence_count,
            value_type,
            ..
        } = first_pattern
        {
            assert_eq!(value, "42");
            assert_eq!(*occurrence_count, 3);
            assert!(matches!(value_type, MagicValueType::NumericLiteral));
        } else {
            panic!("Expected MagicValue pattern");
        }
    }

    #[test]
    fn test_detect_string_magic_values() {
        let detector = MagicValueDetector::new();

        let file: syn::File = parse_quote! {
            fn process() {
                let msg1 = "error occurred";
                let msg2 = "error occurred";
                let msg3 = "error occurred";
            }
        };

        let patterns = detector.detect_anti_patterns(&file);

        // Should detect "error occurred" appearing 3 times
        assert!(!patterns.is_empty());

        let string_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                OrganizationAntiPattern::MagicValue {
                    value_type: MagicValueType::StringLiteral,
                    ..
                }
            )
        });

        assert!(string_pattern.is_some());

        if let Some(OrganizationAntiPattern::MagicValue {
            value,
            occurrence_count,
            ..
        }) = string_pattern
        {
            assert_eq!(value, "error occurred");
            assert_eq!(*occurrence_count, 3);
        }
    }

    #[test]
    fn test_ignores_common_values() {
        let detector = MagicValueDetector::new();

        let file: syn::File = parse_quote! {
            fn common_values() {
                let a = 0;
                let b = 0;
                let c = 1;
                let d = 1;
                let e = 1;
            }
        };

        let patterns = detector.detect_anti_patterns(&file);

        // Should not detect 0 and 1 as magic values (they're common)
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_threshold_behavior() {
        let mut detector = MagicValueDetector::new();
        detector.min_occurrence_threshold = 3;

        let file: syn::File = parse_quote! {
            fn threshold_test() {
                let x = 99;
                let y = 99;  // Only 2 occurrences
            }
        };

        let patterns = detector.detect_anti_patterns(&file);

        // Should not detect 99 since it appears only 2 times (below threshold of 3)
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_literal_visitor() {
        let mut visitor = LiteralVisitor::new();

        let expr: syn::Expr = parse_quote! { 42 };
        visitor.visit_expr(&expr);
        assert_eq!(visitor.numeric_literals.len(), 1);
        assert_eq!(visitor.numeric_literals[0].0, "42");

        let expr: syn::Expr = parse_quote! { 3.14 };
        visitor.visit_expr(&expr);
        assert_eq!(visitor.numeric_literals.len(), 2);
        assert_eq!(visitor.numeric_literals[1].0, "3.14");

        let expr: syn::Expr = parse_quote! { "hello" };
        visitor.visit_expr(&expr);
        assert_eq!(visitor.string_literals.len(), 1);
        assert_eq!(visitor.string_literals[0].0, "hello");
    }
}
