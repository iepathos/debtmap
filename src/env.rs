//! Environment trait and implementations for debtmap analysis.
//!
//! This module provides the core environment abstraction for the effect system,
//! defining all I/O capabilities that analysis operations may need. The environment
//! pattern enables:
//!
//! - **Dependency injection**: Pass capabilities explicitly rather than using globals
//! - **Testability**: Use mock environments in tests
//! - **Pure core**: Separate I/O from business logic at the type level
//!
//! # The Environment Pattern
//!
//! The environment pattern (also known as Reader pattern) threads shared context
//! through a computation without explicit parameter passing. Effect types are
//! parameterized by the environment, allowing functions to declare what capabilities
//! they need.
//!
//! ```rust,ignore
//! use debtmap::effects::AnalysisEffect;
//!
//! // This function needs file system access, declared via the Effect type
//! fn read_source(path: PathBuf) -> AnalysisEffect<String> {
//!     Effect::new(|env| env.file_system().read_to_string(&path))
//! }
//! ```
//!
//! # Usage
//!
//! ## Production Code
//!
//! ```rust,ignore
//! use debtmap::env::RealEnv;
//! use debtmap::config::DebtmapConfig;
//!
//! let config = DebtmapConfig::default();
//! let env = RealEnv::new(config);
//! let result = my_effect.run(&env)?;
//! ```
//!
//! ## Testing
//!
//! ```rust,ignore
//! use debtmap::env::TestEnv;
//!
//! let mut env = TestEnv::new();
//! env.add_file("test.rs", "fn main() {}");
//! let result = my_effect.run(&env)?;
//! ```

use crate::config::DebtmapConfig;
use crate::io::real::{MemoryCache, NoOpCache, RealCoverageLoader, RealFileSystem};
use crate::io::traits::{Cache, CoverageLoader, FileSystem};
use crate::progress::traits::{HasProgress, ProgressSink};
use crate::progress::SilentProgressSink;
use std::sync::Arc;

/// Environment trait defining all I/O capabilities for analysis operations.
///
/// This trait provides access to all external resources that analysis code
/// might need. By parameterizing Effect types with this trait, we can:
///
/// 1. Make I/O requirements explicit in function signatures
/// 2. Easily swap implementations for testing
/// 3. Add new capabilities without changing existing code
///
/// # Thread Safety
///
/// All environment implementations must be `Clone + Send + Sync` to support
/// parallel analysis. Implementations should use `Arc` for shared resources.
///
/// # Design Notes
///
/// The environment returns trait objects (`&dyn Trait`) rather than concrete
/// types for flexibility. This allows different implementations while keeping
/// the interface stable.
pub trait AnalysisEnv: Clone + Send + Sync {
    /// Access file system operations.
    ///
    /// Use this for reading source files, writing output, etc.
    fn file_system(&self) -> &dyn FileSystem;

    /// Access coverage data loading.
    ///
    /// Use this for loading LCOV, Cobertura, or other coverage formats.
    fn coverage_loader(&self) -> &dyn CoverageLoader;

    /// Access cache operations.
    ///
    /// Use this for caching parsed ASTs, analysis results, etc.
    fn cache(&self) -> &dyn Cache;

    /// Access the debtmap configuration.
    ///
    /// This provides access to thresholds, scoring weights, and other settings.
    fn config(&self) -> &DebtmapConfig;

    /// Create a new environment with the updated config.
    ///
    /// This is used by the Reader pattern's `local` operation to run effects
    /// with temporarily modified configuration. Returns a new environment
    /// instance with the updated config (immutable pattern).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use debtmap::env::AnalysisEnv;
    ///
    /// fn with_strict_thresholds<E: AnalysisEnv>(env: E) -> E {
    ///     let mut config = env.config().clone();
    ///     // Modify config...
    ///     env.with_config(config)
    /// }
    /// ```
    fn with_config(self, config: DebtmapConfig) -> Self;
}

