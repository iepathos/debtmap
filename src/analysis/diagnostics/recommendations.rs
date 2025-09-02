use crate::analysis::attribution::ComplexityAttribution;
use crate::analysis::multi_pass::{
    ComplexityRecommendation, RecommendationCategory, RecommendationPriority,
};

/// Engine for generating complexity recommendations
pub struct RecommendationEngine {
    strategies: Vec<Box<dyn RecommendationStrategy>>,
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self {
            strategies: vec![
                Box::new(RefactoringStrategy::new()),
                Box::new(FormattingStrategy::new()),
                Box::new(PatternStrategy::new()),
                Box::new(GeneralStrategy::new()),
            ],
        }
    }
}

impl RecommendationEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn generate_recommendations(
        &self,
        attribution: &ComplexityAttribution,
    ) -> Vec<ComplexityRecommendation> {
        let mut recommendations = Vec::new();

        for strategy in &self.strategies {
            recommendations.extend(strategy.recommend(attribution));
        }

        // Sort by priority
        recommendations.sort_by_key(|r| match r.priority {
            RecommendationPriority::High => 0,
            RecommendationPriority::Medium => 1,
            RecommendationPriority::Low => 2,
        });

        // Limit to top 10 recommendations
        recommendations.truncate(10);

        recommendations
    }
}

/// Trait for recommendation strategies
trait RecommendationStrategy: Send + Sync {
    fn recommend(&self, attribution: &ComplexityAttribution) -> Vec<ComplexityRecommendation>;
}

/// Strategy for refactoring recommendations
struct RefactoringStrategy {
    complexity_threshold: u32,
}

impl RefactoringStrategy {
    fn new() -> Self {
        Self {
            complexity_threshold: 15,
        }
    }
}

impl RecommendationStrategy for RefactoringStrategy {
    fn recommend(&self, attribution: &ComplexityAttribution) -> Vec<ComplexityRecommendation> {
        let mut recommendations = Vec::new();

        // Recommend refactoring for high-complexity components
        for component in &attribution.logical_complexity.breakdown {
            if component.contribution > self.complexity_threshold {
                recommendations.push(ComplexityRecommendation {
                    priority: RecommendationPriority::High,
                    category: RecommendationCategory::Refactoring,
                    title: format!("Refactor {}", component.description),
                    description: format!(
                        "This component contributes {} complexity points, exceeding the threshold of {}",
                        component.contribution, self.complexity_threshold
                    ),
                    estimated_impact: component.contribution / 2,
                    suggested_actions: vec![
                        "Extract complex logic into separate functions".to_string(),
                        "Apply the Single Responsibility Principle".to_string(),
                        "Consider using design patterns to simplify structure".to_string(),
                    ]
                        .into_iter()
                        .chain(component.suggestions.clone())
                        .collect(),
                });
            }
        }

        // General refactoring recommendation if overall complexity is high
        if attribution.logical_complexity.total > 30 {
            recommendations.push(ComplexityRecommendation {
                priority: RecommendationPriority::High,
                category: RecommendationCategory::Refactoring,
                title: "Consider major refactoring".to_string(),
                description: format!(
                    "Total logical complexity ({}) suggests this code needs significant restructuring",
                    attribution.logical_complexity.total
                ),
                estimated_impact: attribution.logical_complexity.total / 3,
                suggested_actions: vec![
                    "Break down into smaller, focused modules".to_string(),
                    "Identify and extract common patterns".to_string(),
                    "Review architecture for simplification opportunities".to_string(),
                ],
            });
        }

        recommendations
    }
}

/// Strategy for formatting recommendations
struct FormattingStrategy {
    impact_threshold: f32,
}

impl FormattingStrategy {
    fn new() -> Self {
        Self {
            impact_threshold: 0.2,
        }
    }
}

