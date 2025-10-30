//! Pruning logic for SharedCache
//!
//! This module contains all logic related to cache pruning and eviction:
//! - Pure functions for pruning decisions
//! - Pruning execution methods
//! - Cache projection calculations
//! - Stats creation utilities

use crate::cache::auto_pruner::{AutoPruner, PruneStats};
use crate::cache::index_manager::{CacheMetadata, IndexManager};
use crate::cache::pruning::{InternalCacheStats, PruningConfig, PruningStrategyType};
use crate::cache::shared_cache::CacheStats;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// Pure functions for pruning decisions

/// Determine pruning configuration based on environment and test conditions
pub(super) fn determine_pruning_config() -> PruningConfig {
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

/// Determine if pruning is needed after insertion
pub(super) fn should_prune_after_insertion(
    pruner: &AutoPruner,
    stats: &InternalCacheStats,
) -> bool {
    let size_exceeded = stats.total_size > pruner.max_size_bytes as u64;
    let count_exceeded = stats.entry_count > pruner.max_entries;
    size_exceeded || count_exceeded
}

/// Determine the appropriate pruning strategy - pure function
pub(super) fn determine_pruning_strategy(
    config: &PruningConfig,
    has_auto_pruner: bool,
    has_background_pruner: bool,
) -> PruningStrategyType {
    if !has_auto_pruner {
        return PruningStrategyType::NoAutoPruner;
    }

    // If auto-pruning is disabled, don't perform any automatic pruning
    if !config.auto_prune_enabled {
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

/// Check if post-insertion pruning should occur - pure predicate
pub(super) fn should_perform_post_insertion_pruning(
    config: &PruningConfig,
    has_auto_pruner: bool,
) -> bool {
    has_auto_pruner && config.use_sync_pruning
}

/// Log debug information for post-insertion pruning in test mode
pub(super) fn log_post_insertion_debug(stats: &CacheStats, pruner: &AutoPruner) {
    if cfg!(test) {
        println!(
            "Post-insertion check: size={}/{}, count={}/{}",
            stats.total_size, pruner.max_size_bytes, stats.entry_count, pruner.max_entries
        );
        println!("Triggering post-insertion pruning due to limit exceeded");
    }
}

/// Log configuration details for debugging in test environment - pure function
pub(super) fn log_config_if_test_environment(config: &PruningConfig) {
    if config.is_test_environment {
        log::debug!(
            "use_sync_pruning={}, auto_prune={}, cfg_test={}",
            config.use_sync_pruning,
            config.auto_prune_enabled,
            config.is_test_environment
        );
    }
}

// Pure functions for cache projections

/// Calculate projected cache state after adding a new entry
pub(super) fn calculate_cache_projections(
    current_size: u64,
    current_count: usize,
    new_entry_size: usize,
) -> (u64, usize) {
    let projected_size = current_size + new_entry_size as u64;
    let projected_count = current_count + if new_entry_size > 0 { 1 } else { 0 };
    (projected_size, projected_count)
}

/// Determine if pruning is needed based on projections
pub(super) fn should_prune_based_on_projections(
    projected_size: u64,
    projected_count: usize,
    max_size_bytes: usize,
    max_entries: usize,
) -> bool {
    projected_size > max_size_bytes as u64 || projected_count > max_entries
}

/// Determine pruning decision given all inputs
pub(super) fn calculate_pruning_decision(
    current_size: u64,
    current_count: usize,
    new_entry_size: usize,
    max_size_bytes: usize,
    max_entries: usize,
    additional_check: bool,
) -> bool {
    let (projected_size, projected_count) =
        calculate_cache_projections(current_size, current_count, new_entry_size);

    should_prune_based_on_projections(projected_size, projected_count, max_size_bytes, max_entries)
        || additional_check
}

// Pure functions for stats creation

/// Create empty prune stats with current cache state
pub(super) fn create_no_prune_stats(entry_count: usize, total_size: u64) -> PruneStats {
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
pub(super) fn calculate_max_age_duration(max_age_days: i64) -> Duration {
    Duration::from_secs(max_age_days as u64 * 86400)
}

/// Determine if an entry should be removed based on age
pub(super) fn should_remove_entry_by_age(
    now: SystemTime,
    last_accessed: SystemTime,
    max_age: Duration,
) -> bool {
    now.duration_since(last_accessed)
        .map(|age| age >= max_age) // Use >= to handle zero-age case
        .unwrap_or(false) // If time calculation fails, don't remove
}

/// Filter entries to find those that should be removed based on age
pub(super) fn filter_entries_by_age(
    entries: &HashMap<String, CacheMetadata>,
    now: SystemTime,
    max_age: Duration,
) -> Vec<String> {
    entries
        .iter()
        .filter_map(|(key, metadata)| {
            if should_remove_entry_by_age(now, metadata.last_accessed, max_age) {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect()
}

// Helper functions for file operations

/// Delete cache files for the given keys and components
pub(super) fn delete_cache_files_for_keys(
    get_cache_file_path: impl Fn(&str, &str) -> PathBuf,
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
            let cache_path = get_cache_file_path(key, component);
            if cache_path.exists() {
                let _ = fs::remove_file(&cache_path); // Ignore errors
            }
        }
    }
    Ok(())
}

/// Delete files for pruned entries and return counts
pub(super) fn delete_pruned_files(
    get_cache_file_path: impl Fn(&str, &str) -> PathBuf,
    entries_to_remove: &[(String, CacheMetadata)],
) -> (usize, usize) {
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
            let cache_path = get_cache_file_path(key, component);
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

/// Calculate which entries should be pruned
pub(super) fn calculate_entries_to_prune(
    index_manager: &Arc<IndexManager>,
    pruner: &AutoPruner,
) -> Result<Vec<(String, CacheMetadata)>> {
    let index_arc = index_manager.get_index_arc();
    let index = index_arc
        .read()
        .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;
    Ok(pruner.calculate_entries_to_remove(&index))
}

/// Remove entries from index and return bytes freed
pub(super) fn remove_entries_from_index(
    index_manager: &Arc<IndexManager>,
    entries_to_remove: &[(String, CacheMetadata)],
) -> Result<u64> {
    let keys: Vec<String> = entries_to_remove.iter().map(|(k, _)| k.clone()).collect();

    let bytes_freed: u64 = entries_to_remove
        .iter()
        .map(|(_, metadata)| metadata.size_bytes)
        .sum();

    index_manager.remove_entries(&keys)?;

    Ok(bytes_freed)
}

/// Create prune stats from operation results
pub(super) fn create_prune_stats(
    start: SystemTime,
    entries_removed: usize,
    bytes_freed: u64,
    files_deleted: usize,
    files_not_found: usize,
    entry_count: usize,
    total_size: u64,
) -> Result<PruneStats> {
    let duration = start.elapsed().unwrap_or(Duration::ZERO).as_millis() as u64;

    Ok(PruneStats {
        entries_removed,
        bytes_freed,
        entries_remaining: entry_count,
        bytes_remaining: total_size,
        duration_ms: duration,
        files_deleted,
        files_not_found,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cache_projections() {
        let (size, count) = calculate_cache_projections(1000, 10, 500);
        assert_eq!(size, 1500);
        assert_eq!(count, 11);

        // Zero-size entry doesn't increase count
        let (size, count) = calculate_cache_projections(1000, 10, 0);
        assert_eq!(size, 1000);
        assert_eq!(count, 10);
    }

    #[test]
    fn test_should_prune_based_on_projections() {
        // Size exceeded
        assert!(should_prune_based_on_projections(2000, 10, 1000, 100));

        // Count exceeded
        assert!(should_prune_based_on_projections(500, 101, 1000, 100));

        // Neither exceeded
        assert!(!should_prune_based_on_projections(500, 10, 1000, 100));
    }

    #[test]
    fn test_calculate_pruning_decision() {
        // Should prune due to size projection
        assert!(calculate_pruning_decision(900, 10, 200, 1000, 100, false));

        // Should prune due to additional check
        assert!(calculate_pruning_decision(500, 10, 100, 1000, 100, true));

        // Should not prune
        assert!(!calculate_pruning_decision(500, 10, 100, 1000, 100, false));
    }

    #[test]
    fn test_should_remove_entry_by_age() {
        let now = SystemTime::now();
        let old = now - Duration::from_secs(100);
        let max_age = Duration::from_secs(50);

        // Old entry should be removed
        assert!(should_remove_entry_by_age(now, old, max_age));

        // Recent entry should not be removed
        let recent = now - Duration::from_secs(10);
        assert!(!should_remove_entry_by_age(now, recent, max_age));
    }

    #[test]
    fn test_filter_entries_by_age() {
        let now = SystemTime::now();
        let old_time = now - Duration::from_secs(100);
        let recent_time = now - Duration::from_secs(10);

        let mut entries = HashMap::new();
        entries.insert(
            "old_key".to_string(),
            CacheMetadata {
                version: "1.0".to_string(),
                size_bytes: 100,
                last_accessed: old_time,
                created_at: old_time,
                access_count: 1,
                debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        );
        entries.insert(
            "recent_key".to_string(),
            CacheMetadata {
                version: "1.0".to_string(),
                size_bytes: 100,
                last_accessed: recent_time,
                created_at: recent_time,
                access_count: 1,
                debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        );

        let max_age = Duration::from_secs(50);
        let to_remove = filter_entries_by_age(&entries, now, max_age);

        assert_eq!(to_remove.len(), 1);
        assert!(to_remove.contains(&"old_key".to_string()));
    }

    #[test]
    fn test_create_no_prune_stats() {
        let stats = create_no_prune_stats(42, 1024);

        assert_eq!(stats.entries_removed, 0);
        assert_eq!(stats.bytes_freed, 0);
        assert_eq!(stats.entries_remaining, 42);
        assert_eq!(stats.bytes_remaining, 1024);
        assert_eq!(stats.duration_ms, 0);
        assert_eq!(stats.files_deleted, 0);
        assert_eq!(stats.files_not_found, 0);
    }
}
