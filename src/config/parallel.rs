//! Parallelism configuration for batch analysis operations.
//!
//! This module provides configuration for controlling parallel execution
//! of file analysis operations using stillwater's traverse patterns.

use serde::{Deserialize, Serialize};

/// Default value for parallel processing enabled
fn default_enabled() -> bool {
    true
}

/// Default batch size for chunked processing
fn default_batch_size() -> usize {
    100
}

/// Configuration for parallel processing operations.
///
/// This struct controls how debtmap executes batch operations like
/// multi-file analysis. When enabled, files are processed concurrently
/// using rayon's thread pool.
///
/// # Example
///
/// ```rust
/// use debtmap::config::ParallelConfig;
///
/// let config = ParallelConfig {
///     enabled: true,
///     max_concurrency: Some(4),
///     batch_size: Some(50),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParallelConfig {
    /// Enable parallel processing (default: true)
    ///
    /// When disabled, files are processed sequentially.
    /// Useful for debugging or when running in constrained environments.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Maximum concurrent operations (default: num_cpus)
    ///
    /// Limits the number of concurrent file analyses.
    /// If None, uses all available CPU cores.
    #[serde(default)]
    pub max_concurrency: Option<usize>,

    /// Batch size for chunked processing (default: 100)
    ///
    /// Large codebases are processed in batches to prevent
    /// resource exhaustion and enable progress reporting.
    #[serde(default = "default_batch_size_option")]
    pub batch_size: Option<usize>,
}

fn default_batch_size_option() -> Option<usize> {
    Some(default_batch_size())
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            max_concurrency: None,
            batch_size: Some(default_batch_size()),
        }
    }
}

impl ParallelConfig {
    /// Create a new parallel config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config with parallel processing disabled.
    pub fn sequential() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Get the effective concurrency level.
    ///
    /// Returns the configured max_concurrency, or the number of
    /// available CPU cores if not specified.
    pub fn effective_concurrency(&self) -> usize {
        self.max_concurrency.unwrap_or_else(num_cpus)
    }

    /// Get the effective batch size.
    pub fn effective_batch_size(&self) -> usize {
        self.batch_size.unwrap_or(default_batch_size())
    }
}

/// Returns the number of available CPU cores.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1)
}

/// Configuration for batch analysis operations.
///
/// This struct controls how batch file analysis behaves,
/// including parallelism, error handling, and timing collection.
///
/// # Example
///
/// ```rust
/// use debtmap::config::{BatchAnalysisConfig, ParallelConfig};
///
/// let config = BatchAnalysisConfig {
///     parallelism: ParallelConfig::default(),
///     fail_fast: false,  // Collect all errors
///     collect_timing: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BatchAnalysisConfig {
    /// Parallelism configuration
    #[serde(default)]
    pub parallelism: ParallelConfig,

    /// Fail fast on first error (default: false)
    ///
    /// When false, uses Validation to accumulate ALL errors.
    /// When true, stops at the first error (traditional Result behavior).
    #[serde(default)]
    pub fail_fast: bool,

    /// Collect timing information (default: false)
    ///
    /// When enabled, each FileAnalysisResult includes the
    /// analysis duration for performance monitoring.
    #[serde(default)]
    pub collect_timing: bool,
}

impl BatchAnalysisConfig {
    /// Create a config for accumulating all errors.
    pub fn accumulating() -> Self {
        Self {
            parallelism: ParallelConfig::default(),
            fail_fast: false,
            collect_timing: false,
        }
    }

    /// Create a config for fail-fast behavior.
    pub fn fail_fast() -> Self {
        Self {
            parallelism: ParallelConfig::default(),
            fail_fast: true,
            collect_timing: false,
        }
    }

    /// Create a config with timing collection enabled.
    pub fn with_timing(mut self) -> Self {
        self.collect_timing = true;
        self
    }

    /// Create a config with sequential processing.
    pub fn sequential(mut self) -> Self {
        self.parallelism = ParallelConfig::sequential();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert!(config.enabled);
        assert!(config.max_concurrency.is_none());
        assert_eq!(config.batch_size, Some(100));
    }

    #[test]
    fn test_parallel_config_sequential() {
        let config = ParallelConfig::sequential();
        assert!(!config.enabled);
    }

    #[test]
    fn test_effective_concurrency() {
        // With explicit value
        let config = ParallelConfig {
            enabled: true,
            max_concurrency: Some(4),
            batch_size: None,
        };
        assert_eq!(config.effective_concurrency(), 4);

        // Without explicit value (uses num_cpus)
        let config = ParallelConfig::default();
        assert!(config.effective_concurrency() >= 1);
    }

    #[test]
    fn test_effective_batch_size() {
        let config = ParallelConfig::default();
        assert_eq!(config.effective_batch_size(), 100);

        let config = ParallelConfig {
            batch_size: Some(50),
            ..Default::default()
        };
        assert_eq!(config.effective_batch_size(), 50);

        let config = ParallelConfig {
            batch_size: None,
            ..Default::default()
        };
        assert_eq!(config.effective_batch_size(), 100);
    }

    #[test]
    fn test_batch_config_default() {
        let config = BatchAnalysisConfig::default();
        assert!(!config.fail_fast);
        assert!(!config.collect_timing);
        assert!(config.parallelism.enabled);
    }

    #[test]
    fn test_batch_config_accumulating() {
        let config = BatchAnalysisConfig::accumulating();
        assert!(!config.fail_fast);
    }

    #[test]
    fn test_batch_config_fail_fast() {
        let config = BatchAnalysisConfig::fail_fast();
        assert!(config.fail_fast);
    }

    #[test]
    fn test_batch_config_with_timing() {
        let config = BatchAnalysisConfig::default().with_timing();
        assert!(config.collect_timing);
    }

    #[test]
    fn test_batch_config_sequential() {
        let config = BatchAnalysisConfig::default().sequential();
        assert!(!config.parallelism.enabled);
    }

    #[test]
    fn test_parallel_config_serde() {
        let config = ParallelConfig {
            enabled: true,
            max_concurrency: Some(8),
            batch_size: Some(200),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ParallelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_batch_config_serde() {
        let config = BatchAnalysisConfig {
            parallelism: ParallelConfig::default(),
            fail_fast: true,
            collect_timing: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: BatchAnalysisConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
    }
}
