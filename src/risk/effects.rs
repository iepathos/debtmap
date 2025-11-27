//! Effect-wrapped coverage operations for risk analysis.
//!
//! This module provides Effect-based wrappers around coverage loading operations,
//! enabling pure functional composition with fallback strategies and error context.
//!
//! # Coverage Loading Strategies
//!
//! Coverage data can come from multiple sources:
//! - LCOV format (from cargo-llvm-cov, pytest-cov, etc.)
//! - Cobertura XML format (from cargo-tarpaulin, etc.)
//!
//! The `load_coverage_effect` function tries multiple strategies and uses the
//! first one that succeeds, making it robust to different project configurations.
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::risk::effects::load_coverage_effect;
//! use debtmap::effects::run_effect;
//!
//! // Load coverage with automatic fallback
//! let coverage = run_effect(
//!     load_coverage_effect("coverage.lcov".into(), ".".into()),
//!     config,
//! )?;
//!
//! // Use coverage data
//! let file_coverage = coverage.get_file_coverage(Path::new("src/main.rs"));
//! ```

use crate::effects::AnalysisEffect;
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use crate::io::traits::CoverageData;
use std::path::PathBuf;
use stillwater::Effect;

// ============================================================================
// LCOV Loading
// ============================================================================

/// Load coverage data from an LCOV format file as an Effect.
///
/// LCOV is a common coverage format supported by:
/// - `cargo-llvm-cov` (Rust)
/// - `pytest-cov` (Python)
/// - `jest --coverage` (JavaScript)
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::risk::effects::load_lcov_effect;
///
/// let effect = load_lcov_effect("target/coverage/lcov.info".into());
/// let coverage = run_effect(effect, config)?;
/// ```
///
/// # Errors
///
/// Returns `AnalysisError::CoverageError` if:
/// - The file doesn't exist
/// - The file isn't valid LCOV format
pub fn load_lcov_effect(path: PathBuf) -> AnalysisEffect<CoverageData> {
    let path_display = path.display().to_string();
    Effect::from_fn(move |env: &RealEnv| {
        env.coverage_loader().load_lcov(&path).map_err(|e| {
            AnalysisError::coverage_with_path(
                format!(
                    "Failed to load LCOV from '{}': {}",
                    path_display,
                    e.message()
                ),
                &path,
            )
        })
    })
}

/// Load coverage data from a Cobertura XML file as an Effect.
///
/// Cobertura is an XML-based coverage format supported by:
/// - `cargo-tarpaulin` (Rust)
/// - `coverage.py` (Python)
/// - Various CI systems
///
/// # Errors
///
/// Returns `AnalysisError::CoverageError` if:
/// - The file doesn't exist
/// - The file isn't valid Cobertura XML
pub fn load_cobertura_effect(path: PathBuf) -> AnalysisEffect<CoverageData> {
    let path_display = path.display().to_string();
    Effect::from_fn(move |env: &RealEnv| {
        env.coverage_loader().load_cobertura(&path).map_err(|e| {
            AnalysisError::coverage_with_path(
                format!(
                    "Failed to load Cobertura from '{}': {}",
                    path_display,
                    e.message()
                ),
                &path,
            )
        })
    })
}

// ============================================================================
// Coverage with Fallback
// ============================================================================

/// Load coverage data with automatic fallback strategies.
///
/// This function tries multiple coverage sources in order and returns
/// the first one that succeeds. The fallback order is:
///
/// 1. User-specified primary path
/// 2. Default cargo-llvm-cov location: `target/llvm-cov-target/debug/coverage/lcov.info`
/// 3. Alternative lcov location: `target/coverage/lcov.info`
/// 4. Cargo-tarpaulin Cobertura: `target/coverage/cobertura.xml`
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::risk::effects::load_coverage_effect;
///
/// // Try user-specified path first, then fallback to defaults
/// let effect = load_coverage_effect(
///     "my-coverage.lcov".into(),
///     "/path/to/project".into(),
/// );
/// let coverage = run_effect(effect, config)?;
/// ```
///
/// # Errors
///
/// Returns `AnalysisError::CoverageError` only if ALL fallback strategies fail.
/// The error message includes details about what was tried.
pub fn load_coverage_effect(
    primary_path: PathBuf,
    project_root: PathBuf,
) -> AnalysisEffect<CoverageData> {
    // Build list of paths to try in order
    let fallback_paths = vec![
        primary_path.clone(),
        project_root.join("target/llvm-cov-target/debug/coverage/lcov.info"),
        project_root.join("target/coverage/lcov.info"),
        project_root.join("lcov.info"),
    ];

    let cobertura_path = project_root.join("target/coverage/cobertura.xml");

    // Try each LCOV path in sequence
    try_coverage_paths(fallback_paths, cobertura_path)
}

