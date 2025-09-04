use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

use super::shared_cache::{CacheIndex, CacheMetadata};

/// Pruning strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PruneStrategy {
    /// Least recently used
    Lru,
    /// Least frequently used
    Lfu,
    /// First in, first out
    Fifo,
    /// Only remove old entries
    AgeBasedOnly,
}

/// Environment variable snapshot for configuration resolution
#[derive(Debug, Clone, Default)]
pub struct EnvironmentSnapshot {
    /// Map of environment variable names to values
    pub vars: HashMap<String, String>,
}

impl EnvironmentSnapshot {
    /// Create snapshot of current environment
    pub fn from_current_env() -> Self {
        let mut vars = HashMap::new();
        for (key, value) in std::env::vars() {
            if key.starts_with("DEBTMAP_CACHE_") {
                vars.insert(key, value);
            }
        }
        Self { vars }
    }

    /// Get environment variable value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(|s| s.as_str())
    }

    /// Check if environment variable is set to "true"
    pub fn is_true(&self, key: &str) -> bool {
        self.get(key).unwrap_or("").to_lowercase() == "true"
    }

    /// Parse environment variable as type T
    pub fn parse<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.get(key)?.parse().ok()
    }
}

/// Cache configuration derived from environment or explicit settings
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Whether auto-pruning is enabled
    pub auto_prune_enabled: bool,
    /// Whether to use synchronous pruning (for tests)
    pub use_sync_pruning: bool,
    /// Maximum cache size in bytes
    pub max_cache_size: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            auto_prune_enabled: true,
            use_sync_pruning: cfg!(test),
            max_cache_size: 1024 * 1024 * 1024, // 1GB
        }
    }
}

/// Auto-pruner configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPruner {
    /// Maximum total cache size in bytes
    pub max_size_bytes: usize,
    /// Maximum age for cache entries in days
    pub max_age_days: i64,
    /// Maximum number of cache entries
    pub max_entries: usize,
    /// Percentage of entries to remove when limit is hit (0.0 to 1.0)
    pub prune_percentage: f32,
    /// Pruning strategy to use
    pub strategy: PruneStrategy,
}

impl Default for AutoPruner {
    fn default() -> Self {
        Self {
            max_size_bytes: 1024 * 1024 * 1024, // 1GB default
            max_age_days: 30,                   // 30 days default
            max_entries: 10000,                 // 10k entries default
            prune_percentage: 0.25,             // Remove 25% when pruning
            strategy: PruneStrategy::Lru,
        }
    }
}

impl AutoPruner {
    /// Create cache configuration from environment snapshot (pure function)
    pub fn create_cache_config(env: &EnvironmentSnapshot) -> CacheConfig {
        let auto_prune_enabled = env.is_true("DEBTMAP_CACHE_AUTO_PRUNE") 
            || env.get("DEBTMAP_CACHE_AUTO_PRUNE").is_none(); // Default to true if not set
        
        let use_sync_pruning = env.is_true("DEBTMAP_CACHE_SYNC_PRUNE") || cfg!(test);
        
        let max_cache_size = if auto_prune_enabled {
            let pruner = Self::from_env_snapshot(env);
            pruner.max_size_bytes as u64
        } else {
            1024 * 1024 * 1024 // 1GB default
        };

        CacheConfig {
            auto_prune_enabled,
            use_sync_pruning,
            max_cache_size,
        }
    }

    /// Create cache configuration from current environment (for backward compatibility)
    pub fn create_cache_config_from_env() -> CacheConfig {
        let env = EnvironmentSnapshot::from_current_env();
        Self::create_cache_config(&env)
    }

    /// Check if cache size exceeds the configured limit
    fn exceeds_size_limit(total_size: u64, max_size_bytes: usize) -> bool {
        total_size > max_size_bytes as u64
    }

    /// Check if entry count exceeds the configured limit
    fn exceeds_entry_limit(entry_count: usize, max_entries: usize) -> bool {
        entry_count > max_entries
    }

    /// Check if enough time has passed since last cleanup to warrant age-based checks
    fn should_check_for_old_entries(
        last_cleanup: Option<SystemTime>, 
        now: SystemTime
    ) -> bool {
        match last_cleanup {
            Some(cleanup_time) => {
                now.duration_since(cleanup_time)
                    .map(|elapsed| elapsed > Duration::from_secs(86400))
                    .unwrap_or(false)
            }
            None => true, // No cleanup record, should check
        }
    }

