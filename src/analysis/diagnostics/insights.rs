use crate::analysis::attribution::ComplexityAttribution;
use crate::analysis::multi_pass::{ComplexityInsight, ImpactLevel, InsightType};

/// Engine for generating complexity insights
pub struct InsightGenerator {
    thresholds: InsightThresholds,
}

impl InsightGenerator {
    pub fn new() -> Self {
        Self {
            thresholds: InsightThresholds::default(),
        }
    }

    pub fn with_thresholds(thresholds: InsightThresholds) -> Self {
        Self { thresholds }
    }

    pub fn generate_insights(&self, attribution: &ComplexityAttribution) -> Vec<ComplexityInsight> {
        let mut insights = Vec::new();

        // Analyze formatting impact
        insights.extend(self.analyze_formatting_impact(attribution));

        // Analyze pattern recognition
        insights.extend(self.analyze_pattern_recognition(attribution));

        // Analyze logical complexity
        insights.extend(self.analyze_logical_complexity(attribution));

        // Analyze complexity distribution
        insights.extend(self.analyze_complexity_distribution(attribution));

        // Sort by impact level
        insights.sort_by_key(|i| match i.impact_level {
            ImpactLevel::Critical => 0,
            ImpactLevel::High => 1,
            ImpactLevel::Medium => 2,
            ImpactLevel::Low => 3,
        });

        insights
    }

    fn analyze_formatting_impact(
        &self,
        attribution: &ComplexityAttribution,
    ) -> Vec<ComplexityInsight> {
        let mut insights = Vec::new();

        let total_complexity =
            attribution.logical_complexity.total + attribution.formatting_artifacts.total;
        if total_complexity == 0 {
            return insights;
        }

        let formatting_ratio =
            attribution.formatting_artifacts.total as f32 / total_complexity as f32;

        if formatting_ratio > self.thresholds.high_formatting_ratio {
            insights.push(ComplexityInsight {
                insight_type: InsightType::FormattingImpact,
                description: format!(
                    "Formatting artifacts contribute {:.0}% of total complexity",
                    formatting_ratio * 100.0
                ),
                impact_level: ImpactLevel::High,
                actionable_steps: vec![
                    "Adopt consistent code formatting standards".to_string(),
                    "Use automated formatting tools (rustfmt, black, prettier)".to_string(),
                    "Configure IDE to format on save".to_string(),
                ],
            });
        } else if formatting_ratio > self.thresholds.medium_formatting_ratio {
            insights.push(ComplexityInsight {
                insight_type: InsightType::FormattingImpact,
                description: format!(
                    "Moderate formatting impact detected ({:.0}%)",
                    formatting_ratio * 100.0
                ),
                impact_level: ImpactLevel::Medium,
                actionable_steps: vec![
                    "Review code formatting guidelines".to_string(),
                    "Consider automated formatting in CI/CD".to_string(),
                ],
            });
        }

        insights
    }

    fn analyze_pattern_recognition(
        &self,
        attribution: &ComplexityAttribution,
    ) -> Vec<ComplexityInsight> {
        let mut insights = Vec::new();

        let confidence = attribution.pattern_complexity.confidence;

        if confidence < self.thresholds.low_pattern_confidence {
            insights.push(ComplexityInsight {
                insight_type: InsightType::PatternOpportunity,
                description:
                    "Low pattern recognition suggests unique or inconsistent code structure"
                        .to_string(),
                impact_level: ImpactLevel::Medium,
                actionable_steps: vec![
                    "Identify and extract common patterns".to_string(),
                    "Consider using design patterns where appropriate".to_string(),
                    "Review for opportunities to standardize similar code".to_string(),
                ],
            });
        } else if confidence > self.thresholds.high_pattern_confidence {
            insights.push(ComplexityInsight {
                insight_type: InsightType::PatternOpportunity,
                description: format!(
                    "High pattern confidence ({:.0}%) indicates well-structured code",
                    confidence * 100.0
                ),
                impact_level: ImpactLevel::Low,
                actionable_steps: vec![
                    "Document recognized patterns for team reference".to_string(),
                    "Consider creating reusable components for common patterns".to_string(),
                ],
            });
        }

        // Analyze specific patterns
        for component in &attribution.pattern_complexity.breakdown {
            if component.contribution > self.thresholds.significant_pattern_contribution {
                insights.push(ComplexityInsight {
                    insight_type: InsightType::PatternOpportunity,
                    description: format!(
                        "{} reduces complexity by {}",
                        component.description, component.contribution
                    ),
                    impact_level: ImpactLevel::Low,
                    actionable_steps: component.suggestions.clone(),
                });
            }
        }

        insights
    }

