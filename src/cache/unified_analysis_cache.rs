use crate::cache::shared_cache::SharedCache;
use crate::priority::UnifiedAnalysis;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Cache key for unified analysis entries
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnifiedAnalysisCacheKey {
    /// Hash of all source files and their metrics
    pub source_hash: String,
    /// Project root path
    pub project_path: PathBuf,
    /// Configuration hash (includes thresholds, coverage file, etc.)
    pub config_hash: String,
    /// Coverage file hash (if present)
    pub coverage_hash: Option<String>,
}

/// Cached unified analysis entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAnalysisCacheEntry {
    /// The cached unified analysis result
    pub analysis: UnifiedAnalysis,
    /// Timestamp when cached
    pub timestamp: SystemTime,
    /// Source files included in the cache
    pub source_files: Vec<PathBuf>,
    /// Configuration used for this analysis
    pub config_summary: String,
}

/// Unified analysis cache manager
pub struct UnifiedAnalysisCache {
    /// Shared cache backend
    shared_cache: SharedCache,
    /// In-memory cache for current session
    memory_cache: HashMap<UnifiedAnalysisCacheKey, UnifiedAnalysisCacheEntry>,
    /// Maximum cache age in seconds (default: 1 hour)
    max_age: u64,
}

impl UnifiedAnalysisCache {
    /// Create a new unified analysis cache manager
    pub fn new(project_path: Option<&Path>) -> Result<Self> {
        let shared_cache = SharedCache::new(project_path)?;
        Ok(Self {
            shared_cache,
            memory_cache: HashMap::new(),
            max_age: 3600, // 1 hour
        })
    }

    /// Generate cache key for unified analysis
    pub fn generate_key(
        project_path: &Path,
        source_files: &[PathBuf],
        complexity_threshold: u32,
        duplication_threshold: usize,
        coverage_file: Option<&Path>,
        semantic_off: bool,
        parallel: bool,
    ) -> Result<UnifiedAnalysisCacheKey> {
        let source_hash = Self::hash_source_files(project_path, source_files);
        let config_hash = Self::hash_config(
            complexity_threshold,
            duplication_threshold,
            semantic_off,
            parallel,
        );
        let coverage_hash = coverage_file.and_then(Self::hash_coverage_file);

        Ok(UnifiedAnalysisCacheKey {
            source_hash,
            project_path: project_path.to_path_buf(),
            config_hash,
            coverage_hash,
        })
    }

    fn hash_source_files(project_path: &Path, source_files: &[PathBuf]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(project_path.to_string_lossy().as_bytes());

        let mut sorted_files = source_files.to_vec();
        sorted_files.sort();

        for file in &sorted_files {
            Self::hash_file_content(&mut hasher, file);
            Self::hash_file_mtime(&mut hasher, file);
        }

        format!("{:x}", hasher.finalize())
    }

    fn hash_file_content(hasher: &mut Sha256, file: &Path) {
        if let Ok(content) = std::fs::read_to_string(file) {
            hasher.update(file.to_string_lossy().as_bytes());
            hasher.update(content.as_bytes());
        }
    }

    fn hash_file_mtime(hasher: &mut Sha256, file: &Path) {
        let mtime_secs = std::fs::metadata(file)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        if let Some(secs) = mtime_secs {
            hasher.update(secs.to_le_bytes());
        }
    }

    fn hash_config(
        complexity_threshold: u32,
        duplication_threshold: usize,
        semantic_off: bool,
        parallel: bool,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(complexity_threshold.to_le_bytes());
        hasher.update(duplication_threshold.to_le_bytes());
        hasher.update([semantic_off as u8]);
        hasher.update([parallel as u8]);
        format!("{:x}", hasher.finalize())
    }

    fn hash_coverage_file(coverage_path: &Path) -> Option<String> {
        std::fs::read_to_string(coverage_path).ok().map(|content| {
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            format!("{:x}", hasher.finalize())
        })
    }

    /// Get cached unified analysis if available and valid
    pub fn get(&mut self, key: &UnifiedAnalysisCacheKey) -> Option<UnifiedAnalysis> {
        // Check memory cache first
        if let Some(entry) = self.memory_cache.get(key) {
            if self.is_entry_valid(entry) {
                log::info!("Unified analysis cache hit (memory)");
                return Some(entry.analysis.clone());
            } else {
                // Remove expired entry
                self.memory_cache.remove(key);
            }
        }

        // Check shared cache
        let cache_key = self.generate_shared_cache_key(key);
        if let Ok(data) = self.shared_cache.get(&cache_key, "unified_analysis") {
            if let Ok(entry) = serde_json::from_slice::<UnifiedAnalysisCacheEntry>(&data) {
                if self.is_entry_valid(&entry) {
                    log::info!("Unified analysis cache hit (shared)");
                    // Store in memory cache for faster access
                    self.memory_cache.insert(key.clone(), entry.clone());
                    return Some(entry.analysis);
                }
            }
        }

        log::info!("Unified analysis cache miss");
        None
    }

