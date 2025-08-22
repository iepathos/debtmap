use debtmap::core::{
    cache::AnalysisCache, ComplexityMetrics, FileMetrics, FunctionMetrics, Language,
};
use std::path::PathBuf;
use tempfile::TempDir;

// Helper function to create test metrics
fn create_test_metrics(path: &str, cyclo: u32, cognitive: u32) -> FileMetrics {
    FileMetrics {
        path: PathBuf::from(path),
        language: Language::Rust,
        complexity: ComplexityMetrics {
            functions: vec![FunctionMetrics {
                name: "test_func".to_string(),
                file: PathBuf::from(path),
                line: 1,
                cyclomatic: cyclo,
                cognitive,
                nesting: 1,
                length: 15,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            }],
            cyclomatic_complexity: cyclo,
            cognitive_complexity: cognitive,
        },
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
    }
}

#[test]
fn test_cache_new_creates_directory() {
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("cache");

    assert!(!cache_path.exists());
    let cache = AnalysisCache::new(cache_path.clone()).unwrap();
    assert!(cache_path.exists());

    // Verify initial stats
    let stats = cache.stats();
    assert_eq!(stats.entries, 0);
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.hit_rate, 0.0);
}

#[test]
fn test_cache_get_or_compute_miss_then_hit() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn test() {}").unwrap();

    let mut cache = AnalysisCache::new(temp_dir.path().join("cache")).unwrap();

    // First call should be a miss
    let compute_count = std::cell::RefCell::new(0);
    let metrics1 = cache
        .get_or_compute(&test_file, || {
            *compute_count.borrow_mut() += 1;
            Ok(create_test_metrics("test.rs", 2, 3))
        })
        .unwrap();

    assert_eq!(*compute_count.borrow(), 1);
    assert_eq!(cache.stats().misses, 1);
    assert_eq!(cache.stats().hits, 0);
    assert_eq!(metrics1.complexity.cyclomatic_complexity, 2);

    // Second call should be a hit
    let metrics2 = cache
        .get_or_compute(&test_file, || {
            *compute_count.borrow_mut() += 1;
            Ok(create_test_metrics("test.rs", 99, 99)) // Different values to prove cache is used
        })
        .unwrap();

    assert_eq!(*compute_count.borrow(), 1); // Compute function not called again
    assert_eq!(cache.stats().misses, 1);
    assert_eq!(cache.stats().hits, 1);
    assert_eq!(metrics2.complexity.cyclomatic_complexity, 2); // Same as cached value
}

#[test]
fn test_cache_invalidation_on_file_change() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn test() {}").unwrap();

    let mut cache = AnalysisCache::new(temp_dir.path().join("cache")).unwrap();

    // Initial computation
    let compute_count = std::cell::RefCell::new(0);
    cache
        .get_or_compute(&test_file, || {
            *compute_count.borrow_mut() += 1;
            Ok(create_test_metrics("test.rs", 2, 3))
        })
        .unwrap();

    assert_eq!(*compute_count.borrow(), 1);

    // Modify file content
    std::fs::write(&test_file, "fn test() { if true {} }").unwrap();

    // Should recompute due to changed content
    let metrics = cache
        .get_or_compute(&test_file, || {
            *compute_count.borrow_mut() += 1;
            Ok(create_test_metrics("test.rs", 4, 5))
        })
        .unwrap();

    assert_eq!(*compute_count.borrow(), 2); // Compute called again
    assert_eq!(cache.stats().misses, 2);
    assert_eq!(cache.stats().hits, 0);
    assert_eq!(metrics.complexity.cyclomatic_complexity, 4);
}

#[test]
fn test_cache_clear() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn test() {}").unwrap();

    let mut cache = AnalysisCache::new(temp_dir.path().join("cache")).unwrap();

    // Add entry to cache
    cache
        .get_or_compute(&test_file, || Ok(create_test_metrics("test.rs", 2, 3)))
        .unwrap();

    assert_eq!(cache.stats().entries, 1);
    assert_eq!(cache.stats().misses, 1);

    // Clear cache
    cache.clear().unwrap();

    let stats = cache.stats();
    assert_eq!(stats.entries, 0);
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);

    // Verify index file is removed
    let index_path = temp_dir.path().join("cache").join("index.json");
    assert!(!index_path.exists());
}

#[test]
fn test_cache_stats_hit_rate_calculation() {
    let temp_dir = TempDir::new().unwrap();
    let mut cache = AnalysisCache::new(temp_dir.path().join("cache")).unwrap();

    // Create test files
    for i in 0..3 {
        let test_file = temp_dir.path().join(format!("test{i}.rs"));
        std::fs::write(&test_file, format!("fn test{i}() {{}}")).unwrap();

        // First access - miss
        cache
            .get_or_compute(&test_file, || {
                Ok(create_test_metrics(
                    &format!("test{i}.rs"),
                    i as u32,
                    i as u32,
                ))
            })
            .unwrap();
    }

    // Access first file again - hit
    let test_file0 = temp_dir.path().join("test0.rs");
    cache
        .get_or_compute(&test_file0, || Ok(create_test_metrics("test0.rs", 99, 99)))
        .unwrap();

    let stats = cache.stats();
    assert_eq!(stats.entries, 3);
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 3);
    assert_eq!(stats.hit_rate, 0.25); // 1 hit / (1 hit + 3 misses)
}

