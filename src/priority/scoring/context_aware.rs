//! Context-aware recommendations for specialized code patterns
//!
//! This module provides tailored recommendations based on the detected context
//! of a function (formatter, parser, CLI handler, etc.).

use crate::analysis::FunctionContext;
use crate::core::FunctionMetrics;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Moderate,
    High,
    Critical,
}

impl Severity {
    pub fn display_name(&self) -> &'static str {
        match self {
            Severity::Low => "LOW",
            Severity::Moderate => "MODERATE",
            Severity::High => "HIGH",
            Severity::Critical => "CRITICAL",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualRecommendation {
    pub context: FunctionContext,
    pub explanation: String,
    pub suggestions: Vec<String>,
    pub patterns: Vec<String>,
    pub examples: Vec<String>,
    pub severity: Severity,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
struct RecommendationTemplate {
    explanation: String,
    suggestions: Vec<String>,
    patterns: Vec<String>,
    examples: Vec<String>,
    severity_adjustment: f64,
}

pub struct ContextRecommendationEngine {
    templates: HashMap<FunctionContext, RecommendationTemplate>,
}

impl Default for ContextRecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextRecommendationEngine {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        templates.insert(
            FunctionContext::Formatter,
            RecommendationTemplate {
                explanation: "This is output formatting code. High cyclomatic complexity is \
                             typical for formatters with many output variants. Focus on \
                             cognitive complexity and consider builder pattern if deeply nested."
                    .to_string(),
                suggestions: vec![
                    "If complexity is from nesting (not just cases), consider builder pattern"
                        .to_string(),
                    "Extract format helpers for repeated patterns".to_string(),
                    "Use match expressions for clean exhaustive formatting".to_string(),
                ],
                patterns: vec![
                    "Builder pattern for complex output".to_string(),
                    "Template method for format variants".to_string(),
                ],
                examples: vec![
                    "colored::Colorize for terminal formatting".to_string(),
                    "serde for structured output".to_string(),
                ],
                severity_adjustment: 0.6, // Lower severity for formatters
            },
        );

        templates.insert(
            FunctionContext::Parser,
            RecommendationTemplate {
                explanation: "This is parsing code. Consider using parser combinators or \
                             grammar-based approaches for better maintainability and clearer intent."
                    .to_string(),
                suggestions: vec![
                    "Use parser combinator library (nom, pest, combine)".to_string(),
                    "Define grammar separately from parsing logic".to_string(),
                    "Break parsing into lexing + parsing phases".to_string(),
                ],
                patterns: vec![
                    "Parser combinators for composable parsing".to_string(),
                    "Recursive descent for simple grammars".to_string(),
                ],
                examples: vec![
                    "nom for binary/text parsing".to_string(),
                    "pest for grammar-based parsing".to_string(),
                    "serde for structured data".to_string(),
                ],
                severity_adjustment: 0.8,
            },
        );

        templates.insert(
            FunctionContext::CliHandler,
            RecommendationTemplate {
                explanation: "This is a CLI command handler. Orchestration naturally involves \
                             multiple branches. Consider command pattern or dispatch table for cleaner structure."
                    .to_string(),
                suggestions: vec![
                    "Use command pattern with trait-based dispatch".to_string(),
                    "Extract validation, execution, and output into separate functions".to_string(),
                    "Consider dispatch table for subcommand routing".to_string(),
                ],
                patterns: vec![
                    "Command pattern for each subcommand".to_string(),
                    "Strategy pattern for different output formats".to_string(),
                ],
                examples: vec![
                    "clap::derive for arg parsing".to_string(),
                    "Trait-based command dispatch".to_string(),
                ],
                severity_adjustment: 0.7,
            },
        );

        templates.insert(
            FunctionContext::StateMachine,
            RecommendationTemplate {
                explanation: "This appears to be state machine logic. Exhaustive state \
                             handling results in high complexity. Consider state pattern or state machine library."
                    .to_string(),
                suggestions: vec![
                    "Use state machine library (rust-fsm, machine)".to_string(),
                    "Implement state pattern with trait-based states".to_string(),
                    "Extract transition logic into separate functions".to_string(),
                ],
                patterns: vec![
                    "State pattern for cleaner transitions".to_string(),
                    "Type-state pattern for compile-time validation".to_string(),
                ],
                examples: vec![
                    "rust-fsm for simple state machines".to_string(),
                    "Type-state pattern for API design".to_string(),
                ],
                severity_adjustment: 0.75,
            },
        );

        templates.insert(
            FunctionContext::Configuration,
            RecommendationTemplate {
                explanation: "This is configuration code. Complex validation and defaults can \
                             increase complexity, but this is often acceptable."
                    .to_string(),
                suggestions: vec![
                    "Use builder pattern for complex configuration".to_string(),
                    "Extract validation into separate functions".to_string(),
                    "Consider using serde defaults for simpler code".to_string(),
                ],
                patterns: vec![
                    "Builder pattern for configuration".to_string(),
                    "Validation layer pattern".to_string(),
                ],
                examples: vec![
                    "serde with derive for config parsing".to_string(),
                    "Builder pattern with type-state".to_string(),
                ],
                severity_adjustment: 0.7,
            },
        );

        templates.insert(
            FunctionContext::Validator,
            RecommendationTemplate {
                explanation: "This is validation code. Multiple validation rules naturally \
                             increase complexity."
                    .to_string(),
                suggestions: vec![
                    "Extract individual validation rules into separate functions".to_string(),
                    "Use validation combinator pattern".to_string(),
                    "Consider declarative validation if complexity > 20".to_string(),
                ],
                patterns: vec![
                    "Combinator pattern for composable validation".to_string(),
                    "Chain of responsibility for validation rules".to_string(),
                ],
                examples: vec![
                    "validator crate for declarative validation".to_string(),
                    "Custom validation trait for composability".to_string(),
                ],
                severity_adjustment: 0.75,
            },
        );

        templates.insert(
            FunctionContext::DatabaseQuery,
            RecommendationTemplate {
                explanation: "This is database query code. Complex queries and result mapping \
                             can be simplified with query builders."
                    .to_string(),
                suggestions: vec![
                    "Use query builder for complex queries".to_string(),
                    "Extract result mapping into separate functions".to_string(),
                    "Consider ORM for complex data access patterns".to_string(),
                ],
                patterns: vec![
                    "Repository pattern for data access".to_string(),
                    "Query object pattern".to_string(),
                ],
                examples: vec![
                    "sqlx for type-safe SQL queries".to_string(),
                    "diesel for ORM-based access".to_string(),
                ],
                severity_adjustment: 0.7,
            },
        );

        templates.insert(
            FunctionContext::TestHelper,
            RecommendationTemplate {
                explanation: "This is test helper code. Complexity in test helpers can make tests \
                             harder to understand."
                    .to_string(),
                suggestions: vec![
                    "Break complex test helpers into smaller utilities".to_string(),
                    "Use builder pattern for test data creation".to_string(),
                    "Consider property-based testing for complex scenarios".to_string(),
                ],
                patterns: vec![
                    "Builder pattern for test data".to_string(),
                    "Test fixture pattern".to_string(),
                ],
                examples: vec![
                    "proptest for property-based testing".to_string(),
                    "Builder pattern for test objects".to_string(),
                ],
                severity_adjustment: 0.8,
            },
        );

        templates.insert(
            FunctionContext::Generic,
            RecommendationTemplate {
                explanation: "This function has high complexity that should be reduced."
                    .to_string(),
                suggestions: vec![
                    "Extract pure functions from complex logic".to_string(),
                    "Reduce nesting depth with early returns".to_string(),
                    "Break into smaller, testable functions".to_string(),
                ],
                patterns: vec![],
                examples: vec![],
                severity_adjustment: 1.0, // No adjustment
            },
        );

        Self { templates }
    }

    pub fn generate_recommendation(
        &self,
        function: &FunctionMetrics,
        context: FunctionContext,
        confidence: f64,
        base_score: f64,
    ) -> ContextualRecommendation {
        let template = self
            .templates
            .get(&context)
            .unwrap_or_else(|| self.templates.get(&FunctionContext::Generic).unwrap());

        let adjusted_severity = self.adjust_severity(base_score, template.severity_adjustment);
        let explanation = self.customize_explanation(template, function, &context);

        ContextualRecommendation {
            context,
            explanation,
            suggestions: template.suggestions.clone(),
            patterns: template.patterns.clone(),
            examples: template.examples.clone(),
            severity: adjusted_severity,
            confidence,
        }
    }

    fn adjust_severity(&self, base_score: f64, adjustment: f64) -> Severity {
        let adjusted_score = base_score * adjustment;

        if adjusted_score > 20.0 {
            Severity::Critical
        } else if adjusted_score > 12.0 {
            Severity::High
        } else if adjusted_score > 7.0 {
            Severity::Moderate
        } else {
            Severity::Low
        }
    }

    fn customize_explanation(
        &self,
        template: &RecommendationTemplate,
        function: &FunctionMetrics,
        context: &FunctionContext,
    ) -> String {
        // Add context-specific details to explanation
        let base = &template.explanation;

        match context {
            FunctionContext::Formatter if function.cognitive < 5 && function.cyclomatic > 10 => {
                format!(
                    "{} Note: Low cognitive complexity ({}) with high cyclomatic ({}) \
                     suggests pattern-heavy code, which is appropriate for formatters.",
                    base, function.cognitive, function.cyclomatic
                )
            }
            FunctionContext::Parser if function.cyclomatic > 20 => {
                format!(
                    "{} With cyclomatic complexity of {}, a parser combinator library \
                     would significantly improve maintainability.",
                    base, function.cyclomatic
                )
            }
            _ => base.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function(name: &str, cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic,
            cognitive,
            nesting: 2,
            length: 50,
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
        }
    }

    #[test]
    fn adjusts_severity_for_formatters() {
        let engine = ContextRecommendationEngine::new();
        let function = create_test_function("format_output", 15, 3);

        // Base score of 15 * 0.6 adjustment = 9.0 (Moderate)
        let recommendation =
            engine.generate_recommendation(&function, FunctionContext::Formatter, 0.85, 15.0);

        assert_eq!(recommendation.severity, Severity::Moderate);
        assert_eq!(recommendation.context, FunctionContext::Formatter);
        assert!(!recommendation.suggestions.is_empty());
    }

    #[test]
    fn provides_parser_recommendations() {
        let engine = ContextRecommendationEngine::new();
        let function = create_test_function("parse_input", 20, 18);

        let recommendation =
            engine.generate_recommendation(&function, FunctionContext::Parser, 0.9, 20.0);

        assert_eq!(recommendation.context, FunctionContext::Parser);
        assert!(recommendation.explanation.contains("parser"));
        assert!(recommendation
            .suggestions
            .iter()
            .any(|s| s.contains("combinator")));
        assert!(!recommendation.examples.is_empty());
    }

    #[test]
    fn provides_cli_handler_recommendations() {
        let engine = ContextRecommendationEngine::new();
        let function = create_test_function("handle_command", 12, 10);

        let recommendation =
            engine.generate_recommendation(&function, FunctionContext::CliHandler, 0.8, 12.0);

        assert_eq!(recommendation.context, FunctionContext::CliHandler);
        assert!(recommendation.explanation.contains("CLI"));
        assert!(recommendation
            .suggestions
            .iter()
            .any(|s| s.contains("command pattern")));
    }

    #[test]
    fn generic_context_no_adjustment() {
        let engine = ContextRecommendationEngine::new();
        let function = create_test_function("complex_logic", 15, 20);

        let recommendation =
            engine.generate_recommendation(&function, FunctionContext::Generic, 0.1, 15.0);

        assert_eq!(recommendation.context, FunctionContext::Generic);
        assert_eq!(recommendation.severity, Severity::High); // No adjustment, 15.0 > 12.0
    }

    #[test]
    fn state_machine_recommendations() {
        let engine = ContextRecommendationEngine::new();
        let function = create_test_function("transition_state", 18, 15);

        let recommendation =
            engine.generate_recommendation(&function, FunctionContext::StateMachine, 0.85, 18.0);

        assert_eq!(recommendation.context, FunctionContext::StateMachine);
        assert!(recommendation.explanation.contains("state"));
        assert!(recommendation
            .suggestions
            .iter()
            .any(|s| s.contains("state machine")));
    }

    #[test]
    fn validator_recommendations() {
        let engine = ContextRecommendationEngine::new();
        let function = create_test_function("validate_config", 14, 12);

        let recommendation =
            engine.generate_recommendation(&function, FunctionContext::Validator, 0.75, 14.0);

        assert_eq!(recommendation.context, FunctionContext::Validator);
        assert!(recommendation.explanation.contains("validation"));
    }

    #[test]
    fn confidence_is_preserved() {
        let engine = ContextRecommendationEngine::new();
        let function = create_test_function("format_output", 10, 5);

        let recommendation =
            engine.generate_recommendation(&function, FunctionContext::Formatter, 0.95, 10.0);

        assert_eq!(recommendation.confidence, 0.95);
    }

    #[test]
    fn all_contexts_have_templates() {
        let engine = ContextRecommendationEngine::new();
        let function = create_test_function("test_func", 10, 8);

        let contexts = vec![
            FunctionContext::Formatter,
            FunctionContext::Parser,
            FunctionContext::CliHandler,
            FunctionContext::StateMachine,
            FunctionContext::Configuration,
            FunctionContext::TestHelper,
            FunctionContext::DatabaseQuery,
            FunctionContext::Validator,
            FunctionContext::Generic,
        ];

        for context in contexts {
            let recommendation = engine.generate_recommendation(&function, context, 0.8, 10.0);
            assert!(!recommendation.explanation.is_empty());
        }
    }
}
