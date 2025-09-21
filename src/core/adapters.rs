//! Adapter implementations for trait boundaries

use crate::core::traits::{Cache, CacheStats};
use std::sync::{Arc, Mutex};

/// Adapter for existing AnalysisCache to implement Cache trait
pub struct CacheAdapter {
    inner: Arc<Mutex<crate::core::cache::AnalysisCache>>,
}

impl CacheAdapter {
    /// Create a new cache adapter
    pub fn new(cache: crate::core::cache::AnalysisCache) -> Self {
        Self {
            inner: Arc::new(Mutex::new(cache)),
        }
    }
}

impl Cache for CacheAdapter {
    type Key = String;
    type Value = Vec<u8>;

    fn get(&self, key: &Self::Key) -> Option<Self::Value> {
        // The AnalysisCache stores FileMetrics, so we need to serialize them
        // This is a simplified adapter that treats keys as file paths
        let cache = self.inner.lock().unwrap();

        // Try to get from memory index first (fast path)
        if let Some(entry) = cache.memory_index().get(key) {
            // Serialize the metrics for our generic cache interface
            if let Ok(data) = serde_json::to_vec(&entry.metrics) {
                return Some(data);
            }
        }

        // Fall back to shared cache if not in memory
        if let Ok(data) = cache.shared_cache().get(key, "file_metrics") {
            return Some(data);
        }

        None
    }

    fn set(&mut self, key: Self::Key, value: Self::Value) {
        let mut cache = self.inner.lock().unwrap();

        // Deserialize the value back to FileMetrics if possible
        if let Ok(metrics) = serde_json::from_slice::<crate::core::FileMetrics>(&value) {
            // Create a cache entry with current timestamp
            let entry = crate::core::cache::CacheEntry {
                file_hash: key.clone(),
                timestamp: chrono::Utc::now(),
                metrics,
            };

            // Update memory index (need to clone and replace since we can't get mutable HashMap directly)
            let mut new_index = cache.memory_index().clone();
            new_index.insert(key.clone(), entry);
            *cache.memory_index_mut() = new_index;

            // Store in shared cache
            let _ = cache.shared_cache_mut().put(&key, "file_metrics", &value);
        } else {
            // If it's not FileMetrics, store raw data directly
            let _ = cache.shared_cache_mut().put(&key, "generic", &value);
        }
    }

    fn clear(&mut self) {
        let mut cache = self.inner.lock().unwrap();
        let _ = cache.clear(); // clear() returns Result, ignore for now
    }

    fn stats(&self) -> CacheStats {
        let cache = self.inner.lock().unwrap();
        let cache_stats = cache.stats();
        CacheStats {
            hits: cache_stats.hits,
            misses: cache_stats.misses,
            entries: cache_stats.entries,
            memory_usage: 0, // Would need to calculate actual memory usage
        }
    }
}
