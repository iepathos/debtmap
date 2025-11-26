use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use super::core::DebtmapConfig;
use super::scoring::ScoringWeights;
use super::validation::validate_config;
use crate::effects::{run_validation, validation_failure, validation_success, AnalysisValidation};
use crate::errors::AnalysisError;

/// Load configuration from .debtmap.toml if it exists
/// Pure function to read and parse config file contents
pub(crate) fn read_config_file(path: &Path) -> Result<String, std::io::Error> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Pure function to parse and validate config from TOML string
pub fn parse_and_validate_config(contents: &str) -> Result<DebtmapConfig, String> {
    parse_and_validate_config_impl(contents)
}

pub(crate) fn parse_and_validate_config_impl(contents: &str) -> Result<DebtmapConfig, String> {
    let mut config = toml::from_str::<DebtmapConfig>(contents)
        .map_err(|e| format!("Failed to parse .debtmap.toml: {}", e))?;

    // Validate and normalize scoring weights if present
    if let Some(ref mut scoring) = config.scoring {
        if let Err(e) = scoring.validate() {
            eprintln!("Warning: Invalid scoring weights: {}. Using defaults.", e);
            config.scoring = Some(ScoringWeights::default());
        } else {
            scoring.normalize(); // Ensure exact sum of 1.0
        }
    }

    Ok(config)
}

/// Pure function to try loading config from a specific path
pub(crate) fn try_load_config_from_path(config_path: &Path) -> Option<DebtmapConfig> {
    let contents = match read_config_file(config_path) {
        Ok(contents) => contents,
        Err(e) => {
            handle_read_error(config_path, &e);
            return None;
        }
    };

    match parse_and_validate_config_impl(&contents) {
        Ok(config) => {
            log::debug!("Loaded config from {}", config_path.display());
            Some(config)
        }
        Err(e) => {
            eprintln!("Warning: {}. Using defaults.", e);
            None
        }
    }
}

/// Handle file read errors with appropriate logging
pub(crate) fn handle_read_error(config_path: &Path, error: &std::io::Error) {
    // Only log actual errors, not "file not found"
    if error.kind() != std::io::ErrorKind::NotFound {
        log::warn!(
            "Failed to read config file {}: {}",
            config_path.display(),
            error
        );
    }
}

/// Pure function to generate directory ancestors up to a depth limit
pub fn directory_ancestors(start: PathBuf, max_depth: usize) -> impl Iterator<Item = PathBuf> {
    directory_ancestors_impl(start, max_depth)
}

pub(crate) fn directory_ancestors_impl(
    start: PathBuf,
    max_depth: usize,
) -> impl Iterator<Item = PathBuf> {
    std::iter::successors(Some(start), |dir| {
        let mut parent = dir.clone();
        if parent.pop() {
            Some(parent)
        } else {
            None
        }
    })
    .take(max_depth)
}

pub fn load_config() -> DebtmapConfig {
    const MAX_TRAVERSAL_DEPTH: usize = 10;

    // Get current directory or return default
    let current = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!(
                "Failed to get current directory: {}. Using default config.",
                e
            );
            return DebtmapConfig::default();
        }
    };

    // Search for config file in directory hierarchy
    directory_ancestors_impl(current, MAX_TRAVERSAL_DEPTH)
        .map(|dir| dir.join(".debtmap.toml"))
        .find_map(|path| try_load_config_from_path(&path))
        .unwrap_or_else(|| {
            log::debug!(
                "No config found after checking {} directories. Using default config.",
                MAX_TRAVERSAL_DEPTH
            );
            DebtmapConfig::default()
        })
}

// ============================================================================
// Validation-based config loading (Spec 197)
// ============================================================================

/// Load and validate config using error accumulation.
///
/// This function reads the config file and validates it, accumulating
/// ALL errors instead of failing at the first one.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::config::loader::load_config_validated;
/// use std::path::Path;
///
/// let validation = load_config_validated(Path::new("project/"));
///
/// // Check all errors at once
/// match validation {
///     stillwater::Validation::Success(config) => {
///         // Use config
///     }
///     stillwater::Validation::Failure(errors) => {
///         // Show ALL errors to user
///         for error in errors {
///             eprintln!("  - {}", error);
///         }
///     }
/// }
/// ```
pub fn load_config_validated(start_dir: &Path) -> AnalysisValidation<DebtmapConfig> {
    const MAX_TRAVERSAL_DEPTH: usize = 10;

    // Find config file
    let config_path = directory_ancestors_impl(start_dir.to_path_buf(), MAX_TRAVERSAL_DEPTH)
        .map(|dir| dir.join(".debtmap.toml"))
        .find(|path| path.exists());

    let Some(config_path) = config_path else {
        // No config file found - return default (this is not an error)
        return validation_success(DebtmapConfig::default());
    };

    // Read config file
    let contents = match read_config_file(&config_path) {
        Ok(contents) => contents,
        Err(e) => {
            return validation_failure(AnalysisError::io_with_path(
                format!("Cannot read config file: {}", e),
                &config_path,
            ));
        }
    };

    // Parse config
    let config = match toml::from_str::<DebtmapConfig>(&contents) {
        Ok(config) => config,
        Err(e) => {
            return validation_failure(AnalysisError::config_with_path(
                format!("Failed to parse config: {}", e),
                &config_path,
            ));
        }
    };

    // Validate config (accumulates ALL validation errors)
    match validate_config(&config) {
        stillwater::Validation::Success(_) => validation_success(config),
        stillwater::Validation::Failure(errors) => stillwater::Validation::Failure(errors),
    }
}

/// Load and validate config with backwards-compatible Result API.
///
/// This wraps `load_config_validated` to return `anyhow::Result` for use
/// with existing code that expects fail-fast error handling.
pub fn load_config_validated_result(start_dir: &Path) -> anyhow::Result<DebtmapConfig> {
    run_validation(load_config_validated(start_dir))
}

/// Load config from a specific path with validation.
///
/// Unlike `load_config_validated`, this requires the config file to exist
/// at the specified path.
pub fn load_config_from_path_validated(config_path: &Path) -> AnalysisValidation<DebtmapConfig> {
    // Check file exists
    if !config_path.exists() {
        return validation_failure(AnalysisError::config_with_path(
            format!("Config file not found: {}", config_path.display()),
            config_path,
        ));
    }

    // Read config file
    let contents = match read_config_file(config_path) {
        Ok(contents) => contents,
        Err(e) => {
            return validation_failure(AnalysisError::io_with_path(
                format!("Cannot read config file: {}", e),
                config_path,
            ));
        }
    };

    // Parse config
    let config = match toml::from_str::<DebtmapConfig>(&contents) {
        Ok(config) => config,
        Err(e) => {
            return validation_failure(AnalysisError::config_with_path(
                format!("Failed to parse config: {}", e),
                config_path,
            ));
        }
    };

    // Validate config (accumulates ALL validation errors)
    match validate_config(&config) {
        stillwater::Validation::Success(_) => validation_success(config),
        stillwater::Validation::Failure(errors) => stillwater::Validation::Failure(errors),
    }
}

/// Load config from a specific path with backwards-compatible Result API.
pub fn load_config_from_path_result(config_path: &Path) -> anyhow::Result<DebtmapConfig> {
    run_validation(load_config_from_path_validated(config_path))
}
