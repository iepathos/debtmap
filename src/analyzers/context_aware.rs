//! Context-aware analyzer wrapper that integrates context detection

use crate::analyzers::Analyzer;
use crate::context::rules::DebtPattern;
use crate::context::{
    detect_file_type, ContextDetector, ContextRuleEngine, FileType, FunctionContext, RuleAction,
};
use crate::core::{
    ast::{Ast, RustAst},
    DebtItem, DebtType, FileMetrics, Language, Priority,
};
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

    /// Process a debt item based on the rule action
    fn process_rule_action(
        &self,
        action: RuleAction,
        item: &mut DebtItem,
        pattern: &DebtPattern,
        context: &FunctionContext,
    ) -> bool {
        match action {
            RuleAction::Allow | RuleAction::Skip => false,
            RuleAction::Warn => {
                item.priority = adjust_priority(item.priority, -2);
                self.add_context_note(item, pattern, context);
                true
            }
            RuleAction::ReduceSeverity(n) => {
                item.priority = adjust_priority(item.priority, -n);
                self.add_context_note(item, pattern, context);
                true
            }
            RuleAction::Deny => true,
        }
    }

    /// Add context note to debt item message
    fn add_context_note(
        &self,
        item: &mut DebtItem,
        pattern: &DebtPattern,
        context: &FunctionContext,
    ) {
        if let Some(reason) = self
            .rule_engine
            .write()
            .unwrap()
            .get_reason(pattern, context)
        {
            item.message = format!("{} (Context: {})", item.message, reason);
        }
    }

    /// Process debt items for Rust code
    fn process_rust_items(
        &self,
        metrics: &mut FileMetrics,
        rust_ast: &RustAst,
        file_type: FileType,
    ) {
        let mut detector = ContextDetector::new(file_type);
        detector.visit_file(&rust_ast.file);

        metrics.debt_items.retain_mut(|item| {
            let context = detector
                .get_context_for_line(item.line)
                .cloned()
                .unwrap_or_else(|| FunctionContext::new().with_file_type(file_type));

            let pattern = debt_type_to_pattern(&item.debt_type, &item.message);
            let action = self
                .rule_engine
                .write()
                .unwrap()
                .evaluate(&pattern, &context);

            self.process_rule_action(action, item, &pattern, &context)
        });
    }

    /// Process debt items for non-Rust code
    fn process_non_rust_items(&self, metrics: &mut FileMetrics, file_type: FileType) {
        let context = FunctionContext::new().with_file_type(file_type);

        metrics.debt_items.retain_mut(|item| {
            let pattern = debt_type_to_pattern(&item.debt_type, &item.message);
            let action = self
                .rule_engine
                .write()
                .unwrap()
                .evaluate(&pattern, &context);

            self.process_rule_action(action, item, &pattern, &context)
        });
    }

    /// Filter debt items based on context rules
    fn filter_debt_items(&self, mut metrics: FileMetrics, ast: &Ast, path: &Path) -> FileMetrics {
        if !self.enabled {
            return metrics;
        }

        let file_type = detect_file_type(path);

        if let Ast::Rust(rust_ast) = ast {
            self.process_rust_items(&mut metrics, rust_ast, file_type);
        } else {
            self.process_non_rust_items(&mut metrics, file_type);
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
fn debt_type_to_pattern(debt_type: &DebtType, _message: &str) -> DebtPattern {
    // All debt types are mapped directly to DebtPattern::DebtType
    DebtPattern::DebtType(*debt_type)
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
}
