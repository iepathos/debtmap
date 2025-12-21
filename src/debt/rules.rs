//! Predicate-based debt detection rules.
//!
//! This module provides composable predicates for debt detection,
//! enabling dynamic rule configuration and clear error messages.
//!
//! # Predicate Composition
//!
//! Rules are built using stillwater's predicate combinators:
//!
//! ```rust,ignore
//! use debtmap::debt::rules::*;
//!
//! // Compose rules for function validation
//! let function_rules = FunctionDebtRules::default();
//!
//! // Check if a function violates any rules
//! let violations = function_rules.check(&function_metrics);
//! ```
//!
//! # Custom Rules
//!
//! Create custom rules by implementing predicates:
//!
//! ```rust,ignore
//! use stillwater::predicate::*;
//!
//! // Custom naming convention predicate
//! let snake_case_pred = all_chars(|c: char| c.is_lowercase() || c == '_' || c.is_numeric());
//! ```

use crate::core::{DebtItem, DebtType, FunctionMetrics, Priority};
use crate::effects::validation::ValidationRuleSet;
use std::path::PathBuf;
use stillwater::predicate::*;

/// Debt rule violation with context for error reporting.
#[derive(Debug, Clone)]
pub struct DebtViolation {
    /// The type of debt this violation represents.
    pub debt_type: DebtType,
    /// Priority of this violation.
    pub priority: Priority,
    /// Human-readable message describing the violation.
    pub message: String,
    /// The rule that was violated.
    pub rule_name: String,
}

impl DebtViolation {
    /// Convert this violation into a DebtItem for a specific location.
    pub fn into_debt_item(self, file: PathBuf, line: usize) -> DebtItem {
        DebtItem {
            id: format!("{}:{}:{}", file.display(), line, self.rule_name),
            debt_type: self.debt_type,
            priority: self.priority,
            file,
            line,
            column: None,
            message: self.message,
            context: Some(format!("Rule: {}", self.rule_name)),
        }
    }
}

/// Predicate-based rules for function-level debt detection.
#[derive(Debug, Clone, Default)]
pub struct FunctionDebtRules {
    /// Validation thresholds.
    pub rules: ValidationRuleSet,
}

impl FunctionDebtRules {
    /// Create a new FunctionDebtRules with the given rule set.
    pub fn new(rules: ValidationRuleSet) -> Self {
        Self { rules }
    }

    /// Create strict rules for high-quality codebases.
    pub fn strict() -> Self {
        Self {
            rules: ValidationRuleSet::strict(),
        }
    }

    /// Create lenient rules for legacy codebases.
    pub fn lenient() -> Self {
        Self {
            rules: ValidationRuleSet::lenient(),
        }
    }

    /// Check a function against all debt rules, returning violations.
    pub fn check(&self, metrics: &FunctionMetrics) -> Vec<DebtViolation> {
        let mut violations = Vec::new();

        // Check cyclomatic complexity
        if let Some(violation) = self.check_cyclomatic_complexity(metrics) {
            violations.push(violation);
        }

        // Check cognitive complexity
        if let Some(violation) = self.check_cognitive_complexity(metrics) {
            violations.push(violation);
        }

        // Check function length
        if let Some(violation) = self.check_function_length(metrics) {
            violations.push(violation);
        }

        // Check nesting depth
        if let Some(violation) = self.check_nesting_depth(metrics) {
            violations.push(violation);
        }

        violations
    }

    /// Check cyclomatic complexity against thresholds.
    fn check_cyclomatic_complexity(&self, metrics: &FunctionMetrics) -> Option<DebtViolation> {
        let complexity = metrics.cyclomatic;
        let critical_pred = ge(self.rules.complexity_critical);
        let warning_pred =
            ge(self.rules.complexity_warning).and(lt(self.rules.complexity_critical));

        if critical_pred.check(&complexity) {
            Some(DebtViolation {
                debt_type: DebtType::Complexity {
                    cyclomatic: complexity,
                    cognitive: metrics.cognitive,
                },
                priority: Priority::Critical,
                message: format!(
                    "Function '{}' has critical cyclomatic complexity: {} (threshold: {})",
                    metrics.name, complexity, self.rules.complexity_critical
                ),
                rule_name: "critical_cyclomatic_complexity".to_string(),
            })
        } else if warning_pred.check(&complexity) {
            Some(DebtViolation {
                debt_type: DebtType::Complexity {
                    cyclomatic: complexity,
                    cognitive: metrics.cognitive,
                },
                priority: Priority::High,
                message: format!(
                    "Function '{}' has high cyclomatic complexity: {} (warning threshold: {})",
                    metrics.name, complexity, self.rules.complexity_warning
                ),
                rule_name: "high_cyclomatic_complexity".to_string(),
            })
        } else {
            None
        }
    }

