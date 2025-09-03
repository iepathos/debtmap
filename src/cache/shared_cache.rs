use crate::cache::cache_location::{CacheLocation, CacheStrategy};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Metadata for cache management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub version: String,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub access_count: u64,
    pub size_bytes: u64,
}

/// Index for tracking cache entries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheIndex {
    pub entries: HashMap<String, CacheMetadata>,
    pub total_size: u64,
    pub last_cleanup: Option<SystemTime>,
}

/// Thread-safe shared cache implementation
#[derive(Debug)]
pub struct SharedCache {
    pub location: CacheLocation,
    index: Arc<RwLock<CacheIndex>>,
    max_cache_size: u64,
    cleanup_threshold: f64,
}

impl SharedCache {
    /// Create a new shared cache instance
    pub fn new(repo_path: Option<&Path>) -> Result<Self> {
        let location = CacheLocation::resolve(repo_path)?;
        location.ensure_directories()?;

        let index = Self::load_or_create_index(&location)?;

        Ok(Self {
            location,
            index: Arc::new(RwLock::new(index)),
            max_cache_size: 1024 * 1024 * 1024, // 1GB default
            cleanup_threshold: 0.9,             // Cleanup when 90% full
        })
    }

    /// Load existing index or create new one
    fn load_or_create_index(location: &CacheLocation) -> Result<CacheIndex> {
        let index_path = location.get_component_path("metadata").join("index.json");

        if index_path.exists() {
            let content = fs::read_to_string(&index_path)
                .with_context(|| format!("Failed to read index from {:?}", index_path))?;

            serde_json::from_str(&content)
                .with_context(|| "Failed to deserialize cache index")
                .or_else(|_| {
                    // If deserialization fails, start with a new index
                    log::warn!("Cache index corrupted, creating new index");
                    Ok(CacheIndex::default())
                })
        } else {
            Ok(CacheIndex::default())
        }
    }

    /// Save the current index to disk
    pub fn save_index(&self) -> Result<()> {
        let index_path = self
            .location
            .get_component_path("metadata")
            .join("index.json");

        let index = self
            .index
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        let content =
            serde_json::to_string_pretty(&*index).context("Failed to serialize cache index")?;

        // Write atomically using temp file and rename
        let temp_path = index_path.with_extension("tmp");
        fs::write(&temp_path, content)
            .with_context(|| format!("Failed to write temp index to {:?}", temp_path))?;

        fs::rename(&temp_path, &index_path)
            .with_context(|| format!("Failed to rename index file: {:?}", index_path))?;

        Ok(())
    }

    /// Get a cache entry
    pub fn get(&self, key: &str, component: &str) -> Result<Vec<u8>> {
        let cache_path = self.get_cache_file_path(key, component);

        if !cache_path.exists() {
            anyhow::bail!("Cache entry not found");
        }

        // Update access metadata
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            if let Some(metadata) = index.entries.get_mut(key) {
                metadata.last_accessed = SystemTime::now();
                metadata.access_count += 1;
            }
        }