    /// Check if any entries exceed the maximum age
    fn has_old_entries(
        entries: &std::collections::HashMap<String, CacheMetadata>,
        max_age_days: i64,
        now: SystemTime
    ) -> bool {
        let max_age = Duration::from_secs(max_age_days as u64 * 86400);
        
        entries.values().any(|metadata| {
            now.duration_since(metadata.last_accessed)
                .map(|age| age > max_age)
                .unwrap_or(false)
        })
    }

    /// Create pruner from environment variables (for backward compatibility)
    pub fn from_env() -> Self {
        let env = EnvironmentSnapshot::from_current_env();
        Self::from_env_snapshot(&env)
    }

    /// Create pruner from environment snapshot (pure function)
    pub fn from_env_snapshot(env: &EnvironmentSnapshot) -> Self {
        let mut pruner = Self::default();

        if let Some(size) = env.parse::<usize>("DEBTMAP_CACHE_MAX_SIZE") {
            pruner.max_size_bytes = size;
        }

        if let Some(days) = env.parse::<i64>("DEBTMAP_CACHE_MAX_AGE_DAYS") {
            pruner.max_age_days = days;
        }

        if let Some(entries) = env.parse::<usize>("DEBTMAP_CACHE_MAX_ENTRIES") {
            pruner.max_entries = entries;
        }

        if let Some(percentage) = env.parse::<f32>("DEBTMAP_CACHE_PRUNE_PERCENTAGE") {
            pruner.prune_percentage = percentage.clamp(0.1, 0.9);
        }

        if let Some(strategy) = env.get("DEBTMAP_CACHE_STRATEGY") {
            pruner.strategy = match strategy.to_lowercase().as_str() {
                "lru" => PruneStrategy::Lru,
                "lfu" => PruneStrategy::Lfu,
                "fifo" => PruneStrategy::Fifo,
                "age" | "age_based" => PruneStrategy::AgeBasedOnly,
                _ => PruneStrategy::Lru,
            };
        }

        pruner
    }

    /// Check if pruning is needed based on current index
    pub fn should_prune(&self, index: &CacheIndex) -> bool {
        let now = SystemTime::now();
        
        // Check size limit - immediate pruning needed
        if Self::exceeds_size_limit(index.total_size, self.max_size_bytes) {
            return true;
        }

        // Check entry count limit - immediate pruning needed
        if Self::exceeds_entry_limit(index.entries.len(), self.max_entries) {
            return true;
        }

        // Check age-based pruning only if enough time has passed since last cleanup
        // and there are actually entries that might be old
        if !index.entries.is_empty() 
            && Self::should_check_for_old_entries(index.last_cleanup, now)
            && Self::has_old_entries(&index.entries, self.max_age_days, now) 
        {
            return true;
        }

        false
    }

    /// Calculate entries to remove based on strategy
    pub fn calculate_entries_to_remove(&self, index: &CacheIndex) -> Vec<(String, CacheMetadata)> {
        match self.strategy {
            PruneStrategy::Lru => self.prune_by_lru(index),
            PruneStrategy::Lfu => self.prune_by_lfu(index),
            PruneStrategy::Fifo => self.prune_by_fifo(index),
            PruneStrategy::AgeBasedOnly => self.prune_by_age_only(index),
        }
    }

    /// Prune using Least Recently Used strategy
    fn prune_by_lru(&self, index: &CacheIndex) -> Vec<(String, CacheMetadata)> {
        let mut entries: Vec<(String, CacheMetadata)> = index
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Sort by last accessed time (oldest first)
        entries.sort_by_key(|(_, metadata)| metadata.last_accessed);

        self.select_entries_to_remove(entries, index)
    }

    /// Prune using Least Frequently Used strategy
    fn prune_by_lfu(&self, index: &CacheIndex) -> Vec<(String, CacheMetadata)> {
        let mut entries: Vec<(String, CacheMetadata)> = index
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Sort by access count (least accessed first)
        entries.sort_by_key(|(_, metadata)| metadata.access_count);

        self.select_entries_to_remove(entries, index)
    }

    /// Prune using First In First Out strategy
    fn prune_by_fifo(&self, index: &CacheIndex) -> Vec<(String, CacheMetadata)> {
        let mut entries: Vec<(String, CacheMetadata)> = index
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Sort by creation time (oldest first)
        entries.sort_by_key(|(_, metadata)| metadata.created_at);

        self.select_entries_to_remove(entries, index)
    }

