// Tests for file analysis progress tracking logic
// These tests verify the throttling and progress tracking patterns
// used in analyze_files_for_debt without requiring full UnifiedAnalysis setup

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn test_progress_throttling_every_10_files() {
    // Simulate the throttling logic from analyze_files_for_debt
    let total_files = 25;
    let update_count = Arc::new(AtomicUsize::new(0));
    let update_count_clone = update_count.clone();
    let mut last_update = Instant::now();

    for idx in 0..total_files {
        // Simulate file processing
        std::thread::sleep(Duration::from_micros(10));

        // Throttled progress updates (every 10 files or 100ms)
        if (idx + 1) % 10 == 0 || last_update.elapsed() > Duration::from_millis(100) {
            update_count_clone.fetch_add(1, Ordering::Relaxed);
            last_update = Instant::now();
        }
    }

    let final_count = update_count.load(Ordering::Relaxed);

    // With 25 files and throttling every 10:
    // - Update at file 10
    // - Update at file 20
    // So we expect 2-4 updates (accounting for timing variations)
    assert!(
        final_count >= 2 && final_count <= 10,
        "Expected 2-10 updates with throttling, got {}",
        final_count
    );
    assert!(
        final_count < total_files,
        "Throttling should reduce updates from {} to {}",
        total_files,
        final_count
    );
}

#[test]
fn test_progress_values_are_monotonic() {
    // Test that progress values increase monotonically
    let total_files = 15;
    let mut progress_values = Vec::new();

    // Simulate the progress tracking loop
    for idx in 0..total_files {
        if (idx + 1) % 10 == 0 {
            progress_values.push((idx + 1, total_files));
        }
    }

    // Add final update (which happens at the end in real code)
    progress_values.push((total_files, total_files));

    // Verify progress values
    for (current, total) in &progress_values {
        assert_eq!(*total, total_files, "Total should always be {}", total_files);
        assert!(*current <= total_files, "Current should never exceed total");
        assert!(*current > 0, "Current should be positive");
    }

    // Verify monotonic increase
    for i in 1..progress_values.len() {
        assert!(
            progress_values[i].0 >= progress_values[i - 1].0,
            "Progress should be monotonically increasing"
        );
    }
}

#[test]
fn test_throttling_with_time_based_updates() {
    // Test that time-based throttling works correctly
    let total_files = 50;
    let update_count = Arc::new(AtomicUsize::new(0));
    let update_count_clone = update_count.clone();
    let mut last_update = Instant::now();

    for idx in 0..total_files {
        // Simulate variable processing time
        if idx % 5 == 0 {
            std::thread::sleep(Duration::from_millis(25));
        }

        // Throttled progress updates (every 10 files or 100ms)
        if (idx + 1) % 10 == 0 || last_update.elapsed() > Duration::from_millis(100) {
            update_count_clone.fetch_add(1, Ordering::Relaxed);
            last_update = Instant::now();
        }
    }

    let final_count = update_count.load(Ordering::Relaxed);

    // Should have at least 5 updates (10, 20, 30, 40, 50)
    // May have more due to time-based throttling
    assert!(
        final_count >= 5,
        "Expected at least 5 updates, got {}",
        final_count
    );
    assert!(
        final_count < total_files,
        "Throttling should limit updates to less than total files"
    );
}

#[test]
fn test_progress_callback_invocation_pattern() {
    // Test the exact pattern used in detect_duplications_with_progress
    let total_files = 25;
    let mut progress_calls = Vec::new();
    let mut last_update = Instant::now();

    for idx in 0..total_files {
        // Simulate the throttling check from detect_duplications_with_progress
        if (idx + 1) % 10 == 0 || last_update.elapsed() > Duration::from_millis(100) {
            progress_calls.push((idx + 1, total_files));
            last_update = Instant::now();
        }
    }

    // Final update (always happens)
    progress_calls.push((total_files, total_files));

    // Verify we have progress updates
    assert!(!progress_calls.is_empty(), "Should have progress updates");

    // Verify final update
    let final_call = progress_calls.last().unwrap();
    assert_eq!(final_call.0, total_files, "Final current should be total");
    assert_eq!(final_call.1, total_files, "Final total should be total");

    // Verify all updates have correct total
    for (current, total) in &progress_calls {
        assert_eq!(*total, total_files);
        assert!(*current > 0 && *current <= total_files);
    }
}

#[test]
fn test_throttling_prevents_excessive_updates() {
    // Verify throttling effectively limits update frequency
    let total_files = 100;
    let update_count = AtomicUsize::new(0);
    let mut last_update = Instant::now();

    for idx in 0..total_files {
        // Throttled updates
        if (idx + 1) % 10 == 0 || last_update.elapsed() > Duration::from_millis(100) {
            update_count.fetch_add(1, Ordering::Relaxed);
            last_update = Instant::now();
        }
    }

    let final_count = update_count.load(Ordering::Relaxed);

    // With 100 files and throttling every 10, we expect exactly 10 updates
    // (at files 10, 20, 30, ..., 100)
    assert!(
        final_count >= 10 && final_count <= 20,
        "Expected 10-20 updates for 100 files, got {}",
        final_count
    );

    // Verify we're throttling by at least 80%
    let throttle_percentage = (1.0 - (final_count as f64 / total_files as f64)) * 100.0;
    assert!(
        throttle_percentage >= 80.0,
        "Should throttle by at least 80%, got {}%",
        throttle_percentage
    );
}

#[test]
fn test_time_based_throttling_100ms() {
    // Test the 100ms time-based throttling
    let mut update_times = Vec::new();
    let mut last_update = Instant::now();

    // Simulate processing that takes time
    for _ in 0..5 {
        std::thread::sleep(Duration::from_millis(30));

        if last_update.elapsed() > Duration::from_millis(100) {
            update_times.push(last_update.elapsed());
            last_update = Instant::now();
        }
    }

    // Should have at least 1 time-based update
    assert!(
        !update_times.is_empty(),
        "Should have time-based updates after 150ms"
    );

    // All update intervals should be >= 100ms
    for elapsed in &update_times {
        assert!(
            *elapsed >= Duration::from_millis(100),
            "Update interval should be >= 100ms, got {:?}",
            elapsed
        );
    }
}
