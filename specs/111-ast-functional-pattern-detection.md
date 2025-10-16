---
number: 111
title: AST-Based Functional Pattern Detection
category: optimization
priority: medium
status: draft
dependencies: [109, 110]
created: 2025-10-16
---

# Specification 111: AST-Based Functional Pattern Detection

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 109 (Call Graph Role Classification), Spec 110 (Orchestration Score Adjustment)

## Context

**Current state**:
- Spec 109 provides call graph-based role classification (Orchestrator vs Worker)
- Spec 110 implements score adjustments based on orchestration patterns
- Existing pattern detection in `src/refactoring/patterns/functional_patterns.rs` uses name-based heuristics
- `EntropyAnalyzer` examines token distribution but doesn't analyze functional composition structure

**Key limitation**: Name-based detection (`has_functional_patterns()`) misses actual functional pipelines and can trigger on misleading function names. Cannot distinguish between:
- True functional pipelines: `data.iter().map(transform).filter(predicate).collect()`
- Imperative code with functional-sounding names: `fn map_results() { for item in items { /* mutation */ } }`

**Real-world impact**: False positives occur when well-composed functional code is flagged as complex because:
1. Iterator chains contribute to cyclomatic complexity counts
2. Pattern matching in functional pipelines increases branching metrics
3. Delegation to pure functions isn't recognized as quality orchestration

## Objective

Implement deep AST analysis to detect actual functional composition patterns in Rust code, providing high-confidence signals for orchestration quality assessment and score adjustment beyond what call graph analysis can detect.

## Requirements

### Functional Requirements

1. **Functional Pipeline Detection**:
   - Detect iterator chains: `.iter()`, `.map()`, `.filter()`, `.fold()`, `.collect()`
   - Identify method chaining depth and complexity
   - Recognize parallel iterators: `.par_iter()`, `.par_bridge()`
   - Count pipeline stages and transformations

2. **Pure Function Analysis**:
   - Detect absence of mutable state within functions
   - Identify functions with no side effects (no I/O, no global mutation)
   - Recognize immutable parameter patterns (`&self`, `&T` vs `&mut self`, `&mut T`)
   - Detect `const fn` declarations

3. **Functional Pattern Recognition**:
   - **Map/Filter/Fold patterns**: Detect transformations, filtering, aggregations
   - **Function composition**: Identify composed function calls `f(g(x))`
   - **Higher-order functions**: Detect functions taking closures or function pointers
   - **Monadic patterns**: Recognize `Result` and `Option` chaining with `?`, `.and_then()`, `.map()`

4. **Immutability Scoring**:
   - Calculate ratio of immutable bindings to total bindings
   - Detect use of immutable collections (`im` crate types)
   - Identify builder patterns with immutable updates
   - Score based on absence of `mut` keywords

5. **Composition Quality Metrics**:
   - **Pipeline depth**: Number of chained method calls
   - **Transformation purity**: Percentage of pure transformations in chain
   - **Closure complexity**: Complexity of closures vs extracted functions
   - **Pattern adherence**: Degree of functional idiom usage

### Non-Functional Requirements

- **Performance**: AST traversal overhead < 10% of total analysis time
- **Language coverage**: Rust-first, extensible to Python/TypeScript functional patterns
- **Accuracy**: Precision ≥ 90% for functional pattern detection (low false positives)
- **Minimal false negatives**: Recall ≥ 85% for common functional patterns
- **Backward compatibility**: Enhance existing pattern detection without breaking changes

## Acceptance Criteria

- [ ] Detects iterator chain patterns: `.iter().map().filter().collect()`
- [ ] Identifies parallel iterator usage: `.par_iter()`
- [ ] Recognizes pure functions (no mutable state, no side effects)
- [ ] Calculates immutability ratio for functions
- [ ] Detects higher-order function usage (closures, function pointers)
- [ ] Identifies monadic chaining: `Result`/`Option` with `?` and `.map()`
- [ ] Computes composition quality score (0.0-1.0)
- [ ] Integrates with Spec 110's `OrchestrationAdjuster` for quality multiplier
- [ ] Provides detailed pattern breakdown in analysis output
- [ ] Performance overhead < 10% for large codebases
- [ ] Tests cover edge cases: complex closures, nested pipelines, mixed patterns
- [ ] Documentation includes pattern detection examples

## Technical Details

### Implementation Approach

