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

### Core Functionality
- [ ] Detects iterator chain patterns: `.iter().map().filter().collect()`
- [ ] Identifies parallel iterator usage: `.par_iter()` with context-aware bonus
- [ ] Recognizes nested pipelines within closures (e.g., `flat_map` with inner pipelines)
- [ ] Distinguishes builder patterns from functional pipelines
- [ ] Recognizes pure functions (no mutable state, no side effects)
- [ ] Calculates immutability ratio for functions
- [ ] Detects higher-order function usage (closures, function pointers)
- [ ] Identifies monadic chaining: `Result`/`Option` with `?`, `.and_then()`, `.map()`
- [ ] Classifies side effects as Pure, Benign (logging), or Impure (I/O)

### Scoring and Integration
- [ ] Computes composition quality score (0.0-1.0) using functional approach
- [ ] Context-aware closure complexity penalty based on pipeline depth
- [ ] Integrates with Spec 110's `OrchestrationAdjuster` for quality multiplier
- [ ] Provides detailed pattern breakdown in analysis output with side effect classification
- [ ] Early termination for trivial functions (< 3 complexity by default)

### Configuration
- [ ] Supports three configuration profiles: strict, balanced, lenient
- [ ] CLI flag `--functional-analysis-profile` to select profile
- [ ] Custom configuration overrides via `.debtmap.toml`
- [ ] `min_function_complexity` threshold for skipping analysis

### Testing
- [ ] Test corpus with 95 files: 65 positive, 45 negative, 10 edge cases
- [ ] Precision ≥ 90% for functional pattern detection (low false positives)
- [ ] Recall ≥ 85% for functional pattern detection (low false negatives)
- [ ] F1 Score ≥ 0.87
- [ ] Tests cover edge cases: complex closures, nested pipelines, mixed patterns, benign logging
- [ ] Performance overhead < 10% for large codebases

### Documentation
- [ ] Code documentation with examples for all public APIs
- [ ] User guide section on AST-based functional pattern detection
- [ ] Configuration profile examples and tuning guidance
- [ ] Pattern detection examples showing detected vs missed patterns
- [ ] Architecture documentation explaining functional accumulation patterns

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

/// Main entry point for functional composition analysis
/// Uses pure functions for stateless analysis
pub fn analyze_composition(function: &syn::ItemFn, config: &FunctionalAnalysisConfig) -> CompositionMetrics {
    let pipelines = detect_pipelines(function, config);
    let purity = analyze_purity(function, config);
    let quality = score_composition(&pipelines, &purity, config);

    CompositionMetrics {
        pipelines,
        purity_score: purity.score,
        immutability_ratio: purity.immutability_ratio,
        composition_quality: quality,
        side_effect_kind: purity.side_effect_kind,
    }
}

// Pipeline detection - pure function approach
pub fn detect_pipelines(function: &syn::ItemFn, config: &FunctionalAnalysisConfig) -> Vec<Pipeline> {
    collect_pipelines(&function.block, config)
        .into_iter()
        .filter(|p| p.depth >= config.min_pipeline_depth)
        .collect()
}

fn collect_pipelines(block: &syn::Block, config: &FunctionalAnalysisConfig) -> Vec<Pipeline> {
    block.stmts.iter()
        .flat_map(|stmt| extract_pipeline_from_stmt(stmt, config))
        .collect()
}

pub struct Pipeline {
    pub stages: Vec<PipelineStage>,
    pub depth: usize,
    pub is_parallel: bool,
    pub terminal_operation: Option<TerminalOp>,
    pub nesting_level: usize,  // 0 for top-level, >0 for nested pipelines
    pub builder_pattern: bool, // Distinguish builders from functional pipelines
}