impl RecommendationStrategy for FormattingStrategy {
    fn recommend(&self, attribution: &ComplexityAttribution) -> Vec<ComplexityRecommendation> {
        let mut recommendations = Vec::new();

        let total = attribution.logical_complexity.total + attribution.formatting_artifacts.total;
        if total == 0 {
            return recommendations;
        }

        let formatting_ratio = attribution.formatting_artifacts.total as f32 / total as f32;

        if formatting_ratio > self.impact_threshold {
            recommendations.push(ComplexityRecommendation {
                priority: RecommendationPriority::Medium,
                category: RecommendationCategory::Formatting,
                title: "Standardize code formatting".to_string(),
                description: format!(
                    "Formatting artifacts contribute {:.0}% of complexity",
                    formatting_ratio * 100.0
                ),
                estimated_impact: attribution.formatting_artifacts.total,
                suggested_actions: vec![
                    "Configure and run automated formatters".to_string(),
                    "Establish team formatting standards".to_string(),
                    "Add formatting checks to CI/CD pipeline".to_string(),
                    "Use pre-commit hooks for consistent formatting".to_string(),
                ],
            });
        }

        // Specific formatting issues
        for component in &attribution.formatting_artifacts.breakdown {
            if component.contribution > 2 {
                recommendations.push(ComplexityRecommendation {
                    priority: RecommendationPriority::Low,
                    category: RecommendationCategory::Formatting,
                    title: format!("Fix {}", component.description),
                    description: "Formatting issue detected".to_string(),
                    estimated_impact: component.contribution,
                    suggested_actions: component.suggestions.clone(),
                });
            }
        }

        recommendations
    }
}

/// Strategy for pattern-based recommendations
struct PatternStrategy {
    confidence_threshold: f32,
}

impl PatternStrategy {
    fn new() -> Self {
        Self {
            confidence_threshold: 0.3,
        }
    }
}

impl RecommendationStrategy for PatternStrategy {
    fn recommend(&self, attribution: &ComplexityAttribution) -> Vec<ComplexityRecommendation> {
        let mut recommendations = Vec::new();

        if attribution.pattern_complexity.confidence < self.confidence_threshold {
            recommendations.push(ComplexityRecommendation {
                priority: RecommendationPriority::Medium,
                category: RecommendationCategory::Pattern,
                title: "Improve code pattern consistency".to_string(),
                description: format!(
                    "Low pattern recognition confidence ({:.0}%) suggests inconsistent code structure",
                    attribution.pattern_complexity.confidence * 100.0
                ),
                estimated_impact: 5,
                suggested_actions: vec![
                    "Identify and document common patterns".to_string(),
                    "Create abstractions for repeated patterns".to_string(),
                    "Use established design patterns where appropriate".to_string(),
                    "Review codebase for duplication opportunities".to_string(),
                ],
            });
        }

        // Recommendations for recognized patterns
        for component in &attribution.pattern_complexity.breakdown {
            if !component.suggestions.is_empty() {
                recommendations.push(ComplexityRecommendation {
                    priority: RecommendationPriority::Low,
                    category: RecommendationCategory::Pattern,
                    title: format!("Optimize {}", component.description),
                    description: "Pattern optimization opportunity".to_string(),
                    estimated_impact: component.contribution,
                    suggested_actions: component.suggestions.clone(),
                });
            }
        }

        recommendations
    }
}

/// Strategy for general recommendations
struct GeneralStrategy;

impl GeneralStrategy {
    fn new() -> Self {
        Self
    }
}