**Phase 1: Pipeline Detection**
1. Traverse Rust AST to find method call chains
2. Identify iterator patterns: `iter()`, `map()`, `filter()`, `fold()`, `collect()`
3. Calculate pipeline depth and stage count
4. Detect parallel iterator usage (`rayon` crate patterns)

**Phase 2: Purity Analysis**
1. Analyze function bodies for mutable bindings (`let mut`)
2. Detect side effects: I/O operations, global state mutation, unsafe blocks
3. Check parameter mutability: `&mut` vs `&` usage
4. Identify `const fn` declarations for compile-time purity

**Phase 3: Composition Metrics**
1. Calculate immutability ratio: immutable bindings / total bindings
2. Detect higher-order function patterns (closures, Fn traits)
3. Recognize monadic chaining patterns
4. Compute composition quality score

**Phase 4: Integration**
1. Extend `OrchestrationAdjuster` to use composition quality score
2. Add AST-based confidence boost to role classification
3. Include pattern details in JSON output and verbose logs

### Architecture Changes

```rust
// src/analysis/functional_composition.rs
pub struct FunctionalCompositionAnalyzer {
    pipeline_detector: PipelineDetector,
    purity_analyzer: PurityAnalyzer,
    composition_scorer: CompositionScorer,
}

impl FunctionalCompositionAnalyzer {
    pub fn analyze(&self, function: &syn::ItemFn) -> CompositionMetrics {
        let pipelines = self.pipeline_detector.detect_pipelines(function);
        let purity = self.purity_analyzer.analyze_purity(function);
        let quality = self.composition_scorer.score_composition(&pipelines, &purity);

        CompositionMetrics {
            pipelines,
            purity_score: purity.score,
            immutability_ratio: purity.immutability_ratio,
            composition_quality: quality,
        }
    }
}

// Pipeline detection
pub struct PipelineDetector;

impl PipelineDetector {
    pub fn detect_pipelines(&self, function: &syn::ItemFn) -> Vec<Pipeline> {
        let mut visitor = PipelineVisitor::new();
        visitor.visit_item_fn(function);
        visitor.pipelines
    }
}

pub struct Pipeline {
    pub stages: Vec<PipelineStage>,
    pub depth: usize,
    pub is_parallel: bool,
    pub terminal_operation: Option<TerminalOp>,
}

pub enum PipelineStage {
    Iterator { method: String },              // .iter(), .into_iter()
    Map { closure_complexity: u32 },          // .map(|x| ...)
    Filter { closure_complexity: u32 },       // .filter(|x| ...)
    Fold { init_complexity: u32, fold_complexity: u32 },  // .fold(init, |acc, x| ...)
    FlatMap { closure_complexity: u32 },      // .flat_map(|x| ...)
    Inspect { closure_complexity: u32 },      // .inspect(|x| ...)
}

pub enum TerminalOp {
    Collect,
    Sum,
    Count,
    Any,
    All,
    Find,
    Reduce,
}

// Purity analysis
pub struct PurityAnalyzer;

impl PurityAnalyzer {
    pub fn analyze_purity(&self, function: &syn::ItemFn) -> PurityMetrics {
        let mut visitor = PurityVisitor::new();
        visitor.visit_item_fn(function);

        PurityMetrics {
            has_mutable_state: visitor.mutable_bindings > 0,
            has_side_effects: visitor.has_io || visitor.has_global_mutation,
            immutability_ratio: visitor.immutable_bindings as f64
                              / (visitor.immutable_bindings + visitor.mutable_bindings) as f64,
            is_const_fn: function.sig.constness.is_some(),
            score: Self::calculate_purity_score(&visitor),
        }
    }

    fn calculate_purity_score(visitor: &PurityVisitor) -> f64 {
        let mut score = 1.0;
        if visitor.has_io { score -= 0.4; }
        if visitor.has_global_mutation { score -= 0.3; }
        if visitor.mutable_bindings > 0 {
            score -= 0.3 * (visitor.mutable_bindings as f64 /
                           (visitor.mutable_bindings + visitor.immutable_bindings) as f64);
        }
        score.max(0.0)
    }
}

pub struct PurityMetrics {
    pub has_mutable_state: bool,
    pub has_side_effects: bool,
    pub immutability_ratio: f64,
    pub is_const_fn: bool,
    pub score: f64,  // 0.0 (impure) to 1.0 (pure)
}

struct PurityVisitor {
    mutable_bindings: usize,
    immutable_bindings: usize,
    has_io: bool,
    has_global_mutation: bool,
}

// Composition scoring
pub struct CompositionScorer;

impl CompositionScorer {
    pub fn score_composition(&self, pipelines: &[Pipeline], purity: &PurityMetrics) -> f64 {
        let pipeline_score = self.score_pipelines(pipelines);
        let purity_weight = 0.4;
        let pipeline_weight = 0.6;

        (purity.score * purity_weight) + (pipeline_score * pipeline_weight)
    }

    fn score_pipelines(&self, pipelines: &[Pipeline]) -> f64 {
        if pipelines.is_empty() {
            return 0.0;
        }

        let total_score: f64 = pipelines.iter()
            .map(|p| self.score_single_pipeline(p))
            .sum();

        (total_score / pipelines.len() as f64).min(1.0)
    }

    fn score_single_pipeline(&self, pipeline: &Pipeline) -> f64 {
        let base_score = 0.5;
        let depth_bonus = (pipeline.depth as f64 * 0.1).min(0.3);
        let parallel_bonus = if pipeline.is_parallel { 0.2 } else { 0.0 };

        // Penalize complex closures
        let avg_closure_complexity = self.average_closure_complexity(pipeline);
        let complexity_penalty = (avg_closure_complexity as f64 * 0.05).min(0.3);

        (base_score + depth_bonus + parallel_bonus - complexity_penalty).clamp(0.0, 1.0)
    }

    fn average_closure_complexity(&self, pipeline: &Pipeline) -> f64 {
        let complexities: Vec<u32> = pipeline.stages.iter()
            .filter_map(|stage| match stage {
                PipelineStage::Map { closure_complexity } => Some(*closure_complexity),
                PipelineStage::Filter { closure_complexity } => Some(*closure_complexity),
                PipelineStage::FlatMap { closure_complexity } => Some(*closure_complexity),
                PipelineStage::Fold { fold_complexity, .. } => Some(*fold_complexity),
                _ => None,
            })
            .collect();

        if complexities.is_empty() {
            return 0.0;
        }

        complexities.iter().sum::<u32>() as f64 / complexities.len() as f64
    }
}

// Final composition metrics
pub struct CompositionMetrics {
    pub pipelines: Vec<Pipeline>,
    pub purity_score: f64,
    pub immutability_ratio: f64,
    pub composition_quality: f64,  // 0.0 to 1.0
}
```

