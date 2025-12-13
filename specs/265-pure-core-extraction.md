---
number: 265
title: Pure Core Extraction - unified_analysis.rs
category: foundation
priority: high
status: draft
dependencies: [262]
created: 2025-12-13
---

# Specification 265: Pure Core Extraction - unified_analysis.rs

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: 262 (Effects-Based Progress System)

## Context

`unified_analysis.rs` (2,093 lines) is the heart of debtmap's analysis pipeline. Currently it violates the Stillwater "Pure Core, Imperative Shell" principle by mixing:

1. **Pure computation** - Call graph building, metric calculation, scoring
2. **Progress reporting** - 47 calls to `ProgressManager::global()`
3. **Environment access** - `std::env::var("DEBTMAP_*")` throughout
4. **I/O operations** - File reading, coverage loading

**Current Problems:**

```rust
// Lines 200-600+ in perform_unified_analysis_computation()
fn perform_unified_analysis_computation(...) -> Result<UnifiedAnalysis> {
    // Environment variable access mixed in
    let enable_purity = std::env::var("DEBTMAP_ENABLE_PURITY").is_ok();

    // Progress reporting throughout
    ProgressManager::global().map(|p| p.start_stage("Building call graph"));

    // Pure computation
    let call_graph = build_call_graph(&files);  // This should be pure!

    // More progress
    ProgressManager::global().map(|p| p.complete_stage("Building call graph"));

    // ... 500+ more lines of interleaved concerns
}
```

**Stillwater Philosophy:**

> "Push effects to the edges, keep the core pure." Analysis logic should be independent of how progress is reported.

## Objective

Decompose `unified_analysis.rs` into:

1. **Pure computation modules** - No I/O, no progress, easily testable
2. **Orchestration layer** - Effects-based composition with progress
3. **Options builder** - Type-safe configuration construction

Result: Core analysis becomes pure functions composed with effects for I/O.

## Requirements

### Functional Requirements

1. **Pure Call Graph Module**
   - `compute_call_graph(files: &[FileData], config: &CallGraphConfig) -> CallGraph`
   - No I/O, no progress reporting
   - Pure transformation of AST data to call graph

2. **Pure File Analysis Module**
   - `analyze_file_metrics(file: &FileData, graph: &CallGraph) -> FileMetrics`
   - `analyze_files_batch(files: &[FileData], graph: &CallGraph) -> Vec<FileMetrics>`
   - Pure metric computation

3. **Pure Scoring Module**
   - `calculate_complexity_scores(metrics: &[FileMetrics], weights: &Weights) -> Vec<Score>`
   - `prioritize_debt_items(scores: Vec<Score>, config: &PriorityConfig) -> Vec<PrioritizedDebt>`
   - Pure scoring algorithms

4. **Pure God Object Detection**
   - `detect_god_objects(metrics: &[FileMetrics], thresholds: &Thresholds) -> Vec<GodObject>`
   - Pure pattern detection

5. **Effects-Based Orchestration**
   - Use `with_stage` from spec 262 for progress
   - Use `asks_config` for configuration access
   - Compose pure functions with effect combinators

6. **Options Builder**
   - Replace 47-field struct with builder pattern
   - Type-safe construction with defaults
   - Validation at build time

### Non-Functional Requirements

1. **Testability**
   - Pure functions testable without mocking I/O
   - Each module independently testable
   - Property-based tests for pure functions

2. **Performance**
   - No regression from current implementation
   - Parallel analysis preserved via `par_traverse_with_progress`

3. **Maintainability**
   - Each file under 400 lines
   - Single responsibility per module

## Acceptance Criteria

- [ ] `compute_call_graph` is a pure function with no side effects
- [ ] `analyze_file_metrics` is a pure function
- [ ] `calculate_complexity_scores` is a pure function
- [ ] Zero direct `ProgressManager::global()` calls in pure modules
- [ ] Zero `std::env::var()` calls outside config initialization
- [ ] Orchestration uses effects from spec 262
- [ ] All existing tests pass
- [ ] Unit tests added for each pure function
- [ ] No clippy warnings

