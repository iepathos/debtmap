use std::time::{SystemTime, Duration};
use std::collections::HashMap;

// Copy the relevant types and functions for testing
#[derive(Debug, Clone)]
pub struct CacheMetadata {
    pub version: String,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub access_count: u64,
    pub size_bytes: u64,
}

/// Calculate the maximum age duration from days
fn calculate_max_age_duration(max_age_days: i64) -> Duration {
    Duration::from_secs(max_age_days as u64 * 86400)
}

/// Determine if an entry should be removed based on age
fn should_remove_entry_by_age(
    now: SystemTime,
    last_accessed: SystemTime,
    max_age: Duration,
) -> bool {
    now.duration_since(last_accessed)
        .map(|age| age >= max_age)  // Use >= to handle zero-age case
        .unwrap_or(false)  // If time calculation fails, don't remove
}

/// Filter entries to find those that should be removed based on age
fn filter_entries_by_age(
    entries: &HashMap<String, CacheMetadata>,
    now: SystemTime,
    max_age: Duration,
) -> Vec<String> {
    entries
        .iter()
        .filter_map(|(key, metadata)| {
            if should_remove_entry_by_age(now, metadata.last_accessed, max_age) {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect()
}

fn main() {
    println!("Testing refactored age-based cleanup logic...");
    
    let now = SystemTime::now();
    
    // Create test entries - some created now, some created 100 seconds ago
    let mut entries = HashMap::new();
    
    // Entries created at "now" (zero age)
    for i in 0..3 {
        let key = format!("new_entry_{}", i);
        entries.insert(key, CacheMetadata {
            version: "1.0".to_string(),
            created_at: now,
            last_accessed: now,  // Created and accessed at same time
            access_count: 1,
            size_bytes: 100,
        });
    }
    
    // Entries created 100 seconds ago
    let old_time = now - Duration::from_secs(100);
    for i in 0..2 {
        let key = format!("old_entry_{}", i);
        entries.insert(key, CacheMetadata {
            version: "1.0".to_string(),
            created_at: old_time,
            last_accessed: old_time,
            access_count: 1,
            size_bytes: 100,
        });
    }
    
    println!("Created {} test entries", entries.len());
    
    // Test 1: max_age_days = 0 (should remove all entries)
    let max_age_0 = calculate_max_age_duration(0);
    let to_remove_0 = filter_entries_by_age(&entries, now, max_age_0);
    println!("With max_age_days=0: {} entries to remove (expected: 5)", to_remove_0.len());
    assert_eq!(to_remove_0.len(), 5, "Should remove all 5 entries when max_age_days=0");
    
    // Test 2: max_age_days corresponding to 50 seconds (should remove only old entries)
    let max_age_50s = Duration::from_secs(50);
    let to_remove_50s = filter_entries_by_age(&entries, now, max_age_50s);
    println!("With max_age=50s: {} entries to remove (expected: 2)", to_remove_50s.len());
    assert_eq!(to_remove_50s.len(), 2, "Should remove only the 2 old entries");
    
    // Test 3: max_age_days corresponding to 200 seconds (should remove no entries)
    let max_age_200s = Duration::from_secs(200);
    let to_remove_200s = filter_entries_by_age(&entries, now, max_age_200s);
    println!("With max_age=200s: {} entries to remove (expected: 0)", to_remove_200s.len());
    assert_eq!(to_remove_200s.len(), 0, "Should remove no entries");
    
    println!("\n All tests passed! The refactored logic correctly handles:");
    println!("   - Zero-age entries (same creation and check time)");
    println!("   - Edge case where max_age_days = 0");
    println!("   - Proper >= comparison instead of >");
    println!("   - Functional programming style with pure functions");
}