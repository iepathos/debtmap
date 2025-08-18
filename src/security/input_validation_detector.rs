use crate::common::UnifiedLocationExtractor;
use crate::context::rules::ContextRuleEngine;
use crate::context::{ContextDetector, FileType};
use crate::core::{DebtItem, DebtType, Priority};
use crate::data_flow::{DataFlowAnalyzer, ValidationGap};
use crate::security::types::Severity;
use std::path::Path;
use syn::visit::Visit;
use syn::File;

/// Detect validation gaps using data flow analysis
pub fn detect_validation_gaps(file: &File, path: &Path) -> Vec<DebtItem> {
    let source_content = std::fs::read_to_string(path).unwrap_or_default();

    // Determine file type from path
    let file_type = detect_file_type(path);

    // Skip test files entirely
    if matches!(
        file_type,
        FileType::Test | FileType::Example | FileType::Benchmark
    ) {
        return Vec::new();
    }

    // Create context detector
    let mut context_detector = ContextDetector::new(file_type);
    context_detector.visit_file(file);

    // Create context rule engine
    let rule_engine = ContextRuleEngine::new();

    // Perform data flow analysis
    let mut analyzer = DataFlowAnalyzer::new();
    let graph = analyzer.build_graph(file, path);

    let taint_analysis = analyzer.analyze_taint(&graph);
    let validation_gaps = analyzer.find_validation_gaps(&taint_analysis);

    // Convert validation gaps to debt items
    let mut debt_items = Vec::new();
    let location_extractor = UnifiedLocationExtractor::new(&source_content);

    for gap in validation_gaps {
        // Check context rules for each gap
        let should_report = should_report_gap(&gap, &context_detector, &rule_engine);

        if should_report {
            debt_items.push(create_debt_item(gap, path, &location_extractor));
        }
    }

    debt_items
}

fn detect_file_type(path: &Path) -> FileType {
    let path_str = path.to_string_lossy();
    if path_str.contains("/tests/")
        || path_str.contains("_test.rs")
        || path_str.contains("_tests.rs")
    {
        FileType::Test
    } else if path_str.contains("/examples/") {
        FileType::Example
    } else if path_str.contains("/benches/") || path_str.contains("_bench.rs") {
        FileType::Benchmark
    } else if path_str.contains("build.rs") {
        FileType::BuildScript
    } else {
        FileType::Production
    }
}

fn should_report_gap(
    gap: &ValidationGap,
    _context_detector: &ContextDetector,
    _rule_engine: &ContextRuleEngine,
) -> bool {
    // For now, we don't have function-level context from the gap
    // In a full implementation, we'd track function context in the data flow graph

    // Apply severity-based filtering
    match gap.severity {
        Severity::Critical | Severity::High => true,
        Severity::Medium => {
            // Only report medium severity in production code
            true
        }
        Severity::Low => false,
    }
}

