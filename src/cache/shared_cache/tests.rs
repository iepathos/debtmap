use super::*;
use std::collections::HashMap;
use tempfile::TempDir;

#[test]
fn test_shared_cache_operations() {
    let temp_dir = TempDir::new().unwrap();

    // Set environment variables directly
    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    // Test put and get
    let key = "test_key";
    let component = "test_component";
    let data = b"test data";

    cache.put(key, component, data).unwrap();
    assert!(cache.exists(key, component));

    let retrieved = cache.get(key, component).unwrap();
    assert_eq!(retrieved, data);

    // Test delete
    cache.delete(key, component).unwrap();
    assert!(!cache.exists(key, component));

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cache_stats() {
    let temp_dir = TempDir::new().unwrap();

    // Set environment variables directly
    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    // Ensure directories are created
    cache.location.ensure_directories().unwrap();

    // Add some entries
    cache.put("key1", "component1", b"data1").unwrap();
    cache.put("key2", "component1", b"data2").unwrap();

    let stats = cache.get_stats();
    assert_eq!(stats.entry_count, 2);
    assert_eq!(stats.total_size, 10); // "data1" + "data2"

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_age_calculation_pure_functions() {
    // Test max age calculation
    let max_age_0_days = SharedCache::calculate_max_age_duration(0);
    let max_age_1_day = SharedCache::calculate_max_age_duration(1);

    assert_eq!(max_age_0_days, Duration::from_secs(0));
    assert_eq!(max_age_1_day, Duration::from_secs(86400));

    // Test should_remove_entry_by_age with zero age (the key fix)
    let now = SystemTime::now();
    let same_time = now;
    let older_time = now - Duration::from_secs(100);

    // With max_age = 0, entries created at same time should be removed
    assert!(SharedCache::should_remove_entry_by_age(
        now,
        same_time,
        Duration::from_secs(0)
    ));

    // With max_age = 0, older entries should definitely be removed
    assert!(SharedCache::should_remove_entry_by_age(
        now,
        older_time,
        Duration::from_secs(0)
    ));

    // With max_age = 200s, older entries (100s old) should not be removed
    assert!(!SharedCache::should_remove_entry_by_age(
        now,
        older_time,
        Duration::from_secs(200)
    ));
}

#[test]
fn test_filter_entries_by_age() {
    let now = SystemTime::now();
    let old_time = now - Duration::from_secs(100);

    let mut entries = HashMap::new();
    entries.insert(
        "recent_entry".to_string(),
        CacheMetadata {
            version: "1.0".to_string(),
            created_at: now,
            last_accessed: now,
            access_count: 1,
            size_bytes: 100,
            debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    );
    entries.insert(
        "old_entry".to_string(),
        CacheMetadata {
            version: "1.0".to_string(),
            created_at: old_time,
            last_accessed: old_time,
            access_count: 1,
            size_bytes: 100,
            debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    );

    // With max_age = 0, both entries should be removed
    let to_remove_0 = SharedCache::filter_entries_by_age(&entries, now, Duration::from_secs(0));
    assert_eq!(to_remove_0.len(), 2);
    assert!(to_remove_0.contains(&"recent_entry".to_string()));
    assert!(to_remove_0.contains(&"old_entry".to_string()));

    // With max_age = 50s, only the old entry should be removed
    let to_remove_50 = SharedCache::filter_entries_by_age(&entries, now, Duration::from_secs(50));
    assert_eq!(to_remove_50.len(), 1);
    assert!(to_remove_50.contains(&"old_entry".to_string()));
}

#[test]
fn test_put_with_config_test_environment() {
    let temp_dir = TempDir::new().unwrap();

    // Set environment variables
    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    let test_config = PruningConfig {
        auto_prune_enabled: false,
        use_sync_pruning: false,
        is_test_environment: true,
    };

    let key = "test_key";
    let component = "test_component";
    let data = b"test data for config";

    // Test put_with_config in test environment
    cache
        .put_with_config(key, component, data, &test_config)
        .unwrap();

    // Verify the data was stored correctly
    assert!(cache.exists(key, component));
    let retrieved = cache.get(key, component).unwrap();
    assert_eq!(retrieved, data);

    // Verify index was updated
    let stats = cache.get_stats();
    assert_eq!(stats.entry_count, 1);
    assert!(stats.total_size >= data.len() as u64);

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_put_with_config_sync_pruning_enabled() {
    let temp_dir = TempDir::new().unwrap();

    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "true");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    let sync_prune_config = PruningConfig {
        auto_prune_enabled: true,
        use_sync_pruning: true,
        is_test_environment: false,
    };

    let key = "sync_prune_key";
    let component = "sync_component";
    let data = b"data with sync pruning";

    // Test put_with_config with sync pruning
    cache
        .put_with_config(key, component, data, &sync_prune_config)
        .unwrap();

    // Verify the data was stored correctly
    assert!(cache.exists(key, component));
    let retrieved = cache.get(key, component).unwrap();
    assert_eq!(retrieved, data);

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_put_with_config_auto_prune_disabled() {
    let temp_dir = TempDir::new().unwrap();

    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    let no_prune_config = PruningConfig {
        auto_prune_enabled: false,
        use_sync_pruning: false,
        is_test_environment: false,
    };

    let key = "no_prune_key";
    let component = "no_prune_component";
    let data = b"data with no auto pruning";

    // Test put_with_config with auto pruning disabled
    cache
        .put_with_config(key, component, data, &no_prune_config)
        .unwrap();

    // Verify the data was stored correctly
    assert!(cache.exists(key, component));
    let retrieved = cache.get(key, component).unwrap();
    assert_eq!(retrieved, data);

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_put_with_config_multiple_entries() {
    let temp_dir = TempDir::new().unwrap();

    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    let config = PruningConfig {
        auto_prune_enabled: false,
        use_sync_pruning: false,
        is_test_environment: true,
    };

    // Store multiple entries with the same config
    let entries = vec![
        ("key1", "component1", b"data1" as &[u8]),
        ("key2", "component2", b"data2"),
        ("key3", "component1", b"data3"),
    ];

    for (key, component, data) in &entries {
        cache
            .put_with_config(key, component, data, &config)
            .unwrap();
    }

    // Verify all entries were stored correctly
    for (key, component, expected_data) in &entries {
        assert!(cache.exists(key, component));
        let retrieved = cache.get(key, component).unwrap();
        assert_eq!(retrieved, *expected_data);
    }

    // Verify index reflects all entries
    let stats = cache.get_stats();
    assert_eq!(stats.entry_count, 3);

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_put_with_config_overwrites_existing() {
    let temp_dir = TempDir::new().unwrap();

    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    let config = PruningConfig {
        auto_prune_enabled: false,
        use_sync_pruning: false,
        is_test_environment: false,
    };

    let key = "overwrite_key";
    let component = "overwrite_component";
    let original_data = b"original data";
    let new_data = b"updated data that is longer";

    // Store original data
    cache
        .put_with_config(key, component, original_data, &config)
        .unwrap();
    assert!(cache.exists(key, component));
    let retrieved = cache.get(key, component).unwrap();
    assert_eq!(retrieved, original_data);

    // Overwrite with new data
    cache
        .put_with_config(key, component, new_data, &config)
        .unwrap();
    assert!(cache.exists(key, component));
    let retrieved = cache.get(key, component).unwrap();
    assert_eq!(retrieved, new_data);

    // Verify index still shows only one entry
    let stats = cache.get_stats();
    assert_eq!(stats.entry_count, 1);

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cache_version_validation() {
    let temp_dir = TempDir::new().unwrap();

    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    // Create a cache and add an entry
    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
    cache
        .put("test_key", "test_component", b"test data")
        .unwrap();

    // Verify the entry exists
    assert!(cache.exists("test_key", "test_component"));
    let stats = cache.get_stats();
    assert_eq!(stats.entry_count, 1);

    // Create another cache instance - should validate version (same version, no clear)
    let cache2 = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
    assert!(cache2.exists("test_key", "test_component"));

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cache_clear() {
    let temp_dir = TempDir::new().unwrap();

    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    // Add multiple entries
    cache.put("key1", "component1", b"data1").unwrap();
    cache.put("key2", "component2", b"data2").unwrap();
    cache.put("key3", "component3", b"data3").unwrap();

    let stats = cache.get_stats();
    assert_eq!(stats.entry_count, 3);

    // Clear the entire cache
    cache.clear().unwrap();

    // Verify all entries are gone
    let stats = cache.get_stats();
    assert_eq!(stats.entry_count, 0);
    assert!(!cache.exists("key1", "component1"));
    assert!(!cache.exists("key2", "component2"));
    assert!(!cache.exists("key3", "component3"));

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_compute_cache_key_with_file() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, "fn main() {}").unwrap();

    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    let key = cache.compute_cache_key(&test_file).unwrap();
    assert!(key.contains("test.rs"));
    assert!(key.contains(":")); // Should have hash separator

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_compute_cache_key_without_file() {
    let temp_dir = TempDir::new().unwrap();
    let non_existent_path = temp_dir.path().join("non_existent.rs");

    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    let key = cache.compute_cache_key(&non_existent_path).unwrap();
    assert!(key.contains("non_existent.rs"));
    assert!(!key.contains(":")); // Should not have hash separator

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_put_with_config_large_data() {
    let temp_dir = TempDir::new().unwrap();

    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    let config = PruningConfig {
        auto_prune_enabled: false,
        use_sync_pruning: false,
        is_test_environment: true,
    };

    let key = "large_data_key";
    let component = "large_component";
    let large_data = vec![0u8; 1024 * 1024]; // 1MB of data

    // Test put_with_config with large data
    cache
        .put_with_config(key, component, &large_data, &config)
        .unwrap();

    // Verify the data was stored correctly
    assert!(cache.exists(key, component));
    let retrieved = cache.get(key, component).unwrap();
    assert_eq!(retrieved, large_data);

    // Verify index reflects the large size
    let stats = cache.get_stats();
    assert_eq!(stats.entry_count, 1);
    assert!(stats.total_size >= large_data.len() as u64);

    // Cleanup
    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cleanup_removes_oldest_entries() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let mut cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
    cache.max_cache_size = 100; // Set small size to trigger cleanup

    // Create large entries to ensure we exceed max_cache_size
    let large_data = vec![0u8; 40]; // Each entry is 40 bytes

    // Add old entries
    cache.put("old_key1", "component1", &large_data).unwrap();
    cache.put("old_key2", "component1", &large_data).unwrap();

    // Sleep briefly to ensure time difference
    std::thread::sleep(Duration::from_millis(10));

    // Add recent entries
    cache.put("recent_key1", "component1", &large_data).unwrap();
    cache.put("recent_key2", "component1", &large_data).unwrap();

    // Access recent entries to update their access time
    cache.get("recent_key1", "component1").unwrap();
    cache.get("recent_key2", "component1").unwrap();

    // Debug: Check actual size before cleanup
    let stats_before = cache.get_stats();
    eprintln!(
        "Before cleanup - entries: {}, size: {}",
        stats_before.entry_count, stats_before.total_size
    );

    // Total size should be ~160 bytes, max is 100, target after cleanup is 50
    // Manually trigger cleanup
    cache.cleanup().unwrap();

    // Debug: Check actual size after cleanup
    let stats_after = cache.get_stats();
    eprintln!(
        "After cleanup - entries: {}, size: {}",
        stats_after.entry_count, stats_after.total_size
    );

    // The cleanup should have removed some entries to get under target (50 bytes)
    assert!(
        stats_after.entry_count < stats_before.entry_count,
        "Cleanup should have removed entries: {} -> {}",
        stats_before.entry_count,
        stats_after.entry_count
    );
    assert!(
        stats_after.total_size <= cache.max_cache_size / 2,
        "Size should be under target: {} <= {}",
        stats_after.total_size,
        cache.max_cache_size / 2
    );

    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cleanup_target_size_calculation() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    // Create cache with specific max size
    let mut cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
    cache.max_cache_size = 1000; // Set a small size for testing

    // Add entries that exceed half the max size
    let data = vec![0u8; 300]; // Each entry is 300 bytes
    cache.put("key1", "component", &data).unwrap();
    cache.put("key2", "component", &data).unwrap();
    cache.put("key3", "component", &data).unwrap();

    // Total size should be ~900 bytes, target after cleanup is 500

    // Run cleanup
    cache.cleanup().unwrap();

    // Verify total size is now under target (500 bytes)
    let stats = cache.get_stats();
    assert!(stats.total_size <= 500);

    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cleanup_handles_empty_cache() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    // Cleanup on empty cache should not error
    let result = cache.cleanup();
    assert!(result.is_ok());

    // Cache should still be empty
    let stats = cache.get_stats();
    assert_eq!(stats.entry_count, 0);
    assert_eq!(stats.total_size, 0);

    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cleanup_removes_files_from_all_components() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    // Add entries to different components
    let components = vec![
        "call_graphs",
        "analysis",
        "metadata",
        "temp",
        "file_metrics",
        "test",
    ];

    let key = "test_key";
    let data = b"test_data";

    // Add to each component
    for component in &components {
        cache.put(key, component, data).unwrap();
        assert!(cache.exists(key, component));
    }

    // Force cache size to be large enough to trigger cleanup
    {
        let index_arc = cache.index_manager.get_index_arc();
        let mut index = index_arc.write().unwrap();
        index.total_size = cache.max_cache_size + 1;
    }

    // Run cleanup
    cache.cleanup().unwrap();

    // Verify all component files are removed
    for component in &components {
        assert!(!cache.exists(key, component));
    }

    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cleanup_updates_index_correctly() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let mut cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
    cache.max_cache_size = 200; // Increase size to allow all entries to be added first

    // Add multiple entries
    let entries: Vec<(&str, Vec<u8>)> = vec![
        ("key1", vec![0u8; 20]),
        ("key2", vec![0u8; 20]),
        ("key3", vec![0u8; 20]),
        ("key4", vec![0u8; 20]),
    ];

    for (key, data) in &entries {
        cache.put(key, "component", data).unwrap();
    }

    let initial_stats = cache.get_stats();
    assert!(initial_stats.entry_count > 0, "Should have entries");

    // Now reduce max_cache_size to force cleanup
    cache.max_cache_size = 50;

    // Run cleanup - should remove entries to get under 25 bytes (50% of max)
    cache.cleanup().unwrap();

    // Verify index is updated
    let final_stats = cache.get_stats();
    assert!(
        final_stats.entry_count < initial_stats.entry_count,
        "Entry count should decrease: {} -> {}",
        initial_stats.entry_count,
        final_stats.entry_count
    );
    assert!(
        final_stats.total_size <= cache.max_cache_size / 2,
        "Total size should be under target: {} <= {}",
        final_stats.total_size,
        cache.max_cache_size / 2
    );

    // Verify last_cleanup is set
    {
        let index_arc = cache.index_manager.get_index_arc();
        let index = index_arc.read().unwrap();
        assert!(index.last_cleanup.is_some());
    }

    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cleanup_handles_concurrent_file_access() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    // Add an entry
    let key = "concurrent_key";
    let component = "component";
    cache.put(key, component, b"data").unwrap();

    // Manually remove the file to simulate concurrent deletion
    let cache_path = cache.get_cache_file_path(key, component);
    if cache_path.exists() {
        fs::remove_file(&cache_path).ok();
    }

    // Cleanup should handle the missing file gracefully
    let result = cache.cleanup();
    assert!(result.is_ok());

    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cleanup_preserves_entries_under_target_size() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

    let mut cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
    cache.max_cache_size = 1000;

    // Add entries that total less than half the max size (target = 500)
    cache.put("keep1", "component", b"small").unwrap();
    cache.put("keep2", "component", b"data").unwrap();

    let initial_count = cache.get_stats().entry_count;

    // Run cleanup
    cache.cleanup().unwrap();

    // All entries should be preserved since we're under target
    let final_count = cache.get_stats().entry_count;
    assert_eq!(initial_count, final_count);
    assert!(cache.exists("keep1", "component"));
    assert!(cache.exists("keep2", "component"));

    std::env::remove_var("DEBTMAP_CACHE_DIR");
    std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
}

#[test]
fn test_cleanup_pure_functions_behavior() {
    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};

    // Test sort_entries_by_access_time
    let now = SystemTime::now();
    let old_time = now - Duration::from_secs(3600); // 1 hour ago
    let very_old_time = now - Duration::from_secs(7200); // 2 hours ago

    let mut entries = HashMap::new();
    entries.insert(
        "newest".to_string(),
        CacheMetadata {
            version: "1.0".to_string(),
            created_at: now,
            last_accessed: now,
            access_count: 1,
            size_bytes: 10,
            debtmap_version: "0.2.0".to_string(),
        },
    );
    entries.insert(
        "oldest".to_string(),
        CacheMetadata {
            version: "1.0".to_string(),
            created_at: very_old_time,
            last_accessed: very_old_time,
            access_count: 1,
            size_bytes: 20,
            debtmap_version: "0.2.0".to_string(),
        },
    );
    entries.insert(
        "middle".to_string(),
        CacheMetadata {
            version: "1.0".to_string(),
            created_at: old_time,
            last_accessed: old_time,
            access_count: 1,
            size_bytes: 15,
            debtmap_version: "0.2.0".to_string(),
        },
    );

    // Sort entries by access time (oldest first)
    let mut sorted: Vec<(String, CacheMetadata)> = entries.into_iter().collect();
    sorted.sort_by_key(|(_, metadata)| metadata.last_accessed);

    // Should be sorted with oldest first
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0, "oldest");
    assert_eq!(sorted[1].0, "middle");
    assert_eq!(sorted[2].0, "newest");

    // Test select_keys_for_removal
    let keys_to_remove = SharedCache::select_keys_for_removal(sorted.clone(), 25, 45);

    // With total size 45 and target 25, should remove oldest (20 bytes) to get to 25
    assert_eq!(keys_to_remove.len(), 1);
    assert_eq!(keys_to_remove[0], "oldest");

    // Test with smaller target - should remove multiple entries
    let keys_to_remove_multiple = SharedCache::select_keys_for_removal(sorted, 10, 45);

    // Should remove oldest (20) + middle (15) = 35, leaving 10 which is under target
    assert_eq!(keys_to_remove_multiple.len(), 2);
    assert_eq!(keys_to_remove_multiple[0], "oldest");
    assert_eq!(keys_to_remove_multiple[1], "middle");
}

// Tests for copy_dir_recursive
#[cfg(test)]
mod copy_dir_recursive_tests {
    use super::*;
    use std::fs;

    fn create_test_cache() -> (SharedCache, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        std::env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "false");

        let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();
        (cache, temp_dir)
    }

    fn create_test_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    fn create_test_dir(base: &Path, name: &str) -> PathBuf {
        let path = base.join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_copy_single_file() {
        let (cache, temp_dir) = create_test_cache();

        // Create source directory with a single file
        let src_dir = create_test_dir(temp_dir.path(), "source");
        create_test_file(&src_dir, "file.txt", "test content");

        // Create destination directory
        let dest_dir = create_test_dir(temp_dir.path(), "destination");

        // Copy the directory
        cache.copy_dir_recursive(&src_dir, &dest_dir).unwrap();

        // Verify the file was copied
        let dest_file = dest_dir.join("file.txt");
        assert!(dest_file.exists());
        let content = fs::read_to_string(dest_file).unwrap();
        assert_eq!(content, "test content");

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_copy_directory_with_multiple_files() {
        let (cache, temp_dir) = create_test_cache();

        // Create source directory with multiple files
        let src_dir = create_test_dir(temp_dir.path(), "source");
        create_test_file(&src_dir, "file1.txt", "content 1");
        create_test_file(&src_dir, "file2.txt", "content 2");
        create_test_file(&src_dir, "file3.txt", "content 3");

        // Create destination directory
        let dest_dir = create_test_dir(temp_dir.path(), "destination");

        // Copy the directory
        cache.copy_dir_recursive(&src_dir, &dest_dir).unwrap();

        // Verify all files were copied
        assert!(dest_dir.join("file1.txt").exists());
        assert!(dest_dir.join("file2.txt").exists());
        assert!(dest_dir.join("file3.txt").exists());

        assert_eq!(
            fs::read_to_string(dest_dir.join("file1.txt")).unwrap(),
            "content 1"
        );
        assert_eq!(
            fs::read_to_string(dest_dir.join("file2.txt")).unwrap(),
            "content 2"
        );
        assert_eq!(
            fs::read_to_string(dest_dir.join("file3.txt")).unwrap(),
            "content 3"
        );

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_copy_nested_directories() {
        let (cache, temp_dir) = create_test_cache();

        // Create nested directory structure
        let src_dir = create_test_dir(temp_dir.path(), "source");
        create_test_file(&src_dir, "root.txt", "root content");

        let level1 = create_test_dir(&src_dir, "level1");
        create_test_file(&level1, "level1.txt", "level1 content");

        let level2 = create_test_dir(&level1, "level2");
        create_test_file(&level2, "level2.txt", "level2 content");

        // Create destination directory
        let dest_dir = create_test_dir(temp_dir.path(), "destination");

        // Copy the directory recursively
        cache.copy_dir_recursive(&src_dir, &dest_dir).unwrap();

        // Verify nested structure was copied
        assert!(dest_dir.join("root.txt").exists());
        assert!(dest_dir.join("level1").exists());
        assert!(dest_dir.join("level1/level1.txt").exists());
        assert!(dest_dir.join("level1/level2").exists());
        assert!(dest_dir.join("level1/level2/level2.txt").exists());

        // Verify content
        assert_eq!(
            fs::read_to_string(dest_dir.join("root.txt")).unwrap(),
            "root content"
        );
        assert_eq!(
            fs::read_to_string(dest_dir.join("level1/level1.txt")).unwrap(),
            "level1 content"
        );
        assert_eq!(
            fs::read_to_string(dest_dir.join("level1/level2/level2.txt")).unwrap(),
            "level2 content"
        );

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_copy_empty_directory() {
        let (cache, temp_dir) = create_test_cache();

        // Create empty source directory
        let src_dir = create_test_dir(temp_dir.path(), "source");

        // Create destination directory
        let dest_dir = create_test_dir(temp_dir.path(), "destination");

        // Copy the empty directory (should succeed with no operations)
        let result = cache.copy_dir_recursive(&src_dir, &dest_dir);
        assert!(result.is_ok());

        // Verify destination exists and is empty
        assert!(dest_dir.exists());
        let entries: Vec<_> = fs::read_dir(&dest_dir).unwrap().collect();
        assert_eq!(entries.len(), 0);

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_copy_error_source_not_found() {
        let (cache, temp_dir) = create_test_cache();

        // Try to copy from non-existent source
        let src_dir = temp_dir.path().join("nonexistent");
        let dest_dir = create_test_dir(temp_dir.path(), "destination");

        // Should fail with error
        let result = cache.copy_dir_recursive(&src_dir, &dest_dir);
        assert!(result.is_err());

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_copy_mixed_files_and_directories() {
        let (cache, temp_dir) = create_test_cache();

        // Create source with mixed content
        let src_dir = create_test_dir(temp_dir.path(), "source");
        create_test_file(&src_dir, "file1.txt", "file content 1");

        let subdir1 = create_test_dir(&src_dir, "subdir1");
        create_test_file(&subdir1, "file2.txt", "file content 2");

        create_test_file(&src_dir, "file3.txt", "file content 3");

        let subdir2 = create_test_dir(&src_dir, "subdir2");
        create_test_file(&subdir2, "file4.txt", "file content 4");

        // Create destination directory
        let dest_dir = create_test_dir(temp_dir.path(), "destination");

        // Copy the directory
        cache.copy_dir_recursive(&src_dir, &dest_dir).unwrap();

        // Verify structure
        assert!(dest_dir.join("file1.txt").exists());
        assert!(dest_dir.join("file3.txt").exists());
        assert!(dest_dir.join("subdir1").is_dir());
        assert!(dest_dir.join("subdir1/file2.txt").exists());
        assert!(dest_dir.join("subdir2").is_dir());
        assert!(dest_dir.join("subdir2/file4.txt").exists());

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_copy_deeply_nested_structure() {
        let (cache, temp_dir) = create_test_cache();

        // Create deeply nested structure (5 levels)
        let src_dir = create_test_dir(temp_dir.path(), "source");
        let mut current_dir = src_dir.clone();

        for i in 1..=5 {
            create_test_file(
                &current_dir,
                &format!("file_{}.txt", i),
                &format!("content {}", i),
            );
            current_dir = create_test_dir(&current_dir, &format!("level{}", i));
        }

        // Create destination directory
        let dest_dir = create_test_dir(temp_dir.path(), "destination");

        // Copy the deeply nested directory
        cache.copy_dir_recursive(&src_dir, &dest_dir).unwrap();

        // Verify the deepest file exists
        assert!(dest_dir.join("level1/level2/level3/level4/level5").exists());

        // Verify files at each level
        for i in 1..=5 {
            let mut path = dest_dir.clone();
            for j in 1..i {
                path = path.join(format!("level{}", j));
            }
            path = path.join(format!("file_{}.txt", i));
            assert!(path.exists(), "File should exist at: {:?}", path);
        }

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_copy_preserves_file_contents() {
        let (cache, temp_dir) = create_test_cache();

        // Create source with various file types and contents
        let src_dir = create_test_dir(temp_dir.path(), "source");
        create_test_file(&src_dir, "text.txt", "Simple text content");
        create_test_file(&src_dir, "empty.txt", "");
        create_test_file(&src_dir, "multiline.txt", "Line 1\nLine 2\nLine 3");

        // Binary-like content
        let binary_content = "Binary\0content\nwith\0nulls";
        fs::write(src_dir.join("binary.dat"), binary_content).unwrap();

        // Create destination directory
        let dest_dir = create_test_dir(temp_dir.path(), "destination");

        // Copy the directory
        cache.copy_dir_recursive(&src_dir, &dest_dir).unwrap();

        // Verify all contents match exactly
        assert_eq!(
            fs::read_to_string(dest_dir.join("text.txt")).unwrap(),
            "Simple text content"
        );
        assert_eq!(fs::read_to_string(dest_dir.join("empty.txt")).unwrap(), "");
        assert_eq!(
            fs::read_to_string(dest_dir.join("multiline.txt")).unwrap(),
            "Line 1\nLine 2\nLine 3"
        );
        assert_eq!(
            fs::read(dest_dir.join("binary.dat")).unwrap(),
            binary_content.as_bytes()
        );

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_copy_with_empty_subdirectories() {
        let (cache, temp_dir) = create_test_cache();

        // Create source with empty subdirectories
        let src_dir = create_test_dir(temp_dir.path(), "source");
        create_test_file(&src_dir, "root.txt", "root");

        let empty_dir1 = create_test_dir(&src_dir, "empty1");
        let _ = empty_dir1; // Silence unused warning

        let subdir = create_test_dir(&src_dir, "subdir");
        create_test_file(&subdir, "file.txt", "content");

        let empty_dir2 = create_test_dir(&subdir, "empty2");
        let _ = empty_dir2; // Silence unused warning

        // Create destination directory
        let dest_dir = create_test_dir(temp_dir.path(), "destination");

        // Copy the directory
        cache.copy_dir_recursive(&src_dir, &dest_dir).unwrap();

        // Verify structure including empty directories
        assert!(dest_dir.join("root.txt").exists());
        assert!(dest_dir.join("empty1").is_dir());
        assert!(dest_dir.join("subdir").is_dir());
        assert!(dest_dir.join("subdir/file.txt").exists());
        assert!(dest_dir.join("subdir/empty2").is_dir());

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }

    #[test]
    fn test_copy_handles_special_filenames() {
        let (cache, temp_dir) = create_test_cache();

        // Create source with various filename patterns
        let src_dir = create_test_dir(temp_dir.path(), "source");

        // Various special but valid filenames
        create_test_file(&src_dir, "file with spaces.txt", "spaces");
        create_test_file(&src_dir, "file-with-dashes.txt", "dashes");
        create_test_file(&src_dir, "file_with_underscores.txt", "underscores");
        create_test_file(&src_dir, "file.multiple.dots.txt", "dots");

        // Create destination directory
        let dest_dir = create_test_dir(temp_dir.path(), "destination");

        // Copy the directory
        cache.copy_dir_recursive(&src_dir, &dest_dir).unwrap();

        // Verify all files were copied correctly
        assert!(dest_dir.join("file with spaces.txt").exists());
        assert!(dest_dir.join("file-with-dashes.txt").exists());
        assert!(dest_dir.join("file_with_underscores.txt").exists());
        assert!(dest_dir.join("file.multiple.dots.txt").exists());

        std::env::remove_var("DEBTMAP_CACHE_DIR");
        std::env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    }
}

#[cfg(test)]
mod pure_function_tests {
    use super::*;
    use crate::cache::shared_cache::{build_dest_path, classify_entry, EntryType};

    #[test]
    fn test_classify_entry_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test").unwrap();

        assert_eq!(classify_entry(&file_path), EntryType::File);
    }

    #[test]
    fn test_classify_entry_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("testdir");
        fs::create_dir(&dir_path).unwrap();

        assert_eq!(classify_entry(&dir_path), EntryType::Directory);
    }

    #[test]
    fn test_classify_entry_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent");

        assert_eq!(classify_entry(&nonexistent), EntryType::Other);
    }

    #[test]
    fn test_build_dest_path_simple() {
        let base = Path::new("/tmp/dest");
        let name = std::ffi::OsStr::new("file.txt");

        let result = build_dest_path(base, name);
        assert_eq!(result, Path::new("/tmp/dest/file.txt"));
    }

    #[test]
    fn test_build_dest_path_with_spaces() {
        let base = Path::new("/tmp/dest");
        let name = std::ffi::OsStr::new("file with spaces.txt");

        let result = build_dest_path(base, name);
        assert_eq!(result, Path::new("/tmp/dest/file with spaces.txt"));
    }

    #[test]
    fn test_build_dest_path_nested() {
        let base = Path::new("/tmp/dest/nested");
        let name = std::ffi::OsStr::new("file.txt");

        let result = build_dest_path(base, name);
        assert_eq!(result, Path::new("/tmp/dest/nested/file.txt"));
    }
}

#[cfg(test)]
mod io_function_tests {
    use super::*;
    use crate::cache::shared_cache::{copy_dir_entry, copy_file_entry};

    #[test]
    fn test_copy_file_entry_success() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        fs::write(&src, "test content").unwrap();

        let result = copy_file_entry(&src, &dest);
        assert!(result.is_ok());
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "test content");
    }

    #[test]
    fn test_copy_file_entry_source_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("nonexistent.txt");
        let dest = temp_dir.path().join("dest.txt");

        let result = copy_file_entry(&src, &dest);
        assert!(result.is_err());
    }

    #[test]
    fn test_copy_dir_entry_success() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("newdir");

        let result = copy_dir_entry(&dest);
        assert!(result.is_ok());
        assert!(dest.exists());
        assert!(dest.is_dir());
    }

    #[test]
    fn test_copy_dir_entry_nested() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("nested").join("dirs").join("deep");

        let result = copy_dir_entry(&dest);
        assert!(result.is_ok());
        assert!(dest.exists());
        assert!(dest.is_dir());
    }

    #[test]
    fn test_copy_dir_entry_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("existing");

        fs::create_dir(&dest).unwrap();

        let result = copy_dir_entry(&dest);
        assert!(result.is_ok());
        assert!(dest.exists());
    }
}
