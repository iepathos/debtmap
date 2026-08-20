//! File parsing stage.
//!
//! Analyzes discovered files using language-specific parsers to extract
//! function metrics (complexity, LOC, parameters, etc.).

use crate::errors::AnalysisError;
use crate::pipeline::data::PipelineData;
use crate::pipeline::stage::Stage;

/// Stage 2: Parse files to extract metrics
///
/// Analyzes discovered files using language-specific parsers to extract
/// function metrics (complexity, LOC, parameters, etc.).
pub struct ParsingStage;

impl ParsingStage {
    #[deprecated(
        note = "the composable analysis pipeline is incomplete and fails closed; use the canonical unified analysis API"
    )]
    pub fn new() -> Self {
        Self
    }
}

impl Default for ParsingStage {
    fn default() -> Self {
        Self
    }
}

impl Stage for ParsingStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, _data: Self::Input) -> Result<Self::Output, Self::Error> {
        Err(AnalysisError::analysis(
            "Composable analysis parsing is not implemented; use `debtmap analyze` or the canonical unified analysis API",
        ))
    }

    fn name(&self) -> &str {
        "Parsing"
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)]

    use super::*;

    #[test]
    fn test_parsing_stage_creation() {
        let stage = ParsingStage::new();
        assert_eq!(stage.name(), "Parsing");
    }
}
