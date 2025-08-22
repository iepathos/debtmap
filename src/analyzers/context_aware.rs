//! Context-aware analyzer wrapper that integrates context detection

use crate::analyzers::Analyzer;
use crate::context::rules::DebtPattern;
use crate::context::{
    detect_file_type, ContextDetector, ContextRuleEngine, FunctionContext, RuleAction,
};
use crate::core::{ast::Ast, DebtType, FileMetrics, Language, Priority};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use syn::visit::Visit;

/// Wrapper that adds context-awareness to any analyzer
pub struct ContextAwareAnalyzer {
    /// The underlying analyzer
    inner: Box<dyn Analyzer>,
    /// Context rule engine
    rule_engine: RwLock<ContextRuleEngine>,
    /// Whether context awareness is enabled
    enabled: bool,
}

impl ContextAwareAnalyzer {
    /// Create a new context-aware analyzer
    pub fn new(inner: Box<dyn Analyzer>) -> Self {
        Self {
            inner,
            rule_engine: RwLock::new(ContextRuleEngine::new()),
            enabled: true,
        }
    }

    /// Enable or disable context awareness
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Filter debt items based on context rules
    fn filter_debt_items(&self, mut metrics: FileMetrics, ast: &Ast, path: &Path) -> FileMetrics {
        if !self.enabled {
            return metrics;
        }

        // Detect file type
        let file_type = detect_file_type(path);

        // For Rust code, perform context detection
        if let Ast::Rust(rust_ast) = ast {
            let mut detector = ContextDetector::new(file_type);
            detector.visit_file(&rust_ast.file);

            // Filter debt items based on context
            metrics.debt_items.retain_mut(|item| {
                // Find the function context for this debt item using line number
                let context = detector
                    .get_context_for_line(item.line)
                    .cloned()
                    .unwrap_or_else(|| FunctionContext::new().with_file_type(file_type));

                // Convert debt type to pattern
                let pattern = debt_type_to_pattern(&item.debt_type, &item.message);

                // Evaluate the rule
                let action = self
                    .rule_engine
                    .write()
                    .unwrap()
                    .evaluate(&pattern, &context);

                match action {
                    RuleAction::Allow => {
                        // Pattern is allowed in this context, filter it out
                        false
                    }
                    RuleAction::Skip => {
                        // Skip analysis for this pattern
                        false
                    }
                    RuleAction::Warn => {
                        // Reduce severity by 2
                        item.priority = adjust_priority(item.priority, -2);

                        // Add context note to the message
                        if let Some(reason) = self
                            .rule_engine
                            .write()
                            .unwrap()
                            .get_reason(&pattern, &context)
                        {
                            item.message = format!("{} (Context: {})", item.message, reason);
                        }
                        true
                    }
                    RuleAction::ReduceSeverity(n) => {
                        // Reduce severity by n
                        item.priority = adjust_priority(item.priority, -n);

                        // Add context note to the message
                        if let Some(reason) = self
                            .rule_engine
                            .write()
                            .unwrap()
                            .get_reason(&pattern, &context)
                        {
                            item.message = format!("{} (Context: {})", item.message, reason);
                        }
                        true
                    }
                    RuleAction::Deny => {
                        // Keep the item as-is
                        true
                    }
                }
            });
        } else {
            // For non-Rust code, apply file-type based rules
            let context = FunctionContext::new().with_file_type(file_type);

            metrics.debt_items.retain_mut(|item| {
                let pattern = debt_type_to_pattern(&item.debt_type, &item.message);
                let action = self
                    .rule_engine
                    .write()
                    .unwrap()
                    .evaluate(&pattern, &context);

                match action {
                    RuleAction::Allow | RuleAction::Skip => false,
                    RuleAction::Warn => {
                        item.priority = adjust_priority(item.priority, -2);
                        true
                    }
                    RuleAction::ReduceSeverity(n) => {
                        item.priority = adjust_priority(item.priority, -n);
                        true
                    }
                    RuleAction::Deny => true,
                }
            });
        }

        metrics
    }
}

impl Analyzer for ContextAwareAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        self.inner.parse(content, path)
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        let metrics = self.inner.analyze(ast);

        // Apply context-aware filtering if we have the path
        if !metrics.path.as_os_str().is_empty() {
            let path = metrics.path.clone();
            self.filter_debt_items(metrics, ast, &path)
        } else {
            metrics
        }
    }

    fn language(&self) -> Language {
        self.inner.language()
    }
}

