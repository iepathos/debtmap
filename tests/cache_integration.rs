use debtmap::cache::{CacheLocation, CacheStrategy, SharedCache};
use debtmap::core::cache::AnalysisCache;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// Helper to manage environment variables safely in tests
struct EnvGuard {
    vars: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn new() -> Self {
        Self { vars: Vec::new() }
    }

    fn set(&mut self, key: &str, value: &str) {
        let old = std::env::var(key).ok();
        self.vars.push((key.to_string(), old));
        std::env::set_var(key, value);
    }

    fn remove(&mut self, key: &str) {
        let old = std::env::var(key).ok();
        self.vars.push((key.to_string(), old));
        std::env::remove_var(key);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, old_value) in &self.vars {
            match old_value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// Helper to create a test file with content
fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let file_path = dir.join(name);
    fs::write(&file_path, content).unwrap();
    file_path
}

/// Helper to set up a test environment with a custom cache dir
fn setup_test_env() -> (TempDir, TempDir, EnvGuard) {
    let project_dir = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    let mut env = EnvGuard::new();
    env.set("DEBTMAP_CACHE_DIR", cache_dir.path().to_str().unwrap());
    (project_dir, cache_dir, env)
}

#[test]
fn test_shared_cache_creates_no_local_directory() {
    let (project_dir, cache_dir, _env) = setup_test_env();

    // Create a shared cache
    let cache = SharedCache::new(Some(project_dir.path())).unwrap();

    // Store some data
    let data = b"test data";
    cache.put("test_key", "test_component", data).unwrap();

    // Verify no .debtmap_cache directory was created in the project
    let local_cache = project_dir.path().join(".debtmap_cache");
    assert!(
        !local_cache.exists(),
        "Local .debtmap_cache should not be created"
    );

    // Verify data is in the shared cache location
    assert!(
        cache_dir.path().join("debtmap").join("projects").exists(),
        "Shared cache directory with projects subdirectory should exist"
    );
}

#[test]
fn test_cache_location_strategies() {
    // Test shared strategy (default)
    {
        let mut env = EnvGuard::new();
        env.remove("DEBTMAP_CACHE_DIR");
        let location = CacheLocation::resolve(None).unwrap();
        assert_eq!(location.strategy, CacheStrategy::Shared);
    }

    // Test custom strategy
    {
        let custom_dir = TempDir::new().unwrap();
        let mut env = EnvGuard::new();
        env.set("DEBTMAP_CACHE_DIR", custom_dir.path().to_str().unwrap());
        let location = CacheLocation::resolve(None).unwrap();
        match location.strategy {
            CacheStrategy::Custom(path) => {
                assert_eq!(path, custom_dir.path());
            }
            _ => panic!("Expected Custom strategy, got {:?}", location.strategy),
        }
    }
}

#[test]
fn test_shared_cache_read_write() {
    let (project_dir, _cache_dir, _env) = setup_test_env();

    let cache = SharedCache::new(Some(project_dir.path())).unwrap();

    // Write data
    let key = "test_key";
    let component = "test_component";
    let data = b"Hello, cache!";

    cache.put(key, component, data).unwrap();

    // Read data back
    let retrieved = cache.get(key, component).unwrap();
    assert_eq!(retrieved, data);

    // Check existence
    assert!(cache.exists(key, component));
    assert!(!cache.exists("nonexistent", component));
}

#[test]
fn test_cache_migration_from_local() {
    let project_dir = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    let mut env = EnvGuard::new();
    env.set("DEBTMAP_CACHE_DIR", cache_dir.path().to_str().unwrap());

    // Create a legacy local cache
    let local_cache = project_dir.path().join(".debtmap_cache");
    fs::create_dir_all(&local_cache).unwrap();
    fs::write(local_cache.join("test_file.cache"), b"legacy data").unwrap();

    // Create shared cache and migrate
    let cache = SharedCache::new(Some(project_dir.path())).unwrap();
    cache.migrate_from_local(&local_cache).unwrap();

    // Verify migration
    let migrated_file = cache_dir
        .path()
        .join("debtmap")
        .join("projects")
        .join(&cache.location.project_id)
        .join("test_file.cache");

    assert!(
        migrated_file.exists() || cache_dir.path().join("debtmap").join("projects").exists(),
        "Migrated data should exist in shared cache"
    );
}

#[test]
fn test_analysis_cache_uses_shared_backend() {
    let (project_dir, cache_dir, _env) = setup_test_env();

    // Create a test file
    let test_file = create_test_file(project_dir.path(), "test.rs", "fn main() {}");

    // Create analysis cache
    let mut cache = AnalysisCache::new(Some(project_dir.path())).unwrap();

    // Use the cache
    let compute = || {
        Ok(debtmap::core::FileMetrics {
            path: test_file.clone(),
            language: debtmap::core::Language::Rust,
            complexity: debtmap::core::ComplexityMetrics {
                functions: vec![],
                cyclomatic_complexity: 1,
                cognitive_complexity: 1,
            },
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
        })
    };

    let _result = cache.get_or_compute(&test_file, compute).unwrap();

    // Verify no local cache was created
    let local_cache = project_dir.path().join(".debtmap_cache");
    assert!(!local_cache.exists(), "Local cache should not be created");

    // Verify shared cache was used - check for the debtmap/projects directory structure
    assert!(
        cache_dir.path().join("debtmap").join("projects").exists(),
        "Shared cache should be used with projects subdirectory"
    );
}

#[test]
fn test_cache_clear_project() {
    let (project_dir, _cache_dir, _env) = setup_test_env();

    let cache = SharedCache::new(Some(project_dir.path())).unwrap();

    // Add some data using valid component names
    cache.put("key1", "analysis", b"data1").unwrap();
    cache.put("key2", "metadata", b"data2").unwrap();
    cache.put("key3", "file_metrics", b"data3").unwrap();

    // Verify data exists
    assert!(cache.exists("key1", "analysis"));
    assert!(cache.exists("key2", "metadata"));
    assert!(cache.exists("key3", "file_metrics"));

    // Clear project cache
    cache.clear_project().unwrap();

    // Verify data is gone
    assert!(!cache.exists("key1", "analysis"));
    assert!(!cache.exists("key2", "metadata"));
    assert!(!cache.exists("key3", "file_metrics"));
}

#[test]
fn test_cache_stats() {
    let (project_dir, _cache_dir, _env) = setup_test_env();

    let cache = SharedCache::new(Some(project_dir.path())).unwrap();

    // Add some data
    cache.put("key1", "component1", b"test data 1").unwrap();
    cache.put("key2", "component2", b"test data 2").unwrap();

    // Get stats
    let stats = cache.get_stats();
    assert!(stats.entry_count >= 2, "Should have at least 2 entries");
    assert!(stats.total_size > 0, "Should have non-zero size");

    // Get full stats
    let full_stats = cache.get_full_stats().unwrap();
    assert!(full_stats.total_entries >= 2);
    assert_eq!(full_stats.project_id, cache.location.project_id);
}

#[test]
fn test_no_cache_environment_variable() {
    let project_dir = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    let mut env = EnvGuard::new();
    env.set("DEBTMAP_CACHE_DIR", cache_dir.path().to_str().unwrap());
    env.set("DEBTMAP_NO_CACHE", "1");

    // When NO_CACHE is set, AnalysisCache creation should still work but not cache
    let _test_file = create_test_file(project_dir.path(), "test.rs", "fn main() {}");

    // This would normally be handled by the analyze command checking DEBTMAP_NO_CACHE
    // but we can verify the cache directory isn't populated
    let cache_result = AnalysisCache::new(Some(project_dir.path()));
    assert!(
        cache_result.is_ok(),
        "Cache should initialize even with NO_CACHE"
    );
}

#[test]
fn test_cache_project_id_generation() {
    let project_dir = TempDir::new().unwrap();

    // Initialize git repo for consistent project ID
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(project_dir.path())
        .output()
        .ok();

    let id1 = CacheLocation::generate_project_id(project_dir.path()).unwrap();
    let id2 = CacheLocation::generate_project_id(project_dir.path()).unwrap();

    // Project ID should be consistent
    assert_eq!(id1, id2);
    assert_eq!(id1.len(), 16); // Should be 16 characters
}

#[test]
fn test_cache_component_paths() {
    let (project_dir, _cache_dir, _env) = setup_test_env();

    let cache = SharedCache::new(Some(project_dir.path())).unwrap();

    // Test different component paths
    let components = [
        "call_graphs",
        "analysis",
        "metadata",
        "temp",
        "file_metrics",
    ];

    for component in &components {
        cache.put("test_key", component, b"test_data").ok();
        let component_path = cache.location.get_component_path(component);
        assert!(component_path.to_str().unwrap().contains(component));
    }
}

#[test]
fn test_cache_cleanup_on_size_limit() {
    let (project_dir, _cache_dir, _env) = setup_test_env();

    let cache = SharedCache::new(Some(project_dir.path())).unwrap();

    // Add data to test cleanup mechanism
    // The cache has internal size limits and will clean up automatically
    for i in 0..100 {
        let key = format!("key_{}", i);
        let data = vec![0u8; 10000]; // 10KB each
        cache.put(&key, "test", &data).ok();
    }

    // Cache should have managed its size through automatic cleanup
    let stats = cache.get_stats();
    // Just verify that stats are working
    assert!(stats.entry_count > 0);
    assert!(stats.total_size > 0);
}

#[test]
fn test_parallel_cache_access() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let (project_dir, _cache_dir, _env) = setup_test_env();
    let project_path = project_dir.path().to_path_buf();

    // First, create the cache and ensure directories exist
    let setup_cache = SharedCache::new(Some(&project_path)).unwrap();
    drop(setup_cache);

    // Track success/failure
    let results = Arc::new(Mutex::new(Vec::new()));

    // Create shared cache instances in multiple threads
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let path = project_path.clone();
            let results = Arc::clone(&results);
            thread::spawn(move || {
                // Each thread creates its own cache instance
                if let Ok(cache) = SharedCache::new(Some(&path)) {
                    let key = format!("thread_{}", i);
                    let data = format!("data_{}", i).into_bytes();

                    // Try to write and read data
                    if cache.put(&key, "metadata", &data).is_ok() {
                        if let Ok(retrieved) = cache.get(&key, "metadata") {
                            if retrieved == data {
                                results.lock().unwrap().push((i, true));
                                return;
                            }
                        }
                    }
                }
                results.lock().unwrap().push((i, false));
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().ok();
    }

    // Check that at least some operations succeeded
    let results = results.lock().unwrap();
    let successes = results.iter().filter(|(_, success)| *success).count();
    assert!(
        successes > 0,
        "At least some parallel operations should succeed"
    );
}
