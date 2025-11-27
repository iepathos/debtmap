---
number: 199
title: Reader Pattern for Configuration Management
category: foundation
priority: medium
status: ready
dependencies: [195, 196, 197, 198]
created: 2025-11-24
updated: 2025-11-27
---

# Specification 199: Reader Pattern for Configuration Management

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 195-198 (Stillwater Foundation through Effect Composition)

## Context

Debtmap's configuration is currently threaded through function parameters across the entire codebase. A typical call chain looks like:

```rust
analyze_project(config) →
  analyze_files(files, config) →
    analyze_file(file, config) →
      calculate_metrics(ast, config.thresholds) →
        calculate_score(metrics, config.scoring) →
          apply_adjustments(score, config.patterns)
```

This creates several problems:
- **Parameter pollution**: Config appears in 200+ function signatures
- **Refactoring friction**: Adding config fields requires updating many functions
- **Testing complexity**: Must pass config even when unused
- **Unclear dependencies**: Can't tell which config fields a function actually uses

The **Reader Pattern** solves this by making configuration available through the environment, eliminating parameter threading while making dependencies explicit in return types.

## Stillwater 0.11.0 Support

Stillwater 0.11.0 provides all necessary Reader pattern primitives as **zero-cost abstractions**:

| Primitive | Function | Description |
|-----------|----------|-------------|
| `Ask` | `ask::<E, Env>()` | Get the entire environment (cloned) |
| `Asks` | `asks(\|env\| ...)` | Query a value from the environment |
| `Local` | `local(transform, effect)` | Run effect with modified environment |

These are re-exported from `stillwater::effect::prelude::*` and `stillwater::{ask, asks, local}`.

**Debtmap responsibility:** Define the `AnalysisEnv` trait that exposes domain-specific methods like `config()`, `file_system()`, etc. Stillwater provides the generic Reader machinery.

## Objective

Eliminate configuration parameter threading by implementing the Reader pattern via stillwater's `Effect::asks()`, reducing function parameters by ~75% while making configuration dependencies explicit and type-safe.

## Requirements

### Functional Requirements

#### Environment Access
- Config accessible via `env.config()` from any Effect
- `Effect::asks()` for reading config fields
- `Effect::local()` for temporary config overrides
- Type-safe access to nested config fields

#### Function Conversion
- Convert config-taking functions to use `asks()`
- Remove config parameters from 150+ functions
- Maintain same behavior and logic
- Preserve type safety

#### Temporary Overrides
- Support strict mode via `Effect::local()`
- Support custom thresholds for specific operations
- Support feature flags and toggles
- Revert automatically after operation

#### Testing Support
- Mock config in tests via MockEnv
- Override specific fields for test cases
- Verify correct config fields accessed

### Non-Functional Requirements
- No performance regression
- No breaking changes to public API
- All tests pass
- Backwards-compatible wrappers

## Acceptance Criteria

- [ ] Config accessible via `env.config()`
- [ ] `Effect::asks()` used in 100+ functions
- [ ] `Effect::local()` used for temporary overrides
- [ ] Config parameters removed from internal functions
- [ ] Public API maintains backwards compatibility
- [ ] Tests use MockEnv with custom config
- [ ] Documentation explains Reader pattern
- [ ] Type safety preserved

## Technical Details

### Implementation Approach

#### 1. Environment Config Access

```rust
// src/env.rs - Already done in Spec 195
pub trait AnalysisEnv {
    fn config(&self) -> &Config;
    // ... other methods
}
```

#### 2. Convert Functions to Use Reader Pattern

**Before: Parameter Threading**
```rust
// Config passed everywhere
pub fn analyze_file(
    path: &Path,
    config: &Config,
) -> Result<FileAnalysis> {
    let ast = parse(path)?;
    let metrics = calculate_metrics(ast, &config.thresholds)?;
    let score = calculate_score(metrics, &config.scoring)?;
    Ok(FileAnalysis { metrics, score })
}

fn calculate_metrics(
    ast: Ast,
    thresholds: &Thresholds,
) -> Result<Metrics> {
    // Uses thresholds
}

fn calculate_score(
    metrics: Metrics,
    scoring: &ScoringConfig,
) -> Result<Score> {
    // Uses scoring
}
```

**After: Reader Pattern (using stillwater 0.11.0)**
```rust
use stillwater::{asks, Effect, EffectExt};

// No config parameters!
pub fn analyze_file_effect<Env>(
    path: PathBuf,
) -> impl Effect<Output = FileAnalysis, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    read_file_effect(path)
        .and_then(|content| parse_effect(content))
        .and_then(|ast| calculate_metrics_effect(ast))
        .and_then(|metrics| calculate_score_effect(metrics))
        .map(|(metrics, score)| FileAnalysis { metrics, score })
}

fn calculate_metrics_effect<Env>(
    ast: Ast,
) -> impl Effect<Output = Metrics, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    // Ask for thresholds when needed - zero-cost!
    asks(move |env: &Env| {
        let thresholds = &env.config().thresholds;
        calculate_metrics_pure(&ast, thresholds)
    })
}

fn calculate_score_effect<Env>(
    metrics: Metrics,
) -> impl Effect<Output = Score, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    // Ask for scoring config when needed - zero-cost!
    asks(move |env: &Env| {
        let scoring = &env.config().scoring;
        calculate_score_pure(&metrics, scoring)
    })
}
```