    /// Prune only old entries
    fn prune_by_age_only(&self, index: &CacheIndex) -> Vec<(String, CacheMetadata)> {
        let max_age = Duration::from_secs(self.max_age_days as u64 * 86400);
        let now = SystemTime::now();
        let mut entries_to_remove = Vec::new();

        for (key, metadata) in &index.entries {
            if let Ok(age) = now.duration_since(metadata.last_accessed) {
                if age > max_age {
                    entries_to_remove.push((key.clone(), metadata.clone()));
                }
            }
        }

        entries_to_remove
    }

    /// Calculate size removal target based on current size and limits
    fn calculate_size_removal_target(
        current_size: u64,
        max_size_bytes: usize,
        prune_percentage: f32,
    ) -> u64 {
        if current_size <= max_size_bytes as u64 {
            return 0;
        }

        let excess = current_size - max_size_bytes as u64;
        let buffer_amount = (max_size_bytes as f32 * prune_percentage) as u64;
        excess + buffer_amount
    }

    /// Calculate count removal target based on current count and limits
    fn calculate_count_removal_target(
        current_count: usize,
        max_entries: usize,
        prune_percentage: f32,
    ) -> usize {
        if current_count <= max_entries {
            return 0;
        }

        let excess = current_count - max_entries;
        let buffer_amount = (max_entries as f32 * prune_percentage) as usize;
        excess + buffer_amount
    }

    /// Check if an entry is old enough to be removed based on age policy
    fn is_entry_old_enough(
        metadata: &CacheMetadata,
        max_age_days: i64,
        now: SystemTime,
    ) -> bool {
        let max_age = Duration::from_secs(max_age_days as u64 * 86400);
        if let Ok(age) = now.duration_since(metadata.last_accessed) {
            age > max_age
        } else {
            false
        }
    }

    /// Determine if we should continue removing entries based on targets and age
    fn should_continue_removing(
        removed_count: usize,
        removed_size: u64,
        target_count: usize,
        target_size: u64,
        metadata: &CacheMetadata,
        max_age_days: i64,
        now: SystemTime,
    ) -> bool {
        // Continue removing if we haven't satisfied EITHER target (both need to be met)
        let count_satisfied = removed_count >= target_count;
        let size_satisfied = removed_size >= target_size;
        
        // Continue if either target is not yet satisfied
        if !count_satisfied || !size_satisfied {
            return true;
        }

        // Continue if the entry is old enough to remove anyway
        Self::is_entry_old_enough(metadata, max_age_days, now)
    }

    /// Select entries to remove using functional approach with proper state management
    fn select_entries_functionally(
        sorted_entries: &[(String, CacheMetadata)],
        target_count: usize,
        target_size: u64,
        max_age_days: i64,
        now: SystemTime,
    ) -> Vec<(String, CacheMetadata)> {
        let mut entries_to_remove = Vec::new();
        let mut removed_count = 0;
        let mut removed_size = 0u64;

        for (key, metadata) in sorted_entries {
            let should_continue = Self::should_continue_removing(
                removed_count,
                removed_size,
                target_count,
                target_size,
                metadata,
                max_age_days,
                now,
            );

            if !should_continue {
                break;
            }

            entries_to_remove.push((key.clone(), metadata.clone()));
            removed_count += 1;
            removed_size += metadata.size_bytes;
        }

        entries_to_remove
    }

    /// Select entries to remove based on limits
    fn select_entries_to_remove(
        &self,
        sorted_entries: Vec<(String, CacheMetadata)>,
        index: &CacheIndex,
    ) -> Vec<(String, CacheMetadata)> {
        let target_size = Self::calculate_size_removal_target(
            index.total_size,
            self.max_size_bytes,
            self.prune_percentage,
        );

        let target_count = Self::calculate_count_removal_target(
            index.entries.len(),
            self.max_entries,
            self.prune_percentage,
        );

        let now = SystemTime::now();

        Self::select_entries_functionally(
            &sorted_entries,
            target_count,
            target_size,
            self.max_age_days,
            now,
        )
    }
}

/// Statistics from a pruning operation
#[derive(Debug, Clone)]
pub struct PruneStats {
    pub entries_removed: usize,
    pub bytes_freed: u64,
    pub entries_remaining: usize,
    pub bytes_remaining: u64,
    pub duration_ms: u64,
    pub files_deleted: usize,
    pub files_not_found: usize,
}

impl std::fmt::Display for PruneStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pruned {} entries ({} MB) in {}ms. Remaining: {} entries ({} MB)",
            self.entries_removed,
            self.bytes_freed / (1024 * 1024),
            self.duration_ms,
            self.entries_remaining,
            self.bytes_remaining / (1024 * 1024)
        )
    }
}

