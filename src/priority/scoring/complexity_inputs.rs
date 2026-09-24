//! Complexity operands captured before scoring; rendering never reloads configuration.

use super::calculation::calculate_complexity_factor;
use crate::complexity::EntropyAnalysis;
use crate::config::ComplexityWeightsConfig;

/// Selected preprocessing and weights used by the complexity factor calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComplexityInputs {
    raw: [u32; 2],
    purity_factor: f64,
    // Entropy selects the original cognitive metric, replacing purity adjustment.
    entropy: Option<(f64, u32)>,
    weights: [f64; 2],
}

impl ComplexityInputs {
    pub(crate) fn for_function(
        func: &crate::core::FunctionMetrics,
        purity_factor: f64,
        orchestrator: bool,
        config: &crate::config::DebtmapConfig,
    ) -> Self {
        let entropy = super::computation::calculate_entropy_analysis(func);
        let enabled = config.entropy.clone().unwrap_or_default().enabled;
        Self::new(
            [func.cyclomatic, func.cognitive],
            purity_factor,
            entropy.as_ref().filter(|_| enabled),
            complexity_weights(config.complexity_weights.as_ref(), orchestrator),
        )
    }

    pub(crate) fn new(
        raw: [u32; 2],
        purity_factor: f64,
        entropy: Option<&EntropyAnalysis>,
        weights: [f64; 2],
    ) -> Self {
        Self {
            raw,
            purity_factor,
            entropy: entropy.map(|e| (e.dampening_factor, e.adjusted_complexity)),
            weights,
        }
    }

    fn cyclomatic(self) -> u32 {
        (self.raw[0] as f64 * self.purity_factor) as u32
    }

    fn cognitive(self) -> u32 {
        self.entropy
            .map(|(_, adjusted)| adjusted)
            .unwrap_or_else(|| (self.raw[1] as f64 * self.purity_factor) as u32)
    }

    pub(crate) fn weighted_complexity(self) -> f64 {
        self.cyclomatic() as f64 * self.weights[0] + self.cognitive() as f64 * self.weights[1]
    }

    pub(crate) fn factor(self) -> f64 {
        calculate_complexity_factor(self.weighted_complexity())
    }

    pub(crate) fn preprocessing_lines(self) -> Vec<String> {
        let (source, factor) = self
            .entropy
            .map(|(factor, _)| ("entropy", factor))
            .unwrap_or(("purity", self.purity_factor));
        vec![
            format!(
                "Cyclomatic input (purity): trunc({} × {:.4}) = {}",
                self.raw[0],
                self.purity_factor,
                self.cyclomatic()
            ),
            format!(
                "Cognitive input ({source}): trunc({} × {factor:.4}) = {}",
                self.raw[1],
                self.cognitive()
            ),
        ]
    }
}

impl std::fmt::Display for ComplexityInputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "clamp(({} × {:.4} + {} × {:.4}) / 2, 0, 10)",
            self.cyclomatic(),
            self.weights[0],
            self.cognitive(),
            self.weights[1]
        )
    }
}

/// Configured weights override role defaults, including orchestrator weights.
pub(crate) fn complexity_weights(
    config: Option<&ComplexityWeightsConfig>,
    orchestrator: bool,
) -> [f64; 2] {
    match config {
        Some(weights) => [weights.cyclomatic, weights.cognitive],
        None if orchestrator => [0.25, 0.75],
        None => [0.4, 0.6],
    }
}