### Data Structures

```rust
// Add to FunctionMetrics
pub struct FunctionMetrics {
    // ... existing fields ...
    pub composition_metrics: Option<CompositionMetrics>,
}

// Add to AnalysisConfig
pub struct AnalysisConfig {
    // ... existing fields ...
    pub enable_ast_functional_analysis: bool,
    pub functional_analysis_config: FunctionalAnalysisConfig,
}

pub struct FunctionalAnalysisConfig {
    pub min_pipeline_depth: usize,          // Minimum chain length to consider (default: 2)
    pub max_closure_complexity: u32,        // Max acceptable closure complexity (default: 5)
    pub purity_threshold: f64,              // Minimum purity score for "pure" label (default: 0.8)
    pub composition_quality_threshold: f64, // Minimum quality for score boost (default: 0.6)
}

impl Default for FunctionalAnalysisConfig {
    fn default() -> Self {
        Self {
            min_pipeline_depth: 2,
            max_closure_complexity: 5,
            purity_threshold: 0.8,
            composition_quality_threshold: 0.6,
        }
    }
}
```

### APIs and Interfaces

```rust
// Integration with OrchestrationAdjuster (from Spec 110)
impl OrchestrationAdjuster {
    fn calculate_composition_quality(&self, metrics: &RoleMetrics,
                                     function_metrics: &FunctionMetrics) -> f64 {
        // Use AST-based composition metrics if available
        if let Some(comp_metrics) = &function_metrics.composition_metrics {
            let base_quality = self.calculate_base_composition_quality(metrics);
            let ast_boost = comp_metrics.composition_quality * 0.3; // Up to 30% boost
            (base_quality + ast_boost).min(1.0)
        } else {
            // Fallback to call graph-based calculation
            self.calculate_base_composition_quality(metrics)
        }
    }
}

// CLI flag
Commands::Analyze {
    // ... existing fields ...

    /// Enable deep AST-based functional pattern analysis
    #[arg(long = "ast-functional-analysis")]
    ast_functional_analysis: bool,
}

// Config file
[analysis]
enable_ast_functional_analysis = true

[analysis.functional_analysis]
min_pipeline_depth = 2
max_closure_complexity = 5
purity_threshold = 0.8
composition_quality_threshold = 0.6
```

