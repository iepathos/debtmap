//! Unified location structure for debt items (spec 108)
//!
//! Provides a consistent location format for both file and function debt items,
//! with optional line number and function name fields.

use serde::{Deserialize, Serialize};

/// Unified location structure for all debt items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedLocation {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    /// "TEST FILE" or "PROBABLE TEST" for test files (spec 166)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_context_label: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_location_serialization() {
        let loc = UnifiedLocation {
            file: "test.rs".to_string(),
            line: Some(42),
            function: Some("test_function".to_string()),
            file_context_label: None,
        };

        let json = serde_json::to_string(&loc).unwrap();
        assert!(json.contains("\"file\":\"test.rs\""));
        assert!(json.contains("\"line\":42"));
        assert!(json.contains("\"function\":\"test_function\""));
    }

    #[test]
    fn test_file_location_omits_optional_fields() {
        let loc = UnifiedLocation {
            file: "test.rs".to_string(),
            line: None,
            function: None,
            file_context_label: None,
        };

        let json = serde_json::to_string(&loc).unwrap();
        assert!(json.contains("\"file\":\"test.rs\""));
        assert!(!json.contains("\"line\""));
        assert!(!json.contains("\"function\""));
    }
}
