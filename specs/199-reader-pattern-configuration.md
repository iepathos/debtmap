---
number: 199
title: Reader Pattern for Configuration Management
category: foundation
priority: medium
status: draft
dependencies: [195, 196, 197, 198]
created: 2025-11-24
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

**After: Reader Pattern**
```rust
// No config parameters!
pub fn analyze_file_effect(
    path: PathBuf,
) -> AnalysisEffect<FileAnalysis> {
    read_file_effect(path)
        .and_then(|content| parse_effect(content))
        .and_then(|ast| calculate_metrics_effect(ast))
        .and_then(|metrics| calculate_score_effect(metrics))
        .map(|(metrics, score)| FileAnalysis { metrics, score })
}

fn calculate_metrics_effect(
    ast: Ast,
) -> AnalysisEffect<Metrics> {
    // Ask for thresholds when needed
    Effect::asks(|env: &impl AnalysisEnv| {
        let thresholds = &env.config().thresholds;
        calculate_metrics_pure(&ast, thresholds)
    })
}

fn calculate_score_effect(
    metrics: Metrics,
) -> AnalysisEffect<Score> {
    // Ask for scoring config when needed
    Effect::asks(|env: &impl AnalysisEnv| {
        let scoring = &env.config().scoring;
        calculate_score_pure(&metrics, scoring)
    })
}
```

#### 3. Temporary Config Overrides

```rust
// src/analysis/strict_mode.rs

/// Run analysis in strict mode (higher thresholds)
pub fn analyze_strict(
    path: PathBuf,
) -> AnalysisEffect<FileAnalysis> {
    Effect::local(
        |env: &impl AnalysisEnv| {
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
pub fn analyze_with_thresholds(
    path: PathBuf,
    custom_thresholds: Thresholds,
) -> AnalysisEffect<FileAnalysis> {
    Effect::local(
        move |env: &impl AnalysisEnv| {
            env.with_config(Config {
                thresholds: custom_thresholds.clone(),
                ..env.config().clone()
            })
        },
        analyze_file_effect(path)
    )
}

/// Temporarily disable specific patterns
pub fn analyze_without_patterns(
    path: PathBuf,
    disabled_patterns: Vec<String>,
) -> AnalysisEffect<FileAnalysis> {
    Effect::local(
        move |env: &impl AnalysisEnv| {
            let mut config = env.config().clone();
            config.patterns.retain(|p| !disabled_patterns.contains(p));
            env.with_config(config)
        },
        analyze_file_effect(path)
    )
}
```

#### 4. Nested Config Access

```rust
// src/scoring/effects.rs

/// Access specific config sections directly
pub fn score_with_coverage_factor() -> AnalysisEffect<f64> {
    Effect::asks(|env: &impl AnalysisEnv| {
        env.config().scoring.coverage_weight
    })
}

/// Conditional logic based on config
pub fn should_analyze_tests() -> AnalysisEffect<bool> {
    Effect::asks(|env: &impl AnalysisEnv| {
        env.config().analysis.include_tests
    })
}

/// Get threshold for specific metric
pub fn get_complexity_threshold() -> AnalysisEffect<f64> {
    Effect::asks(|env: &impl AnalysisEnv| {
        env.config().thresholds.complexity
    })
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

### Testing with Reader Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_custom_config() {
        let config = Config {
            thresholds: Thresholds {
                complexity: 5.0,
                coverage: 90.0,
                depth: 3,
            },
            ..Config::default()
        };

        let env = MockEnv::new()
            .with_config(config)
            .with_file("test.rs", "fn foo() {}");

        let result = analyze_file_effect("test.rs".into())
            .run(&env)
            .unwrap();

        // Analysis used custom thresholds
        assert!(result.score.is_within_threshold(&config.thresholds));
    }

    #[test]
    fn test_strict_mode() {
        let env = MockEnv::new()
            .with_config(Config::default())
            .with_file("test.rs", "fn complex() { /* ... */ }");

        let normal = analyze_file_effect("test.rs".into())
            .run(&env)
            .unwrap();

        let strict = analyze_strict("test.rs".into())
            .run(&env)
            .unwrap();

        // Strict mode has higher score (stricter thresholds)
        assert!(strict.score > normal.score);
    }

    #[test]
    fn test_temporary_override() {
        let env = MockEnv::new()
            .with_config(Config::default())
            .with_file("test.rs", "fn foo() {}");

        let custom_thresholds = Thresholds {
            complexity: 1.0,  // Very strict
            coverage: 100.0,
            depth: 1,
        };

        let result = analyze_with_thresholds(
            "test.rs".into(),
            custom_thresholds
        )
        .run(&env)
        .unwrap();

        // Used custom thresholds
        assert_eq!(result.thresholds_used, custom_thresholds);
    }

    #[test]
    fn test_config_field_access() {
        let config = Config {
            scoring: ScoringConfig {
                coverage_weight: 0.8,
                ..ScoringConfig::default()
            },
            ..Config::default()
        };

        let env = MockEnv::new().with_config(config);

        let weight = score_with_coverage_factor()
            .run(&env)
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

**After: Reader Pattern**
```rust
pub fn analyze_project_effect(
    root: PathBuf,
) -> AnalysisEffect<ProjectAnalysis> {
    discover_files_effect(root)
        .and_then(|files| {
            Effect::traverse(files, analyze_file_effect)
        })
        .and_then(|analyses| {
            aggregate_analyses_effect(analyses)
        })
}

fn discover_files_effect(
    root: PathBuf,
) -> AnalysisEffect<Vec<PathBuf>> {
    Effect::asks(|env: &impl AnalysisEnv| {
        let exclusions = &env.config().exclusions;
        discover_files_pure(&root, exclusions)
    })
}

fn aggregate_analyses_effect(
    analyses: Vec<FileAnalysis>,
) -> AnalysisEffect<ProjectAnalysis> {
    Effect::asks(|env: &impl AnalysisEnv| {
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

### Files to Modify
- `src/analysis/**/*.rs` - Remove config params
- `src/scoring/**/*.rs` - Use `asks()`
- `src/builders/**/*.rs` - Reader pattern

### Estimated Effort
- Convert 50 core functions: 10-12 hours
- Convert 100 secondary functions: 8-10 hours
- Update tests: 4-6 hours
- Documentation: 2-3 hours
- **Total: 24-31 hours**

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
