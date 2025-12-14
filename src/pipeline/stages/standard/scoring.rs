//! Debt detection and scoring stages.
//!
//! These are thin wrapper stages that delegate to pure functions in sibling modules:
//! - `super::debt::detect_debt_from_pipeline`
//! - `super::scoring::score_debt_items`

use crate::pipeline::data::PipelineData;
use crate::pipeline::stage::Stage;

/// Stage 8: Detect technical debt
///
/// Identifies technical debt patterns in the analyzed code.
pub struct DebtDetectionStage;

impl DebtDetectionStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DebtDetectionStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage for DebtDetectionStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = std::convert::Infallible;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        let debt_items =
            super::super::debt::detect_debt_from_pipeline(&data.metrics, data.call_graph.as_ref());
        data.debt_items = debt_items;
        Ok(data)
    }

    fn name(&self) -> &str {
        "Debt Detection"
    }
}

/// Stage 9: Score and prioritize debt
///
/// Assigns priority scores to debt items based on impact, risk, and context.
pub struct ScoringStage;

impl ScoringStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ScoringStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage for ScoringStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = std::convert::Infallible;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        let scored_items = super::super::scoring::score_debt_items(
            &data.debt_items,
            data.call_graph.as_ref(),
            data.coverage.as_ref(),
            data.purity_scores.as_ref(),
        );
        data.scored_items = scored_items;
        Ok(data)
    }

    fn name(&self) -> &str {
        "Scoring & Prioritization"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debt_detection_stage_creation() {
        let stage = DebtDetectionStage::new();
        assert_eq!(stage.name(), "Debt Detection");
    }

    #[test]
    fn test_scoring_stage_creation() {
        let stage = ScoringStage::new();
        assert_eq!(stage.name(), "Scoring & Prioritization");
    }
}
