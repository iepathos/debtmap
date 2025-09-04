use crate::cache::auto_pruner::{AutoPruner, BackgroundPruner, PruneStats, PruneStrategy};
use crate::cache::cache_location::{CacheLocation, CacheStrategy};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

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

/// Configuration for pruning behavior
#[derive(Debug, Clone)]
struct PruningConfig {
    auto_prune_enabled: bool,
    use_sync_pruning: bool,
    is_test_environment: bool,
}

/// Internal cache statistics for pruning decisions
#[derive(Debug, Clone)]
struct InternalCacheStats {
    total_size: u64,
    entry_count: usize,
}

/// Thread-safe shared cache implementation
pub struct SharedCache {
    pub location: CacheLocation,
    index: Arc<RwLock<CacheIndex>>,
    max_cache_size: u64,
    cleanup_threshold: f64,
    auto_pruner: Option<AutoPruner>,
    background_pruner: Option<BackgroundPruner>,
}

impl std::fmt::Debug for SharedCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedCache")
            .field("location", &self.location)
            .field("max_cache_size", &self.max_cache_size)
            .field("cleanup_threshold", &self.cleanup_threshold)
            .field("auto_pruner", &self.auto_pruner)
            .field("has_background_pruner", &self.background_pruner.is_some())
            .finish()
    }
}

impl SharedCache {
    /// Create a new shared cache instance
    pub fn new(repo_path: Option<&Path>) -> Result<Self> {
        let location = CacheLocation::resolve(repo_path)?;
        Self::new_with_location(location)
    }

    /// Create a new shared cache instance with explicit cache directory (for testing)
    pub fn new_with_cache_dir(repo_path: Option<&Path>, cache_dir: PathBuf) -> Result<Self> {
        let strategy = CacheStrategy::Custom(cache_dir);
        let location = CacheLocation::resolve_with_strategy(repo_path, strategy)?;
        Self::new_with_location(location)
    }