/// Try loading coverage from a list of paths, returning first success.
fn try_coverage_paths(
    lcov_paths: Vec<PathBuf>,
    cobertura_path: PathBuf,
) -> AnalysisEffect<CoverageData> {
    let paths_display: Vec<String> = lcov_paths.iter().map(|p| p.display().to_string()).collect();
    let cobertura_display = cobertura_path.display().to_string();

    Effect::from_fn(move |env: &RealEnv| {
        let mut last_error = None;

        // Try each LCOV path
        for path in &lcov_paths {
            if env.file_system().exists(path) {
                match env.coverage_loader().load_lcov(path) {
                    Ok(data) => return Ok(data),
                    Err(e) => last_error = Some(e),
                }
            }
        }

        // Try Cobertura as fallback
        if env.file_system().exists(&cobertura_path) {
            match env.coverage_loader().load_cobertura(&cobertura_path) {
                Ok(data) => return Ok(data),
                Err(e) => last_error = Some(e),
            }
        }

        // All strategies failed
        Err(AnalysisError::coverage(format!(
            "No coverage data found. Tried:\n  LCOV: {}\n  Cobertura: {}\n\n\
             Generate coverage with:\n  cargo llvm-cov --lcov --output-path coverage.lcov\n\
             Last error: {}",
            paths_display.join(", "),
            cobertura_display,
            last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "none".to_string())
        )))
    })
}

/// Load coverage data, returning empty coverage if not found.
///
/// This is useful when coverage is optional and you want to proceed
/// without it rather than failing.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::risk::effects::load_coverage_optional_effect;
///
/// // Returns empty CoverageData if no coverage file found
/// let effect = load_coverage_optional_effect("coverage.lcov".into());
/// let coverage = run_effect(effect, config)?;
/// ```
pub fn load_coverage_optional_effect(path: PathBuf) -> AnalysisEffect<CoverageData> {
    let path_clone = path.clone();
    Effect::from_fn(move |env: &RealEnv| {
        if !env.file_system().exists(&path_clone) {
            return Ok(CoverageData::default());
        }

        match env.coverage_loader().load_lcov(&path_clone) {
            Ok(data) => Ok(data),
            Err(_) => {
                // Coverage file exists but couldn't be parsed - return empty
                Ok(CoverageData::default())
            }
        }
    })
}

// ============================================================================
// Coverage Utilities
// ============================================================================

/// Check if any coverage data is available for a project.
///
/// Returns true if any of the common coverage file locations exist.
pub fn has_coverage_effect(project_root: PathBuf) -> AnalysisEffect<bool> {
    Effect::from_fn(move |env: &RealEnv| {
        let paths = [
            project_root.join("target/llvm-cov-target/debug/coverage/lcov.info"),
            project_root.join("target/coverage/lcov.info"),
            project_root.join("target/coverage/cobertura.xml"),
            project_root.join("lcov.info"),
            project_root.join("coverage.lcov"),
        ];

        Ok(paths.iter().any(|p| env.file_system().exists(p)))
    })
}

