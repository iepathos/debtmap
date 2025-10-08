use crate::cache::atomic_io::RetryStrategy;
use crate::cache::auto_pruner::{AutoPruner, BackgroundPruner, PruneStats, PruneStrategy};
use crate::cache::cache_location::{CacheLocation, CacheStrategy};
use crate::cache::index_manager::{CacheMetadata, IndexManager};
use crate::cache::pruning::{InternalCacheStats, PruningConfig, PruningStrategyType};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Types now imported from other modules

/// Thread-safe shared cache implementation
pub struct SharedCache {
    pub location: CacheLocation,
    index_manager: IndexManager,
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
        let cache = Self::new_with_location(location)?;
        cache.validate_version()?;
        Ok(cache)
    }

    /// Create a new shared cache instance with explicit cache directory (for testing)
    pub fn new_with_cache_dir(repo_path: Option<&Path>, cache_dir: PathBuf) -> Result<Self> {
        let strategy = CacheStrategy::Custom(cache_dir);
        let location = CacheLocation::resolve_with_strategy(repo_path, strategy)?;
        let cache = Self::new_with_location(location)?;
        cache.validate_version()?;
        Ok(cache)
    }

    /// Create a new shared cache instance with explicit location
    fn new_with_location(location: CacheLocation) -> Result<Self> {
        location.ensure_directories()?;

        let index_manager = IndexManager::load_or_create(&location)?;

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
            index_manager,
            max_cache_size,
            cleanup_threshold: 0.9, // Cleanup when 90% full
            auto_pruner,
            background_pruner,
        })
    }

    // Pure functions for path operations and atomic file handling

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
    fn retry_with_backoff<T, F>(operation: F, operation_name: &str) -> Result<T>
    where
        F: Fn() -> Result<T>,
    {
        const MAX_ATTEMPTS: usize = 3;
        const BASE_DELAY_MS: u64 = 10;

        // Create retry strategy
        let retry_strategy = RetryStrategy::new(MAX_ATTEMPTS, BASE_DELAY_MS);

        // Execute with retry logic
        retry_strategy.execute(operation, operation_name)
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
                temp_path.parent().is_some_and(|p| p.exists()),
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
                target_path.parent().is_some_and(|p| p.exists()),
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
        self.index_manager.save(&self.location)
    }

    /// Get a cache entry
    pub fn get(&self, key: &str, component: &str) -> Result<Vec<u8>> {
        let cache_path = self.get_cache_file_path(key, component);

        if !cache_path.exists() {
            anyhow::bail!("Cache entry not found");
        }

        // Update access metadata
        self.index_manager.update_access_metadata(key)?;

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
    fn is_existing_entry(&self, key: &str) -> bool {
        self.index_manager.is_existing_entry(key)
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
            debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Write cache file atomically to disk
    fn write_cache_file_atomically(cache_path: &Path, data: &[u8]) -> Result<()> {
        let temp_path = Self::create_safe_temp_path(cache_path);
        Self::ensure_atomic_write_directories(cache_path, &temp_path)?;
        Self::write_bytes_atomically(cache_path, &temp_path, data)
    }

    /// Check if key represents a new entry
    fn is_new_entry(&self, key: &str) -> Result<bool> {
        Ok(!self.is_existing_entry(key))
    }

    /// Execute synchronous pruning based on entry status
    fn execute_sync_pruning(&self, key: &str, data_len: usize) -> Result<()> {
        if self.is_new_entry(key)? {
            self.trigger_pruning_if_needed_with_new_entry(data_len)?;
        } else {
            self.trigger_pruning_if_needed()?;
        }
        Ok(())
    }

    /// Determine the appropriate pruning strategy - pure function
    fn determine_pruning_strategy(
        config: &PruningConfig,
        has_auto_pruner: bool,
        has_background_pruner: bool,
    ) -> PruningStrategyType {
        if !has_auto_pruner {
            return PruningStrategyType::NoAutoPruner;
        }

        if config.use_sync_pruning {
            return PruningStrategyType::SyncPruning;
        }

        if has_background_pruner {
            PruningStrategyType::BackgroundPruning
        } else {
            PruningStrategyType::SyncPruning
        }
    }

    /// Execute the determined pruning strategy
    fn execute_pruning_strategy(
        &self,
        strategy: PruningStrategyType,
        key: &str,
        data_len: usize,
    ) -> Result<()> {
        match strategy {
            PruningStrategyType::NoAutoPruner => self.maybe_cleanup(),
            PruningStrategyType::SyncPruning => self.execute_sync_pruning(key, data_len),
            PruningStrategyType::BackgroundPruning => {
                if let Some(bg_pruner) = &self.background_pruner {
                    if !bg_pruner.is_running() {
                        bg_pruner.start_if_needed(self.index_manager.get_index_arc());
                    }
                }
                Ok(())
            }
        }
    }

    /// Handle pre-insertion pruning based on configuration
    fn handle_pre_insertion_pruning(
        &self,
        key: &str,
        data_len: usize,
        config: &PruningConfig,
    ) -> Result<()> {
        let strategy = Self::determine_pruning_strategy(
            config,
            self.auto_pruner.is_some(),
            self.background_pruner.is_some(),
        );
        self.execute_pruning_strategy(strategy, key, data_len)
    }

    /// Check if post-insertion pruning should occur - pure predicate
    fn should_perform_post_insertion_pruning(
        config: &PruningConfig,
        has_auto_pruner: bool,
    ) -> bool {
        has_auto_pruner && config.use_sync_pruning
    }

    /// Log debug information for post-insertion pruning in test mode
    fn log_post_insertion_debug(stats: &CacheStats, pruner: &AutoPruner) {
        if cfg!(test) {
            println!(
                "Post-insertion check: size={}/{}, count={}/{}",
                stats.total_size, pruner.max_size_bytes, stats.entry_count, pruner.max_entries
            );
            println!("Triggering post-insertion pruning due to limit exceeded");
        }
    }

    /// Execute post-insertion pruning check and action
    fn execute_post_insertion_check(&self) -> Result<()> {
        let stats = self.get_stats();
        if let Some(ref pruner) = self.auto_pruner {
            let cache_stats = InternalCacheStats {
                total_size: stats.total_size,
                entry_count: stats.entry_count,
            };

            if Self::should_prune_after_insertion(pruner, &cache_stats) {
                Self::log_post_insertion_debug(&stats, pruner);
                self.trigger_pruning()?;
            }
        }
        Ok(())
    }

    /// Handle post-insertion pruning if needed
    fn handle_post_insertion_pruning(&self, config: &PruningConfig) -> Result<()> {
        if Self::should_perform_post_insertion_pruning(config, self.auto_pruner.is_some()) {
            self.execute_post_insertion_check()?;
        }
        Ok(())
    }

    /// Log configuration details for debugging in test environment - pure function
    fn log_config_if_test_environment(config: &PruningConfig) {
        if config.is_test_environment {
            log::debug!(
                "use_sync_pruning={}, auto_prune={}, cfg_test={}",
                config.use_sync_pruning,
                config.auto_prune_enabled,
                config.is_test_environment
            );
        }
    }

    /// Execute index update operation with proper lock management
    fn execute_index_update(&self, key: &str, data_len: usize) -> Result<()> {
        let metadata = Self::create_cache_metadata(data_len);
        self.index_manager.add_entry(key.to_string(), metadata)
    }

    /// Execute the core cache storage operation - coordinates all steps
    fn execute_cache_storage(&self, key: &str, component: &str, data: &[u8]) -> Result<()> {
        // Write cache file atomically
        let cache_path = self.get_cache_file_path(key, component);
        Self::write_cache_file_atomically(&cache_path, data)?;

        // Update index with new entry
        self.execute_index_update(key, data.len())?;

        // Persist index changes
        self.save_index()?;

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
        // Log configuration for debugging
        Self::log_config_if_test_environment(config);

        // Handle pre-insertion pruning/cleanup
        self.handle_pre_insertion_pruning(key, data.len(), config)?;

        // Execute core storage operations
        self.execute_cache_storage(key, component, data)?;

        // Handle post-insertion pruning if needed
        self.handle_post_insertion_pruning(config)?;

        Ok(())
    }

    /// Compute cache key including file hash
    pub fn compute_cache_key(&self, file_path: &Path) -> Result<String> {
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
        self.index_manager.remove_entry(key)?;

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
        let threshold = (self.max_cache_size as f64 * self.cleanup_threshold) as u64;
        let should_cleanup = self.index_manager.check_total_size_exceeds(threshold);

        if should_cleanup {
            self.cleanup()?;
        }

        Ok(())
    }

    /// Clean up old cache entries
    pub fn cleanup(&self) -> Result<()> {
        let removed_keys = self.determine_keys_to_remove()?;
        self.delete_cache_files(&removed_keys)?;
        self.save_index()?;
        Ok(())
    }

    /// Determine which cache keys should be removed based on size and age
    fn determine_keys_to_remove(&self) -> Result<Vec<String>> {
        let (sorted_entries, total_size) = self.index_manager.get_sorted_entries_and_stats();
        let target_size = self.max_cache_size / 2;
        let keys_to_remove = Self::select_keys_for_removal(sorted_entries, target_size, total_size);

        self.index_manager.remove_entries(&keys_to_remove)?;
        Ok(keys_to_remove)
    }

    /// Select keys for removal until target size is reached
    fn select_keys_for_removal(
        entries: Vec<(String, CacheMetadata)>,
        target_size: u64,
        current_size: u64,
    ) -> Vec<String> {
        let mut removed_keys = Vec::new();
        let mut remaining_size = current_size;

        for (key, metadata) in entries {
            if remaining_size <= target_size {
                break;
            }
            removed_keys.push(key);
            remaining_size -= metadata.size_bytes;
        }
        removed_keys
    }

    /// Delete cache files for the given keys
    fn delete_cache_files(&self, removed_keys: &[String]) -> Result<()> {
        const CACHE_COMPONENTS: &[&str] = &[
            "call_graphs",
            "analysis",
            "metadata",
            "temp",
            "file_metrics",
            "test",
        ];

        for key in removed_keys {
            for component in CACHE_COMPONENTS {
                self.delete_component_file(key, component);
            }
        }
        Ok(())
    }

    /// Delete a single cache component file with error handling
    fn delete_component_file(&self, key: &str, component: &str) {
        let cache_path = self.get_cache_file_path(key, component);
        if cache_path.exists() {
            if let Err(e) = fs::remove_file(&cache_path) {
                log::debug!(
                    "Failed to delete cache file {:?}: {}. This may be due to concurrent access.",
                    cache_path,
                    e
                );
            }
        }
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
    /// Validate cache version and clear if mismatched
    pub fn validate_version(&self) -> Result<bool> {
        let current_version = env!("CARGO_PKG_VERSION");

        let needs_clear = self.index_manager.check_version_mismatch(current_version);

        if needs_clear {
            log::info!(
                "Cache version mismatch detected. Clearing cache for version upgrade to {}",
                current_version
            );
            self.clear()?;
            return Ok(false);
        }

        Ok(true)
    }

    /// Clear entire cache (all entries across all components)
    pub fn clear(&self) -> Result<()> {
        // Get all cache components from the cache directory
        let cache_path = self.location.get_cache_path();

        // Clear all component directories
        if cache_path.exists() {
            for entry in fs::read_dir(cache_path)? {
                let entry = entry?;
                let path = entry.path();

                // Skip non-directories and special files
                if path.is_dir() && entry.file_name() != "." && entry.file_name() != ".." {
                    let component_name = entry.file_name().to_string_lossy().to_string();
                    self.clear_component_files(&component_name)?;
                }
            }
        }

        // Clear index
        self.index_manager.clear_all_entries()?;

        self.save_index()?;
        log::info!("Cache cleared successfully");
        Ok(())
    }

    /// Clear all cache entries for this project
    pub fn clear_project(&self) -> Result<()> {
        // Clear all files in all components
        let components = [
            "call_graphs",
            "analysis",
            "metadata",
            "temp",
            "file_metrics",
        ];

        for component in &components {
            self.clear_component_files(component)?;
        }

        // Clear index
        self.index_manager.clear_all_entries()?;

        self.save_index()?;
        Ok(())
    }

    /// Clear all files in a component directory
    fn clear_component_files(&self, component: &str) -> Result<()> {
        let component_path = self.location.get_component_path(component);
        if !component_path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&component_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                fs::remove_file(&path)?;
                continue;
            }

            if path.is_dir() {
                // Remove files in sharded subdirectories
                for subentry in fs::read_dir(&path)? {
                    let subentry = subentry?;
                    if subentry.path().is_file() {
                        fs::remove_file(subentry.path())?;
                    }
                }
                // Try to remove the now-empty shard directory
                let _ = fs::remove_dir(&path);
            }
        }

        Ok(())
    }

    pub fn get_stats(&self) -> CacheStats {
        let stats = self.index_manager.get_stats();

        CacheStats {
            entry_count: stats.entry_count,
            total_size: stats.total_size,
        }
    }

    pub fn get_full_stats(&self) -> Result<FullCacheStats> {
        let (total_entries, total_size) = self.index_manager.get_full_stats();

        Ok(FullCacheStats {
            total_entries,
            total_size,
            cache_location: self.location.get_cache_path().to_path_buf(),
            strategy: self.location.strategy.clone(),
            project_id: self.location.project_id.clone(),
        })
    }

    /// Create a new shared cache with auto-pruning enabled
    pub fn with_auto_pruning(repo_path: Option<&Path>, pruner: AutoPruner) -> Result<Self> {
        let location = CacheLocation::resolve(repo_path)?;
        location.ensure_directories()?;
        let index_manager = IndexManager::load_or_create(&location)?;

        let background_pruner = BackgroundPruner::new(pruner.clone());

        Ok(Self {
            location,
            index_manager,
            max_cache_size: pruner.max_size_bytes as u64,
            cleanup_threshold: 0.9,
            auto_pruner: Some(pruner),
            background_pruner: Some(background_pruner),
        })
    }

    /// Create a new shared cache with auto-pruning enabled and explicit cache directory (for testing)
    pub fn with_auto_pruning_and_cache_dir(
        repo_path: Option<&Path>,
        cache_dir: PathBuf,
        pruner: AutoPruner,
    ) -> Result<Self> {
        let strategy = CacheStrategy::Custom(cache_dir);
        let location = CacheLocation::resolve_with_strategy(repo_path, strategy)?;
        location.ensure_directories()?;
        let index_manager = IndexManager::load_or_create(&location)?;

        let background_pruner = BackgroundPruner::new(pruner.clone());

        Ok(Self {
            location,
            index_manager,
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
                let (entry_count, total_size) = self.index_manager.get_full_stats();

                // Calculate pruning decision without lock
                let basic_decision = Self::calculate_pruning_decision(
                    total_size,
                    entry_count,
                    new_entry_size,
                    pruner.max_size_bytes,
                    pruner.max_entries,
                    false,
                );

                // If basic check says prune, we're done
                if basic_decision {
                    true
                } else {
                    // Otherwise check the more complex pruner logic
                    let index_arc = self.index_manager.get_index_arc();
                    let index = index_arc
                        .read()
                        .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
                    pruner.should_prune(&index)
                }
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

        // Early return with fallback for no pruner case
        let pruner = match &self.auto_pruner {
            Some(p) => p,
            None => return self.execute_fallback_cleanup(start),
        };

        // Calculate entries to remove (pure function)
        let entries_to_remove = self.calculate_entries_to_prune(pruner)?;

        // Early return if nothing to prune
        if entries_to_remove.is_empty() {
            return Ok(self.create_empty_prune_stats());
        }

        // Execute pruning operations
        let bytes_freed = self.remove_entries_from_index(&entries_to_remove)?;
        let (files_deleted, files_not_found) = self.delete_pruned_files(&entries_to_remove);
        self.save_index()?;

        // Create result stats
        self.create_prune_stats(
            start,
            entries_to_remove.len(),
            bytes_freed,
            files_deleted,
            files_not_found,
        )
    }

    /// Execute fallback cleanup when no pruner is configured
    fn execute_fallback_cleanup(&self, start: SystemTime) -> Result<PruneStats> {
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

    /// Calculate which entries should be pruned - pure function
    fn calculate_entries_to_prune(
        &self,
        pruner: &AutoPruner,
    ) -> Result<Vec<(String, CacheMetadata)>> {
        let index_arc = self.index_manager.get_index_arc();
        let index = index_arc
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
        Ok(pruner.calculate_entries_to_remove(&index))
    }

    /// Remove entries from index and return bytes freed
    fn remove_entries_from_index(
        &self,
        entries_to_remove: &[(String, CacheMetadata)],
    ) -> Result<u64> {
        let keys: Vec<String> = entries_to_remove.iter().map(|(k, _)| k.clone()).collect();

        let bytes_freed: u64 = entries_to_remove
            .iter()
            .map(|(_, metadata)| metadata.size_bytes)
            .sum();

        self.index_manager.remove_entries(&keys)?;

        Ok(bytes_freed)
    }

    /// Create empty stats when no pruning is needed
    fn create_empty_prune_stats(&self) -> PruneStats {
        let stats = self.get_stats();
        PruneStats {
            entries_removed: 0,
            bytes_freed: 0,
            entries_remaining: stats.entry_count,
            bytes_remaining: stats.total_size,
            duration_ms: 0,
            files_deleted: 0,
            files_not_found: 0,
        }
    }

    /// Create prune stats from operation results
    fn create_prune_stats(
        &self,
        start: SystemTime,
        entries_removed: usize,
        bytes_freed: u64,
        files_deleted: usize,
        files_not_found: usize,
    ) -> Result<PruneStats> {
        let duration = start.elapsed().unwrap_or(Duration::ZERO).as_millis() as u64;
        let final_stats = self.get_stats();

        Ok(PruneStats {
            entries_removed,
            bytes_freed,
            entries_remaining: final_stats.entry_count,
            bytes_remaining: final_stats.total_size,
            duration_ms: duration,
            files_deleted,
            files_not_found,
        })
    }

    /// Delete files for pruned entries and return counts
    fn delete_pruned_files(&self, entries_to_remove: &[(String, CacheMetadata)]) -> (usize, usize) {
        let mut files_deleted = 0usize;
        let mut files_not_found = 0usize;
        let components = [
            "call_graphs",
            "analysis",
            "metadata",
            "temp",
            "file_metrics",
            "test",
        ];

        for (key, _) in entries_to_remove {
            let mut any_file_found = false;

            for component in &components {
                let cache_path = self.get_cache_file_path(key, component);
                if !cache_path.exists() {
                    continue;
                }

                any_file_found = true;
                if fs::remove_file(&cache_path).is_ok() {
                    files_deleted += 1;
                } else {
                    log::warn!("Failed to delete cache file: {:?}", cache_path);
                }
            }

            if !any_file_found {
                files_not_found += 1;
                log::debug!("No files found for cache entry: {}", key);
            }
        }

        (files_deleted, files_not_found)
    }

    /// Prune entries with a specific strategy
    pub fn prune_with_strategy(&self, strategy: PruneStrategy) -> Result<PruneStats> {
        // Create a temporary pruner with the specified strategy
        let temp_pruner = AutoPruner {
            strategy,
            ..self.auto_pruner.clone().unwrap_or_default()
        };

        let start = SystemTime::now();
        let index_arc = self.index_manager.get_index_arc();
        let entries_to_remove = {
            let index = index_arc
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

        let bytes_freed: u64 = entries_to_remove.iter().map(|(_, m)| m.size_bytes).sum();
        let mut files_deleted = 0usize;

        // Remove from index
        let keys: Vec<String> = entries_to_remove.iter().map(|(k, _)| k.clone()).collect();
        self.index_manager.remove_entries(&keys)?;

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
        let entries_to_check = self.index_manager.get_all_entry_keys();

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
            removed_count = self.index_manager.remove_entries(&orphaned_entries)? as usize;
            for key in &orphaned_entries {
                log::debug!("Removed orphaned cache entry: {}", key);
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
        let entries_snapshot = self.index_manager.get_entries_snapshot();
        let entries_to_remove = Self::filter_entries_by_age(&entries_snapshot, now, max_age);

        // Step 3: Update index (side effect - but isolated)
        let removed_count = self.index_manager.remove_entries(&entries_to_remove)?;

        // Step 4: Delete files (side effect - but isolated)
        Self::delete_cache_files_for_keys(self, &entries_to_remove)
            .map_err(|_| anyhow::anyhow!("Failed to delete cache files"))?;

        // Step 5: Persist changes (side effect - but isolated)
        self.save_index()?;

        Ok(removed_count as usize)
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
                debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
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
                debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
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

    #[test]
    fn test_put_with_config_test_environment() {
        let temp_dir = TempDir::new().unwrap();

        // Set environment variables
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        let test_config = PruningConfig {
            auto_prune_enabled: false,
            use_sync_pruning: false,
            is_test_environment: true,
        };

        let key = "test_key";
        let component = "test_component";
        let data = b"test data for config";

        // Test put_with_config in test environment
        cache
            .put_with_config(key, component, data, &test_config)
            .unwrap();

        // Verify the data was stored correctly
        assert!(cache.exists(key, component));
        let retrieved = cache.get(key, component).unwrap();
        assert_eq!(retrieved, data);

        // Verify index was updated
        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 1);
        assert!(stats.total_size >= data.len() as u64);

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_put_with_config_sync_pruning_enabled() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "true");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        let sync_prune_config = PruningConfig {
            auto_prune_enabled: true,
            use_sync_pruning: true,
            is_test_environment: false,
        };

        let key = "sync_prune_key";
        let component = "sync_component";
        let data = b"data with sync pruning";

        // Test put_with_config with sync pruning
        cache
            .put_with_config(key, component, data, &sync_prune_config)
            .unwrap();

        // Verify the data was stored correctly
        assert!(cache.exists(key, component));
        let retrieved = cache.get(key, component).unwrap();
        assert_eq!(retrieved, data);

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_put_with_config_auto_prune_disabled() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        let no_prune_config = PruningConfig {
            auto_prune_enabled: false,
            use_sync_pruning: false,
            is_test_environment: false,
        };

        let key = "no_prune_key";
        let component = "no_prune_component";
        let data = b"data with no auto pruning";

        // Test put_with_config with auto pruning disabled
        cache
            .put_with_config(key, component, data, &no_prune_config)
            .unwrap();

        // Verify the data was stored correctly
        assert!(cache.exists(key, component));
        let retrieved = cache.get(key, component).unwrap();
        assert_eq!(retrieved, data);

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_put_with_config_multiple_entries() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        let config = PruningConfig {
            auto_prune_enabled: false,
            use_sync_pruning: false,
            is_test_environment: true,
        };

        // Store multiple entries with the same config
        let entries = vec![
            ("key1", "component1", b"data1" as &[u8]),
            ("key2", "component2", b"data2"),
            ("key3", "component1", b"data3"),
        ];

        for (key, component, data) in &entries {
            cache
                .put_with_config(key, component, data, &config)
                .unwrap();
        }

        // Verify all entries were stored correctly
        for (key, component, expected_data) in &entries {
            assert!(cache.exists(key, component));
            let retrieved = cache.get(key, component).unwrap();
            assert_eq!(retrieved, *expected_data);
        }

        // Verify index reflects all entries
        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 3);

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_put_with_config_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        let config = PruningConfig {
            auto_prune_enabled: false,
            use_sync_pruning: false,
            is_test_environment: false,
        };

        let key = "overwrite_key";
        let component = "overwrite_component";
        let original_data = b"original data";
        let new_data = b"updated data that is longer";

        // Store original data
        cache
            .put_with_config(key, component, original_data, &config)
            .unwrap();
        assert!(cache.exists(key, component));
        let retrieved = cache.get(key, component).unwrap();
        assert_eq!(retrieved, original_data);

        // Overwrite with new data
        cache
            .put_with_config(key, component, new_data, &config)
            .unwrap();
        assert!(cache.exists(key, component));
        let retrieved = cache.get(key, component).unwrap();
        assert_eq!(retrieved, new_data);

        // Verify index still shows only one entry
        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 1);

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cache_version_validation() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        // Create a cache and add an entry
        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
        cache
            .put("test_key", "test_component", b"test data")
            .unwrap();

        // Verify the entry exists
        assert!(cache.exists("test_key", "test_component"));
        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 1);

        // Create another cache instance - should validate version (same version, no clear)
        let cache2 = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
        assert!(cache2.exists("test_key", "test_component"));

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        // Add multiple entries
        cache.put("key1", "component1", b"data1").unwrap();
        cache.put("key2", "component2", b"data2").unwrap();
        cache.put("key3", "component3", b"data3").unwrap();

        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 3);

        // Clear the entire cache
        cache.clear().unwrap();

        // Verify all entries are gone
        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 0);
        assert!(!cache.exists("key1", "component1"));
        assert!(!cache.exists("key2", "component2"));
        assert!(!cache.exists("key3", "component3"));

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_compute_cache_key_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        let key = cache.compute_cache_key(&test_file).unwrap();
        assert!(key.contains("test.rs"));
        assert!(key.contains(":")); // Should have hash separator

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_compute_cache_key_without_file() {
        let temp_dir = TempDir::new().unwrap();
        let non_existent_path = temp_dir.path().join("non_existent.rs");

        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        let key = cache.compute_cache_key(&non_existent_path).unwrap();
        assert!(key.contains("non_existent.rs"));
        assert!(!key.contains(":")); // Should not have hash separator

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_put_with_config_large_data() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        let config = PruningConfig {
            auto_prune_enabled: false,
            use_sync_pruning: false,
            is_test_environment: true,
        };

        let key = "large_data_key";
        let component = "large_component";
        let large_data = vec![0u8; 1024 * 1024]; // 1MB of data

        // Test put_with_config with large data
        cache
            .put_with_config(key, component, &large_data, &config)
            .unwrap();

        // Verify the data was stored correctly
        assert!(cache.exists(key, component));
        let retrieved = cache.get(key, component).unwrap();
        assert_eq!(retrieved, large_data);

        // Verify index reflects the large size
        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 1);
        assert!(stats.total_size >= large_data.len() as u64);

        // Cleanup
        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cleanup_removes_oldest_entries() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let mut cache =
            SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
        cache.max_cache_size = 100; // Set small size to trigger cleanup

        // Create large entries to ensure we exceed max_cache_size
        let large_data = vec![0u8; 40]; // Each entry is 40 bytes

        // Add old entries
        cache.put("old_key1", "component1", &large_data).unwrap();
        cache.put("old_key2", "component1", &large_data).unwrap();

        // Sleep briefly to ensure time difference
        std::thread::sleep(Duration::from_millis(10));

        // Add recent entries
        cache.put("recent_key1", "component1", &large_data).unwrap();
        cache.put("recent_key2", "component1", &large_data).unwrap();

        // Access recent entries to update their access time
        cache.get("recent_key1", "component1").unwrap();
        cache.get("recent_key2", "component1").unwrap();

        // Debug: Check actual size before cleanup
        let stats_before = cache.get_stats();
        eprintln!(
            "Before cleanup - entries: {}, size: {}",
            stats_before.entry_count, stats_before.total_size
        );

        // Total size should be ~160 bytes, max is 100, target after cleanup is 50
        // Manually trigger cleanup
        cache.cleanup().unwrap();

        // Debug: Check actual size after cleanup
        let stats_after = cache.get_stats();
        eprintln!(
            "After cleanup - entries: {}, size: {}",
            stats_after.entry_count, stats_after.total_size
        );

        // The cleanup should have removed some entries to get under target (50 bytes)
        assert!(
            stats_after.entry_count < stats_before.entry_count,
            "Cleanup should have removed entries: {} -> {}",
            stats_before.entry_count,
            stats_after.entry_count
        );
        assert!(
            stats_after.total_size <= cache.max_cache_size / 2,
            "Size should be under target: {} <= {}",
            stats_after.total_size,
            cache.max_cache_size / 2
        );

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cleanup_target_size_calculation() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        // Create cache with specific max size
        let mut cache =
            SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
        cache.max_cache_size = 1000; // Set a small size for testing

        // Add entries that exceed half the max size
        let data = vec![0u8; 300]; // Each entry is 300 bytes
        cache.put("key1", "component", &data).unwrap();
        cache.put("key2", "component", &data).unwrap();
        cache.put("key3", "component", &data).unwrap();

        // Total size should be ~900 bytes, target after cleanup is 500

        // Run cleanup
        cache.cleanup().unwrap();

        // Verify total size is now under target (500 bytes)
        let stats = cache.get_stats();
        assert!(stats.total_size <= 500);

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cleanup_handles_empty_cache() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        // Cleanup on empty cache should not error
        let result = cache.cleanup();
        assert!(result.is_ok());

        // Cache should still be empty
        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_size, 0);

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cleanup_removes_files_from_all_components() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        // Add entries to different components
        let components = vec![
            "call_graphs",
            "analysis",
            "metadata",
            "temp",
            "file_metrics",
            "test",
        ];

        let key = "test_key";
        let data = b"test_data";

        // Add to each component
        for component in &components {
            cache.put(key, component, data).unwrap();
            assert!(cache.exists(key, component));
        }

        // Force cache size to be large enough to trigger cleanup
        {
            let index_arc = cache.index_manager.get_index_arc();
            let mut index = index_arc.write().unwrap();
            index.total_size = cache.max_cache_size + 1;
        }

        // Run cleanup
        cache.cleanup().unwrap();

        // Verify all component files are removed
        for component in &components {
            assert!(!cache.exists(key, component));
        }

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cleanup_updates_index_correctly() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let mut cache =
            SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
        cache.max_cache_size = 200; // Increase size to allow all entries to be added first

        // Add multiple entries
        let entries: Vec<(&str, Vec<u8>)> = vec![
            ("key1", vec![0u8; 20]),
            ("key2", vec![0u8; 20]),
            ("key3", vec![0u8; 20]),
            ("key4", vec![0u8; 20]),
        ];

        for (key, data) in &entries {
            cache.put(key, "component", data).unwrap();
        }

        let initial_stats = cache.get_stats();
        assert!(initial_stats.entry_count > 0, "Should have entries");

        // Now reduce max_cache_size to force cleanup
        cache.max_cache_size = 50;

        // Run cleanup - should remove entries to get under 25 bytes (50% of max)
        cache.cleanup().unwrap();

        // Verify index is updated
        let final_stats = cache.get_stats();
        assert!(
            final_stats.entry_count < initial_stats.entry_count,
            "Entry count should decrease: {} -> {}",
            initial_stats.entry_count,
            final_stats.entry_count
        );
        assert!(
            final_stats.total_size <= cache.max_cache_size / 2,
            "Total size should be under target: {} <= {}",
            final_stats.total_size,
            cache.max_cache_size / 2
        );

        // Verify last_cleanup is set
        {
            let index_arc = cache.index_manager.get_index_arc();
            let index = index_arc.read().unwrap();
            assert!(index.last_cleanup.is_some());
        }

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cleanup_handles_concurrent_file_access() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

        // Add an entry
        let key = "concurrent_key";
        let component = "component";
        cache.put(key, component, b"data").unwrap();

        // Manually remove the file to simulate concurrent deletion
        let cache_path = cache.get_cache_file_path(key, component);
        if cache_path.exists() {
            fs::remove_file(&cache_path).ok();
        }

        // Cleanup should handle the missing file gracefully
        let result = cache.cleanup();
        assert!(result.is_ok());

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cleanup_preserves_entries_under_target_size() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let mut cache =
            SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
        cache.max_cache_size = 1000;

        // Add entries that total less than half the max size (target = 500)
        cache.put("keep1", "component", b"small").unwrap();
        cache.put("keep2", "component", b"data").unwrap();

        let initial_count = cache.get_stats().entry_count;

        // Run cleanup
        cache.cleanup().unwrap();

        // All entries should be preserved since we're under target
        let final_count = cache.get_stats().entry_count;
        assert_eq!(initial_count, final_count);
        assert!(cache.exists("keep1", "component"));
        assert!(cache.exists("keep2", "component"));

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_cleanup_pure_functions_behavior() {
        use std::collections::HashMap;
        use std::time::{Duration, SystemTime};

        // Test sort_entries_by_access_time
        let now = SystemTime::now();
        let old_time = now - Duration::from_secs(3600); // 1 hour ago
        let very_old_time = now - Duration::from_secs(7200); // 2 hours ago

        let mut entries = HashMap::new();
        entries.insert(
            "newest".to_string(),
            CacheMetadata {
                version: "1.0".to_string(),
                created_at: now,
                last_accessed: now,
                access_count: 1,
                size_bytes: 10,
                debtmap_version: "0.2.0".to_string(),
            },
        );
        entries.insert(
            "oldest".to_string(),
            CacheMetadata {
                version: "1.0".to_string(),
                created_at: very_old_time,
                last_accessed: very_old_time,
                access_count: 1,
                size_bytes: 20,
                debtmap_version: "0.2.0".to_string(),
            },
        );
        entries.insert(
            "middle".to_string(),
            CacheMetadata {
                version: "1.0".to_string(),
                created_at: old_time,
                last_accessed: old_time,
                access_count: 1,
                size_bytes: 15,
                debtmap_version: "0.2.0".to_string(),
            },
        );

        // Sort entries by access time (oldest first)
        let mut sorted: Vec<(String, CacheMetadata)> = entries.into_iter().collect();
        sorted.sort_by_key(|(_, metadata)| metadata.last_accessed);

        // Should be sorted with oldest first
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].0, "oldest");
        assert_eq!(sorted[1].0, "middle");
        assert_eq!(sorted[2].0, "newest");

        // Test select_keys_for_removal
        let keys_to_remove = SharedCache::select_keys_for_removal(sorted.clone(), 25, 45);

        // With total size 45 and target 25, should remove oldest (20 bytes) to get to 25
        assert_eq!(keys_to_remove.len(), 1);
        assert_eq!(keys_to_remove[0], "oldest");

        // Test with smaller target - should remove multiple entries
        let keys_to_remove_multiple = SharedCache::select_keys_for_removal(sorted, 10, 45);

        // Should remove oldest (20) + middle (15) = 35, leaving 10 which is under target
        assert_eq!(keys_to_remove_multiple.len(), 2);
        assert_eq!(keys_to_remove_multiple[0], "oldest");
        assert_eq!(keys_to_remove_multiple[1], "middle");
    }
}
