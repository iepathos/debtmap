use super::graph::DataFlowNode;
use crate::security::types::SinkOperation;

/// Detects dangerous sink operations
pub struct SinkDetector {
    sql_patterns: Vec<String>,
    process_patterns: Vec<String>,
    file_patterns: Vec<String>,
    network_patterns: Vec<String>,
    deserialization_patterns: Vec<String>,
}

impl SinkDetector {
    pub fn new() -> Self {
        Self {
            sql_patterns: vec![
                "execute".to_string(),
                "query".to_string(),
                "sql".to_string(),
                "raw_sql".to_string(),
                "diesel::sql_query".to_string(),
                "sqlx::query".to_string(),
            ],
            process_patterns: vec![
                "Command::new".to_string(),
                "Command::spawn".to_string(),
                "system".to_string(),
                "exec".to_string(),
                "spawn".to_string(),
                "shell".to_string(),
            ],
            file_patterns: vec![
                "File::create".to_string(),
                "File::write".to_string(),
                "fs::write".to_string(),
                "OpenOptions".to_string(),
                "write_all".to_string(),
            ],
            network_patterns: vec![
                "send".to_string(),
                "request".to_string(),
                "post".to_string(),
                "put".to_string(),
                "http".to_string(),
            ],
            deserialization_patterns: vec![
                "deserialize".to_string(),
                "from_str".to_string(),
                "from_slice".to_string(),
                "from_reader".to_string(),
                "parse".to_string(),
                "serde_json::from_str".to_string(),
            ],
        }
    }

    /// Detect the type of sink operation
    pub fn detect_sink_type(&self, method_name: &str, context: &str) -> Option<SinkOperation> {
        let normalized = format!("{} {}", method_name, context).to_lowercase();

        // SQL operations
        for pattern in &self.sql_patterns {
            if normalized.contains(&pattern.to_lowercase()) {
                return Some(SinkOperation::SqlQuery);
            }
        }

        // Process execution
        for pattern in &self.process_patterns {
            if normalized.contains(&pattern.to_lowercase()) {
                return Some(SinkOperation::ProcessExecution);
            }
        }

        // File system operations
        for pattern in &self.file_patterns {
            if normalized.contains(&pattern.to_lowercase()) {
                return Some(SinkOperation::FileSystem);
            }
        }

        // Deserialization (check before network)
        for pattern in &self.deserialization_patterns {
            if normalized.contains(&pattern.to_lowercase()) {
                // But not if it's just parsing in a safe context
                if !normalized.contains("validate") && !normalized.contains("sanitize") {
                    return Some(SinkOperation::Deserialization);
                }
            }
        }

        // Network operations
        for pattern in &self.network_patterns {
            if normalized.contains(&pattern.to_lowercase()) {
                return Some(SinkOperation::NetworkRequest);
            }
        }

        None
    }

    /// Check if a node is a dangerous sink
    pub fn is_dangerous_sink(&self, node: &DataFlowNode) -> bool {
        matches!(node, DataFlowNode::Sink { .. })
    }

    /// Assess the severity of a sink operation
    pub fn assess_severity(&self, sink: &SinkOperation) -> SinkSeverity {
        match sink {
            SinkOperation::SqlQuery | SinkOperation::ProcessExecution => SinkSeverity::Critical,
            SinkOperation::FileSystem | SinkOperation::Deserialization => SinkSeverity::High,
            SinkOperation::NetworkRequest => SinkSeverity::Medium,
            SinkOperation::CryptoOperation => SinkSeverity::Medium,
        }
    }
}

/// Severity levels for sink operations
#[derive(Debug, Clone, PartialEq)]
pub enum SinkSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl Default for SinkDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_sink_type() {
        let detector = SinkDetector::new();

        // SQL sinks
        assert_eq!(
            detector.detect_sink_type("execute", "db.execute(query)"),
            Some(SinkOperation::SqlQuery)
        );
        assert_eq!(
            detector.detect_sink_type("query", "connection.query(sql)"),
            Some(SinkOperation::SqlQuery)
        );

        // Process sinks
        assert_eq!(
            detector.detect_sink_type("spawn", "Command::spawn()"),
            Some(SinkOperation::ProcessExecution)
        );

        // File sinks
        assert_eq!(
            detector.detect_sink_type("write", "File::write(data)"),
            Some(SinkOperation::FileSystem)
        );

        // Network sinks
        assert_eq!(
            detector.detect_sink_type("send", "client.send(request)"),
            Some(SinkOperation::NetworkRequest)
        );

        // Deserialization sinks
        assert_eq!(
            detector.detect_sink_type("from_str", "serde_json::from_str(input)"),
            Some(SinkOperation::Deserialization)
        );

        // Not a sink
        assert_eq!(
            detector.detect_sink_type("calculate", "calculate_sum(a, b)"),
            None
        );
    }

    #[test]
    fn test_assess_severity() {
        let detector = SinkDetector::new();

        assert_eq!(
            detector.assess_severity(&SinkOperation::SqlQuery),
            SinkSeverity::Critical
        );
        assert_eq!(
            detector.assess_severity(&SinkOperation::ProcessExecution),
            SinkSeverity::Critical
        );
        assert_eq!(
            detector.assess_severity(&SinkOperation::FileSystem),
            SinkSeverity::High
        );
        assert_eq!(
            detector.assess_severity(&SinkOperation::NetworkRequest),
            SinkSeverity::Medium
        );
    }
}
