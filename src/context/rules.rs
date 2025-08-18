//! Context-aware rules engine for debt detection

use crate::context::{FileType, FrameworkPattern, FunctionContext, FunctionRole};
use crate::core::DebtType;
use std::collections::HashMap;

/// Represents a context-aware rule for debt detection
#[derive(Debug, Clone)]
pub struct ContextRule {
    /// Pattern to match (debt type or specific pattern)
    pub pattern: DebtPattern,
    /// Context where this rule applies
    pub context_matcher: ContextMatcher,
    /// Action to take when rule matches
    pub action: RuleAction,
    /// Priority of this rule (higher = takes precedence)
    pub priority: i32,
    /// Optional reason for the rule
    pub reason: Option<String>,
}

/// Pattern that a rule matches against
#[derive(Debug, Clone)]
pub enum DebtPattern {
    /// Match a specific debt type
    DebtType(DebtType),
    /// Match blocking I/O patterns
    BlockingIO,
    /// Match input validation issues
    InputValidation,
    /// Match security issues
    Security,
    /// Match performance issues
    Performance,
    /// Match all patterns
    All,
}

/// Matcher for function context
#[derive(Debug, Clone)]
pub struct ContextMatcher {
    /// Required function role (None = any)
    pub role: Option<FunctionRole>,
    /// Required file type (None = any)
    pub file_type: Option<FileType>,
    /// Required async status (None = any)
    pub is_async: Option<bool>,
    /// Required framework pattern (None = any)
    pub framework_pattern: Option<FrameworkPattern>,
    /// Function name pattern (regex)
    pub name_pattern: Option<String>,
}

impl ContextMatcher {
    /// Create a matcher that matches any context
    pub fn any() -> Self {
        Self {
            role: None,
            file_type: None,
            is_async: None,
            framework_pattern: None,
            name_pattern: None,
        }
    }

    /// Create a matcher for a specific role
    pub fn for_role(role: FunctionRole) -> Self {
        Self {
            role: Some(role),
            file_type: None,
            is_async: None,
            framework_pattern: None,
            name_pattern: None,
        }
    }

    /// Create a matcher for a specific file type
    pub fn for_file_type(file_type: FileType) -> Self {
        Self {
            role: None,
            file_type: Some(file_type),
            is_async: None,
            framework_pattern: None,
            name_pattern: None,
        }
    }

