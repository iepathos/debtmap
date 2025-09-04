use debtmap::cache::{AutoPruner, PruneStrategy, SharedCache};
use std::env;
use std::time::Duration;
use tempfile::TempDir;

mod helpers;
use helpers::cache_isolation::EnvGuard;

#[test]
fn test_auto_pruning_size_limit() {
    let temp_dir = TempDir::new().unwrap();
    let mut env_guard = EnvGuard::new();
    env_guard.set("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    env_guard.set("DEBTMAP_CACHE_AUTO_PRUNE", "true");
    env_guard.set("DEBTMAP_CACHE_MAX_SIZE", "1000"); // 1KB limit

    let cache = SharedCache::new(None).unwrap();

    // Add entries that exceed the size limit
    for i in 0..10 {
        let key = format!("key_{}", i);
        let data = vec![0u8; 200]; // 200 bytes each
        cache.put(&key, "test_component", &data).unwrap();
    }

    // Should have triggered pruning
    let stats = cache.get_stats();
    assert!(
        stats.total_size <= 1000,
        "Cache size should be pruned to under 1KB"
    );

    // Automatic cleanup when env_guard is dropped
}

#[test]
fn test_auto_pruning_entry_count_limit() {
    let temp_dir = TempDir::new().unwrap();
    let mut env_guard = EnvGuard::new();
    env_guard.set("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    env_guard.set("DEBTMAP_CACHE_AUTO_PRUNE", "true");
    env_guard.set("DEBTMAP_CACHE_MAX_ENTRIES", "5");
    
    // Enable debug output for post-insertion pruning
    env_guard.set("DEBTMAP_CACHE_SYNC_PRUNE", "true");

    let cache = SharedCache::new(None).unwrap();

    // Add more entries than the limit
    for i in 0..10 {
        let key = format!("entry_{}", i);
        let data = b"test data";
        println!("About to add entry {}, DEBTMAP_CACHE_AUTO_PRUNE={}", i, env::var("DEBTMAP_CACHE_AUTO_PRUNE").unwrap_or("NOT_SET".to_string()));
        cache.put(&key, "test_component", data).unwrap();
        let stats = cache.get_stats();
        println!("After adding entry {}: entry_count={}, total_size={}", i, stats.entry_count, stats.total_size);
        if stats.entry_count > 5 {
            println!("WARNING: Entry count exceeded limit after adding entry {}", i);
        }
    }

    // Should have triggered pruning
    let stats = cache.get_stats();
    println!("Final stats: entry_count={}, limit was 5", stats.entry_count);
    
    // Debug: manually trigger pruning to see what happens
    if stats.entry_count > 5 {
        println!("DEBUG: Manually triggering pruning to investigate...");
        let manual_prune_stats = cache.trigger_pruning().unwrap();
        println!("Manual pruning results: {:?}", manual_prune_stats);
        let new_stats = cache.get_stats();
        println!("Stats after manual pruning: entry_count={}", new_stats.entry_count);
    }
    
    assert!(
        stats.entry_count <= 5,
        "Entry count should be pruned to under 5, got {}", stats.entry_count
    );

    // Automatic cleanup when env_guard is dropped
}

#[test]
fn test_pruning_strategies() {
    let temp_dir = TempDir::new().unwrap();

    // Test LRU strategy
    {
        let mut env_guard = EnvGuard::new();
        env_guard.set("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        env_guard.set("DEBTMAP_CACHE_AUTO_PRUNE", "true");
        env_guard.set("DEBTMAP_CACHE_SYNC_PRUNE", "true");
        env_guard.set("DEBTMAP_CACHE_STRATEGY", "lru");
        env_guard.set("DEBTMAP_CACHE_MAX_ENTRIES", "3");

        let cache = SharedCache::new(None).unwrap();

        // Add entries with different access patterns
        cache.put("old_1", "test", b"data").unwrap();
        std::thread::sleep(Duration::from_millis(10));
        cache.put("old_2", "test", b"data").unwrap();
        std::thread::sleep(Duration::from_millis(10));
        cache.put("recent", "test", b"data").unwrap();

        // Access the first entry to make it more recently used
        let _ = cache.get("old_1", "test");

        // Add one more to trigger pruning
        println!("Adding trigger entry to test LRU...");
        cache.put("trigger", "test", b"data").unwrap();

        let stats = cache.get_stats();
        println!("After trigger entry: entry_count={}, expected=3", stats.entry_count);
        assert_eq!(stats.entry_count, 3, "Should keep only 3 entries");

        // old_2 should be removed (least recently used)
        assert!(!cache.exists("old_2", "test"));

        // Automatic cleanup when env_guard is dropped
    }

    // Test FIFO strategy
    {
        let temp_dir2 = TempDir::new().unwrap();
        let mut env_guard = EnvGuard::new();
        env_guard.set("DEBTMAP_CACHE_DIR", temp_dir2.path().to_str().unwrap());
        env_guard.set("DEBTMAP_CACHE_AUTO_PRUNE", "true");
        env_guard.set("DEBTMAP_CACHE_SYNC_PRUNE", "true");
        env_guard.set("DEBTMAP_CACHE_STRATEGY", "fifo");
        env_guard.set("DEBTMAP_CACHE_MAX_ENTRIES", "3");

        let cache = SharedCache::new(None).unwrap();

        // Add entries in order
        cache.put("first", "test", b"data").unwrap();
        cache.put("second", "test", b"data").unwrap();
        cache.put("third", "test", b"data").unwrap();
        cache.put("fourth", "test", b"data").unwrap();

        let stats = cache.get_stats();
        assert_eq!(stats.entry_count, 3, "Should keep only 3 entries");

        // First entry should be removed (FIFO)
        assert!(!cache.exists("first", "test"));
        assert!(cache.exists("fourth", "test"));

        // Automatic cleanup when env_guard is dropped
    }

    // Automatic cleanup when temp_dir is dropped
}

#[test]
fn test_manual_pruning_trigger() {
    let temp_dir = TempDir::new().unwrap();
    let mut env_guard = EnvGuard::new();
    env_guard.set("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());

    let pruner = AutoPruner {
        max_size_bytes: 500,
        max_age_days: 30,
        max_entries: 5,
        prune_percentage: 0.5,
        strategy: PruneStrategy::Lru,
    };

    let cache = SharedCache::with_auto_pruning(None, pruner).unwrap();

    // Add entries
    for i in 0..10 {
        let key = format!("manual_{}", i);
        let data = vec![0u8; 100];
        cache.put(&key, "test", &data).unwrap();
    }

    // Manually trigger pruning
    let prune_stats = cache.trigger_pruning().unwrap();

    assert!(
        prune_stats.entries_removed > 0,
        "Should have removed entries"
    );
    assert!(prune_stats.bytes_freed > 0, "Should have freed bytes");
    assert!(
        prune_stats.entries_remaining <= 5,
        "Should have at most 5 entries"
    );

    // Automatic cleanup when env_guard is dropped
}

#[test]
fn test_age_based_pruning() {
    let temp_dir = TempDir::new().unwrap();
    let mut env_guard = EnvGuard::new();
    env_guard.set("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());

    let cache = SharedCache::new(None).unwrap();

    // Add some entries
    for i in 0..5 {
        let key = format!("age_test_{}", i);
        cache.put(&key, "test", b"data").unwrap();
    }

    // Cleanup entries older than 0 days (should remove all)
    let removed = cache.cleanup_old_entries(0).unwrap();
    assert_eq!(removed, 5, "Should remove all 5 entries");

    let stats = cache.get_stats();
    assert_eq!(stats.entry_count, 0, "Cache should be empty");

    // Automatic cleanup when env_guard is dropped
}

#[test]
fn test_prune_stats_display() {
    use debtmap::cache::PruneStats;

    let stats = PruneStats {
        entries_removed: 100,
        bytes_freed: 1024 * 1024 * 5, // 5MB
        entries_remaining: 50,
        bytes_remaining: 1024 * 1024 * 2, // 2MB
        duration_ms: 150,
        files_deleted: 100,
        files_not_found: 5,
    };

    let display = format!("{}", stats);
    assert!(display.contains("100 entries"));
    assert!(display.contains("5 MB"));
    assert!(display.contains("150ms"));
    assert!(display.contains("50 entries"));
    assert!(display.contains("2 MB"));
}

#[test]
fn debug_auto_pruning_issue() {
    let temp_dir = TempDir::new().unwrap();
    let mut env_guard = EnvGuard::new();
    env_guard.set("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    env_guard.set("DEBTMAP_CACHE_AUTO_PRUNE", "true");
    env_guard.set("DEBTMAP_CACHE_MAX_SIZE", "1000"); // 1KB limit

    println!("Creating cache with auto-pruning enabled...");
    let cache = SharedCache::new(None).unwrap();
    
    println!("Initial stats: {:?}", cache.get_stats());
    
    // Add entries that exceed the size limit
    for i in 0..10 {
        let key = format!("key_{}", i);
        let data = vec![0u8; 200]; // 200 bytes each
        println!("Adding entry {} (200 bytes)", i);
        cache.put(&key, "test_component", &data).unwrap();
        let stats = cache.get_stats();
        println!("Stats after entry {}: entry_count={}, total_size={}", i, stats.entry_count, stats.total_size);
    }

    // Check final stats
    let stats = cache.get_stats();
    println!("Final stats: entry_count={}, total_size={}", stats.entry_count, stats.total_size);
    
    if stats.total_size > 1000 {
        println!("❌ Cache size {} exceeds limit of 1000 bytes", stats.total_size);
        
        // Try manual pruning
        println!("Attempting manual pruning...");
        let prune_stats = cache.trigger_pruning().unwrap();
        println!("Prune stats: {:?}", prune_stats);
        
        let new_stats = cache.get_stats();
        println!("Stats after manual pruning: entry_count={}, total_size={}", new_stats.entry_count, new_stats.total_size);
    } else {
        println!("✅ Cache size {} is within limit", stats.total_size);
    }

    // Automatic cleanup when env_guard is dropped
}

#[test]
fn test_disabled_auto_pruning() {
    let temp_dir = TempDir::new().unwrap();
    let mut env_guard = EnvGuard::new();
    env_guard.set("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    env_guard.set("DEBTMAP_CACHE_AUTO_PRUNE", "false");
    env_guard.set("DEBTMAP_CACHE_MAX_ENTRIES", "1");

    let cache = SharedCache::new(None).unwrap();

    // Add more entries than limit
    for i in 0..5 {
        let key = format!("disabled_{}", i);
        cache.put(&key, "test", b"data").unwrap();
    }

    // Should NOT have triggered pruning (auto-prune disabled)
    let stats = cache.get_stats();
    assert_eq!(
        stats.entry_count, 5,
        "All entries should remain when auto-prune is disabled"
    );

    // Automatic cleanup when env_guard is dropped
}

#[test]
fn test_concurrent_pruning() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = TempDir::new().unwrap();
    let mut env_guard = EnvGuard::new();
    env_guard.set("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    env_guard.set("DEBTMAP_CACHE_AUTO_PRUNE", "true");
    env_guard.set("DEBTMAP_CACHE_SYNC_PRUNE", "true");

    let pruner = AutoPruner {
        max_size_bytes: 2000,
        max_entries: 10,
        max_age_days: 30,
        prune_percentage: 0.3,
        strategy: PruneStrategy::Lru,
    };

    let cache = Arc::new(SharedCache::with_auto_pruning(None, pruner).unwrap());

    // Spawn threads to concurrently add entries
    let mut handles = vec![];
    for thread_id in 0..5 {
        let cache_clone = Arc::clone(&cache);
        let handle = thread::spawn(move || {
            for i in 0..5 {
                let key = format!("thread_{}_entry_{}", thread_id, i);
                let data = vec![0u8; 100];
                cache_clone.put(&key, "test", &data).unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Check that pruning maintained limits
    let stats = cache.get_stats();
    assert!(
        stats.entry_count <= 10,
        "Should maintain entry limit with concurrent access"
    );
    assert!(
        stats.total_size <= 2000,
        "Should maintain size limit with concurrent access"
    );

    // Automatic cleanup when env_guard is dropped
}
