//! Cache index management with functional patterns
//!
//! This module provides pure, functional operations for managing the cache index.
//! I/O operations are isolated using the atomic_io module, while business logic
//! remains pure and testable.

use crate::cache::atomic_io::AtomicFileWriter;
use crate::cache::cache_location::CacheLocation;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Default function for debtmap_version field when deserializing old cache entries
fn default_debtmap_version() -> String {
    String::new()
}

/// Metadata for cache management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub version: String,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub access_count: u64,
    pub size_bytes: u64,
    #[serde(default = "default_debtmap_version")]
    pub debtmap_version: String,
}

impl CacheMetadata {
    pub fn new(size_bytes: u64) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            access_count: 1,
            size_bytes,
            debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Index for tracking cache entries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheIndex {
    pub entries: HashMap<String, CacheMetadata>,
    pub total_size: u64,
    pub last_cleanup: Option<SystemTime>,
}

/// Statistics about the cache index
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub entry_count: usize,
    pub total_size: u64,
}

/// Manages cache index operations with functional patterns
pub struct IndexManager {
    index: Arc<RwLock<CacheIndex>>,
}

impl IndexManager {
    pub fn new(index: CacheIndex) -> Self {
        Self {
            index: Arc::new(RwLock::new(index)),
        }
    }

    pub fn load_or_create(location: &CacheLocation) -> Result<Self> {
        let index = Self::load_index_from_location(location)?;
        Ok(Self::new(index))
    }

    // Pure functions

    fn calculate_stats(index: &CacheIndex) -> IndexStats {
        IndexStats {
            entry_count: index.entries.len(),
            total_size: index.total_size,
        }
    }

    fn recalculate_total_size(entries: &HashMap<String, CacheMetadata>) -> u64 {
        entries.values().map(|m| m.size_bytes).sum()
    }

    fn add_entry_to_index(
        mut index: CacheIndex,
        key: String,
        metadata: CacheMetadata,
    ) -> CacheIndex {
        index.entries.insert(key, metadata);
        index.total_size = Self::recalculate_total_size(&index.entries);
        index
    }

    fn remove_entries_from_index(mut index: CacheIndex, keys: &[String]) -> (CacheIndex, u64) {
        let mut removed_count = 0;

        for key in keys {
            if index.entries.remove(key).is_some() {
                removed_count += 1;
            }
        }

        index.total_size = Self::recalculate_total_size(&index.entries);
        index.last_cleanup = Some(SystemTime::now());

        (index, removed_count)
    }

    fn entry_exists(index: &CacheIndex, key: &str) -> bool {
        index.entries.contains_key(key)
    }

    // I/O operations

    fn resolve_index_path(location: &CacheLocation) -> PathBuf {
        location.get_component_path("metadata").join("index.json")
    }

    fn load_index_from_location(location: &CacheLocation) -> Result<CacheIndex> {
        let index_path = Self::resolve_index_path(location);

        if index_path.exists() {
            Self::read_index_from_file(&index_path).or_else(|_| {
                log::warn!("Cache index corrupted, creating new index");
                Ok(Self::create_new_index())
            })
        } else {
            Ok(Self::create_new_index())
        }
    }

    fn read_index_from_file(path: &Path) -> Result<CacheIndex> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read index from {:?}", path))?;

