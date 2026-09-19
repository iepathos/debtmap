//! In-memory arithmetic evidence captured when a score is calculated.
//! This trace is deliberately excluded from persisted/public records.

use std::fmt;

/// Render actual arithmetic, or an explicit limitation for older records.
pub fn explanation_lines(score: &crate::priority::UnifiedScore) -> Vec<String> {
    let mut lines = if score.score_trace.is_empty() {
        vec![
            "Arithmetic trace unavailable; recorded factors do not reconstruct the final score."
                .to_string(),
            format!("Recorded complexity factor: {:.4}", score.complexity_factor),
            format!(
                "Recorded coverage gap indicator: {:.4}",
                score.coverage_factor
            ),
            format!("Recorded dependency factor: {:.4}", score.dependency_factor),
        ]
    } else {
        score
            .score_trace
            .iter()
            .flat_map(ScoreStep::explanation_lines)
            .collect()
    };
    lines.push(format!("Final Score: {:.2}", score.final_score));
    lines
}

/// One arithmetic operation, with the operands used during scoring.
#[derive(Debug, Clone, PartialEq)]
pub enum ScoreOperation {
    Complexity(super::complexity_inputs::ComplexityInputs),
    WeightedBase {
        complexity: f64,
        dependency: f64,
    },
    Multiply(f64),
    Add(f64),
    Floor(f64),
    Power(f64),
    WeightedBlend {
        factors: [f64; 3],
        weights: [f64; 3],
    },
    /// An algorithmic adjustment whose result cannot be expressed as one factor.
    Adjustment,
}

/// Numeric evidence for an applied scoring step; never reconstructed from config.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreStep {
    pub label: &'static str,
    pub input: f64,
    pub operation: ScoreOperation,
    pub output: f64,
}

impl ScoreStep {
    /// Include preprocessing operands before the scalar scoring operation.
    pub fn explanation_lines(&self) -> Vec<String> {
        let mut lines = match self.operation {
            ScoreOperation::Complexity(inputs) => inputs.preprocessing_lines(),
            _ => Vec::new(),
        };
        lines.push(self.to_string());
        lines
    }

    pub fn new(label: &'static str, input: f64, operation: ScoreOperation, output: f64) -> Self {
        Self {
            label,
            input,
            operation,
            output,
        }
    }

    /// Re-evaluate a recorded operation for consistency checks.
    pub fn calculated_output(&self) -> f64 {
        match self.operation {
            ScoreOperation::Complexity(inputs) => inputs.factor(),
            ScoreOperation::WeightedBase {
                complexity,
                dependency,
            } => complexity * 5.0 + dependency * 2.5,
            ScoreOperation::Multiply(factor) => self.input * factor,
            ScoreOperation::Add(value) => self.input + value,
            ScoreOperation::Floor(minimum) => self.input.max(minimum),
            ScoreOperation::Power(exponent) => self.input.powf(exponent),
            ScoreOperation::WeightedBlend { factors, weights } => {
                self.input * blend(factors, weights)
            }
            ScoreOperation::Adjustment => self.output,
        }
    }
}

fn blend(factors: [f64; 3], weights: [f64; 3]) -> f64 {
    let total: f64 = weights.iter().sum();
    if total > 0.0 {
        factors
            .iter()
            .zip(weights)
            .map(|(factor, weight)| factor * weight)
            .sum::<f64>()
            / total
    } else {
        1.0
    }
}

impl fmt::Display for ScoreStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.label)?;
        match self.operation {
            ScoreOperation::Complexity(inputs) => write!(f, "{inputs}")?,
            ScoreOperation::WeightedBase {
                complexity,
                dependency,
            } => write!(f, "{complexity:.4} × 5 + {dependency:.4} × 2.5")?,
            ScoreOperation::Multiply(factor) => write!(f, "{:.4} × {factor:.4}", self.input)?,
            ScoreOperation::Add(value) => write!(f, "{:.4} + {value:.4}", self.input)?,
            ScoreOperation::Floor(minimum) => write!(f, "max({:.4}, {minimum:.4})", self.input)?,
            ScoreOperation::Power(exponent) => write!(f, "{:.4}^{exponent:.4}", self.input)?,
            ScoreOperation::WeightedBlend {
                factors: [p, r, t],
                weights: [pw, rw, tw],
            } => {
                write!(
                    f,
                    "{:.4} × blend(purity {p:.4} × {pw:.4}, refactorability {r:.4} × {rw:.4}, pattern {t:.4} × {tw:.4}; weight sum {:.4}, multiplier {:.4})",
                    self.input,
                    pw + rw + tw,
                    blend([p, r, t], [pw, rw, tw])
                )?;
            }
            ScoreOperation::Adjustment => write!(f, "{:.4} →", self.input)?,
        }
        write!(f, " = {:.4}", self.output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_actual_weighted_blend_instead_of_multiplying_factors() {
        let step = ScoreStep::new(
            "Data-flow blend",
            20.0,
            ScoreOperation::WeightedBlend {
                factors: [0.3, 1.0, 0.85],
                weights: [0.2, 0.5, 0.3],
            },
            16.3,
        );
        assert!((step.calculated_output() - 16.3).abs() < 1e-10);
        assert!(step.to_string().contains("0.2000"));
        assert!(step.to_string().contains("16.3000"));
    }

    #[test]
    fn zero_weight_blend_is_neutral_and_floors_are_explicit() {
        let blend = ScoreStep::new(
            "Data-flow blend",
            12.0,
            ScoreOperation::WeightedBlend {
                factors: [0.0; 3],
                weights: [0.0; 3],
            },
            12.0,
        );
        assert_eq!(blend.calculated_output(), 12.0);
        let floor = ScoreStep::new("Minimum score", -2.0, ScoreOperation::Floor(0.0), 0.0);
        assert_eq!(floor.calculated_output(), 0.0);
        assert!(floor.to_string().contains("max(-2.0000, 0.0000)"));
    }
}
