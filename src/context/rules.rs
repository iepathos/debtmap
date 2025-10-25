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
#[derive(Debug, Clone, PartialEq)]
pub enum DebtPattern {
    /// Match a specific debt type
    DebtType(DebtType),
    /// Match blocking I/O patterns
    BlockingIO,
    /// Match input validation issues
    InputValidation,
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
        self.matches_role(context)
            && self.matches_file_type(context)
            && self.matches_async_status(context)
            && self.matches_framework_and_name(context)
    }

    /// Check if the context matches the role requirement
    fn matches_role(&self, context: &FunctionContext) -> bool {
        match self.role {
            Some(role) => context.role == role,
            None => true,
        }
    }

    /// Check if the context matches the file type requirement
    fn matches_file_type(&self, context: &FunctionContext) -> bool {
        match self.file_type {
            Some(file_type) => context.file_type == file_type,
            None => true,
        }
    }

    /// Check if the context matches the async status requirement
    fn matches_async_status(&self, context: &FunctionContext) -> bool {
        match self.is_async {
            Some(is_async) => context.is_async == is_async,
            None => true,
        }
    }

    /// Check if the context matches both framework pattern and name pattern requirements
    fn matches_framework_and_name(&self, context: &FunctionContext) -> bool {
        Self::matches_framework_pattern_pure(self.framework_pattern, context.framework_pattern)
            && Self::matches_name_pattern_pure(&self.name_pattern, &context.function_name)
    }

    /// Pure function to check framework pattern matching
    fn matches_framework_pattern_pure(
        required: Option<FrameworkPattern>,
        actual: Option<FrameworkPattern>,
    ) -> bool {
        match required {
            Some(pattern) => actual == Some(pattern),
            None => true,
        }
    }

    /// Pure function to check name pattern matching
    fn matches_name_pattern_pure(
        required_pattern: &Option<String>,
        actual_name: &Option<String>,
    ) -> bool {
        match (required_pattern, actual_name) {
            (Some(pattern), Some(name)) => name.contains(pattern),
            (Some(_), None) => false, // Pattern required but name is None
            (None, _) => true,        // No pattern requirement
        }
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
        engine.load_config_rules();
        engine
    }

    /// Load rules from configuration
    fn load_config_rules(&mut self) {
        if let Ok(config) = crate::config::get_config_safe() {
            if let Some(context_config) = config.context {
                for rule_config in context_config.rules {
                    if let Some(rule) = Self::parse_config_rule(rule_config) {
                        self.add_rule(rule);
                    }
                }
            }
        }
    }

    /// Parse a configuration rule into a ContextRule
    fn parse_config_rule(config: crate::config::ContextRuleConfig) -> Option<ContextRule> {
        // Parse pattern
        let pattern = match config.pattern.as_str() {
            "blocking_io" => DebtPattern::BlockingIO,
            "input_validation" => DebtPattern::InputValidation,
            "all" => DebtPattern::All,
            _ => return None, // Unknown pattern
        };

        // Parse action
        let action = match config.action.as_str() {
            "allow" => RuleAction::Allow,
            "skip" => RuleAction::Skip,
            "warn" => RuleAction::Warn,
            "deny" => RuleAction::Deny,
            s if s.starts_with("reduce_severity:") => {
                let n = s
                    .strip_prefix("reduce_severity:")
                    .and_then(|v| v.parse::<i32>().ok())
                    .unwrap_or(1);
                RuleAction::ReduceSeverity(n)
            }
            _ => RuleAction::Deny, // Default to deny
        };

        // Build context matcher
        let mut matcher = ContextMatcher::any();

        if let Some(role_str) = config.context.role {
            matcher.role = Self::parse_role(&role_str);
        }

        if let Some(file_type_str) = config.context.file_type {
            matcher.file_type = Self::parse_file_type(&file_type_str);
        }

        matcher.is_async = config.context.is_async;

        if let Some(framework_str) = config.context.framework_pattern {
            matcher.framework_pattern = Self::parse_framework_pattern(&framework_str);
        }

        matcher.name_pattern = config.context.name_pattern;

        Some(ContextRule {
            pattern,
            context_matcher: matcher,
            action,
            priority: config.priority,
            reason: config.reason,
        })
    }

    fn parse_role(role: &str) -> Option<FunctionRole> {
        match role.to_lowercase().as_str() {
            "main" => Some(FunctionRole::Main),
            "config_loader" | "configloader" => Some(FunctionRole::ConfigLoader),
            "test" | "test_function" => Some(FunctionRole::TestFunction),
            "handler" => Some(FunctionRole::Handler),
            "initialization" | "init" => Some(FunctionRole::Initialization),
            "utility" | "util" => Some(FunctionRole::Utility),
            "build_script" | "build" => Some(FunctionRole::BuildScript),
            "example" => Some(FunctionRole::Example),
            "debug" | "diagnostic" => Some(FunctionRole::Debug),
            _ => None,
        }
    }

    fn parse_file_type(file_type: &str) -> Option<FileType> {
        match file_type.to_lowercase().as_str() {
            "production" | "prod" => Some(FileType::Production),
            "test" => Some(FileType::Test),
            "benchmark" | "bench" => Some(FileType::Benchmark),
            "example" => Some(FileType::Example),
            "build_script" | "build" => Some(FileType::BuildScript),
            "documentation" | "doc" => Some(FileType::Documentation),
            "configuration" | "config" => Some(FileType::Configuration),
            _ => None,
        }
    }

    fn parse_framework_pattern(pattern: &str) -> Option<FrameworkPattern> {
        match pattern.to_lowercase().as_str() {
            "rust_main" => Some(FrameworkPattern::RustMain),
            "python_main" => Some(FrameworkPattern::PythonMain),
            "web_handler" => Some(FrameworkPattern::WebHandler),
            "cli_handler" => Some(FrameworkPattern::CliHandler),
            "test_framework" => Some(FrameworkPattern::TestFramework),
            "async_runtime" => Some(FrameworkPattern::AsyncRuntime),
            "config_init" => Some(FrameworkPattern::ConfigInit),
            _ => None,
        }
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

        // Input validation with literals in test functions should be allowed
        self.add_rule(ContextRule {
            pattern: DebtPattern::InputValidation,
            context_matcher: ContextMatcher::for_role(FunctionRole::TestFunction),
            action: RuleAction::Allow,
            priority: 85,
            reason: Some("Test functions use literal strings for test cases".to_string()),
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
    }

    #[test]
    fn test_custom_rules() {
        let mut engine = ContextRuleEngine::new();

        // Add a custom rule for testing using InputValidation instead
        engine.add_rule(ContextRule {
            pattern: DebtPattern::InputValidation,
            context_matcher: ContextMatcher {
                role: None,
                file_type: None,
                is_async: None,
                framework_pattern: None,
                name_pattern: Some("benchmark".to_string()),
            },
            action: RuleAction::Skip,
            priority: 200,
            reason: Some("Benchmarks are test contexts".to_string()),
        });

        // Test the custom rule
        let benchmark_context =
            FunctionContext::new().with_function_name("run_benchmark".to_string());
        let action = engine.evaluate(&DebtPattern::InputValidation, &benchmark_context);
        assert_eq!(action, RuleAction::Skip);
    }

    #[test]
    fn test_context_matcher_pure_functions() {
        use super::ContextMatcher;

        // Test matches_framework_pattern_pure
        assert!(ContextMatcher::matches_framework_pattern_pure(None, None));
        assert!(ContextMatcher::matches_framework_pattern_pure(
            None,
            Some(FrameworkPattern::WebHandler)
        ));
        assert!(ContextMatcher::matches_framework_pattern_pure(
            Some(FrameworkPattern::WebHandler),
            Some(FrameworkPattern::WebHandler)
        ));
        assert!(!ContextMatcher::matches_framework_pattern_pure(
            Some(FrameworkPattern::WebHandler),
            Some(FrameworkPattern::ConfigInit)
        ));
        assert!(!ContextMatcher::matches_framework_pattern_pure(
            Some(FrameworkPattern::WebHandler),
            None
        ));

        // Test matches_name_pattern_pure
        assert!(ContextMatcher::matches_name_pattern_pure(&None, &None));
        assert!(ContextMatcher::matches_name_pattern_pure(
            &None,
            &Some("any_name".to_string())
        ));
        assert!(ContextMatcher::matches_name_pattern_pure(
            &Some("test".to_string()),
            &Some("test_function".to_string())
        ));
        assert!(!ContextMatcher::matches_name_pattern_pure(
            &Some("test".to_string()),
            &Some("production_function".to_string())
        ));
        assert!(!ContextMatcher::matches_name_pattern_pure(
            &Some("test".to_string()),
            &None
        ));
    }

    #[test]
    fn test_context_matcher_comprehensive_matching() {
        // Test complex matching scenarios with multiple criteria
        let complex_matcher = ContextMatcher {
            role: Some(FunctionRole::TestFunction),
            file_type: Some(FileType::Test),
            is_async: Some(true),
            framework_pattern: Some(FrameworkPattern::WebHandler),
            name_pattern: Some("api_test".to_string()),
        };

        // Should match all criteria
        let matching_context = FunctionContext {
            role: FunctionRole::TestFunction,
            file_type: FileType::Test,
            is_async: true,
            framework_pattern: Some(FrameworkPattern::WebHandler),
            function_name: Some("test_api_test_endpoint".to_string()),
            module_path: Vec::new(),
        };
        assert!(complex_matcher.matches(&matching_context));

        // Should fail on role mismatch
        let role_mismatch = FunctionContext {
            role: FunctionRole::Main,
            file_type: FileType::Test,
            is_async: true,
            framework_pattern: Some(FrameworkPattern::WebHandler),
            function_name: Some("test_api_test_endpoint".to_string()),
            module_path: Vec::new(),
        };
        assert!(!complex_matcher.matches(&role_mismatch));

        // Should fail on name pattern mismatch
        let name_mismatch = FunctionContext {
            role: FunctionRole::TestFunction,
            file_type: FileType::Test,
            is_async: true,
            framework_pattern: Some(FrameworkPattern::WebHandler),
            function_name: Some("test_database_function".to_string()),
            module_path: Vec::new(),
        };
        assert!(!complex_matcher.matches(&name_mismatch));

        // Should fail on missing function name when pattern required
        let missing_name = FunctionContext {
            role: FunctionRole::TestFunction,
            file_type: FileType::Test,
            is_async: true,
            framework_pattern: Some(FrameworkPattern::WebHandler),
            function_name: None,
            module_path: Vec::new(),
        };
        assert!(!complex_matcher.matches(&missing_name));
    }
}