### Integration Points

1. **Rust analyzer** (`src/analyzers/rust_analyzer.rs`):
   - Run `FunctionalCompositionAnalyzer` during AST traversal
   - Attach `CompositionMetrics` to `FunctionMetrics`

2. **Orchestration adjuster** (`src/analysis/orchestration_adjuster.rs` from Spec 110):
   - Use `composition_quality` to boost quality multiplier
   - Increase confidence when AST analysis confirms functional patterns

3. **Pattern detection** (`src/refactoring/patterns/`):
   - Replace name-based heuristics with AST-based detection
   - Provide detailed pattern breakdown in output

4. **JSON output** (`src/io/output/json.rs`):
   - Include `composition_metrics` in function-level output
   - Add summary statistics for functional patterns

## Dependencies

- **Prerequisites**: Spec 109 (Call Graph Role Classification), Spec 110 (Orchestration Score Adjustment)
- **Affected Components**:
  - `src/analyzers/rust_analyzer.rs` - AST traversal integration
  - `src/analysis/orchestration_adjuster.rs` - Quality multiplier calculation
  - `src/refactoring/patterns/functional_patterns.rs` - Pattern detection upgrade
  - `src/config.rs` - Configuration schema
- **External Dependencies**:
  - `syn` crate (already used) - AST parsing and traversal
  - `quote` crate (optional) - Code generation for testing

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_detect_simple_iterator_chain() {
        let function: syn::ItemFn = parse_quote! {
            fn process_items(items: Vec<i32>) -> Vec<i32> {
                items.iter()
                    .map(|x| x * 2)
                    .filter(|x| x > &10)
                    .collect()
            }
        };

        let detector = PipelineDetector;
        let pipelines = detector.detect_pipelines(&function);

        assert_eq!(pipelines.len(), 1);
        assert_eq!(pipelines[0].depth, 4);
        assert_eq!(pipelines[0].stages.len(), 4);
        assert!(matches!(pipelines[0].stages[0], PipelineStage::Iterator { .. }));
        assert!(matches!(pipelines[0].stages[1], PipelineStage::Map { .. }));
        assert!(matches!(pipelines[0].stages[2], PipelineStage::Filter { .. }));
    }

    #[test]
    fn test_detect_parallel_iterator() {
        let function: syn::ItemFn = parse_quote! {
            fn parallel_process(items: Vec<i32>) -> i32 {
                items.par_iter()
                    .map(|x| expensive_computation(*x))
                    .sum()
            }
        };

        let detector = PipelineDetector;
        let pipelines = detector.detect_pipelines(&function);

        assert_eq!(pipelines.len(), 1);
        assert!(pipelines[0].is_parallel);
    }

    #[test]
    fn test_purity_analysis_pure_function() {
        let function: syn::ItemFn = parse_quote! {
            fn pure_calculation(x: i32, y: i32) -> i32 {
                let sum = x + y;
                let product = x * y;
                sum + product
            }
        };

        let analyzer = PurityAnalyzer;
        let metrics = analyzer.analyze_purity(&function);

        assert!(!metrics.has_mutable_state);
        assert!(!metrics.has_side_effects);
        assert_eq!(metrics.immutability_ratio, 1.0);
        assert!(metrics.score > 0.9);
    }

    #[test]
    fn test_purity_analysis_impure_function() {
        let function: syn::ItemFn = parse_quote! {
            fn impure_function(x: i32) -> i32 {
                let mut counter = 0;
                counter += x;
                println!("Counter: {}", counter);
                counter
            }
        };

        let analyzer = PurityAnalyzer;
        let metrics = analyzer.analyze_purity(&function);

        assert!(metrics.has_mutable_state);
        assert!(metrics.has_side_effects);
        assert!(metrics.score < 0.6);
    }

    #[test]
    fn test_composition_scoring_high_quality() {
        let pipeline = Pipeline {
            stages: vec![
                PipelineStage::Iterator { method: "iter".to_string() },
                PipelineStage::Map { closure_complexity: 2 },
                PipelineStage::Filter { closure_complexity: 1 },
            ],
            depth: 3,
            is_parallel: false,
            terminal_operation: Some(TerminalOp::Collect),
        };

        let purity = PurityMetrics {
            has_mutable_state: false,
            has_side_effects: false,
            immutability_ratio: 1.0,
            is_const_fn: false,
            score: 1.0,
        };

        let scorer = CompositionScorer;
        let quality = scorer.score_composition(&[pipeline], &purity);

        assert!(quality > 0.7);
    }

    #[test]
    fn test_complex_closure_penalty() {
        let pipeline = Pipeline {
            stages: vec![
                PipelineStage::Iterator { method: "iter".to_string() },
                PipelineStage::Map { closure_complexity: 15 },  // Complex closure
            ],
            depth: 2,
            is_parallel: false,
            terminal_operation: Some(TerminalOp::Collect),
        };

        let purity = PurityMetrics {
            has_mutable_state: false,
            has_side_effects: false,
            immutability_ratio: 1.0,
            is_const_fn: false,
            score: 1.0,
        };

        let scorer = CompositionScorer;
        let quality = scorer.score_composition(&[pipeline], &purity);

        // Should have penalty for complex closure
        assert!(quality < 0.6);
    }
}
```

### Integration Tests

1. **Full analysis with AST patterns**:
   - Analyze Rust project with functional code
   - Verify `composition_metrics` populated in JSON output
   - Confirm score adjustments for high-quality functional code

2. **Orchestration quality boost**:
   - Compare scores with/without `--ast-functional-analysis`
   - Verify well-composed orchestrators get higher quality multipliers
   - Ensure imperative code doesn't get false boosts

3. **Pattern detection accuracy**:
   - Create test corpus with known functional patterns
   - Measure precision and recall (target: ≥90% precision, ≥85% recall)
   - Verify edge cases: nested pipelines, complex closures, mixed patterns

4. **Performance test**:
   - Analyze large codebase with AST analysis enabled/disabled
   - Measure overhead (should be < 10%)
   - Profile hot paths for optimization opportunities

## Documentation Requirements

### Code Documentation

- Document `FunctionalCompositionAnalyzer` API with examples
- Explain pipeline detection algorithm and limitations
- Document purity analysis heuristics (what counts as side effect)
- Provide examples of detected patterns in doctests

### User Documentation

Add to debtmap user guide:

```markdown
## AST-Based Functional Pattern Detection

