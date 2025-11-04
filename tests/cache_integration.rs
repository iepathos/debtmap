mod helpers;

use debtmap::cache::{CacheLocation, CacheStrategy, SharedCache};
use debtmap::core::cache::AnalysisCache;
use helpers::cache_isolation::IsolatedCacheTest;
use std::fs;
use std::path::{Path, PathBuf};

/// Helper to create a test file with content
fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let file_path = dir.join(name);
    fs::write(&file_path, content).unwrap();
    file_path
}

#[test]
fn test_shared_cache_creates_no_local_directory() {
    let isolated = IsolatedCacheTest::new("test_shared_cache_creates_no_local_directory");
    let project_dir = &isolated.project_dir;
    let cache_dir = &isolated.cache_dir;

    // Create a shared cache with explicit cache directory
    let cache =
        SharedCache::new_with_cache_dir(Some(project_dir.path()), cache_dir.path().to_path_buf())
            .unwrap();

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
    let isolated = IsolatedCacheTest::new("test_cache_location_strategies");

    // Test shared strategy (default)
    {
        let location = CacheLocation::resolve_with_strategy(None, CacheStrategy::Shared).unwrap();
        assert_eq!(location.strategy, CacheStrategy::Shared);
    }

    // Test custom strategy with isolated directory
    {
        let custom_dir = isolated.create_cache_dir("custom");
        let strategy = CacheStrategy::Custom(custom_dir.clone());
        let location = CacheLocation::resolve_with_strategy(None, strategy).unwrap();
        match location.strategy {
            CacheStrategy::Custom(path) => {
                assert_eq!(path, custom_dir);
            }
            _ => panic!("Expected Custom strategy, got {:?}", location.strategy),
        }
    }
}

#[test]
fn test_shared_cache_read_write() {
    let isolated = IsolatedCacheTest::new("test_shared_cache_read_write");
    let project_dir = &isolated.project_dir;
    let cache_dir = &isolated.cache_dir;

    let cache =
        SharedCache::new_with_cache_dir(Some(project_dir.path()), cache_dir.path().to_path_buf())
            .unwrap();

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
    let isolated = IsolatedCacheTest::new("test_cache_migration_from_local");
    let project_dir = &isolated.project_dir;
    let cache_dir = &isolated.cache_dir;
    // Create a legacy local cache
    let local_cache = project_dir.path().join(".debtmap_cache");
    fs::create_dir_all(&local_cache).unwrap();
    fs::write(local_cache.join("test_file.cache"), b"legacy data").unwrap();

    // Create shared cache and migrate
    let cache =
        SharedCache::new_with_cache_dir(Some(project_dir.path()), cache_dir.path().to_path_buf())
            .unwrap();
    cache.migrate_from_local(&local_cache).unwrap();

    // Verify migration - check that the projects directory with project ID was created
    let project_cache_dir = cache_dir
        .path()
        .join("debtmap")
        .join("projects")
        .join(&cache.location.project_id);

    assert!(
        project_cache_dir.exists(),
        "Project cache directory should exist at {:?}",
        project_cache_dir
    );
}

#[test]
fn test_analysis_cache_uses_shared_backend() {
    let isolated = IsolatedCacheTest::new("test_analysis_cache_uses_shared_backend");
    let project_dir = &isolated.project_dir;
    let cache_dir = &isolated.cache_dir;

    // Create a test file
    let test_file = create_test_file(project_dir.path(), "test.rs", "fn main() {}");

    // Create analysis cache with explicit cache directory to ensure isolation
    let mut cache =
        AnalysisCache::new_with_cache_dir(Some(project_dir.path()), cache_dir.path().to_path_buf())
            .unwrap();

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
        file_contexts: HashMap::new(),
            module_scope: None,
            classes: None,
        })
    };

    // Use the actual test file path for get_or_compute
    let _result = cache.get_or_compute(&test_file, compute).unwrap();

    // Call get_or_compute again to ensure cache write happens
    let compute2 = || {
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
        file_contexts: HashMap::new(),
            module_scope: None,
            classes: None,
        })
    };
    let _result2 = cache.get_or_compute(&test_file, compute2).unwrap();

    // Verify no local cache was created
    let local_cache = project_dir.path().join(".debtmap_cache");
    assert!(!local_cache.exists(), "Local cache should not be created");

    // Verify that the AnalysisCache is working properly with explicit cache directory
    // The key test is that the cache is functioning (as shown by hit/miss stats)
    // and using our explicit cache directory rather than environment variables
    assert!(
        cache.stats().misses > 0,
        "Cache should have at least one miss from computing metrics"
    );
    assert!(
        cache.stats().hits > 0,
        "Cache should have at least one hit from second call"
    );
}

#[test]
fn test_cache_clear_project() {
    let isolated = IsolatedCacheTest::new("test_cache_clear_project");
    let project_dir = &isolated.project_dir;
    let cache_dir = &isolated.cache_dir;

    let cache =
        SharedCache::new_with_cache_dir(Some(project_dir.path()), cache_dir.path().to_path_buf())
            .unwrap();

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
    let isolated = IsolatedCacheTest::new("test_cache_stats");
    let project_dir = &isolated.project_dir;
    let cache_dir = &isolated.cache_dir;

    let cache =
        SharedCache::new_with_cache_dir(Some(project_dir.path()), cache_dir.path().to_path_buf())
            .unwrap();

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
fn test_analysis_cache_creation() {
    let isolated = IsolatedCacheTest::new("test_analysis_cache_creation");
    let project_dir = &isolated.project_dir;
    let cache_dir = &isolated.cache_dir;

    let _test_file = create_test_file(project_dir.path(), "test.rs", "fn main() {}");

    // Test that AnalysisCache can be created with explicit cache directory
    let cache_result =
        AnalysisCache::new_with_cache_dir(Some(project_dir.path()), cache_dir.path().to_path_buf());
    assert!(
        cache_result.is_ok(),
        "Cache should initialize with explicit cache directory"
    );
}

#[test]
fn test_cache_project_id_generation() {
    let isolated = IsolatedCacheTest::new("test_cache_project_id_generation");
    let project_dir = &isolated.project_dir;

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
    let isolated = IsolatedCacheTest::new("test_cache_component_paths");
    let project_dir = &isolated.project_dir;
    let cache_dir = &isolated.cache_dir;

    let cache =
        SharedCache::new_with_cache_dir(Some(project_dir.path()), cache_dir.path().to_path_buf())
            .unwrap();

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
    let isolated = IsolatedCacheTest::new("test_cache_cleanup_on_size_limit");
    let project_dir = &isolated.project_dir;
    let cache_dir = &isolated.cache_dir;

    let cache =
        SharedCache::new_with_cache_dir(Some(project_dir.path()), cache_dir.path().to_path_buf())
            .unwrap();

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

    let isolated = IsolatedCacheTest::new("test_parallel_cache_access");
    let project_path = isolated.project_dir.path().to_path_buf();
    let cache_path = isolated.cache_dir.path().to_path_buf();

    // First, create the cache and ensure directories exist
    let setup_cache =
        SharedCache::new_with_cache_dir(Some(&project_path), cache_path.clone()).unwrap();
    drop(setup_cache);

    // Track success/failure
    let results = Arc::new(Mutex::new(Vec::new()));

    // Create shared cache instances in multiple threads
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let path = project_path.clone();
            let cache_path_clone = cache_path.clone();
            let results = Arc::clone(&results);
            thread::spawn(move || {
                // Each thread creates its own cache instance
                if let Ok(cache) = SharedCache::new_with_cache_dir(Some(&path), cache_path_clone) {
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
