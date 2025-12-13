---
number: 262
title: Effects-Based Progress System
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 262: Effects-Based Progress System

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap's current progress reporting system (47 `ProgressManager::global()` calls in unified_analysis.rs alone) violates Stillwater's "Pure Core, Imperative Shell" principle. Progress reporting is deeply interleaved with computation logic, making it impossible to:

1. **Test pure computation** without UI dependencies
2. **Compose analysis phases** cleanly
3. **Handle errors** with automatic cleanup
4. **Support different output modes** (TUI, CLI, silent)

The existing effects infrastructure (`src/effects.rs`) provides Reader pattern helpers (`asks_config`, `asks_thresholds`) that demonstrate the right pattern. Progress should follow the same model - becoming a capability accessed through the environment.

**Current Problem:**

```rust
// Mixed concerns throughout unified_analysis.rs
fn perform_unified_analysis_computation(...) {
    ProgressManager::global().start_phase("Call Graph");  // Side effect!
    let graph = build_call_graph();  // Pure
    ProgressManager::global().update(50);  // Side effect!
    let metrics = analyze_files();  // Pure
    ProgressManager::global().complete();  // Side effect!
}
```

**Target Architecture:**

```rust
// Composition via effects
fn analyze_codebase(files: Vec<PathBuf>) -> AnalysisEffect<UnifiedAnalysis> {
    with_stage("Call Graph", build_call_graph_effect(files.clone()))
        .and_then(|graph|
            traverse_with_progress(files, "File Analysis", analyze_file_effect)
        )
}
```

## Objective

Implement an effects-based progress system that:

1. **Defines `ProgressSink` trait** for progress reporting abstraction
2. **Extends `RealEnv`** with progress capability via `HasProgress` trait
3. **Provides progress combinators** (`with_stage`, `traverse_with_progress`, `par_traverse_with_progress`)
4. **Implements progress sinks** (TUI, CLI, Silent, Recording)
5. **Integrates with existing effects** (`asks_config`, `with_retry`)

Result: Progress reporting that composes naturally with effects, enabling clean separation of pure computation from I/O.

## Requirements

### Functional Requirements

1. **ProgressSink Trait**
   - `report(stage, current, total)` - Report item-level progress
   - `start_stage(name)` - Begin a named stage
   - `complete_stage(name)` - Complete a named stage
   - `warn(message)` - Report warnings without interruption
   - `child(prefix)` - Create nested progress sink

2. **HasProgress Trait**
   - Environment extension trait
   - `progress() -> &dyn ProgressSink`
   - Implemented by `RealEnv`

3. **Progress Combinators**
   - `with_stage(name, effect)` - Wrap effect with stage tracking
   - `traverse_with_progress(items, name, f)` - Sequential traversal with progress
   - `par_traverse_with_progress(items, name, f)` - Parallel traversal with atomic progress
   - `report_progress(stage, current, total)` - Direct progress effect

4. **Progress Implementations**
   - `TuiProgressSink` - Updates ratatui App state
   - `CliProgressSink` - Simple stderr output
   - `SilentProgressSink` - No-op for testing/CI
   - `RecordingProgressSink` - Captures events for testing

### Non-Functional Requirements

1. **Performance**
   - Zero overhead when using `SilentProgressSink`
   - Minimal allocation in hot paths
   - Thread-safe progress updates

2. **Composability**
   - Natural composition with existing effects
   - Nested stages work correctly
   - Error cleanup automatic via bracket

3. **Testability**
   - Pure computation testable without progress mocks
   - `RecordingProgressSink` for behavior verification

## Acceptance Criteria

- [ ] `ProgressSink` trait defined in `src/progress/traits.rs`
- [ ] `HasProgress` trait extends environment capability
- [ ] `RealEnv` includes progress field and implements `HasProgress`
- [ ] `with_stage` combinator provides start/complete bracketing
- [ ] `traverse_with_progress` reports per-item progress
- [ ] `par_traverse_with_progress` uses `AtomicUsize` for thread safety
- [ ] `TuiProgressSink` integrates with existing TUI
- [ ] `CliProgressSink` provides simple CLI output
- [ ] `SilentProgressSink` has zero overhead
- [ ] `RecordingProgressSink` captures all events
- [ ] All existing tests pass with `SilentProgressSink`
- [ ] Documentation covers usage patterns
- [ ] No clippy warnings

## Technical Details

### Implementation Approach

**Phase 1: Define Traits**