### Enabling AST Analysis

Enable deep functional pattern analysis:
```bash
debtmap analyze src --ast-functional-analysis
```

Or configure in `.debtmap.toml`:
```toml
[analysis]
enable_ast_functional_analysis = true

[analysis.functional_analysis]
min_pipeline_depth = 2
max_closure_complexity = 5
purity_threshold = 0.8
composition_quality_threshold = 0.6
```

### Detected Patterns

**Iterator pipelines**:
- Method chains: `.iter().map().filter().collect()`
- Parallel iterators: `.par_iter()`
- Fold operations: `.fold(init, |acc, x| ...)`

**Purity analysis**:
- Functions without mutable state
- Functions without side effects (I/O, global mutation)
- Immutability ratio calculation

**Composition quality**:
- Pipeline depth and complexity
- Closure complexity within pipelines
- Overall functional composition score

### Output Format

JSON output includes composition metrics:
```json
{
  "function": "process_data",
  "composition_metrics": {
    "pipelines": [
      {
        "depth": 4,
        "is_parallel": false,
        "stages": ["Iterator", "Map", "Filter", "Collect"]
      }
    ],
    "purity_score": 0.95,
    "immutability_ratio": 1.0,
    "composition_quality": 0.82
  }
}
```

### Tuning Analysis