        fs::read(&cache_path)
            .with_context(|| format!("Failed to read cache file: {:?}", cache_path))
    }

    /// Store a cache entry
    pub fn put(&self, key: &str, component: &str, data: &[u8]) -> Result<()> {
        // Check if we need cleanup
        self.maybe_cleanup()?;

        let cache_path = self.get_cache_file_path(key, component);

        // Ensure parent directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cache directory: {:?}", parent))?;
        }

        // Write data atomically
        let temp_path = cache_path.with_extension("tmp");
        fs::write(&temp_path, data)
            .with_context(|| format!("Failed to write temp cache file: {:?}", temp_path))?;

        fs::rename(&temp_path, &cache_path)
            .with_context(|| format!("Failed to rename cache file: {:?}", cache_path))?;

        // Update index
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            let metadata = CacheMetadata {
                version: env!("CARGO_PKG_VERSION").to_string(),
                created_at: SystemTime::now(),
                last_accessed: SystemTime::now(),
                access_count: 1,
                size_bytes: data.len() as u64,
            };

            index.entries.insert(key.to_string(), metadata);
            index.total_size = index.entries.values().map(|m| m.size_bytes).sum();
        }

        self.save_index()?;
        Ok(())
    }

    /// Check if a cache entry exists
    pub fn exists(&self, key: &str, component: &str) -> bool {
        let cache_path = self.get_cache_file_path(key, component);
        cache_path.exists()
    }

    /// Delete a cache entry
    pub fn delete(&self, key: &str, component: &str) -> Result<()> {
        let cache_path = self.get_cache_file_path(key, component);

        if cache_path.exists() {
            fs::remove_file(&cache_path)
                .with_context(|| format!("Failed to delete cache file: {:?}", cache_path))?;
        }

        // Update index
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            index.entries.remove(key);
            index.total_size = index.entries.values().map(|m| m.size_bytes).sum();
        }

        self.save_index()?;
        Ok(())
    }

    /// Get the file path for a cache entry
    fn get_cache_file_path(&self, key: &str, component: &str) -> PathBuf {
        let component_path = self.location.get_component_path(component);

        // Use first 2 chars of key for directory sharding
        let shard = if key.len() >= 2 { &key[..2] } else { "00" };

        component_path.join(shard).join(format!("{}.cache", key))
    }

    /// Perform cleanup if cache is too large
    fn maybe_cleanup(&self) -> Result<()> {
        let should_cleanup = {
            let index = self
                .index
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

            index.total_size > (self.max_cache_size as f64 * self.cleanup_threshold) as u64
        };

        if should_cleanup {
            self.cleanup()?;
        }

        Ok(())
    }

    /// Clean up old cache entries
    pub fn cleanup(&self) -> Result<()> {
        let removed_keys = {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            // Sort entries by last access time
            let mut entries: Vec<_> = index
                .entries
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            entries.sort_by_key(|(_, metadata)| metadata.last_accessed);

            // Remove oldest entries until we're under 50% of max size
            let target_size = self.max_cache_size / 2;
            let mut removed_keys = Vec::new();
            let mut current_size = index.total_size;

            for (key, metadata) in entries {
                if current_size <= target_size {
                    break;
                }
                removed_keys.push(key);
                current_size -= metadata.size_bytes;
            }

            // Remove from index
            for key in &removed_keys {
                if let Some(metadata) = index.entries.remove(key) {
                    index.total_size -= metadata.size_bytes;
                }
            }

            index.last_cleanup = Some(SystemTime::now());

            removed_keys
        };

        // Delete files
        for key in removed_keys {
            // Try to delete from all components
            for component in &["call_graphs", "analysis", "metadata", "temp"] {
                let cache_path = self.get_cache_file_path(&key, component);
                if cache_path.exists() {
                    let _ = fs::remove_file(&cache_path);
                }
            }
        }

        self.save_index()?;
        Ok(())
    }

    /// Migrate cache from local to shared location
    pub fn migrate_from_local(&self, local_cache_path: &Path) -> Result<()> {
        if !local_cache_path.exists() {
            return Ok(()); // Nothing to migrate
        }

        log::info!(
            "Migrating cache from {:?} to {:?}",
            local_cache_path,
            self.location.get_cache_path()
        );

        // Copy all cache files
        for entry in fs::read_dir(local_cache_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    let dest = self.location.get_cache_path().join(file_name);
                    fs::copy(&path, &dest).with_context(|| {
                        format!("Failed to copy cache file from {:?} to {:?}", path, dest)
                    })?;
                }
            } else if path.is_dir() {
                // Recursively copy subdirectories
                if let Some(dir_name) = path.file_name() {
                    let dest_dir = self.location.get_cache_path().join(dir_name);
                    fs::create_dir_all(&dest_dir)?;
                    self.copy_dir_recursive(&path, &dest_dir)?;
                }
            }
        }

        log::info!("Cache migration completed successfully");
        Ok(())
    }

    /// Recursively copy directory contents
    fn copy_dir_recursive(&self, src: &Path, dest: &Path) -> Result<()> {
        #[allow(clippy::only_used_in_recursion)]
        let _ = self; // Silence clippy warning
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if path.is_file() {
                fs::copy(&path, &dest_path)?;
            } else if path.is_dir() {
                fs::create_dir_all(&dest_path)?;
                self.copy_dir_recursive(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    /// Get cache statistics
    /// Clear all cache entries for this project
    pub fn clear_project(&self) -> Result<()> {
        // Clear all files in all components
        for component in &[
            "call_graphs",
            "analysis",
            "metadata",
            "temp",
            "file_metrics",
        ] {
            let component_path = self.location.get_component_path(component);
            if component_path.exists() {
                // Remove all files in component directory and sharded subdirectories
                for entry in fs::read_dir(&component_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        fs::remove_file(&path)?;
                    } else if path.is_dir() {
                        // Remove files in sharded subdirectories
                        for subentry in fs::read_dir(&path)? {
                            let subentry = subentry?;
                            if subentry.path().is_file() {
                                fs::remove_file(subentry.path())?;
                            }
                        }
                        // Remove the now-empty shard directory
                        fs::remove_dir(&path).ok();
                    }
                }
            }
        }

        // Clear index
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
            index.entries.clear();
            index.total_size = 0;
            index.last_cleanup = Some(SystemTime::now());
        }

        self.save_index()?;
        Ok(())
    }

    pub fn get_stats(&self) -> CacheStats {
        let index = self.index.read().unwrap_or_else(|e| {
            log::warn!("Failed to acquire read lock for stats: {}", e);
            e.into_inner()
        });

        CacheStats {
            entry_count: index.entries.len(),
            total_size: index.total_size,
        }
    }

    pub fn get_full_stats(&self) -> Result<FullCacheStats> {
        let index = self
            .index
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(FullCacheStats {
            total_entries: index.entries.len(),
            total_size: index.total_size,
            cache_location: self.location.get_cache_path().to_path_buf(),
            strategy: self.location.strategy.clone(),
            project_id: self.location.project_id.clone(),
        })
    }
}