/// Production environment implementation.
///
/// This is the default environment used in production, providing real
/// implementations of all I/O traits.
///
/// # Example
///
/// ```rust
/// use debtmap::env::RealEnv;
/// use debtmap::config::DebtmapConfig;
///
/// let config = DebtmapConfig::default();
/// let env = RealEnv::new(config);
///
/// // Now use env with Effect types
/// ```
#[derive(Clone)]
pub struct RealEnv {
    file_system: Arc<dyn FileSystem>,
    coverage_loader: Arc<dyn CoverageLoader>,
    cache: Arc<dyn Cache>,
    config: DebtmapConfig,
    progress: Arc<dyn ProgressSink>,
}

impl RealEnv {
    /// Create a new production environment with the given configuration.
    ///
    /// This sets up:
    /// - Real file system access
    /// - Real coverage loader (LCOV, etc.)
    /// - In-memory cache (for analysis results)
    /// - Silent progress sink (default, no output)
    ///
    /// For progress reporting, use [`RealEnv::with_progress`] to provide
    /// a custom progress sink.
    pub fn new(config: DebtmapConfig) -> Self {
        Self {
            file_system: Arc::new(RealFileSystem::new()),
            coverage_loader: Arc::new(RealCoverageLoader::new()),
            cache: Arc::new(MemoryCache::new()),
            config,
            progress: Arc::new(SilentProgressSink),
        }
    }

    /// Create an environment with a custom progress sink.
    ///
    /// Use this when you need progress reporting:
    ///
    /// ```rust,ignore
    /// use debtmap::progress::CliProgressSink;
    ///
    /// let sink = Arc::new(CliProgressSink::new(false));
    /// let env = RealEnv::with_progress(config, sink);
    /// ```
    ///
    /// For testing, use [`RecordingProgressSink`](crate::progress::RecordingProgressSink)
    /// to capture and verify progress events.
    pub fn with_progress(config: DebtmapConfig, progress: Arc<dyn ProgressSink>) -> Self {
        Self {
            file_system: Arc::new(RealFileSystem::new()),
            coverage_loader: Arc::new(RealCoverageLoader::new()),
            cache: Arc::new(MemoryCache::new()),
            config,
            progress,
        }
    }

    /// Create an environment with no caching.
    ///
    /// Useful for one-shot analysis where caching overhead isn't worth it.
    pub fn without_cache(config: DebtmapConfig) -> Self {
        Self {
            file_system: Arc::new(RealFileSystem::new()),
            coverage_loader: Arc::new(RealCoverageLoader::new()),
            cache: Arc::new(NoOpCache::new()),
            config,
            progress: Arc::new(SilentProgressSink),
        }
    }

    /// Create an environment with custom implementations.
    ///
    /// This is useful for advanced use cases where you need to customize
    /// specific components while keeping others at their defaults.
    pub fn custom(
        file_system: Arc<dyn FileSystem>,
        coverage_loader: Arc<dyn CoverageLoader>,
        cache: Arc<dyn Cache>,
        config: DebtmapConfig,
    ) -> Self {
        Self {
            file_system,
            coverage_loader,
            cache,
            config,
            progress: Arc::new(SilentProgressSink),
        }
    }

    /// Create an environment with all custom components including progress.
    ///
    /// This is the most flexible constructor, allowing customization of
    /// all environment components.
    pub fn custom_with_progress(
        file_system: Arc<dyn FileSystem>,
        coverage_loader: Arc<dyn CoverageLoader>,
        cache: Arc<dyn Cache>,
        config: DebtmapConfig,
        progress: Arc<dyn ProgressSink>,
    ) -> Self {
        Self {
            file_system,
            coverage_loader,
            cache,
            config,
            progress,
        }
    }

    /// Update the configuration.
    ///
    /// Returns a new environment with the updated config (immutable pattern).
    pub fn with_config(self, config: DebtmapConfig) -> Self {
        Self { config, ..self }
    }

    /// Set the progress sink.
    ///
    /// Returns a new environment with the updated progress sink (immutable pattern).
    pub fn set_progress(self, progress: Arc<dyn ProgressSink>) -> Self {
        Self { progress, ..self }
    }
}

impl AnalysisEnv for RealEnv {
    fn file_system(&self) -> &dyn FileSystem {
        &*self.file_system
    }