    fn analyze_logical_complexity(
        &self,
        attribution: &ComplexityAttribution,
    ) -> Vec<ComplexityInsight> {
        let mut insights = Vec::new();

        if attribution.logical_complexity.total > self.thresholds.high_logical_complexity {
            insights.push(ComplexityInsight {
                insight_type: InsightType::RefactoringCandidate,
                description: format!(
                    "High logical complexity ({}) indicates refactoring opportunity",
                    attribution.logical_complexity.total
                ),
                impact_level: ImpactLevel::High,
                actionable_steps: vec![
                    "Break down complex functions into smaller units".to_string(),
                    "Extract complex conditions into well-named functions".to_string(),
                    "Use early returns to reduce nesting levels".to_string(),
                    "Consider the Single Responsibility Principle".to_string(),
                ],
            });
        }

        // Find complexity hotspots
        let mut hotspots = Vec::new();
        for component in &attribution.logical_complexity.breakdown {
            if component.contribution > self.thresholds.hotspot_threshold {
                hotspots.push(component);
            }
        }

        if !hotspots.is_empty() {
            let descriptions: Vec<String> = hotspots
                .iter()
                .take(3)
                .map(|c| format!("{} ({})", c.description, c.contribution))
                .collect();

            insights.push(ComplexityInsight {
                insight_type: InsightType::ComplexityHotspot,
                description: format!("Complexity hotspots found: {}", descriptions.join(", ")),
                impact_level: ImpactLevel::High,
                actionable_steps: vec![
                    "Focus refactoring efforts on these high-complexity areas".to_string(),
                    "Consider splitting these into multiple functions".to_string(),
                    "Add unit tests before refactoring".to_string(),
                ],
            });
        }

        insights
    }

    fn analyze_complexity_distribution(
        &self,
        attribution: &ComplexityAttribution,
    ) -> Vec<ComplexityInsight> {
        let mut insights = Vec::new();

        let total = attribution.logical_complexity.total
            + attribution.formatting_artifacts.total
            + attribution.pattern_complexity.total;

        if total == 0 {
            return insights;
        }

        let logical_ratio = attribution.logical_complexity.total as f32 / total as f32;
        let formatting_ratio = attribution.formatting_artifacts.total as f32 / total as f32;
        let pattern_ratio = attribution.pattern_complexity.total as f32 / total as f32;

        if logical_ratio > 0.8 {
            insights.push(ComplexityInsight {
                insight_type: InsightType::ImprovementSuggestion,
                description:
                    "Complexity is primarily logical, indicating genuine algorithmic complexity"
                        .to_string(),
                impact_level: ImpactLevel::Medium,
                actionable_steps: vec![
                    "Consider algorithmic improvements".to_string(),
                    "Look for opportunities to simplify logic".to_string(),
                    "Review for over-engineering".to_string(),
                ],
            });
        }

        if formatting_ratio > 0.3 && pattern_ratio < 0.2 {
            insights.push(ComplexityInsight {
                insight_type: InsightType::ImprovementSuggestion,
                description: "High formatting impact with low pattern recognition".to_string(),
                impact_level: ImpactLevel::Medium,
                actionable_steps: vec![
                    "Standardize code structure".to_string(),
                    "Apply consistent patterns across the codebase".to_string(),
                ],
            });
        }

        insights
    }
}

/// Thresholds for insight generation
#[derive(Debug, Clone)]
pub struct InsightThresholds {
    pub high_formatting_ratio: f32,
    pub medium_formatting_ratio: f32,
    pub low_pattern_confidence: f32,
    pub high_pattern_confidence: f32,
    pub high_logical_complexity: u32,
    pub hotspot_threshold: u32,
    pub significant_pattern_contribution: u32,
}

