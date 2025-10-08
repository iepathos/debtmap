//! Pruning decision and execution logic for cache management

pub mod decision;

pub use decision::{
    calculate_cache_projections, determine_pruning_config, determine_pruning_strategy,
    is_existing_entry, should_perform_post_insertion_pruning, should_prune_after_insertion,
    should_prune_based_on_projections, InternalCacheStats, ProjectedCacheStats, PruningConfig,
    PruningStrategyType,
};
