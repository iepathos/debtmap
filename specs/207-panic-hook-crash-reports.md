---
number: 207
title: Panic Hook with Structured Crash Reports
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-14
---

# Specification 207: Panic Hook with Structured Crash Reports

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

When debtmap crashes during analysis, the error message is nearly useless for debugging:

```
Error: Thread panic: Any { .. }
thread '<unnamed>' panicked at %
```

This violates the Stillwater principle: **"Errors Should Tell Stories"**. Users and developers have no information about:
- What file was being analyzed when the crash occurred
- Which phase of analysis failed (parsing, call graph, debt scoring, etc.)
- The actual panic message and source location
- A stack trace showing the call chain

The crash occurred analyzing a large codebase (zed), and without context, reproducing and debugging is extremely difficult.

## Objective

Install a custom panic hook that produces structured, actionable crash reports containing:

1. **Panic details**: Actual error message and source location
2. **Operation context**: Current analysis phase and file being processed
3. **Progress state**: How far through the analysis we got
4. **Stack trace**: Full backtrace for developer debugging
5. **Reproduction hints**: Information to help reproduce the issue

## Requirements

### Functional Requirements

1. **Structured Crash Report Format**
   - Header with debtmap version, platform, and timestamp
   - Panic message extracted from payload
   - Source location (file:line:column)
   - Operation context section showing current phase
   - Progress information (files processed, current file)
   - Full stack backtrace
   - Footer with issue reporting guidance

2. **Context Capture**
   - Track current analysis phase via thread-local or atomic state
   - Track current file being analyzed
   - Track overall progress (N of M files)
   - Capture tracing span context when available (spec 208)

3. **Graceful Degradation**
   - Still produce useful output if context unavailable
   - Handle nested panics gracefully
   - Work in both TUI and non-TUI modes

### Non-Functional Requirements

1. **Zero runtime overhead when not panicking**
2. **Thread-safe context tracking for parallel analysis**
3. **Works with rayon parallel iterators**
4. **Respects RUST_BACKTRACE environment variable**

## Acceptance Criteria

- [ ] Custom panic hook installed at application startup
- [ ] Crash reports include panic message and location
- [ ] Crash reports show current analysis phase
- [ ] Crash reports show file being analyzed (if available)
- [ ] Crash reports show progress (N/M files)
- [ ] Stack backtrace included when RUST_BACKTRACE=1
- [ ] Report format is visually distinct and scannable
- [ ] Thread-safe context tracking works with rayon
- [ ] Graceful handling when context unavailable
- [ ] Issue reporting URL included in output
- [ ] No performance regression in normal operation

## Technical Details

### Implementation Approach

**Phase 1: Create Observability Module**

```rust
// src/observability/mod.rs
pub mod context;
pub mod panic_hook;

pub use context::AnalysisContext;
pub use panic_hook::install_panic_hook;
```

**Phase 2: Thread-Local Context Tracking**

```rust
// src/observability/context.rs
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global progress counters (atomic for thread-safety)
static FILES_PROCESSED: AtomicUsize = AtomicUsize::new(0);
static FILES_TOTAL: AtomicUsize = AtomicUsize::new(0);

/// Thread-local context for the current operation
thread_local! {
    static CURRENT_CONTEXT: RefCell<AnalysisContext> = RefCell::new(AnalysisContext::default());
}

#[derive(Debug, Clone, Default)]
pub struct AnalysisContext {
    pub phase: Option<AnalysisPhase>,
    pub current_file: Option<PathBuf>,
    pub current_function: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisPhase {
    FileDiscovery,
    Parsing,
    CallGraphBuilding,
    PurityAnalysis,
    CoverageLoading,
    DebtScoring,
    Prioritization,
    OutputGeneration,
}

impl std::fmt::Display for AnalysisPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileDiscovery => write!(f, "file_discovery"),
            Self::Parsing => write!(f, "parsing"),
            Self::CallGraphBuilding => write!(f, "call_graph_building"),
            Self::PurityAnalysis => write!(f, "purity_analysis"),
            Self::CoverageLoading => write!(f, "coverage_loading"),
            Self::DebtScoring => write!(f, "debt_scoring"),
            Self::Prioritization => write!(f, "prioritization"),
            Self::OutputGeneration => write!(f, "output_generation"),
        }
    }
}

/// RAII guard for setting analysis context
pub struct ContextGuard {
    previous: AnalysisContext,
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        CURRENT_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = self.previous.clone();
        });
    }
}

/// Set the current analysis phase (returns guard that restores previous on drop)
pub fn set_phase(phase: AnalysisPhase) -> ContextGuard {
    CURRENT_CONTEXT.with(|ctx| {
        let previous = ctx.borrow().clone();
        ctx.borrow_mut().phase = Some(phase);
        ContextGuard { previous }
    })
}

/// Set the current file being analyzed
pub fn set_current_file(path: impl Into<PathBuf>) -> ContextGuard {
    CURRENT_CONTEXT.with(|ctx| {
        let previous = ctx.borrow().clone();
        ctx.borrow_mut().current_file = Some(path.into());
        ContextGuard { previous }
    })
}

/// Set progress counters
pub fn set_progress(processed: usize, total: usize) {
    FILES_PROCESSED.store(processed, Ordering::Relaxed);
    FILES_TOTAL.store(total, Ordering::Relaxed);
}

/// Increment processed count
pub fn increment_processed() {
    FILES_PROCESSED.fetch_add(1, Ordering::Relaxed);
}

/// Get current context snapshot (for panic handler)
pub fn get_current_context() -> AnalysisContext {
    CURRENT_CONTEXT.with(|ctx| ctx.borrow().clone())
}

/// Get progress snapshot
pub fn get_progress() -> (usize, usize) {
    (
        FILES_PROCESSED.load(Ordering::Relaxed),
        FILES_TOTAL.load(Ordering::Relaxed),
    )
}
```

