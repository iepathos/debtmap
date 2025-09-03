use crate::cache::shared_cache::SharedCache;
use crate::priority::call_graph::CallGraph;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Cache key for call graph entries
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    /// Hash of all source files included in the call graph
    pub source_hash: String,
    /// Project root path
    pub project_path: PathBuf,
    /// Configuration hash (for settings that affect call graph)
    pub config_hash: String,
}

/// Cached call graph entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The cached call graph
    pub call_graph: CallGraph,
    /// Framework exclusions
    pub framework_exclusions: Vec<crate::priority::call_graph::FunctionId>,
    /// Function pointer used functions
    pub function_pointer_used: Vec<crate::priority::call_graph::FunctionId>,
    /// Timestamp when cached
    pub timestamp: SystemTime,
    /// Source files included in the cache
    pub source_files: Vec<PathBuf>,
}

/// Call graph cache manager
pub struct CallGraphCache {
    /// Shared cache backend
    shared_cache: SharedCache,
    /// In-memory cache for current session
    memory_cache: HashMap<CacheKey, CacheEntry>,
    /// Maximum cache age in seconds
    max_age: u64,
    /// Legacy cache directory path (for migration)
    _legacy_cache_dir: Option<PathBuf>,
}

impl CallGraphCache {
    /// Create a new cache manager
    pub fn new() -> Result<Self> {
        // Create shared cache instance
        let shared_cache = SharedCache::new(None)?;

        // Check for legacy cache directory
        let legacy_cache_dir = Self::get_legacy_cache_dir();

        // Perform migration if needed
        if let Some(ref legacy_dir) = legacy_cache_dir {
            if legacy_dir.exists() {
                log::info!("Migrating cache from legacy location: {:?}", legacy_dir);
                if let Err(e) = shared_cache.migrate_from_local(legacy_dir) {
                    log::warn!("Failed to migrate legacy cache: {}", e);
                }
            }
        }

        Ok(Self {
            shared_cache,
            memory_cache: HashMap::new(),
            max_age: 3600 * 24, // 24 hours by default
            _legacy_cache_dir: legacy_cache_dir,
        })
    }

    /// Get the legacy cache directory path (for migration)
    fn get_legacy_cache_dir() -> Option<PathBuf> {
        // Check for .debtmap_cache in current directory
        let local_cache = PathBuf::from(".debtmap_cache");
        if local_cache.exists() {
            return Some(local_cache);
        }

        // Check for old XDG location
        if let Some(cache_dir) = dirs::cache_dir() {
            let old_cache = cache_dir.join("debtmap").join("call_graphs");
            if old_cache.exists() {
                return Some(old_cache);
            }
        }

        None
    }

    /// Generate a cache key for the given project and files
    pub fn generate_key(
        project_path: &Path,
        source_files: &[PathBuf],
        config: &crate::config::DebtmapConfig,
    ) -> Result<CacheKey> {
        // Hash all source file contents
        let mut hasher = Sha256::new();

        // Sort files for deterministic hashing
        let mut sorted_files = source_files.to_vec();
        sorted_files.sort();

        for file in &sorted_files {
            if let Ok(content) = fs::read_to_string(file) {
                hasher.update(content.as_bytes());
                hasher.update(b"\n");
            }
        }

        let source_hash = format!("{:x}", hasher.finalize());

        // Hash configuration
        let config_str = serde_json::to_string(config)?;
        let mut config_hasher = Sha256::new();
        config_hasher.update(config_str.as_bytes());
        let config_hash = format!("{:x}", config_hasher.finalize());

        Ok(CacheKey {
            source_hash,
            project_path: project_path.to_path_buf(),
            config_hash,
        })
    }

    /// Get cached call graph if available and valid
    pub fn get(
        &mut self,
        key: &CacheKey,
    ) -> Option<(
        CallGraph,
        Vec<crate::priority::call_graph::FunctionId>,
        Vec<crate::priority::call_graph::FunctionId>,
    )> {
        // Check memory cache first
        if let Some(entry) = self.memory_cache.get(key) {
            if self.is_valid_entry(entry) {
                log::info!("Using in-memory cached call graph");
                return Some((
                    entry.call_graph.clone(),
                    entry.framework_exclusions.clone(),
                    entry.function_pointer_used.clone(),
                ));
            }
        }

        // Check shared cache
        let cache_key = self.generate_cache_key(key);
        if let Ok(data) = self.shared_cache.get(&cache_key, "call_graphs") {
            if let Ok(entry) = serde_json::from_slice::<CacheEntry>(&data) {
                if self.is_valid_entry(&entry) {
                    log::info!("Using shared cached call graph");
                    // Store in memory cache for faster access
                    self.memory_cache.insert(key.clone(), entry.clone());
                    return Some((
                        entry.call_graph,
                        entry.framework_exclusions,
                        entry.function_pointer_used,
                    ));
                }
            }
        }

        None
    }

    /// Store call graph in cache
    pub fn put(
        &mut self,
        key: CacheKey,
        call_graph: CallGraph,
        framework_exclusions: Vec<crate::priority::call_graph::FunctionId>,
        function_pointer_used: Vec<crate::priority::call_graph::FunctionId>,
        source_files: Vec<PathBuf>,
    ) -> Result<()> {
        let entry = CacheEntry {
            call_graph,
            framework_exclusions,
            function_pointer_used,
            timestamp: SystemTime::now(),
            source_files,
        };

        // Store in memory cache
        self.memory_cache.insert(key.clone(), entry.clone());

        // Store in shared cache
        let cache_key = self.generate_cache_key(&key);
        let data = serde_json::to_vec(&entry).context("Failed to serialize cache entry")?;
        self.shared_cache.put(&cache_key, "call_graphs", &data)?;

        Ok(())
    }

    /// Check if a cache entry is still valid
    fn is_valid_entry(&self, entry: &CacheEntry) -> bool {
        // Check age
        if let Ok(elapsed) = entry.timestamp.elapsed() {
            if elapsed.as_secs() > self.max_age {
                return false;
            }
        }

        // Check if all source files still exist and haven't been modified
        for file in &entry.source_files {
            if !file.exists() {
                return false;
            }

            // Check modification time
            if let Ok(metadata) = fs::metadata(file) {
                if let Ok(modified) = metadata.modified() {
                    if modified > entry.timestamp {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Generate a string cache key from the CacheKey struct
    fn generate_cache_key(&self, key: &CacheKey) -> String {
        // Use source hash as the primary key component
        // Truncate to 32 chars for reasonable length
        key.source_hash[..32.min(key.source_hash.len())].to_string()
    }

    /// Clear all cached entries
    pub fn clear(&mut self) -> Result<()> {
        // Clear memory cache
        self.memory_cache.clear();

        // Clear shared cache by component
        // Note: This only clears call_graphs component
        log::info!("Clearing call graph cache");
        self.shared_cache.cleanup()?;

        Ok(())
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> Result<crate::cache::CacheStats> {
        Ok(self.shared_cache.get_stats())
    }
}

// Add dirs dependency helper module for cache directory
mod dirs {
    use std::path::PathBuf;

    pub fn cache_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            home_dir().map(|h| h.join("Library").join("Caches"))
        }

        #[cfg(target_os = "linux")]
        {
            std::env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .or_else(|| home_dir().map(|h| h.join(".cache")))
        }

        #[cfg(target_os = "windows")]
        {
            std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }

    fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
}
