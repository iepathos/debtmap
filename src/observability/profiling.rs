//! Profiling infrastructure for timing analysis phases and operations.
//!
//! This module provides timing instrumentation that integrates with debtmap's
//! observability infrastructure. Following Stillwater's "Pragmatism Over Purity"
//! principle, it uses simple RAII guards and thread-safe collection without
//! heavy external dependencies.
//!
//! ## Usage
//!
//! Enable profiling at application startup:
//!
//! ```ignore
//! use debtmap::observability::profiling::{enable_profiling, get_timing_report};
//!
//! enable_profiling();
//! // ... run analysis ...
//! let report = get_timing_report();
//! println!("{}", report.to_summary());
//! ```
//!
//! Add timing spans to functions:
//!
//! ```ignore
//! use debtmap::time_span;
//!
//! fn analyze_file(path: &Path) -> Result<FileAnalysis> {
//!     time_span!("analyze_file");
//!     // ... analysis code ...
//! }
//! ```
//!
//! Nested timing is also supported:
//!
//! ```ignore
//! fn score_functions(functions: &[Function]) -> Vec<Score> {
//!     time_span!("score_functions");
//!     functions.iter().map(|f| {
//!         time_span!("score_single_function", parent: "score_functions");
//!         compute_score(f)
//!     }).collect()
//! }
//! ```

use dashmap::DashMap;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Global flag indicating whether profiling is enabled.
static PROFILING_ENABLED: AtomicBool = AtomicBool::new(false);

/// Global timing collector singleton.
static TIMING_COLLECTOR: OnceLock<TimingCollector> = OnceLock::new();

/// Enable profiling for this process.
///
/// Call this before analysis begins. Timing data will be collected
/// for all subsequent operations until the process exits.
pub fn enable_profiling() {
    PROFILING_ENABLED.store(true, Ordering::SeqCst);
    // Initialize the collector if not already done
    let _ = get_collector();
}

/// Disable profiling.
///
/// Primarily useful for testing. In production, profiling should remain
/// enabled for the duration of the analysis.
#[cfg(test)]
pub fn disable_profiling() {
    PROFILING_ENABLED.store(false, Ordering::SeqCst);
}

/// Check if profiling is currently enabled.
#[inline]
pub fn is_profiling_enabled() -> bool {
    PROFILING_ENABLED.load(Ordering::Relaxed)
}

/// Get the global timing collector.
fn get_collector() -> &'static TimingCollector {
    TIMING_COLLECTOR.get_or_init(TimingCollector::new)
}

/// Record a timing measurement.
///
/// Called automatically by `TimingSpan::drop()`.
pub fn record_timing(name: &'static str, parent: Option<&'static str>, duration: Duration) {
    if is_profiling_enabled() {
        get_collector().record(name, parent, duration);
    }
}

/// Get the current timing report.
///
/// Returns a report with all recorded timing data, organized hierarchically.
#[must_use]
pub fn get_timing_report() -> TimingReport {
    get_collector().generate_report()
}

/// Reset all timing data.
///
/// Useful for testing or between analysis runs.
pub fn reset_timing_data() {
    get_collector().reset();
}

/// RAII guard that records timing when dropped.
///
/// Create via `TimingSpan::new()` or the `time_span!` macro.
/// The timing is recorded when the span goes out of scope.
pub struct TimingSpan {
    name: &'static str,
    start: Instant,
    parent: Option<&'static str>,
}

impl TimingSpan {
    /// Create a new timing span.
    ///
    /// The span starts timing immediately and records the duration when dropped.
    #[inline]
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
            parent: None,
        }
    }

    /// Create a timing span with a parent relationship.
    ///
    /// This allows building hierarchical timing reports where child
    /// operations are nested under their parent.
    #[inline]
    #[must_use]
    pub fn with_parent(name: &'static str, parent: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
            parent: Some(parent),
        }
    }
}

