---
number: 1
title: Comprehensive Profiling Infrastructure
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-21
---

# Specification 001: Comprehensive Profiling Infrastructure

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap analysis of large codebases (10,000+ functions) currently takes several minutes, with no systematic way to identify performance bottlenecks. Recent optimization attempts (e.g., switching git subprocess calls to git2 library) were made without profiling data, leading to wasted effort and regressions.

The project already has:
- `tracing` and `tracing-subscriber` dependencies
- `criterion` for micro-benchmarks (26 benchmark files in `benches/`)
- Observability infrastructure in `src/observability/` with phase tracking
- TUI-aware logging that suppresses output during interactive mode

What's missing:
- **Timing instrumentation** for analysis phases and sub-operations
- **Profiling output** that can be analyzed after a run
- **CLI flag** to enable profiling without code changes
- **Integration** with existing observability infrastructure

Following the Stillwater philosophy of "Pragmatism Over Purity," this spec focuses on practical, incremental profiling that integrates with existing infrastructure rather than introducing heavy new dependencies.

## Objective

Add comprehensive profiling infrastructure that enables systematic identification of performance bottlenecks in debtmap analysis, with minimal overhead when disabled and seamless integration with existing observability.

## Requirements

### Functional Requirements

1. **Phase-Level Timing**
   - Automatically time each `AnalysisPhase` (FileDiscovery, Parsing, CallGraphBuilding, etc.)
   - Record start time, end time, and duration for each phase
   - Support nested timing within phases

2. **Operation-Level Timing**
   - Time significant sub-operations within phases (e.g., "score_functions", "gather_context", "git_history")
   - Support hierarchical timing spans (parent/child relationships)
   - Track operation counts (e.g., "processed 12,640 functions in 45.2s")

3. **Profiling Output**
   - Generate human-readable timing summary after analysis
   - Output structured data (JSON) for programmatic analysis
   - Support file output for post-run analysis

4. **CLI Integration**
   - Add `--profile` flag to `debtmap analyze` command
   - Add `--profile-output <file>` for writing profile data to file
   - Profile output should work with TUI mode (write to file, not stderr)

5. **Sampling Profiler Support**
   - Document how to use external profilers (samply, perf, Instruments)
   - Ensure release builds have debug symbols for meaningful profiles

### Non-Functional Requirements

1. **Zero Overhead When Disabled**
   - Profiling instrumentation should have negligible cost when `--profile` is not specified
   - Use compile-time feature flags where possible

2. **Minimal Code Changes**
   - Leverage existing `tracing` infrastructure
   - Use `#[tracing::instrument]` attributes where possible
   - Avoid invasive changes to hot paths

3. **Thread-Safe**
   - Timing data must be safe to collect from parallel analysis (rayon)
   - Use atomic operations or thread-local storage as appropriate

## Acceptance Criteria

- [ ] `debtmap analyze . --profile` outputs timing summary for each phase
- [ ] Timing summary shows hierarchical breakdown (phase → sub-operations)
- [ ] `--profile-output timing.json` writes structured timing data to file
- [ ] Profile output works correctly with `--format tui` (no display corruption)
- [ ] Overhead when profiling disabled is < 1% of analysis time
- [ ] Documentation explains how to use samply/perf for CPU profiling
- [ ] At least 3 key bottleneck areas are instrumented (e.g., scoring, git history, coverage lookup)

## Technical Details

### Implementation Approach

#### Layer 1: tracing-based Timing Spans

Extend existing tracing infrastructure with timing:

```rust
// In src/observability/profiling.rs
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

static PROFILING_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable_profiling() {
    PROFILING_ENABLED.store(true, Ordering::Relaxed);
}

pub fn is_profiling_enabled() -> bool {
    PROFILING_ENABLED.load(Ordering::Relaxed)
}

/// RAII guard that records timing when dropped
pub struct TimingSpan {
    name: &'static str,
    start: Instant,
    parent: Option<&'static str>,
}

impl TimingSpan {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
            parent: None,
        }
    }

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

/// Macro for convenient timing span creation
#[macro_export]
macro_rules! time_span {
    ($name:expr) => {
        let _span = if $crate::observability::profiling::is_profiling_enabled() {
            Some($crate::observability::profiling::TimingSpan::new($name))
        } else {
            None
        };
    };
    ($name:expr, parent: $parent:expr) => {
        let _span = if $crate::observability::profiling::is_profiling_enabled() {
            Some($crate::observability::profiling::TimingSpan::with_parent($name, $parent))
        } else {
            None
        };
    };
}
```

