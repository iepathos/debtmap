---
number: 268
title: Effects System Expansion
category: architecture
priority: medium
status: draft
dependencies: [262, 265]
created: 2025-12-13
---

# Specification 268: Effects System Expansion

**Category**: architecture
**Priority**: medium
**Status**: draft
**Dependencies**: 262 (Effects-Based Progress), 265 (Pure Core Extraction)

## Context

Debtmap has a well-designed effects system in `src/effects.rs` and `src/analysis/effects.rs`, but adoption is limited to ~20 files. Key patterns exist but aren't used consistently:

**Existing Infrastructure:**
- `Effect<T, E, Env>` - Core effect type with Reader pattern
- `AnalysisEffect<T>` - Type alias for analysis operations
- `asks_config`, `asks_thresholds` - Reader helpers for config access
- `with_retry` - Effect combinator for retries
- `AnalysisValidation<T>` - Validation/accumulating errors

**Current Problems:**

```rust
// Most analysis code bypasses effects:
fn analyze_file(path: &Path, config: &Config) -> Result<Metrics> {
    let content = fs::read_to_string(path)?;  // Direct I/O
    let ast = parse_content(&content)?;
    compute_metrics(&ast, config)  // Config passed manually
}

// Instead of using effects:
fn analyze_file_effect(path: PathBuf) -> AnalysisEffect<Metrics> {
    read_file_effect(path)
        .and_then(|content| asks_config(|config| {
            let ast = parse_content(&content)?;
            compute_metrics(&ast, config)
        }))
}
```

**Stillwater Philosophy:**

> "Pure Core, Imperative Shell" - Effects enable clean separation of what to do from how to do it.

## Objective

Expand effects system adoption to core analysis paths:

1. **File analysis pipeline** - Use effects for I/O and config
2. **Scoring functions** - Use Reader pattern for config access
3. **Analyzer modules** - Convert to effect-based interfaces

Result: Consistent use of effects across codebase, enabling testability and composability.

## Requirements

### Functional Requirements

1. **File Analysis Effects**
   - `read_file_effect(path: PathBuf) -> AnalysisEffect<String>`
   - `parse_file_effect(content: String) -> AnalysisEffect<Ast>`
   - `analyze_file_effect(path: PathBuf) -> AnalysisEffect<FileMetrics>`

2. **Scoring Effects**
   - Convert scoring functions to use `asks_config`
   - Remove explicit config parameters where appropriate
   - Use `asks_thresholds` for threshold access

3. **Analyzer Conversions**
   - `src/analyzers/*.rs` - Effect-based interfaces
   - Preserve pure computation functions internally
   - Effects at module boundaries

4. **Effect Combinators**
   - `traverse_effect` - Sequential effect mapping
   - `par_traverse_effect` - Parallel effect mapping
   - Combine with progress from spec 262

### Non-Functional Requirements

1. **Consistency**
   - All I/O through effects
   - Config access via Reader pattern
   - No direct filesystem calls in analysis code

2. **Testability**
   - Mock environments for testing
   - No I/O in unit tests
   - Effects compose for integration tests

3. **Performance**
   - No regression from effect overhead
   - Parallel analysis preserved

## Acceptance Criteria

- [ ] File reading uses `read_file_effect`
- [ ] Scoring uses `asks_config` for configuration
- [ ] Analyzers expose effect-based public APIs
- [ ] No direct `fs::read_to_string` in analyzer modules
- [ ] Test environments work with all effects
- [ ] All existing tests pass
- [ ] No clippy warnings

## Technical Details

### Effect Type Hierarchy

```rust
// Core effect types (existing in src/effects.rs)
pub struct Effect<T, E, Env> {
    run: Box<dyn FnOnce(&Env) -> Result<T, E> + Send>,
}

// Analysis-specific aliases
pub type AnalysisEffect<T> = Effect<T, AnalysisError, RealEnv>;
pub type AnalysisValidation<T> = Validation<T, Vec<AnalysisError>>;

// Extended environment (from spec 262)
pub trait FullEnv: AnalysisEnv + HasProgress {}
```

### Implementation Approach

