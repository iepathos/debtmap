//! Pure pruning decision logic
//!
//! This module contains pure functions for making pruning decisions without side effects.
//! All functions here are deterministic and easily testable.

use crate::cache::auto_pruner::AutoPruner;
use crate::cache::index_manager::CacheIndex;

/// Configuration for pruning behavior
#[derive(Debug, Clone)]
pub struct PruningConfig {
    pub auto_prune_enabled: bool,
    pub use_sync_pruning: bool,
    pub is_test_environment: bool,
}

/// Pruning strategy selection
#[derive(Debug, Clone, PartialEq)]
pub enum PruningStrategyType {
    NoAutoPruner,      // No auto pruner configured
    SyncPruning,       // Use synchronous pruning
    BackgroundPruning, // Use background pruning
}

/// Internal cache statistics for pruning decisions
#[derive(Debug, Clone)]
pub struct InternalCacheStats {
    pub total_size: u64,
    pub entry_count: usize,
}

/// Projected cache statistics after a potential insertion
#[derive(Debug, Clone)]
pub struct ProjectedCacheStats {
    pub projected_size: u64,
    pub projected_count: usize,
}

/// Determine pruning configuration from environment
pub fn determine_pruning_config() -> PruningConfig {
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
pub fn is_existing_entry(index: &CacheIndex, key: &str) -> bool {
    index.entries.contains_key(key)
}

/// Determine if pruning is needed after insertion
pub fn should_prune_after_insertion(pruner: &AutoPruner, stats: &InternalCacheStats) -> bool {
    let size_exceeded = stats.total_size > pruner.max_size_bytes as u64;
    let count_exceeded = stats.entry_count > pruner.max_entries;
    size_exceeded || count_exceeded
}

/// Determine the appropriate pruning strategy - pure function
pub fn determine_pruning_strategy(
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

/// Check if post-insertion pruning should occur - pure predicate
pub fn should_perform_post_insertion_pruning(
    config: &PruningConfig,
    has_auto_pruner: bool,
) -> bool {
    has_auto_pruner && config.use_sync_pruning
}

/// Calculate cache projections after adding new entry
pub fn calculate_cache_projections(
    current: &InternalCacheStats,
    new_entry_size: usize,
    is_new_entry: bool,
) -> ProjectedCacheStats {
    let projected_count = if is_new_entry {
        current.entry_count + 1
    } else {
        current.entry_count
    };

    ProjectedCacheStats {
        projected_size: current.total_size + new_entry_size as u64,
        projected_count,
    }
}

/// Determine if pruning is needed based on projections
pub fn should_prune_based_on_projections(
    projected: &ProjectedCacheStats,
    pruner: &AutoPruner,
) -> bool {
    let size_would_exceed = projected.projected_size > pruner.max_size_bytes as u64;
    let count_would_exceed = projected.projected_count > pruner.max_entries;
    size_would_exceed || count_would_exceed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_existing_entry() {
        let mut index = CacheIndex::default();
        assert!(!is_existing_entry(&index, "key1"));

        index.entries.insert(
            "key1".to_string(),
            crate::cache::index_manager::CacheMetadata {
                version: "1.0.0".to_string(),
                created_at: std::time::SystemTime::now(),
                last_accessed: std::time::SystemTime::now(),
                access_count: 1,
                size_bytes: 100,
                debtmap_version: "1.0.0".to_string(),
            },
        );
        assert!(is_existing_entry(&index, "key1"));
    }

    #[test]
    fn test_should_prune_after_insertion_size_exceeded() {
        let pruner = AutoPruner {
            max_size_bytes: 1000,
            max_entries: 100,
            ..Default::default()
        };

        let stats = InternalCacheStats {
            total_size: 1500,
            entry_count: 10,
        };

        assert!(should_prune_after_insertion(&pruner, &stats));
    }

    #[test]
    fn test_should_prune_after_insertion_count_exceeded() {
        let pruner = AutoPruner {
            max_size_bytes: 10000,
            max_entries: 5,
            ..Default::default()
        };

        let stats = InternalCacheStats {
            total_size: 500,
            entry_count: 10,
        };

        assert!(should_prune_after_insertion(&pruner, &stats));
    }

    #[test]
    fn test_should_not_prune_when_under_limits() {
        let pruner = AutoPruner {
            max_size_bytes: 10000,
            max_entries: 100,
            ..Default::default()
        };

        let stats = InternalCacheStats {
            total_size: 500,
            entry_count: 10,
        };

        assert!(!should_prune_after_insertion(&pruner, &stats));
    }

    #[test]
    fn test_determine_pruning_strategy_no_pruner() {
        let config = PruningConfig {
            auto_prune_enabled: true,
            use_sync_pruning: false,
            is_test_environment: false,
        };

        assert_eq!(
            determine_pruning_strategy(&config, false, false),
            PruningStrategyType::NoAutoPruner
        );
    }

    #[test]
    fn test_determine_pruning_strategy_sync() {
        let config = PruningConfig {
            auto_prune_enabled: true,
            use_sync_pruning: true,
            is_test_environment: true,
        };

        assert_eq!(
            determine_pruning_strategy(&config, true, true),
            PruningStrategyType::SyncPruning
        );
    }

    #[test]
    fn test_determine_pruning_strategy_background() {
        let config = PruningConfig {
            auto_prune_enabled: true,
            use_sync_pruning: false,
            is_test_environment: false,
        };

        assert_eq!(
            determine_pruning_strategy(&config, true, true),
            PruningStrategyType::BackgroundPruning
        );
    }

    #[test]
    fn test_should_perform_post_insertion_pruning() {
        let config = PruningConfig {
            auto_prune_enabled: true,
            use_sync_pruning: true,
            is_test_environment: true,
        };

        assert!(should_perform_post_insertion_pruning(&config, true));
        assert!(!should_perform_post_insertion_pruning(&config, false));
    }

    #[test]
    fn test_calculate_cache_projections_new_entry() {
        let current = InternalCacheStats {
            total_size: 1000,
            entry_count: 5,
        };

        let projected = calculate_cache_projections(&current, 200, true);
        assert_eq!(projected.projected_size, 1200);
        assert_eq!(projected.projected_count, 6);
    }

    #[test]
    fn test_calculate_cache_projections_existing_entry() {
        let current = InternalCacheStats {
            total_size: 1000,
            entry_count: 5,
        };

        let projected = calculate_cache_projections(&current, 200, false);
        assert_eq!(projected.projected_size, 1200);
        assert_eq!(projected.projected_count, 5);
    }

    #[test]
    fn test_should_prune_based_on_projections() {
        let pruner = AutoPruner {
            max_size_bytes: 1000,
            max_entries: 10,
            ..Default::default()
        };

        let projected = ProjectedCacheStats {
            projected_size: 1500,
            projected_count: 5,
        };

        assert!(should_prune_based_on_projections(&projected, &pruner));

        let projected_count = ProjectedCacheStats {
            projected_size: 500,
            projected_count: 15,
        };

        assert!(should_prune_based_on_projections(&projected_count, &pruner));
    }
}
