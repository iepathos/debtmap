// Module declarations
pub mod builder;
mod file_ops;
mod pruning;
pub mod reader;
pub mod writer;

use crate::cache::auto_pruner::{AutoPruner, BackgroundPruner, PruneStats, PruneStrategy};
use crate::cache::cache_location::{CacheLocation, CacheStrategy};
use crate::cache::index_manager::{CacheMetadata, IndexManager};
use crate::cache::pruning::{InternalCacheStats, PruningConfig, PruningStrategyType};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// Re-export for backward compatibility
pub use builder::SharedCacheBuilder;
pub use reader::CacheReader;
pub use writer::CacheWriter;

// Re-export for testing
#[cfg(test)]
pub(crate) use file_ops::{
    build_dest_path, classify_entry, copy_dir_entry, copy_file_entry, EntryType,
};

/// Thread-safe shared cache implementation
pub struct SharedCache {
    pub location: CacheLocation,
    pub(super) reader: CacheReader,
    pub(super) writer: CacheWriter,
    pub(super) index_manager: Arc<IndexManager>,
    pub(super) max_cache_size: u64,
    pub(super) cleanup_threshold: f64,
    pub(super) auto_pruner: Option<AutoPruner>,
    pub(super) background_pruner: Option<BackgroundPruner>,
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
        let mut builder = SharedCacheBuilder::new();
        if let Some(path) = repo_path {
            builder = builder.repo_path(path);
        }
        let cache = builder.build()?;
        cache.validate_version()?;
        Ok(cache)
    }

    /// Create a new shared cache instance with explicit cache directory (for testing)
    pub fn new_with_cache_dir(repo_path: Option<&Path>, cache_dir: PathBuf) -> Result<Self> {
        let mut builder = SharedCacheBuilder::new().cache_dir(cache_dir);
        if let Some(path) = repo_path {
            builder = builder.repo_path(path);
        }
        let cache = builder.build()?;
        cache.validate_version()?;
        Ok(cache)
    }

    /// Save the current index to disk with comprehensive error handling
    pub fn save_index(&self) -> Result<()> {
        self.index_manager.save(&self.location)
    }

    /// Get a cache entry
    pub fn get(&self, key: &str, component: &str) -> Result<Vec<u8>> {
        self.reader.get(key, component)
    }

    // Delegate to pruning module

    /// Determine pruning configuration based on environment and test conditions
    fn determine_pruning_config() -> PruningConfig {
        pruning::determine_pruning_config()
    }

    /// Determine if an entry already exists in the index
    fn is_existing_entry(&self, key: &str) -> bool {
        self.index_manager.is_existing_entry(key)
    }

    /// Determine if pruning is needed after insertion
    fn should_prune_after_insertion(pruner: &AutoPruner, stats: &InternalCacheStats) -> bool {
        pruning::should_prune_after_insertion(pruner, stats)
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
        pruning::determine_pruning_strategy(config, has_auto_pruner, has_background_pruner)
    }

    /// Execute the determined pruning strategy
    fn execute_pruning_strategy(
        &self,
        strategy: PruningStrategyType,
        key: &str,
        data_len: usize,
    ) -> Result<()> {
        match strategy {
            PruningStrategyType::NoAutoPruner => {
                // Only run cleanup if there's no auto_pruner configured
                // If auto_pruner exists but is disabled, we respect that and don't cleanup
                if self.auto_pruner.is_none() {
                    self.maybe_cleanup()
                } else {
                    Ok(())
                }
            }
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
        pruning::should_perform_post_insertion_pruning(config, has_auto_pruner)
    }

    /// Log debug information for post-insertion pruning in test mode
    fn log_post_insertion_debug(stats: &CacheStats, pruner: &AutoPruner) {
        pruning::log_post_insertion_debug(stats, pruner)
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
        pruning::log_config_if_test_environment(config)
    }

    /// Execute the core cache storage operation - coordinates all steps
    fn execute_cache_storage(&self, key: &str, component: &str, data: &[u8]) -> Result<()> {
        // Delegate to writer which handles atomicity and index updates
        self.writer.put(key, component, data)
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
        self.reader.compute_cache_key(file_path)
    }

    /// Store a cache entry
    pub fn put(&self, key: &str, component: &str, data: &[u8]) -> Result<()> {
        let config = Self::determine_pruning_config();
        self.put_with_config(key, component, data, &config)
    }

    /// Check if a cache entry exists
    pub fn exists(&self, key: &str, component: &str) -> bool {
        self.reader.exists(key, component)
    }

    /// Delete a cache entry
    pub fn delete(&self, key: &str, component: &str) -> Result<()> {
        self.writer.delete(key, component)
    }

    /// Get the file path for a cache entry
    fn get_cache_file_path(&self, key: &str, component: &str) -> PathBuf {
        file_ops::get_cache_file_path(&self.location, key, component)
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
        let removed_keys =
            file_ops::determine_keys_to_remove(&self.index_manager, self.max_cache_size)?;
        file_ops::delete_cache_files(&self.location, &removed_keys)?;
        self.save_index()?;
        Ok(())
    }

    /// Migrate cache from local to shared location
    pub fn migrate_from_local(&self, local_cache_path: &Path) -> Result<()> {
        file_ops::migrate_from_local(&self.location, local_cache_path)
    }

    /// Recursively copy directory contents (used in tests)
    #[cfg(test)]
    pub(crate) fn copy_dir_recursive(&self, src: &Path, dest: &Path) -> Result<()> {
        file_ops::copy_dir_recursive(src, dest)
    }

    /// Select keys for removal (used in tests)
    #[cfg(test)]
    pub(crate) fn select_keys_for_removal(
        entries: Vec<(String, CacheMetadata)>,
        target_size: u64,
        current_size: u64,
    ) -> Vec<String> {
        file_ops::select_keys_for_removal(entries, target_size, current_size)
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
        file_ops::clear_component_files(&self.location, component)
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
        let mut builder = SharedCacheBuilder::new().auto_pruner(pruner);
        if let Some(path) = repo_path {
            builder = builder.repo_path(path);
        }
        builder.build()
    }

    /// Create a new shared cache with auto-pruning enabled and explicit cache directory (for testing)
    pub fn with_auto_pruning_and_cache_dir(
        repo_path: Option<&Path>,
        cache_dir: PathBuf,
        pruner: AutoPruner,
    ) -> Result<Self> {
        let mut builder = SharedCacheBuilder::new()
            .cache_dir(cache_dir)
            .auto_pruner(pruner);
        if let Some(path) = repo_path {
            builder = builder.repo_path(path);
        }
        builder.build()
    }

    // Delegate to pruning module
    fn calculate_pruning_decision(
        current_size: u64,
        current_count: usize,
        new_entry_size: usize,
        max_size_bytes: usize,
        max_entries: usize,
        additional_check: bool,
    ) -> bool {
        pruning::calculate_pruning_decision(
            current_size,
            current_count,
            new_entry_size,
            max_size_bytes,
            max_entries,
            additional_check,
        )
    }

    fn create_no_prune_stats(entry_count: usize, total_size: u64) -> PruneStats {
        pruning::create_no_prune_stats(entry_count, total_size)
    }

    fn calculate_max_age_duration(max_age_days: i64) -> Duration {
        pruning::calculate_max_age_duration(max_age_days)
    }

    #[allow(dead_code)] // Used in tests
    fn should_remove_entry_by_age(
        now: SystemTime,
        last_accessed: SystemTime,
        max_age: Duration,
    ) -> bool {
        pruning::should_remove_entry_by_age(now, last_accessed, max_age)
    }

    fn filter_entries_by_age(
        entries: &HashMap<String, CacheMetadata>,
        now: SystemTime,
        max_age: Duration,
    ) -> Vec<String> {
        pruning::filter_entries_by_age(entries, now, max_age)
    }

    fn delete_cache_files_for_keys(
        cache: &SharedCache,
        keys: &[String],
    ) -> std::result::Result<(), ()> {
        pruning::delete_cache_files_for_keys(|k, c| cache.get_cache_file_path(k, c), keys)
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

    /// Calculate which entries should be pruned
    fn calculate_entries_to_prune(
        &self,
        pruner: &AutoPruner,
    ) -> Result<Vec<(String, CacheMetadata)>> {
        pruning::calculate_entries_to_prune(&self.index_manager, pruner)
    }

    /// Remove entries from index and return bytes freed
    fn remove_entries_from_index(
        &self,
        entries_to_remove: &[(String, CacheMetadata)],
    ) -> Result<u64> {
        pruning::remove_entries_from_index(&self.index_manager, entries_to_remove)
    }

    /// Create empty stats when no pruning is needed
    fn create_empty_prune_stats(&self) -> PruneStats {
        let stats = self.get_stats();
        pruning::create_no_prune_stats(stats.entry_count, stats.total_size)
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
        let stats = self.get_stats();
        pruning::create_prune_stats(
            start,
            entries_removed,
            bytes_freed,
            files_deleted,
            files_not_found,
            stats.entry_count,
            stats.total_size,
        )
    }

    /// Delete files for pruned entries and return counts
    fn delete_pruned_files(&self, entries_to_remove: &[(String, CacheMetadata)]) -> (usize, usize) {
        pruning::delete_pruned_files(|k, c| self.get_cache_file_path(k, c), entries_to_remove)
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
mod tests;