impl RecommendationStrategy for GeneralStrategy {
    fn recommend(&self, attribution: &ComplexityAttribution) -> Vec<ComplexityRecommendation> {
        let mut recommendations = Vec::new();

        let total_complexity =
            attribution.logical_complexity.total + attribution.formatting_artifacts.total;

        // Testing recommendation for complex code
        if total_complexity > 20 {
            recommendations.push(ComplexityRecommendation {
                priority: RecommendationPriority::Medium,
                category: RecommendationCategory::General,
                title: "Improve test coverage".to_string(),
                description: "Complex code requires comprehensive testing".to_string(),
                estimated_impact: 0,
                suggested_actions: vec![
                    "Add unit tests for complex functions".to_string(),
                    "Use property-based testing for edge cases".to_string(),
                    "Ensure all code paths are covered".to_string(),
                ],
            });
        }

        // Documentation recommendation
        if attribution.logical_complexity.total > 15 {
            recommendations.push(ComplexityRecommendation {
                priority: RecommendationPriority::Low,
                category: RecommendationCategory::General,
                title: "Enhance documentation".to_string(),
                description: "Complex logic requires clear documentation".to_string(),
                estimated_impact: 0,
                suggested_actions: vec![
                    "Add comprehensive function documentation".to_string(),
                    "Include examples in documentation".to_string(),
                    "Document complex algorithms and decisions".to_string(),
                ],
            });
        }

        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::attribution::source_tracker::ComplexitySourceType;
    use crate::analysis::attribution::LogicalConstruct;
    use crate::analysis::attribution::{AttributedComplexity, CodeLocation, ComplexityComponent};

    #[test]
    fn test_recommendation_engine_new() {
        let engine = RecommendationEngine::new();
        assert!(!engine.strategies.is_empty());
    }

    #[test]
    fn test_refactoring_strategy() {
        let strategy = RefactoringStrategy::new();
        let mut attribution = create_test_attribution();

        // Add high-complexity component
        attribution
            .logical_complexity
            .breakdown
            .push(ComplexityComponent {
                source_type: ComplexitySourceType::LogicalStructure {
                    construct_type: LogicalConstruct::Function,
                    nesting_level: 3,
                },
                contribution: 20,
                location: CodeLocation {
                    file: "test.rs".to_string(),
                    line: 100,
                    column: 0,
                    span: None,
                },
                description: "complex_function".to_string(),
                suggestions: vec!["Break down function".to_string()],
            });

        let recommendations = strategy.recommend(&attribution);
        assert!(!recommendations.is_empty());
        assert_eq!(
            recommendations[0].category,
            RecommendationCategory::Refactoring
        );
        assert_eq!(recommendations[0].priority, RecommendationPriority::High);
    }

    #[test]
    fn test_formatting_strategy() {
        let strategy = FormattingStrategy::new();
        let mut attribution = create_test_attribution();
        attribution.formatting_artifacts.total = 10;
        attribution.logical_complexity.total = 20;

        let recommendations = strategy.recommend(&attribution);
        assert!(!recommendations.is_empty());
        assert_eq!(
            recommendations[0].category,
            RecommendationCategory::Formatting
        );
    }

    #[test]
    fn test_pattern_strategy_low_confidence() {
        let strategy = PatternStrategy::new();
        let mut attribution = create_test_attribution();
        attribution.pattern_complexity.confidence = 0.2;

        let recommendations = strategy.recommend(&attribution);
        assert!(!recommendations.is_empty());
        assert_eq!(recommendations[0].category, RecommendationCategory::Pattern);
    }

    #[test]
    fn test_general_strategy() {
        let strategy = GeneralStrategy::new();
        let mut attribution = create_test_attribution();
        attribution.logical_complexity.total = 25;

        let recommendations = strategy.recommend(&attribution);
        assert!(!recommendations.is_empty());
        assert_eq!(recommendations[0].category, RecommendationCategory::General);
    }

    #[test]
    fn test_generate_recommendations_sorted() {
        let engine = RecommendationEngine::new();
        let mut attribution = create_test_attribution();
        attribution.logical_complexity.total = 35;
        attribution.formatting_artifacts.total = 15;

        let recommendations = engine.generate_recommendations(&attribution);

        // Verify recommendations are sorted by priority
        for i in 1..recommendations.len() {
            let prev_priority = match recommendations[i - 1].priority {
                RecommendationPriority::High => 0,
                RecommendationPriority::Medium => 1,
                RecommendationPriority::Low => 2,
            };
            let curr_priority = match recommendations[i].priority {
                RecommendationPriority::High => 0,
                RecommendationPriority::Medium => 1,
                RecommendationPriority::Low => 2,
            };
            assert!(prev_priority <= curr_priority);
        }
    }

    #[test]
    fn test_recommendations_limited() {
        let engine = RecommendationEngine::new();
        let mut attribution = create_test_attribution();

        // Create conditions that would generate many recommendations
        attribution.logical_complexity.total = 50;
        attribution.formatting_artifacts.total = 20;

        // Add many components
        for i in 0..20 {
            attribution
                .logical_complexity
                .breakdown
                .push(ComplexityComponent {
                    source_type: ComplexitySourceType::LogicalStructure {
                        construct_type: LogicalConstruct::Function,
                        nesting_level: 2,
                    },
                    contribution: 16,
                    location: CodeLocation {
                        file: format!("test{}.rs", i),
                        line: i * 10,
                        column: 0,
                        span: None,
                    },
                    description: format!("function_{}", i),
                    suggestions: vec![],
                });
        }

        let recommendations = engine.generate_recommendations(&attribution);
        assert!(recommendations.len() <= 10);
    }

    fn create_test_attribution() -> ComplexityAttribution {
        ComplexityAttribution {
            logical_complexity: AttributedComplexity {
                total: 10,
                breakdown: vec![],
                confidence: 0.9,
            },
            formatting_artifacts: AttributedComplexity {
                total: 2,
                breakdown: vec![],
                confidence: 0.8,
            },
            pattern_complexity: AttributedComplexity {
                total: 1,
                breakdown: vec![],
                confidence: 0.6,
            },
            source_mappings: vec![],
        }
    }
}
