---
number: 209
title: Rayon Parallel Span Propagation
category: optimization
priority: medium
status: draft
dependencies: [207, 208]
created: 2025-12-14
---

# Specification 209: Rayon Parallel Span Propagation

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 207 (Panic Hook), Spec 208 (Structured Tracing)

## Context

Debtmap uses `rayon` for parallel processing of files. When a panic occurs in a rayon worker thread:

1. **Span context is lost** - The parent span doesn't propagate to worker threads
2. **Panic hook shows wrong context** - Thread-local context from spec 207 isn't inherited
3. **Tracing spans break** - Child spans don't link to parent phase spans

This is because rayon spawns worker threads that don't inherit the calling thread's context. The crash report would show the file being processed but lose the parent span hierarchy.

## Objective

Create utilities for propagating both tracing spans and analysis context (spec 207) into rayon parallel iterators:

1. **Span propagation** - Child spans in workers link to parent spans
2. **Context propagation** - AnalysisContext carries into worker threads
3. **Ergonomic API** - Easy to use with existing par_iter patterns
4. **Zero overhead** - No cost when tracing is disabled

## Requirements

### Functional Requirements

1. **Span Propagation into Workers**
   - Parent span captured before par_iter
   - Each worker enters a child span of the parent
   - Span hierarchy preserved in traces

2. **Context Propagation**
   - AnalysisContext (phase, progress) available in workers
   - Current file context set per-item in worker
   - Panic handler can access context from any thread

3. **Ergonomic Wrappers**
   - Extension trait for ParallelIterator
   - Closure wrapper for with_context pattern
   - Minimal boilerplate for common patterns

### Non-Functional Requirements

1. **Thread-safe**: All context types are Send + Sync
2. **Zero-cost when unused**: No overhead if context not captured
3. **Compatible with existing rayon patterns**: Drop-in replacement

## Acceptance Criteria

- [ ] Parent span propagates into rayon workers
- [ ] Worker spans are children of parent span
- [ ] AnalysisContext propagates into workers
- [ ] Per-item file context works in workers
- [ ] Panic in worker shows full span hierarchy
- [ ] Panic in worker shows correct file being processed
- [ ] Extension trait works with par_iter, par_bridge, etc.
- [ ] No performance regression vs raw par_iter
- [ ] Examples documented for common patterns

## Technical Details

### Implementation Approach

**Phase 1: Span-Aware Parallel Wrapper**

```rust
// src/observability/parallel.rs
use rayon::prelude::*;
use tracing::Span;

/// Extension trait for propagating tracing context into rayon parallel iterators
pub trait ParallelTracingExt<T>: ParallelIterator<Item = T> {
    /// Execute parallel iterator with span context propagated to workers
    fn with_span_context<R, F>(self, f: F) -> R
    where
        F: Fn(Self) -> R + Sync,
        R: Send,
    {
        let parent_span = Span::current();
        f(self.map(move |item| {
            let _entered = parent_span.clone().entered();
            item
        }))
    }
}

impl<T, I: ParallelIterator<Item = T>> ParallelTracingExt<T> for I {}
```

**Phase 2: Combined Context Wrapper**

```rust
// src/observability/parallel.rs
use super::context::{AnalysisContext, get_current_context, CURRENT_CONTEXT};

/// Wrapper that carries both tracing span and analysis context into parallel workers
pub struct ParallelContext {
    span: Span,
    analysis_context: AnalysisContext,
}

impl ParallelContext {
    /// Capture current context for propagation
    pub fn capture() -> Self {
        Self {
            span: Span::current(),
            analysis_context: get_current_context(),
        }
    }

    /// Enter this context in the current thread
    pub fn enter(&self) -> ParallelContextGuard {
        let span_guard = self.span.clone().entered();

        // Set thread-local analysis context
        CURRENT_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = self.analysis_context.clone();
        });

        ParallelContextGuard { _span: span_guard }
    }
}

pub struct ParallelContextGuard {
    _span: tracing::span::EnteredSpan,
}

/// Execute closure with propagated context
pub fn with_parallel_context<T, F>(ctx: &ParallelContext, f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = ctx.enter();
    f()
}
```

**Phase 3: Ergonomic ParallelIterator Extension**