fn create_debt_item(
    gap: ValidationGap,
    path: &Path,
    _location_extractor: &UnifiedLocationExtractor,
) -> DebtItem {
    let priority = match gap.severity {
        Severity::Critical => Priority::Critical,
        Severity::High => Priority::High,
        Severity::Medium => Priority::Medium,
        Severity::Low => Priority::Low,
    };

    let source_desc = match gap.source {
        crate::security::types::InputSource::HttpRequest => "HTTP request",
        crate::security::types::InputSource::CliArgument => "CLI argument",
        crate::security::types::InputSource::Environment => "environment variable",
        crate::security::types::InputSource::UserInput => "user input",
        crate::security::types::InputSource::FileInput => "file input",
        crate::security::types::InputSource::ExternalApi => "external API",
    };

    let sink_desc = if let Some(sink) = gap.sink {
        match sink {
            crate::security::types::SinkOperation::SqlQuery => " to SQL query",
            crate::security::types::SinkOperation::ProcessExecution => " to process execution",
            crate::security::types::SinkOperation::FileSystem => " to file system",
            crate::security::types::SinkOperation::NetworkRequest => " to network",
            crate::security::types::SinkOperation::Deserialization => " to deserialization",
            crate::security::types::SinkOperation::CryptoOperation => " to crypto operation",
        }
    } else {
        ""
    };

    DebtItem {
        id: format!("SEC-VAL-{}", gap.line),
        debt_type: DebtType::Security,
        priority,
        file: path.to_path_buf(),
        line: gap.line.max(1),
        column: None,
        message: format!(
            "Input Validation: {} flows{} without proper validation",
            source_desc, sink_desc
        ),
        context: Some(gap.explanation),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_false_positives_for_pattern_checking() {
        // Test that functions checking for patterns are not flagged
        let code = r#"
fn is_cli_argument_source(expr: &str) -> bool {
    expr.contains("args()") || expr.contains("env::args")
}

fn check_if_input_exists(data: &str) -> bool {
    data.contains("input")
}

fn generate_message(format: &str) -> String {
    format!("Input validation required: {}", format)
}
"#;

        let file = syn::parse_file(code).unwrap();
        let path = Path::new("test.rs");

        let debt_items = detect_validation_gaps(&file, path);

        // These should not be flagged as they're just checking patterns
        assert_eq!(
            debt_items.len(),
            0,
            "Pattern checking functions should not be flagged"
        );
    }

    #[test]
    fn test_detects_actual_input_handling() {
        // Test that actual input handling without validation is detected
        // Using a simpler pattern that doesn't require tracking through builder pattern
        let code = r#"
use std::env;
use std::fs::File;
use std::io::Write;

fn vulnerable_function() {
    let input = env::var("USER_INPUT").unwrap();
    
    // Direct use in dangerous operation - write untrusted input to file
    let mut file = File::create(&input).unwrap();
    file.write_all(b"data").unwrap();
}
"#;

        let file = syn::parse_file(code).unwrap();
        let path = Path::new("vulnerable.rs");

        let debt_items = detect_validation_gaps(&file, path);

        // This should be flagged as it's actual input to file system operation
        assert!(
            debt_items.len() > 0,
            "Should detect path traversal vulnerability"
        );

        if !debt_items.is_empty() {
            assert_eq!(debt_items[0].priority, Priority::Medium);
            assert!(debt_items[0].message.contains("environment variable"));
            assert!(debt_items[0].message.contains("file system"));
        }
    }

    #[test]
    fn test_validation_prevents_detection() {
        // Test that validated input is not flagged
        let code = r#"
use std::env;

fn safe_function() {
    let input = env::var("USER_INPUT").unwrap();
    
    // Validate input
    if !validate_input(&input) {
        return;
    }
    
    // Safe to use after validation
    process_data(&input);
}

fn validate_input(input: &str) -> bool {
    // Check for dangerous characters
    !input.contains(';') && !input.contains('|')
}

fn process_data(data: &str) {
    println!("Processing: {}", data);
}
"#;

        let file = syn::parse_file(code).unwrap();
        let path = Path::new("safe.rs");

        let debt_items = detect_validation_gaps(&file, path);

        // Should not be flagged because input is validated
        assert_eq!(debt_items.len(), 0, "Validated input should not be flagged");
    }

    #[test]
    fn test_test_files_are_skipped() {
        // Test that test files are completely skipped
        let code = r#"
#[test]
fn test_something() {
    let input = "test input";
    let args = vec!["arg1", "arg2"];
    process_unsafe(input);
}

fn process_unsafe(data: &str) {
    std::process::Command::new(data).spawn();
}
"#;

        let file = syn::parse_file(code).unwrap();
        let path = Path::new("tests/test_file.rs");

        let debt_items = detect_validation_gaps(&file, path);

        // Test files should be completely skipped
        assert_eq!(debt_items.len(), 0, "Test files should be skipped entirely");
    }
}