    /// Check cognitive complexity against thresholds.
    fn check_cognitive_complexity(&self, metrics: &FunctionMetrics) -> Option<DebtViolation> {
        let complexity = metrics.cognitive;
        let critical_pred = ge(self.rules.complexity_critical);
        let warning_pred =
            ge(self.rules.complexity_warning).and(lt(self.rules.complexity_critical));

        if critical_pred.check(&complexity) {
            Some(DebtViolation {
                debt_type: DebtType::Complexity {
                    cyclomatic: metrics.cyclomatic,
                    cognitive: complexity,
                },
                priority: Priority::Critical,
                message: format!(
                    "Function '{}' has critical cognitive complexity: {} (threshold: {})",
                    metrics.name, complexity, self.rules.complexity_critical
                ),
                rule_name: "critical_cognitive_complexity".to_string(),
            })
        } else if warning_pred.check(&complexity) {
            Some(DebtViolation {
                debt_type: DebtType::Complexity {
                    cyclomatic: metrics.cyclomatic,
                    cognitive: complexity,
                },
                priority: Priority::Medium,
                message: format!(
                    "Function '{}' has high cognitive complexity: {} (warning threshold: {})",
                    metrics.name, complexity, self.rules.complexity_warning
                ),
                rule_name: "high_cognitive_complexity".to_string(),
            })
        } else {
            None
        }
    }

    /// Check function length against threshold.
    fn check_function_length(&self, metrics: &FunctionMetrics) -> Option<DebtViolation> {
        let length = metrics.length;
        let too_long = gt(self.rules.max_function_length);

        if too_long.check(&length) {
            let priority = if length > self.rules.max_function_length * 2 {
                Priority::High
            } else {
                Priority::Medium
            };

            Some(DebtViolation {
                debt_type: DebtType::CodeSmell {
                    smell_type: Some("long_function".to_string()),
                },
                priority,
                message: format!(
                    "Function '{}' is too long: {} lines (max: {})",
                    metrics.name, length, self.rules.max_function_length
                ),
                rule_name: "function_too_long".to_string(),
            })
        } else {
            None
        }
    }

    /// Check nesting depth against threshold.
    fn check_nesting_depth(&self, metrics: &FunctionMetrics) -> Option<DebtViolation> {
        let nesting = metrics.nesting;
        let too_deep = gt(self.rules.max_nesting_depth);

        if too_deep.check(&nesting) {
            let priority = if nesting > self.rules.max_nesting_depth * 2 {
                Priority::High
            } else {
                Priority::Medium
            };

            Some(DebtViolation {
                debt_type: DebtType::CodeSmell {
                    smell_type: Some("deep_nesting".to_string()),
                },
                priority,
                message: format!(
                    "Function '{}' has excessive nesting: {} levels (max: {})",
                    metrics.name, nesting, self.rules.max_nesting_depth
                ),
                rule_name: "excessive_nesting".to_string(),
            })
        } else {
            None
        }
    }
}

/// Predicate-based rules for naming conventions.
pub struct NamingRules {
    /// Minimum name length.
    pub min_length: usize,
    /// Maximum name length.
    pub max_length: usize,
}

impl Default for NamingRules {
    fn default() -> Self {
        Self {
            min_length: 2,
            max_length: 50,
        }
    }
}