/// Basic cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_size: u64,
}

/// Full cache statistics for reporting
#[derive(Debug)]
pub struct FullCacheStats {
    pub total_entries: usize,
    pub total_size: u64,
    pub cache_location: PathBuf,
    pub strategy: CacheStrategy,
    pub project_id: String,
}

impl std::fmt::Display for FullCacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cache Statistics:")?;
        writeln!(f, "  Strategy: {:?}", self.strategy)?;
        writeln!(f, "  Location: {}", self.cache_location.display())?;
        writeln!(f, "  Project ID: {}", self.project_id)?;
        writeln!(f, "  Total entries: {}", self.total_entries)?;
        writeln!(f, "  Total size: {} MB", self.total_size / (1024 * 1024))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_shared_cache_operations() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());

        let cache = SharedCache::new(None).unwrap();

        // Test put and get
        let key = "test_key";
        let component = "test_component";
        let data = b"test data";

        cache.put(key, component, data).unwrap();
        assert!(cache.exists(key, component));

        let retrieved = cache.get(key, component).unwrap();
        assert_eq!(retrieved, data);

        // Test delete
        cache.delete(key, component).unwrap();
        assert!(!cache.exists(key, component));

        std::env::remove_var("DEBTMAP_CACHE_DIR");
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());

        let cache = SharedCache::new(None).unwrap();

        // Ensure directories are created
        cache.location.ensure_directories().unwrap();

        // Add some entries
        cache.put("key1", "component1", b"data1").unwrap();
        cache.put("key2", "component1", b"data2").unwrap();

        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.total_size, 10); // "data1" + "data2"

        std::env::remove_var("DEBTMAP_CACHE_DIR");
    }
}