## Technical Details

### Target Module Structure

```
src/builders/
├── unified_analysis.rs           (~200 lines) - Public API, re-exports
├── unified_analysis/
│   ├── mod.rs                    (~50 lines)  - Module exports
│   ├── options.rs                (~200 lines) - UnifiedAnalysisOptions builder
│   ├── phases/
│   │   ├── mod.rs                (~30 lines)  - Phase exports
│   │   ├── call_graph.rs         (~300 lines) - Pure call graph computation
│   │   ├── file_analysis.rs      (~400 lines) - Pure file analysis
│   │   ├── god_object.rs         (~200 lines) - Pure god object detection
│   │   ├── scoring.rs            (~250 lines) - Pure scoring
│   │   └── coverage.rs           (~150 lines) - Coverage loading (I/O)
│   └── orchestration.rs          (~300 lines) - Effects-based orchestration
```

### Implementation Approach

**Phase 1: Extract Pure Call Graph**

```rust
// src/builders/unified_analysis/phases/call_graph.rs

/// Pure call graph computation - no I/O, no progress
pub fn compute_call_graph(
    files: &[FileData],
    config: &CallGraphConfig,
) -> CallGraph {
    let mut graph = CallGraph::new();

    for file in files {
        let functions = extract_functions(file);
        for func in functions {
            graph.add_node(func.id.clone(), func.clone());
            for callee in &func.callees {
                graph.add_edge(func.id.clone(), callee.clone());
            }
        }
    }

    if config.compute_transitive {
        graph.compute_transitive_closure();
    }

    graph
}

/// Pure function to extract call relationships
fn extract_functions(file: &FileData) -> Vec<FunctionInfo> {
    // Pure transformation
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_empty_files_produces_empty_graph() {
        let graph = compute_call_graph(&[], &CallGraphConfig::default());
        assert!(graph.is_empty());
    }

    proptest! {
        #[test]
        fn graph_node_count_equals_function_count(files: Vec<FileData>) {
            let graph = compute_call_graph(&files, &CallGraphConfig::default());
            let total_functions: usize = files.iter()
                .map(|f| f.functions.len())
                .sum();
            prop_assert_eq!(graph.node_count(), total_functions);
        }
    }
}
```

**Phase 2: Extract Pure Scoring**

```rust
// src/builders/unified_analysis/phases/scoring.rs

/// Pure complexity scoring - no I/O
pub fn calculate_complexity_scores(
    metrics: &[FileMetrics],
    weights: &ScoringWeights,
) -> Vec<ComplexityScore> {
    metrics
        .iter()
        .map(|m| score_file(m, weights))
        .collect()
}

/// Pure single file scoring
fn score_file(metrics: &FileMetrics, weights: &ScoringWeights) -> ComplexityScore {
    let cyclomatic = metrics.cyclomatic_complexity as f64 * weights.cyclomatic;
    let cognitive = metrics.cognitive_complexity as f64 * weights.cognitive;
    let coupling = metrics.coupling_score as f64 * weights.coupling;

    ComplexityScore {
        file: metrics.file.clone(),
        total: cyclomatic + cognitive + coupling,
        breakdown: ScoreBreakdown {
            cyclomatic,
            cognitive,
            coupling,
        },
    }
}

/// Pure debt prioritization
pub fn prioritize_debt_items(
    scores: Vec<ComplexityScore>,
    config: &PriorityConfig,
) -> Vec<PrioritizedDebt> {
    let mut items: Vec<_> = scores
        .into_iter()
        .filter(|s| s.total >= config.threshold)
        .map(|s| PrioritizedDebt {
            score: s,
            priority: compute_priority(&s, config),
        })
        .collect();

    items.sort_by(|a, b| b.priority.cmp(&a.priority));
    items
}
```

**Phase 3: Effects-Based Orchestration**

