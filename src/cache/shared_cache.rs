use crate::cache::auto_pruner::{AutoPruner, BackgroundPruner, PruneStats, PruneStrategy};
use crate::cache::cache_location::{CacheLocation, CacheStrategy};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

    // Pure functions for path operations and atomic file handling

    /// Resolve the index file path based on cache location
    fn resolve_index_paths(location: &CacheLocation) -> (PathBuf, PathBuf) {
        let index_path = location.get_component_path("metadata").join("index.json");
        let temp_path = Self::create_safe_temp_path(&index_path);
        (index_path, temp_path)
    }

    /// Create a safe temporary file path that avoids collisions
    fn create_safe_temp_path(target_path: &Path) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let process_id = std::process::id();

        // Create a unique temp filename to avoid collisions
        let temp_name = format!(
            "{}.tmp.{}.{}.{}",
            target_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file"),
            process_id,
            timestamp,
            counter
        );

        target_path.with_file_name(temp_name)
    }

    /// Validate that a file path is safe for atomic operations
    fn validate_file_path(path: &Path) -> Result<()> {
        // Ensure the path is absolute to avoid ambiguity
        if !path.is_absolute() {
            anyhow::bail!(
                "File path must be absolute for safe atomic operations: {:?}",
                path
            );
        }

        // Check for path traversal attempts
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            anyhow::bail!(
                "File path contains unsafe parent directory references: {:?}",
                path
            );
        }

        Ok(())
    }

    /// Ensure the parent directory exists for the given path with race condition handling
    fn ensure_parent_directory(file_path: &Path) -> Result<()> {
        Self::validate_file_path(file_path)?;

        if let Some(parent) = file_path.parent() {
            Self::create_directories_safely(parent)
                .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
        }
        Ok(())
    }

    /// Execute a fallible operation with exponential backoff retry
    fn retry_with_backoff<T, F>(mut operation: F, operation_name: &str) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        use std::io::ErrorKind;
        use std::time::Duration;

        const MAX_ATTEMPTS: usize = 3;
        const BASE_DELAY_MS: u64 = 10;

        for attempt in 0..MAX_ATTEMPTS {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // Check if this is a potentially transient error
                    let is_retryable = e.chain().any(|err| {
                        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
                            matches!(
                                io_err.kind(),
                                ErrorKind::AlreadyExists |
                                ErrorKind::NotFound |
                                ErrorKind::Interrupted |
                                // Note: PermissionDenied is often not transient, but in concurrent
                                // scenarios it might be due to temporary file locks
                                ErrorKind::PermissionDenied
                            )
                        } else {
                            false
                        }
                    });

                    if !is_retryable || attempt == MAX_ATTEMPTS - 1 {
                        return Err(e.context(format!(
                            "Operation '{}' failed after {} attempts",
                            operation_name,
                            attempt + 1
                        )));
                    }

                    // Exponential backoff with simple jitter
                    let delay_ms = BASE_DELAY_MS * (1 << attempt);
                    // Simple deterministic jitter to avoid contention
                    let jitter_ms = delay_ms / 4; // 25% jitter
                    std::thread::sleep(Duration::from_millis(delay_ms + jitter_ms));
                }
            }
        }

        unreachable!("Retry loop should have returned or failed by now")
    }

    /// Create directories safely with proper race condition handling and retries
    fn create_directories_safely(dir_path: &Path) -> Result<()> {
        use std::io::ErrorKind;

        // Fast path: directory already exists
        if dir_path.exists() {
            return Ok(());
        }

        let dir_path_clone = dir_path.to_path_buf();
        Self::retry_with_backoff(
            || {
                match fs::create_dir_all(&dir_path_clone) {
                    Ok(()) => Ok(()),
                    Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                        // Another thread created it between our check and creation attempt
                        Ok(())
                    }
                    Err(e) => Err(anyhow::Error::from(e)),
                }
            },
            &format!("create directories {:?}", dir_path),
        )
        .with_context(|| {
            format!(
                "Failed to create directory {:?}. Current working directory: {:?}",
                dir_path,
                std::env::current_dir().unwrap_or_else(|_| "<unknown>".into())
            )
        })
    }

    /// Ensure both temp and target file paths have their parent directories created
    fn ensure_atomic_write_directories(target_path: &Path, temp_path: &Path) -> Result<()> {
        // Ensure target directory exists
        Self::ensure_parent_directory(target_path)?;

        // Ensure temp directory exists (might be different from target)
        if temp_path.parent() != target_path.parent() {
            Self::ensure_parent_directory(temp_path)?;
        }

        Ok(())
    }

    /// Serialize cache index to JSON string
    fn serialize_index_to_json(index: &CacheIndex) -> Result<String> {
        serde_json::to_string_pretty(index).context("Failed to serialize cache index")
    }

    /// Write content atomically using temporary file and rename
    fn write_file_atomically(target_path: &Path, temp_path: &Path, content: &str) -> Result<()> {
        Self::write_bytes_atomically(target_path, temp_path, content.as_bytes())
    }

    /// Write data to a temporary file with proper error context and retries
    fn write_temp_file(temp_path: &Path, data: &[u8]) -> Result<()> {
        let temp_path_clone = temp_path.to_path_buf();
        let data_len = data.len();

        Self::retry_with_backoff(
            || {
                fs::write(&temp_path_clone, data)
                    .map_err(anyhow::Error::from)
            },
            &format!("write temp file {:?}", temp_path),
        ).with_context(|| {
            format!(
                "Failed to write temporary file at {:?}. Size: {} bytes. Parent exists: {}, Temp path valid: {}",
                temp_path,
                data_len,
                temp_path.parent().map_or(false, |p| p.exists()),
                temp_path.is_absolute()
            )
        })
    }

    /// Flush and sync file data to ensure durability (optional but recommended)
    fn sync_temp_file(_temp_path: &Path) -> Result<()> {
        // Note: We could open and sync the temp file here for extra durability,
        // but for cache files, the performance cost may not be worth it.
        // This function is a placeholder for future enhancement if needed.
        Ok(())
    }

    /// Atomically rename temporary file to target with retries and detailed error context
    fn atomic_rename(temp_path: &Path, target_path: &Path) -> Result<()> {
        let temp_path_clone = temp_path.to_path_buf();
        let target_path_clone = target_path.to_path_buf();

        Self::retry_with_backoff(
            || {
                fs::rename(&temp_path_clone, &target_path_clone)
                    .map_err(anyhow::Error::from)
            },
            &format!("atomic rename {:?} -> {:?}", temp_path, target_path),
        ).with_context(|| {
            format!(
                "Failed to rename file atomically: {:?} -> {:?}. Temp exists: {}, Target parent exists: {}, Same filesystem: {}",
                temp_path,
                target_path,
                temp_path.exists(),
                target_path.parent().map_or(false, |p| p.exists()),
                Self::paths_on_same_filesystem(temp_path, target_path)
            )
        })
    }

    /// Check if two paths are likely on the same filesystem (heuristic)
    fn paths_on_same_filesystem(path1: &Path, path2: &Path) -> bool {
        // Simple heuristic: if both paths have the same root, assume same filesystem
        // This isn't perfect but gives us debugging info
        path1.ancestors().last() == path2.ancestors().last()
    }

    /// Write bytes atomically using temporary file and rename - composed from pure functions
    fn write_bytes_atomically(target_path: &Path, temp_path: &Path, data: &[u8]) -> Result<()> {
        // Step 1: Write data to temporary file
        Self::write_temp_file(temp_path, data)?;

        // Step 2: Optional sync for durability
        Self::sync_temp_file(temp_path)?;

        // Step 3: Atomic rename
        Self::atomic_rename(temp_path, target_path)?;

        Ok(())
    }

    /// Save the current index to disk with comprehensive error handling
    pub fn save_index(&self) -> Result<()> {
        let (index_path, temp_path) = Self::resolve_index_paths(&self.location);

        // Ensure directories exist before any file operations
        Self::ensure_atomic_write_directories(&index_path, &temp_path)?;

        let index = self
            .index
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        let content = Self::serialize_index_to_json(&*index)?;

        Self::write_file_atomically(&index_path, &temp_path, &content)
            .with_context(|| format!("Failed to save cache index to {:?}", index_path))?;

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
        let auto_prune_enabled =
            std::env::var("DEBTMAP_CACHE_AUTO_PRUNE").unwrap_or_default() == "true";
        let sync_prune_requested =
            std::env::var("DEBTMAP_CACHE_SYNC_PRUNE").unwrap_or_default() == "true";
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
    fn should_prune_after_insertion(pruner: &AutoPruner, stats: &InternalCacheStats) -> bool {
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
    fn update_index_with_entry(index: &mut CacheIndex, key: String, metadata: CacheMetadata) {
        index.entries.insert(key, metadata);
        index.total_size = index.entries.values().map(|m| m.size_bytes).sum();
    }

    /// Write cache file atomically to disk
    fn write_cache_file_atomically(cache_path: &Path, data: &[u8]) -> Result<()> {
        let temp_path = Self::create_safe_temp_path(cache_path);
        Self::ensure_atomic_write_directories(cache_path, &temp_path)?;
        Self::write_bytes_atomically(cache_path, &temp_path, data)
    }

    /// Handle pre-insertion pruning based on configuration
    fn handle_pre_insertion_pruning(
        &self,
        key: &str,
        data_len: usize,
        config: &PruningConfig,
    ) -> Result<()> {
        if self.auto_pruner.is_some() {
            if config.use_sync_pruning {
                // Use synchronous pruning for tests and when explicitly requested
                let is_new_entry = {
                    let index = self
                        .index
                        .read()
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
                    let index = self
                        .index
                        .read()
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
                    // Always print debug in test mode
                    if cfg!(test) {
                        println!(
                            "Post-insertion check: size={}/{}, count={}/{}",
                            stats.total_size,
                            pruner.max_size_bytes,
                            stats.entry_count,
                            pruner.max_entries
                        );
                        println!("Triggering post-insertion pruning due to limit exceeded");
                    }
                    self.trigger_pruning()?;
                }
            }
        }
        Ok(())
    }

    /// Store a cache entry with explicit pruning configuration  
    fn put_with_config(
        &self,
        key: &str,
        component: &str,
        data: &[u8],
        config: &PruningConfig,
    ) -> Result<()> {
        if config.is_test_environment {
            log::debug!(
                "use_sync_pruning={}, auto_prune={}, cfg_test={}",
                config.use_sync_pruning,
                config.auto_prune_enabled,
                config.is_test_environment
            );
        }

        // Handle pre-insertion pruning/cleanup
        self.handle_pre_insertion_pruning(key, data.len(), config)?;

        // Write cache file atomically
        let cache_path = self.get_cache_file_path(key, component);
        Self::write_cache_file_atomically(&cache_path, data)?;

        // Update index with new entry
        {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
            let metadata = Self::create_cache_metadata(data.len());
            Self::update_index_with_entry(&mut index, key.to_string(), metadata);
        }

        self.save_index()?;

        // Handle post-insertion pruning if needed
        self.handle_post_insertion_pruning(config)?;

        Ok(())
    }

    /// Store a cache entry
    pub fn put(&self, key: &str, component: &str, data: &[u8]) -> Result<()> {
        let config = Self::determine_pruning_config();
        self.put_with_config(key, component, data, &config)
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

        // Delete files with better error handling
        for key in removed_keys {
            // Try to delete from all components
            for component in &[
                "call_graphs",
                "analysis",
                "metadata",
                "temp",
                "file_metrics",
                "test",
            ] {
                let cache_path = self.get_cache_file_path(&key, component);
                if cache_path.exists() {
                    if let Err(e) = fs::remove_file(&cache_path) {
                        log::debug!("Failed to delete cache file {:?}: {}. This may be due to concurrent access.", cache_path, e);
                    }
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

    // Pure function to calculate projected cache state after adding a new entry
    fn calculate_cache_projections(
        current_size: u64,
        current_count: usize,
        new_entry_size: usize,
    ) -> (u64, usize) {
        let projected_size = current_size + new_entry_size as u64;
        let projected_count = current_count + if new_entry_size > 0 { 1 } else { 0 };
        (projected_size, projected_count)
    }

    // Pure function to determine if pruning is needed based on projections
    fn should_prune_based_on_projections(
        projected_size: u64,
        projected_count: usize,
        max_size_bytes: usize,
        max_entries: usize,
    ) -> bool {
        projected_size > max_size_bytes as u64 || projected_count > max_entries
    }

    // Pure function to determine pruning decision given all inputs
    fn calculate_pruning_decision(
        current_size: u64,
        current_count: usize,
        new_entry_size: usize,
        max_size_bytes: usize,
        max_entries: usize,
        additional_check: bool,
    ) -> bool {
        let (projected_size, projected_count) =
            Self::calculate_cache_projections(current_size, current_count, new_entry_size);

        Self::should_prune_based_on_projections(
            projected_size,
            projected_count,
            max_size_bytes,
            max_entries,
        ) || additional_check
    }

    // Pure function to create empty prune stats with current cache state
    fn create_no_prune_stats(entry_count: usize, total_size: u64) -> PruneStats {
        PruneStats {
            entries_removed: 0,
            bytes_freed: 0,
            entries_remaining: entry_count,
            bytes_remaining: total_size,
            duration_ms: 0,
            files_deleted: 0,
            files_not_found: 0,
        }
    }

    // Pure functions for age-based cleanup

    /// Calculate the maximum age duration from days
    fn calculate_max_age_duration(max_age_days: i64) -> Duration {
        Duration::from_secs(max_age_days as u64 * 86400)
    }

    /// Determine if an entry should be removed based on age
    fn should_remove_entry_by_age(
        now: SystemTime,
        last_accessed: SystemTime,
        max_age: Duration,
    ) -> bool {
        now.duration_since(last_accessed)
            .map(|age| age >= max_age) // Use >= to handle zero-age case
            .unwrap_or(false) // If time calculation fails, don't remove
    }

    /// Filter entries to find those that should be removed based on age
    fn filter_entries_by_age(
        entries: &HashMap<String, CacheMetadata>,
        now: SystemTime,
        max_age: Duration,
    ) -> Vec<String> {
        entries
            .iter()
            .filter_map(|(key, metadata)| {
                if Self::should_remove_entry_by_age(now, metadata.last_accessed, max_age) {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Update index by removing entries and recalculating total size
    fn update_index_after_removal(index: &mut CacheIndex, entries_to_remove: &[String]) -> usize {
        let mut removed_count = 0;

        for key in entries_to_remove {
            if let Some(metadata) = index.entries.remove(key) {
                index.total_size -= metadata.size_bytes;
                removed_count += 1;
            }
        }

        index.last_cleanup = Some(SystemTime::now());
        removed_count
    }

    /// Delete cache files for the given keys and components
    fn delete_cache_files_for_keys(
        cache: &SharedCache,
        keys: &[String],
    ) -> std::result::Result<(), ()> {
        let components = [
            "call_graphs",
            "analysis",
            "metadata",
            "temp",
            "file_metrics",
            "test",
        ];

        for key in keys {
            for component in &components {
                let cache_path = cache.get_cache_file_path(key, component);
                if cache_path.exists() {
                    let _ = fs::remove_file(&cache_path); // Ignore errors
                }
            }
        }
        Ok(())
    }

    /// Trigger pruning if needed based on auto-pruner configuration
    pub fn trigger_pruning_if_needed(&self) -> Result<PruneStats> {
        self.trigger_pruning_if_needed_with_new_entry(0)
    }

    /// Trigger pruning if needed, considering a new entry of the given size
    pub fn trigger_pruning_if_needed_with_new_entry(
        &self,
        new_entry_size: usize,
    ) -> Result<PruneStats> {
        // Step 1: Clean up orphaned entries (side effect)
        self.clean_orphaned_entries()?;

        // Step 2: Check if pruning is needed (pure decision logic)
        let should_prune = match &self.auto_pruner {
            Some(pruner) => {
                let index = self
                    .index
                    .read()
                    .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

                Self::calculate_pruning_decision(
                    index.total_size,
                    index.entries.len(),
                    new_entry_size,
                    pruner.max_size_bytes,
                    pruner.max_entries,
                    pruner.should_prune(&index),
                )
            }
            None => false,
        };

        // Step 3: Take action based on decision
        if should_prune {
            self.trigger_pruning()
        } else {
            let stats = self.get_stats();
            Ok(Self::create_no_prune_stats(
                stats.entry_count,
                stats.total_size,
            ))
        }
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
                    "test",
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
                "test",
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
                "test", // Added for test compatibility
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
        // Step 1: Calculate timing parameters (pure)
        let max_age = Self::calculate_max_age_duration(max_age_days);
        let now = SystemTime::now();

        // Step 2: Identify entries to remove (pure logic)
        let entries_to_remove = {
            let index = self
                .index
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

            Self::filter_entries_by_age(&index.entries, now, max_age)
        };

        // Step 3: Update index (side effect - but isolated)
        let removed_count = {
            let mut index = self
                .index
                .write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            Self::update_index_after_removal(&mut index, &entries_to_remove)
        };

        // Step 4: Delete files (side effect - but isolated)
        Self::delete_cache_files_for_keys(self, &entries_to_remove)
            .map_err(|_| anyhow::anyhow!("Failed to delete cache files"))?;

        // Step 5: Persist changes (side effect - but isolated)
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
    use crate::cache::EnvironmentSnapshot;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_shared_cache_operations() {
        let temp_dir = TempDir::new().unwrap();

        // Set environment variables directly
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

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

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();

        // Set environment variables directly
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        // Ensure directories are created
        cache.location.ensure_directories().unwrap();

        // Add some entries
        cache.put("key1", "component1", b"data1").unwrap();
        cache.put("key2", "component1", b"data2").unwrap();

        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.total_size, 10); // "data1" + "data2"

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_age_calculation_pure_functions() {
        // Test max age calculation
        let max_age_0_days = SharedCache::calculate_max_age_duration(0);
        let max_age_1_day = SharedCache::calculate_max_age_duration(1);

        assert_eq!(max_age_0_days, Duration::from_secs(0));
        assert_eq!(max_age_1_day, Duration::from_secs(86400));

        // Test should_remove_entry_by_age with zero age (the key fix)
        let now = SystemTime::now();
        let same_time = now;
        let older_time = now - Duration::from_secs(100);

        // With max_age = 0, entries created at same time should be removed
        assert!(SharedCache::should_remove_entry_by_age(
            now,
            same_time,
            Duration::from_secs(0)
        ));

        // With max_age = 0, older entries should definitely be removed
        assert!(SharedCache::should_remove_entry_by_age(
            now,
            older_time,
            Duration::from_secs(0)
        ));

        // With max_age = 200s, older entries (100s old) should not be removed
        assert!(!SharedCache::should_remove_entry_by_age(
            now,
            older_time,
            Duration::from_secs(200)
        ));
    }

    #[test]
    fn test_filter_entries_by_age() {
        let now = SystemTime::now();
        let old_time = now - Duration::from_secs(100);

        let mut entries = HashMap::new();
        entries.insert(
            "recent_entry".to_string(),
            CacheMetadata {
                version: "1.0".to_string(),
                created_at: now,
                last_accessed: now,
                access_count: 1,
                size_bytes: 100,
            },
        );
        entries.insert(
            "old_entry".to_string(),
            CacheMetadata {
                version: "1.0".to_string(),
                created_at: old_time,
                last_accessed: old_time,
                access_count: 1,
                size_bytes: 100,
            },
        );

        // With max_age = 0, both entries should be removed
        let to_remove_0 = SharedCache::filter_entries_by_age(&entries, now, Duration::from_secs(0));
        assert_eq!(to_remove_0.len(), 2);
        assert!(to_remove_0.contains(&"recent_entry".to_string()));
        assert!(to_remove_0.contains(&"old_entry".to_string()));

        // With max_age = 50s, only the old entry should be removed
        let to_remove_50 =
            SharedCache::filter_entries_by_age(&entries, now, Duration::from_secs(50));
        assert_eq!(to_remove_50.len(), 1);
        assert!(to_remove_50.contains(&"old_entry".to_string()));
    }
}
