use debtmap::cache::{AutoPruner, PruneStrategy, SharedCache};
use std::env;
use tempfile::TempDir;

fn main() {
    // Set up environment
    let temp_dir = TempDir::new().unwrap();
    env::set_var("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
    env::set_var("DEBTMAP_CACHE_AUTO_PRUNE", "true");
    env::set_var("DEBTMAP_CACHE_MAX_SIZE", "1000"); // 1KB limit

    println!("Creating cache with auto-pruning enabled...");
    let cache = SharedCache::new(None).unwrap();
    
    println!("Initial stats: {:?}", cache.get_stats());
    println!("Auto-pruner config: {:?}", cache);
    
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

    // Clean up
    env::remove_var("DEBTMAP_CACHE_DIR");
    env::remove_var("DEBTMAP_CACHE_AUTO_PRUNE");
    env::remove_var("DEBTMAP_CACHE_MAX_SIZE");
}