/// Get the path to the first available coverage file.
///
/// Returns `None` if no coverage file is found.
pub fn find_coverage_path_effect(project_root: PathBuf) -> AnalysisEffect<Option<PathBuf>> {
    Effect::from_fn(move |env: &RealEnv| {
        let paths = [
            project_root.join("target/llvm-cov-target/debug/coverage/lcov.info"),
            project_root.join("target/coverage/lcov.info"),
            project_root.join("target/coverage/cobertura.xml"),
            project_root.join("lcov.info"),
            project_root.join("coverage.lcov"),
        ];

        Ok(paths.into_iter().find(|p| env.file_system().exists(p)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::effects::run_effect;
    use tempfile::TempDir;

    fn create_test_env() -> (TempDir, DebtmapConfig) {
        let temp_dir = TempDir::new().unwrap();
        (temp_dir, DebtmapConfig::default())
    }

    fn create_lcov_content() -> &'static str {
        r#"SF:src/main.rs
DA:1,5
DA:2,5
DA:3,0
end_of_record
SF:src/lib.rs
DA:1,1
DA:2,0
end_of_record
"#
    }

    #[test]
    fn test_load_lcov_effect_success() {
        let (temp_dir, config) = create_test_env();
        let lcov_path = temp_dir.path().join("coverage.lcov");
        std::fs::write(&lcov_path, create_lcov_content()).unwrap();

        let effect = load_lcov_effect(lcov_path);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let coverage = result.unwrap();
        let main_coverage = coverage
            .get_file_coverage(std::path::Path::new("src/main.rs"))
            .unwrap();
        // 2 out of 3 lines hit
        assert!((main_coverage - 66.67).abs() < 1.0);
    }

    #[test]
    fn test_load_lcov_effect_not_found() {
        let config = DebtmapConfig::default();
        let effect = load_lcov_effect("/nonexistent/coverage.lcov".into());
        let result = run_effect(effect, config);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to load LCOV"));
    }

    #[test]
    fn test_load_coverage_optional_effect_missing() {
        let config = DebtmapConfig::default();
        let effect = load_coverage_optional_effect("/nonexistent/coverage.lcov".into());
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let coverage = result.unwrap();
        // Should return empty coverage data
        assert!(coverage.files().next().is_none());
    }

    #[test]
    fn test_load_coverage_optional_effect_exists() {
        let (temp_dir, config) = create_test_env();
        let lcov_path = temp_dir.path().join("coverage.lcov");
        std::fs::write(&lcov_path, create_lcov_content()).unwrap();

        let effect = load_coverage_optional_effect(lcov_path);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let coverage = result.unwrap();
        assert!(coverage
            .get_file_coverage(std::path::Path::new("src/main.rs"))
            .is_some());
    }

    #[test]
    fn test_has_coverage_effect_no_files() {
        let (temp_dir, config) = create_test_env();

        let effect = has_coverage_effect(temp_dir.path().to_path_buf());
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_has_coverage_effect_with_files() {
        let (temp_dir, config) = create_test_env();
        let coverage_dir = temp_dir.path().join("target/coverage");
        std::fs::create_dir_all(&coverage_dir).unwrap();
        std::fs::write(coverage_dir.join("lcov.info"), create_lcov_content()).unwrap();

        let effect = has_coverage_effect(temp_dir.path().to_path_buf());
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_find_coverage_path_effect_found() {
        let (temp_dir, config) = create_test_env();
        let coverage_dir = temp_dir.path().join("target/coverage");
        std::fs::create_dir_all(&coverage_dir).unwrap();
        let lcov_path = coverage_dir.join("lcov.info");
        std::fs::write(&lcov_path, create_lcov_content()).unwrap();

        let effect = find_coverage_path_effect(temp_dir.path().to_path_buf());
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let found_path = result.unwrap();
        assert!(found_path.is_some());
        assert_eq!(found_path.unwrap(), lcov_path);
    }

    #[test]
    fn test_find_coverage_path_effect_not_found() {
        let (temp_dir, config) = create_test_env();

        let effect = find_coverage_path_effect(temp_dir.path().to_path_buf());
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_load_coverage_effect_with_fallback() {
        let (temp_dir, config) = create_test_env();

        // Create coverage in fallback location
        let coverage_dir = temp_dir.path().join("target/coverage");
        std::fs::create_dir_all(&coverage_dir).unwrap();
        std::fs::write(coverage_dir.join("lcov.info"), create_lcov_content()).unwrap();

        // Try primary path that doesn't exist - should fall back
        let effect = load_coverage_effect(
            temp_dir.path().join("nonexistent.lcov"),
            temp_dir.path().to_path_buf(),
        );
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let coverage = result.unwrap();
        assert!(coverage
            .get_file_coverage(std::path::Path::new("src/main.rs"))
            .is_some());
    }
}