impl Drop for TimingSpan {
    fn drop(&mut self) {
        if is_profiling_enabled() {
            let duration = self.start.elapsed();
            record_timing(self.name, self.parent, duration);
        }
    }
}

/// Thread-safe timing data collector.
///
/// Uses `DashMap` for concurrent access from parallel analysis operations.
pub struct TimingCollector {
    /// Timing data: name -> (total_duration_ns, count)
    timings: DashMap<&'static str, TimingEntry>,
    /// Parent-child relationships for hierarchical reports
    hierarchy: DashMap<&'static str, &'static str>,
    /// Overall analysis start time
    start_time: OnceLock<Instant>,
}

/// Entry for a single timing measurement.
struct TimingEntry {
    total_nanos: AtomicU64,
    count: AtomicU64,
}

impl TimingEntry {
    fn new() -> Self {
        Self {
            total_nanos: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }
}

impl TimingCollector {
    /// Create a new timing collector.
    fn new() -> Self {
        Self {
            timings: DashMap::new(),
            hierarchy: DashMap::new(),
            start_time: OnceLock::new(),
        }
    }

    /// Record a timing measurement.
    pub fn record(&self, name: &'static str, parent: Option<&'static str>, duration: Duration) {
        // Set start time on first recording
        let _ = self.start_time.get_or_init(Instant::now);

        let nanos = duration.as_nanos() as u64;

        // Get or create entry
        let entry = self.timings.entry(name).or_insert_with(TimingEntry::new);
        entry.total_nanos.fetch_add(nanos, Ordering::Relaxed);
        entry.count.fetch_add(1, Ordering::Relaxed);

        // Record hierarchy if parent specified
        if let Some(p) = parent {
            self.hierarchy.insert(name, p);
        }
    }

    /// Reset all timing data.
    pub fn reset(&self) {
        self.timings.clear();
        self.hierarchy.clear();
    }

    /// Generate a timing report from collected data.
    #[must_use]
    pub fn generate_report(&self) -> TimingReport {
        let total_duration = self
            .start_time
            .get()
            .map(|start| start.elapsed())
            .unwrap_or(Duration::ZERO);

        // Build phase timings from collected data
        let mut phases: Vec<PhaseTiming> = Vec::new();

        // Collect all entries
        for entry in self.timings.iter() {
            let name = *entry.key();
            let timing = entry.value();

            let duration_nanos = timing.total_nanos.load(Ordering::Relaxed);
            let count = timing.count.load(Ordering::Relaxed);
            let duration = Duration::from_nanos(duration_nanos);

            // Check if this has a parent
            let parent = self.hierarchy.get(name).map(|p| *p);

            let percentage = if total_duration.as_nanos() > 0 {
                (duration.as_nanos() as f64 / total_duration.as_nanos() as f64) * 100.0
            } else {
                0.0
            };

            phases.push(PhaseTiming {
                name: name.to_string(),
                duration,
                percentage,
                count,
                parent: parent.map(|s| s.to_string()),
                children: Vec::new(),
            });
        }

        // Sort by duration descending for easy identification of bottlenecks
        phases.sort_by(|a, b| b.duration.cmp(&a.duration));

        // Build hierarchy
        let phases = build_hierarchy(phases);

        TimingReport {
            total_duration,
            phases,
        }
    }
}

/// Build a hierarchical structure from flat phase timings.
fn build_hierarchy(mut phases: Vec<PhaseTiming>) -> Vec<PhaseTiming> {
    // Separate root phases (no parent) from child phases
    let (mut roots, children): (Vec<_>, Vec<_>) =
        phases.drain(..).partition(|p| p.parent.is_none());

    // For each child, find its parent and add it
    for child in children {
        if let Some(parent_name) = &child.parent {
            // Find the parent in roots and add this child
            for root in &mut roots {
                if add_child_to_parent(root, parent_name, child.clone()) {
                    break;
                }
            }
        }
    }

    // Sort children within each parent by duration descending
    for root in &mut roots {
        sort_children_recursive(root);
    }

    roots
}

/// Recursively add a child to the correct parent.
fn add_child_to_parent(parent: &mut PhaseTiming, target_name: &str, child: PhaseTiming) -> bool {
    if parent.name == target_name {
        parent.children.push(child);
        return true;
    }

    for existing_child in &mut parent.children {
        if add_child_to_parent(existing_child, target_name, child.clone()) {
            return true;
        }
    }

    false
}

/// Recursively sort children by duration.
fn sort_children_recursive(phase: &mut PhaseTiming) {
    phase.children.sort_by(|a, b| b.duration.cmp(&a.duration));
    for child in &mut phase.children {
        sort_children_recursive(child);
    }
}

/// Complete timing report with hierarchical breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct TimingReport {
    /// Total analysis duration
    pub total_duration: Duration,
    /// Timing breakdown by phase
    pub phases: Vec<PhaseTiming>,
}

