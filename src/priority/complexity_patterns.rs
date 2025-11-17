//! # Complexity Pattern Detection
//!
//! Classifies complexity hotspots by their primary driver:
//! - **High Nesting**: Cognitive >> Cyclomatic (deep conditionals)
//! - **High Branching**: Many decision points, moderate depth
//! - **Mixed Complexity**: Both nesting and branching high
//! - **Chaotic Structure**: High entropy, inconsistent patterns
//! - **Moderate Complexity**: Approaching thresholds
//!
//! Each pattern gets tailored refactoring recommendations based on
//! the root cause identified through metric ratio analysis.

use serde::{Deserialize, Serialize};

/// Complexity pattern classification based on metric ratios
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplexityPattern {
    /// State machine pattern: nested conditionals on enum states
    StateMachine {
        state_transitions: u32,
        cyclomatic: u32,
        cognitive: u32,
        nesting: u32,
    },
    /// Coordinator pattern: orchestrates actions based on state comparisons
    Coordinator {
        action_count: u32,
        comparison_count: u32,
        cyclomatic: u32,
        cognitive: u32,
    },
    /// Repetitive validation pattern: many early returns with same structure
    RepetitiveValidation {
        validation_count: u32,    // Number of validation checks
        entropy: f64,             // Token entropy (low = repetitive)
        cyclomatic: u32,          // Raw cyclomatic (before dampening)
        adjusted_cyclomatic: u32, // Dampened complexity (reflects cognitive load)
    },
    /// Deep nesting drives complexity (cognitive >> cyclomatic)
    HighNesting {
        nesting_depth: u32,
        cognitive_score: u32,
        ratio: f64, // cognitive/cyclomatic
    },
    /// Many decision points (high cyclomatic, moderate cognitive)
    HighBranching { branch_count: u32, cyclomatic: u32 },
    /// Both nesting and branching contribute to complexity
    MixedComplexity {
        nesting_depth: u32,
        cyclomatic: u32,
        cognitive: u32,
    },
    /// Inconsistent structure (high entropy)
    ChaoticStructure { entropy: f64, cyclomatic: u32 },
    /// Approaching complexity thresholds
    ModerateComplexity { cyclomatic: u32, cognitive: u32 },
}

/// Signals indicating state machine pattern
#[derive(Debug, Clone)]
pub struct StateMachineSignals {
    pub transition_count: u32,
    pub has_enum_match: bool,
    pub has_state_comparison: bool,
    pub action_dispatch_count: u32,
    pub confidence: f64,
}

/// Signals indicating coordinator pattern
#[derive(Debug, Clone)]
pub struct CoordinatorSignals {
    pub actions: u32,
    pub comparisons: u32,
    pub has_action_accumulation: bool,
    pub has_helper_calls: bool,
    pub confidence: f64,
}

/// Signals indicating repetitive validation pattern
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ValidationSignals {
    pub check_count: u32,
    pub early_return_count: u32,
    pub structural_similarity: f64,
    pub has_validation_name: bool,
    pub confidence: f64,
}

/// Complexity metrics for pattern detection
#[derive(Debug, Clone)]
pub struct ComplexityMetrics {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub entropy_score: Option<f64>,
    pub state_signals: Option<StateMachineSignals>,
    pub coordinator_signals: Option<CoordinatorSignals>,
    pub validation_signals: Option<ValidationSignals>,
}

