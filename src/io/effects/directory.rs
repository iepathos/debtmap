//! Directory walking effect operations.
//!
//! This module provides Effect-based wrappers for directory traversal:
//! - Walking directories with default settings
//! - Walking directories with configuration-based ignore patterns
//!
//! # Design Philosophy
//!
//! Directory walking operations are wrapped in Effect types, enabling
//! testing with mock environments and composability with other effects.

use crate::effects::AnalysisEffect;
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use crate::io::walker::FileWalker;
use std::path::PathBuf;
use stillwater::effect::prelude::*;

/// Walk a directory and return all files as an Effect.
///
/// Uses the configured FileWalker with default settings. For more control
/// over which files are returned, use `walk_dir_with_config_effect`.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::walk_dir_effect;
///
/// let effect = walk_dir_effect("src".into());
/// let files = run_effect(effect, DebtmapConfig::default())?;
/// for file in files {
///     println!("{}", file.display());
/// }
/// ```
///
/// # Errors
///
/// Returns `AnalysisError::IoError` if:
/// - The directory doesn't exist
/// - Permission is denied
pub fn walk_dir_effect(path: PathBuf) -> AnalysisEffect<Vec<PathBuf>> {
    let path_display = path.display().to_string();
    from_fn(move |_env: &RealEnv| {
        FileWalker::new(path.clone()).walk().map_err(|e| {
            AnalysisError::io_with_path(
                format!("Failed to walk directory '{}': {}", path_display, e),
                &path,
            )
        })
    })
    .boxed()
}

/// Walk a directory with configuration-based ignore patterns.
///
/// This uses the environment's configuration to determine which files
/// to include or exclude during the walk.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::walk_dir_with_config_effect;
/// use debtmap::core::Language;
///
/// let effect = walk_dir_with_config_effect(
///     "src".into(),
///     vec![Language::Rust],
/// );
/// let rust_files = run_effect(effect, config)?;
/// ```
pub fn walk_dir_with_config_effect(
    path: PathBuf,
    languages: Vec<crate::core::Language>,
) -> AnalysisEffect<Vec<PathBuf>> {
    let path_display = path.display().to_string();
    from_fn(move |env: &RealEnv| {
        let ignore_patterns = env.config().get_ignore_patterns();

        FileWalker::new(path.clone())
            .with_languages(languages.clone())
            .with_ignore_patterns(ignore_patterns)
            .walk()
            .map_err(|e| {
                AnalysisError::io_with_path(
                    format!("Failed to walk directory '{}': {}", path_display, e),
                    &path,
                )
            })
    })
    .boxed()
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

    #[test]
    fn test_walk_dir_effect() {
        let (temp_dir, config) = create_test_env();
        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(src_dir.join("lib.rs"), "pub fn hello() {}").unwrap();

        let effect = walk_dir_effect(src_dir);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 2);
    }
}