#[test]
fn test_cache_prune_old_entries() {
    let temp_dir = TempDir::new().unwrap();
    let mut cache = AnalysisCache::new(temp_dir.path().join("cache")).unwrap();

    // Create test files and add to cache
    for i in 0..3 {
        let test_file = temp_dir.path().join(format!("test{i}.rs"));
        std::fs::write(&test_file, format!("fn test{i}() {{}}")).unwrap();

        cache
            .get_or_compute(&test_file, || {
                Ok(create_test_metrics(
                    &format!("test{i}.rs"),
                    i as u32,
                    i as u32,
                ))
            })
            .unwrap();
    }

    assert_eq!(cache.stats().entries, 3);

    // Prune entries older than 1 day (should keep all recent entries)
    cache.prune(1).unwrap();
    assert_eq!(cache.stats().entries, 3);

    // Prune entries older than 0 days (removes all entries created before now)
    cache.prune(0).unwrap();
    assert_eq!(cache.stats().entries, 0);

    // Note: Testing actual pruning of old entries would require mocking time,
    // which is beyond the scope of this test. The prune logic is verified
    // by checking that recent entries are kept.
}

#[test]
fn test_cache_persistence_across_instances() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn test() {}").unwrap();

    // First cache instance - add entry
    {
        let mut cache = AnalysisCache::new(cache_dir.clone()).unwrap();
        cache
            .get_or_compute(&test_file, || Ok(create_test_metrics("test.rs", 2, 3)))
            .unwrap();

        assert_eq!(cache.stats().entries, 1);
        assert_eq!(cache.stats().misses, 1);
    }

    // Second cache instance - should load existing entry
    {
        let mut cache = AnalysisCache::new(cache_dir.clone()).unwrap();

        // Access should be a hit
        let compute_called = std::cell::RefCell::new(false);
        let metrics = cache
            .get_or_compute(&test_file, || {
                *compute_called.borrow_mut() = true;
                Ok(create_test_metrics("test.rs", 99, 99))
            })
            .unwrap();

        assert!(!*compute_called.borrow()); // Compute not called
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().misses, 0);
        assert_eq!(metrics.complexity.cyclomatic_complexity, 2); // Original cached value
    }
}

#[test]
fn test_cache_handles_io_errors_gracefully() {
    let temp_dir = TempDir::new().unwrap();
    let non_existent_file = temp_dir.path().join("non_existent.rs");

    let mut cache = AnalysisCache::new(temp_dir.path().join("cache")).unwrap();

    // Should return error for non-existent file
    let result = cache.get_or_compute(&non_existent_file, || {
        Ok(create_test_metrics("test.rs", 2, 3))
    });

    assert!(result.is_err());
    assert_eq!(cache.stats().hits, 0);
    assert_eq!(cache.stats().misses, 0);
}

#[test]
fn test_cache_compute_function_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn test() {}").unwrap();

    let mut cache = AnalysisCache::new(temp_dir.path().join("cache")).unwrap();

    // Compute function returns error
    let result = cache.get_or_compute(&test_file, || Err(anyhow::anyhow!("Compute failed")));

    assert!(result.is_err());
    assert_eq!(cache.stats().entries, 0); // No entry added on error
    assert_eq!(cache.stats().misses, 1);
}

#[test]
fn test_cache_with_different_file_types() {
    let temp_dir = TempDir::new().unwrap();
    let mut cache = AnalysisCache::new(temp_dir.path().join("cache")).unwrap();

    // Test with different file extensions
    let files = vec![
        ("test.rs", Language::Rust, "fn main() {}"),
        ("test.py", Language::Python, "def main(): pass"),
        ("test.js", Language::JavaScript, "function main() {}"),
        ("test.ts", Language::TypeScript, "function main(): void {}"),
    ];

    for (filename, lang, content) in files {
        let test_file = temp_dir.path().join(filename);
        std::fs::write(&test_file, content).unwrap();

        let metrics = cache
            .get_or_compute(&test_file, || {
                let mut m = create_test_metrics(filename, 1, 1);
                m.language = lang;
                Ok(m)
            })
            .unwrap();

        assert_eq!(metrics.language, lang);
    }

    assert_eq!(cache.stats().entries, 4);
    assert_eq!(cache.stats().misses, 4);
}

#[test]
fn test_cache_stats_display_formatting() {
    let temp_dir = TempDir::new().unwrap();
    let cache = AnalysisCache::new(temp_dir.path().join("cache")).unwrap();

    let stats = cache.stats();
    let display = format!("{stats}");

    assert!(display.contains("Cache Stats"));
    assert!(display.contains("0 entries"));
    assert!(display.contains("0 hits"));
    assert!(display.contains("0 misses"));
    assert!(display.contains("0.0% hit rate"));
}

#[test]
fn test_cache_handles_concurrent_modifications() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn test() {}").unwrap();

    let mut cache = AnalysisCache::new(temp_dir.path().join("cache")).unwrap();

    // Add initial entry
    cache
        .get_or_compute(&test_file, || Ok(create_test_metrics("test.rs", 2, 3)))
        .unwrap();

    // Simulate file modification between check and compute
    // This tests the robustness of the cache invalidation logic
    let metrics = cache
        .get_or_compute(&test_file, || {
            // Modify file during computation
            std::fs::write(&test_file, "fn test() { modified }").unwrap();
            Ok(create_test_metrics("test.rs", 5, 6))
        })
        .unwrap();

    // Should have used cached value since hash was checked before modification
    assert_eq!(metrics.complexity.cyclomatic_complexity, 2);
    assert_eq!(cache.stats().hits, 1);
}
