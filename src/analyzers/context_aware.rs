//! Context-aware analyzer wrapper that integrates context detection

use crate::analyzers::Analyzer;
use crate::context::rules::DebtPattern;
use crate::context::{
    detect_file_type, ContextDetector, ContextRuleEngine, FunctionContext, RuleAction,
};
use crate::core::{ast::Ast, DebtItem, DebtType, FileMetrics, Language, Priority};
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

    /// Process a single debt item based on rule action
    fn process_debt_item(
        &self,
        item: &mut DebtItem,
        action: RuleAction,
        pattern: &DebtPattern,
        context: &FunctionContext,
    ) -> bool {
        match action {
            RuleAction::Allow | RuleAction::Skip => false,
            RuleAction::Warn => {
                self.apply_severity_reduction(item, -2, pattern, context);
                true
            }
            RuleAction::ReduceSeverity(n) => {
                self.apply_severity_reduction(item, -n, pattern, context);
                true
            }
            RuleAction::Deny => true,
        }
    }

    /// Apply severity reduction and add context reason
    fn apply_severity_reduction(
        &self,
        item: &mut DebtItem,
        reduction: i32,
        pattern: &DebtPattern,
        context: &FunctionContext,
    ) {
        item.priority = adjust_priority(item.priority, reduction);

        if let Some(reason) = self
            .rule_engine
            .write()
            .unwrap()
            .get_reason(pattern, context)
        {
            item.message = format!("{} (Context: {})", item.message, reason);
        }
    }

    /// Filter debt items based on context rules
    fn filter_debt_items(&self, mut metrics: FileMetrics, ast: &Ast, path: &Path) -> FileMetrics {
        if !self.enabled {
            return metrics;
        }

        let file_type = detect_file_type(path);

        if let Ast::Rust(rust_ast) = ast {
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

                self.process_debt_item(item, action, &pattern, &context)
            });
        } else {
            let context = FunctionContext::new().with_file_type(file_type);

            metrics.debt_items.retain_mut(|item| {
                let pattern = debt_type_to_pattern(&item.debt_type, &item.message);
                let action = self
                    .rule_engine
                    .write()
                    .unwrap()
                    .evaluate(&pattern, &context);

                self.process_debt_item(item, action, &pattern, &context)
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
    use crate::core::{ComplexityMetrics, DebtItem};
    use std::path::PathBuf;

    #[test]
    fn test_adjust_priority() {
        assert_eq!(adjust_priority(Priority::High, -1), Priority::Medium);
        assert_eq!(adjust_priority(Priority::Medium, -2), Priority::Low);
        assert_eq!(adjust_priority(Priority::Low, -1), Priority::Low);
        assert_eq!(adjust_priority(Priority::Critical, -999), Priority::Low);
    }

    #[test]
    fn test_adjust_priority_edge_cases() {
        // Test positive adjustments (should be clamped)
        assert_eq!(adjust_priority(Priority::High, 1), Priority::Critical);
        assert_eq!(adjust_priority(Priority::Critical, 1), Priority::Critical);

        // Test extreme negative adjustments
        assert_eq!(adjust_priority(Priority::Critical, -10), Priority::Low);

        // Test no adjustment
        assert_eq!(adjust_priority(Priority::Medium, 0), Priority::Medium);
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
    fn test_process_debt_item_allow() {
        let analyzer = create_test_analyzer();
        let mut item = create_test_debt_item();
        let pattern = DebtPattern::DebtType(DebtType::Todo);
        let context = FunctionContext::new();

        // RuleAction::Allow should filter out the item
        let keep = analyzer.process_debt_item(&mut item, RuleAction::Allow, &pattern, &context);
        assert!(!keep, "Allow action should filter out the item");
    }

    #[test]
    fn test_process_debt_item_skip() {
        let analyzer = create_test_analyzer();
        let mut item = create_test_debt_item();
        let pattern = DebtPattern::DebtType(DebtType::Todo);
        let context = FunctionContext::new();

        // RuleAction::Skip should filter out the item
        let keep = analyzer.process_debt_item(&mut item, RuleAction::Skip, &pattern, &context);
        assert!(!keep, "Skip action should filter out the item");
    }

    #[test]
    fn test_process_debt_item_warn() {
        let analyzer = create_test_analyzer();
        let mut item = create_test_debt_item();
        let original_priority = item.priority;
        let pattern = DebtPattern::DebtType(DebtType::Todo);
        let context = FunctionContext::new();

        // RuleAction::Warn should reduce severity by 2 and keep the item
        let keep = analyzer.process_debt_item(&mut item, RuleAction::Warn, &pattern, &context);
        assert!(keep, "Warn action should keep the item");
        assert_eq!(
            item.priority,
            adjust_priority(original_priority, -2),
            "Priority should be reduced by 2"
        );
    }

    #[test]
    fn test_process_debt_item_reduce_severity() {
        let analyzer = create_test_analyzer();
        let mut item = create_test_debt_item();
        let original_priority = item.priority;
        let pattern = DebtPattern::DebtType(DebtType::Todo);
        let context = FunctionContext::new();

        // RuleAction::ReduceSeverity should reduce by specified amount
        let keep = analyzer.process_debt_item(
            &mut item,
            RuleAction::ReduceSeverity(3),
            &pattern,
            &context,
        );
        assert!(keep, "ReduceSeverity action should keep the item");
        assert_eq!(
            item.priority,
            adjust_priority(original_priority, -3),
            "Priority should be reduced by 3"
        );
    }

    #[test]
    fn test_process_debt_item_deny() {
        let analyzer = create_test_analyzer();
        let mut item = create_test_debt_item();
        let original_priority = item.priority;
        let pattern = DebtPattern::DebtType(DebtType::Todo);
        let context = FunctionContext::new();

        // RuleAction::Deny should keep the item unchanged
        let keep = analyzer.process_debt_item(&mut item, RuleAction::Deny, &pattern, &context);
        assert!(keep, "Deny action should keep the item");
        assert_eq!(
            item.priority, original_priority,
            "Priority should be unchanged"
        );
    }

    #[test]
    fn test_apply_severity_reduction() {
        let analyzer = create_test_analyzer();
        let mut item = create_test_debt_item();
        let pattern = DebtPattern::DebtType(DebtType::Todo);
        let context = FunctionContext::new();

        // Test severity reduction
        analyzer.apply_severity_reduction(&mut item, -2, &pattern, &context);
        assert_eq!(
            item.priority,
            Priority::Low,
            "Priority should be reduced to Low"
        );
    }

    #[test]
    fn test_filter_debt_items_disabled() {
        let mut analyzer = create_test_analyzer();
        analyzer.set_enabled(false);

        let metrics = create_test_metrics();
        let ast = Ast::Unknown;
        let path = Path::new("test.rs");

        let result = analyzer.filter_debt_items(metrics.clone(), &ast, path);
        assert_eq!(
            result.debt_items.len(),
            metrics.debt_items.len(),
            "Should not filter when disabled"
        );
    }

    // Helper functions for testing
    fn create_test_analyzer() -> ContextAwareAnalyzer {
        struct MockAnalyzer;
        impl Analyzer for MockAnalyzer {
            fn parse(&self, _content: &str, _path: PathBuf) -> Result<Ast> {
                Ok(Ast::Unknown)
            }
            fn analyze(&self, _ast: &Ast) -> FileMetrics {
                create_test_metrics()
            }
            fn language(&self) -> Language {
                Language::Rust
            }
        }

        ContextAwareAnalyzer::new(Box::new(MockAnalyzer))
    }

    fn create_test_debt_item() -> DebtItem {
        DebtItem {
            id: "test-item".to_string(),
            debt_type: DebtType::Todo,
            message: "Test debt item".to_string(),
            line: 42,
            column: Some(0),
            priority: Priority::Medium,
            file: PathBuf::from("test.rs"),
            context: None,
        }
    }

    fn create_test_metrics() -> FileMetrics {
        FileMetrics {
            path: PathBuf::from("test.rs"),
            language: Language::Rust,
            complexity: ComplexityMetrics::default(),
            debt_items: vec![create_test_debt_item()],
            dependencies: vec![],
            duplications: vec![],
        }
    }
}
