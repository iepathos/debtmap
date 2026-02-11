//! Validate-improvement command handler
//!
//! This module contains the handler for the `validate-improvement` subcommand,
//! which validates that technical debt improvements have been made.

use crate::cli::args::Commands;
use crate::cli::is_automation_mode;
use crate::commands::validate_improvement::ValidateImprovementConfig;
use anyhow::Result;

/// Extracts parameters from the ValidateImprovement command variant.
///
/// This function handles the destructuring of CLI parameters from the Commands enum
/// and builds the ValidateImprovementConfig. It's separated to keep the main handler
/// focused on coordination.
pub fn extract_validate_improvement_params(command: Commands) -> Result<ValidateImprovementConfig> {
    if let Commands::ValidateImprovement {
        comparison,
        output,
        previous_validation,
        threshold,
        format,
        quiet,
    } = command
    {
        Ok(ValidateImprovementConfig {
            comparison_path: comparison,
            output_path: output,
            previous_validation,
            threshold,
            format: format.into(),
            quiet: quiet || is_automation_mode(),
        })
    } else {
        Err(anyhow::anyhow!(
            "Invalid command: expected ValidateImprovement variant"
        ))
    }
}

/// Handle the validate-improvement command
///
/// This is the entry point for the validate-improvement command. It coordinates:
/// 1. Extract parameters and build configuration
/// 2. Delegate to validation logic
///
/// # Architecture
///
/// This function follows the "pure core, imperative shell" pattern and serves as a thin
/// coordination layer. The heavy lifting is delegated to:
/// - `extract_validate_improvement_params`: Parameter extraction and config building
/// - `validate_improvement`: Core validation logic
pub fn handle_validate_improvement_command(command: Commands) -> Result<()> {
    let config = extract_validate_improvement_params(command)?;
    crate::commands::validate_improvement::validate_improvement(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::OutputFormat as CliOutputFormat;
    use crate::commands::validate_improvement::OutputFormat;
    use std::path::PathBuf;

    #[test]
    fn test_extract_validate_improvement_params_wrong_variant() {
        // Test that extract_validate_improvement_params returns an error for non-ValidateImprovement commands
        let init_command = Commands::Init { force: false };

        let result = extract_validate_improvement_params(init_command);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("expected ValidateImprovement variant"));
    }

    #[test]
    fn test_extract_validate_improvement_params_success() {
        let command = Commands::ValidateImprovement {
            comparison: PathBuf::from("/test/comparison.json"),
            output: PathBuf::from("/test/output.json"),
            previous_validation: Some(PathBuf::from("/test/previous.json")),
            threshold: 80.0,
            format: CliOutputFormat::Json,
            quiet: true,
        };

        let result = extract_validate_improvement_params(command);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(
            config.comparison_path,
            PathBuf::from("/test/comparison.json")
        );
        assert_eq!(config.output_path, PathBuf::from("/test/output.json"));
        assert_eq!(
            config.previous_validation,
            Some(PathBuf::from("/test/previous.json"))
        );
        assert_eq!(config.threshold, 80.0);
        assert!(matches!(config.format, OutputFormat::Json));
        assert!(config.quiet);
    }

    #[test]
    fn test_extract_validate_improvement_params_minimal() {
        let command = Commands::ValidateImprovement {
            comparison: PathBuf::from("comp.json"),
            output: PathBuf::from("out.json"),
            previous_validation: None,
            threshold: 75.0,
            format: CliOutputFormat::Terminal,
            quiet: false,
        };

        let result = extract_validate_improvement_params(command);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.comparison_path, PathBuf::from("comp.json"));
        assert!(config.previous_validation.is_none());
        assert_eq!(config.threshold, 75.0);
        assert!(matches!(config.format, OutputFormat::Terminal));
    }

    #[test]
    fn test_extract_validate_improvement_params_markdown_format() {
        let command = Commands::ValidateImprovement {
            comparison: PathBuf::from("comp.json"),
            output: PathBuf::from("out.md"),
            previous_validation: None,
            threshold: 90.0,
            format: CliOutputFormat::Markdown,
            quiet: false,
        };

        let result = extract_validate_improvement_params(command);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(matches!(config.format, OutputFormat::Markdown));
    }
}