**Phase 1: File I/O Effects**

```rust
// src/effects/io.rs

use crate::env::AnalysisEnv;
use crate::effects::{Effect, asks};
use std::path::PathBuf;

/// Read file contents via environment
pub fn read_file_effect<Env>(path: PathBuf) -> Effect<String, AnalysisError, Env>
where
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
{
    Effect::new(move |env: &Env| {
        env.file_system()
            .read_to_string(&path)
            .map_err(|e| AnalysisError::io_error(&path, e))
    })
}

/// Check if file exists via environment
pub fn file_exists_effect<Env>(path: PathBuf) -> Effect<bool, AnalysisError, Env>
where
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
{
    asks(move |env: &Env| env.file_system().exists(&path))
}

/// List directory contents via environment
pub fn list_dir_effect<Env>(path: PathBuf) -> Effect<Vec<PathBuf>, AnalysisError, Env>
where
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
{
    Effect::new(move |env: &Env| {
        env.file_system()
            .list_dir(&path)
            .map_err(|e| AnalysisError::io_error(&path, e))
    })
}
```

**Phase 2: Scoring with Reader Pattern**

```rust
// src/priority/scoring/effects.rs

use crate::effects::{AnalysisEffect, asks_config, asks_thresholds, pure};
use crate::priority::scoring::core::{
    calculate_complexity_score_pure,
    calculate_debt_priority_pure,
};

/// Score a file's complexity using config from environment
pub fn score_file_effect(metrics: FileMetrics) -> AnalysisEffect<ComplexityScore> {
    asks_config(move |config| {
        Ok(calculate_complexity_score_pure(&metrics, &config.scoring.weights))
    })
}

/// Determine debt priority using thresholds from environment
pub fn prioritize_debt_effect(score: ComplexityScore) -> AnalysisEffect<DebtPriority> {
    asks_thresholds(move |thresholds| {
        Ok(calculate_debt_priority_pure(&score, thresholds))
    })
}

/// Score multiple files, combining scores into report
pub fn score_files_effect(
    metrics: Vec<FileMetrics>,
) -> AnalysisEffect<Vec<ComplexityScore>> {
    traverse_effect(metrics, score_file_effect)
}
```

**Phase 3: Analyzer Module Conversion**

```rust
// src/analyzers/complexity/effects.rs

use crate::effects::{AnalysisEffect, asks_config, pure};
use super::core::{calculate_cyclomatic, calculate_cognitive};

/// Analyze complexity of a parsed AST
pub fn analyze_complexity_effect(ast: Ast) -> AnalysisEffect<ComplexityMetrics> {
    asks_config(move |config| {
        let cyclomatic = calculate_cyclomatic(&ast);
        let cognitive = calculate_cognitive(&ast, config.cognitive_weights());

        Ok(ComplexityMetrics {
            cyclomatic,
            cognitive,
            nesting_depth: ast.max_nesting_depth(),
        })
    })
}

/// Full file analysis effect
pub fn analyze_file_effect(path: PathBuf) -> AnalysisEffect<FileAnalysis> {
    read_file_effect(path.clone())
        .and_then(|content| parse_file_effect(content, path.clone()))
        .and_then(|ast| analyze_complexity_effect(ast))
        .map(|metrics| FileAnalysis { path, metrics })
}
```

**Phase 4: Effect Combinators**