```rust
// src/progress/traits.rs

use std::sync::Arc;

/// Progress sink abstraction - receives progress updates.
///
/// Implementations handle progress visualization (TUI, CLI, logging).
/// All methods should be cheap - expensive work should be deferred.
pub trait ProgressSink: Send + Sync + 'static {
    /// Report progress for a named stage.
    fn report(&self, stage: &str, current: usize, total: usize);

    /// Report a sub-stage starting.
    fn start_stage(&self, name: &str);

    /// Report a stage completing.
    fn complete_stage(&self, name: &str);

    /// Report a warning without interrupting progress.
    fn warn(&self, message: &str);

    /// Create a child sink for nested progress.
    fn child(&self, prefix: &str) -> Arc<dyn ProgressSink>;
}

/// Environment extension for progress capability.
pub trait HasProgress {
    fn progress(&self) -> &dyn ProgressSink;
}
```

**Phase 2: Extend Environment**

```rust
// src/env.rs additions

use crate::progress::traits::{HasProgress, ProgressSink, SilentProgressSink};

#[derive(Clone)]
pub struct RealEnv {
    file_system: Arc<dyn FileSystem>,
    coverage_loader: Arc<dyn CoverageLoader>,
    cache: Arc<dyn Cache>,
    config: DebtmapConfig,
    progress: Arc<dyn ProgressSink>,  // NEW
}

impl RealEnv {
    pub fn new(config: DebtmapConfig) -> Self {
        Self::with_progress(config, Arc::new(SilentProgressSink))
    }

    pub fn with_progress(config: DebtmapConfig, progress: Arc<dyn ProgressSink>) -> Self {
        Self {
            file_system: Arc::new(RealFileSystem::new()),
            coverage_loader: Arc::new(RealCoverageLoader::new()),
            cache: Arc::new(MemoryCache::new()),
            config,
            progress,
        }
    }

    pub fn set_progress(self, progress: Arc<dyn ProgressSink>) -> Self {
        Self { progress, ..self }
    }
}

impl HasProgress for RealEnv {
    fn progress(&self) -> &dyn ProgressSink {
        &*self.progress
    }
}
```

**Phase 3: Progress Combinators**

```rust
// src/effects/progress.rs

use crate::errors::AnalysisError;
use crate::progress::traits::HasProgress;
use stillwater::effect::prelude::*;
use stillwater::Effect;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Wrap an effect with stage tracking.
///
/// Automatically calls `start_stage` before and `complete_stage` after,
/// even if the effect fails (bracket pattern).
pub fn with_stage<T, Env>(
    stage_name: &str,
    effect: impl Effect<Output = T, Error = AnalysisError, Env = Env>,
) -> impl Effect<Output = T, Error = AnalysisError, Env = Env>
where
    Env: HasProgress + Clone + Send + Sync + 'static,
    T: Send + 'static,
{
    let start_name = stage_name.to_string();
    let end_name = stage_name.to_string();

    from_async(move |env: &Env| {
        let env = env.clone();
        let start = start_name.clone();
        let end = end_name.clone();
        async move {
            env.progress().start_stage(&start);
            let result = effect.run(&env).await;
            env.progress().complete_stage(&end);
            result
        }
    })
}

/// Traverse items with automatic progress reporting.
pub fn traverse_with_progress<T, U, Env, F, Eff>(
    items: Vec<T>,
    stage_name: &str,
    f: F,
) -> impl Effect<Output = Vec<U>, Error = AnalysisError, Env = Env>
where
    T: Send + 'static,
    U: Send + 'static,
    Env: HasProgress + Clone + Send + Sync + 'static,
    F: Fn(T) -> Eff + Send + Sync + 'static,
    Eff: Effect<Output = U, Error = AnalysisError, Env = Env>,
{
    let name = stage_name.to_string();
    let total = items.len();

    from_async(move |env: &Env| {
        let env = env.clone();
        let stage = name.clone();
        async move {
            env.progress().start_stage(&stage);
            let mut results = Vec::with_capacity(total);

            for (i, item) in items.into_iter().enumerate() {
                env.progress().report(&stage, i, total);
                let result = f(item).run(&env).await?;
                results.push(result);
            }

            env.progress().complete_stage(&stage);
            Ok(results)
        }
    })
}

/// Parallel traverse with atomic progress counter.
pub fn par_traverse_with_progress<T, U, Env, F, Eff>(
    items: Vec<T>,
    stage_name: &str,
    f: F,
) -> impl Effect<Output = Vec<U>, Error = AnalysisError, Env = Env>
where
    T: Send + 'static,
    U: Send + 'static,
    Env: HasProgress + Clone + Send + Sync + 'static,
    F: Fn(T) -> Eff + Send + Sync + Clone + 'static,
    Eff: Effect<Output = U, Error = AnalysisError, Env = Env> + Send,
{
    let name = stage_name.to_string();
    let total = items.len();

    from_async(move |env: &Env| {
        let env = env.clone();
        let stage = name.clone();
        let f = f.clone();

        async move {
            env.progress().start_stage(&stage);
            let counter = Arc::new(AtomicUsize::new(0));

            // Use rayon for CPU parallelism, report progress atomically
            let results: Result<Vec<U>, AnalysisError> = items
                .into_iter()
                .map(|item| {
                    let current = counter.fetch_add(1, Ordering::Relaxed);
                    env.progress().report(&stage, current, total);
                    // Note: For true async parallelism, would need tokio::spawn
                    // This version uses sequential execution with progress
                    futures::executor::block_on(f(item).run(&env))
                })
                .collect();

            env.progress().complete_stage(&stage);
            results
        }
    })
}

/// Report progress for current operation.
pub fn report_progress<Env>(
    stage: &str,
    current: usize,
    total: usize,
) -> impl Effect<Output = (), Error = AnalysisError, Env = Env>
where
    Env: HasProgress + Clone + Send + Sync + 'static,
{
    let stage = stage.to_string();
    stillwater::asks(move |env: &Env| {
        env.progress().report(&stage, current, total);
    })
}
```