/// Timing data for a single phase or operation.
#[derive(Debug, Clone, Serialize)]
pub struct PhaseTiming {
    /// Name of the phase or operation
    pub name: String,
    /// Total duration spent in this phase
    pub duration: Duration,
    /// Percentage of total analysis time
    pub percentage: f64,
    /// Number of times this operation was executed
    pub count: u64,
    /// Parent phase name (if nested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Child operations within this phase
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<PhaseTiming>,
}

impl TimingReport {
    /// Generate a human-readable summary of the timing report.
    #[must_use]
    pub fn to_summary(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "\n=== Profiling Report ===\nTotal analysis time: {:.2?}\n\n",
            self.total_duration
        ));

        output.push_str("Phase breakdown:\n");
        output.push_str(&format!(
            "{:<40} {:>12} {:>8} {:>10}\n",
            "Operation", "Duration", "%", "Count"
        ));
        output.push_str(&"-".repeat(72));
        output.push('\n');

        for phase in &self.phases {
            output.push_str(&format_phase(phase, 0));
        }

        output
    }

    /// Serialize the report to JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Format a single phase with indentation for hierarchy.
fn format_phase(phase: &PhaseTiming, indent: usize) -> String {
    let mut output = String::new();

    let indent_str = "  ".repeat(indent);
    let name = format!("{}{}", indent_str, phase.name);

    output.push_str(&format!(
        "{:<40} {:>12} {:>7.1}% {:>10}\n",
        name,
        format_duration(phase.duration),
        phase.percentage,
        phase.count
    ));

    for child in &phase.children {
        output.push_str(&format_phase(child, indent + 1));
    }

    output
}

/// Format a duration for display.
fn format_duration(d: Duration) -> String {
    if d.as_secs() >= 1 {
        format!("{:.2}s", d.as_secs_f64())
    } else if d.as_millis() >= 1 {
        format!("{:.1}ms", d.as_secs_f64() * 1000.0)
    } else {
        format!("{:.0}µs", d.as_secs_f64() * 1_000_000.0)
    }
}