#### 3. Temporary Config Overrides (using stillwater::local)

```rust
// src/analysis/strict_mode.rs
use stillwater::{local, Effect};

/// Run analysis in strict mode (higher thresholds)
pub fn analyze_strict<Env>(
    path: PathBuf,
) -> impl Effect<Output = FileAnalysis, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    // local(transform_fn, inner_effect) - runs inner with modified env
    local(
        |env: &Env| {
            // Create modified environment
            env.with_config(Config {
                thresholds: Thresholds {
                    complexity: env.config().thresholds.complexity * 0.5,
                    coverage: 95.0,  // Stricter coverage requirement
                    ..env.config().thresholds
                },
                ..env.config().clone()
            })
        },
        analyze_file_effect(path)  // Runs with modified config
    )
}

/// Run with custom thresholds for specific operation
pub fn analyze_with_thresholds<Env>(
    path: PathBuf,
    custom_thresholds: Thresholds,
) -> impl Effect<Output = FileAnalysis, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    local(
        move |env: &Env| {
            env.with_config(Config {
                thresholds: custom_thresholds.clone(),
                ..env.config().clone()
            })
        },
        analyze_file_effect(path)
    )
}

/// Temporarily disable specific patterns
pub fn analyze_without_patterns<Env>(
    path: PathBuf,
    disabled_patterns: Vec<String>,
) -> impl Effect<Output = FileAnalysis, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    local(
        move |env: &Env| {
            let mut config = env.config().clone();
            config.patterns.retain(|p| !disabled_patterns.contains(p));
            env.with_config(config)
        },
        analyze_file_effect(path)
    )
}
```

#### 4. Nested Config Access (using stillwater::asks)

```rust
// src/scoring/effects.rs
use stillwater::{asks, Effect};

/// Access specific config sections directly
pub fn score_with_coverage_factor<Env>() -> impl Effect<Output = f64, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    asks(|env: &Env| env.config().scoring.coverage_weight)
}

/// Conditional logic based on config
pub fn should_analyze_tests<Env>() -> impl Effect<Output = bool, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    asks(|env: &Env| env.config().analysis.include_tests)
}

/// Get threshold for specific metric
pub fn get_complexity_threshold<Env>() -> impl Effect<Output = f64, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    asks(|env: &Env| env.config().thresholds.complexity)
}
```

#### 5. Backwards-Compatible Public API

```rust
// src/lib.rs - Public API maintains config parameters

/// Public API - still takes config for backwards compatibility
pub fn analyze_file(
    path: &Path,
    config: &Config,
) -> anyhow::Result<FileAnalysis> {
    let env = RealEnv::new(config.clone());

    // Internally uses Reader pattern
    analyze_file_effect(path.to_path_buf())
        .run(&env)
        .map_err(Into::into)
}

/// Public API - still takes config
pub fn analyze_project(
    root: &Path,
    config: &Config,
) -> anyhow::Result<ProjectAnalysis> {
    let env = RealEnv::new(config.clone());

    // Internally uses Reader pattern
    analyze_project_effect(root.to_path_buf())
        .run(&env)
        .map_err(Into::into)
}
```

