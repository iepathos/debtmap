//! Pattern extraction functions for complexity analysis
//!
//! Provides functions to extract complexity patterns from recommendation text
//! and language-specific data (state machines, coordinators).

use crate::core::LanguageSpecificData;
use crate::io::writers::pattern_display::PATTERN_CONFIDENCE_THRESHOLD;

/// Extract complexity pattern from recommendation text
pub fn extract_complexity_pattern(rationale: &str, action: &str) -> Option<String> {
    // Check for moderate complexity (preventive)
    if action.contains("Maintain current low complexity")
        || action.contains("approaching thresholds")
    {
        return Some("ModerateComplexity".to_string());
    }

    // Check for specific patterns in the rationale
    if rationale.contains("Deep nesting") || rationale.contains("nesting is primary issue") {
        Some("DeepNesting".to_string())
    } else if rationale.contains("Many decision points")
        || rationale.contains("branches) drive cyclomatic")
    {
        Some("HighBranching".to_string())
    } else if rationale.contains("State machine pattern") {
        Some("StateMachine".to_string())
    } else if rationale.contains("High token entropy")
        || rationale.contains("inconsistent structure")
    {
        Some("ChaoticStructure".to_string())
    } else if action.contains("Clean dispatcher pattern") || rationale.contains("dispatcher") {
        Some("Dispatcher".to_string())
    } else if rationale.contains("repetitive validation")
        || rationale.contains("Repetitive validation")
    {
        Some("RepetitiveValidation".to_string())
    } else if rationale.contains("coordinator") || rationale.contains("orchestrat") {
        Some("Coordinator".to_string())
    } else if rationale.contains("nesting and branching") || action.contains("two-phase approach") {
        Some("MixedComplexity".to_string())
    } else {
        None
    }
}

/// Extract pattern data from language-specific information
///
/// Returns (pattern_type, confidence, details) if a pattern is detected with sufficient confidence
pub fn extract_pattern_data(
    language_specific: &Option<LanguageSpecificData>,
) -> (Option<String>, Option<f64>, Option<serde_json::Value>) {
    if let Some(LanguageSpecificData::Rust(rust_data)) = language_specific {
        // Check state machine first (higher priority)
        if let Some(sm_signals) = &rust_data.state_machine_signals {
            if sm_signals.confidence >= PATTERN_CONFIDENCE_THRESHOLD {
                let details = serde_json::json!({
                    "transition_count": sm_signals.transition_count,
                    "match_expression_count": sm_signals.match_expression_count,
                    "action_dispatch_count": sm_signals.action_dispatch_count,
                });
                return (
                    Some("state_machine".to_string()),
                    Some(sm_signals.confidence),
                    Some(details),
                );
            }
        }

        // Check coordinator second
        if let Some(coord_signals) = &rust_data.coordinator_signals {
            if coord_signals.confidence >= PATTERN_CONFIDENCE_THRESHOLD {
                let details = serde_json::json!({
                    "actions": coord_signals.actions,
                    "comparisons": coord_signals.comparisons,
                });
                return (
                    Some("coordinator".to_string()),
                    Some(coord_signals.confidence),
                    Some(details),
                );
            }
        }
    }
    (None, None, None)
}
