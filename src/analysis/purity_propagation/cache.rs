//! Persistent Caching for Purity Propagation Results
//!
//! This module provides persistent caching of purity propagation results to avoid
//! re-analysis on subsequent runs. The cache uses xxHash64 for fast content hashing
//! and bincode for efficient serialization.

use crate::priority::call_graph::FunctionId;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::PurityResult;

const CACHE_VERSION: u32 = 1;
const CACHE_FILE: &str = ".debtmap/purity_cache.bincode";

/// Persistent cache for purity propagation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityCache {
    /// Schema version for migration compatibility
    version: u32,

    /// Cached purity results indexed by function ID
    entries: HashMap<FunctionId, CachedPurity>,
}

/// Cached purity entry with validation information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedPurity {
    /// Purity propagation result
    result: PurityResult,

    /// xxHash64 of function source code
    source_hash: u64,

    /// xxHash64 of sorted dependency IDs
    deps_hash: u64,

    /// File modification time (seconds since epoch)
    file_mtime: u64,
}

impl PurityCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: HashMap::new(),
        }
    }

    /// Load cache from disk, creating new if doesn't exist
    pub fn load(project_root: &Path) -> Result<Self> {
        let cache_path = project_root.join(CACHE_FILE);

        if !cache_path.exists() {
            return Ok(Self::new());
        }

        let bytes = std::fs::read(&cache_path)?;
        let cache: PurityCache = bincode::deserialize(&bytes)?;

        // Validate version
        if cache.version != CACHE_VERSION {
            eprintln!("Cache version mismatch, rebuilding cache");
            return Ok(Self::new());
        }

        Ok(cache)
    }

    /// Save cache to disk
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let cache_path = project_root.join(CACHE_FILE);
        std::fs::create_dir_all(cache_path.parent().unwrap())?;

        let bytes = bincode::serialize(self)?;
        std::fs::write(&cache_path, bytes)?;

        Ok(())
    }

    /// Check if cached entry is still valid
    pub fn is_valid(
        &self,
        func_id: &FunctionId,
        current_mtime: u64,
        current_source_hash: u64,
        current_deps_hash: u64,
    ) -> bool {
        if let Some(cached) = self.entries.get(func_id) {
            cached.file_mtime == current_mtime
                && cached.source_hash == current_source_hash
                && cached.deps_hash == current_deps_hash
        } else {
            false
        }
    }

    /// Insert a new cache entry
    pub fn insert(
        &mut self,
        func_id: FunctionId,
        result: PurityResult,
        source_hash: u64,
        deps_hash: u64,
        file_mtime: u64,
    ) {
        self.entries.insert(
            func_id,
            CachedPurity {
                result,
                source_hash,
                deps_hash,
                file_mtime,
            },
        );
    }

    /// Get a cached result if valid
    pub fn get(&self, func_id: &FunctionId) -> Option<&PurityResult> {
        self.entries.get(func_id).map(|cached| &cached.result)
    }

    /// Invalidate entries for a specific file
    pub fn invalidate_file(&mut self, file_path: &Path) {
        self.entries.retain(|id, _| id.file != file_path);
    }

    /// Invalidate all entries that depend on a changed function
    pub fn invalidate_dependents(&mut self, changed_func_ids: &[FunctionId]) {
        // For now, we use a simple approach: invalidate all entries
        // A more sophisticated approach would track the call graph
        // and only invalidate transitive callers
        if !changed_func_ids.is_empty() {
            // Simple heuristic: clear cache when dependencies change
            // This ensures correctness at the cost of some redundant analysis
            self.entries.clear();
        }
    }

    /// Get cache size in number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for PurityCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a string using xxHash64
#[allow(dead_code)]
pub fn hash_string(s: &str) -> u64 {
    // For now, use a simple hash
    // TODO: Replace with xxhash-rust once dependency is added
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Hash a list of function IDs for dependency tracking
#[allow(dead_code)]
pub fn hash_deps(deps: &[FunctionId]) -> u64 {
    let mut sorted_deps = deps.to_vec();
    sorted_deps.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.name.cmp(&b.name))
            .then(a.line.cmp(&b.line))
    });

    let deps_string = sorted_deps
        .iter()
        .map(|id| format!("{}:{}:{}", id.file.display(), id.name, id.line))
        .collect::<Vec<_>>()
        .join("|");

    hash_string(&deps_string)
}

/// Get file modification time in seconds since epoch
#[allow(dead_code)]
pub fn get_mtime(file_path: &Path) -> Result<u64> {
    let metadata = std::fs::metadata(file_path)?;
    let modified = metadata.modified()?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH)?;
    Ok(duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cache_new() {
        let cache = PurityCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.version, CACHE_VERSION);
    }

    #[test]
    fn test_hash_deps_deterministic() {
        let func1 = FunctionId::new(PathBuf::from("test.rs"), "foo".to_string(), 100);
        let func2 = FunctionId::new(PathBuf::from("test.rs"), "bar".to_string(), 200);

        let deps1 = vec![func1.clone(), func2.clone()];
        let deps2 = vec![func2.clone(), func1.clone()];

        // Hash should be same regardless of order
        assert_eq!(hash_deps(&deps1), hash_deps(&deps2));
    }
}
