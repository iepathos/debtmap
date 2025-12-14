//! File discovery stage for finding source files.
//!
//! Scans project directories to find source files matching specified languages.

use crate::core::Language;
use crate::errors::AnalysisError;
use crate::pipeline::data::PipelineData;
use crate::pipeline::stage::Stage;
use std::path::{Path, PathBuf};

/// Stage 1: Discover project files
///
/// Scans the project directory for source files matching the specified languages.
pub struct FileDiscoveryStage {
    path: PathBuf,
    languages: Vec<Language>,
}

impl FileDiscoveryStage {
    pub fn new(path: &Path, languages: &[Language]) -> Self {
        Self {
            path: path.to_path_buf(),
            languages: languages.to_vec(),
        }
    }
}

impl Stage for FileDiscoveryStage {
    type Input = ();
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, _input: Self::Input) -> Result<Self::Output, Self::Error> {
        let files = discover_files(&self.path, &self.languages)?;
        Ok(PipelineData::new(files))
    }

    fn name(&self) -> &str {
        "File Discovery"
    }
}

// =============================================================================
// Pure Predicates
// =============================================================================

/// Check if a path has one of the specified extensions.
///
/// Pure function: path + extensions in, boolean out.
fn matches_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.contains(&ext))
        .unwrap_or(false)
}

// =============================================================================
// I/O - File Discovery
// =============================================================================

/// Discover source files in the given path.
///
/// Walks the directory tree and filters files by extension.
fn discover_files(path: &Path, _languages: &[Language]) -> Result<Vec<PathBuf>, AnalysisError> {
    use walkdir::WalkDir;

    let mut files = Vec::new();

    // For now, only support Rust files
    let extensions = ["rs"];

    let mut skipped_count = 0;
    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                if skipped_count < 10 {
                    eprintln!("Warning: Skipping directory entry: {}", err);
                }
                skipped_count += 1;
                None
            }
        })
    {
        if entry.file_type().is_file() {
            let file_path = entry.path();
            if matches_extension(file_path, &extensions) {
                files.push(file_path.to_path_buf());
            }
        }
    }

    if skipped_count > 10 {
        eprintln!(
            "Warning: Skipped {} additional directory entries",
            skipped_count - 10
        );
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_discovery_stage_creation() {
        let stage = FileDiscoveryStage::new(Path::new("."), &[Language::Rust]);
        assert_eq!(stage.name(), "File Discovery");
    }

    #[test]
    fn matches_extension_rust_file() {
        assert!(matches_extension(Path::new("foo.rs"), &["rs"]));
        assert!(matches_extension(Path::new("/path/to/bar.rs"), &["rs"]));
    }

    #[test]
    fn matches_extension_non_rust_file() {
        assert!(!matches_extension(Path::new("foo.py"), &["rs"]));
        assert!(!matches_extension(Path::new("foo.rs.bak"), &["rs"]));
    }

    #[test]
    fn matches_extension_no_extension() {
        assert!(!matches_extension(Path::new("Makefile"), &["rs"]));
        assert!(!matches_extension(Path::new("/path/to/README"), &["rs"]));
    }

    #[test]
    fn matches_extension_multiple_extensions() {
        assert!(matches_extension(Path::new("foo.rs"), &["rs", "py", "js"]));
        assert!(matches_extension(Path::new("foo.py"), &["rs", "py", "js"]));
        assert!(!matches_extension(Path::new("foo.rb"), &["rs", "py", "js"]));
    }
}
