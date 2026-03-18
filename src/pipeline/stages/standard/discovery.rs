//! File discovery stage for finding source files.
//!
//! Scans project directories to find source files matching specified languages.

use crate::core::Language;
use crate::errors::AnalysisError;
use crate::io::walker::FileWalker;
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

/// Discover source files in the given path.
///
/// debtmap:ignore[testing] - I/O shell delegates to `FileWalker`, which requires
/// filesystem fixtures to exercise ignore handling and language filtering.
fn discover_files(path: &Path, languages: &[Language]) -> Result<Vec<PathBuf>, AnalysisError> {
    FileWalker::new(path.to_path_buf())
        .with_languages(languages.to_vec())
        .walk()
        .map_err(AnalysisError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_discovery_stage_creation() {
        let stage = FileDiscoveryStage::new(Path::new("."), &[Language::Rust]);
        assert_eq!(stage.name(), "File Discovery");
    }

    #[test]
    fn discover_files_filters_to_requested_languages() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("src/tool.py"), "def tool(): pass").unwrap();

        let files = discover_files(root, &[Language::Rust]).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("src/main.rs"));
    }

    #[test]
    fn discover_files_skips_git_worktree_entries() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join(".git/worktrees/feature/src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(
            root.join(".git/worktrees/feature/src/duplicate.rs"),
            "fn duplicate() {}",
        )
        .unwrap();

        let files = discover_files(root, &[Language::Rust]).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("src/main.rs"));
    }
}