impl NamingRules {
    /// Check if a function name follows naming conventions.
    pub fn check_function_name(&self, name: &str) -> Option<DebtViolation> {
        // Check length
        if name.len() < self.min_length {
            return Some(DebtViolation {
                debt_type: DebtType::CodeSmell {
                    smell_type: Some("naming_convention".to_string()),
                },
                priority: Priority::Low,
                message: format!(
                    "Function name '{}' is too short (min: {} chars)",
                    name, self.min_length
                ),
                rule_name: "short_function_name".to_string(),
            });
        }

        if name.len() > self.max_length {
            return Some(DebtViolation {
                debt_type: DebtType::CodeSmell {
                    smell_type: Some("naming_convention".to_string()),
                },
                priority: Priority::Low,
                message: format!(
                    "Function name '{}' is too long (max: {} chars)",
                    name, self.max_length
                ),
                rule_name: "long_function_name".to_string(),
            });
        }

        // Check for snake_case (Rust convention)
        let first_char = name.chars().next()?;
        if first_char.is_uppercase() {
            return Some(DebtViolation {
                debt_type: DebtType::CodeSmell {
                    smell_type: Some("naming_convention".to_string()),
                },
                priority: Priority::Low,
                message: format!(
                    "Function name '{}' should be snake_case (starts with uppercase)",
                    name
                ),
                rule_name: "function_name_case".to_string(),
            });
        }

        None
    }
}

/// Severity-based rule configuration for debt detection.
#[derive(Debug, Clone)]
pub struct DebtSeverityRules {
    /// Complexity threshold for high severity.
    pub high_complexity_threshold: u32,
    /// Complexity threshold for critical severity.
    pub critical_complexity_threshold: u32,
    /// Function length threshold for high severity.
    pub high_length_threshold: usize,
    /// Function length threshold for critical severity.
    pub critical_length_threshold: usize,
}

impl Default for DebtSeverityRules {
    fn default() -> Self {
        Self {
            high_complexity_threshold: 21,
            critical_complexity_threshold: 50,
            high_length_threshold: 50,
            critical_length_threshold: 100,
        }
    }
}

impl DebtSeverityRules {
    /// Determine the severity of a complexity value.
    pub fn complexity_severity(&self, complexity: u32) -> Option<Priority> {
        let critical = ge(self.critical_complexity_threshold);
        let high = ge(self.high_complexity_threshold).and(lt(self.critical_complexity_threshold));

        if critical.check(&complexity) {
            Some(Priority::Critical)
        } else if high.check(&complexity) {
            Some(Priority::High)
        } else {
            None
        }
    }

