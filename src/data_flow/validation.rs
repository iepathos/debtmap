use super::graph::{DataFlowNode, NodeId};
use super::taint::{TaintAnalysis, TaintPath};
use crate::security::types::{InputSource, Severity, SinkOperation};
use std::path::PathBuf;

/// Represents a gap in input validation
#[derive(Debug, Clone)]
pub struct ValidationGap {
    pub source: InputSource,
    pub sink: Option<SinkOperation>,
    pub path: Vec<NodeId>,
    pub location: PathBuf,
    pub line: usize,
    pub severity: Severity,
    pub explanation: String,
}

/// Detects validation and sanitization operations
pub struct ValidationDetector {
    validation_methods: Vec<String>,
    sanitization_methods: Vec<String>,
    parsing_methods: Vec<String>,
}

impl ValidationDetector {
    pub fn new() -> Self {
        Self {
            validation_methods: vec![
                "validate".to_string(),
                "is_valid".to_string(),
                "check".to_string(),
                "verify".to_string(),
                "ensure".to_string(),
                "assert".to_string(),
            ],
            sanitization_methods: vec![
                "sanitize".to_string(),
                "escape".to_string(),
                "clean".to_string(),
                "filter".to_string(),
                "strip".to_string(),
                "encode".to_string(),
                "normalize".to_string(),
            ],
            parsing_methods: vec![
                "parse".to_string(),
                "from_str".to_string(),
                "try_from".to_string(),
                "try_into".to_string(),
            ],
        }
    }

    /// Check if a node represents validation
    pub fn is_validation_node(&self, node: &DataFlowNode) -> bool {
        match node {
            DataFlowNode::Validator { .. } => true,
            DataFlowNode::Expression { kind, .. } => {
                if let super::graph::ExpressionKind::MethodCall { method, .. } = kind {
                    self.is_validation_method(method)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if a method name indicates validation
    pub fn is_validation_method(&self, method: &str) -> bool {
        let method_lower = method.to_lowercase();

        // Check validation methods
        for pattern in &self.validation_methods {
            if method_lower.contains(pattern) {
                return true;
            }
        }

        // Check sanitization methods
        for pattern in &self.sanitization_methods {
            if method_lower.contains(pattern) {
                return true;
            }
        }

        // Check parsing methods (with error handling)
        for pattern in &self.parsing_methods {
            if method_lower.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Find validation gaps in the analysis
    pub fn find_gaps(&self, analysis: &TaintAnalysis) -> Vec<ValidationGap> {
        let mut gaps = Vec::new();

        for path in &analysis.taint_paths {
            if !path.has_validation {
                gaps.push(self.create_gap(path));
            }
        }

        gaps
    }

    /// Create a validation gap from a taint path
    fn create_gap(&self, path: &TaintPath) -> ValidationGap {
        let severity = self.assess_severity(&path.source, &path.sink);
        let explanation = self.generate_explanation(path);

        ValidationGap {
            source: path.source,
            sink: Some(path.sink),
            path: path.path.clone(),
            location: PathBuf::from("unknown"), // Would need location info from nodes
            line: 0,                            // Would need line info from nodes
            severity,
            explanation,
        }
    }

    /// Assess the severity of a validation gap
    fn assess_severity(&self, source: &InputSource, sink: &SinkOperation) -> Severity {
        match (source, sink) {
            // Critical: External input to dangerous operations
            (InputSource::HttpRequest | InputSource::UserInput, SinkOperation::SqlQuery) => {
                Severity::Critical
            }
            (
                InputSource::HttpRequest | InputSource::UserInput,
                SinkOperation::ProcessExecution,
            ) => Severity::Critical,

            // High: External input to file system or deserialization
            (InputSource::HttpRequest | InputSource::UserInput, SinkOperation::FileSystem) => {
                Severity::High
            }
            (InputSource::HttpRequest | InputSource::UserInput, SinkOperation::Deserialization) => {
                Severity::High
            }

            // High: Any input to SQL or process execution
            (_, SinkOperation::SqlQuery | SinkOperation::ProcessExecution) => Severity::High,

            // Medium: File or environment input to sensitive operations
            (InputSource::FileInput | InputSource::Environment, SinkOperation::FileSystem) => {
                Severity::Medium
            }
            (InputSource::FileInput | InputSource::Environment, SinkOperation::Deserialization) => {
                Severity::Medium
            }

            // Medium: CLI arguments to any sink
            (InputSource::CliArgument, _) => Severity::Medium,

            // Low: Other combinations
            _ => Severity::Low,
        }
    }

    /// Generate an explanation for the validation gap
    fn generate_explanation(&self, path: &TaintPath) -> String {
        let source_desc = match path.source {
            InputSource::HttpRequest => "HTTP request data",
            InputSource::CliArgument => "command-line arguments",
            InputSource::Environment => "environment variables",
            InputSource::UserInput => "user input",
            InputSource::FileInput => "file content",
            InputSource::ExternalApi => "external API data",
        };

        let sink_desc = match path.sink {
            SinkOperation::SqlQuery => "SQL query execution",
            SinkOperation::ProcessExecution => "process execution",
            SinkOperation::FileSystem => "file system operations",
            SinkOperation::NetworkRequest => "network requests",
            SinkOperation::Deserialization => "deserialization",
            SinkOperation::CryptoOperation => "cryptographic operations",
        };

        format!(
            "Data from {} flows to {} without proper validation. \
             This could lead to security vulnerabilities. \
             Path length: {} nodes",
            source_desc,
            sink_desc,
            path.path.len()
        )
    }
}

impl Default for ValidationDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_validation_method() {
        let detector = ValidationDetector::new();

        // Validation methods
        assert!(detector.is_validation_method("validate_input"));
        assert!(detector.is_validation_method("is_valid"));
        assert!(detector.is_validation_method("check_data"));
        assert!(detector.is_validation_method("verify_signature"));

        // Sanitization methods
        assert!(detector.is_validation_method("sanitize_html"));
        assert!(detector.is_validation_method("escape_sql"));
        assert!(detector.is_validation_method("clean_input"));

        // Parsing methods
        assert!(detector.is_validation_method("parse"));
        assert!(detector.is_validation_method("from_str"));
        assert!(detector.is_validation_method("try_from"));

        // Not validation
        assert!(!detector.is_validation_method("read_file"));
        assert!(!detector.is_validation_method("execute_query"));
        assert!(!detector.is_validation_method("send_request"));
    }

    #[test]
    fn test_assess_severity() {
        let detector = ValidationDetector::new();

        // Critical severity
        assert_eq!(
            detector.assess_severity(&InputSource::HttpRequest, &SinkOperation::SqlQuery),
            Severity::Critical
        );
        assert_eq!(
            detector.assess_severity(&InputSource::UserInput, &SinkOperation::ProcessExecution),
            Severity::Critical
        );

        // High severity
        assert_eq!(
            detector.assess_severity(&InputSource::HttpRequest, &SinkOperation::FileSystem),
            Severity::High
        );
        assert_eq!(
            detector.assess_severity(&InputSource::FileInput, &SinkOperation::SqlQuery),
            Severity::High
        );

        // Medium severity
        assert_eq!(
            detector.assess_severity(&InputSource::CliArgument, &SinkOperation::FileSystem),
            Severity::Medium
        );
        assert_eq!(
            detector.assess_severity(&InputSource::Environment, &SinkOperation::Deserialization),
            Severity::Medium
        );
    }
}
