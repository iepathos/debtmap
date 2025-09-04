#[cfg(test)]
mod debug_pruning {
    use std::env;
    use tempfile::TempDir;
    use debtmap::cache::SharedCache;

    #[test]
    fn debug_pruning_trigger_logic() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
        env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "true");
        env::set_var("DEBTMAP_CACHE_MAX_ENTRIES", "3");

        let cache = SharedCache::new(None).unwrap();

        println!("=== Adding 3 entries (should be at limit) ===");
        for i in 1..=3 {
            cache.put(&format!("entry_{}", i), "test", b"data").unwrap();
            let stats = cache.get_stats();
            println!("After entry {}: count={}", i, stats.entry_count);
        }

        println!("\n=== Adding 4th entry (should trigger pruning) ===");
        cache.put("trigger_entry", "test", b"data").unwrap();
        let stats = cache.get_stats();
        println!("After trigger entry: count={}", stats.entry_count);
        
        // Let's also manually check what the pruner thinks
        if let Some(ref pruner) = cache.auto_pruner {
            let index = cache.index.read().unwrap();
            let should_prune = pruner.should_prune(&index);
            println!("Pruner thinks should_prune: {}", should_prune);
            println!("Current entry count: {}, max_entries: {}", index.entries.len(), pruner.max_entries);
        }

        env::remove_var("DEBTMAP_CACHE_DIR");
        env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
        env::remove_var("DEBTMAP_CACHE_MAX_ENTRIES");
    }
}