impl ComplexityPattern {
    /// Detect complexity pattern from metrics.
    ///
    /// # Pattern Detection Logic
    ///
    /// 1. **State Machine** (checked first): High-confidence state transition signals
    ///    - Detects functions with nested conditionals on enum states
    ///    - Requires: cyclomatic >= 6, cognitive >= 12, confidence >= 0.7
    ///    - Refactoring: extract state transition functions, create transition map
    ///
    /// 2. **Coordinator** (checked second): Action accumulation and state comparisons
    ///    - Detects functions that orchestrate actions based on state
    ///    - Requires: actions >= 3, comparisons >= 2, confidence >= 0.7
    ///    - Refactoring: extract reconciliation logic into transition map
    ///
    /// 3. **Chaotic Structure**: token_entropy >= 0.45
    ///    - Uses token_entropy (Shannon entropy of code tokens, 0.0-1.0 scale)
    ///    - Threshold 0.45 chosen empirically (typical range: 0.2-0.8)
    ///    - High token entropy indicates inconsistent patterns that make refactoring risky
    ///    - Should be standardized before other refactorings
    ///
    /// 4. **High Nesting**: cognitive/cyclomatic > 3.0 AND nesting >= 4
    ///    - Cognitive dominates cyclomatic (high ratio)
    ///    - Deep nesting (4+ levels) is the primary driver
    ///    - Refactoring: early returns, guard clauses, extract conditionals
    ///
    /// 5. **High Branching**: cyclomatic >= 15 AND ratio < 2.5
    ///    - Many decision points with moderate nesting
    ///    - Refactoring: extract functions, lookup tables, strategy pattern
    ///
    /// 6. **Mixed Complexity**: cyclomatic >= 12 AND cognitive >= 40 AND 2.5 <= ratio <= 3.5
    ///    - Both nesting and branching contribute significantly
    ///    - Refactoring: two-phase approach (flatten then extract)
    ///
    /// 7. **Moderate Complexity**: default
    ///    - Approaching thresholds but not critical
    ///    - Preventive refactoring recommended
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::priority::complexity_patterns::{ComplexityPattern, ComplexityMetrics};
    ///
    /// // High nesting example
    /// let metrics = ComplexityMetrics {
    ///     cyclomatic: 12,
    ///     cognitive: 50,  // 4.2x ratio
    ///     nesting: 5,
    ///     entropy_score: Some(0.35),
    ///     state_signals: None,
    ///     coordinator_signals: None,
    ///     validation_signals: None,
    /// };
    /// let pattern = ComplexityPattern::detect(&metrics);
    /// assert!(matches!(pattern, ComplexityPattern::HighNesting { .. }));
    ///
    /// // High branching example
    /// let metrics = ComplexityMetrics {
    ///     cyclomatic: 18,
    ///     cognitive: 35,  // 1.9x ratio
    ///     nesting: 2,
    ///     entropy_score: Some(0.30),
    ///     state_signals: None,
    ///     coordinator_signals: None,
    ///     validation_signals: None,
    /// };
    /// let pattern = ComplexityPattern::detect(&metrics);
    /// assert!(matches!(pattern, ComplexityPattern::HighBranching { .. }));
    /// ```
    pub fn detect(metrics: &ComplexityMetrics) -> Self {
        let ratio = metrics.cognitive as f64 / metrics.cyclomatic.max(1) as f64;

        // Check for repetitive validation pattern FIRST (low entropy + high branching)
        // This prevents validation boilerplate from being misclassified as high complexity
        if let Some(entropy) = metrics.entropy_score {
            if is_repetitive_validation(metrics.cyclomatic, entropy, &metrics.validation_signals) {
                let adjusted = dampen_complexity_for_repetition(metrics.cyclomatic, entropy);
                return ComplexityPattern::RepetitiveValidation {
                    validation_count: metrics
                        .validation_signals
                        .as_ref()
                        .map(|v| v.check_count)
                        .unwrap_or(metrics.cyclomatic),
                    entropy,
                    cyclomatic: metrics.cyclomatic,
                    adjusted_cyclomatic: adjusted,
                };
            }
        }

        // Check for state machine pattern (highest priority - specific, high-value)
        if let Some(ref state_signals) = metrics.state_signals {
            if state_signals.confidence >= 0.7 && metrics.cyclomatic >= 6 && metrics.cognitive >= 12
            {
                return ComplexityPattern::StateMachine {
                    state_transitions: state_signals.transition_count,
                    cyclomatic: metrics.cyclomatic,
                    cognitive: metrics.cognitive,
                    nesting: metrics.nesting,
                };
            }
        }

        // Check for coordinator pattern (second priority - specific, high-value)
        if let Some(ref coord_signals) = metrics.coordinator_signals {
            if coord_signals.confidence >= 0.7
                && coord_signals.actions >= 3
                && coord_signals.comparisons >= 2
            {
                return ComplexityPattern::Coordinator {
                    action_count: coord_signals.actions,
                    comparison_count: coord_signals.comparisons,
                    cyclomatic: metrics.cyclomatic,
                    cognitive: metrics.cognitive,
                };
            }
        }

        // Chaotic: high token entropy (check before generic patterns - requires standardization)
        // Note: entropy_score here is token_entropy (Shannon entropy), not effective_complexity
        if let Some(token_entropy) = metrics.entropy_score {
            if token_entropy >= 0.45 {
                return ComplexityPattern::ChaoticStructure {
                    entropy: token_entropy,
                    cyclomatic: metrics.cyclomatic,
                };
            }
        }

        // High nesting: cognitive dominates
        if ratio > 3.0 && metrics.nesting >= 4 {
            return ComplexityPattern::HighNesting {
                nesting_depth: metrics.nesting,
                cognitive_score: metrics.cognitive,
                ratio,
            };
        }

        // High branching: cyclomatic high, ratio moderate
        if metrics.cyclomatic >= 15 && ratio < 2.5 {
            return ComplexityPattern::HighBranching {
                branch_count: metrics.cyclomatic,
                cyclomatic: metrics.cyclomatic,
            };
        }

        // Mixed: both high
        if metrics.cyclomatic >= 12 && metrics.cognitive >= 40 && (2.5..=3.5).contains(&ratio) {
            return ComplexityPattern::MixedComplexity {
                nesting_depth: metrics.nesting,
                cyclomatic: metrics.cyclomatic,
                cognitive: metrics.cognitive,
            };
        }

        // Default: moderate
        ComplexityPattern::ModerateComplexity {
            cyclomatic: metrics.cyclomatic,
            cognitive: metrics.cognitive,
        }
    }

