//! Context detection for specialized code patterns
//!
//! This module detects the context of functions (formatter, parser, CLI handler, etc.)
//! to provide specialized, context-aware recommendations.

use crate::core::FunctionMetrics;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionContext {
    Formatter,
    Parser,
    CliHandler,
    StateMachine,
    Configuration,
    TestHelper,
    DatabaseQuery,
    Validator,
    Generic,
}

impl FunctionContext {
    pub fn display_name(&self) -> &'static str {
        match self {
            FunctionContext::Formatter => "Formatter",
            FunctionContext::Parser => "Parser",
            FunctionContext::CliHandler => "CLI Handler",
            FunctionContext::StateMachine => "State Machine",
            FunctionContext::Configuration => "Configuration",
            FunctionContext::TestHelper => "Test Helper",
            FunctionContext::DatabaseQuery => "Database Query",
            FunctionContext::Validator => "Validator",
            FunctionContext::Generic => "Generic",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextAnalysis {
    pub context: FunctionContext,
    pub confidence: f64,
    pub detected_signals: Vec<String>,
}

pub struct ContextDetector {
    // Cache compiled regexes for performance
    format_patterns: Vec<Regex>,
    parse_patterns: Vec<Regex>,
    cli_patterns: Vec<Regex>,
}

impl Default for ContextDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextDetector {
    pub fn new() -> Self {
        Self {
            format_patterns: vec![
                Regex::new(r"^format_").unwrap(),
                Regex::new(r"^render_").unwrap(),
                Regex::new(r"^display_").unwrap(),
                Regex::new(r"^to_string").unwrap(),
                Regex::new(r"^write_").unwrap(),
                Regex::new(r"_formatter$").unwrap(),
                Regex::new(r"_display$").unwrap(),
            ],
            parse_patterns: vec![
                Regex::new(r"^parse_").unwrap(),
                Regex::new(r"^read_").unwrap(),
                Regex::new(r"^decode_").unwrap(),
                Regex::new(r"^from_str").unwrap(),
                Regex::new(r"_parser$").unwrap(),
            ],
            cli_patterns: vec![
                Regex::new(r"^handle_").unwrap(),
                Regex::new(r"^cmd_").unwrap(),
                Regex::new(r"^command_").unwrap(),
                Regex::new(r"^execute_").unwrap(),
                Regex::new(r"^run_").unwrap(),
            ],
        }
    }

    /// Detect the context of a function
    pub fn detect_context(&self, function: &FunctionMetrics, file_path: &Path) -> ContextAnalysis {
        let signals = self.gather_signals(function, file_path);
        let context = self.classify_context(&signals);
        let confidence = self.calculate_confidence(&signals, &context);

        ContextAnalysis {
            context,
            confidence,
            detected_signals: signals.descriptions(),
        }
    }

    fn gather_signals(&self, function: &FunctionMetrics, file_path: &Path) -> ContextSignals {
        let file_path_str = file_path.to_string_lossy().to_lowercase();

        ContextSignals {
            function_name: function.name.to_lowercase(),
            in_formatter_file: file_path_str.contains("format")
                || file_path_str.contains("output")
                || file_path_str.contains("display"),
            in_parser_file: file_path_str.contains("parse") || file_path_str.contains("input"),
            in_cli_file: file_path_str.contains("cli")
                || file_path_str.contains("command")
                || file_path_str.contains("cmd"),
            in_config_file: file_path_str.contains("config"),
            in_db_file: file_path_str.contains("db")
                || file_path_str.contains("database")
                || file_path_str.contains("query"),
            has_validate_name: function.name.to_lowercase().contains("valid"),
            has_state_keywords: function.name.to_lowercase().contains("state")
                || function.name.to_lowercase().contains("transition"),
            is_test_helper: function.is_test || function.in_test_module,
        }
    }

    fn classify_context(&self, signals: &ContextSignals) -> FunctionContext {
        // Test helpers have high precedence
        if signals.is_test_helper {
            return FunctionContext::TestHelper;
        }

        // Name-based detection (high confidence)
        if self.matches_name_pattern(&signals.function_name, &self.format_patterns) {
            return FunctionContext::Formatter;
        }

        if self.matches_name_pattern(&signals.function_name, &self.parse_patterns) {
            return FunctionContext::Parser;
        }

        if self.matches_name_pattern(&signals.function_name, &self.cli_patterns) {
            return FunctionContext::CliHandler;
        }

        if signals.has_validate_name {
            return FunctionContext::Validator;
        }

        // File location-based detection (medium confidence)
        if signals.in_formatter_file {
            return FunctionContext::Formatter;
        }

        if signals.in_parser_file {
            return FunctionContext::Parser;
        }

        if signals.in_cli_file {
            return FunctionContext::CliHandler;
        }

        if signals.in_config_file {
            return FunctionContext::Configuration;
        }

        if signals.in_db_file {
            return FunctionContext::DatabaseQuery;
        }

        // State machine detection
        if signals.has_state_keywords {
            return FunctionContext::StateMachine;
        }

        FunctionContext::Generic
    }

    fn matches_name_pattern(&self, name: &str, patterns: &[Regex]) -> bool {
        patterns.iter().any(|pattern| pattern.is_match(name))
    }

    fn calculate_confidence(&self, signals: &ContextSignals, context: &FunctionContext) -> f64 {
        let signal_count = signals.matching_signal_count(context);

        match signal_count {
            0 => 0.1,  // Default/generic
            1 => 0.6,  // Single signal
            2 => 0.8,  // Two signals
            _ => 0.95, // Three or more signals
        }
    }
}

#[derive(Debug, Clone)]
struct ContextSignals {
    function_name: String,
    in_formatter_file: bool,
    in_parser_file: bool,
    in_cli_file: bool,
    in_config_file: bool,
    in_db_file: bool,
    has_validate_name: bool,
    has_state_keywords: bool,
    is_test_helper: bool,
}

impl ContextSignals {
    fn descriptions(&self) -> Vec<String> {
        let mut signals = Vec::new();

        if self.in_formatter_file {
            signals.push("Located in formatter/output file".to_string());
        }
        if self.in_parser_file {
            signals.push("Located in parser/input file".to_string());
        }
        if self.in_cli_file {
            signals.push("Located in CLI/command file".to_string());
        }
        if self.in_config_file {
            signals.push("Located in configuration file".to_string());
        }
        if self.in_db_file {
            signals.push("Located in database file".to_string());
        }
        if self.has_validate_name {
            signals.push("Name contains 'valid'".to_string());
        }
        if self.has_state_keywords {
            signals.push("Name contains state-related keywords".to_string());
        }
        if self.is_test_helper {
            signals.push("Is test or in test module".to_string());
        }

        signals
    }

    fn matching_signal_count(&self, context: &FunctionContext) -> usize {
        match context {
            FunctionContext::Formatter => {
                let mut count = 0;
                if self.function_name.contains("format")
                    || self.function_name.contains("render")
                    || self.function_name.contains("display")
                {
                    count += 1;
                }
                if self.in_formatter_file {
                    count += 1;
                }
                count
            }
            FunctionContext::Parser => {
                let mut count = 0;
                if self.function_name.contains("parse")
                    || self.function_name.contains("read")
                    || self.function_name.contains("decode")
                {
                    count += 1;
                }
                if self.in_parser_file {
                    count += 1;
                }
                count
            }
            FunctionContext::CliHandler => {
                let mut count = 0;
                if self.function_name.contains("handle")
                    || self.function_name.contains("cmd")
                    || self.function_name.contains("command")
                {
                    count += 1;
                }
                if self.in_cli_file {
                    count += 1;
                }
                count
            }
            FunctionContext::TestHelper => {
                if self.is_test_helper {
                    2
                } else {
                    0
                }
            }
            FunctionContext::Configuration => {
                if self.in_config_file {
                    1
                } else {
                    0
                }
            }
            FunctionContext::DatabaseQuery => {
                if self.in_db_file {
                    1
                } else {
                    0
                }
            }
            FunctionContext::Validator => {
                if self.has_validate_name {
                    1
                } else {
                    0
                }
            }
            FunctionContext::StateMachine => {
                if self.has_state_keywords {
                    1
                } else {
                    0
                }
            }
            FunctionContext::Generic => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function(name: &str, file: &str) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from(file),
            line: 10,
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
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }
    }

    #[test]
    fn detects_formatter_by_name() {
        let detector = ContextDetector::new();
        let function = create_test_function("format_output", "src/output.rs");
        let context = detector.detect_context(&function, Path::new("src/output.rs"));

        assert_eq!(context.context, FunctionContext::Formatter);
        assert!(context.confidence > 0.6);
    }

    #[test]
    fn detects_parser_by_name() {
        let detector = ContextDetector::new();
        let function = create_test_function("parse_input", "src/parser.rs");
        let context = detector.detect_context(&function, Path::new("src/parser.rs"));

        assert_eq!(context.context, FunctionContext::Parser);
        assert!(context.confidence > 0.6);
    }

    #[test]
    fn detects_cli_handler_by_name() {
        let detector = ContextDetector::new();
        let function = create_test_function("handle_command", "src/cli.rs");
        let context = detector.detect_context(&function, Path::new("src/cli.rs"));

        assert_eq!(context.context, FunctionContext::CliHandler);
        assert!(context.confidence > 0.6);
    }

    #[test]
    fn detects_formatter_by_file_location() {
        let detector = ContextDetector::new();
        let function = create_test_function("process_data", "src/io/formatter.rs");
        let context = detector.detect_context(&function, Path::new("src/io/formatter.rs"));

        assert_eq!(context.context, FunctionContext::Formatter);
        assert!(context.confidence > 0.5);
    }

    #[test]
    fn detects_parser_by_file_location() {
        let detector = ContextDetector::new();
        let function = create_test_function("process_data", "src/parser/input.rs");
        let context = detector.detect_context(&function, Path::new("src/parser/input.rs"));

        assert_eq!(context.context, FunctionContext::Parser);
    }

    #[test]
    fn detects_validator() {
        let detector = ContextDetector::new();
        let function = create_test_function("validate_config", "src/config.rs");
        let context = detector.detect_context(&function, Path::new("src/config.rs"));

        assert_eq!(context.context, FunctionContext::Validator);
    }

    #[test]
    fn detects_state_machine() {
        let detector = ContextDetector::new();
        let function = create_test_function("transition_state", "src/state.rs");
        let context = detector.detect_context(&function, Path::new("src/state.rs"));

        assert_eq!(context.context, FunctionContext::StateMachine);
    }

    #[test]
    fn detects_test_helper() {
        let detector = ContextDetector::new();
        let mut function = create_test_function("setup_test", "tests/helper.rs");
        function.in_test_module = true;
        let context = detector.detect_context(&function, Path::new("tests/helper.rs"));

        assert_eq!(context.context, FunctionContext::TestHelper);
        assert!(context.confidence > 0.7);
    }

    #[test]
    fn defaults_to_generic() {
        let detector = ContextDetector::new();
        let function = create_test_function("process_data", "src/core/logic.rs");
        let context = detector.detect_context(&function, Path::new("src/core/logic.rs"));

        assert_eq!(context.context, FunctionContext::Generic);
        assert!(context.confidence < 0.2);
    }

    #[test]
    fn high_confidence_with_multiple_signals() {
        let detector = ContextDetector::new();
        let function = create_test_function("format_pattern_type", "src/io/pattern_output.rs");
        let context = detector.detect_context(&function, Path::new("src/io/pattern_output.rs"));

        assert_eq!(context.context, FunctionContext::Formatter);
        assert!(context.confidence >= 0.8);
        assert!(!context.detected_signals.is_empty());
    }
}
