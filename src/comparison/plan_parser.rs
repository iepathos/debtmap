use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct PlanParser;

impl PlanParser {
    /// Extract target location from implementation plan markdown
    pub fn extract_target_location(plan_path: &Path) -> Result<String> {
        let content =
            fs::read_to_string(plan_path).context("Failed to read implementation plan")?;

        // Look for **Location**: pattern
        for line in content.lines() {
            if let Some(location) = Self::parse_location_line(line) {
                return Ok(location);
            }
        }

        Err(anyhow::anyhow!(
            "Could not find **Location**: in plan file. Expected format: **Location**: ./file.rs:function:line"
        ))
    }

    fn parse_location_line(line: &str) -> Option<String> {
        // Match: **Location**: ./src/file.rs:function:123
        // or:    **Location**: ./src/file.rs:123
        if line.trim().starts_with("**Location**:") {
            let location = line.split("**Location**:").nth(1)?.trim();

            // Validate format and normalize path
            if Self::is_valid_location(location) {
                return Some(Self::normalize_location(location));
            }
        }

        None
    }

    fn is_valid_location(location: &str) -> bool {
        let parts: Vec<&str> = location.split(':').collect();

        // Must have file:function:line format
        if parts.len() != 3 {
            return false;
        }

        // File must start with ./ or /
        if !parts[0].starts_with("./") && !parts[0].starts_with('/') {
            return false;
        }

        // Line must be a number
        parts[2].parse::<usize>().is_ok()
    }

    fn normalize_location(location: &str) -> String {
        // Strip leading ./ for consistency
        location.strip_prefix("./").unwrap_or(location).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_location_line() {
        let line = "**Location**: ./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120";
        let result = PlanParser::parse_location_line(line);
        assert_eq!(
            result,
            Some(
                "src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_parse_location_line_with_whitespace() {
        let line = "  **Location**:  ./src/main.rs:func:42  ";
        let result = PlanParser::parse_location_line(line);
        assert_eq!(result, Some("src/main.rs:func:42".to_string()));
    }

    #[test]
    fn test_parse_invalid_location() {
        let line = "**Location**: invalid";
        let result = PlanParser::parse_location_line(line);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_location_missing_parts() {
        let line = "**Location**: ./src/file.rs:123";
        let result = PlanParser::parse_location_line(line);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_target_location_from_file() {
        let content = r#"
# Implementation Plan

## Problem Summary

**Location**: ./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120
**Priority Score**: 81.9

## Implementation Steps
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();

        let location = PlanParser::extract_target_location(temp_file.path()).unwrap();

        assert_eq!(
            location,
            "src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120"
        );
    }

    #[test]
    fn test_extract_target_location_not_found() {
        let content = r#"
# Implementation Plan

Some content without location marker.
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();

        let result = PlanParser::extract_target_location(temp_file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Could not find **Location**:"));
    }

    #[test]
    fn test_normalize_location() {
        assert_eq!(
            PlanParser::normalize_location("./src/main.rs:func:42"),
            "src/main.rs:func:42"
        );
        assert_eq!(
            PlanParser::normalize_location("src/main.rs:func:42"),
            "src/main.rs:func:42"
        );
        assert_eq!(
            PlanParser::normalize_location("/abs/path/main.rs:func:42"),
            "/abs/path/main.rs:func:42"
        );
    }

    #[test]
    fn test_parse_file_level_debt_location() {
        // Test parsing file-level debt with :file:0 format
        let line = "**Location**: ./src/priority/scoring/debt_item.rs:file:0";
        let result = PlanParser::parse_location_line(line);
        assert_eq!(
            result,
            Some("src/priority/scoring/debt_item.rs:file:0".to_string())
        );
    }

    #[test]
    fn test_parse_file_level_debt_with_comment() {
        // Test parsing file-level debt with trailing comment (should be rejected)
        let line = "**Location**: ./src/priority/scoring/debt_item.rs:file:1 (File-level debt)";
        let result = PlanParser::parse_location_line(line);
        // This should be None because of the trailing text
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_file_level_location_from_plan() {
        let content = r#"
# Implementation Plan: Refactor God Object

## Problem Summary

**Location**: ./src/priority/scoring/debt_item.rs:file:0
**Priority Score**: 85.78
**Debt Type**: God Object
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();

        let location = PlanParser::extract_target_location(temp_file.path()).unwrap();

        assert_eq!(location, "src/priority/scoring/debt_item.rs:file:0");
    }
}