**Phase 4: Implementations**

```rust
// src/progress/implementations.rs

use super::traits::ProgressSink;
use std::sync::{Arc, Mutex};

/// Silent progress - no-op implementation for testing/CI.
#[derive(Clone, Default)]
pub struct SilentProgressSink;

impl ProgressSink for SilentProgressSink {
    fn report(&self, _stage: &str, _current: usize, _total: usize) {}
    fn start_stage(&self, _name: &str) {}
    fn complete_stage(&self, _name: &str) {}
    fn warn(&self, _message: &str) {}
    fn child(&self, _prefix: &str) -> Arc<dyn ProgressSink> {
        Arc::new(SilentProgressSink)
    }
}

/// CLI progress - simple stderr output.
#[derive(Clone)]
pub struct CliProgressSink {
    quiet: bool,
}

impl CliProgressSink {
    pub fn new(quiet: bool) -> Self {
        Self { quiet }
    }
}

impl ProgressSink for CliProgressSink {
    fn report(&self, stage: &str, current: usize, total: usize) {
        if !self.quiet {
            eprint!("\r{}: {}/{}", stage, current + 1, total);
        }
    }

    fn start_stage(&self, name: &str) {
        if !self.quiet {
            eprintln!("\n{}", name);
        }
    }

    fn complete_stage(&self, name: &str) {
        if !self.quiet {
            eprintln!("\n{} complete", name);
        }
    }

    fn warn(&self, message: &str) {
        eprintln!("\nWarning: {}", message);
    }

    fn child(&self, prefix: &str) -> Arc<dyn ProgressSink> {
        Arc::new(CliProgressSink { quiet: self.quiet })
    }
}

/// Recording progress - captures events for testing.
#[derive(Clone, Default)]
pub struct RecordingProgressSink {
    events: Arc<Mutex<Vec<ProgressEvent>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProgressEvent {
    Report { stage: String, current: usize, total: usize },
    StartStage { name: String },
    CompleteStage { name: String },
    Warn { message: String },
}

impl RecordingProgressSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<ProgressEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn stages(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter_map(|e| match e {
                ProgressEvent::StartStage { name } => Some(name),
                _ => None,
            })
            .collect()
    }
}

impl ProgressSink for RecordingProgressSink {
    fn report(&self, stage: &str, current: usize, total: usize) {
        self.events.lock().unwrap().push(ProgressEvent::Report {
            stage: stage.to_string(),
            current,
            total,
        });
    }

    fn start_stage(&self, name: &str) {
        self.events.lock().unwrap().push(ProgressEvent::StartStage {
            name: name.to_string(),
        });
    }

    fn complete_stage(&self, name: &str) {
        self.events.lock().unwrap().push(ProgressEvent::CompleteStage {
            name: name.to_string(),
        });
    }

    fn warn(&self, message: &str) {
        self.events.lock().unwrap().push(ProgressEvent::Warn {
            message: message.to_string(),
        });
    }

    fn child(&self, _prefix: &str) -> Arc<dyn ProgressSink> {
        Arc::new(self.clone())
    }
}
```

### Architecture Changes

```
src/
├── progress/
│   ├── mod.rs              - Re-exports
│   ├── traits.rs           - ProgressSink, HasProgress
│   └── implementations.rs  - Silent, CLI, Recording, TUI
├── effects/
│   ├── mod.rs              - Current effects module
│   └── progress.rs         - Progress combinators (NEW)
└── env.rs                  - Add progress field, HasProgress impl
```

### Integration with TUI

