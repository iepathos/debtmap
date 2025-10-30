//! Builder pattern for SharedCache construction
//!
//! This module provides a flexible builder API for constructing SharedCache instances
//! with various configuration options.

use crate::cache::auto_pruner::{AutoPruner, BackgroundPruner};
use crate::cache::cache_location::{CacheLocation, CacheStrategy};
use crate::cache::index_manager::IndexManager;
use crate::cache::shared_cache::{CacheReader, CacheWriter, SharedCache};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Builder for constructing SharedCache instances
pub struct SharedCacheBuilder {
    repo_path: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    auto_pruner: Option<AutoPruner>,
    max_cache_size: Option<u64>,
    cleanup_threshold: Option<f64>,
}

impl SharedCacheBuilder {
    /// Start building a new SharedCache
    pub fn new() -> Self {
        Self {
            repo_path: None,
            cache_dir: None,
            auto_pruner: None,
            max_cache_size: None,
            cleanup_threshold: None,
        }
    }

    /// Set the repository path
    pub fn repo_path(mut self, path: impl AsRef<Path>) -> Self {
        self.repo_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set a custom cache directory
    pub fn cache_dir(mut self, dir: PathBuf) -> Self {
        self.cache_dir = Some(dir);
        self
    }

    /// Configure auto-pruning
    pub fn auto_pruner(mut self, pruner: AutoPruner) -> Self {
        self.auto_pruner = Some(pruner);
        self
    }

    /// Set maximum cache size in bytes
    pub fn max_cache_size(mut self, size: u64) -> Self {
        self.max_cache_size = Some(size);
        self
    }

    /// Set cleanup threshold (0.0 to 1.0)
    pub fn cleanup_threshold(mut self, threshold: f64) -> Self {
        self.cleanup_threshold = Some(threshold);
        self
    }

    /// Build the SharedCache instance
    pub fn build(self) -> Result<SharedCache> {
        let location = match (self.repo_path.as_deref(), self.cache_dir) {
            (repo_path, Some(cache_dir)) => {
                let strategy = CacheStrategy::Custom(cache_dir);
                CacheLocation::resolve_with_strategy(repo_path, strategy)?
            }
            (repo_path, None) => CacheLocation::resolve(repo_path)?,
        };

        location.ensure_directories()?;

        // Create shared IndexManager wrapped in Arc
        let index_manager = Arc::new(IndexManager::load_or_create(&location)?);

        // Create reader and writer with shared index manager
        let reader = CacheReader::new(location.clone(), Arc::clone(&index_manager));
        let writer = CacheWriter::new(location.clone(), Arc::clone(&index_manager));

        // Determine auto-pruner: use provided, or create from environment
        let auto_pruner = match self.auto_pruner {
            Some(pruner) => Some(pruner),
            None => {
                if std::env::var("DEBTMAP_CACHE_AUTO_PRUNE")
                    .unwrap_or_else(|_| "true".to_string())
                    .to_lowercase()
                    == "true"
                {
                    Some(AutoPruner::from_env())
                } else {
                    None
                }
            }
        };

        let background_pruner = auto_pruner
            .as_ref()
            .map(|p| BackgroundPruner::new(p.clone()));

        // Use provided max size, auto-pruner's max size, or default
        let max_cache_size = self
            .max_cache_size
            .or_else(|| auto_pruner.as_ref().map(|p| p.max_size_bytes as u64))
            .unwrap_or(1024 * 1024 * 1024); // 1GB default

        // Use provided cleanup threshold or default
        let cleanup_threshold = self.cleanup_threshold.unwrap_or(0.9);

        Ok(SharedCache {
            location,
            reader,
            writer,
            index_manager,
            max_cache_size,
            cleanup_threshold,
            auto_pruner,
            background_pruner,
        })
    }
}

impl Default for SharedCacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_builder_basic() {
        let temp = TempDir::new().unwrap();
        let cache = SharedCacheBuilder::new()
            .cache_dir(temp.path().to_path_buf())
            .build();

        assert!(cache.is_ok());
    }

    #[test]
    fn test_builder_with_auto_pruner() {
        let temp = TempDir::new().unwrap();
        let pruner = AutoPruner {
            max_size_bytes: 1024 * 1024,
            max_entries: 100,
            ..Default::default()
        };

        let cache = SharedCacheBuilder::new()
            .cache_dir(temp.path().to_path_buf())
            .auto_pruner(pruner)
            .build();

        assert!(cache.is_ok());
        let cache = cache.unwrap();
        assert_eq!(cache.max_cache_size, 1024 * 1024);
    }

    #[test]
    fn test_builder_with_custom_max_size() {
        let temp = TempDir::new().unwrap();
        let custom_size = 2048 * 1024;

        let cache = SharedCacheBuilder::new()
            .cache_dir(temp.path().to_path_buf())
            .max_cache_size(custom_size)
            .build();

        assert!(cache.is_ok());
        let cache = cache.unwrap();
        assert_eq!(cache.max_cache_size, custom_size);
    }

    #[test]
    fn test_builder_with_cleanup_threshold() {
        let temp = TempDir::new().unwrap();
        let custom_threshold = 0.8;

        let cache = SharedCacheBuilder::new()
            .cache_dir(temp.path().to_path_buf())
            .cleanup_threshold(custom_threshold)
            .build();

        assert!(cache.is_ok());
        let cache = cache.unwrap();
        assert_eq!(cache.cleanup_threshold, custom_threshold);
    }

    #[test]
    fn test_builder_default() {
        let temp = TempDir::new().unwrap();
        let cache = SharedCacheBuilder::default()
            .cache_dir(temp.path().to_path_buf())
            .build();

        assert!(cache.is_ok());
    }
}