    /// Check if this matcher matches the given context
    pub fn matches(&self, context: &FunctionContext) -> bool {
        // Check role
        if let Some(role) = self.role {
            if context.role != role {
                return false;
            }
        }

        // Check file type
        if let Some(file_type) = self.file_type {
            if context.file_type != file_type {
                return false;
            }
        }

        // Check async status
        if let Some(is_async) = self.is_async {
            if context.is_async != is_async {
                return false;
            }
        }

        // Check framework pattern
        if let Some(pattern) = self.framework_pattern {
            if context.framework_pattern != Some(pattern) {
                return false;
            }
        }

        // Check name pattern
        if let Some(ref pattern) = self.name_pattern {
            if let Some(ref name) = context.function_name {
                if !name.contains(pattern) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

/// Action to take when a rule matches
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAction {
    /// Allow the pattern (not technical debt in this context)
    Allow,
    /// Warn about the pattern (reduced severity)
    Warn,
    /// Deny the pattern (flag as debt - default behavior)
    Deny,
    /// Skip analysis entirely
    Skip,
    /// Reduce severity by N levels
    ReduceSeverity(i32),
}

/// Context-aware rules engine
pub struct ContextRuleEngine {
    /// All registered rules
    rules: Vec<ContextRule>,
    /// Cache of rule evaluations
    cache: HashMap<(String, String), RuleAction>,
}

impl ContextRuleEngine {
    /// Create a new rules engine with default rules
    pub fn new() -> Self {
        let mut engine = Self {
            rules: Vec::new(),
            cache: HashMap::new(),
        };
        engine.load_default_rules();
        engine
    }

    /// Load default context-aware rules
    fn load_default_rules(&mut self) {
        // Blocking I/O is allowed in main functions
        self.add_rule(ContextRule {
            pattern: DebtPattern::BlockingIO,
            context_matcher: ContextMatcher::for_role(FunctionRole::Main),
            action: RuleAction::Allow,
            priority: 100,
            reason: Some("Blocking I/O is acceptable in main functions".to_string()),
        });

        // Blocking I/O is allowed in config loaders
        self.add_rule(ContextRule {
            pattern: DebtPattern::BlockingIO,
            context_matcher: ContextMatcher::for_role(FunctionRole::ConfigLoader),
            action: RuleAction::Allow,
            priority: 100,
            reason: Some("Config loading typically happens at startup".to_string()),
        });

        // Blocking I/O is allowed in test functions
        self.add_rule(ContextRule {
            pattern: DebtPattern::BlockingIO,
            context_matcher: ContextMatcher::for_role(FunctionRole::TestFunction),
            action: RuleAction::Allow,
            priority: 90,
            reason: Some("Test simplicity is more important than async performance".to_string()),
        });

        // Blocking I/O is allowed in initialization
        self.add_rule(ContextRule {
            pattern: DebtPattern::BlockingIO,
            context_matcher: ContextMatcher::for_role(FunctionRole::Initialization),
            action: RuleAction::Allow,
            priority: 90,
            reason: Some("Initialization typically happens before async runtime".to_string()),
        });

        // Input validation is less critical in test code
        self.add_rule(ContextRule {
            pattern: DebtPattern::InputValidation,
            context_matcher: ContextMatcher::for_file_type(FileType::Test),
            action: RuleAction::ReduceSeverity(2),
            priority: 80,
            reason: Some("Test code often uses hardcoded inputs".to_string()),
        });

        // Security checks can be skipped in test files
        self.add_rule(ContextRule {
            pattern: DebtPattern::Security,
            context_matcher: ContextMatcher::for_file_type(FileType::Test),
            action: RuleAction::Skip,
            priority: 90,
            reason: Some("Security is not a concern in test code".to_string()),
        });

        // Security checks can be skipped in examples
        self.add_rule(ContextRule {
            pattern: DebtPattern::Security,
            context_matcher: ContextMatcher::for_file_type(FileType::Example),
            action: RuleAction::Skip,
            priority: 90,
            reason: Some("Examples often demonstrate concepts without security".to_string()),
        });

        // Performance is less critical in test code
        self.add_rule(ContextRule {
            pattern: DebtPattern::Performance,
            context_matcher: ContextMatcher::for_file_type(FileType::Test),
            action: RuleAction::ReduceSeverity(2),
            priority: 70,
            reason: Some("Test performance is less critical than correctness".to_string()),
        });

        // Performance is less critical in benchmarks (they measure it)
        self.add_rule(ContextRule {
            pattern: DebtPattern::Performance,
            context_matcher: ContextMatcher::for_file_type(FileType::Benchmark),
            action: RuleAction::Warn,
            priority: 70,
            reason: Some("Benchmarks intentionally test performance patterns".to_string()),
        });

        // Build scripts have different constraints
        self.add_rule(ContextRule {
            pattern: DebtPattern::All,
            context_matcher: ContextMatcher::for_file_type(FileType::BuildScript),
            action: RuleAction::ReduceSeverity(1),
            priority: 60,
            reason: Some(
                "Build scripts run at compile time with different constraints".to_string(),
            ),
        });
    }

    /// Add a custom rule
    pub fn add_rule(&mut self, rule: ContextRule) {
        self.rules.push(rule);
        // Sort by priority (highest first)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        // Clear cache when rules change
        self.cache.clear();
    }

    /// Evaluate a debt pattern in a given context
    pub fn evaluate(&mut self, pattern: &DebtPattern, context: &FunctionContext) -> RuleAction {
        // Check cache
        let cache_key = (format!("{:?}", pattern), format!("{:?}", context));
        if let Some(&action) = self.cache.get(&cache_key) {
            return action;
        }

        // Find the highest priority matching rule
        let action = self
            .rules
            .iter()
            .filter(|rule| self.pattern_matches(&rule.pattern, pattern))
            .filter(|rule| rule.context_matcher.matches(context))
            .map(|rule| rule.action)
            .next()
            .unwrap_or(RuleAction::Deny);

        // Cache the result
        self.cache.insert(cache_key, action);
        action
    }

    /// Check if a rule pattern matches a debt pattern
    fn pattern_matches(&self, rule_pattern: &DebtPattern, debt_pattern: &DebtPattern) -> bool {
        match (rule_pattern, debt_pattern) {
            (DebtPattern::All, _) => true,
            (DebtPattern::DebtType(rule_type), DebtPattern::DebtType(debt_type)) => {
                rule_type == debt_type
            }
            (DebtPattern::BlockingIO, DebtPattern::BlockingIO) => true,
            (DebtPattern::InputValidation, DebtPattern::InputValidation) => true,
            (DebtPattern::Security, DebtPattern::Security) => true,
            (DebtPattern::Performance, DebtPattern::Performance) => true,
            _ => false,
        }
    }

    /// Get the reason for a rule action
    pub fn get_reason(&self, pattern: &DebtPattern, context: &FunctionContext) -> Option<String> {
        self.rules
            .iter()
            .filter(|rule| self.pattern_matches(&rule.pattern, pattern))
            .filter(|rule| rule.context_matcher.matches(context))
            .find_map(|rule| rule.reason.clone())
    }

    /// Check if a debt type should be analyzed in a context
    pub fn should_analyze(&mut self, debt_type: &DebtType, context: &FunctionContext) -> bool {
        let pattern = DebtPattern::DebtType(*debt_type);
        let action = self.evaluate(&pattern, context);
        action != RuleAction::Skip
    }

    /// Get severity adjustment for a debt type in a context
    pub fn get_severity_adjustment(
        &mut self,
        debt_type: &DebtType,
        context: &FunctionContext,
    ) -> i32 {
        let pattern = DebtPattern::DebtType(*debt_type);
        match self.evaluate(&pattern, context) {
            RuleAction::Allow => -999, // Effectively disable
            RuleAction::Warn => -2,
            RuleAction::ReduceSeverity(n) => -n,
            RuleAction::Deny => 0,
            RuleAction::Skip => 0,
        }
    }
}

impl Default for ContextRuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_matcher() {
        let context = FunctionContext::new()
            .with_role(FunctionRole::Main)
            .with_file_type(FileType::Production);

        let matcher = ContextMatcher::for_role(FunctionRole::Main);
        assert!(matcher.matches(&context));

        let matcher = ContextMatcher::for_role(FunctionRole::TestFunction);
        assert!(!matcher.matches(&context));

        let matcher = ContextMatcher::for_file_type(FileType::Production);
        assert!(matcher.matches(&context));
    }

    #[test]
    fn test_rules_engine() {
        let mut engine = ContextRuleEngine::new();

        // Test blocking I/O in main function
        let main_context = FunctionContext::new().with_role(FunctionRole::Main);
        let action = engine.evaluate(&DebtPattern::BlockingIO, &main_context);
        assert_eq!(action, RuleAction::Allow);

        // Test blocking I/O in regular function
        let regular_context = FunctionContext::new().with_role(FunctionRole::Unknown);
        let action = engine.evaluate(&DebtPattern::BlockingIO, &regular_context);
        assert_eq!(action, RuleAction::Deny);

        // Test security in test file
        let test_context = FunctionContext::new().with_file_type(FileType::Test);
        let action = engine.evaluate(&DebtPattern::Security, &test_context);
        assert_eq!(action, RuleAction::Skip);
    }

    #[test]
    fn test_custom_rules() {
        let mut engine = ContextRuleEngine::new();

        // Add a custom rule
        engine.add_rule(ContextRule {
            pattern: DebtPattern::Performance,
            context_matcher: ContextMatcher {
                role: None,
                file_type: None,
                is_async: None,
                framework_pattern: None,
                name_pattern: Some("benchmark".to_string()),
            },
            action: RuleAction::Skip,
            priority: 200,
            reason: Some("Benchmarks measure performance".to_string()),
        });

        // Test the custom rule
        let benchmark_context =
            FunctionContext::new().with_function_name("run_benchmark".to_string());
        let action = engine.evaluate(&DebtPattern::Performance, &benchmark_context);
        assert_eq!(action, RuleAction::Skip);
    }
}