        serde_json::from_str(&content).with_context(|| "Failed to deserialize cache index")
    }

    fn create_new_index() -> CacheIndex {
        CacheIndex {
            last_cleanup: Some(SystemTime::now()),
            ..Default::default()
        }
    }

    // Public API

    pub fn save(&self, location: &CacheLocation) -> Result<()> {
        let index_path = Self::resolve_index_path(location);
        let temp_path = AtomicFileWriter::create_safe_temp_path(&index_path);

        AtomicFileWriter::ensure_atomic_write_directories(&index_path, &temp_path)?;

        let index = self
            .index
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        let content = AtomicFileWriter::serialize_index_to_json(&*index)?;

        AtomicFileWriter::write_file_atomically(&index_path, &temp_path, &content)
            .with_context(|| format!("Failed to save cache index to {:?}", index_path))
    }

    pub fn add_entry(&self, key: String, metadata: CacheMetadata) -> Result<()> {
        let mut index = self
            .index
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        *index = Self::add_entry_to_index(index.clone(), key, metadata);
        Ok(())
    }

    pub fn remove_entries(&self, keys: &[String]) -> Result<u64> {
        let mut index = self
            .index
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        let (new_index, removed_count) = Self::remove_entries_from_index(index.clone(), keys);
        *index = new_index;

        Ok(removed_count)
    }

    pub fn get_stats(&self) -> IndexStats {
        let index = self.index.read().unwrap_or_else(|e| {
            log::warn!("Failed to acquire read lock for stats: {}", e);
            e.into_inner()
        });

        Self::calculate_stats(&index)
    }

    pub fn is_existing_entry(&self, key: &str) -> bool {
        let index = self.index.read().unwrap_or_else(|e| e.into_inner());
        Self::entry_exists(&index, key)
    }

    pub fn get_index_arc(&self) -> Arc<RwLock<CacheIndex>> {
        Arc::clone(&self.index)
    }

    pub fn update_access_metadata(&self, key: &str) -> Result<()> {
        let mut index = self
            .index
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        if let Some(metadata) = index.entries.get_mut(key) {
            metadata.last_accessed = SystemTime::now();
            metadata.access_count += 1;
        }

        Ok(())
    }

    pub fn remove_entry(&self, key: &str) -> Result<()> {
        let mut index = self
            .index
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        index.entries.remove(key);
        index.total_size = Self::recalculate_total_size(&index.entries);

        Ok(())
    }

    pub fn check_total_size_exceeds(&self, threshold: u64) -> bool {
        let index = self.index.read().unwrap_or_else(|e| e.into_inner());
        index.total_size > threshold
    }

    pub fn get_all_entry_keys(&self) -> Vec<String> {
        let index = self.index.read().unwrap_or_else(|e| e.into_inner());
        index.entries.keys().cloned().collect()
    }

    pub fn clear_all_entries(&self) -> Result<()> {
        let mut index = self
            .index
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        index.entries.clear();
        index.total_size = 0;
        index.last_cleanup = Some(SystemTime::now());

        Ok(())
    }

    pub fn get_full_stats(&self) -> (usize, u64) {
        let index = self.index.read().unwrap_or_else(|e| e.into_inner());
        (index.entries.len(), index.total_size)
    }

    pub fn check_version_mismatch(&self, current_version: &str) -> bool {
        let index = self.index.read().unwrap_or_else(|e| e.into_inner());

        if index.entries.is_empty() {
            false
        } else {
            index.entries.values().any(|metadata| {
                !metadata.debtmap_version.is_empty() && metadata.debtmap_version != current_version
            })
        }
    }

    pub fn get_sorted_entries_and_stats(&self) -> (Vec<(String, CacheMetadata)>, u64) {
        let index = self.index.read().unwrap_or_else(|e| e.into_inner());

        let mut sorted: Vec<_> = index
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        sorted.sort_by_key(|(_, metadata)| metadata.last_accessed);

        (sorted, index.total_size)
    }

    pub fn get_entries_snapshot(&self) -> HashMap<String, CacheMetadata> {
        let index = self.index.read().unwrap_or_else(|e| e.into_inner());
        index.entries.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_metadata(size: u64) -> CacheMetadata {
        CacheMetadata {
            version: "1.0.0".to_string(),
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            access_count: 1,
            size_bytes: size,
            debtmap_version: "0.1.0".to_string(),
        }
    }

    #[test]
    fn test_calculate_stats_empty() {
        let index = CacheIndex::default();
        let stats = IndexManager::calculate_stats(&index);

        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[test]
    fn test_recalculate_total_size() {
        let mut entries = HashMap::new();
        entries.insert("key1".to_string(), create_test_metadata(100));
        entries.insert("key2".to_string(), create_test_metadata(200));

        let total = IndexManager::recalculate_total_size(&entries);
        assert_eq!(total, 300);
    }

    #[test]
    fn test_add_entry_to_index() {
        let index = CacheIndex::default();
        let metadata = create_test_metadata(100);

        let updated = IndexManager::add_entry_to_index(index, "test".to_string(), metadata);

        assert_eq!(updated.entries.len(), 1);
        assert_eq!(updated.total_size, 100);
    }

    #[test]
    fn test_remove_entries_from_index() {
        let mut entries = HashMap::new();
        entries.insert("key1".to_string(), create_test_metadata(100));
        entries.insert("key2".to_string(), create_test_metadata(200));

        let index = CacheIndex {
            entries,
            total_size: 300,
            last_cleanup: None,
        };

        let (updated, removed) =
            IndexManager::remove_entries_from_index(index, &["key1".to_string()]);

        assert_eq!(removed, 1);
        assert_eq!(updated.total_size, 200);
    }

    #[test]
    fn test_index_manager_add_entry() {
        let manager = IndexManager::new(CacheIndex::default());
        let result = manager.add_entry("test".to_string(), create_test_metadata(100));

        assert!(result.is_ok());
        assert_eq!(manager.get_stats().total_size, 100);
    }

    #[test]
    fn test_save_and_load_index() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path());

        let location = CacheLocation::resolve(None).unwrap();
        location.ensure_directories().unwrap();

        let manager = IndexManager::new(CacheIndex::default());
        manager
            .add_entry("key1".to_string(), create_test_metadata(100))
            .unwrap();

        manager.save(&location).unwrap();

        let loaded = IndexManager::load_or_create(&location).unwrap();
        assert_eq!(loaded.get_stats().entry_count, 1);
    }
}
