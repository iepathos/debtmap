//! Cache Writer Module
//!
//! Provides write operations with atomic guarantees for the shared cache system.
//!
//! ## Safety Guarantees
//!
//! - **Atomic Writes**: Uses filesystem-level atomic operations for crash-safe writes
//! - **Index Consistency**: Metadata always updated after successful writes
//! - **Batch Operations**: Efficient deletion for pruning scenarios
//!
//! ## Example
//!
//! ```rust,no_run
//! use debtmap::cache::shared_cache::writer::CacheWriter;
//! use debtmap::cache::cache_location::CacheLocation;
//! use debtmap::cache::index_manager::IndexManager;
//! use std::sync::Arc;
//!
//! # fn example() -> anyhow::Result<()> {
//! let location = CacheLocation::resolve(None)?;
//! let index_manager = Arc::new(IndexManager::load_or_create(&location)?);
//! let writer = CacheWriter::new(location, index_manager);
//! writer.put("my_key", "ast", &b"serialized_data".to_vec())?;
//! # Ok(())
//! # }
//! ```

use crate::cache::cache_location::CacheLocation;
use crate::cache::index_manager::{CacheMetadata, IndexManager};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// Write operations for cache management
pub struct CacheWriter {
    location: CacheLocation,
    index_manager: Arc<IndexManager>,
}

impl CacheWriter {
    /// Create a new CacheWriter
    pub fn new(location: CacheLocation, index_manager: Arc<IndexManager>) -> Self {
        Self {
            location,
            index_manager,
        }
    }

    /// Store a cache entry
    ///
    /// Writes data atomically to the cache and updates the index.
    ///
    /// # Errors
    ///
    /// Returns an error if the write operation or index update fails.
    pub fn put(&self, key: &str, component: &str, data: &[u8]) -> Result<()> {
        let cache_path = Self::get_cache_file_path(&self.location, key, component);

        // Write cache file atomically
        Self::write_cache_file_atomically(&cache_path, data)?;

        // Update index with new entry
        let metadata = Self::create_cache_metadata(data.len());
        self.index_manager.add_entry(key.to_string(), metadata)?;

        // Persist index changes
        self.index_manager.save(&self.location)?;

        Ok(())
    }

    /// Delete a cache entry
    ///
    /// Removes the cache file and updates the index.
    ///
    /// # Errors
    ///
    /// Returns an error if the file deletion or index update fails.
    pub fn delete(&self, key: &str, component: &str) -> Result<()> {
        let cache_path = Self::get_cache_file_path(&self.location, key, component);

        if cache_path.exists() {
            fs::remove_file(&cache_path)
                .with_context(|| format!("Failed to delete cache file: {:?}", cache_path))?;
        }

        // Update index
        self.index_manager.remove_entry(key)?;
        self.index_manager.save(&self.location)?;

        Ok(())
    }

    /// Delete multiple cache entries efficiently
    ///
    /// Used primarily for pruning operations.
    ///
    /// # Errors
    ///
    /// Returns the count of successfully deleted entries. Errors are logged but don't stop
    /// the batch operation.
    pub fn delete_batch(&self, entries: &[(String, String)]) -> Result<usize> {
        let mut deleted_count = 0;

        for (key, component) in entries {
            if self.delete(key, component).is_ok() {
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
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

    /// Create metadata for a new cache entry
    fn create_cache_metadata(data_len: usize) -> CacheMetadata {
        CacheMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            access_count: 1,
            size_bytes: data_len as u64,
            debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Write cache file atomically to disk
    fn write_cache_file_atomically(cache_path: &Path, data: &[u8]) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cache directory: {:?}", parent))?;
        }

        // Create temp path for atomic write
        let temp_path = Self::create_safe_temp_path(cache_path);

        // Ensure temp directory exists
        if let Some(temp_parent) = temp_path.parent() {
            fs::create_dir_all(temp_parent)
                .with_context(|| format!("Failed to create temp directory: {:?}", temp_parent))?;
        }

        // Write to temp file
        fs::write(&temp_path, data)
            .with_context(|| format!("Failed to write temp file: {:?}", temp_path))?;

        // Atomic rename
        fs::rename(&temp_path, cache_path).with_context(|| {
            format!(
                "Failed to rename temp file atomically: {:?} -> {:?}",
                temp_path, cache_path
            )
        })?;

        Ok(())
    }

    /// Create a safe temporary path for atomic writes
    fn create_safe_temp_path(target_path: &Path) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let temp_name = format!(
            "{}.tmp.{}",
            target_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            timestamp
        );

        if let Some(parent) = target_path.parent() {
            parent.join(temp_name)
        } else {
            PathBuf::from(temp_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_put_creates_file_atomically() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::resolve_with_strategy(
            None,
            crate::cache::cache_location::CacheStrategy::Custom(temp_dir.path().to_path_buf()),
        )
        .unwrap();
        let index = Arc::new(IndexManager::load_or_create(&location).unwrap());
        let writer = CacheWriter::new(location.clone(), index);

        let data = b"test data";
        writer.put("testkey", "ast", data).unwrap();

        let path = CacheWriter::get_cache_file_path(&location, "testkey", "ast");
        assert!(path.exists());

        let read_data = fs::read(&path).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_delete_removes_file() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::resolve_with_strategy(
            None,
            crate::cache::cache_location::CacheStrategy::Custom(temp_dir.path().to_path_buf()),
        )
        .unwrap();
        let index = Arc::new(IndexManager::load_or_create(&location).unwrap());
        let writer = CacheWriter::new(location.clone(), index);

        writer.put("testkey", "ast", b"data").unwrap();
        writer.delete("testkey", "ast").unwrap();

        let path = CacheWriter::get_cache_file_path(&location, "testkey", "ast");
        assert!(!path.exists());
    }

    #[test]
    fn test_delete_batch() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::resolve_with_strategy(
            None,
            crate::cache::cache_location::CacheStrategy::Custom(temp_dir.path().to_path_buf()),
        )
        .unwrap();
        let index = Arc::new(IndexManager::load_or_create(&location).unwrap());
        let writer = CacheWriter::new(location, index);

        writer.put("key1", "ast", b"data1").unwrap();
        writer.put("key2", "ast", b"data2").unwrap();
        writer.put("key3", "ast", b"data3").unwrap();

        let to_delete = vec![
            ("key1".to_string(), "ast".to_string()),
            ("key2".to_string(), "ast".to_string()),
        ];

        let deleted = writer.delete_batch(&to_delete).unwrap();
        assert_eq!(deleted, 2);
    }

    #[test]
    fn test_create_safe_temp_path() {
        let target = PathBuf::from("/cache/dir/file.cache");
        let temp_path = CacheWriter::create_safe_temp_path(&target);

        assert!(temp_path.to_string_lossy().contains("file.cache.tmp"));
        assert_eq!(temp_path.parent(), target.parent());
    }
}