```rust
// src/observability/parallel.rs

/// Extension trait for parallel iterators with full context propagation
pub trait ParallelContextExt<T>: ParallelIterator<Item = T> + Sized {
    /// Map with context propagation - each item processed with parent context
    fn map_with_context<R, F>(self, f: F) -> impl ParallelIterator<Item = R>
    where
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        let ctx = ParallelContext::capture();
        self.map(move |item| {
            with_parallel_context(&ctx, || f(item))
        })
    }

    /// Filter-map with context propagation
    fn filter_map_with_context<R, F>(self, f: F) -> impl ParallelIterator<Item = R>
    where
        F: Fn(T) -> Option<R> + Sync + Send,
        R: Send,
    {
        let ctx = ParallelContext::capture();
        self.filter_map(move |item| {
            with_parallel_context(&ctx, || f(item))
        })
    }

    /// For-each with context propagation
    fn for_each_with_context<F>(self, f: F)
    where
        F: Fn(T) + Sync + Send,
    {
        let ctx = ParallelContext::capture();
        self.for_each(move |item| {
            with_parallel_context(&ctx, || f(item))
        })
    }
}

impl<T, I: ParallelIterator<Item = T> + Sized> ParallelContextExt<T> for I {}
```

**Phase 4: File-Level Context Helper**

```rust
// src/observability/parallel.rs
use std::path::Path;
use super::context::{set_current_file, increment_processed};
use tracing::debug_span;

/// Process a file with full context setup
pub fn process_file_with_context<T, F>(
    path: &Path,
    parent_ctx: &ParallelContext,
    f: F,
) -> T
where
    F: FnOnce() -> T,
{
    let _parent = parent_ctx.enter();
    let _file = set_current_file(path);
    let _span = debug_span!("process_file", path = %path.display()).entered();

    increment_processed();

    f()
}

// Usage in debtmap:
pub fn analyze_files_parallel(files: &[PathBuf]) -> Vec<Result<FileMetrics>> {
    let ctx = ParallelContext::capture();

    files
        .par_iter()
        .map(|path| {
            process_file_with_context(path, &ctx, || {
                analyze_single_file(path)
            })
        })
        .collect()
}
```

**Phase 5: Usage in Analysis Pipeline**

```rust
// src/builders/unified_analysis.rs
use crate::observability::parallel::{ParallelContext, process_file_with_context};
use tracing::info_span;

pub fn score_debt_items(
    file_metrics: &HashMap<PathBuf, FileMetrics>,
    call_graph: &CallGraph,
) -> Result<Vec<DebtItem>> {
    let _span = info_span!("debt_scoring", file_count = file_metrics.len()).entered();
    let ctx = ParallelContext::capture();

    file_metrics
        .par_iter()
        .flat_map(|(path, metrics)| {
            process_file_with_context(path, &ctx, || {
                // If panic here, crash report shows:
                // - Phase: debt_scoring
                // - File: /path/to/current/file.rs
                // - Span: debt_scoring > process_file
                score_file_items(path, metrics, call_graph)
            })
        })
        .collect()
}
```

### Architecture Changes

New file: `src/observability/parallel.rs`
- `ParallelContext` - Combined span + analysis context
- `ParallelContextExt` trait - Extension methods for ParallelIterator
- `process_file_with_context` - Helper for file processing pattern

Modified files:
- `src/observability/mod.rs` - Export parallel module
- `src/builders/unified_analysis.rs` - Use context propagation
- `src/analyzers/*.rs` - Use file context helpers

### Data Structures

```rust
/// Combined context for parallel propagation
pub struct ParallelContext {
    /// Tracing span to propagate
    span: Span,
    /// Analysis context to propagate
    analysis_context: AnalysisContext,
}

/// RAII guard for entered parallel context
pub struct ParallelContextGuard {
    _span: tracing::span::EnteredSpan,
    // Note: AnalysisContext cleanup happens via thread-local
}
```

### Thread Safety Considerations

```rust
// ParallelContext must be Send + Sync for rayon
// - Span: is Send + Sync
// - AnalysisContext: is Clone, we clone it per-thread

// The pattern is:
// 1. Capture context on main thread
// 2. Clone/enter in each worker
// 3. Thread-local stores per-worker copy
```

## Dependencies

- **Prerequisites**:
  - Spec 207 (Panic Hook) - for AnalysisContext
  - Spec 208 (Tracing) - for Span propagation
- **Affected Components**:
  - All parallel processing code
  - Analysis builders
  - File analyzers