```rust
// src/effects/combinators.rs

/// Sequential traverse - map effect over collection
pub fn traverse_effect<T, U, E, Env, F>(
    items: Vec<T>,
    f: F,
) -> Effect<Vec<U>, E, Env>
where
    T: Send + 'static,
    U: Send + 'static,
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
    F: Fn(T) -> Effect<U, E, Env> + Send + 'static,
{
    Effect::new(move |env: &Env| {
        items
            .into_iter()
            .map(|item| f(item).run(env))
            .collect()
    })
}

/// Parallel traverse - map effect over collection in parallel
pub fn par_traverse_effect<T, U, E, Env, F>(
    items: Vec<T>,
    f: F,
) -> Effect<Vec<U>, E, Env>
where
    T: Send + 'static,
    U: Send + 'static,
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
    F: Fn(T) -> Effect<U, E, Env> + Send + Sync + 'static,
{
    Effect::new(move |env: &Env| {
        use rayon::prelude::*;
        items
            .into_par_iter()
            .map(|item| f(item).run(env))
            .collect()
    })
}

/// Filter with predicate effect
pub fn filter_effect<T, E, Env, F>(
    items: Vec<T>,
    predicate: F,
) -> Effect<Vec<T>, E, Env>
where
    T: Send + 'static,
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
    F: Fn(&T) -> Effect<bool, E, Env> + Send + 'static,
{
    Effect::new(move |env: &Env| {
        items
            .into_iter()
            .filter_map(|item| {
                match predicate(&item).run(env) {
                    Ok(true) => Some(Ok(item)),
                    Ok(false) => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .collect()
    })
}
```

**Phase 5: Test Environment**

```rust
// src/testing/test_env.rs

use crate::env::AnalysisEnv;
use crate::config::DebtmapConfig;
use crate::io::traits::{FileSystem, CoverageLoader, Cache};
use std::collections::HashMap;
use std::sync::Arc;

/// Test environment with in-memory file system
#[derive(Clone)]
pub struct TestEnv {
    files: Arc<HashMap<PathBuf, String>>,
    config: DebtmapConfig,
    progress: Arc<dyn ProgressSink>,
}

impl TestEnv {
    pub fn new() -> Self {
        Self {
            files: Arc::new(HashMap::new()),
            config: DebtmapConfig::default(),
            progress: Arc::new(SilentProgressSink),
        }
    }

    pub fn with_file(mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Arc::make_mut(&mut self.files).insert(path.into(), content.into());
        self
    }

    pub fn with_config(mut self, config: DebtmapConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_recording_progress(mut self) -> (Self, RecordingProgressSink) {
        let sink = RecordingProgressSink::new();
        self.progress = Arc::new(sink.clone());
        (self, sink)
    }
}

impl AnalysisEnv for TestEnv {
    fn file_system(&self) -> &dyn FileSystem {
        &*self.files as &dyn FileSystem  // Implement FileSystem for HashMap
    }

    fn config(&self) -> &DebtmapConfig {
        &self.config
    }

    fn with_config(self, config: DebtmapConfig) -> Self {
        Self { config, ..self }
    }

    // ... other trait methods
}
```

### Files to Modify

**New Files:**

1. **Create** `src/effects/io.rs` - File I/O effects
2. **Create** `src/effects/combinators.rs` - Effect combinators
3. **Create** `src/testing/test_env.rs` - Test environment

**Modify Existing:**

4. **Modify** `src/effects/mod.rs` - Re-export new modules
5. **Modify** `src/analyzers/complexity/mod.rs` - Add effect APIs
6. **Modify** `src/analyzers/cyclomatic.rs` - Effect wrappers
7. **Modify** `src/analyzers/cognitive.rs` - Effect wrappers
8. **Modify** `src/priority/scoring/mod.rs` - Reader pattern
9. **Modify** `src/builders/unified_analysis.rs` - Use effects throughout

### Migration Strategy

1. **Add I/O effects** - New functions, don't remove old
2. **Add scoring effects** - Parallel to existing
3. **Convert analyzers** - One module at a time
4. **Update callers** - Gradually switch to effect versions
5. **Deprecate old** - Mark non-effect versions deprecated
6. **Remove old** - After full migration

### Usage Examples

**Before (Direct I/O):**

```rust
fn analyze_project(paths: &[PathBuf], config: &Config) -> Result<Vec<FileMetrics>> {
    paths
        .iter()
        .map(|p| {
            let content = fs::read_to_string(p)?;
            let ast = parse(&content)?;
            Ok(compute_metrics(&ast, config))
        })
        .collect()
}
```

**After (Effect-Based):**

```rust
fn analyze_project_effect(paths: Vec<PathBuf>) -> AnalysisEffect<Vec<FileMetrics>> {
    par_traverse_with_progress(paths, "Analyzing", analyze_file_effect)
}

// Usage
let env = RealEnv::new(config);
let results = analyze_project_effect(paths).run(&env)?;

// Testing
let test_env = TestEnv::new()
    .with_file("test.rs", "fn main() {}")
    .with_config(test_config);
let results = analyze_project_effect(vec!["test.rs".into()]).run(&test_env)?;
```