pub enum PipelineStage {
    Iterator { method: String },              // .iter(), .into_iter()
    Map { closure_complexity: u32, has_nested_pipeline: bool },
    Filter { closure_complexity: u32, has_nested_pipeline: bool },
    Fold { init_complexity: u32, fold_complexity: u32 },
    FlatMap { closure_complexity: u32, has_nested_pipeline: bool },
    Inspect { closure_complexity: u32 },
    AndThen { closure_complexity: u32 },      // Result/Option chaining
    MapErr { closure_complexity: u32 },       // Error transformation
}

pub enum TerminalOp {
    Collect,
    Sum,
    Count,
    Any,
    All,
    Find,
    Reduce,
    ForEach,
}

// Purity analysis - functional accumulation pattern
pub fn analyze_purity(function: &syn::ItemFn, config: &FunctionalAnalysisConfig) -> PurityMetrics {
    let metrics = analyze_block_purity(&function.block);
    let is_const_fn = function.sig.constness.is_some();

    let immutability_ratio = if metrics.total_bindings() == 0 {
        1.0
    } else {
        metrics.immutable_bindings as f64 / metrics.total_bindings() as f64
    };

    let side_effect_kind = classify_side_effects(&metrics);
    let score = calculate_purity_score(&metrics, &side_effect_kind);

    PurityMetrics {
        has_mutable_state: metrics.mutable_bindings > 0,
        has_side_effects: matches!(side_effect_kind, SideEffectKind::Impure),
        immutability_ratio,
        is_const_fn,
        side_effect_kind,
        score,
    }
}

#[derive(Default, Clone, Debug)]
struct PurityAccumulator {
    mutable_bindings: usize,
    immutable_bindings: usize,
    io_operations: Vec<String>,
    global_mutations: Vec<String>,
    benign_side_effects: Vec<String>,  // logging, tracing, metrics
}

impl PurityAccumulator {
    fn merge(self, other: Self) -> Self {
        Self {
            mutable_bindings: self.mutable_bindings + other.mutable_bindings,
            immutable_bindings: self.immutable_bindings + other.immutable_bindings,
            io_operations: [self.io_operations, other.io_operations].concat(),
            global_mutations: [self.global_mutations, other.global_mutations].concat(),
            benign_side_effects: [self.benign_side_effects, other.benign_side_effects].concat(),
        }
    }

    fn total_bindings(&self) -> usize {
        self.mutable_bindings + self.immutable_bindings
    }
}

fn analyze_block_purity(block: &syn::Block) -> PurityAccumulator {
    block.stmts.iter()
        .map(|stmt| analyze_stmt_purity(stmt))
        .fold(PurityAccumulator::default(), |acc, metrics| acc.merge(metrics))
}

fn analyze_stmt_purity(stmt: &syn::Stmt) -> PurityAccumulator {
    match stmt {
        syn::Stmt::Local(local) => analyze_local_purity(local),
        syn::Stmt::Expr(expr, _) | syn::Stmt::Semi(expr, _) => analyze_expr_purity(expr),
        _ => PurityAccumulator::default(),
    }
}

fn analyze_local_purity(local: &syn::Local) -> PurityAccumulator {
    let mut acc = if local.mutability.is_some() {
        PurityAccumulator { mutable_bindings: 1, ..Default::default() }
    } else {
        PurityAccumulator { immutable_bindings: 1, ..Default::default() }
    };

    if let Some(init) = &local.init {
        acc = acc.merge(analyze_expr_purity(&init.expr));
    }

    acc
}

fn analyze_expr_purity(expr: &syn::Expr) -> PurityAccumulator {
    match expr {
        syn::Expr::Macro(mac) => classify_macro_side_effect(&mac.mac),
        syn::Expr::Block(block) => analyze_block_purity(&block.block),
        syn::Expr::If(if_expr) => {
            let then_branch = analyze_block_purity(&if_expr.then_branch);
            let else_branch = if_expr.else_branch.as_ref()
                .map(|(_, expr)| analyze_expr_purity(expr))
                .unwrap_or_default();
            then_branch.merge(else_branch)
        }
        syn::Expr::Match(match_expr) => {
            match_expr.arms.iter()
                .map(|arm| analyze_expr_purity(&arm.body))
                .fold(PurityAccumulator::default(), |acc, metrics| acc.merge(metrics))
        }
        syn::Expr::Call(call) => analyze_call_purity(call),
        syn::Expr::MethodCall(method) => analyze_method_call_purity(method),
        _ => PurityAccumulator::default(),
    }
}