**Phase 3: Panic Hook Implementation**

```rust
// src/observability/panic_hook.rs
use super::context::{get_current_context, get_progress, AnalysisContext};
use std::panic::PanicInfo;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Install the custom panic hook
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        print_crash_report(info);
    }));
}

fn print_crash_report(info: &PanicInfo<'_>) {
    let context = get_current_context();
    let (processed, total) = get_progress();

    eprintln!();
    eprintln!("╔══════════════════════════════════════════════════════════════════════════════╗");
    eprintln!("║                           DEBTMAP CRASH REPORT                               ║");
    eprintln!("╠══════════════════════════════════════════════════════════════════════════════╣");
    eprintln!("║  Version: {:<67} ║", VERSION);
    eprintln!("║  Platform: {:<66} ║", std::env::consts::OS);
    eprintln!("║  Time: {:<70} ║", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
    eprintln!("╠══════════════════════════════════════════════════════════════════════════════╣");

    // Panic message
    let message = extract_panic_message(info);
    eprintln!("║  PANIC: {:<68} ║", truncate(&message, 68));

    // Location
    if let Some(location) = info.location() {
        eprintln!("║  Location: {}:{}:{:<52} ║",
            location.file(), location.line(), location.column());
    }

    eprintln!("╠══════════════════════════════════════════════════════════════════════════════╣");

    // Operation context
    print_context_section(&context, processed, total);

    eprintln!("╠══════════════════════════════════════════════════════════════════════════════╣");

    // Stack trace
    if std::env::var("RUST_BACKTRACE").is_ok() {
        eprintln!("║  STACK TRACE:                                                                ║");
        eprintln!("╚══════════════════════════════════════════════════════════════════════════════╝");
        eprintln!();
        eprintln!("{}", std::backtrace::Backtrace::capture());
    } else {
        eprintln!("║  Run with RUST_BACKTRACE=1 for stack trace                                   ║");
        eprintln!("╚══════════════════════════════════════════════════════════════════════════════╝");
    }

    eprintln!();
    eprintln!("════════════════════════════════════════════════════════════════════════════════");
    eprintln!("To report this issue: https://github.com/user/debtmap/issues/new");
    if let Some(file) = &context.current_file {
        eprintln!("Include this crash report and the file: {}", file.display());
    }
    eprintln!("════════════════════════════════════════════════════════════════════════════════");
}

fn print_context_section(context: &AnalysisContext, processed: usize, total: usize) {
    eprintln!("║  OPERATION CONTEXT:                                                          ║");

    if let Some(phase) = &context.phase {
        eprintln!("║    Phase: {:<66} ║", phase);
    } else {
        eprintln!("║    Phase: unknown                                                            ║");
    }

    if let Some(file) = &context.current_file {
        let file_str = file.display().to_string();
        eprintln!("║    File: {:<67} ║", truncate(&file_str, 67));
    }

    if let Some(func) = &context.current_function {
        eprintln!("║    Function: {:<63} ║", truncate(func, 63));
    }

    if total > 0 {
        let pct = (processed as f64 / total as f64 * 100.0) as usize;
        eprintln!("║    Progress: {} / {} files ({}%)                                   ║",
            processed, total, pct);
    }
}

fn extract_panic_message(info: &PanicInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
```

**Phase 4: Integration Points**