#### Layer 2: Thread-Safe Timing Collection

```rust
use dashmap::DashMap;
use std::sync::atomic::AtomicU64;

/// Global timing collector (thread-safe)
pub struct TimingCollector {
    /// Timing data: name -> (total_duration_ns, count)
    timings: DashMap<&'static str, (AtomicU64, AtomicU64)>,
    /// Parent-child relationships
    hierarchy: DashMap<&'static str, &'static str>,
}

impl TimingCollector {
    pub fn record(&self, name: &'static str, parent: Option<&'static str>, duration: Duration) {
        let nanos = duration.as_nanos() as u64;

        self.timings
            .entry(name)
            .or_insert_with(|| (AtomicU64::new(0), AtomicU64::new(0)))
            .0.fetch_add(nanos, Ordering::Relaxed);
        self.timings.get(name).unwrap().1.fetch_add(1, Ordering::Relaxed);

        if let Some(p) = parent {
            self.hierarchy.insert(name, p);
        }
    }

    pub fn generate_report(&self) -> TimingReport {
        // Build hierarchical report from collected data
    }
}
```

#### Layer 3: Output Formats

```rust
#[derive(Debug, Serialize)]
pub struct TimingReport {
    pub total_duration: Duration,
    pub phases: Vec<PhaseTimng>,
}

#[derive(Debug, Serialize)]
pub struct PhaseTiming {
    pub name: String,
    pub duration: Duration,
    pub percentage: f64,
    pub count: u64,
    pub children: Vec<PhaseTiming>,
}

impl TimingReport {
    pub fn to_summary(&self) -> String {
        // Human-readable summary
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
```

### Integration Points

1. **CLI**: Add `--profile` and `--profile-output` flags to `analyze` command
2. **Observability**: Add `profiling` module to `src/observability/`
3. **Phases**: Instrument each `AnalysisPhase` transition
4. **Hot Paths**: Instrument `score_functions`, `gather_context`, `get_function_history`

### Architecture Changes

```
src/observability/
├── mod.rs              # Add: pub mod profiling
├── profiling.rs        # NEW: Timing infrastructure
├── context.rs          # Integrate timing with phase tracking
├── tracing.rs          # Unchanged
├── panic_hook.rs       # Unchanged
└── parallel.rs         # Unchanged
```

### Data Structures

```rust
/// Thread-local timing stack for hierarchical spans
thread_local! {
    static TIMING_STACK: RefCell<Vec<&'static str>> = RefCell::new(Vec::new());
}

/// Global singleton for collecting timing data
static TIMING_COLLECTOR: OnceLock<TimingCollector> = OnceLock::new();
```

### APIs and Interfaces

```rust
// Public API
pub fn enable_profiling();
pub fn is_profiling_enabled() -> bool;
pub fn get_timing_report() -> TimingReport;

// Macro API
time_span!("operation_name");
time_span!("child_operation", parent: "parent_operation");

// Integration with existing phases
impl AnalysisPhase {
    pub fn timed_scope(&self) -> TimingSpan {
        TimingSpan::new(self.as_str())
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/observability/` - New profiling module
  - `src/commands/analyze/` - CLI flag handling
  - `src/builders/unified_analysis_phases/` - Phase timing instrumentation
  - `src/priority/unified_scorer.rs` - Hot path instrumentation
- **External Dependencies**: None new (uses existing `dashmap`, `serde`)

## Testing Strategy

- **Unit Tests**: Test timing collection, hierarchy building, report generation
- **Integration Tests**: End-to-end test with `--profile` flag
- **Performance Tests**: Benchmark overhead when profiling disabled vs enabled
- **User Acceptance**: Manual verification of timing accuracy on real codebase

## Documentation Requirements

- **Code Documentation**: Document public profiling API
- **User Documentation**: Add `--profile` flag to CLI help and README
- **Architecture Updates**: Document profiling infrastructure in ARCHITECTURE.md

## Implementation Notes

1. **Start Simple**: Begin with phase-level timing, add granularity incrementally
2. **Use Existing Infrastructure**: Leverage `tracing::instrument` where natural
3. **Avoid Hot Path Overhead**: Use conditional compilation or runtime checks
4. **Test on Real Workload**: Profile debtmap analyzing itself (12,640 functions)

## Migration and Compatibility

- No breaking changes
- New CLI flags are additive
- Profiling is opt-in via `--profile` flag