```rust
// src/builders/unified_analysis/orchestration.rs

use crate::effects::{AnalysisEffect, asks_config};
use crate::effects::progress::{with_stage, traverse_with_progress, par_traverse_with_progress};
use super::phases::{call_graph, file_analysis, god_object, scoring};

/// Main analysis pipeline using effects
pub fn analyze_codebase(
    files: Vec<PathBuf>,
) -> AnalysisEffect<UnifiedAnalysis> {
    // Load file data (I/O)
    with_stage("Loading Files",
        par_traverse_with_progress(files.clone(), "File Loading", load_file_data)
    )
    .and_then(|file_data| {
        asks_config(move |config| {
            // Build call graph (pure)
            with_stage("Call Graph",
                pure(call_graph::compute_call_graph(&file_data, &config.call_graph))
            )
            .and_then(|graph| {
                // Analyze files (pure, parallel)
                with_stage("File Analysis",
                    par_traverse_with_progress(
                        file_data.clone(),
                        "Analyzing",
                        |file| pure(file_analysis::analyze_file_metrics(&file, &graph))
                    )
                )
                .map(move |metrics| (graph, metrics))
            })
        })
    })
    .and_then(|(graph, metrics)| {
        asks_config(|config| {
            // Score and prioritize (pure)
            with_stage("Scoring",
                pure(scoring::calculate_complexity_scores(&metrics, &config.scoring.weights))
            )
            .and_then(|scores| {
                with_stage("Prioritization",
                    pure(scoring::prioritize_debt_items(scores, &config.priority))
                )
            })
            .map(|debt_items| UnifiedAnalysis {
                call_graph: graph,
                metrics,
                debt_items,
            })
        })
    })
}
```

**Phase 4: Options Builder**

```rust
// src/builders/unified_analysis/options.rs

/// Builder for UnifiedAnalysisOptions
#[derive(Default)]
pub struct UnifiedAnalysisOptionsBuilder {
    root_path: Option<PathBuf>,
    language: Option<Language>,
    enable_coverage: bool,
    enable_purity: bool,
    parallel: bool,
    // ... other fields with defaults
}

impl UnifiedAnalysisOptionsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn root_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.root_path = Some(path.into());
        self
    }

    pub fn language(mut self, lang: Language) -> Self {
        self.language = Some(lang);
        self
    }

    pub fn with_coverage(mut self) -> Self {
        self.enable_coverage = true;
        self
    }

    pub fn with_purity_analysis(mut self) -> Self {
        self.enable_purity = true;
        self
    }

    pub fn parallel(mut self) -> Self {
        self.parallel = true;
        self
    }

    /// Build options, applying environment variables for any unset fields
    pub fn build(self) -> Result<UnifiedAnalysisOptions, ConfigError> {
        let root_path = self.root_path
            .ok_or(ConfigError::MissingField("root_path"))?;

        // Environment variables only read here, at the edge
        let enable_purity = self.enable_purity
            || std::env::var("DEBTMAP_ENABLE_PURITY").is_ok();

        Ok(UnifiedAnalysisOptions {
            root_path,
            language: self.language.unwrap_or(Language::Rust),
            enable_coverage: self.enable_coverage,
            enable_purity,
            parallel: self.parallel,
            // ...
        })
    }
}
```

### Migration Strategy

1. **Create module structure** - Empty files with re-exports
2. **Extract call_graph.rs** - Copy pure functions, add tests
3. **Extract scoring.rs** - Copy pure functions, add tests
4. **Extract file_analysis.rs** - Copy pure functions, add tests
5. **Extract god_object.rs** - Copy pure functions, add tests
6. **Create orchestration.rs** - New effects-based composition
7. **Create options.rs** - Builder pattern
8. **Update unified_analysis.rs** - Thin wrapper calling orchestration
9. **Remove old code** - After all tests pass

### Files to Modify

