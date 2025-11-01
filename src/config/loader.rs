use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use super::core::DebtmapConfig;
use super::scoring::ScoringWeights;

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
pub fn directory_ancestors(
    start: PathBuf,
    max_depth: usize,
) -> impl Iterator<Item = PathBuf> {
    directory_ancestors_impl(start, max_depth)
}

pub(crate) fn directory_ancestors_impl(start: PathBuf, max_depth: usize) -> impl Iterator<Item = PathBuf> {
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