    fn coverage_loader(&self) -> &dyn CoverageLoader {
        &*self.coverage_loader
    }

    fn cache(&self) -> &dyn Cache {
        &*self.cache
    }

    fn config(&self) -> &DebtmapConfig {
        &self.config
    }

    fn with_config(self, config: DebtmapConfig) -> Self {
        Self { config, ..self }
    }
}

impl HasProgress for RealEnv {
    fn progress(&self) -> &dyn ProgressSink {
        &*self.progress
    }
}

impl Default for RealEnv {
    fn default() -> Self {
        Self::new(DebtmapConfig::default())
    }
}

impl std::fmt::Debug for RealEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealEnv")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_real_env_creation() {
        let config = DebtmapConfig::default();
        let env = RealEnv::new(config);

        // Config should be accessible (just verify it doesn't panic)
        let _ = env.config();
    }

    #[test]
    fn test_real_env_file_system() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let env = RealEnv::default();

        // File doesn't exist yet
        assert!(!env.file_system().exists(&file_path));

        // Write via std::fs for testing
        std::fs::write(&file_path, "test content").unwrap();

        // Now it exists
        assert!(env.file_system().exists(&file_path));
        assert!(env.file_system().is_file(&file_path));

        // Read via env
        let content = env.file_system().read_to_string(&file_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_real_env_cache() {
        let env = RealEnv::default();

        // Cache operations
        env.cache().set("test_key", b"test_value").unwrap();
        assert_eq!(env.cache().get("test_key"), Some(b"test_value".to_vec()));

        env.cache().invalidate("test_key").unwrap();
        assert!(env.cache().get("test_key").is_none());
    }

    #[test]
    fn test_real_env_without_cache() {
        let env = RealEnv::without_cache(DebtmapConfig::default());

        // Cache operations should work but return None
        env.cache().set("key", b"value").unwrap();
        assert!(env.cache().get("key").is_none());
    }

    #[test]
    fn test_real_env_with_config() {
        let config1 = DebtmapConfig::default();
        let config2 = DebtmapConfig {
            ignore: Some(crate::config::IgnoreConfig {
                patterns: vec!["test".to_string()],
            }),
            ..Default::default()
        };

        let env = RealEnv::new(config1);
        assert!(env.config().ignore.is_none());

        let env = env.with_config(config2);
        assert!(env.config().ignore.is_some());
    }

    #[test]
    fn test_real_env_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RealEnv>();
    }

    #[test]
    fn test_real_env_is_clone() {
        let env1 = RealEnv::default();
        let env2 = env1.clone();

        // Both should work independently
        assert!(!env1.file_system().exists(Path::new("/nonexistent")));
        assert!(!env2.file_system().exists(Path::new("/nonexistent")));
    }

    #[test]
    fn test_real_env_default_has_silent_progress() {
        let env = RealEnv::default();

        // Silent progress should not panic on any operation
        env.progress().start_stage("Test");
        env.progress().report("Test", 0, 10);
        env.progress().complete_stage("Test");
        env.progress().warn("Warning");
    }

    #[test]
    fn test_real_env_with_progress() {
        use crate::progress::RecordingProgressSink;

        let recorder = Arc::new(RecordingProgressSink::new());
        let env = RealEnv::with_progress(DebtmapConfig::default(), recorder.clone());

        env.progress().start_stage("Analysis");
        env.progress().report("Analysis", 5, 10);
        env.progress().complete_stage("Analysis");

        assert_eq!(recorder.stages(), vec!["Analysis"]);
        assert_eq!(recorder.completed_stages(), vec!["Analysis"]);
        assert_eq!(recorder.event_count(), 3);
    }

    #[test]
    fn test_real_env_set_progress() {
        use crate::progress::RecordingProgressSink;

        let env = RealEnv::default();
        let recorder = Arc::new(RecordingProgressSink::new());

        let env = env.set_progress(recorder.clone());

        env.progress().start_stage("Test");
        assert_eq!(recorder.stages(), vec!["Test"]);
    }
}