fn classify_macro_side_effect(mac: &syn::Macro) -> PurityAccumulator {
    let path = mac.path.segments.last().map(|s| s.ident.to_string());

    match path.as_deref() {
        Some("println") | Some("eprintln") | Some("print") | Some("eprint") => {
            PurityAccumulator {
                io_operations: vec!["console_output".to_string()],
                ..Default::default()
            }
        }
        Some("debug") | Some("info") | Some("warn") | Some("error") | Some("trace") => {
            // logging macros (tracing crate)
            PurityAccumulator {
                benign_side_effects: vec![format!("logging::{}", path.unwrap())],
                ..Default::default()
            }
        }
        Some("log") => {
            // log crate
            PurityAccumulator {
                benign_side_effects: vec!["logging::log".to_string()],
                ..Default::default()
            }
        }
        _ => PurityAccumulator::default(),
    }
}

fn analyze_call_purity(_call: &syn::ExprCall) -> PurityAccumulator {
    // Conservative: assume function calls might have side effects
    // TODO: Implement function signature analysis for known pure functions
    PurityAccumulator::default()
}

fn analyze_method_call_purity(method: &syn::ExprMethodCall) -> PurityAccumulator {
    let method_name = method.method.to_string();

    // Detect known mutating methods
    if method_name.starts_with("push") || method_name.starts_with("insert") ||
       method_name.starts_with("remove") || method_name.starts_with("clear") {
        PurityAccumulator {
            global_mutations: vec![format!("mutation::{}", method_name)],
            ..Default::default()
        }
    } else {
        PurityAccumulator::default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SideEffectKind {
    Pure,          // No side effects
    Benign,        // Only logging/tracing/metrics
    Impure,        // I/O, mutation, network, etc.
}

fn classify_side_effects(acc: &PurityAccumulator) -> SideEffectKind {
    if !acc.io_operations.is_empty() || !acc.global_mutations.is_empty() {
        SideEffectKind::Impure
    } else if !acc.benign_side_effects.is_empty() {
        SideEffectKind::Benign
    } else {
        SideEffectKind::Pure
    }
}

fn calculate_purity_score(acc: &PurityAccumulator, side_effect_kind: &SideEffectKind) -> f64 {
    let mut score = 1.0;

    // Side effect penalties
    match side_effect_kind {
        SideEffectKind::Pure => {},
        SideEffectKind::Benign => score -= 0.1,  // Small penalty for logging
        SideEffectKind::Impure => {
            if !acc.io_operations.is_empty() {
                score -= 0.4;
            }
            if !acc.global_mutations.is_empty() {
                score -= 0.3;
            }
        }
    }

    // Mutability penalty
    if acc.mutable_bindings > 0 && acc.total_bindings() > 0 {
        let mutability_ratio = acc.mutable_bindings as f64 / acc.total_bindings() as f64;
        score -= 0.3 * mutability_ratio;
    }

    score.max(0.0)
}

pub struct PurityMetrics {
    pub has_mutable_state: bool,
    pub has_side_effects: bool,
    pub immutability_ratio: f64,
    pub is_const_fn: bool,
    pub side_effect_kind: SideEffectKind,
    pub score: f64,  // 0.0 (impure) to 1.0 (pure)
}

// Composition scoring - pure functional approach
pub fn score_composition(
    pipelines: &[Pipeline],
    purity: &PurityMetrics,
    config: &FunctionalAnalysisConfig,
) -> f64 {
    if pipelines.is_empty() {
        return purity.score * 0.5;  // No pipelines, but may be pure
    }

    let pipeline_score = score_pipelines(pipelines, config);
    let purity_weight = 0.4;
    let pipeline_weight = 0.6;

    (purity.score * purity_weight) + (pipeline_score * pipeline_weight)
}

fn score_pipelines(pipelines: &[Pipeline], config: &FunctionalAnalysisConfig) -> f64 {
    // Filter out builder patterns from functional pipeline scoring
    let functional_pipelines: Vec<&Pipeline> = pipelines.iter()
        .filter(|p| !p.builder_pattern)
        .collect();

    if functional_pipelines.is_empty() {
        return 0.0;
    }

    let total_score: f64 = functional_pipelines.iter()
        .map(|p| score_single_pipeline(p, config))
        .sum();

    (total_score / functional_pipelines.len() as f64).min(1.0)
}

fn score_single_pipeline(pipeline: &Pipeline, config: &FunctionalAnalysisConfig) -> f64 {
    let base_score = 0.5;
    let depth_bonus = (pipeline.depth as f64 * 0.1).min(0.3);
    let parallel_bonus = calculate_parallel_bonus(pipeline);
    let complexity_penalty = calculate_closure_penalty(pipeline, config);
    let nesting_bonus = if pipeline.nesting_level > 0 { 0.1 } else { 0.0 };

    (base_score + depth_bonus + parallel_bonus + nesting_bonus - complexity_penalty)
        .clamp(0.0, 1.0)
}

fn calculate_parallel_bonus(pipeline: &Pipeline) -> f64 {
    // Only award parallel bonus for pipelines with sufficient depth
    // to justify parallelization overhead
    if pipeline.is_parallel && pipeline.depth >= 3 {
        0.2
    } else {
        0.0
    }
}

fn calculate_closure_penalty(pipeline: &Pipeline, config: &FunctionalAnalysisConfig) -> f64 {
    let complexities: Vec<u32> = pipeline.stages.iter()
        .filter_map(|stage| extract_closure_complexity(stage))
        .collect();

    if complexities.is_empty() {
        return 0.0;
    }

    let avg_complexity = complexities.iter().sum::<u32>() as f64 / complexities.len() as f64;
    let expected_complexity = (pipeline.depth as u32 * 2).min(config.max_closure_complexity);

    // Penalty based on how much closure complexity exceeds expectations
    if avg_complexity > expected_complexity as f64 {
        ((avg_complexity - expected_complexity as f64) * 0.05).min(0.3)
    } else {
        0.0
    }
}

fn extract_closure_complexity(stage: &PipelineStage) -> Option<u32> {
    match stage {
        PipelineStage::Map { closure_complexity, .. } => Some(*closure_complexity),
        PipelineStage::Filter { closure_complexity, .. } => Some(*closure_complexity),
        PipelineStage::FlatMap { closure_complexity, .. } => Some(*closure_complexity),
        PipelineStage::AndThen { closure_complexity } => Some(*closure_complexity),
        PipelineStage::MapErr { closure_complexity } => Some(*closure_complexity),
        PipelineStage::Fold { fold_complexity, .. } => Some(*fold_complexity),
        _ => None,
    }
}

// Final composition metrics
pub struct CompositionMetrics {
    pub pipelines: Vec<Pipeline>,
    pub purity_score: f64,
    pub immutability_ratio: f64,
    pub composition_quality: f64,
    pub side_effect_kind: SideEffectKind,
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
    pub min_function_complexity: u32,       // Skip analysis for trivial functions (default: 3)
}

impl Default for FunctionalAnalysisConfig {
    fn default() -> Self {
        Self::balanced()
    }
}

impl FunctionalAnalysisConfig {
    /// Strict configuration for codebases emphasizing functional purity
    pub fn strict() -> Self {
        Self {
            min_pipeline_depth: 3,
            max_closure_complexity: 3,
            purity_threshold: 0.9,
            composition_quality_threshold: 0.7,
            min_function_complexity: 2,
        }
    }

    /// Balanced configuration (default) for typical Rust codebases
    pub fn balanced() -> Self {
        Self {
            min_pipeline_depth: 2,
            max_closure_complexity: 5,
            purity_threshold: 0.8,
            composition_quality_threshold: 0.6,
            min_function_complexity: 3,
        }
    }

    /// Lenient configuration for imperative-heavy codebases
    pub fn lenient() -> Self {
        Self {
            min_pipeline_depth: 2,
            max_closure_complexity: 10,
            purity_threshold: 0.5,
            composition_quality_threshold: 0.4,
            min_function_complexity: 5,
        }
    }

    /// Check if a function should be analyzed based on complexity threshold
    pub fn should_analyze(&self, complexity: u32) -> bool {
        complexity >= self.min_function_complexity
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

// CLI flags
Commands::Analyze {
    // ... existing fields ...

    /// Enable deep AST-based functional pattern analysis
    #[arg(long = "ast-functional-analysis")]
    ast_functional_analysis: bool,

    /// Configuration profile: strict, balanced, or lenient
    #[arg(long = "functional-analysis-profile", default_value = "balanced")]
    functional_analysis_profile: String,
}

// Config file
[analysis]
enable_ast_functional_analysis = true

# Use a profile (strict, balanced, lenient) or specify custom values
functional_analysis_profile = "balanced"

# Or customize individual settings (overrides profile)
[analysis.functional_analysis]
min_pipeline_depth = 2
max_closure_complexity = 5
purity_threshold = 0.8
composition_quality_threshold = 0.6
min_function_complexity = 3
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

### Test Corpus Design

Create a comprehensive test corpus in `tests/fixtures/functional_patterns/`:

#### Positive Examples (Should Detect as Functional)

**1. Simple Iterator Pipelines** (20 files):
```rust
// tests/fixtures/functional_patterns/positive/simple_pipeline_01.rs
fn process_numbers(nums: Vec<i32>) -> Vec<i32> {
    nums.iter()
        .map(|x| x * 2)
        .filter(|x| x > &10)
        .collect()
}
```

**2. Parallel Iterators** (10 files):
```rust
// tests/fixtures/functional_patterns/positive/parallel_01.rs
use rayon::prelude::*;

fn parallel_transform(data: Vec<Data>) -> Vec<Result> {
    data.par_iter()
        .map(|item| expensive_computation(item))
        .collect()
}
```

**3. Pure Functions** (15 files):
```rust
// tests/fixtures/functional_patterns/positive/pure_01.rs
fn calculate_score(metrics: &Metrics) -> f64 {
    let base = metrics.complexity as f64;
    let adjusted = base * metrics.weight;
    adjusted.clamp(0.0, 100.0)
}
```

**4. Monadic Chaining** (10 files):
```rust
// tests/fixtures/functional_patterns/positive/monadic_01.rs
fn parse_and_validate(input: &str) -> Result<Config, Error> {
    parse_config(input)?
        .validate()
        .and_then(|cfg| cfg.normalize())
        .map(|cfg| cfg.with_defaults())
}
```

**5. Nested Pipelines** (10 files):
```rust
// tests/fixtures/functional_patterns/positive/nested_01.rs
fn process_matrix(data: Vec<Vec<i32>>) -> Vec<i32> {
    data.iter()
        .flat_map(|row| row.iter().map(|x| x * 2))
        .filter(|x| x > &10)
        .collect()
}
```

#### Negative Examples (Should NOT Detect as Functional)

**1. Imperative Loops** (15 files):
```rust
// tests/fixtures/functional_patterns/negative/imperative_01.rs
fn process_items_imperative(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 10 {
            result.push(item * 2);
        }
    }
    result
}
```

**2. Functional Names, Imperative Implementation** (10 files):
```rust
// tests/fixtures/functional_patterns/negative/misleading_01.rs
fn map_results(data: &[Data]) -> Vec<Result> {
    let mut results = Vec::new();
    for item in data {
        let mut processed = item.clone();
        processed.apply_mutation();
        results.push(processed.into());
    }
    results
}
```

**3. Builder Patterns** (5 files):
```rust
// tests/fixtures/functional_patterns/negative/builder_01.rs
fn create_config() -> Config {
    ConfigBuilder::new()
        .with_timeout(100)
        .with_retries(3)
        .with_logging(true)
        .build()
}
```

**4. Impure Functions with Side Effects** (10 files):
```rust
// tests/fixtures/functional_patterns/negative/impure_01.rs
fn process_with_logging(items: Vec<i32>) -> Vec<i32> {
    println!("Processing {} items", items.len());
    let mut cache = HashMap::new();
    items.iter()
        .map(|x| {
            cache.insert(*x, x * 2);  // Mutation
            x * 2
        })
        .collect()
}
```

**5. Complex Closures** (5 files):
```rust
// tests/fixtures/functional_patterns/negative/complex_closure_01.rs
fn overly_complex_pipeline(data: Vec<Data>) -> Vec<Result> {
    data.iter()
        .map(|item| {
            // 20+ lines of complex logic
            let mut intermediate = Vec::new();
            for i in 0..item.count {
                if validate_condition(i) {
                    intermediate.push(transform(i));
                }
            }
            aggregate(intermediate)
        })
        .collect()
}
```

#### Edge Cases (Mixed Patterns)

**1. Benign Side Effects** (5 files):
```rust
// tests/fixtures/functional_patterns/edge/benign_logging_01.rs
use tracing::debug;

fn process_with_tracing(items: Vec<i32>) -> Vec<i32> {
    debug!("Processing items");
    items.iter()
        .map(|x| {
            debug!("Processing item: {}", x);
            x * 2
        })
        .collect()
}
```

**2. Async Patterns** (5 files - deferred):
```rust
// tests/fixtures/functional_patterns/edge/async_01.rs
async fn async_pipeline(items: Vec<Data>) -> Vec<Result> {
    futures::stream::iter(items)
        .then(|item| async move { process(item).await })
        .collect()
        .await
}
```

#### Precision/Recall Measurement

```rust
// tests/integration/functional_pattern_accuracy.rs
#[test]
fn test_pattern_detection_precision_recall() {
    let positive_files = glob("tests/fixtures/functional_patterns/positive/**/*.rs");
    let negative_files = glob("tests/fixtures/functional_patterns/negative/**/*.rs");

    let mut true_positives = 0;
    let mut false_positives = 0;
    let mut true_negatives = 0;
    let mut false_negatives = 0;

    for file in positive_files {
        let result = analyze_file(&file);
        if result.composition_quality >= 0.6 {
            true_positives += 1;
        } else {
            false_negatives += 1;
        }
    }

    for file in negative_files {
        let result = analyze_file(&file);
        if result.composition_quality < 0.6 {
            true_negatives += 1;
        } else {
            false_positives += 1;
        }
    }

    let precision = true_positives as f64 / (true_positives + false_positives) as f64;
    let recall = true_positives as f64 / (true_positives + false_negatives) as f64;

    assert!(precision >= 0.90, "Precision: {:.2}", precision);
    assert!(recall >= 0.85, "Recall: {:.2}", recall);
}
```

#### Test Corpus Summary

- **Total files**: 95
  - Positive examples: 65 files
  - Negative examples: 45 files
  - Edge cases: 10 files (5 benign logging, 5 async deferred)
- **Target metrics**:
  - Precision ≥ 90% (low false positives)
  - Recall ≥ 85% (low false negatives)
  - F1 Score ≥ 0.87

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

1. **Complex closures**: Closures with high complexity should reduce pipeline quality score relative to expected complexity for pipeline depth

2. **Nested pipelines**: Detect pipelines within closures using recursive detection
   ```rust
   fn detect_nested_pipeline(closure: &syn::ExprClosure) -> Option<Pipeline> {
       // Recursively analyze closure body for iterator chains
       match &*closure.body {
           syn::Expr::MethodCall(method_call) => {
               extract_pipeline_from_method_call(method_call, &config)
           }
           _ => None,
       }
   }

   // Example: flat_map with nested pipeline
   data.iter()
       .flat_map(|row| row.iter().map(|x| x * 2))  // Nested pipeline detected!
       .collect()
   ```

3. **Builder patterns vs functional pipelines**: Distinguish by checking for terminal operations and iterator traits
   ```rust
   fn is_builder_pattern(pipeline: &Pipeline) -> bool {
       // Builders typically end with .build(), not collection operations
       matches!(
           pipeline.terminal_operation,
           None  // No terminal op, just returns self
       ) && pipeline.stages.iter().all(|stage| {
           // Builder methods don't use closures, they take values
           !matches!(stage, PipelineStage::Map { .. } | PipelineStage::Filter { .. })
       })
   }

   // Builder pattern example:
   ConfigBuilder::new()
       .with_timeout(100)    // Returns self, not iterator transformation
       .build()              // Terminal but not iterator terminal

   // Functional pipeline example:
   items.iter()
       .map(|x| x * 2)       // Closure transformation
       .collect()            // Iterator terminal
   ```

4. **Mixed imperative/functional**: Score based on ratio of functional vs imperative patterns
   ```rust
   fn calculate_functional_ratio(function: &syn::ItemFn) -> f64 {
       let pipeline_lines = count_pipeline_lines(function);
       let total_lines = count_total_lines(function);

       if total_lines == 0 {
           return 0.0;
       }

       pipeline_lines as f64 / total_lines as f64
   }
   ```

5. **External iterator adapters**: Recognize custom iterator traits beyond std library
   ```rust
   const KNOWN_ITERATOR_METHODS: &[&str] = &[
       // std iterators
       "iter", "into_iter", "iter_mut", "map", "filter", "fold",
       // rayon parallel iterators
       "par_iter", "par_bridge", "par_iter_mut",
       // itertools
       "chunks", "windows", "tuple_windows", "group_by",
   ];

   fn is_iterator_method(method_name: &str) -> bool {
       KNOWN_ITERATOR_METHODS.contains(&method_name)
   }
   ```

6. **Async iterator patterns** (deferred to future spec):
   - `futures::stream::Stream` trait methods
   - `.then()`, `.and_then()` for async transformations
   - `.collect().await` for async terminal operations

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

## Open Questions (Resolved)

### 1. Closure complexity threshold
**Question**: Should complex closures (> 5 complexity) always penalize pipeline quality, or are some patterns acceptable?

**Resolution**: Use context-aware penalty based on pipeline depth. Complexity is expected to scale with pipeline depth:
```rust
fn calculate_closure_penalty(pipeline: &Pipeline, config: &FunctionalAnalysisConfig) -> f64 {
    let avg_complexity = average_closure_complexity(pipeline);
    let expected_complexity = (pipeline.depth as u32 * 2).min(config.max_closure_complexity);

    // Only penalize if exceeds expected complexity for pipeline depth
    if avg_complexity > expected_complexity as f64 {
        ((avg_complexity - expected_complexity as f64) * 0.05).min(0.3)
    } else {
        0.0
    }
}
```

### 2. Parallel iterator scoring
**Question**: Should parallel iterators always get bonus, or only when workload justifies parallelism?

**Resolution**: Award bonus only for pipelines with sufficient depth (≥ 3 stages) to justify parallelization overhead:
```rust
fn calculate_parallel_bonus(pipeline: &Pipeline) -> f64 {
    if pipeline.is_parallel && pipeline.depth >= 3 {
        0.2  // Likely worth parallelization
    } else {
        0.0  // Overhead may outweigh benefit
    }
}
```

### 3. Purity edge cases
**Question**: How to handle functions with tracing/logging (side effects but harmless)?

**Resolution**: Introduce `SideEffectKind` enum to distinguish benign from impure side effects:
```rust
pub enum SideEffectKind {
    Pure,    // No side effects
    Benign,  // Only logging/tracing/metrics (small penalty: -0.1)
    Impure,  // I/O, mutation, network (large penalty: -0.4 to -0.7)
}

fn classify_macro_side_effect(mac: &syn::Macro) -> SideEffectKind {
    match mac.path.segments.last().map(|s| s.ident.to_string()).as_deref() {
        Some("debug") | Some("info") | Some("warn") | Some("error") | Some("trace") =>
            SideEffectKind::Benign,
        Some("println") | Some("eprintln") =>
            SideEffectKind::Impure,
        _ => SideEffectKind::Pure,
    }
}
```

### 4. Multi-language priority
**Question**: Which language to support next after Rust?

**Resolution**: Python, due to:
- Strong functional features (comprehensions, `map`/`filter`, `itertools`)
- Debtmap already supports Python analysis infrastructure
- Prevalent in data science where functional pipelines are common
- Easier to implement than TypeScript (no complex type system to analyze)

**Implementation approach** (deferred to future spec):
```python
# Functional patterns to detect in Python:
result = [
    transform(x)
    for x in items
    if predicate(x)
]  # List comprehension - functional pattern

result = map(lambda x: x * 2, items)  # map/filter built-ins

from itertools import chain, groupby
result = chain.from_iterable(groups)  # itertools patterns
```

## Summary of Refinements

This refined specification incorporates the following improvements over the initial draft:

### 1. Functional Programming Principles

**Improved**: Replaced stateful visitors with pure functional accumulation patterns
- `PurityVisitor` → `PurityAccumulator` with `merge()` operations
- Stateless free functions instead of stateful analyzers
- Immutable data flow using `fold()` and functional composition

```rust
// Before: Stateful visitor
struct PurityVisitor { mutable_bindings: usize, ... }

// After: Functional accumulation
fn analyze_block_purity(block: &syn::Block) -> PurityAccumulator {
    block.stmts.iter()
        .map(|stmt| analyze_stmt_purity(stmt))
        .fold(PurityAccumulator::default(), |acc, metrics| acc.merge(metrics))
}
```

### 2. Enhanced Edge Case Handling

**Added**:
- Nested pipeline detection within closures
- Builder pattern vs functional pipeline distinction
- Benign side effect classification (logging/tracing)
- Context-aware closure complexity penalties
- Async pattern recognition (deferred)

### 3. Configuration Simplification

**Added**: Three predefined profiles to reduce configuration complexity
- **Strict**: For pure functional codebases (higher thresholds)
- **Balanced**: Default for typical Rust code
- **Lenient**: For imperative-heavy codebases

### 4. Comprehensive Test Corpus

**Specified**: 95-file test corpus with concrete examples
- 65 positive examples (functional patterns)
- 45 negative examples (imperative, builders)
- 10 edge cases (benign logging, async)
- Measurable precision/recall targets (≥90%/≥85%)

### 5. Resolved Open Questions

**Clarified**:
- Closure complexity: Use context-aware penalties based on pipeline depth
- Parallel bonus: Only for pipelines with ≥3 stages
- Side effects: Three-tier classification (Pure, Benign, Impure)
- Multi-language: Python prioritized for next implementation

### 6. Performance Optimizations

**Added**:
- Early termination via `min_function_complexity` threshold
- AST caching strategy
- Parallel processing with rayon
- Lazy evaluation for non-critical functions

### Implementation Readiness

The refined spec provides:
- ✅ Clear functional architecture (pure functions, immutable data)
- ✅ Detailed edge case handling with code examples
- ✅ Measurable acceptance criteria with test corpus
- ✅ Configuration flexibility via profiles
- ✅ Resolved design decisions

**Estimated Implementation**: 12-15 days (unchanged, but now with higher confidence)