/// Macro for convenient timing span creation.
///
/// Creates a timing span that records its duration when it goes out of scope.
/// The span is only created if profiling is enabled, minimizing overhead.
///
/// # Examples
///
/// Basic usage:
/// ```ignore
/// use debtmap::time_span;
///
/// fn analyze() {
///     time_span!("analyze");
///     // ... analysis code ...
/// }
/// ```
///
/// With parent for hierarchical timing:
/// ```ignore
/// fn score_all() {
///     time_span!("score_all");
///     for item in items {
///         time_span!("score_item", parent: "score_all");
///         // ... scoring code ...
///     }
/// }
/// ```
#[macro_export]
macro_rules! time_span {
    ($name:expr) => {
        let _timing_span = if $crate::observability::profiling::is_profiling_enabled() {
            Some($crate::observability::profiling::TimingSpan::new($name))
        } else {
            None
        };
    };
    ($name:expr, parent: $parent:expr) => {
        let _timing_span = if $crate::observability::profiling::is_profiling_enabled() {
            Some($crate::observability::profiling::TimingSpan::with_parent(
                $name, $parent,
            ))
        } else {
            None
        };
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    /// Get current count for an operation, or 0 if not present
    fn get_count(name: &str) -> u64 {
        get_timing_report()
            .phases
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.count)
            .unwrap_or(0)
    }

    #[test]
    fn test_timing_span_records_duration() {
        enable_profiling();
        let before = get_count("test_duration_op");

        {
            let _span = TimingSpan::new("test_duration_op");
            thread::sleep(Duration::from_millis(10));
        }

        let report = get_timing_report();
        let phase = report
            .phases
            .iter()
            .find(|p| p.name == "test_duration_op")
            .expect("Should have recorded test_duration_op");

        assert_eq!(phase.count, before + 1, "Count should increase by 1");
        assert!(
            phase.duration >= Duration::from_millis(10),
            "Duration should be at least 10ms"
        );
    }

    #[test]
    fn test_timing_with_parent() {
        enable_profiling();

        {
            let _parent = TimingSpan::new("test_parent_op");
            {
                let _child = TimingSpan::with_parent("test_child_op", "test_parent_op");
                thread::sleep(Duration::from_millis(5));
            }
        }

        let report = get_timing_report();

        // Find parent - it should have the child nested
        let parent = report
            .phases
            .iter()
            .find(|p| p.name == "test_parent_op")
            .expect("Parent should exist");

        let child = parent
            .children
            .iter()
            .find(|c| c.name == "test_child_op");
        assert!(child.is_some(), "Child should be nested under parent");
    }

    #[test]
    fn test_timing_count_accumulates() {
        enable_profiling();
        let before = get_count("test_accumulate_op");

        for _ in 0..5 {
            let _span = TimingSpan::new("test_accumulate_op");
        }

        let after = get_count("test_accumulate_op");
        assert_eq!(after, before + 5, "Count should increase by 5");
    }

    #[test]
    fn test_timing_conditional_creation() {
        // Test the conditional pattern works when profiling is enabled.
        // We measure delta to be robust against parallel test execution.
        enable_profiling();
        let before = get_count("test_conditional_op");

        for _ in 0..10 {
            if is_profiling_enabled() {
                let _span = TimingSpan::new("test_conditional_op");
            }
        }

        let after = get_count("test_conditional_op");
        assert_eq!(after, before + 10, "Should record 10 spans when enabled");
    }

    #[test]
    fn test_report_to_summary() {
        enable_profiling();

        {
            let _span = TimingSpan::new("test_summary_op");
            thread::sleep(Duration::from_millis(1));
        }

        let report = get_timing_report();
        let summary = report.to_summary();

        assert!(summary.contains("Profiling Report"));
        assert!(summary.contains("test_summary_op"));
    }

    #[test]
    fn test_report_to_json() {
        enable_profiling();

        {
            let _span = TimingSpan::new("test_json_op");
        }

        let report = get_timing_report();
        let json = report.to_json();

        assert!(json.contains("\"name\": \"test_json_op\""));
        assert!(json.contains("\"total_duration\""));
    }

    #[test]
    fn test_time_span_macro() {
        enable_profiling();
        let before = get_count("test_macro_op");

        fn test_function() {
            time_span!("test_macro_op");
            thread::sleep(Duration::from_millis(1));
        }

        test_function();

        let after = get_count("test_macro_op");
        assert_eq!(after, before + 1, "Macro should record timing");
    }

    #[test]
    fn test_thread_safe_collection() {
        enable_profiling();
        let before = get_count("test_parallel_op");

        let handles: Vec<_> = (0..4)
            .map(|_| {
                thread::spawn(|| {
                    for _ in 0..10 {
                        let _span = TimingSpan::new("test_parallel_op");
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let after = get_count("test_parallel_op");
        assert_eq!(
            after,
            before + 40,
            "Should record 40 spans (4 threads × 10 each)"
        );
    }
}
