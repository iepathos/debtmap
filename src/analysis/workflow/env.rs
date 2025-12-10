//! Environment Traits for Analysis Workflow (Spec 202)
//!
//! Environment traits enable the "pure guards, effectful actions" pattern.
//! Actions receive environment references to perform side effects like:
//! - Progress reporting (TUI updates, spinners)
//! - File system operations (reading files)
//! - Logging and warnings
//!
//! This separation allows:
//! - Easy mocking for tests
//! - Different implementations for CLI vs TUI
//! - Pure guards that don't need environments

use std::io;
use std::path::Path;

/// Environment trait for progress reporting.
///
/// Implementations can report progress to different targets:
/// - TUI progress bars
/// - CLI spinners
/// - Quiet mode (no output)
/// - Tests (capture output)
pub trait ProgressReporter {
    /// Report that a phase is starting.
    fn phase_starting(&mut self, phase: &str);

    /// Report progress within a phase (0.0 - 1.0).
    fn phase_progress(&mut self, progress: f64);

    /// Report that a phase completed.
    fn phase_complete(&mut self);

    /// Report a warning during analysis.
    fn warn(&mut self, message: &str);

    /// Report an info message during analysis.
    fn info(&mut self, message: &str);
}

/// Environment trait for file system operations.
///
/// Abstraction over file system to enable testing without real files.
pub trait FileSystem {
    /// Read file contents as bytes.
    fn read_file(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Read file contents as UTF-8 string.
    fn read_file_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read_file(path)?;
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Check if file exists.
    fn file_exists(&self, path: &Path) -> bool;
}

/// Combined environment for analysis workflow.
///
/// Types implementing both `ProgressReporter` and `FileSystem` automatically
/// implement `AnalysisEnv`.
pub trait AnalysisEnv: ProgressReporter + FileSystem {}

impl<T: ProgressReporter + FileSystem> AnalysisEnv for T {}

/// Real environment implementation using actual filesystem and progress reporting.
pub struct RealAnalysisEnv {
    quiet_mode: bool,
}

impl RealAnalysisEnv {
    /// Create a new real analysis environment.
    pub fn new() -> Self {
        let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();
        Self { quiet_mode }
    }

    /// Create a new real analysis environment with explicit quiet mode.
    pub fn with_quiet(quiet: bool) -> Self {
        Self { quiet_mode: quiet }
    }
}

impl Default for RealAnalysisEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for RealAnalysisEnv {
    fn phase_starting(&mut self, phase: &str) {
        // Log the phase start
        log::info!("Phase starting: {}", phase);
    }

    fn phase_progress(&mut self, progress: f64) {
        // Report to TUI if available
        if let Some(manager) = crate::progress::ProgressManager::global() {
            manager.tui_set_progress(progress);
        }
    }

    fn phase_complete(&mut self) {
        log::debug!("Phase complete");
    }

    fn warn(&mut self, message: &str) {
        if !self.quiet_mode {
            log::warn!("{}", message);
        }
    }

    fn info(&mut self, message: &str) {
        if !self.quiet_mode {
            log::info!("{}", message);
        }
    }
}

impl FileSystem for RealAnalysisEnv {
    fn read_file(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

/// Mock environment for testing.
///
/// Captures all progress events and provides controlled file system responses.
#[cfg(test)]
pub struct MockAnalysisEnv {
    /// Recorded phase start events.
    pub phases: Vec<String>,

    /// Recorded progress updates.
    pub progress_updates: Vec<f64>,

    /// Recorded warnings.
    pub warnings: Vec<String>,

    /// Recorded info messages.
    pub infos: Vec<String>,

    /// Mock file system contents.
    pub files: std::collections::HashMap<std::path::PathBuf, Vec<u8>>,
}

#[cfg(test)]
impl MockAnalysisEnv {
    /// Create a new mock environment.
    pub fn new() -> Self {
        Self {
            phases: vec![],
            progress_updates: vec![],
            warnings: vec![],
            infos: vec![],
            files: std::collections::HashMap::new(),
        }
    }

    /// Add a mock file to the environment.
    pub fn with_file(
        mut self,
        path: impl Into<std::path::PathBuf>,
        content: impl AsRef<[u8]>,
    ) -> Self {
        self.files.insert(path.into(), content.as_ref().to_vec());
        self
    }

    /// Check if a phase was started.
    pub fn phase_started(&self, phase: &str) -> bool {
        self.phases
            .iter()
            .any(|p| p.starts_with(&format!("start:{}", phase)))
    }

    /// Get the number of phase completions.
    pub fn completion_count(&self) -> usize {
        self.phases.iter().filter(|p| *p == "complete").count()
    }
}

#[cfg(test)]
impl Default for MockAnalysisEnv {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl ProgressReporter for MockAnalysisEnv {
    fn phase_starting(&mut self, phase: &str) {
        self.phases.push(format!("start:{}", phase));
    }

    fn phase_progress(&mut self, progress: f64) {
        self.progress_updates.push(progress);
    }

    fn phase_complete(&mut self) {
        self.phases.push("complete".to_string());
    }

    fn warn(&mut self, message: &str) {
        self.warnings.push(message.to_string());
    }

    fn info(&mut self, message: &str) {
        self.infos.push(message.to_string());
    }
}

#[cfg(test)]
impl FileSystem for MockAnalysisEnv {
    fn read_file(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.files.get(path).cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("file not found: {}", path.display()),
            )
        })
    }

    fn file_exists(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_mock_env_phase_tracking() {
        let mut env = MockAnalysisEnv::new();

        env.phase_starting("Building call graph");
        env.phase_progress(0.5);
        env.phase_complete();

        assert!(env.phase_started("Building call graph"));
        assert_eq!(env.progress_updates, vec![0.5]);
        assert_eq!(env.completion_count(), 1);
    }

    #[test]
    fn test_mock_env_file_system() {
        let env = MockAnalysisEnv::new()
            .with_file("test.rs", "fn main() {}")
            .with_file(PathBuf::from("src/lib.rs"), b"pub fn lib() {}");

        assert!(env.file_exists(Path::new("test.rs")));
        assert!(env.file_exists(Path::new("src/lib.rs")));
        assert!(!env.file_exists(Path::new("nonexistent.rs")));

        let content = env.read_file(Path::new("test.rs")).unwrap();
        assert_eq!(content, b"fn main() {}");

        let string_content = env.read_file_string(Path::new("test.rs")).unwrap();
        assert_eq!(string_content, "fn main() {}");
    }

    #[test]
    fn test_mock_env_file_not_found() {
        let env = MockAnalysisEnv::new();

        let result = env.read_file(Path::new("nonexistent.rs"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_mock_env_warnings() {
        let mut env = MockAnalysisEnv::new();

        env.warn("Warning 1");
        env.warn("Warning 2");
        env.info("Info message");

        assert_eq!(env.warnings.len(), 2);
        assert_eq!(env.warnings[0], "Warning 1");
        assert_eq!(env.infos.len(), 1);
    }

    #[test]
    fn test_real_env_file_system() {
        // Create a temporary file for testing
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn test() {}").unwrap();

        let env = RealAnalysisEnv::new();

        assert!(env.file_exists(&test_file));
        assert!(!env.file_exists(&temp_dir.path().join("nonexistent.rs")));

        let content = env.read_file(&test_file).unwrap();
        assert_eq!(content, b"fn test() {}");
    }
}
