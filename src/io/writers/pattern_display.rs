/// Pattern display utilities for showing state machine and coordinator patterns
/// across different output formats.
///
/// This module provides pure functions for extracting and formatting pattern information
/// from FunctionMetrics, following the functional programming principles of debtmap.
use crate::core::{FunctionMetrics, LanguageSpecificData};
use crate::priority::complexity_patterns::{CoordinatorSignals, StateMachineSignals};

/// Confidence threshold for displaying patterns (0.0-1.0)
pub const PATTERN_CONFIDENCE_THRESHOLD: f64 = 0.7;

/// Type of complexity pattern detected
#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    StateMachine,
    Coordinator,
}

impl PatternType {
    /// Returns the display name for the pattern type
    pub fn display_name(&self) -> &'static str {
        match self {
            PatternType::StateMachine => "State Machine",
            PatternType::Coordinator => "Coordinator",
        }
    }

    /// Returns the icon for the pattern type (for terminal output)
    pub fn icon(&self) -> &'static str {
        match self {
            PatternType::StateMachine => "🔄",
            PatternType::Coordinator => "🎯",
        }
    }
}

/// Information about a detected complexity pattern
#[derive(Debug, Clone)]
pub struct PatternInfo {
    /// The type of pattern detected
    pub pattern_type: PatternType,
    /// Detection confidence (0.0-1.0, >= 0.7 required)
    pub confidence: f64,
    /// Pattern-specific metrics for display (key-value pairs)
    pub display_metrics: Vec<(String, String)>,
}

impl PatternInfo {
    /// Creates PatternInfo from state machine signals
    pub fn from_state_machine(signals: &StateMachineSignals) -> Self {
        let display_metrics = vec![
            (
                "transitions".to_string(),
                signals.transition_count.to_string(),
            ),
            (
                "matches".to_string(),
                signals.match_expression_count.to_string(),
            ),
            (
                "actions".to_string(),
                signals.action_dispatch_count.to_string(),
            ),
        ];

        Self {
            pattern_type: PatternType::StateMachine,
            confidence: signals.confidence,
            display_metrics,
        }
    }

    /// Creates PatternInfo from coordinator signals
    pub fn from_coordinator(signals: &CoordinatorSignals) -> Self {
        let display_metrics = vec![
            ("actions".to_string(), signals.actions.to_string()),
            ("comparisons".to_string(), signals.comparisons.to_string()),
        ];

        Self {
            pattern_type: PatternType::Coordinator,
            confidence: signals.confidence,
            display_metrics,
        }
    }

    /// Formats pattern info for terminal display
    pub fn format_terminal(&self) -> String {
        let metrics_str = self
            .display_metrics
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "{} {} ({}, confidence: {:.2})",
            self.pattern_type.icon(),
            self.pattern_type.display_name(),
            metrics_str,
            self.confidence
        )
    }

    /// Formats pattern info for markdown table display
    pub fn format_markdown_type(&self) -> String {
        self.pattern_type.display_name().to_string()
    }

    /// Formats confidence for markdown table display
    pub fn format_markdown_confidence(&self) -> String {
        format!("{:.2}", self.confidence)
    }
}

/// Extracts pattern information from function metrics.
///
/// Returns pattern info if confidence >= 0.7, otherwise None.
/// Prioritizes state machine over coordinator when both detected.
///
/// # Pure Function
/// This function has no side effects and always returns the same output for the same input.
pub fn extract_pattern_info(metrics: &FunctionMetrics) -> Option<PatternInfo> {
    if let Some(LanguageSpecificData::Rust(rust_data)) = &metrics.language_specific {
        // Check state machine first (higher priority)
        if let Some(sm_signals) = &rust_data.state_machine_signals {
            if sm_signals.confidence >= PATTERN_CONFIDENCE_THRESHOLD {
                return Some(PatternInfo::from_state_machine(sm_signals));
            }
        }

        // Check coordinator second
        if let Some(coord_signals) = &rust_data.coordinator_signals {
            if coord_signals.confidence >= PATTERN_CONFIDENCE_THRESHOLD {
                return Some(PatternInfo::from_coordinator(coord_signals));
            }
        }
    }

    None
}

/// Formats pattern type for markdown table (or "-" if none)
///
/// # Pure Function
/// This function has no side effects and always returns the same output for the same input.
pub fn format_pattern_type(metrics: &FunctionMetrics) -> String {
    extract_pattern_info(metrics)
        .map(|info| info.format_markdown_type())
        .unwrap_or_else(|| "-".to_string())
}