1. **Create** `src/builders/unified_analysis/mod.rs`
2. **Create** `src/builders/unified_analysis/options.rs`
3. **Create** `src/builders/unified_analysis/phases/mod.rs`
4. **Create** `src/builders/unified_analysis/phases/call_graph.rs`
5. **Create** `src/builders/unified_analysis/phases/file_analysis.rs`
6. **Create** `src/builders/unified_analysis/phases/god_object.rs`
7. **Create** `src/builders/unified_analysis/phases/scoring.rs`
8. **Create** `src/builders/unified_analysis/phases/coverage.rs`
9. **Create** `src/builders/unified_analysis/orchestration.rs`
10. **Modify** `src/builders/unified_analysis.rs` - Thin wrapper

## Dependencies

- **Prerequisites**: 262 (Effects-Based Progress System)
- **Affected Components**:
  - `src/builders/unified_analysis.rs`
  - `src/main.rs` (calls unified analysis)
  - `src/tui/mod.rs` (uses analysis results)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

Each pure function gets comprehensive unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_file_with_zero_metrics() {
        let metrics = FileMetrics::default();
        let weights = ScoringWeights::default();
        let score = score_file(&metrics, &weights);
        assert_eq!(score.total, 0.0);
    }

    #[test]
    fn test_prioritize_filters_below_threshold() {
        let scores = vec![
            ComplexityScore { total: 5.0, .. },
            ComplexityScore { total: 15.0, .. },
        ];
        let config = PriorityConfig { threshold: 10.0 };
        let items = prioritize_debt_items(scores, &config);
        assert_eq!(items.len(), 1);
    }
}
```

### Property-Based Tests

```rust
proptest! {
    #[test]
    fn scoring_is_deterministic(metrics: FileMetrics, weights: ScoringWeights) {
        let score1 = score_file(&metrics, &weights);
        let score2 = score_file(&metrics, &weights);
        prop_assert_eq!(score1, score2);
    }

    #[test]
    fn prioritization_preserves_count_above_threshold(
        scores in prop::collection::vec(any::<ComplexityScore>(), 0..100),
        threshold in 0.0f64..100.0
    ) {
        let config = PriorityConfig { threshold };
        let items = prioritize_debt_items(scores.clone(), &config);
        let expected = scores.iter().filter(|s| s.total >= threshold).count();
        prop_assert_eq!(items.len(), expected);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_full_analysis_pipeline() {
    let env = TestEnv::new()
        .with_file("test.rs", "fn main() { println!(\"hello\"); }")
        .with_silent_progress();

    let result = analyze_codebase(vec!["test.rs".into()])
        .run(&env)
        .unwrap();

    assert!(!result.metrics.is_empty());
}
```

## Documentation Requirements

### Code Documentation

- Module-level docs explaining pure vs orchestration split
- Function docs with examples for public APIs
- Architecture decision records for the decomposition

### Architecture Updates

Add to `ARCHITECTURE.md`:
- Analysis pipeline architecture
- Pure core pattern explanation
- How to add new analysis phases

## Implementation Notes

### Avoiding Breaking Changes

The public API remains unchanged:

```rust
// Old usage still works
let result = perform_unified_analysis(options)?;

// New effects-based usage available
let result = analyze_codebase(files).run(&env)?;
```

### Performance Considerations

- `par_traverse_with_progress` maintains parallel analysis
- Pure functions enable potential memoization
- No additional allocations vs current implementation

### Pitfalls to Avoid

1. **Accidental I/O in pure modules** - No `println!`, no file access
2. **Progress in pure functions** - All progress in orchestration layer
3. **Environment leakage** - `std::env::var` only in options builder

## Migration and Compatibility

### Breaking Changes

None - public API preserved via re-exports.

### Backward Compatibility

- Old `perform_unified_analysis()` calls new orchestration
- Result types unchanged
- Config types unchanged

## Success Metrics

- Pure functions have zero I/O operations
- Orchestration layer under 300 lines
- Each pure module under 400 lines
- Test coverage of pure functions above 90%
- No performance regression (benchmark before/after)
