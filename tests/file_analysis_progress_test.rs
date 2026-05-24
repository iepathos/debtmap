// Tests for file analysis progress tracking logic
// These tests verify the throttling and progress tracking patterns
// used in analyze_files_for_debt without requiring full UnifiedAnalysis setup

use std::time::Duration;

fn should_update_progress(current_file: usize, elapsed_since_update: Duration) -> bool {
    current_file % 10 == 0 || elapsed_since_update > Duration::from_millis(100)
}

fn simulate_progress_updates(
    total_files: usize,
    elapsed_per_file: impl Fn(usize) -> Duration,
    include_final_update: bool,
) -> Vec<(usize, usize)> {
    let mut progress_calls = Vec::new();
    let mut elapsed_since_update = Duration::ZERO;

    for idx in 0..total_files {
        let current_file = idx + 1;
        elapsed_since_update += elapsed_per_file(idx);

        if should_update_progress(current_file, elapsed_since_update) {
            progress_calls.push((current_file, total_files));
            elapsed_since_update = Duration::ZERO;
        }
    }

    if include_final_update {
        progress_calls.push((total_files, total_files));
    }

    progress_calls
}

#[test]
fn test_progress_throttling_every_10_files() {
    let total_files = 25;
    let progress_calls =
        simulate_progress_updates(total_files, |_| Duration::from_micros(10), false);
    let final_count = progress_calls.len();

    // With 25 files and throttling every 10:
    // - Update at file 10
    // - Update at file 20
    // So we expect 2 updates without time-based pressure.
    assert!(
        final_count == 2,
        "Expected 2 updates with throttling, got {}",
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
    let total_files = 15;
    let progress_values = simulate_progress_updates(total_files, |_| Duration::ZERO, true);

    // Verify progress values
    for (current, total) in &progress_values {
        assert_eq!(
            *total, total_files,
            "Total should always be {}",
            total_files
        );
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
    let total_files = 50;
    let progress_calls = simulate_progress_updates(
        total_files,
        |idx| {
            if idx % 5 == 0 {
                Duration::from_millis(25)
            } else {
                Duration::ZERO
            }
        },
        false,
    );
    let final_count = progress_calls.len();

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
    let total_files = 25;
    let progress_calls = simulate_progress_updates(total_files, |_| Duration::ZERO, true);

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
    let total_files = 100;
    let progress_calls = simulate_progress_updates(total_files, |_| Duration::ZERO, false);
    let final_count = progress_calls.len();

    // With 100 files and throttling every 10, we expect exactly 10 updates
    // (at files 10, 20, 30, ..., 100)
    assert!(
        final_count == 10,
        "Expected 10 updates for 100 files, got {}",
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
    let total_files = 5;
    let mut elapsed_since_update = Duration::ZERO;
    let mut update_times = Vec::new();

    for idx in 0..total_files {
        elapsed_since_update += Duration::from_millis(30);

        if should_update_progress(idx + 1, elapsed_since_update) {
            update_times.push(elapsed_since_update);
            elapsed_since_update = Duration::ZERO;
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