```rust
// src/tui/progress_sink.rs

use crate::progress::traits::ProgressSink;
use crate::tui::app::App;
use std::sync::{Arc, Mutex};

pub struct TuiProgressSink {
    app: Arc<Mutex<App>>,
    prefix: String,
}

impl TuiProgressSink {
    pub fn new(app: Arc<Mutex<App>>) -> Self {
        Self { app, prefix: String::new() }
    }
}

impl ProgressSink for TuiProgressSink {
    fn report(&self, stage: &str, current: usize, total: usize) {
        if let Ok(mut app) = self.app.lock() {
            app.update_progress(stage, current, total);
        }
    }

    fn start_stage(&self, name: &str) {
        if let Ok(mut app) = self.app.lock() {
            app.start_stage(name);
        }
    }

    fn complete_stage(&self, name: &str) {
        if let Ok(mut app) = self.app.lock() {
            app.complete_stage(name);
        }
    }

    fn warn(&self, message: &str) {
        if let Ok(mut app) = self.app.lock() {
            app.add_warning(message);
        }
    }

    fn child(&self, prefix: &str) -> Arc<dyn ProgressSink> {
        Arc::new(TuiProgressSink {
            app: self.app.clone(),
            prefix: format!("{}{}", self.prefix, prefix),
        })
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/env.rs` - Add progress field
  - `src/effects.rs` - Add progress helpers
  - `src/progress/` - New module
  - `src/tui/mod.rs` - TUI integration
- **External Dependencies**: None (uses existing stillwater)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_with_stage_calls_start_and_complete() {
        let recorder = Arc::new(RecordingProgressSink::new());
        let env = RealEnv::default().set_progress(recorder.clone());

        let effect = with_stage("Test Stage", effect_pure(42));
        let result = effect.run(&env).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(recorder.stages(), vec!["Test Stage"]);
    }

    #[tokio::test]
    async fn test_with_stage_completes_on_error() {
        let recorder = Arc::new(RecordingProgressSink::new());
        let env = RealEnv::default().set_progress(recorder.clone());

        let effect: AnalysisEffect<i32> = with_stage(
            "Failing Stage",
            effect_fail(AnalysisError::other("test error"))
        );
        let result = effect.run(&env).await;

        assert!(result.is_err());
        // Stage should still be completed (bracket cleanup)
        let events = recorder.events();
        assert!(events.contains(&ProgressEvent::StartStage {
            name: "Failing Stage".to_string()
        }));
        assert!(events.contains(&ProgressEvent::CompleteStage {
            name: "Failing Stage".to_string()
        }));
    }

    #[tokio::test]
    async fn test_traverse_with_progress_reports_each_item() {
        let recorder = Arc::new(RecordingProgressSink::new());
        let env = RealEnv::default().set_progress(recorder.clone());

        let items = vec![1, 2, 3];
        let effect = traverse_with_progress(items, "Processing", |n| effect_pure(n * 2));
        let result = effect.run(&env).await;

        assert_eq!(result.unwrap(), vec![2, 4, 6]);

        let reports: Vec<_> = recorder.events()
            .into_iter()
            .filter(|e| matches!(e, ProgressEvent::Report { .. }))
            .collect();
        assert_eq!(reports.len(), 3);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_progress_integrates_with_existing_effects() {
    let recorder = Arc::new(RecordingProgressSink::new());
    let config = DebtmapConfig::default();
    let env = RealEnv::with_progress(config, recorder.clone());

    // Compose with existing Reader pattern
    let effect = with_stage("Config Access",
        asks_config(|config| config.get_ignore_patterns())
    );

    let result = effect.run(&env).await;
    assert!(result.is_ok());
    assert_eq!(recorder.stages(), vec!["Config Access"]);
}
```

## Documentation Requirements

### Code Documentation

All public traits and functions documented with:
- Purpose and usage
- Thread safety guarantees
- Example code

### User Documentation

Add to `ARCHITECTURE.md`:
- Progress system overview
- How to use progress combinators
- How to implement custom sinks

## Implementation Notes

### Thread Safety

- `ProgressSink` methods must be cheap and non-blocking
- Use `Arc<Mutex>` or `Arc<AtomicUsize>` for mutable state
- `par_traverse_with_progress` uses atomic counter

### Performance

- `SilentProgressSink` compiles to no-ops
- Avoid string allocation in hot paths
- Progress updates should be O(1)

### Migration Path

1. Add progress infrastructure (this spec)
2. Migrate `unified_analysis.rs` to use progress combinators (Spec 265)
3. Remove `ProgressManager::global()` calls

## Migration and Compatibility

### Breaking Changes

None - new additive functionality.

### Backward Compatibility

- `RealEnv::new()` defaults to `SilentProgressSink`
- Existing code works unchanged
- Opt-in to progress reporting

## Success Metrics

- Zero `ProgressManager::global()` calls after Phase 3 migration
- All analysis effects composable with progress
- TUI and CLI both work with same analysis code
- Tests run silently without progress noise