```rust
// src/main.rs
use debtmap::observability::install_panic_hook;

fn main() -> Result<()> {
    // Install panic hook FIRST
    install_panic_hook();

    // ... rest of main
}

// src/builders/unified_analysis.rs
use crate::observability::context::{set_phase, set_current_file, set_progress, AnalysisPhase};

pub fn perform_unified_analysis_with_options(...) -> Result<UnifiedAnalysis> {
    set_progress(0, results.file_metrics.len());

    let _phase = set_phase(AnalysisPhase::CallGraphBuilding);
    build_call_graph_with_progress(...)?;

    let _phase = set_phase(AnalysisPhase::DebtScoring);
    results.file_metrics
        .par_iter()
        .enumerate()
        .map(|(idx, (path, metrics))| {
            let _file = set_current_file(path);
            set_progress(idx, results.file_metrics.len());
            score_file(path, metrics)
        })
        .collect()
}
```

### Architecture Changes

New module: `src/observability/`
- `mod.rs` - Module exports
- `context.rs` - Thread-local context tracking
- `panic_hook.rs` - Panic hook and crash report formatting

Modified files:
- `src/main.rs` - Install panic hook at startup
- `src/lib.rs` - Export observability module
- `src/builders/unified_analysis.rs` - Set context during analysis phases
- `src/analyzers/*.rs` - Set context during file analysis

### Data Structures

```rust
/// Analysis phase for context tracking
pub enum AnalysisPhase {
    FileDiscovery,
    Parsing,
    CallGraphBuilding,
    PurityAnalysis,
    CoverageLoading,
    DebtScoring,
    Prioritization,
    OutputGeneration,
}

/// Thread-local context snapshot
pub struct AnalysisContext {
    pub phase: Option<AnalysisPhase>,
    pub current_file: Option<PathBuf>,
    pub current_function: Option<String>,
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/main.rs` - Panic hook installation
  - `src/builders/unified_analysis.rs` - Phase tracking
  - Analysis pipeline modules - File tracking
- **External Dependencies**:
  - `chrono` for timestamp formatting (already in deps)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_guard_restores_previous() {
        let _phase1 = set_phase(AnalysisPhase::Parsing);
        assert_eq!(get_current_context().phase, Some(AnalysisPhase::Parsing));

        {
            let _phase2 = set_phase(AnalysisPhase::DebtScoring);
            assert_eq!(get_current_context().phase, Some(AnalysisPhase::DebtScoring));
        }

        // Should restore to Parsing after guard drops
        assert_eq!(get_current_context().phase, Some(AnalysisPhase::Parsing));
    }

    #[test]
    fn test_progress_tracking() {
        set_progress(50, 100);
        let (processed, total) = get_progress();
        assert_eq!(processed, 50);
        assert_eq!(total, 100);
    }

    #[test]
    fn test_extract_panic_message_str() {
        // Test with &str payload
        let msg = "test panic message";
        // ... test extraction
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a long string", 10), "this is...");
    }
}
```

### Integration Tests

```rust
#[test]
fn test_panic_produces_crash_report() {
    // Capture stderr during panic
    let output = std::panic::catch_unwind(|| {
        let _phase = set_phase(AnalysisPhase::DebtScoring);
        let _file = set_current_file("/path/to/test.rs");
        panic!("test panic");
    });

    // Verify crash report was produced
    // (would need stderr capture mechanism)
}
```

## Documentation Requirements

### Code Documentation

- Document all public types in observability module
- Include examples for context tracking usage
- Document thread-safety guarantees

### User Documentation

Update troubleshooting guide:
```markdown
## Crash Reports

When debtmap crashes, it produces a structured crash report:

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                           DEBTMAP CRASH REPORT                               ║
╠══════════════════════════════════════════════════════════════════════════════╣
║  PANIC: index out of bounds...                                               ║
║  Location: src/priority/scoring.rs:287:13                                    ║
╠══════════════════════════════════════════════════════════════════════════════╣
║  OPERATION CONTEXT:                                                          ║
║    Phase: debt_scoring                                                       ║
║    File: /path/to/problematic_file.rs                                        ║
║    Progress: 2847 / 4231 files (67%)                                         ║
╚══════════════════════════════════════════════════════════════════════════════╝
```

For stack traces, run with:
```bash
RUST_BACKTRACE=1 debtmap analyze .
```
```

## Implementation Notes

### Thread Safety

- Use `thread_local!` for per-thread context (works with rayon)
- Use atomics for global progress counters
- Context guards use RAII for automatic cleanup

### Performance

- Thread-local access is very fast (no locks)
- Atomic operations only for progress counters
- Zero overhead when not panicking

### Graceful Degradation

- If context unavailable, show "unknown"
- If progress not set, omit progress line
- Always show panic message and location

## Migration and Compatibility

No breaking changes. This adds new functionality without modifying existing behavior.
