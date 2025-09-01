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
    /// Cache directory path
    cache_dir: PathBuf,
    /// In-memory cache for current session
    memory_cache: HashMap<CacheKey, CacheEntry>,
    /// Maximum cache age in seconds
    max_age: u64,
}

impl CallGraphCache {
    /// Create a new cache manager
    pub fn new() -> Result<Self> {
        let cache_dir = Self::get_cache_dir()?;
        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", cache_dir))?;

        Ok(Self {
            cache_dir,
            memory_cache: HashMap::new(),
            max_age: 3600 * 24, // 24 hours by default
        })
    }

    /// Get the cache directory path
    fn get_cache_dir() -> Result<PathBuf> {
        // Try to use XDG cache directory first, fall back to temp
        if let Some(cache_dir) = dirs::cache_dir() {
            Ok(cache_dir.join("debtmap").join("call_graphs"))
        } else {
            Ok(std::env::temp_dir()
                .join("debtmap_cache")
                .join("call_graphs"))
        }
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

        // Check disk cache
        if let Ok(entry) = self.load_from_disk(key) {
            if self.is_valid_entry(&entry) {
                log::info!("Using disk-cached call graph");
                // Store in memory cache for faster access
                self.memory_cache.insert(key.clone(), entry.clone());
                return Some((
                    entry.call_graph,
                    entry.framework_exclusions,
                    entry.function_pointer_used,
                ));
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

        // Store on disk
        self.save_to_disk(&key, &entry)?;

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

    /// Load cache entry from disk
    fn load_from_disk(&self, key: &CacheKey) -> Result<CacheEntry> {
        let cache_file = self.get_cache_file_path(key);

        if !cache_file.exists() {
            anyhow::bail!("Cache file does not exist");
        }

        let content = fs::read_to_string(&cache_file)
            .with_context(|| format!("Failed to read cache file: {:?}", cache_file))?;

        let entry: CacheEntry =
            serde_json::from_str(&content).with_context(|| "Failed to deserialize cache entry")?;

        Ok(entry)
    }

    /// Save cache entry to disk
    fn save_to_disk(&self, key: &CacheKey, entry: &CacheEntry) -> Result<()> {
        let cache_file = self.get_cache_file_path(key);

        let content = serde_json::to_string_pretty(entry)
            .with_context(|| "Failed to serialize cache entry")?;

        fs::write(&cache_file, content)
            .with_context(|| format!("Failed to write cache file: {:?}", cache_file))?;

        Ok(())
    }

    /// Get the cache file path for a given key
    fn get_cache_file_path(&self, key: &CacheKey) -> PathBuf {
        // Use first 16 chars of source hash as filename
        let filename = format!("{}.json", &key.source_hash[..16.min(key.source_hash.len())]);
        self.cache_dir.join(filename)
    }

    /// Clear all cached entries
    pub fn clear(&mut self) -> Result<()> {
        // Clear memory cache
        self.memory_cache.clear();

        // Clear disk cache
        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                if entry.path().extension() == Some(std::ffi::OsStr::new("json")) {
                    fs::remove_file(entry.path())?;
                }
            }
        }

        Ok(())
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