## Dependencies

- **Prerequisites**:
  - 262 (Effects-Based Progress System)
  - 265 (Pure Core Extraction)
- **Affected Components**:
  - `src/effects/`
  - `src/analyzers/`
  - `src/priority/scoring/`
  - `src/builders/`
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestEnv;

    #[test]
    fn test_read_file_effect() {
        let env = TestEnv::new()
            .with_file("test.rs", "fn main() {}");

        let result = read_file_effect("test.rs".into())
            .run(&env)
            .unwrap();

        assert_eq!(result, "fn main() {}");
    }

    #[test]
    fn test_read_file_not_found() {
        let env = TestEnv::new();

        let result = read_file_effect("missing.rs".into())
            .run(&env);

        assert!(result.is_err());
    }

    #[test]
    fn test_score_file_uses_config() {
        let config = DebtmapConfig {
            scoring: ScoringConfig {
                weights: Weights { complexity: 2.0, .. },
                ..Default::default()
            },
            ..Default::default()
        };

        let env = TestEnv::new().with_config(config);
        let metrics = FileMetrics { cyclomatic: 10, .. };

        let score = score_file_effect(metrics)
            .run(&env)
            .unwrap();

        assert_eq!(score.total, 20.0);  // 10 * 2.0
    }
}
```

### Integration Tests

```rust
#[test]
fn test_full_analysis_pipeline_with_effects() {
    let env = TestEnv::new()
        .with_file("src/lib.rs", include_str!("../fixtures/sample.rs"))
        .with_file("src/main.rs", include_str!("../fixtures/main.rs"));

    let paths = vec!["src/lib.rs".into(), "src/main.rs".into()];

    let results = analyze_project_effect(paths)
        .run(&env)
        .unwrap();

    assert_eq!(results.len(), 2);
    assert!(results[0].metrics.cyclomatic > 0);
}
```

### Property Tests

```rust
proptest! {
    #[test]
    fn traverse_effect_preserves_length(
        items in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let env = TestEnv::new();
        let len = items.len();

        let effect = traverse_effect(items, |x| pure(x * 2));
        let results = effect.run(&env).unwrap();

        prop_assert_eq!(results.len(), len);
    }
}
```

## Documentation Requirements

### Code Documentation

- Module-level docs explaining effect patterns
- Examples in function docs
- Migration guide for existing code

### Architecture Documentation

Add to `ARCHITECTURE.md`:
- Effects system overview
- When to use effects vs pure functions
- Testing with test environments

## Implementation Notes

### Performance Considerations

Effects add minimal overhead:
- Single closure invocation per effect
- No heap allocation for simple effects
- Parallel combinators use rayon

### Gradual Migration

Keep both APIs during transition:

```rust
// Old API (deprecated but works)
pub fn analyze_file(path: &Path, config: &Config) -> Result<Metrics> { ... }

// New API (effects-based)
pub fn analyze_file_effect(path: PathBuf) -> AnalysisEffect<Metrics> { ... }
```

### Pitfalls to Avoid

1. **Effect in effect** - Don't nest effects unnecessarily; use `and_then`
2. **Blocking in async** - Effects should be CPU-bound or use async properly
3. **Lost errors** - Always propagate errors; don't use `.ok()` carelessly

## Migration and Compatibility

### Breaking Changes

None initially - new effect functions added alongside existing.

### Deprecation Plan

After effects adoption is widespread:

```rust
#[deprecated(since = "0.12.0", note = "Use analyze_file_effect instead")]
pub fn analyze_file(path: &Path, config: &Config) -> Result<Metrics> {
    // Internally calls effect version
}
```

## Success Metrics

- All file I/O uses effect-based functions
- All scoring uses Reader pattern for config
- Test coverage uses `TestEnv` exclusively
- No direct `fs::*` calls in analyzer modules
- Effect usage in 80%+ of analysis code paths