Adjust thresholds based on your codebase:
- `min_pipeline_depth`: Minimum chain length (default: 2)
- `max_closure_complexity`: Acceptable closure complexity (default: 5)
- `purity_threshold`: Minimum score for "pure" label (default: 0.8)
- `composition_quality_threshold`: Minimum quality for score boost (default: 0.6)
```

### Architecture Documentation

Update ARCHITECTURE.md:
- Explain AST-based functional analysis architecture
- Document integration with orchestration score adjustment
- Describe pipeline detection and purity analysis algorithms
- Provide examples of composition quality calculation

## Implementation Notes

### AST Traversal Strategy

1. **Use syn::visit pattern**: Implement `Visit` trait for efficient AST traversal
2. **Cache parsed ASTs**: Reuse parsed syntax trees from Rust analyzer
3. **Parallel analysis**: Process functions in parallel using rayon
4. **Early termination**: Skip analysis for low-complexity functions (< 5 complexity)

### Pattern Detection Edge Cases

1. **Complex closures**: Closures with high complexity should reduce pipeline quality score
2. **Nested pipelines**: Detect pipelines within closures (recursive detection)
3. **Mixed imperative/functional**: Score based on ratio of functional vs imperative patterns
4. **External iterator adapters**: Recognize custom iterator traits beyond std library

### Purity Analysis Challenges

1. **Side effect detection**:
   - I/O operations: `println!`, `fs::write`, network calls
   - Global state: `static mut`, `thread_local!` modification
   - Unsafe code: Assume impure unless proven otherwise

2. **Conservative approach**:
   - Default to impure when uncertain
   - Provide escape hatch for manual overrides via annotations

3. **Closure capture analysis**:
   - Detect mutable captures: `move` closures modifying captured state
   - Track variable lifetimes through closure boundaries

### Performance Optimization

1. **Lazy evaluation**: Only run AST analysis when function complexity exceeds threshold
2. **Caching**: Cache pipeline detection results per function
3. **Parallel processing**: Use rayon for independent function analysis
4. **Early bailout**: Skip detailed analysis if function is clearly imperative

### Integration with Existing Systems

1. **Spec 110 integration**:
   ```rust
   // In OrchestrationAdjuster::calculate_composition_quality()
   if let Some(comp_metrics) = &function_metrics.composition_metrics {
       if comp_metrics.composition_quality >= 0.6 {
           base_quality += 0.3 * comp_metrics.composition_quality;
       }
   }
   ```

2. **Confidence boosting**:
   ```rust
   // High composition quality increases confidence
   if comp_metrics.composition_quality >= 0.7 {
       confidence += 0.1;  // Boost confidence by 10%
   }
   ```

## Migration and Compatibility

### Backward Compatibility

- **Opt-in feature**: Disabled by default, users must enable with flag or config
- **No breaking changes**: Existing analysis output unchanged when disabled
- **Graceful degradation**: Falls back to call graph analysis if AST analysis fails

### Migration Path

For users wanting higher accuracy orchestration detection:

1. **Enable AST analysis**: Add `--ast-functional-analysis` flag to commands
2. **Review score changes**: Compare before/after to validate adjustments
3. **Tune thresholds**: Adjust `functional_analysis_config` based on codebase patterns
4. **Incremental adoption**: Enable for specific modules first, expand gradually

### Performance Considerations

- **Large codebases**: May want to disable for initial scans, enable for focused analysis
- **CI/CD pipelines**: Consider caching AST analysis results between runs
- **Incremental analysis**: Only re-analyze changed functions

## Future Enhancements

1. **Multi-language support**:
   - Python: Detect list comprehensions, generator expressions, `map`/`filter` built-ins
   - TypeScript: Detect array methods (`.map()`, `.filter()`, `.reduce()`)
   - JavaScript: Functional libraries (Lodash, Ramda)

2. **Advanced patterns**:
   - Monad transformers and effect systems
   - Lens and optics patterns
   - Free monad and tagless final encodings

3. **AI-assisted detection**:
   - ML model to learn project-specific functional patterns
   - Suggest refactorings to improve composition quality

4. **Composition visualization**:
   - Generate call graphs colored by composition quality
   - Interactive pipeline explorer in HTML output

5. **Quality recommendations**:
   - Suggest extracting complex closures to named functions
   - Recommend parallel iteration for suitable workloads

## Success Metrics

- **Accuracy**: Precision ≥ 90%, Recall ≥ 85% for functional pattern detection
- **Performance**: Overhead < 10% with AST analysis enabled
- **Adoption**: 40% of users enable AST analysis within 6 months
- **False positive reduction**: 30% reduction in orchestrator false positives
- **User satisfaction**: < 2 bug reports per month on pattern detection accuracy

## Open Questions

1. **Closure complexity threshold**: Should complex closures (> 5 complexity) always penalize pipeline quality, or are some patterns acceptable?
2. **Parallel iterator scoring**: Should parallel iterators always get bonus, or only when workload justifies parallelism?
3. **Purity edge cases**: How to handle functions with tracing/logging (side effects but harmless)?
4. **Multi-language priority**: Which language to support next after Rust?