    /// Create a new shared cache instance with explicit location
    fn new_with_location(location: CacheLocation) -> Result<Self> {
        location.ensure_directories()?;

        let index = Self::load_or_create_index(&location)?;

        // Create auto-pruner from environment or defaults
        let auto_pruner = if std::env::var("DEBTMAP_CACHE_AUTO_PRUNE")
            .unwrap_or_else(|_| "true".to_string())
            .to_lowercase()
            == "true"
        {
            Some(AutoPruner::from_env())
        } else {
            None
        };

        let background_pruner = auto_pruner
            .as_ref()
            .map(|p| BackgroundPruner::new(p.clone()));

        // Use auto-pruner's max size if configured
        let max_cache_size = auto_pruner
            .as_ref()
            .map(|p| p.max_size_bytes as u64)
            .unwrap_or(1024 * 1024 * 1024); // 1GB default

        Ok(Self {
            location,
            index: Arc::new(RwLock::new(index)),
            max_cache_size,
            cleanup_threshold: 0.9, // Cleanup when 90% full
            auto_pruner,
            background_pruner,
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
                    Ok(CacheIndex {
                        last_cleanup: Some(SystemTime::now()),
                        ..Default::default()
                    })
                })
        } else {
            Ok(CacheIndex {
                last_cleanup: Some(SystemTime::now()),
                ..Default::default()
            })
        }
    }

    /// Save the current index to disk
    pub fn save_index(&self) -> Result<()> {
        let index_path = self
            .location
            .get_component_path("metadata")
            .join("index.json");

        // Ensure parent directory exists
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create metadata directory: {:?}", parent))?;
        }

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

    // Pure functions for configuration and decision making
    
    /// Determine pruning configuration based on environment and test conditions
    fn determine_pruning_config() -> PruningConfig {
        let auto_prune_enabled = std::env::var("DEBTMAP_CACHE_AUTO_PRUNE")
            .unwrap_or_default() == "true";
        let sync_prune_requested = std::env::var("DEBTMAP_CACHE_SYNC_PRUNE")
            .unwrap_or_default() == "true";
        let is_test_environment = cfg!(test);
        
        let use_sync_pruning = auto_prune_enabled && (is_test_environment || sync_prune_requested);
        
        PruningConfig {
            auto_prune_enabled,
            use_sync_pruning,
            is_test_environment,
        }
    }
    
    /// Determine if an entry already exists in the index
    fn is_existing_entry(index: &CacheIndex, key: &str) -> bool {
        index.entries.contains_key(key)
    }
    
    /// Determine if pruning is needed after insertion
    fn should_prune_after_insertion(
        pruner: &AutoPruner,
        stats: &InternalCacheStats,
    ) -> bool {
        let size_exceeded = stats.total_size > pruner.max_size_bytes as u64;
        let count_exceeded = stats.entry_count > pruner.max_entries;
        size_exceeded || count_exceeded
    }
    
    /// Create metadata for a new cache entry
    fn create_cache_metadata(data_len: usize) -> CacheMetadata {
        CacheMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            access_count: 1,
            size_bytes: data_len as u64,
        }
    }
    
    /// Update index with new entry and recalculate total size
    fn update_index_with_entry(
        index: &mut CacheIndex,
        key: String,
        metadata: CacheMetadata,
    ) {
        index.entries.insert(key, metadata);
        index.total_size = index.entries.values().map(|m| m.size_bytes).sum();
    }
    
    /// Write cache file atomically to disk
    fn write_cache_file_atomically(cache_path: &Path, data: &[u8]) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cache directory: {:?}", parent))?;
        }

        // Write data atomically
        let temp_path = cache_path.with_extension("tmp");
        fs::write(&temp_path, data)
            .with_context(|| format!("Failed to write temp cache file: {:?}", temp_path))?;

        fs::rename(&temp_path, cache_path)
            .with_context(|| format!("Failed to rename cache file: {:?}", cache_path))?;
            
        Ok(())
    }
    
    /// Handle pre-insertion pruning based on configuration
    fn handle_pre_insertion_pruning(&self, key: &str, data_len: usize, config: &PruningConfig) -> Result<()> {
        if self.auto_pruner.is_some() {
            if config.use_sync_pruning {
                // Use synchronous pruning for tests and when explicitly requested
                let is_new_entry = {
                    let index = self.index.read()
                        .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
                    !Self::is_existing_entry(&index, key)
                };
                if is_new_entry {
                    self.trigger_pruning_if_needed_with_new_entry(data_len)?;
                } else {
                    self.trigger_pruning_if_needed()?;
                }
            } else if let Some(ref bg_pruner) = self.background_pruner {
                if !bg_pruner.is_running() {
                    bg_pruner.start_if_needed(self.index.clone());
                }
            } else {
                // Fallback to synchronous pruning
                let is_new_entry = {
                    let index = self.index.read()
                        .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
                    !Self::is_existing_entry(&index, key)
                };
                if is_new_entry {
                    self.trigger_pruning_if_needed_with_new_entry(data_len)?;
                } else {
                    self.trigger_pruning_if_needed()?;
                }
            }
        } else {
            self.maybe_cleanup()?;
        }
        Ok(())
    }
    
    /// Handle post-insertion pruning if needed
    fn handle_post_insertion_pruning(&self, config: &PruningConfig) -> Result<()> {
        // For synchronous operation (especially tests), check if we exceeded limits after adding entry
        if self.auto_pruner.is_some() && config.use_sync_pruning {
            let stats = self.get_stats();
            if let Some(ref pruner) = self.auto_pruner {
                let cache_stats = InternalCacheStats {
                    total_size: stats.total_size,
                    entry_count: stats.entry_count,
                };
                
                if Self::should_prune_after_insertion(pruner, &cache_stats) {
                    if config.is_test_environment {
                        println!("Post-insertion check: size={}/{}, count={}/{}", 
                                 stats.total_size, pruner.max_size_bytes, 
                                 stats.entry_count, pruner.max_entries);
                        println!("Triggering emergency pruning due to limit exceeded");
                    }
                    self.trigger_pruning()?;
                }
            }
        }
        Ok(())
    }
    
    /// Store a cache entry
    pub fn put(&self, key: &str, component: &str, data: &[u8]) -> Result<()> {
        let config = Self::determine_pruning_config();
        
        if config.is_test_environment {
            log::debug!("use_sync_pruning={}, auto_prune={}, cfg_test={}", 
                       config.use_sync_pruning, 
                       config.auto_prune_enabled, 
                       config.is_test_environment);
        }
        
        // Handle pre-insertion pruning/cleanup
        self.handle_pre_insertion_pruning(key, data.len(), &config)?;

        // Write cache file atomically
        let cache_path = self.get_cache_file_path(key, component);
        Self::write_cache_file_atomically(&cache_path, data)?;

        // Update index with new entry
        {
            let mut index = self.index.write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
            let metadata = Self::create_cache_metadata(data.len());
            Self::update_index_with_entry(&mut index, key.to_string(), metadata);
        }

        self.save_index()?;
        
        // Handle post-insertion pruning if needed
        self.handle_post_insertion_pruning(&config)?;
        
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

    /// Create a new shared cache with auto-pruning enabled
    pub fn with_auto_pruning(repo_path: Option<&Path>, pruner: AutoPruner) -> Result<Self> {
        let location = CacheLocation::resolve(repo_path)?;
        location.ensure_directories()?;
        let index = Self::load_or_create_index(&location)?;

        let background_pruner = BackgroundPruner::new(pruner.clone());

        Ok(Self {
            location,
            index: Arc::new(RwLock::new(index)),
            max_cache_size: pruner.max_size_bytes as u64,
            cleanup_threshold: 0.9,
            auto_pruner: Some(pruner),
            background_pruner: Some(background_pruner),
        })
    }

    /// Trigger pruning if needed based on auto-pruner configuration
    pub fn trigger_pruning_if_needed(&self) -> Result<PruneStats> {
        self.trigger_pruning_if_needed_with_new_entry(0)
    }

    /// Trigger pruning if needed, considering a new entry of the given size
    pub fn trigger_pruning_if_needed_with_new_entry(&self, new_entry_size: usize) -> Result<PruneStats> {
        // First, clean up any orphaned index entries for deleted files
        self.clean_orphaned_entries()?;

        if let Some(ref pruner) = self.auto_pruner {
            let should_prune = {
                let index = self
                    .index
                    .read()
                    .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
                
                // Check if we need pruning considering the new entry
                let projected_size = index.total_size + new_entry_size as u64;
                let projected_count = index.entries.len() + if new_entry_size > 0 { 1 } else { 0 };
                
                projected_size > pruner.max_size_bytes as u64 || 
                projected_count > pruner.max_entries ||
                pruner.should_prune(&index)
            };

            if should_prune {
                return self.trigger_pruning();
            }
        }

        Ok(PruneStats {
            entries_removed: 0,
            bytes_freed: 0,
            entries_remaining: self.get_stats().entry_count,
            bytes_remaining: self.get_stats().total_size,
            duration_ms: 0,
            files_deleted: 0,
            files_not_found: 0,
        })
    }

    /// Manually trigger pruning
    pub fn trigger_pruning(&self) -> Result<PruneStats> {
        let start = SystemTime::now();

        if let Some(ref pruner) = self.auto_pruner {
            let entries_to_remove = {
                let index = self
                    .index
                    .read()
                    .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
                pruner.calculate_entries_to_remove(&index)
            };

            if entries_to_remove.is_empty() {
                return Ok(PruneStats {
                    entries_removed: 0,
                    bytes_freed: 0,
                    entries_remaining: self.get_stats().entry_count,
                    bytes_remaining: self.get_stats().total_size,
                    duration_ms: 0,
                    files_deleted: 0,
                    files_not_found: 0,
                });
            }

            let mut bytes_freed = 0u64;
            let mut files_deleted = 0usize;
            let mut files_not_found = 0usize;

            // Remove from index
            {
                let mut index = self
                    .index
                    .write()
                    .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

                for (key, metadata) in &entries_to_remove {
                    if index.entries.remove(key).is_some() {
                        bytes_freed += metadata.size_bytes;
                    }
                }

                index.total_size = index.entries.values().map(|m| m.size_bytes).sum();
                index.last_cleanup = Some(SystemTime::now());
            }

            // Delete files and track missing ones
            for (key, _) in &entries_to_remove {
                let mut any_file_found = false;
                for component in &[
                    "call_graphs",
                    "analysis",
                    "metadata",
                    "temp",
                    "file_metrics",
                ] {
                    let cache_path = self.get_cache_file_path(key, component);
                    if cache_path.exists() {
                        any_file_found = true;
                        match fs::remove_file(&cache_path) {
                            Ok(_) => files_deleted += 1,
                            Err(e) => {
                                log::warn!("Failed to delete cache file {:?}: {}", cache_path, e);
                            }
                        }
                    }
                }
                if !any_file_found {
                    files_not_found += 1;
                    log::debug!("No files found for cache entry: {}", key);
                }
            }

            self.save_index()?;

            let duration = start.elapsed().unwrap_or(Duration::ZERO).as_millis() as u64;
            let final_stats = self.get_stats();

            Ok(PruneStats {
                entries_removed: entries_to_remove.len(),
                bytes_freed,
                entries_remaining: final_stats.entry_count,
                bytes_remaining: final_stats.total_size,
                duration_ms: duration,
                files_deleted,
                files_not_found,
            })
        } else {
            // Fallback to old cleanup method
            self.cleanup()?;
            let duration = start.elapsed().unwrap_or(Duration::ZERO).as_millis() as u64;
            let stats = self.get_stats();

            Ok(PruneStats {
                entries_removed: 0,
                bytes_freed: 0,
                entries_remaining: stats.entry_count,
                bytes_remaining: stats.total_size,
                duration_ms: duration,
                files_deleted: 0,
                files_not_found: 0,
            })
        }
    }

    /// Prune entries with a specific strategy
    pub fn prune_with_strategy(&self, strategy: PruneStrategy) -> Result<PruneStats> {
        // Create a temporary pruner with the specified strategy
        let temp_pruner = AutoPruner {
            strategy,
            ..self.auto_pruner.clone().unwrap_or_default()
        };

        let start = SystemTime::now();
        let entries_to_remove = {
            let index = self
                .index
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
            temp_pruner.calculate_entries_to_remove(&index)
        };

        if entries_to_remove.is_empty() {
            return Ok(PruneStats {
                entries_removed: 0,
                bytes_freed: 0,
                entries_remaining: self.get_stats().entry_count,
                bytes_remaining: self.get_stats().total_size,
                duration_ms: 0,
                files_deleted: 0,
                files_not_found: 0,
            });
        }

        let mut bytes_freed = 0u64;
        let mut files_deleted = 0usize;

        // Remove from index
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            for (key, metadata) in &entries_to_remove {
                if index.entries.remove(key).is_some() {
                    bytes_freed += metadata.size_bytes;
                }
            }

            index.total_size = index.entries.values().map(|m| m.size_bytes).sum();
            index.last_cleanup = Some(SystemTime::now());
        }

        // Delete files
        for (key, _) in &entries_to_remove {
            for component in &[
                "call_graphs",
                "analysis",
                "metadata",
                "temp",
                "file_metrics",
            ] {
                let cache_path = self.get_cache_file_path(key, component);
                if cache_path.exists() && fs::remove_file(&cache_path).is_ok() {
                    files_deleted += 1;
                }
            }
        }

        self.save_index()?;

        let duration = start.elapsed().unwrap_or(Duration::ZERO).as_millis() as u64;
        let final_stats = self.get_stats();

        Ok(PruneStats {
            entries_removed: entries_to_remove.len(),
            bytes_freed,
            entries_remaining: final_stats.entry_count,
            bytes_remaining: final_stats.total_size,
            duration_ms: duration,
            files_deleted,
            files_not_found: 0,
        })
    }

    /// Clean up orphaned index entries where files no longer exist
    pub fn clean_orphaned_entries(&self) -> Result<usize> {
        let mut removed_count = 0;
        let entries_to_check = {
            let index = self
                .index
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
            index.entries.keys().cloned().collect::<Vec<_>>()
        };

        let mut orphaned_entries = Vec::new();

        // Check each entry to see if any of its files exist
        for key in entries_to_check {
            let mut file_exists = false;
            for component in &[
                "call_graphs",
                "analysis",
                "metadata",
                "temp",
                "file_metrics",
            ] {
                let cache_path = self.get_cache_file_path(&key, component);
                if cache_path.exists() {
                    file_exists = true;
                    break;
                }
            }

            if !file_exists {
                orphaned_entries.push(key);
            }
        }

        // Remove orphaned entries from index
        if !orphaned_entries.is_empty() {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            for key in orphaned_entries {
                if let Some(metadata) = index.entries.remove(&key) {
                    index.total_size = index.total_size.saturating_sub(metadata.size_bytes);
                    removed_count += 1;
                    log::debug!("Removed orphaned cache entry: {}", key);
                }
            }
        }

        if removed_count > 0 {
            self.save_index()?;
            log::info!("Cleaned up {} orphaned cache entries", removed_count);
        }

        Ok(removed_count)
    }

    /// Clean up entries older than specified days
    pub fn cleanup_old_entries(&self, max_age_days: i64) -> Result<usize> {
        let max_age = Duration::from_secs(max_age_days as u64 * 86400);
        let now = SystemTime::now();
        let mut removed_count = 0;

        let entries_to_remove = {
            let index = self
                .index
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

            let mut to_remove = Vec::new();
            for (key, metadata) in &index.entries {
                if let Ok(age) = now.duration_since(metadata.last_accessed) {
                    if age > max_age {
                        to_remove.push(key.clone());
                    }
                }
            }
            to_remove
        };

        // Remove from index and delete files
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            for key in &entries_to_remove {
                if let Some(metadata) = index.entries.remove(key) {
                    index.total_size -= metadata.size_bytes;
                    removed_count += 1;

                    // Delete files
                    for component in &[
                        "call_graphs",
                        "analysis",
                        "metadata",
                        "temp",
                        "file_metrics",
                    ] {
                        let cache_path = self.get_cache_file_path(key, component);
                        if cache_path.exists() {
                            let _ = fs::remove_file(&cache_path);
                        }
                    }
                }
            }

            index.last_cleanup = Some(SystemTime::now());
        }

        self.save_index()?;
        Ok(removed_count)
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