impl Default for InsightThresholds {
    fn default() -> Self {
        Self {
            high_formatting_ratio: 0.3,
            medium_formatting_ratio: 0.15,
            low_pattern_confidence: 0.3,
            high_pattern_confidence: 0.8,
            high_logical_complexity: 20,
            hotspot_threshold: 10,
            significant_pattern_contribution: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::attribution::source_tracker::ComplexitySourceType;
    use crate::analysis::attribution::{AttributedComplexity, CodeLocation, ComplexityComponent};
    use crate::analysis::attribution::{LogicalConstruct, RecognizedPattern};

    #[test]
    fn test_insight_generator_new() {
        let generator = InsightGenerator::new();
        assert_eq!(generator.thresholds.high_formatting_ratio, 0.3);
    }

    #[test]
    fn test_formatting_impact_high() {
        let generator = InsightGenerator::new();
        let attribution = create_test_attribution(10, 5, 0);

        let insights = generator.analyze_formatting_impact(&attribution);
        assert!(!insights.is_empty());
        assert_eq!(insights[0].impact_level, ImpactLevel::High);
    }

    #[test]
    fn test_pattern_recognition_low() {
        let generator = InsightGenerator::new();
        let mut attribution = create_test_attribution(15, 2, 1);
        attribution.pattern_complexity.confidence = 0.2;

        let insights = generator.analyze_pattern_recognition(&attribution);
        assert!(!insights.is_empty());
        assert_eq!(insights[0].insight_type, InsightType::PatternOpportunity);
    }

    #[test]
    fn test_logical_complexity_high() {
        let generator = InsightGenerator::new();
        let attribution = create_test_attribution(25, 2, 1);

        let insights = generator.analyze_logical_complexity(&attribution);
        assert!(!insights.is_empty());
        assert_eq!(insights[0].insight_type, InsightType::RefactoringCandidate);
        assert_eq!(insights[0].impact_level, ImpactLevel::High);
    }

    #[test]
    fn test_complexity_hotspots() {
        let generator = InsightGenerator::new();
        let mut attribution = create_test_attribution(20, 2, 1);

        // Add a hotspot
        attribution
            .logical_complexity
            .breakdown
            .push(ComplexityComponent {
                source_type: ComplexitySourceType::LogicalStructure {
                    construct_type: LogicalConstruct::Function,
                    nesting_level: 3,
                },
                contribution: 15,
                location: CodeLocation {
                    file: "test.rs".to_string(),
                    line: 100,
                    column: 0,
                    span: None,
                },
                description: "Complex function".to_string(),
                suggestions: vec![],
            });

        let insights = generator.analyze_logical_complexity(&attribution);
        let hotspot_insight = insights
            .iter()
            .find(|i| i.insight_type == InsightType::ComplexityHotspot);
        assert!(hotspot_insight.is_some());
    }

    #[test]
    fn test_generate_insights_sorted() {
        let generator = InsightGenerator::new();
        let attribution = create_test_attribution(25, 8, 2);

        let insights = generator.generate_insights(&attribution);

        // Verify insights are sorted by impact level
        for i in 1..insights.len() {
            let prev_priority = match insights[i - 1].impact_level {
                ImpactLevel::Critical => 0,
                ImpactLevel::High => 1,
                ImpactLevel::Medium => 2,
                ImpactLevel::Low => 3,
            };
            let curr_priority = match insights[i].impact_level {
                ImpactLevel::Critical => 0,
                ImpactLevel::High => 1,
                ImpactLevel::Medium => 2,
                ImpactLevel::Low => 3,
            };
            assert!(prev_priority <= curr_priority);
        }
    }

    fn create_test_attribution(
        logical: u32,
        formatting: u32,
        pattern: u32,
    ) -> ComplexityAttribution {
        ComplexityAttribution {
            logical_complexity: AttributedComplexity {
                total: logical,
                breakdown: vec![],
                confidence: 0.9,
            },
            formatting_artifacts: AttributedComplexity {
                total: formatting,
                breakdown: vec![],
                confidence: 0.8,
            },
            pattern_complexity: AttributedComplexity {
                total: pattern,
                breakdown: vec![],
                confidence: 0.6,
            },
            source_mappings: vec![],
        }
    }
}