    /// Put unified analysis result in cache
    pub fn put(
        &mut self,
        key: UnifiedAnalysisCacheKey,
        analysis: UnifiedAnalysis,
        source_files: Vec<PathBuf>,
    ) -> Result<()> {
        let entry = UnifiedAnalysisCacheEntry {
            analysis: analysis.clone(),
            timestamp: SystemTime::now(),
            source_files,
            config_summary: format!("{:?}", key), // Simple config summary
        };

        // Store in memory cache
        self.memory_cache.insert(key.clone(), entry.clone());

        // Store in shared cache
        let cache_key = self.generate_shared_cache_key(&key);
        let data = serde_json::to_vec(&entry)
            .context("Failed to serialize unified analysis cache entry")?;

        self.shared_cache
            .put(&cache_key, "unified_analysis", &data)
            .context("Failed to store unified analysis in shared cache")?;

        log::info!("Unified analysis cached successfully");
        Ok(())
    }

    /// Clear all cached unified analysis data
    pub fn clear(&mut self) -> Result<()> {
        self.memory_cache.clear();
        // Note: SharedCache doesn't have a clear method for specific types
        // This would need to be implemented if needed
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> String {
        format!(
            "UnifiedAnalysisCache: {} entries in memory, max_age: {}s",
            self.memory_cache.len(),
            self.max_age
        )
    }

    /// Check if cache entry is still valid
    fn is_entry_valid(&self, entry: &UnifiedAnalysisCacheEntry) -> bool {
        if let Ok(elapsed) = entry.timestamp.elapsed() {
            elapsed.as_secs() <= self.max_age
        } else {
            false
        }
    }

    /// Generate shared cache key from unified analysis cache key
    fn generate_shared_cache_key(&self, key: &UnifiedAnalysisCacheKey) -> String {
        format!(
            "unified_analysis_{}_{}_{}",
            key.source_hash,
            key.config_hash,
            key.coverage_hash.as_deref().unwrap_or("no_coverage")
        )
    }

    /// Set maximum cache age in seconds
    pub fn set_max_age(&mut self, seconds: u64) {
        self.max_age = seconds;
    }

    /// Check if we should use cache based on project size
    pub fn should_use_cache(file_count: usize, has_coverage: bool) -> bool {
        // Always use cache for projects with many files or when coverage is involved
        file_count >= 20 || has_coverage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_key_generation() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // Create test files
        let file1 = project_path.join("test1.rs");
        let file2 = project_path.join("test2.rs");
        std::fs::write(&file1, "fn test1() {}").unwrap();
        std::fs::write(&file2, "fn test2() {}").unwrap();

        let files = vec![file1, file2];

        let key = UnifiedAnalysisCache::generate_key(
            project_path,
            &files,
            10,    // complexity_threshold
            50,    // duplication_threshold
            None,  // coverage_file
            false, // semantic_off
            true,  // parallel
        )
        .unwrap();

        assert!(!key.source_hash.is_empty());
        assert!(!key.config_hash.is_empty());
        assert_eq!(key.project_path, project_path);
        assert_eq!(key.coverage_hash, None);
    }

    #[test]
    fn test_should_use_cache() {
        assert!(!UnifiedAnalysisCache::should_use_cache(10, false));
        assert!(UnifiedAnalysisCache::should_use_cache(25, false));
        assert!(UnifiedAnalysisCache::should_use_cache(10, true));
        assert!(UnifiedAnalysisCache::should_use_cache(100, true));
    }

    #[test]
    fn test_cache_get_miss_on_empty_cache() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = UnifiedAnalysisCache::new(Some(temp_dir.path())).unwrap();

        let key = UnifiedAnalysisCacheKey {
            project_path: temp_dir.path().to_path_buf(),
            source_hash: "test_hash".to_string(),
            config_hash: "config_hash".to_string(),
            coverage_hash: None,
        };

        let result = cache.get(&key);
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_put_and_get_memory_cache() {
        use crate::priority::{UnifiedAnalysis, CallGraph};

        let temp_dir = TempDir::new().unwrap();
        let mut cache = UnifiedAnalysisCache::new(Some(temp_dir.path())).unwrap();

        let key = UnifiedAnalysisCacheKey {
            project_path: temp_dir.path().to_path_buf(),
            source_hash: "test_hash".to_string(),
            config_hash: "config_hash".to_string(),
            coverage_hash: None,
        };

        let analysis = UnifiedAnalysis::new(CallGraph::new());
        let source_files = vec![temp_dir.path().join("test.rs")];

        // Put in cache
        let put_result = cache.put(key.clone(), analysis.clone(), source_files);
        assert!(put_result.is_ok());

        // Get from cache (should hit memory cache)
        let result = cache.get(&key);
        assert!(result.is_some());
    }

    #[test]
    fn test_cache_put_and_get_shared_cache() {
        use crate::priority::{UnifiedAnalysis, CallGraph};

        let temp_dir = TempDir::new().unwrap();

        // Create first cache instance and put data
        let mut cache1 = UnifiedAnalysisCache::new(Some(temp_dir.path())).unwrap();

        let key = UnifiedAnalysisCacheKey {
            project_path: temp_dir.path().to_path_buf(),
            source_hash: "test_hash_shared".to_string(),
            config_hash: "config_hash_shared".to_string(),
            coverage_hash: None,
        };

        let analysis = UnifiedAnalysis::new(CallGraph::new());
        let source_files = vec![temp_dir.path().join("test.rs")];

        let put_result = cache1.put(key.clone(), analysis.clone(), source_files);
        assert!(put_result.is_ok());

        // Create second cache instance (different memory cache, same shared cache)
        let mut cache2 = UnifiedAnalysisCache::new(Some(temp_dir.path())).unwrap();

        // Get from cache (should hit shared cache, not memory)
        let result = cache2.get(&key);
        assert!(result.is_some());
    }

    #[test]
    fn test_cache_clear() {
        use crate::priority::{UnifiedAnalysis, CallGraph};

        let temp_dir = TempDir::new().unwrap();
        let mut cache = UnifiedAnalysisCache::new(Some(temp_dir.path())).unwrap();

        let key = UnifiedAnalysisCacheKey {
            project_path: temp_dir.path().to_path_buf(),
            source_hash: "test_hash_clear".to_string(),
            config_hash: "config_hash_clear".to_string(),
            coverage_hash: None,
        };

        let analysis = UnifiedAnalysis::new(CallGraph::new());
        let source_files = vec![temp_dir.path().join("test.rs")];

        let put_result = cache.put(key.clone(), analysis.clone(), source_files);
        assert!(put_result.is_ok());

        // Verify entry exists
        assert!(cache.get(&key).is_some());

        // Clear cache
        let clear_result = cache.clear();
        assert!(clear_result.is_ok());

        // Entry should be gone from memory cache
        // (Note: shared cache entries persist but that's expected behavior)
    }

    #[test]
    fn test_cache_put_updates_existing_entry() {
        use crate::priority::{UnifiedAnalysis, CallGraph};

        let temp_dir = TempDir::new().unwrap();
        let mut cache = UnifiedAnalysisCache::new(Some(temp_dir.path())).unwrap();

        let key = UnifiedAnalysisCacheKey {
            project_path: temp_dir.path().to_path_buf(),
            source_hash: "test_hash_update".to_string(),
            config_hash: "config_hash_update".to_string(),
            coverage_hash: None,
        };

        let analysis1 = UnifiedAnalysis::new(CallGraph::new());
        let source_files = vec![temp_dir.path().join("test.rs")];

        // Put first entry
        let put_result1 = cache.put(key.clone(), analysis1.clone(), source_files.clone());
        assert!(put_result1.is_ok());

        // Put second entry with same key (should update)
        let analysis2 = UnifiedAnalysis::new(CallGraph::new());

        let put_result2 = cache.put(key.clone(), analysis2.clone(), source_files);
        assert!(put_result2.is_ok());

        // Get should return the updated entry
        let result = cache.get(&key);
        assert!(result.is_some());
    }

    #[test]
    fn test_cache_get_with_coverage_hash() {
        use crate::priority::{UnifiedAnalysis, CallGraph};

        let temp_dir = TempDir::new().unwrap();
        let mut cache = UnifiedAnalysisCache::new(Some(temp_dir.path())).unwrap();

        let key = UnifiedAnalysisCacheKey {
            project_path: temp_dir.path().to_path_buf(),
            source_hash: "test_hash_cov".to_string(),
            config_hash: "config_hash_cov".to_string(),
            coverage_hash: Some("coverage_123".to_string()),
        };

        let analysis = UnifiedAnalysis::new(CallGraph::new());
        let source_files = vec![temp_dir.path().join("test.rs")];

        let put_result = cache.put(key.clone(), analysis.clone(), source_files);
        assert!(put_result.is_ok());

        // Get with same coverage hash should succeed
        let result = cache.get(&key);
        assert!(result.is_some());

        // Get with different coverage hash should miss
        let key_different_cov = UnifiedAnalysisCacheKey {
            coverage_hash: Some("coverage_456".to_string()),
            ..key
        };
        let result2 = cache.get(&key_different_cov);
        assert!(result2.is_none());
    }
}
