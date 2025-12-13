//! Effect-based scoring with Reader pattern (Spec 268).
//!
//! This module provides effect-based scoring functions that access configuration
//! via the Reader pattern, enabling testability and composability.
//!
//! # Design Philosophy
//!
//! The Reader pattern allows scoring functions to:
//! - Access scoring weights from the environment without global state
//! - Be tested with custom configurations via `DebtmapTestEnv`
//! - Compose naturally with other effects in the analysis pipeline
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::priority::scoring::effects::{calculate_score_effect, ScoringEnv};
//! use debtmap::priority::scoring::ScoreWeights;
//!
//! // Create environment with custom weights
//! let env = TestScoringEnv::new(ScoreWeights::quality_focused());
//!
//! let effect = calculate_score_effect(score_components);
//! let score = effect.run(&env).await?;
//! ```

use super::{DebtScore, ScoreComponents, ScoreWeights, ScoringRationale, Severity};
use crate::errors::AnalysisError;
use stillwater::Effect;

/// Environment trait for scoring configuration.
///
/// This trait follows the Reader pattern, allowing scoring functions to access
/// configuration without global state.
pub trait ScoringEnv: Send + Sync + Clone + 'static {
    /// Get the scoring weights configuration.
    fn scoring_weights(&self) -> &ScoreWeights;
}

/// Calculate a debt score using configuration from the environment.
///
/// This effect accesses scoring weights from the environment via the Reader pattern,
/// enabling testable scoring logic.
///
/// # Type Parameters
///
/// * `Env` - Environment type implementing `ScoringEnv`
///
/// # Arguments
///
/// * `components` - The score components to calculate from
///
/// # Returns
///
/// An effect that produces a calculated `DebtScore`.
pub fn calculate_score_effect<Env>(
    components: ScoreComponents,
) -> impl Effect<Output = DebtScore, Error = AnalysisError, Env = Env>
where
    Env: ScoringEnv,
{
    stillwater::asks(move |env: &Env| {
        let weights = env.scoring_weights();
        let total = components.weighted_total(weights);
        let severity = determine_severity(total);
        let rationale = ScoringRationale::explain(&components, weights);

        DebtScore {
            total,
            components: components.clone(),
            severity,
            rationale,
        }
    })
}

/// Calculate severity from a total score.
fn determine_severity(score: f64) -> Severity {
    match score {
        s if s >= 150.0 => Severity::Critical,
        s if s >= 100.0 => Severity::High,
        s if s >= 50.0 => Severity::Medium,
        _ => Severity::Low,
    }
}

/// Get the current scoring weights from the environment.
///
/// This effect provides direct access to the scoring weights.
pub fn get_weights_effect<Env>(
) -> impl Effect<Output = ScoreWeights, Error = AnalysisError, Env = Env>
where
    Env: ScoringEnv,
{
    stillwater::asks(|env: &Env| env.scoring_weights().clone())
}

/// Calculate scores for multiple items using the environment configuration.
///
/// # Arguments
///
/// * `items` - Score components for multiple items
///
/// # Returns
///
/// An effect that produces calculated scores for all items.
pub fn calculate_scores_effect<Env>(
    items: Vec<ScoreComponents>,
) -> impl Effect<Output = Vec<DebtScore>, Error = AnalysisError, Env = Env>
where
    Env: ScoringEnv,
{
    stillwater::asks(move |env: &Env| {
        let weights = env.scoring_weights();
        items
            .iter()
            .map(|components| {
                let total = components.weighted_total(weights);
                let severity = determine_severity(total);
                let rationale = ScoringRationale::explain(components, weights);
                DebtScore {
                    total,
                    components: components.clone(),
                    severity,
                    rationale,
                }
            })
            .collect()
    })
}

/// A test environment for scoring that implements `ScoringEnv`.
///
/// This is useful for unit testing scoring functions with custom weights.
#[derive(Clone)]
pub struct TestScoringEnv {
    weights: ScoreWeights,
}

impl TestScoringEnv {
    /// Create a new test environment with the given weights.
    pub fn new(weights: ScoreWeights) -> Self {
        Self { weights }
    }

    /// Create a test environment with default (balanced) weights.
    pub fn default_weights() -> Self {
        Self::new(ScoreWeights::balanced())
    }

    /// Create a test environment with quality-focused weights.
    pub fn quality_focused() -> Self {
        Self::new(ScoreWeights::quality_focused())
    }
}

impl Default for TestScoringEnv {
    fn default() -> Self {
        Self::default_weights()
    }
}

impl ScoringEnv for TestScoringEnv {
    fn scoring_weights(&self) -> &ScoreWeights {
        &self.weights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_components() -> ScoreComponents {
        ScoreComponents {
            complexity_score: 50.0,
            coverage_score: 40.0,
            structural_score: 30.0,
            size_score: 15.0,
            smell_score: 20.0,
        }
    }

    #[tokio::test]
    async fn test_calculate_score_effect_default() {
        let env = TestScoringEnv::default_weights();
        let effect = calculate_score_effect::<TestScoringEnv>(test_components());
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        let score = result.unwrap();
        assert!(score.total > 0.0);
    }

    #[tokio::test]
    async fn test_calculate_score_effect_quality_focused() {
        let env = TestScoringEnv::quality_focused();
        let effect = calculate_score_effect::<TestScoringEnv>(test_components());
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        let score = result.unwrap();
        assert!(score.total > 0.0);
    }

    #[tokio::test]
    async fn test_get_weights_effect() {
        let env = TestScoringEnv::quality_focused();
        let effect = get_weights_effect::<TestScoringEnv>();
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        let weights = result.unwrap();
        assert!(weights.complexity_weight > 1.0); // Quality focused has higher complexity weight
    }

    #[tokio::test]
    async fn test_calculate_scores_effect_multiple() {
        let env = TestScoringEnv::default_weights();
        let items = vec![
            ScoreComponents {
                complexity_score: 100.0,
                coverage_score: 80.0,
                structural_score: 60.0,
                size_score: 30.0,
                smell_score: 40.0,
            },
            ScoreComponents {
                complexity_score: 10.0,
                coverage_score: 10.0,
                structural_score: 10.0,
                size_score: 10.0,
                smell_score: 10.0,
            },
        ];

        let effect = calculate_scores_effect::<TestScoringEnv>(items);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        let scores = result.unwrap();
        assert_eq!(scores.len(), 2);
        // First item should have higher score
        assert!(scores[0].total > scores[1].total);
    }

    #[test]
    fn test_severity_determination() {
        assert_eq!(determine_severity(200.0), Severity::Critical);
        assert_eq!(determine_severity(150.0), Severity::Critical);
        assert_eq!(determine_severity(125.0), Severity::High);
        assert_eq!(determine_severity(100.0), Severity::High);
        assert_eq!(determine_severity(75.0), Severity::Medium);
        assert_eq!(determine_severity(50.0), Severity::Medium);
        assert_eq!(determine_severity(25.0), Severity::Low);
        assert_eq!(determine_severity(0.0), Severity::Low);
    }

    #[test]
    fn test_test_scoring_env_default() {
        let env: TestScoringEnv = Default::default();
        let weights = env.scoring_weights();
        assert_eq!(weights.complexity_weight, 1.0);
    }
}