- **External Dependencies**:
  - `rayon` (already present)
  - `tracing` (from spec 208)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rayon::prelude::*;

    #[test]
    fn test_context_propagates_to_workers() {
        let _phase = set_phase(AnalysisPhase::DebtScoring);
        let ctx = ParallelContext::capture();

        let results: Vec<_> = (0..10)
            .into_par_iter()
            .map(|i| {
                let _guard = ctx.enter();
                // Context should be available
                let context = get_current_context();
                (i, context.phase)
            })
            .collect();

        for (_, phase) in results {
            assert_eq!(phase, Some(AnalysisPhase::DebtScoring));
        }
    }

    #[test]
    fn test_file_context_per_item() {
        let ctx = ParallelContext::capture();
        let files = vec![
            PathBuf::from("a.rs"),
            PathBuf::from("b.rs"),
            PathBuf::from("c.rs"),
        ];

        files.par_iter().for_each(|path| {
            process_file_with_context(path, &ctx, || {
                let context = get_current_context();
                assert_eq!(context.current_file.as_ref(), Some(path));
            });
        });
    }
}
```

### Integration Tests

```rust
#[test]
fn test_panic_shows_file_context() {
    let files = vec![PathBuf::from("test.rs")];
    let ctx = ParallelContext::capture();

    let result = std::panic::catch_unwind(|| {
        files.par_iter().for_each(|path| {
            process_file_with_context(path, &ctx, || {
                panic!("test panic in worker");
            });
        });
    });

    assert!(result.is_err());
    // Crash report should show file: test.rs
}
```

### Performance Tests

```rust
#[bench]
fn bench_with_context_overhead(b: &mut Bencher) {
    let items: Vec<_> = (0..10000).collect();

    b.iter(|| {
        let ctx = ParallelContext::capture();
        items
            .par_iter()
            .map(|&x| {
                with_parallel_context(&ctx, || x * 2)
            })
            .sum::<i32>()
    });
}

#[bench]
fn bench_without_context(b: &mut Bencher) {
    let items: Vec<_> = (0..10000).collect();

    b.iter(|| {
        items
            .par_iter()
            .map(|&x| x * 2)
            .sum::<i32>()
    });
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Extension trait for parallel iterators with context propagation.
///
/// This trait provides methods to propagate tracing spans and analysis
/// context into rayon worker threads, ensuring that:
///
/// 1. Tracing spans in workers are children of the parent span
/// 2. Analysis context (phase, file) is available for crash reports
/// 3. Per-item file context is correctly set
///
/// # Example
///
/// ```rust
/// use debtmap::observability::parallel::ParallelContextExt;
///
/// files
///     .par_iter()
///     .map_with_context(|path| {
///         // Context propagated from parent thread
///         analyze_file(path)
///     })
///     .collect()
/// ```
pub trait ParallelContextExt<T>: ParallelIterator<Item = T> + Sized {
    // ...
}
```

### User Documentation

```markdown
## Parallel Processing and Observability

Debtmap processes files in parallel for performance. When a crash occurs
in a parallel worker, the crash report includes full context:

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                           DEBTMAP CRASH REPORT                               ║
╠══════════════════════════════════════════════════════════════════════════════╣
║  PANIC: index out of bounds                                                  ║
║  Location: src/priority/scoring.rs:287:13                                    ║
╠══════════════════════════════════════════════════════════════════════════════╣
║  OPERATION CONTEXT:                                                          ║
║    Phase: debt_scoring                                                       ║
║    File: /path/to/problematic_file.rs                                        ║
║    Progress: 2847 / 4231 files (67%)                                         ║
║  SPAN HIERARCHY:                                                             ║
║    unified_analysis > debt_scoring > process_file                            ║
╚══════════════════════════════════════════════════════════════════════════════╝
```

This context is preserved even in parallel worker threads.
```

## Implementation Notes

### Why Context Doesn't Propagate Automatically

Rayon uses a thread pool. When you call `par_iter()`, the closure runs on
arbitrary worker threads, not the calling thread. This means:

1. Thread-locals (like our AnalysisContext) aren't inherited
2. Tracing's current span isn't inherited
3. The worker has no knowledge of the parent's context

Our solution: Capture context before entering parallel section, then
explicitly enter it in each worker.

### Performance Considerations

- Context capture is cheap (clone a few pointers)
- Context entry is cheap (set thread-local, enter span)
- Main overhead is the per-item closure call
- For large items, overhead is negligible
- For tiny items (e.g., summing numbers), use raw par_iter

### When to Use Context Propagation

**Use `map_with_context`** when:
- Processing files or significant work items
- Debugging/observability is important
- Crash context is valuable

**Use raw `map`** when:
- Processing trivial items (numbers, small transforms)
- Maximum performance is critical
- No need for crash context

## Migration and Compatibility

### Breaking Changes

None. This is additive functionality.

### Migration Path

1. Import `ParallelContextExt` trait
2. Replace `.map(|x| ...)` with `.map_with_context(|x| ...)`
3. Or use `process_file_with_context` for file processing

### Example Migration

```rust
// Before
file_metrics
    .par_iter()
    .map(|(path, metrics)| {
        score_file(path, metrics)
    })
    .collect()

// After
let ctx = ParallelContext::capture();
file_metrics
    .par_iter()
    .map(|(path, metrics)| {
        process_file_with_context(path, &ctx, || {
            score_file(path, metrics)
        })
    })
    .collect()
```
