use anyhow::Result;
use chrono::{DateTime, Utc};
use im::HashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::core::FileMetrics;

/// File information for caching
struct FileInfo {
    hash: String,
    modified: DateTime<Utc>,
}

/// Cache entry for analysis results
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_hash: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: FileMetrics,
}

/// Analysis cache using persistent data structures
#[derive(Clone, Debug)]
pub struct AnalysisCache {
    cache_dir: PathBuf,
    index: HashMap<String, CacheEntry>,
    hits: usize,
    misses: usize,
}

impl AnalysisCache {
    /// Create a new cache instance
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        let index = Self::load_index(&cache_dir).unwrap_or_default();

        Ok(Self {
            cache_dir,
            index,
            hits: 0,
            misses: 0,
        })
    }

    /// Get cached metrics or compute new ones
    pub fn get_or_compute<F>(&mut self, path: &Path, compute: F) -> Result<FileMetrics>
    where
        F: FnOnce() -> Result<FileMetrics>,
    {
        let file_info = self.get_file_info(path)?;

        self.try_cache_hit(&file_info)
            .or_else(|| self.compute_and_cache(file_info, compute))
            .unwrap_or_else(|| Err(anyhow::anyhow!("Failed to get or compute metrics")))
    }

    /// Get file information needed for caching
    fn get_file_info(&self, path: &Path) -> Result<FileInfo> {
        let content = std::fs::read_to_string(path)?;
        let hash = Self::calculate_hash(&content);
        let metadata = std::fs::metadata(path)?;
        let modified = DateTime::from(metadata.modified()?);

        Ok(FileInfo { hash, modified })
    }

    /// Try to get a cache hit
    fn try_cache_hit(&mut self, file_info: &FileInfo) -> Option<Result<FileMetrics>> {
        self.index
            .get(&file_info.hash)
            .filter(|entry| entry.timestamp >= file_info.modified)
            .map(|entry| {
                self.hits += 1;
                Ok(entry.metrics.clone())
            })
    }

    /// Compute new metrics and update cache
    fn compute_and_cache<F>(
        &mut self,
        file_info: FileInfo,
        compute: F,
    ) -> Option<Result<FileMetrics>>
    where
        F: FnOnce() -> Result<FileMetrics>,
    {
        self.misses += 1;

        Some(compute().and_then(|metrics| {
            let entry = CacheEntry {
                file_hash: file_info.hash.clone(),
                timestamp: Utc::now(),
                metrics: metrics.clone(),
            };

            self.index = self.index.update(file_info.hash, entry);
            self.save_index()?;
            Ok(metrics)
        }))
    }

    /// Calculate SHA-256 hash of content
    fn calculate_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Load cache index from disk
    fn load_index(cache_dir: &Path) -> Result<HashMap<String, CacheEntry>> {
        let index_path = cache_dir.join("index.json");
        if !index_path.exists() {
            return Ok(HashMap::new());
        }

        let content = std::fs::read_to_string(index_path)?;
        let entries: Vec<(String, CacheEntry)> = serde_json::from_str(&content)?;

        Ok(entries.into_iter().collect())
    }

    /// Save cache index to disk
    fn save_index(&self) -> Result<()> {
        let index_path = self.cache_dir.join("index.json");
        let entries: Vec<(String, CacheEntry)> = self
            .index
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let content = serde_json::to_string_pretty(&entries)?;
        std::fs::write(index_path, content)?;

        Ok(())
    }

    /// Clear the cache
    pub fn clear(&mut self) -> Result<()> {
        self.index = HashMap::new();
        self.hits = 0;
        self.misses = 0;

        let index_path = self.cache_dir.join("index.json");
        if index_path.exists() {
            std::fs::remove_file(index_path)?;
        }

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.index.len(),
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Prune old cache entries
    pub fn prune(&mut self, max_age_days: i64) -> Result<()> {
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days);

        self.index = self
            .index
            .clone()
            .into_iter()
            .filter(|(_, entry)| entry.timestamp > cutoff)
            .collect();

        self.save_index()?;
        Ok(())
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub hits: usize,
    pub misses: usize,
    pub hit_rate: f64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache Stats: {} entries, {} hits, {} misses, {:.1}% hit rate",
            self.entries,
            self.hits,
            self.misses,
            self.hit_rate * 100.0
        )
    }
}

/// Incremental analysis state using persistent data structures
#[derive(Clone, Debug)]
pub struct IncrementalAnalysis {
    pub previous_state: HashMap<PathBuf, FileMetrics>,
    pub current_state: HashMap<PathBuf, FileMetrics>,
    pub changed_files: im::HashSet<PathBuf>,
}

impl Default for IncrementalAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalAnalysis {
    /// Create new incremental analysis state
    pub fn new() -> Self {
        Self {
            previous_state: HashMap::new(),
            current_state: HashMap::new(),
            changed_files: im::HashSet::new(),
        }
    }

    /// Load previous state from cache
    pub fn load_previous(&mut self, cache: &AnalysisCache) {
        self.previous_state = cache
            .index
            .iter()
            .map(|(_, entry)| (entry.metrics.path.clone(), entry.metrics.clone()))
            .collect();
    }

    /// Mark file as changed
    pub fn mark_changed(&mut self, path: PathBuf) {
        self.changed_files = self.changed_files.update(path);
    }

    /// Update current state for a file
    pub fn update_file(&mut self, metrics: FileMetrics) {
        let path = metrics.path.clone();
        self.current_state = self.current_state.update(path, metrics);
    }

    /// Get files that need reanalysis
    pub fn get_files_to_analyze(&self, all_files: &[PathBuf]) -> Vec<PathBuf> {
        all_files
            .iter()
            .filter(|path| {
                self.changed_files.contains(*path) || !self.previous_state.contains_key(*path)
            })
            .cloned()
            .collect()
    }

    /// Calculate diff between previous and current state
    pub fn calculate_diff(&self) -> AnalysisDiff {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut removed = Vec::new();

        for (path, current) in &self.current_state {
            if let Some(previous) = self.previous_state.get(path) {
                if !metrics_equal(previous, current) {
                    modified.push(path.clone());
                }
            } else {
                added.push(path.clone());
            }
        }

        for path in self.previous_state.keys() {
            if !self.current_state.contains_key(path) {
                removed.push(path.clone());
            }
        }

        AnalysisDiff {
            added,
            modified,
            removed,
        }
    }
}

/// Diff between analysis states
#[derive(Debug, Clone)]
pub struct AnalysisDiff {
    pub added: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
}

fn metrics_equal(a: &FileMetrics, b: &FileMetrics) -> bool {
    // Simple equality check - could be more sophisticated
    a.complexity.functions.len() == b.complexity.functions.len()
        && a.debt_items.len() == b.debt_items.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_hash() {
        let content1 = "fn main() {}";
        let content2 = "fn main() { println!(\"hello\"); }";

        let hash1 = AnalysisCache::calculate_hash(content1);
        let hash2 = AnalysisCache::calculate_hash(content2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let cache = AnalysisCache::new(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(cache.stats().entries, 0);
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn test_incremental_analysis() {
        let mut inc = IncrementalAnalysis::new();

        inc.mark_changed(PathBuf::from("file1.rs"));
        inc.mark_changed(PathBuf::from("file2.rs"));

        assert!(inc.changed_files.contains(&PathBuf::from("file1.rs")));
        assert!(inc.changed_files.contains(&PathBuf::from("file2.rs")));
    }
}
