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
        // Since AnalysisCache doesn't have a direct get method,
        // this adapter just demonstrates the pattern
        // In a real implementation, we'd need to refactor the cache
        None
    }

    fn set(&mut self, key: Self::Key, value: Self::Value) {
        // For now, skip implementation as it would require more refactoring
        // This demonstrates the adapter pattern
        let _ = (key, value);
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