    /// Determine the severity of a function length.
    pub fn length_severity(&self, length: usize) -> Option<Priority> {
        let critical = ge(self.critical_length_threshold);
        let high = ge(self.high_length_threshold).and(lt(self.critical_length_threshold));

        if critical.check(&length) {
            Some(Priority::Critical)
        } else if high.check(&length) {
            Some(Priority::High)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_function(
        name: &str,
        cyclomatic: u32,
        cognitive: u32,
        length: usize,
        nesting: u32,
    ) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic,
            cognitive,
            nesting,
            length,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }

    // =========================================================================
    // FunctionDebtRules Tests
    // =========================================================================

    #[test]
    fn test_function_debt_rules_no_violations() {
        let rules = FunctionDebtRules::default();
        let metrics = create_test_function("simple_fn", 5, 3, 20, 2);

        let violations = rules.check(&metrics);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_function_debt_rules_critical_complexity() {
        let rules = FunctionDebtRules::default();
        let metrics = create_test_function("complex_fn", 150, 5, 20, 2);

        let violations = rules.check(&metrics);
        assert!(!violations.is_empty());

        let critical = violations.iter().find(|v| v.priority == Priority::Critical);
        assert!(critical.is_some());
        assert!(critical
            .unwrap()
            .message
            .contains("critical cyclomatic complexity"));
    }

    #[test]
    fn test_function_debt_rules_warning_complexity() {
        let rules = FunctionDebtRules::default();
        let metrics = create_test_function("moderately_complex", 50, 5, 20, 2);

        let violations = rules.check(&metrics);
        assert!(!violations.is_empty());

        let high = violations.iter().find(|v| v.priority == Priority::High);
        assert!(high.is_some());
        assert!(high.unwrap().message.contains("high cyclomatic complexity"));
    }

    #[test]
    fn test_function_debt_rules_long_function() {
        let rules = FunctionDebtRules::default();
        let metrics = create_test_function("long_fn", 5, 3, 100, 2);

        let violations = rules.check(&metrics);
        assert!(!violations.is_empty());

        let length_violation = violations
            .iter()
            .find(|v| v.rule_name == "function_too_long");
        assert!(length_violation.is_some());
    }

    #[test]
    fn test_function_debt_rules_deep_nesting() {
        let rules = FunctionDebtRules::default();
        let metrics = create_test_function("deep_fn", 5, 3, 20, 10);

        let violations = rules.check(&metrics);
        assert!(!violations.is_empty());

        let nesting_violation = violations
            .iter()
            .find(|v| v.rule_name == "excessive_nesting");
        assert!(nesting_violation.is_some());
    }

    #[test]
    fn test_function_debt_rules_multiple_violations() {
        let rules = FunctionDebtRules::default();
        let metrics = create_test_function("problematic_fn", 150, 150, 200, 10);

        let violations = rules.check(&metrics);
        // Should have violations for cyclomatic, cognitive, length, and nesting
        assert!(violations.len() >= 4);
    }

    #[test]
    fn test_function_debt_rules_strict() {
        let rules = FunctionDebtRules::strict();
        // A function that's fine with default rules should violate strict rules
        let metrics = create_test_function("medium_fn", 15, 12, 30, 3);

        let violations = rules.check(&metrics);
        // Strict rules have lower thresholds
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_function_debt_rules_lenient() {
        let rules = FunctionDebtRules::lenient();
        // A function that violates default rules might be fine with lenient
        let metrics = create_test_function("larger_fn", 25, 20, 80, 5);

        let violations = rules.check(&metrics);
        // Lenient rules have higher thresholds
        assert!(violations.is_empty());
    }

    // =========================================================================
    // DebtViolation Tests
    // =========================================================================

    #[test]
    fn test_debt_violation_into_debt_item() {
        let violation = DebtViolation {
            debt_type: DebtType::Complexity {
                cyclomatic: 50,
                cognitive: 30,
            },
            priority: Priority::High,
            message: "Test violation".to_string(),
            rule_name: "test_rule".to_string(),
        };

        let debt_item = violation.into_debt_item(PathBuf::from("src/test.rs"), 42);

        assert_eq!(debt_item.line, 42);
        assert!(debt_item.id.contains("src/test.rs"));
        assert_eq!(debt_item.priority, Priority::High);
        assert!(debt_item.context.as_ref().unwrap().contains("test_rule"));
    }

    // =========================================================================
    // NamingRules Tests
    // =========================================================================

    #[test]
    fn test_naming_rules_valid_name() {
        let rules = NamingRules::default();
        assert!(rules.check_function_name("valid_function_name").is_none());
    }

    #[test]
    fn test_naming_rules_too_short() {
        let rules = NamingRules::default();
        let violation = rules.check_function_name("x");
        assert!(violation.is_some());
        assert!(violation.unwrap().message.contains("too short"));
    }

    #[test]
    fn test_naming_rules_too_long() {
        let rules = NamingRules {
            min_length: 2,
            max_length: 10,
        };
        let violation = rules.check_function_name("very_long_function_name");
        assert!(violation.is_some());
        assert!(violation.unwrap().message.contains("too long"));
    }

    #[test]
    fn test_naming_rules_uppercase_start() {
        let rules = NamingRules::default();
        let violation = rules.check_function_name("UpperCaseStart");
        assert!(violation.is_some());
        assert!(violation.unwrap().message.contains("snake_case"));
    }

    // =========================================================================
    // DebtSeverityRules Tests
    // =========================================================================

    #[test]
    fn test_debt_severity_rules_complexity() {
        let rules = DebtSeverityRules::default();

        assert_eq!(rules.complexity_severity(10), None);
        assert_eq!(rules.complexity_severity(30), Some(Priority::High));
        assert_eq!(rules.complexity_severity(100), Some(Priority::Critical));
    }

    #[test]
    fn test_debt_severity_rules_length() {
        let rules = DebtSeverityRules::default();

        assert_eq!(rules.length_severity(20), None);
        assert_eq!(rules.length_severity(75), Some(Priority::High));
        assert_eq!(rules.length_severity(150), Some(Priority::Critical));
    }
}