### Testing with Reader Pattern (using stillwater::testing)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use stillwater::testing::{MockEnv, TestEffect};
    use stillwater::Effect;

    // Debtmap-specific test environment (see Spec 200)
    fn test_env(config: Config, files: Vec<(&str, &str)>) -> DebtmapTestEnv {
        DebtmapTestEnv::new()
            .with_config(config)
            .with_files(files)
    }

    #[tokio::test]
    async fn test_with_custom_config() {
        let config = Config {
            thresholds: Thresholds {
                complexity: 5.0,
                coverage: 90.0,
                depth: 3,
            },
            ..Config::default()
        };

        let env = test_env(config.clone(), vec![("test.rs", "fn foo() {}")]);

        let result = analyze_file_effect("test.rs".into())
            .run(&env)
            .await
            .unwrap();

        // Analysis used custom thresholds
        assert!(result.score.is_within_threshold(&config.thresholds));
    }

    #[tokio::test]
    async fn test_strict_mode() {
        let env = test_env(
            Config::default(),
            vec![("test.rs", "fn complex() { /* ... */ }")]
        );

        let normal = analyze_file_effect("test.rs".into())
            .run(&env)
            .await
            .unwrap();

        let strict = analyze_strict("test.rs".into())
            .run(&env)
            .await
            .unwrap();

        // Strict mode has higher score (stricter thresholds)
        assert!(strict.score > normal.score);
    }

    #[tokio::test]
    async fn test_temporary_override() {
        let env = test_env(Config::default(), vec![("test.rs", "fn foo() {}")]);

        let custom_thresholds = Thresholds {
            complexity: 1.0,  // Very strict
            coverage: 100.0,
            depth: 1,
        };

        let result = analyze_with_thresholds(
            "test.rs".into(),
            custom_thresholds.clone()
        )
        .run(&env)
        .await
        .unwrap();

        // Used custom thresholds
        assert_eq!(result.thresholds_used, custom_thresholds);
    }

    #[tokio::test]
    async fn test_config_field_access() {
        let config = Config {
            scoring: ScoringConfig {
                coverage_weight: 0.8,
                ..ScoringConfig::default()
            },
            ..Config::default()
        };

        let env = test_env(config, vec![]);

        let weight = score_with_coverage_factor()
            .run(&env)
            .await
            .unwrap();

        assert_eq!(weight, 0.8);
    }
}
```

### Migration Examples

**Before: Config Threading**
```rust
pub fn analyze_project(
    root: &Path,
    config: &Config,
) -> Result<ProjectAnalysis> {
    let files = discover_files(root, &config.exclusions)?;
    let analyses = files.iter()
        .map(|file| analyze_file(file, config))
        .collect::<Result<Vec<_>>>()?;
    aggregate_analyses(analyses, &config.aggregation)
}
```

**After: Reader Pattern (stillwater 0.11.0)**
```rust
use stillwater::{asks, traverse_effect, Effect, EffectExt};

pub fn analyze_project_effect<Env>(
    root: PathBuf,
) -> impl Effect<Output = ProjectAnalysis, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    discover_files_effect(root)
        .and_then(|files| {
            // Use stillwater's traverse for sequencing effects
            traverse_effect(files, analyze_file_effect)
        })
        .and_then(|analyses| {
            aggregate_analyses_effect(analyses)
        })
}

fn discover_files_effect<Env>(
    root: PathBuf,
) -> impl Effect<Output = Vec<PathBuf>, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    asks(move |env: &Env| {
        let exclusions = &env.config().exclusions;
        discover_files_pure(&root, exclusions)
    })
}

fn aggregate_analyses_effect<Env>(
    analyses: Vec<FileAnalysis>,
) -> impl Effect<Output = ProjectAnalysis, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync,
{
    asks(move |env: &Env| {
        let aggregation = &env.config().aggregation;
        aggregate_analyses_pure(analyses, aggregation)
    })
}
```

## Dependencies

- **Prerequisites**: Specs 195-198 (Foundation through Effect Composition)
- **Blocks**: Enables cleaner API for future specs
- **Affected Components**:
  - All analysis modules (100+ functions)
  - Builders and pipelines
  - Scoring and priority calculation

## Testing Strategy

- **Unit Tests**: Verify config access via `asks()`
- **Override Tests**: Test `Effect::local()` behavior
- **Integration Tests**: Full pipeline with custom config
- **Backwards Compat**: Verify public API unchanged

## Documentation Requirements

- **Code Docs**: Explain Reader pattern benefits
- **Examples**: Show common usage patterns
- **Migration Guide**: How to convert functions

## Implementation Notes

### Stillwater 0.11.0 API Summary

| Operation | Stillwater Function | Import |
|-----------|-------------------|--------|
| Query env | `asks(\|env\| ...)` | `stillwater::asks` |
| Get full env | `ask::<E, Env>()` | `stillwater::ask` |
| Modify env | `local(f, effect)` | `stillwater::local` |
| Chain effects | `.and_then(\|x\| ...)` | `stillwater::EffectExt` |
| Map results | `.map(\|x\| ...)` | `stillwater::EffectExt` |

All Reader primitives are **zero-cost** (no heap allocation).

### Benefits of Reader Pattern

**Before:**
- 200+ functions take config parameter
- Unclear which config fields used
- Hard to add new config fields
- Testing requires full config

**After:**
- 0 config parameters in internal functions
- Clear config dependencies in types
- Easy to add new fields
- Testing with minimal config

### Files to Create
- `src/env.rs` - Define `AnalysisEnv` trait

### Files to Modify
- `src/analysis/**/*.rs` - Remove config params, use `asks()`
- `src/scoring/**/*.rs` - Use `asks()`
- `src/builders/**/*.rs` - Reader pattern

## Success Metrics

- **Parameter Reduction**: 75% fewer config parameters
- **Type Safety**: All config access type-safe
- **Testability**: Easier to mock config
- **Maintainability**: Clear config dependencies

## Migration and Compatibility

### Non-Breaking
- Public API unchanged
- Internal refactoring only
- Gradual migration possible

### Migration Priority
1. Core analysis functions
2. Scoring and priority
3. Utility functions
4. Edge cases
