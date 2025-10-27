//! Cache Reader Module
//!
//! Provides read-only operations for the shared cache system.
//!
//! ## Design Principles
//!
//! - **Pure Functions**: `compute_cache_key` and `get_cache_file_path` are deterministic
//! - **Minimal Side Effects**: Only side effect is updating access metadata
//! - **Concurrency-Safe**: Multiple readers can operate concurrently
//!
//! ## Example
//!
//! ```rust,no_run
//! use debtmap::cache::shared_cache::reader::CacheReader;
//! use debtmap::cache::cache_location::CacheLocation;
//! use debtmap::cache::index_manager::IndexManager;
//! use std::sync::Arc;
//! use std::path::Path;
//!
//! # fn example() -> anyhow::Result<()> {
//! let location = CacheLocation::resolve(None)?;
//! let index_manager = Arc::new(IndexManager::load_or_create(&location)?);
//! let reader = CacheReader::new(location, index_manager);
//!
//! if reader.exists("my_key", "ast") {
//!     let data = reader.get("my_key", "ast")?;
//!     // ... use cached data
//! }
//! # Ok(())
//! # }
//! ```

use crate::cache::cache_location::CacheLocation;
use crate::cache::index_manager::IndexManager;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Read-only operations for cache access
pub struct CacheReader {
    location: CacheLocation,
    index_manager: Arc<IndexManager>,
}

impl CacheReader {
    /// Create a new CacheReader
    pub fn new(location: CacheLocation, index_manager: Arc<IndexManager>) -> Self {
        Self {
            location,
            index_manager,
        }
    }

    /// Get a cache entry
    ///
    /// This reads the cached data and updates access metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache entry doesn't exist or cannot be read.
    pub fn get(&self, key: &str, component: &str) -> Result<Vec<u8>> {
        let cache_path = Self::get_cache_file_path(&self.location, key, component);

        if !cache_path.exists() {
            anyhow::bail!("Cache entry not found");
        }

        // Update access metadata (only side effect)
        self.index_manager.update_access_metadata(key)?;

        fs::read(&cache_path)
            .with_context(|| format!("Failed to read cache file: {:?}", cache_path))
    }

    /// Check if a cache entry exists
    ///
    /// This is a pure query operation with no side effects.
    pub fn exists(&self, key: &str, component: &str) -> bool {
        let cache_path = Self::get_cache_file_path(&self.location, key, component);
        cache_path.exists()
    }

    /// Compute cache key including file hash
    ///
    /// This is a deterministic function that generates a unique key for a file
    /// based on its path and content hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn compute_cache_key(&self, file_path: &Path) -> Result<String> {
        Self::compute_cache_key_static(file_path)
    }

    /// Static version of compute_cache_key for pure computation
    fn compute_cache_key_static(file_path: &Path) -> Result<String> {
        if file_path.exists() && file_path.is_file() {
            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {:?}", file_path))?;
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            let hash = format!("{:x}", hasher.finalize());
            Ok(format!("{}:{}", file_path.display(), hash))
        } else {
            // For non-file paths, just use the path as key
            Ok(file_path.display().to_string())
        }
    }

    /// Get the file path for a cache entry
    ///
    /// Pure function that computes the cache file path based on the key and component.
    fn get_cache_file_path(location: &CacheLocation, key: &str, component: &str) -> PathBuf {
        let component_path = location.get_component_path(component);

        // Use first 2 chars of key for directory sharding
        let dir = if key.len() >= 2 {
            component_path.join(&key[..2])
        } else {
            component_path.join("_")
        };

        dir.join(format!("{}.cache", key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_cache_key_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/file.rs");
        let key = CacheReader::compute_cache_key_static(&path).unwrap();
        assert_eq!(key, path.display().to_string());
    }

    #[test]
    fn test_get_cache_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::resolve_with_strategy(
            None,
            crate::cache::cache_location::CacheStrategy::Custom(temp_dir.path().to_path_buf()),
        )
        .unwrap();

        let path = CacheReader::get_cache_file_path(&location, "abcdef123", "ast");
        assert!(path.to_string_lossy().contains("ab"));
        assert!(path.to_string_lossy().ends_with("abcdef123.cache"));
    }

    #[test]
    fn test_exists_returns_false_for_missing() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::resolve_with_strategy(
            None,
            crate::cache::cache_location::CacheStrategy::Custom(temp_dir.path().to_path_buf()),
        )
        .unwrap();
        let index = Arc::new(IndexManager::load_or_create(&location).unwrap());
        let reader = CacheReader::new(location, index);

        assert!(!reader.exists("nonexistent", "ast"));
    }

    #[test]
    fn test_get_nonexistent_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::resolve_with_strategy(
            None,
            crate::cache::cache_location::CacheStrategy::Custom(temp_dir.path().to_path_buf()),
        )
        .unwrap();
        let index = Arc::new(IndexManager::load_or_create(&location).unwrap());
        let reader = CacheReader::new(location, index);

        assert!(reader.get("nonexistent", "ast").is_err());
    }
}