/// Background pruner for non-blocking operations
pub struct BackgroundPruner {
    pruner: Arc<AutoPruner>,
    running: Arc<Mutex<bool>>,
    last_stats: Arc<Mutex<Option<PruneStats>>>,
}

impl BackgroundPruner {
    /// Create a new background pruner
    pub fn new(pruner: AutoPruner) -> Self {
        Self {
            pruner: Arc::new(pruner),
            running: Arc::new(Mutex::new(false)),
            last_stats: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if pruning is currently running
    pub fn is_running(&self) -> bool {
        self.running.lock().map(|r| *r).unwrap_or(false)
    }

    /// Get the last pruning statistics
    pub fn get_last_stats(&self) -> Option<PruneStats> {
        self.last_stats.lock().ok()?.clone()
    }

    /// Start background pruning if not already running
    pub fn start_if_needed(&self, index: Arc<RwLock<CacheIndex>>) -> bool {
        // Try to acquire the running lock
        let mut running = match self.running.try_lock() {
            Ok(r) => r,
            Err(_) => return false, // Already running
        };

        if *running {
            return false;
        }

        // Check if pruning is needed
        let should_prune = {
            match index.read() {
                Ok(idx) => self.pruner.should_prune(&idx),
                Err(_) => return false,
            }
        };

        if !should_prune {
            return false;
        }

        *running = true;

        // Clone necessary data for the thread
        let pruner = self.pruner.clone();
        let index_clone = index.clone();
        let running_flag = self.running.clone();
        let stats_store = self.last_stats.clone();

        // Spawn background thread for pruning
        thread::spawn(move || {
            let stats = Self::perform_pruning_thread(pruner, index_clone);

            // Store stats and mark as not running
            if let Ok(mut last_stats) = stats_store.lock() {
                *last_stats = stats.clone();
            }

            if let Ok(mut r) = running_flag.lock() {
                *r = false;
            }

            if let Some(ref s) = stats {
                log::info!("Background pruning completed: {}", s);
            }
        });

        true
    }

    /// Perform pruning synchronously (for testing or immediate needs)
    pub fn prune_sync(&self, index: Arc<RwLock<CacheIndex>>) -> Option<PruneStats> {
        let mut running = match self.running.try_lock() {
            Ok(r) => r,
            Err(_) => return None, // Already running
        };

        if *running {
            return None;
        }

        *running = true;
        let stats = Self::perform_pruning_thread(self.pruner.clone(), index);
        *running = false;

        if let Ok(mut last_stats) = self.last_stats.lock() {
            *last_stats = stats.clone();
        }

        stats
    }

    /// Perform the actual pruning in a thread context
    fn perform_pruning_thread(
        pruner: Arc<AutoPruner>,
        index: Arc<RwLock<CacheIndex>>,
    ) -> Option<PruneStats> {
        let start = SystemTime::now();

        // Get entries to remove
        let entries_to_remove = {
            let idx = index.read().ok()?;
            pruner.calculate_entries_to_remove(&idx)
        };

        if entries_to_remove.is_empty() {
            return None;
        }

        let mut bytes_freed = 0u64;
        let files_deleted = 0usize;
        let files_not_found = 0usize;

        // Remove from index
        {
            let mut idx = index.write().ok()?;
            for (key, metadata) in &entries_to_remove {
                if idx.entries.remove(key).is_some() {
                    bytes_freed += metadata.size_bytes;
                }
            }
            idx.total_size = idx.entries.values().map(|m| m.size_bytes).sum();
            idx.last_cleanup = Some(SystemTime::now());
        }

        // Note: File deletion would be handled by SharedCache
        // This is just the pruning logic

        let duration = start.elapsed().ok()?.as_millis() as u64;

        let final_stats = {
            let idx = index.read().ok()?;
            PruneStats {
                entries_removed: entries_to_remove.len(),
                bytes_freed,
                entries_remaining: idx.entries.len(),
                bytes_remaining: idx.total_size,
                duration_ms: duration,
                files_deleted,
                files_not_found,
            }
        };

        Some(final_stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_pruner_defaults() {
        let pruner = AutoPruner::default();
        assert_eq!(pruner.max_size_bytes, 1024 * 1024 * 1024);
        assert_eq!(pruner.max_age_days, 30);
        assert_eq!(pruner.max_entries, 10000);
    }

    #[test]
    fn test_calculate_size_removal_target() {
        // No removal needed when under limit
        assert_eq!(
            AutoPruner::calculate_size_removal_target(500, 1000, 0.25),
            0
        );

        // Calculate removal with buffer when over limit
        let target = AutoPruner::calculate_size_removal_target(1500, 1000, 0.25);
        // Excess: 1500 - 1000 = 500
        // Buffer: 1000 * 0.25 = 250
        // Total: 500 + 250 = 750
        assert_eq!(target, 750);
    }

    #[test]
    fn test_calculate_count_removal_target() {
        // No removal needed when under limit
        assert_eq!(
            AutoPruner::calculate_count_removal_target(3, 5, 0.25),
            0
        );

        // Calculate removal with buffer when over limit
        let target = AutoPruner::calculate_count_removal_target(8, 5, 0.25);
        // Excess: 8 - 5 = 3
        // Buffer: 5 * 0.25 = 1.25 -> 1 (usize)
        // Total: 3 + 1 = 4
        assert_eq!(target, 4);
    }

    #[test]
    fn test_is_entry_old_enough() {
        let now = SystemTime::now();
        let old_time = now - Duration::from_secs(2 * 86400); // 2 days ago
        let recent_time = now - Duration::from_secs(3600); // 1 hour ago

        let old_metadata = CacheMetadata {
            version: "0.1.0".to_string(),
            created_at: old_time,
            last_accessed: old_time,
            access_count: 1,
            size_bytes: 100,
        };

        let recent_metadata = CacheMetadata {
            version: "0.1.0".to_string(),
            created_at: recent_time,
            last_accessed: recent_time,
            access_count: 1,
            size_bytes: 100,
        };

        // Entry older than 1 day should be considered old
        assert!(AutoPruner::is_entry_old_enough(&old_metadata, 1, now));
        // Entry newer than 1 day should not be considered old
        assert!(!AutoPruner::is_entry_old_enough(&recent_metadata, 1, now));
    }

    #[test]
    fn test_select_entries_to_remove_count_limit() {
        let pruner = AutoPruner {
            max_entries: 5,
            max_size_bytes: 10000, // Large size limit so it doesn't interfere
            max_age_days: 30,
            prune_percentage: 0.25,
            strategy: PruneStrategy::Lru,
        };

        let now = SystemTime::now();
        let mut index = CacheIndex::default();
        let mut sorted_entries = Vec::new();

        // Create 8 entries, each 100 bytes
        for i in 0..8 {
            let key = format!("entry_{}", i);
            let metadata = CacheMetadata {
                version: "0.1.0".to_string(),
                created_at: now,
                last_accessed: now,
                access_count: 1,
                size_bytes: 100,
            };
            
            index.entries.insert(key.clone(), metadata.clone());
            sorted_entries.push((key, metadata));
        }
        index.total_size = 800; // 8 * 100

        let entries_to_remove = pruner.select_entries_to_remove(sorted_entries, &index);

        // We have 8 entries, max is 5, so excess is 3
        // Buffer is 5 * 0.25 = 1.25 -> 1
        // Total target: 3 + 1 = 4 entries to remove
        assert_eq!(
            entries_to_remove.len(),
            4,
            "Should remove 4 entries (3 excess + 1 buffer)"
        );
    }

    #[test]
    fn test_auto_pruner_from_env() {
        std::env::set_var("DEBTMAP_CACHE_MAX_SIZE", "524288000");
        std::env::set_var("DEBTMAP_CACHE_MAX_AGE_DAYS", "7");
        std::env::set_var("DEBTMAP_CACHE_STRATEGY", "lfu");

        let pruner = AutoPruner::from_env();
        assert_eq!(pruner.max_size_bytes, 524288000);
        assert_eq!(pruner.max_age_days, 7);
        assert!(matches!(pruner.strategy, PruneStrategy::Lfu));

        // Clean up
        std::env::remove_var("DEBTMAP_CACHE_MAX_SIZE");
        std::env::remove_var("DEBTMAP_CACHE_MAX_AGE_DAYS");
        std::env::remove_var("DEBTMAP_CACHE_STRATEGY");
    }

    #[test]
    fn test_should_prune_size_limit() {
        let pruner = AutoPruner {
            max_size_bytes: 1000,
            ..Default::default()
        };

        let mut index = CacheIndex::default();
        index.total_size = 500;
        assert!(!pruner.should_prune(&index));

        index.total_size = 1500;
        assert!(pruner.should_prune(&index));
    }

    #[test]
    fn test_should_prune_entry_limit() {
        let pruner = AutoPruner {
            max_entries: 2,
            ..Default::default()
        };

        let mut index = CacheIndex::default();
        index.entries.insert(
            "key1".to_string(),
            CacheMetadata {
                version: "0.1.0".to_string(),
                created_at: SystemTime::now(),
                last_accessed: SystemTime::now(),
                access_count: 1,
                size_bytes: 100,
            },
        );

        assert!(!pruner.should_prune(&index));

        index.entries.insert(
            "key2".to_string(),
            CacheMetadata {
                version: "0.1.0".to_string(),
                created_at: SystemTime::now(),
                last_accessed: SystemTime::now(),
                access_count: 1,
                size_bytes: 100,
            },
        );
        index.entries.insert(
            "key3".to_string(),
            CacheMetadata {
                version: "0.1.0".to_string(),
                created_at: SystemTime::now(),
                last_accessed: SystemTime::now(),
                access_count: 1,
                size_bytes: 100,
            },
        );

        assert!(pruner.should_prune(&index));
    }

    #[test]
    fn test_exceeds_size_limit() {
        assert!(!AutoPruner::exceeds_size_limit(500, 1000));
        assert!(!AutoPruner::exceeds_size_limit(1000, 1000));
        assert!(AutoPruner::exceeds_size_limit(1001, 1000));
    }

    #[test]
    fn test_exceeds_entry_limit() {
        assert!(!AutoPruner::exceeds_entry_limit(3, 5));
        assert!(!AutoPruner::exceeds_entry_limit(5, 5));
        assert!(AutoPruner::exceeds_entry_limit(6, 5));
    }

    #[test]
    fn test_should_check_for_old_entries() {
        let now = SystemTime::now();
        
        // No cleanup record - should always check
        assert!(AutoPruner::should_check_for_old_entries(None, now));
        
        // Recent cleanup - should not check yet
        let recent = now - Duration::from_secs(3600); // 1 hour ago
        assert!(!AutoPruner::should_check_for_old_entries(Some(recent), now));
        
        // Old cleanup - should check
        let old = now - Duration::from_secs(2 * 86400); // 2 days ago
        assert!(AutoPruner::should_check_for_old_entries(Some(old), now));
    }

    #[test]
    fn test_has_old_entries() {
        let now = SystemTime::now();
        let mut entries = std::collections::HashMap::new();
        
        // No entries
        assert!(!AutoPruner::has_old_entries(&entries, 1, now));
        
        // Only recent entries
        let recent_metadata = CacheMetadata {
            version: "0.1.0".to_string(),
            created_at: now,
            last_accessed: now - Duration::from_secs(3600), // 1 hour ago
            access_count: 1,
            size_bytes: 100,
        };
        entries.insert("recent".to_string(), recent_metadata);
        assert!(!AutoPruner::has_old_entries(&entries, 1, now));
        
        // Add old entry
        let old_metadata = CacheMetadata {
            version: "0.1.0".to_string(),
            created_at: now,
            last_accessed: now - Duration::from_secs(2 * 86400), // 2 days ago
            access_count: 1,
            size_bytes: 100,
        };
        entries.insert("old".to_string(), old_metadata);
        assert!(AutoPruner::has_old_entries(&entries, 1, now));
    }

    #[test]
    fn test_should_prune_age_based() {
        let pruner = AutoPruner {
            max_age_days: 1, // 1 day
            max_size_bytes: 10000, // Large limits
            max_entries: 1000,
            ..Default::default()
        };

        let now = SystemTime::now();
        let mut index = CacheIndex::default();
        
        // Add old entry
        let old_metadata = CacheMetadata {
            version: "0.1.0".to_string(),
            created_at: now,
            last_accessed: now - Duration::from_secs(2 * 86400), // 2 days ago
            access_count: 1,
            size_bytes: 100,
        };
        index.entries.insert("old_key".to_string(), old_metadata);
        index.total_size = 100;
        
        // No cleanup record, should prune due to old entries
        assert!(pruner.should_prune(&index));
        
        // Recent cleanup, should not prune yet
        index.last_cleanup = Some(now - Duration::from_secs(3600)); // 1 hour ago
        assert!(!pruner.should_prune(&index));
        
        // Old cleanup, should prune due to old entries
        index.last_cleanup = Some(now - Duration::from_secs(2 * 86400)); // 2 days ago  
        assert!(pruner.should_prune(&index));
    }
}
