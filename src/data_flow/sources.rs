use super::graph::DataFlowNode;
use crate::security::types::InputSource;

/// Types of operations on potential input sources
#[derive(Debug, Clone, PartialEq)]
pub enum OperationType {
    /// Actually reads external input
    Read,
    /// Checks for pattern/existence (e.g., checking if a string contains "input")
    Check,
    /// Transforms data
    Transform,
    /// Validates/sanitizes data
    Validate,
}

/// Detects and classifies input sources
pub struct SourceDetector {
    /// Patterns that indicate actual read operations
    read_patterns: Vec<String>,
    /// Patterns that indicate checking/analysis operations
    check_patterns: Vec<String>,
}

impl SourceDetector {
    pub fn new() -> Self {
        Self {
            read_patterns: vec![
                "File::open".to_string(),
                "fs::read".to_string(),
                "read_to_string".to_string(),
                "read_line".to_string(),
                "stdin".to_string(),
                "env::args".to_string(),
                "env::var".to_string(),
                "Request::body".to_string(),
                "HttpRequest::body".to_string(),
                "TcpStream::read".to_string(),
            ],
            check_patterns: vec![
                "contains".to_string(),
                "starts_with".to_string(),
                "ends_with".to_string(),
                "matches".to_string(),
                "is_".to_string(),
                "has_".to_string(),
                "check_".to_string(),
                "detect_".to_string(),
                "find_".to_string(),
                "search_".to_string(),
                "analyze_".to_string(),
                "classify_".to_string(),
            ],
        }
    }

    /// Classify an operation based on context
    pub fn classify_operation(&self, method_name: &str, context: &str) -> OperationType {
        // Check if this is a checking/analysis operation
        for pattern in &self.check_patterns {
            if method_name.contains(pattern) || context.contains(&format!("is_{}", pattern)) {
                return OperationType::Check;
            }
        }

        // Check if this is a read operation
        for pattern in &self.read_patterns {
            if context.contains(pattern) {
                return OperationType::Read;
            }
        }

        // Check for validation patterns
        if method_name.contains("validate")
            || method_name.contains("sanitize")
            || method_name.contains("escape")
            || method_name.contains("parse")
        {
            return OperationType::Validate;
        }

        // Default to transform
        OperationType::Transform
    }

    /// Check if a node is an actual input source (not just checking for patterns)
    pub fn is_actual_source(&self, node: &DataFlowNode, context: &str) -> bool {
        if let DataFlowNode::Source { .. } = node {
            // This was already identified as a source by the builder
            // Now verify it's an actual read operation
            let operation = self.classify_operation("", context);
            operation == OperationType::Read
        } else {
            false
        }
    }

    /// Detect the type of input source from context
    pub fn detect_source_type(&self, context: &str) -> Option<InputSource> {
        // Only return Some if this is an actual read operation
        let normalized = context.replace(" ", "").to_lowercase();

        // File operations that actually read
        if (normalized.contains("file::open")
            || normalized.contains("fs::read")
            || normalized.contains("read_to_string"))
            && !normalized.contains("check")
            && !normalized.contains("is_")
        {
            return Some(InputSource::FileInput);
        }

        // CLI arguments
        if normalized.contains("env::args") && !normalized.contains("is_") {
            return Some(InputSource::CliArgument);
        }

        // Environment variables
        if normalized.contains("env::var") && !normalized.contains("is_") {
            return Some(InputSource::Environment);
        }

        // User input from stdin
        if (normalized.contains("stdin") || normalized.contains("read_line"))
            && !normalized.contains("is_")
        {
            return Some(InputSource::UserInput);
        }

        // HTTP requests
        if (normalized.contains("request") && normalized.contains("body"))
            || (normalized.contains("httprequest") && !normalized.contains("is_"))
        {
            return Some(InputSource::HttpRequest);
        }

        None
    }

    /// Check if a function name indicates it's just checking for patterns
    pub fn is_pattern_checker(&self, function_name: &str) -> bool {
        let name_lower = function_name.to_lowercase();

        // Functions that check for patterns are not input sources
        name_lower.starts_with("is_")
            || name_lower.starts_with("has_")
            || name_lower.starts_with("check_")
            || name_lower.starts_with("detect_")
            || name_lower.starts_with("find_")
            || name_lower.starts_with("analyze_")
            || name_lower.starts_with("classify_")
            || name_lower.contains("_is_")
            || name_lower.contains("_check_")
            || name_lower.contains("_detect_")
    }

    /// Check if a variable name suggests it's for analysis/checking
    pub fn is_analysis_variable(&self, var_name: &str) -> bool {
        let name_lower = var_name.to_lowercase();

        // Variables used for pattern checking/analysis
        name_lower.contains("pattern")
            || name_lower.contains("check")
            || name_lower.contains("test")
            || name_lower.contains("expected")
            || name_lower.contains("format")
            || name_lower.contains("template")
            || name_lower.contains("message")
            || name_lower.contains("label")
            || name_lower == "needle"
            || name_lower == "haystack"
    }
}

impl Default for SourceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_operation() {
        let detector = SourceDetector::new();

        // Check operations
        assert_eq!(
            detector.classify_operation("contains", "str.contains('input')"),
            OperationType::Check
        );
        assert_eq!(
            detector.classify_operation("is_cli_argument_source", ""),
            OperationType::Check
        );

        // Read operations
        assert_eq!(
            detector.classify_operation("read", "File::open(path).read()"),
            OperationType::Read
        );

        // Validate operations
        assert_eq!(
            detector.classify_operation("validate_input", ""),
            OperationType::Validate
        );
    }

    #[test]
    fn test_is_pattern_checker() {
        let detector = SourceDetector::new();

        assert!(detector.is_pattern_checker("is_cli_argument_source"));
        assert!(detector.is_pattern_checker("has_input"));
        assert!(detector.is_pattern_checker("check_validation"));
        assert!(detector.is_pattern_checker("detect_pattern"));

        assert!(!detector.is_pattern_checker("read_file"));
        assert!(!detector.is_pattern_checker("get_input"));
        assert!(!detector.is_pattern_checker("process_data"));
    }

    #[test]
    fn test_is_analysis_variable() {
        let detector = SourceDetector::new();

        assert!(detector.is_analysis_variable("input_pattern"));
        assert!(detector.is_analysis_variable("check_value"));
        assert!(detector.is_analysis_variable("test_data"));
        assert!(detector.is_analysis_variable("expected_format"));
        assert!(detector.is_analysis_variable("message_template"));

        assert!(!detector.is_analysis_variable("user_input"));
        assert!(!detector.is_analysis_variable("data"));
        assert!(!detector.is_analysis_variable("value"));
    }

    #[test]
    fn test_detect_source_type() {
        let detector = SourceDetector::new();

        // Actual read operations
        assert_eq!(
            detector.detect_source_type("File::open(path)"),
            Some(InputSource::FileInput)
        );
        assert_eq!(
            detector.detect_source_type("env::args()"),
            Some(InputSource::CliArgument)
        );

        // Pattern checking - should return None
        assert_eq!(detector.detect_source_type("is_file_input()"), None);
        assert_eq!(detector.detect_source_type("check_if_input_exists()"), None);
    }
}
