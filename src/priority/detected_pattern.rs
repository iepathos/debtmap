//! # Detected Pattern
//!
//! Single source of truth for complexity pattern detection in debtmap.
//! Patterns are detected once during analysis and stored in `UnifiedDebtItem`,
//! ensuring consistency across all output formatters.
//!
//! ## Pattern Types
//!
//! - **State Machine**: Functions with explicit state transitions, match expressions
//! - **Coordinator**: Functions that orchestrate actions based on comparisons
//! - **Validator**: Functions with validation logic (future)
//!
//! ## Confidence Threshold
//!
//! Patterns are only reported if confidence ≥ 0.7
//!
//! ## Examples
//!
//! ```
//! use debtmap::priority::detected_pattern::DetectedPattern;
//! use debtmap::core::LanguageSpecificData;
//!
//! let pattern = DetectedPattern::detect(&language_specific);
//! if let Some(p) = pattern {
//!     println!("{} {}", p.icon(), p.type_name());
//! }
//! ```

use crate::core::LanguageSpecificData;
use serde::{Deserialize, Serialize};

/// Detected complexity pattern with confidence and metrics.
///
/// Patterns are detected once during analysis and stored in `UnifiedDebtItem`.
/// All output formatters read from this stored result for consistency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub metrics: PatternMetrics,
}

/// Type of detected complexity pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    StateMachine,
    Coordinator,
    Validator,
}

/// Pattern-specific metrics (flexible for different patterns)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternMetrics {
    pub state_transitions: Option<usize>,
    pub match_expressions: Option<usize>,
    pub action_dispatches: Option<usize>,
    pub comparisons: Option<usize>,
}

impl DetectedPattern {
    /// Detect pattern from language-specific signals.
    ///
    /// Checks for state machine patterns first (higher priority), then coordinator patterns.
    /// Returns `None` if no pattern is detected or confidence is below threshold (0.7).
    pub fn detect(language_specific: &Option<LanguageSpecificData>) -> Option<Self> {
        let rust_data = match language_specific {
            Some(LanguageSpecificData::Rust(data)) => data,
            _ => return None,
        };

        // Check state machine first (higher priority)
        if let Some(sm_signals) = &rust_data.state_machine_signals {
            if sm_signals.confidence >= 0.7 {
                return Some(Self {
                    pattern_type: PatternType::StateMachine,
                    confidence: sm_signals.confidence,
                    metrics: PatternMetrics {
                        state_transitions: Some(sm_signals.transition_count as usize),
                        match_expressions: Some(sm_signals.match_expression_count as usize),
                        action_dispatches: Some(sm_signals.action_dispatch_count as usize),
                        comparisons: None,
                    },
                });
            }
        }

        // Check coordinator second
        if let Some(coord_signals) = &rust_data.coordinator_signals {
            if coord_signals.confidence >= 0.7 {
                return Some(Self {
                    pattern_type: PatternType::Coordinator,
                    confidence: coord_signals.confidence,
                    metrics: PatternMetrics {
                        state_transitions: None,
                        match_expressions: None,
                        action_dispatches: Some(coord_signals.actions as usize),
                        comparisons: Some(coord_signals.comparisons as usize),
                    },
                });
            }
        }

        None
    }

