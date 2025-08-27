use super::{AnalysisContext, ExtractablePattern, MatchedPattern};

pub struct ConfidenceScorer;

impl Default for ConfidenceScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfidenceScorer {
    pub fn new() -> Self {
        Self
    }

    pub fn score_pattern(pattern: &MatchedPattern, context: &AnalysisContext) -> f32 {
        let base_score = Self::calculate_base_score(&pattern.pattern);
        let complexity_factor = Self::calculate_complexity_factor(context.complexity_before);
        let side_effect_penalty = if context.has_side_effects { 0.7 } else { 1.0 };
        let dependency_factor = Self::calculate_dependency_factor(&context.data_dependencies);

        (base_score * complexity_factor * side_effect_penalty * dependency_factor).min(1.0)
    }

    fn calculate_base_score(pattern: &ExtractablePattern) -> f32 {
        match pattern {
            ExtractablePattern::AccumulationLoop {
                filter, transform, ..
            } => {
                // Accumulation loops are highly extractable
                let base = 0.9;
                let filter_penalty = if filter.is_some() { 0.95 } else { 1.0 };
                let transform_penalty = if transform.is_some() { 0.95 } else { 1.0 };
                base * filter_penalty * transform_penalty
            }
            ExtractablePattern::GuardChainSequence { checks, .. } => {
                // Guard chains are very extractable
                let base = 0.95;
                // Reduce confidence slightly for very long guard chains
                let length_factor = match checks.len() {
                    1..=3 => 1.0,
                    4..=6 => 0.95,
                    7..=10 => 0.9,
                    _ => 0.85,
                };
                base * length_factor
            }
            ExtractablePattern::TransformationPipeline { stages, .. } => {
                // Pipelines are naturally functional
                let base = 0.85;
                // Each stage adds slight complexity
                let stage_factor = (1.0 - (stages.len() as f32 * 0.02)).max(0.7);
                base * stage_factor
            }
            ExtractablePattern::SimilarBranches {
                branch_specific, ..
            } => {
                // Similar branches can be extracted but need careful handling
                let base = 0.75;
                // More branches = more complexity
                let branch_factor = (1.0 - (branch_specific.len() as f32 * 0.05)).max(0.5);
                base * branch_factor
            }
            ExtractablePattern::NestedExtraction { inner_patterns, .. } => {
                // Nested patterns are complex but valuable
                let base = 0.7;
                // Recursively score inner patterns
                let inner_avg = if inner_patterns.is_empty() {
                    1.0
                } else {
                    inner_patterns
                        .iter()
                        .map(|p| Self::calculate_base_score(p))
                        .sum::<f32>()
                        / inner_patterns.len() as f32
                };
                base * inner_avg
            }
        }
    }

    fn calculate_complexity_factor(cyclomatic: u32) -> f32 {
        match cyclomatic {
            0..=5 => 1.0,    // Simple functions
            6..=10 => 0.95,  // Moderate complexity
            11..=15 => 0.9,  // High complexity
            16..=20 => 0.85, // Very high complexity
            _ => 0.8,        // Extreme complexity
        }
    }

    fn calculate_dependency_factor(dependencies: &[String]) -> f32 {
        match dependencies.len() {
            0 => 1.0,      // No dependencies - perfect
            1..=2 => 0.95, // Few dependencies
            3..=4 => 0.9,  // Moderate dependencies
            5..=7 => 0.85, // Many dependencies
            _ => 0.8,      // Too many dependencies
        }
    }
}

pub struct ConfidenceFactors {
    pub has_clear_boundaries: bool,
    pub no_external_state: bool,
    pub pure_computation: bool,
    pub single_responsibility: bool,
    pub testable_in_isolation: bool,
}

impl ConfidenceFactors {
    pub fn calculate_confidence(&self) -> f32 {
        let mut score = 0.0;
        let mut factors = 0.0;

        if self.has_clear_boundaries {
            score += 0.25;
            factors += 0.25;
        }

        if self.no_external_state {
            score += 0.25;
            factors += 0.25;
        }

        if self.pure_computation {
            score += 0.2;
            factors += 0.2;
        }

        if self.single_responsibility {
            score += 0.15;
            factors += 0.15;
        }

        if self.testable_in_isolation {
            score += 0.15;
            factors += 0.15;
        }

        if factors > 0.0 {
            score / factors
        } else {
            0.5 // Default confidence if no factors apply
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_scoring() {
        let _scorer = ConfidenceScorer::new();

        // Test accumulation loop pattern
        let pattern = ExtractablePattern::AccumulationLoop {
            iterator_binding: "item".to_string(),
            accumulator: "sum".to_string(),
            operation: super::super::AccumulationOp::Sum,
            filter: None,
            transform: None,
            start_line: 10,
            end_line: 20,
        };

        let matched = MatchedPattern {
            pattern,
            confidence: 0.0, // Will be calculated
            context: AnalysisContext {
                function_name: "test_func".to_string(),
                file_path: "test.rs".to_string(),
                language: "rust".to_string(),
                complexity_before: 5,
                has_side_effects: false,
                data_dependencies: vec![],
            },
        };

        let score = ConfidenceScorer::score_pattern(&matched, &matched.context);
        assert!(score > 0.8); // Should have high confidence
    }

    #[test]
    fn test_confidence_factors() {
        let factors = ConfidenceFactors {
            has_clear_boundaries: true,
            no_external_state: true,
            pure_computation: true,
            single_responsibility: true,
            testable_in_isolation: true,
        };

        let confidence = factors.calculate_confidence();
        assert_eq!(confidence, 1.0); // Perfect confidence with all factors
    }
}
