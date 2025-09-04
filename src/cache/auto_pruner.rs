use serde::{Deserialize, Serialize};
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
    /// Create pruner from environment variables
    pub fn from_env() -> Self {
        let mut pruner = Self::default();

        if let Ok(size) = std::env::var("DEBTMAP_CACHE_MAX_SIZE") {
            if let Ok(bytes) = size.parse::<usize>() {
                pruner.max_size_bytes = bytes;
            }
        }

        if let Ok(days) = std::env::var("DEBTMAP_CACHE_MAX_AGE_DAYS") {
            if let Ok(days) = days.parse::<i64>() {
                pruner.max_age_days = days;
            }
        }

        if let Ok(entries) = std::env::var("DEBTMAP_CACHE_MAX_ENTRIES") {
            if let Ok(count) = entries.parse::<usize>() {
                pruner.max_entries = count;
            }
        }

        if let Ok(percentage) = std::env::var("DEBTMAP_CACHE_PRUNE_PERCENTAGE") {
            if let Ok(pct) = percentage.parse::<f32>() {
                pruner.prune_percentage = pct.clamp(0.1, 0.9);
            }
        }

        if let Ok(strategy) = std::env::var("DEBTMAP_CACHE_STRATEGY") {
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
        // Check size limit
        if index.total_size > self.max_size_bytes as u64 {
            return true;
        }

        // Check entry count limit
        if index.entries.len() > self.max_entries {
            return true;
        }

        // Check if we need age-based pruning (check every day)
        if let Some(last_cleanup) = index.last_cleanup {
            if let Ok(elapsed) = SystemTime::now().duration_since(last_cleanup) {
                if elapsed > Duration::from_secs(86400) {
                    // Check if there are old entries
                    let max_age = Duration::from_secs(self.max_age_days as u64 * 86400);
                    let now = SystemTime::now();
                    for metadata in index.entries.values() {
                        if let Ok(age) = now.duration_since(metadata.last_accessed) {
                            if age > max_age {
                                return true;
                            }
                        }
                    }
                }
            }
        } else if !index.entries.is_empty() {
            // No cleanup record but has entries - check if any are old
            let max_age = Duration::from_secs(self.max_age_days as u64 * 86400);
            let now = SystemTime::now();
            for metadata in index.entries.values() {
                if let Ok(age) = now.duration_since(metadata.last_accessed) {
                    if age > max_age {
                        return true;
                    }
                }
            }
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

    /// Select entries to remove based on limits
    fn select_entries_to_remove(
        &self,
        sorted_entries: Vec<(String, CacheMetadata)>,
        index: &CacheIndex,
    ) -> Vec<(String, CacheMetadata)> {
        let mut entries_to_remove = Vec::new();
        let mut removed_size = 0u64;
        let mut removed_count = 0usize;

        // Calculate targets
        let target_size = if index.total_size > self.max_size_bytes as u64 {
            let excess = index.total_size - self.max_size_bytes as u64;
            let prune_amount = (self.max_size_bytes as f32 * self.prune_percentage) as u64;
            excess.max(prune_amount)
        } else {
            0
        };

        let target_count = if index.entries.len() > self.max_entries {
            let excess = index.entries.len() - self.max_entries;
            let prune_amount = (self.max_entries as f32 * self.prune_percentage) as usize;
            excess.max(prune_amount)
        } else {
            0
        };

        // Remove old entries first
        let max_age = Duration::from_secs(self.max_age_days as u64 * 86400);
        let now = SystemTime::now();

        for (key, metadata) in &sorted_entries {
            // Check if we've removed enough
            if removed_size >= target_size && removed_count >= target_count {
                // Also check if this entry is old enough to remove anyway
                if let Ok(age) = now.duration_since(metadata.last_accessed) {
                    if age <= max_age {
                        break;
                    }
                }
            }

            entries_to_remove.push((key.clone(), metadata.clone()));
            removed_size += metadata.size_bytes;
            removed_count += 1;
        }

        entries_to_remove
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
                Ok(idx) => self.pruner.should_prune(&*idx),
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
    fn perform_pruning_thread(pruner: Arc<AutoPruner>, index: Arc<RwLock<CacheIndex>>) -> Option<PruneStats> {
        let start = SystemTime::now();

        // Get entries to remove
        let entries_to_remove = {
            let idx = index.read().ok()?;
            pruner.calculate_entries_to_remove(&*idx)
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
}