/// Convert a debt type to a debt pattern for rule matching
fn debt_type_to_pattern(debt_type: &DebtType, message: &str) -> DebtPattern {
    match debt_type {
        // Security patterns - check message for specific types
        DebtType::Security => {
            // Check if it's an input validation issue
            if message.contains("Input Validation") || message.contains("input validation") {
                DebtPattern::InputValidation
            } else {
                DebtPattern::Security
            }
        }

        // All other debt types
        _ => DebtPattern::DebtType(*debt_type),
    }
}

/// Adjust priority based on severity adjustment
fn adjust_priority(priority: Priority, adjustment: i32) -> Priority {
    if adjustment == -999 {
        // Special case: effectively disable
        return Priority::Low;
    }

    let current = match priority {
        Priority::Critical => 4,
        Priority::High => 3,
        Priority::Medium => 2,
        Priority::Low => 1,
    };

    let new = (current + adjustment).clamp(1, 4);

    match new {
        4 => Priority::Critical,
        3 => Priority::High,
        2 => Priority::Medium,
        _ => Priority::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjust_priority() {
        assert_eq!(adjust_priority(Priority::High, -1), Priority::Medium);
        assert_eq!(adjust_priority(Priority::Medium, -2), Priority::Low);
        assert_eq!(adjust_priority(Priority::Low, -1), Priority::Low);
        assert_eq!(adjust_priority(Priority::Critical, -999), Priority::Low);
    }

    #[test]
    fn test_debt_type_to_pattern_security_with_input_validation() {
        // Test Security debt type with "Input Validation" in message
        let pattern = debt_type_to_pattern(&DebtType::Security, "Input Validation: missing checks");
        assert_eq!(pattern, DebtPattern::InputValidation);

        // Test with lowercase "input validation"
        let pattern = debt_type_to_pattern(&DebtType::Security, "Need input validation here");
        assert_eq!(pattern, DebtPattern::InputValidation);
    }

    #[test]
    fn test_debt_type_to_pattern_security_without_input_validation() {
        // Test Security debt type without input validation keywords
        let pattern = debt_type_to_pattern(&DebtType::Security, "SQL injection vulnerability");
        assert_eq!(pattern, DebtPattern::Security);

        let pattern = debt_type_to_pattern(&DebtType::Security, "Potential XSS attack");
        assert_eq!(pattern, DebtPattern::Security);
    }

    #[test]
    fn test_debt_type_to_pattern_other_types() {
        // Test Todo debt type
        let pattern = debt_type_to_pattern(&DebtType::Todo, "Implement this feature");
        assert_eq!(pattern, DebtPattern::DebtType(DebtType::Todo));

        // Test Fixme debt type
        let pattern = debt_type_to_pattern(&DebtType::Fixme, "Fix this bug");
        assert_eq!(pattern, DebtPattern::DebtType(DebtType::Fixme));

        // Test CodeSmell debt type
        let pattern = debt_type_to_pattern(&DebtType::CodeSmell, "Code smell detected");
        assert_eq!(pattern, DebtPattern::DebtType(DebtType::CodeSmell));

        // Test Complexity debt type
        let pattern = debt_type_to_pattern(&DebtType::Complexity, "High cyclomatic complexity");
        assert_eq!(pattern, DebtPattern::DebtType(DebtType::Complexity));

        // Test Duplication debt type
        let pattern = debt_type_to_pattern(&DebtType::Duplication, "Duplicate code detected");
        assert_eq!(pattern, DebtPattern::DebtType(DebtType::Duplication));
    }

    #[test]
    fn test_debt_type_to_pattern_edge_cases() {
        // Test Security with empty message
        let pattern = debt_type_to_pattern(&DebtType::Security, "");
        assert_eq!(pattern, DebtPattern::Security);

        // Test Security with partial match (should not trigger)
        let pattern = debt_type_to_pattern(&DebtType::Security, "Input is valid");
        assert_eq!(pattern, DebtPattern::Security);

        // Test Security with different casing variations
        let pattern = debt_type_to_pattern(&DebtType::Security, "INPUT VALIDATION required");
        assert_eq!(pattern, DebtPattern::Security); // No match since contains() is case-sensitive
    }
}
