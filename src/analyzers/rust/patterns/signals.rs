//! Pattern signal detection
//!
//! Detects validation, state machine, and coordinator patterns.

use crate::analyzers::rust::types::PatternSignals;
use crate::analyzers::state_machine_pattern_detector::StateMachinePatternDetector;
use crate::analyzers::validation_pattern_detector::ValidationPatternDetector;
use crate::config::get_state_detection_config;

/// Detect validation, state machine, and coordinator patterns (specs 179, 180)
pub fn detect_pattern_signals(block: &syn::Block, func_name: &str) -> PatternSignals {
    let validation = ValidationPatternDetector::new().detect(block, func_name);
    let state_detector = StateMachinePatternDetector::with_config(get_state_detection_config());

    PatternSignals {
        validation,
        state_machine: state_detector.detect_state_machine(block),
        coordinator: state_detector.detect_coordinator(block),
    }
}
