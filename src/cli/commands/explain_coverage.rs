//! Explain-coverage command handler
//!
//! This module contains the handler for the `explain-coverage` subcommand,
//! which explains how coverage detection works for a specific function.

use crate::cli::args::Commands;
use crate::commands::explain_coverage::ExplainCoverageConfig;
use anyhow::Result;

/// Extracts parameters from the ExplainCoverage command variant.
///
/// This function handles the destructuring of CLI parameters from the Commands enum
/// and builds the ExplainCoverageConfig. It's separated to keep the main handler
/// focused on coordination.
pub fn extract_explain_coverage_params(command: Commands) -> Result<ExplainCoverageConfig> {
    if let Commands::ExplainCoverage {
        path,
        coverage_file,
        function_name,
        file_path,
        verbose,
        format,
    } = command
    {
        Ok(ExplainCoverageConfig {
            path,
            coverage_file,
            function_name,
            file_path,
            verbose,
            format: format.into(),
        })
    } else {
        Err(anyhow::anyhow!(
            "Invalid command: expected ExplainCoverage variant"
        ))
    }
}

/// Handle the explain-coverage command
///
/// This is the entry point for the explain-coverage command. It coordinates:
/// 1. Extract parameters and build configuration
/// 2. Delegate to coverage explanation logic
///
/// # Architecture
///
/// This function follows the "pure core, imperative shell" pattern and serves as a thin
/// coordination layer. The heavy lifting is delegated to:
/// - `extract_explain_coverage_params`: Parameter extraction and config building
/// - `explain_coverage`: Core explanation logic
pub fn handle_explain_coverage_command(command: Commands) -> Result<()> {
    let config = extract_explain_coverage_params(command)?;
    crate::commands::explain_coverage::explain_coverage(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::DebugFormatArg;
    use crate::commands::explain_coverage::DebugFormat;
    use std::path::PathBuf;

    #[test]
    fn test_extract_explain_coverage_params_wrong_variant() {
        // Test that extract_explain_coverage_params returns an error for non-ExplainCoverage commands
        let init_command = Commands::Init { force: false };

        let result = extract_explain_coverage_params(init_command);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("expected ExplainCoverage variant"));
    }

    #[test]
    fn test_extract_explain_coverage_params_success() {
        let command = Commands::ExplainCoverage {
            path: PathBuf::from("/test/project"),
            coverage_file: PathBuf::from("/test/lcov.info"),
            function_name: "test_function".to_string(),
            file_path: Some(PathBuf::from("/test/src/main.rs")),
            verbose: true,
            format: DebugFormatArg::Json,
        };

        let result = extract_explain_coverage_params(command);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.path, PathBuf::from("/test/project"));
        assert_eq!(config.coverage_file, PathBuf::from("/test/lcov.info"));
        assert_eq!(config.function_name, "test_function");
        assert_eq!(config.file_path, Some(PathBuf::from("/test/src/main.rs")));
        assert!(config.verbose);
        assert!(matches!(config.format, DebugFormat::Json));
    }

    #[test]
    fn test_extract_explain_coverage_params_minimal() {
        let command = Commands::ExplainCoverage {
            path: PathBuf::from("."),
            coverage_file: PathBuf::from("coverage.lcov"),
            function_name: "my_func".to_string(),
            file_path: None,
            verbose: false,
            format: DebugFormatArg::Text,
        };

        let result = extract_explain_coverage_params(command);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.path, PathBuf::from("."));
        assert_eq!(config.function_name, "my_func");
        assert!(config.file_path.is_none());
        assert!(!config.verbose);
        assert!(matches!(config.format, DebugFormat::Text));
    }
}
