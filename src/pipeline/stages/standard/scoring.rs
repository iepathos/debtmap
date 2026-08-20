//! Debt detection and scoring stages.
//!
//! These compatibility stages fail closed until they can delegate to the
//! canonical unified analysis implementation.

use crate::errors::AnalysisError;
use crate::pipeline::data::PipelineData;
use crate::pipeline::stage::Stage;

/// Stage 8: Detect technical debt
///
/// Identifies technical debt patterns in the analyzed code.
pub struct DebtDetectionStage;

impl DebtDetectionStage {
    #[deprecated(
        note = "the composable analysis pipeline is incomplete and fails closed; use the canonical unified analysis API"
    )]
    pub fn new() -> Self {
        Self
    }
}

impl Default for DebtDetectionStage {
    fn default() -> Self {
        Self
    }
}

impl Stage for DebtDetectionStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, _data: Self::Input) -> Result<Self::Output, Self::Error> {
        Err(AnalysisError::analysis(
            "Composable analysis debt detection is not implemented; use `debtmap analyze` or the canonical unified analysis API",
        ))
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
    #[deprecated(
        note = "the composable analysis pipeline is incomplete and fails closed; use the canonical unified analysis API"
    )]
    pub fn new() -> Self {
        Self
    }
}

impl Default for ScoringStage {
    fn default() -> Self {
        Self
    }
}

impl Stage for ScoringStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, _data: Self::Input) -> Result<Self::Output, Self::Error> {
        Err(AnalysisError::analysis(
            "Composable analysis scoring is not implemented; use `debtmap analyze` or the canonical unified analysis API",
        ))
    }

    fn name(&self) -> &str {
        "Scoring & Prioritization"
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)]

    use super::*;

    #[test]
    fn test_debt_detection_stage_creation() {
        let stage = DebtDetectionStage::new();
        assert_eq!(stage.name(), "Debt Detection");
    }

    #[test]
    fn debt_detection_stage_fails_closed() {
        let error = DebtDetectionStage::new()
            .execute(PipelineData::new(Vec::new()))
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("debt detection is not implemented")
        );
        assert!(error.to_string().contains("debtmap analyze"));
    }

    #[test]
    fn test_scoring_stage_creation() {
        let stage = ScoringStage::new();
        assert_eq!(stage.name(), "Scoring & Prioritization");
    }

    #[test]
    fn scoring_stage_fails_closed() {
        let error = ScoringStage::new()
            .execute(PipelineData::new(Vec::new()))
            .unwrap_err();

        assert!(error.to_string().contains("scoring is not implemented"));
        assert!(error.to_string().contains("debtmap analyze"));
    }
}