/// Formats pattern confidence for markdown table (or "-" if none)
///
/// # Pure Function
/// This function has no side effects and always returns the same output for the same input.
pub fn format_pattern_confidence(metrics: &FunctionMetrics) -> String {
    extract_pattern_info(metrics)
        .map(|info| info.format_markdown_confidence())
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::rust_patterns::RustPatternResult;
    use std::path::PathBuf;

    fn create_test_metrics(rust_data: RustPatternResult) -> FunctionMetrics {
        FunctionMetrics {
            name: "test_function".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 10,
            cognitive: 15,
            nesting: 2,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: Some(LanguageSpecificData::Rust(rust_data)),
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }
    }

    #[test]
    fn test_extract_state_machine_pattern_high_confidence() {
        let signals = StateMachineSignals {
            transition_count: 4,
            match_expression_count: 2,
            has_enum_match: true,
            has_state_comparison: true,
            action_dispatch_count: 8,
            confidence: 0.85,
            ..Default::default()
        };

        let rust_data = RustPatternResult {
            trait_impl: None,
            async_patterns: vec![],
            error_patterns: vec![],
            builder_patterns: vec![],
            validation_signals: None,
            state_machine_signals: Some(signals),
            coordinator_signals: None,
        };

        let metrics = create_test_metrics(rust_data);
        let info = extract_pattern_info(&metrics);

        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.pattern_type, PatternType::StateMachine);
        assert_eq!(info.confidence, 0.85);
        assert_eq!(info.display_metrics.len(), 3);
    }

    #[test]
    fn test_extract_coordinator_pattern_high_confidence() {
        let signals = CoordinatorSignals {
            actions: 5,
            comparisons: 3,
            has_action_accumulation: true,
            has_helper_calls: true,
            confidence: 0.78,
        };

        let rust_data = RustPatternResult {
            trait_impl: None,
            async_patterns: vec![],
            error_patterns: vec![],
            builder_patterns: vec![],
            validation_signals: None,
            state_machine_signals: None,
            coordinator_signals: Some(signals),
        };

        let metrics = create_test_metrics(rust_data);
        let info = extract_pattern_info(&metrics);

        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.pattern_type, PatternType::Coordinator);
        assert_eq!(info.confidence, 0.78);
        assert_eq!(info.display_metrics.len(), 2);
    }

    #[test]
    fn test_low_confidence_pattern_not_extracted() {
        let signals = StateMachineSignals {
            transition_count: 1,
            match_expression_count: 1,
            has_enum_match: false,
            has_state_comparison: true,
            action_dispatch_count: 1,
            confidence: 0.5, // Below threshold
            ..Default::default()
        };

        let rust_data = RustPatternResult {
            trait_impl: None,
            async_patterns: vec![],
            error_patterns: vec![],
            builder_patterns: vec![],
            validation_signals: None,
            state_machine_signals: Some(signals),
            coordinator_signals: None,
        };

        let metrics = create_test_metrics(rust_data);
        let info = extract_pattern_info(&metrics);

        assert!(info.is_none());
    }

    #[test]
    fn test_state_machine_prioritized_over_coordinator() {
        let sm_signals = StateMachineSignals {
            transition_count: 4,
            match_expression_count: 2,
            has_enum_match: true,
            has_state_comparison: true,
            action_dispatch_count: 8,
            confidence: 0.75,
            ..Default::default()
        };

        let coord_signals = CoordinatorSignals {
            actions: 5,
            comparisons: 3,
            has_action_accumulation: true,
            has_helper_calls: true,
            confidence: 0.85, // Higher confidence but lower priority
        };

        let rust_data = RustPatternResult {
            trait_impl: None,
            async_patterns: vec![],
            error_patterns: vec![],
            builder_patterns: vec![],
            validation_signals: None,
            state_machine_signals: Some(sm_signals),
            coordinator_signals: Some(coord_signals),
        };

        let metrics = create_test_metrics(rust_data);
        let info = extract_pattern_info(&metrics);

        assert!(info.is_some());
        let info = info.unwrap();
        // State machine should be returned even though coordinator has higher confidence
        assert_eq!(info.pattern_type, PatternType::StateMachine);
    }

    #[test]
    fn test_format_pattern_type() {
        let signals = StateMachineSignals {
            transition_count: 4,
            match_expression_count: 2,
            has_enum_match: true,
            has_state_comparison: true,
            action_dispatch_count: 8,
            confidence: 0.85,
            ..Default::default()
        };

        let rust_data = RustPatternResult {
            trait_impl: None,
            async_patterns: vec![],
            error_patterns: vec![],
            builder_patterns: vec![],
            validation_signals: None,
            state_machine_signals: Some(signals),
            coordinator_signals: None,
        };

        let metrics = create_test_metrics(rust_data);
        assert_eq!(format_pattern_type(&metrics), "State Machine");
    }

    #[test]
    fn test_format_pattern_confidence() {
        let signals = CoordinatorSignals {
            actions: 5,
            comparisons: 3,
            has_action_accumulation: true,
            has_helper_calls: true,
            confidence: 0.8567,
        };

        let rust_data = RustPatternResult {
            trait_impl: None,
            async_patterns: vec![],
            error_patterns: vec![],
            builder_patterns: vec![],
            validation_signals: None,
            state_machine_signals: None,
            coordinator_signals: Some(signals),
        };

        let metrics = create_test_metrics(rust_data);
        assert_eq!(format_pattern_confidence(&metrics), "0.86");
    }

    #[test]
    fn test_format_no_pattern_returns_dash() {
        let rust_data = RustPatternResult {
            trait_impl: None,
            async_patterns: vec![],
            error_patterns: vec![],
            builder_patterns: vec![],
            validation_signals: None,
            state_machine_signals: None,
            coordinator_signals: None,
        };

        let metrics = create_test_metrics(rust_data);
        assert_eq!(format_pattern_type(&metrics), "-");
        assert_eq!(format_pattern_confidence(&metrics), "-");
    }

    #[test]
    fn test_pattern_info_terminal_formatting() {
        let signals = StateMachineSignals {
            transition_count: 4,
            match_expression_count: 2,
            has_enum_match: true,
            has_state_comparison: true,
            action_dispatch_count: 8,
            confidence: 0.85,
            ..Default::default()
        };

        let info = PatternInfo::from_state_machine(&signals);
        let formatted = info.format_terminal();

        assert!(formatted.contains("🔄"));
        assert!(formatted.contains("State Machine"));
        assert!(formatted.contains("transitions: 4"));
        assert!(formatted.contains("0.85"));
    }
}