    /// Get a human-readable description of the pattern
    pub fn description(&self) -> &'static str {
        match self {
            ComplexityPattern::StateMachine { .. } => "State machine with transition logic",
            ComplexityPattern::Coordinator { .. } => {
                "Coordinator orchestrating state-based actions"
            }
            ComplexityPattern::RepetitiveValidation { .. } => "Repetitive validation boilerplate",
            ComplexityPattern::HighNesting { .. } => "Deep nesting drives complexity",
            ComplexityPattern::HighBranching { .. } => "Many decision points",
            ComplexityPattern::MixedComplexity { .. } => "Both nesting and branching high",
            ComplexityPattern::ChaoticStructure { .. } => "Inconsistent structure patterns",
            ComplexityPattern::ModerateComplexity { .. } => "Approaching complexity thresholds",
        }
    }
}

/// Determine if metrics indicate repetitive validation pattern
fn is_repetitive_validation(
    cyclomatic: u32,
    entropy: f64,
    validation_signals: &Option<ValidationSignals>,
) -> bool {
    // Low entropy + high branching is primary signal
    let has_low_entropy_high_branching = entropy < 0.35 && cyclomatic >= 10;

    if !has_low_entropy_high_branching {
        return false;
    }

    // Additional validation signals strengthen confidence
    if let Some(signals) = validation_signals {
        // Require majority of branches to be early returns
        let early_return_ratio = signals.early_return_count as f64 / cyclomatic as f64;

        if early_return_ratio < 0.6 {
            return false;
        }

        // High structural similarity (measured by AST pattern matching)
        if signals.structural_similarity < 0.7 {
            return false;
        }

        true
    } else {
        // Without validation signals, we can't confirm it's a validation pattern
        // Return false to avoid false positives
        false
    }
}