    /// Display icon for terminal output
    pub const fn icon(&self) -> &'static str {
        match self.pattern_type {
            PatternType::StateMachine => "🔄",
            PatternType::Coordinator => "🎯",
            PatternType::Validator => "✓",
        }
    }

    /// Display name for all output formats
    pub const fn type_name(&self) -> &'static str {
        match self.pattern_type {
            PatternType::StateMachine => "State Machine",
            PatternType::Coordinator => "Coordinator",
            PatternType::Validator => "Validator",
        }
    }

    /// Display metrics as formatted strings
    pub fn display_metrics(&self) -> Vec<String> {
        let mut metrics = Vec::new();

        if let Some(transitions) = self.metrics.state_transitions {
            metrics.push(format!("transitions: {}", transitions));
        }
        if let Some(matches) = self.metrics.match_expressions {
            metrics.push(format!("matches: {}", matches));
        }
        if let Some(actions) = self.metrics.action_dispatches {
            metrics.push(format!("actions: {}", actions));
        }
        if let Some(comparisons) = self.metrics.comparisons {
            metrics.push(format!("comparisons: {}", comparisons));
        }

        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::rust_patterns::RustPatternResult;
    use crate::priority::complexity_patterns::{CoordinatorSignals, StateMachineSignals};

    fn create_test_rust_data_with_state_machine() -> RustPatternResult {
        RustPatternResult {
            state_machine_signals: Some(StateMachineSignals {
                transition_count: 4,
                match_expression_count: 2,
                has_enum_match: true,
                has_state_comparison: true,
                action_dispatch_count: 8,
                confidence: 0.85,
            }),
            coordinator_signals: None,
            validation_signals: None,
        }
    }

    fn create_test_rust_data_with_coordinator() -> RustPatternResult {
        RustPatternResult {
            state_machine_signals: None,
            coordinator_signals: Some(CoordinatorSignals {
                actions: 4,
                comparisons: 2,
                has_action_accumulation: true,
                has_helper_calls: true,
                confidence: 0.80,
            }),
            validation_signals: None,
        }
    }

    #[test]
    fn detect_state_machine_pattern() {
        let rust_data = create_test_rust_data_with_state_machine();
        let pattern = DetectedPattern::detect(&Some(LanguageSpecificData::Rust(rust_data)));

        assert!(pattern.is_some());
        let pattern = pattern.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::StateMachine);
        assert!(pattern.confidence >= 0.7);
        assert_eq!(pattern.metrics.state_transitions, Some(4));
        assert_eq!(pattern.metrics.match_expressions, Some(2));
        assert_eq!(pattern.metrics.action_dispatches, Some(8));
    }

    #[test]
    fn detect_coordinator_pattern() {
        let rust_data = create_test_rust_data_with_coordinator();
        let pattern = DetectedPattern::detect(&Some(LanguageSpecificData::Rust(rust_data)));

        assert!(pattern.is_some());
        let pattern = pattern.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::Coordinator);
        assert_eq!(pattern.metrics.action_dispatches, Some(4));
        assert_eq!(pattern.metrics.comparisons, Some(2));
    }

    #[test]
    fn no_pattern_below_threshold() {
        let mut rust_data = create_test_rust_data_with_state_machine();
        rust_data.state_machine_signals.as_mut().unwrap().confidence = 0.6;

        let pattern = DetectedPattern::detect(&Some(LanguageSpecificData::Rust(rust_data)));
        assert!(pattern.is_none());
    }

    #[test]
    fn display_metrics_formatting() {
        let pattern = DetectedPattern {
            pattern_type: PatternType::Coordinator,
            confidence: 0.85,
            metrics: PatternMetrics {
                action_dispatches: Some(4),
                comparisons: Some(2),
                state_transitions: None,
                match_expressions: None,
            },
        };

        let metrics = pattern.display_metrics();
        assert_eq!(metrics, vec!["actions: 4", "comparisons: 2"]);
    }

    #[test]
    fn state_machine_priority_over_coordinator() {
        let rust_data = RustPatternResult {
            state_machine_signals: Some(StateMachineSignals {
                transition_count: 4,
                match_expression_count: 2,
                has_enum_match: true,
                has_state_comparison: true,
                action_dispatch_count: 8,
                confidence: 0.75,
            }),
            coordinator_signals: Some(CoordinatorSignals {
                actions: 4,
                comparisons: 2,
                has_action_accumulation: true,
                has_helper_calls: true,
                confidence: 0.80,
            }),
            validation_signals: None,
        };

        let pattern = DetectedPattern::detect(&Some(LanguageSpecificData::Rust(rust_data)));

        assert!(pattern.is_some());
        let pattern = pattern.unwrap();
        // State machine should be detected even though coordinator has higher confidence
        assert_eq!(pattern.pattern_type, PatternType::StateMachine);
    }

    #[test]
    fn no_pattern_for_non_rust() {
        let pattern = DetectedPattern::detect(&None);
        assert!(pattern.is_none());
    }
}