/// Dampen cyclomatic complexity for repetitive patterns
fn dampen_complexity_for_repetition(cyclomatic: u32, entropy: f64) -> u32 {
    // Lower entropy = more dampening (recognition of low cognitive load)
    // entropy < 0.25: 60% dampening (very repetitive)
    // entropy < 0.30: 50% dampening (highly repetitive)
    // entropy < 0.35: 40% dampening (moderately repetitive)
    let dampening_factor = if entropy < 0.25 {
        0.4
    } else if entropy < 0.30 {
        0.5
    } else {
        0.6
    };

    (cyclomatic as f64 * dampening_factor).ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_high_nesting_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 12,
            cognitive: 50, // 4.2x ratio
            nesting: 5,
            entropy_score: Some(0.35),
            state_signals: None,
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(pattern, ComplexityPattern::HighNesting { .. }));

        if let ComplexityPattern::HighNesting {
            nesting_depth,
            cognitive_score,
            ratio,
        } = pattern
        {
            assert_eq!(nesting_depth, 5);
            assert_eq!(cognitive_score, 50);
            assert!((ratio - 4.17).abs() < 0.01);
        }
    }

    #[test]
    fn detect_high_branching_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 18,
            cognitive: 35, // 1.9x ratio
            nesting: 2,
            entropy_score: Some(0.30),
            state_signals: None,
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(pattern, ComplexityPattern::HighBranching { .. }));
    }

    #[test]
    fn detect_mixed_complexity_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 15,
            cognitive: 45, // 3.0x ratio
            nesting: 3,
            entropy_score: Some(0.32),
            state_signals: None,
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(pattern, ComplexityPattern::MixedComplexity { .. }));
    }

    #[test]
    fn detect_chaotic_structure_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 12,
            cognitive: 30,
            nesting: 3,
            entropy_score: Some(0.50), // High entropy
            state_signals: None,
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(
            pattern,
            ComplexityPattern::ChaoticStructure { .. }
        ));
    }

    #[test]
    fn detect_moderate_complexity_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 11,
            cognitive: 18,
            nesting: 2,
            entropy_score: Some(0.30),
            state_signals: None,
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(
            pattern,
            ComplexityPattern::ModerateComplexity { .. }
        ));
    }

    #[test]
    fn chaotic_takes_precedence_over_nesting() {
        // High nesting metrics BUT high entropy
        let metrics = ComplexityMetrics {
            cyclomatic: 12,
            cognitive: 50,
            nesting: 5,
            entropy_score: Some(0.48), // High entropy takes precedence
            state_signals: None,
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(
            matches!(pattern, ComplexityPattern::ChaoticStructure { .. }),
            "Chaotic structure should be detected before high nesting"
        );
    }

    #[test]
    fn ratio_boundary_conditions() {
        // Exactly at high nesting threshold
        let metrics = ComplexityMetrics {
            cyclomatic: 10,
            cognitive: 30, // Exactly 3.0x
            nesting: 4,
            entropy_score: Some(0.30),
            state_signals: None,
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        // ratio > 3.0 requires strictly greater, so this should NOT be HighNesting
        assert!(
            !matches!(pattern, ComplexityPattern::HighNesting { .. }),
            "Exactly 3.0 ratio should not trigger HighNesting (requires > 3.0)"
        );
    }

    #[test]
    fn handles_zero_cyclomatic() {
        // Edge case: cyclomatic = 0 (shouldn't happen but test defensive coding)
        let metrics = ComplexityMetrics {
            cyclomatic: 0,
            cognitive: 10,
            nesting: 2,
            entropy_score: Some(0.30),
            state_signals: None,
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        // Should use max(1) to avoid division by zero
        assert!(matches!(
            pattern,
            ComplexityPattern::ModerateComplexity { .. }
        ));
    }

    #[test]
    fn detect_state_machine_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 9,
            cognitive: 16,
            nesting: 4,
            entropy_score: Some(0.32),
            state_signals: Some(StateMachineSignals {
                transition_count: 3,
                has_enum_match: true,
                has_state_comparison: true,
                action_dispatch_count: 4,
                confidence: 0.85,
            }),
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(pattern, ComplexityPattern::StateMachine { .. }));

        if let ComplexityPattern::StateMachine {
            state_transitions, ..
        } = pattern
        {
            assert_eq!(state_transitions, 3);
        }
    }

    #[test]
    fn detect_coordinator_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 8,
            cognitive: 14,
            nesting: 3,
            entropy_score: Some(0.28),
            state_signals: None,
            coordinator_signals: Some(CoordinatorSignals {
                actions: 4,
                comparisons: 2,
                has_action_accumulation: true,
                has_helper_calls: true,
                confidence: 0.80,
            }),
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(pattern, ComplexityPattern::Coordinator { .. }));
    }

    #[test]
    fn state_pattern_takes_precedence_over_nesting() {
        // High nesting metrics BUT state machine signals
        let metrics = ComplexityMetrics {
            cyclomatic: 12,
            cognitive: 50,
            nesting: 5,
            entropy_score: Some(0.35),
            state_signals: Some(StateMachineSignals {
                transition_count: 4,
                has_enum_match: true,
                has_state_comparison: true,
                action_dispatch_count: 6,
                confidence: 0.90,
            }),
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(
            matches!(pattern, ComplexityPattern::StateMachine { .. }),
            "State machine pattern should take precedence over generic high nesting"
        );
    }

    #[test]
    fn pattern_descriptions() {
        assert_eq!(
            ComplexityPattern::StateMachine {
                state_transitions: 3,
                cyclomatic: 9,
                cognitive: 16,
                nesting: 4,
            }
            .description(),
            "State machine with transition logic"
        );

        assert_eq!(
            ComplexityPattern::Coordinator {
                action_count: 4,
                comparison_count: 2,
                cyclomatic: 8,
                cognitive: 14,
            }
            .description(),
            "Coordinator orchestrating state-based actions"
        );

        assert_eq!(
            ComplexityPattern::HighNesting {
                nesting_depth: 5,
                cognitive_score: 50,
                ratio: 4.0
            }
            .description(),
            "Deep nesting drives complexity"
        );

        assert_eq!(
            ComplexityPattern::HighBranching {
                branch_count: 18,
                cyclomatic: 18
            }
            .description(),
            "Many decision points"
        );
    }

    #[test]
    fn detect_repetitive_validation_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 20,
            cognitive: 25,
            nesting: 1,
            entropy_score: Some(0.28),
            state_signals: None,
            coordinator_signals: None,
            validation_signals: Some(ValidationSignals {
                check_count: 20,
                early_return_count: 20,
                structural_similarity: 0.95,
                has_validation_name: true,
                confidence: 0.9,
            }),
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(
            pattern,
            ComplexityPattern::RepetitiveValidation { .. }
        ));

        if let ComplexityPattern::RepetitiveValidation {
            validation_count,
            entropy,
            cyclomatic,
            adjusted_cyclomatic,
        } = pattern
        {
            assert_eq!(validation_count, 20);
            assert_eq!(cyclomatic, 20);
            assert_eq!(adjusted_cyclomatic, 10); // 20 * 0.5 dampening
            assert!((entropy - 0.28).abs() < 0.01);
        }
    }

    #[test]
    fn validation_pattern_takes_precedence_over_high_branching() {
        // High branching metrics BUT repetitive validation signals
        let metrics = ComplexityMetrics {
            cyclomatic: 18,
            cognitive: 20,
            nesting: 1,
            entropy_score: Some(0.30),
            state_signals: None,
            coordinator_signals: None,
            validation_signals: Some(ValidationSignals {
                check_count: 18,
                early_return_count: 18,
                structural_similarity: 0.92,
                has_validation_name: true,
                confidence: 0.85,
            }),
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(
            matches!(pattern, ComplexityPattern::RepetitiveValidation { .. }),
            "Repetitive validation should take precedence over high branching"
        );
    }

    #[test]
    fn high_entropy_prevents_validation_detection() {
        // High branching but HIGH entropy (varied logic)
        let metrics = ComplexityMetrics {
            cyclomatic: 20,
            cognitive: 45,
            nesting: 2,
            entropy_score: Some(0.55),
            state_signals: None,
            coordinator_signals: None,
            validation_signals: None,
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(
            !matches!(pattern, ComplexityPattern::RepetitiveValidation { .. }),
            "High entropy should prevent validation pattern detection"
        );
    }

    #[test]
    fn dampening_factor_scales_with_entropy() {
        assert_eq!(dampen_complexity_for_repetition(20, 0.20), 8); // 0.4 factor
        assert_eq!(dampen_complexity_for_repetition(20, 0.28), 10); // 0.5 factor
        assert_eq!(dampen_complexity_for_repetition(20, 0.33), 12); // 0.6 factor
    }
}
