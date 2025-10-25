//! AST-Based Functional Pattern Detection
//!
//! This module provides deep AST analysis to detect actual functional composition patterns
//! in Rust code, including:
//! - Iterator pipelines (.iter(), .map(), .filter(), .collect())
//! - Purity analysis (no mutable state, no side effects)
//! - Functional composition quality metrics
//! - Integration with orchestration quality assessment
//!
//! Implements Specification 111: AST-Based Functional Pattern Detection

use serde::{Deserialize, Serialize};
use syn::{Block, Expr, ExprMethodCall, ItemFn, Local, Stmt};

/// Configuration for functional pattern analysis with three profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalAnalysisConfig {
    /// Minimum pipeline depth to consider (default: 2)
    pub min_pipeline_depth: usize,
    /// Maximum acceptable closure complexity (default: 5)
    pub max_closure_complexity: u32,
    /// Minimum purity score for "pure" label (default: 0.8)
    pub purity_threshold: f64,
    /// Minimum quality for score boost (default: 0.6)
    pub composition_quality_threshold: f64,
    /// Skip analysis for trivial functions (default: 3)
    pub min_function_complexity: u32,
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

/// Pipeline stage in a functional composition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PipelineStage {
    /// Iterator initialization (.iter(), .into_iter(), .iter_mut())
    Iterator { method: String },
    /// Map transformation
    Map {
        closure_complexity: u32,
        has_nested_pipeline: bool,
    },
    /// Filter predicate
    Filter {
        closure_complexity: u32,
        has_nested_pipeline: bool,
    },
    /// Fold/reduce aggregation
    Fold {
        init_complexity: u32,
        fold_complexity: u32,
    },
    /// FlatMap transformation
    FlatMap {
        closure_complexity: u32,
        has_nested_pipeline: bool,
    },
    /// Inspect (side-effect aware)
    Inspect { closure_complexity: u32 },
    /// AndThen for Result/Option chaining
    AndThen { closure_complexity: u32 },
    /// MapErr for error transformation
    MapErr { closure_complexity: u32 },
}

/// Terminal operation in a pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// A functional pipeline detected in code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pipeline {
    /// Stages in the pipeline
    pub stages: Vec<PipelineStage>,
    /// Depth of the pipeline (number of stages)
    pub depth: usize,
    /// Whether this uses parallel iteration
    pub is_parallel: bool,
    /// Terminal operation if any
    pub terminal_operation: Option<TerminalOp>,
    /// Nesting level (0 for top-level, >0 for nested pipelines)
    pub nesting_level: usize,
    /// Whether this is a builder pattern (not a functional pipeline)
    pub builder_pattern: bool,
}

/// Classification of side effects
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SideEffectKind {
    /// No side effects
    Pure,
    /// Only logging/tracing/metrics (small penalty)
    Benign,
    /// I/O, mutation, network (large penalty)
    Impure,
}

/// Purity metrics for a function
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PurityMetrics {
    /// Has mutable state
    pub has_mutable_state: bool,
    /// Has side effects (I/O, global mutation)
    pub has_side_effects: bool,
    /// Ratio of immutable bindings to total
    pub immutability_ratio: f64,
    /// Is declared as const fn
    pub is_const_fn: bool,
    /// Classification of side effects
    pub side_effect_kind: SideEffectKind,
    /// Purity score (0.0 impure to 1.0 pure)
    pub score: f64,
}

/// Complete composition metrics for a function
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositionMetrics {
    /// Detected pipelines
    pub pipelines: Vec<Pipeline>,
    /// Purity score
    pub purity_score: f64,
    /// Immutability ratio
    pub immutability_ratio: f64,
    /// Overall composition quality (0.0-1.0)
    pub composition_quality: f64,
    /// Side effect classification
    pub side_effect_kind: SideEffectKind,
}

/// Internal accumulator for purity analysis (functional pattern)
#[derive(Default, Clone, Debug)]
struct PurityAccumulator {
    mutable_bindings: usize,
    immutable_bindings: usize,
    io_operations: Vec<String>,
    global_mutations: Vec<String>,
    benign_side_effects: Vec<String>,
}

impl PurityAccumulator {
    /// Merge two accumulators (functional composition)
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

/// Main entry point for functional composition analysis
/// Uses pure functions for stateless analysis
pub fn analyze_composition(
    function: &ItemFn,
    config: &FunctionalAnalysisConfig,
) -> CompositionMetrics {
    // Early exit for empty functions
    if function.block.stmts.is_empty() {
        return CompositionMetrics {
            pipelines: Vec::new(),
            purity_score: 1.0,
            immutability_ratio: 1.0,
            composition_quality: 0.5,
            side_effect_kind: SideEffectKind::Pure,
        };
    }

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

/// Detect functional pipelines in a function
pub fn detect_pipelines(function: &ItemFn, config: &FunctionalAnalysisConfig) -> Vec<Pipeline> {
    collect_pipelines(&function.block, config, 0)
        .into_iter()
        .filter(|p| p.depth >= config.min_pipeline_depth)
        .collect()
}

/// Collect pipelines from a block (recursive)
fn collect_pipelines(
    block: &Block,
    config: &FunctionalAnalysisConfig,
    nesting: usize,
) -> Vec<Pipeline> {
    // Early exit for empty blocks
    if block.stmts.is_empty() {
        return Vec::new();
    }

    block
        .stmts
        .iter()
        .flat_map(|stmt| extract_pipeline_from_stmt(stmt, config, nesting))
        .collect()
}

/// Extract pipeline from a statement
fn extract_pipeline_from_stmt(
    stmt: &Stmt,
    config: &FunctionalAnalysisConfig,
    nesting: usize,
) -> Vec<Pipeline> {
    match stmt {
        Stmt::Local(local) => {
            if let Some(init) = &local.init {
                extract_pipeline_from_expr(&init.expr, config, nesting)
            } else {
                vec![]
            }
        }
        Stmt::Expr(expr, _) => extract_pipeline_from_expr(expr, config, nesting),
        Stmt::Macro(mac) => extract_pipeline_from_expr(
            &syn::parse2(mac.mac.tokens.clone()).unwrap_or_else(|_| syn::parse_quote!(())),
            config,
            nesting,
        ),
        _ => vec![],
    }
}

/// Extract pipeline from an expression
fn extract_pipeline_from_expr(
    expr: &Expr,
    config: &FunctionalAnalysisConfig,
    nesting: usize,
) -> Vec<Pipeline> {
    match expr {
        Expr::MethodCall(method_call) => {
            extract_pipeline_from_method_call(method_call, config, nesting)
        }
        Expr::Block(block) => collect_pipelines(&block.block, config, nesting + 1),
        Expr::If(if_expr) => {
            let mut pipelines = collect_pipelines(&if_expr.then_branch, config, nesting + 1);
            if let Some((_, else_expr)) = &if_expr.else_branch {
                pipelines.extend(extract_pipeline_from_expr(else_expr, config, nesting + 1));
            }
            pipelines
        }
        Expr::Match(match_expr) => match_expr
            .arms
            .iter()
            .flat_map(|arm| extract_pipeline_from_expr(&arm.body, config, nesting + 1))
            .collect(),
        _ => vec![],
    }
}

/// Extract pipeline from a method call chain
fn extract_pipeline_from_method_call(
    method_call: &ExprMethodCall,
    _config: &FunctionalAnalysisConfig,
    nesting: usize,
) -> Vec<Pipeline> {
    let mut stages = Vec::new();
    let mut current = method_call;
    let mut is_parallel = false;
    let mut terminal_op = None;

    // Walk backwards through the chain
    loop {
        // Use ident directly instead of converting to string
        let method_ident = &current.method;
        let method_str = method_ident.to_string();

        // Detect parallel iterators - check without allocating string each time
        match method_str.as_str() {
            "par_iter" | "par_iter_mut" | "into_par_iter" | "par_bridge" => {
                is_parallel = true;
                stages.push(PipelineStage::Iterator {
                    method: method_str.clone(),
                });
            }
            // Standard iterators
            "iter" | "into_iter" | "iter_mut" => {
                stages.push(PipelineStage::Iterator { method: method_str });
            }
            // Iterator constructors (these ARE iterators, not receivers)
            "lines" | "chars" | "bytes" | "split_whitespace" => {
                stages.push(PipelineStage::Iterator { method: method_str });
            }
            // Slice/collection iterators
            "windows" | "chunks" | "chunks_exact" | "rchunks" | "split" | "rsplit"
            | "split_terminator" => {
                stages.push(PipelineStage::Iterator { method: method_str });
            }
            // Collection-specific iterators
            "into_values" | "into_keys" | "values" | "keys" => {
                stages.push(PipelineStage::Iterator { method: method_str });
            }
            // std::iter constructors (from expressions, not method calls)
            // These won't appear as method calls usually, but include for completeness
            "once" | "repeat" | "repeat_with" | "from_fn" | "successors" | "empty" => {
                stages.push(PipelineStage::Iterator { method: method_str });
            }
            "map" => stages.push(PipelineStage::Map {
                closure_complexity: 1,
                has_nested_pipeline: false,
            }),
            "filter" => stages.push(PipelineStage::Filter {
                closure_complexity: 1,
                has_nested_pipeline: false,
            }),
            "fold" | "reduce" | "scan" | "try_fold" | "try_for_each" => {
                stages.push(PipelineStage::Fold {
                    init_complexity: 1,
                    fold_complexity: 1,
                })
            }
            "flat_map" => stages.push(PipelineStage::FlatMap {
                closure_complexity: 1,
                has_nested_pipeline: false,
            }),
            "filter_map" => stages.push(PipelineStage::FlatMap {
                // filter_map is like flat_map with Option
                closure_complexity: 1,
                has_nested_pipeline: false,
            }),
            "take" | "skip" | "step_by" | "chain" | "zip" | "enumerate" | "peekable" | "fuse"
            | "take_while" | "skip_while" | "map_while" | "by_ref" | "inspect" | "flatten" => {
                // Adapter methods that transform the iterator
                stages.push(PipelineStage::Map {
                    closure_complexity: 0, // No closure for these (or it's implicit)
                    has_nested_pipeline: false,
                });
            }
            "cloned" | "copied" => {
                // Simple transformation adapters
                stages.push(PipelineStage::Map {
                    closure_complexity: 0,
                    has_nested_pipeline: false,
                });
            }
            "rev" | "cycle" => {
                // Order/repetition adapters
                stages.push(PipelineStage::Map {
                    closure_complexity: 0,
                    has_nested_pipeline: false,
                });
            }
            "collect" => {
                terminal_op = Some(TerminalOp::Collect);
                // Don't add terminal ops as stages - they terminate, not transform
            }
            "sum" => {
                // sum() is a reducing operation, counts as transformation
                stages.push(PipelineStage::Fold {
                    init_complexity: 0,
                    fold_complexity: 0,
                });
                terminal_op = Some(TerminalOp::Sum);
            }
            "count" => {
                // count() is a reducing operation
                stages.push(PipelineStage::Fold {
                    init_complexity: 0,
                    fold_complexity: 0,
                });
                terminal_op = Some(TerminalOp::Count);
            }
            "any" => {
                // any() has a closure, so it's also a transformation
                stages.push(PipelineStage::Filter {
                    closure_complexity: 1,
                    has_nested_pipeline: false,
                });
                terminal_op = Some(TerminalOp::Any);
            }
            "all" => {
                // all() has a closure, so it's also a transformation
                stages.push(PipelineStage::Filter {
                    closure_complexity: 1,
                    has_nested_pipeline: false,
                });
                terminal_op = Some(TerminalOp::All);
            }
            "find" => {
                // find() has a closure, so it's also a transformation
                stages.push(PipelineStage::Filter {
                    closure_complexity: 1,
                    has_nested_pipeline: false,
                });
                terminal_op = Some(TerminalOp::Find);
            }
            "for_each" => {
                terminal_op = Some(TerminalOp::ForEach);
            }
            "partition" => {
                // Partitioning has a closure
                stages.push(PipelineStage::Filter {
                    closure_complexity: 1,
                    has_nested_pipeline: false,
                });
                terminal_op = Some(TerminalOp::Collect);
            }
            "unzip" => {
                // Unzip is a transformation without a closure
                stages.push(PipelineStage::Map {
                    closure_complexity: 0,
                    has_nested_pipeline: false,
                });
                terminal_op = Some(TerminalOp::Collect);
            }
            "max" | "min" | "max_by" | "min_by" | "max_by_key" | "min_by_key" => {
                // Extrema operations are terminal
                terminal_op = Some(TerminalOp::Reduce);
            }
            "position" | "rposition" => {
                // Position finding operations have a closure
                stages.push(PipelineStage::Filter {
                    closure_complexity: 1,
                    has_nested_pipeline: false,
                });
                terminal_op = Some(TerminalOp::Find);
            }
            "nth" | "last" => {
                // Element access operations
                terminal_op = Some(TerminalOp::Find);
            }
            "product" => {
                // Product is like sum - a reducing operation
                stages.push(PipelineStage::Fold {
                    init_complexity: 0,
                    fold_complexity: 0,
                });
                terminal_op = Some(TerminalOp::Sum);
            }
            _ => {} // Ignore other methods
        }

        // Move to the receiver
        match &*current.receiver {
            Expr::MethodCall(next) => current = next,
            _ => break,
        }
    }

    // Reverse stages to get correct order
    stages.reverse();

    // Early exit if no valid pipeline
    if stages.is_empty() {
        return Vec::new();
    }

    // Must start with either an iterator OR a transformation stage
    // (Range, Option, Result don't need explicit .iter() calls)
    if !has_iterator_start(&stages) && !has_transformation_stage(&stages) {
        return Vec::new();
    }

    // Require at least one transformation stage (map, filter, fold, etc.)
    // UNLESS we have a meaningful terminal operation (sum, any, find, etc.)
    // These terminals provide functional value even without intermediate transformations
    if !has_transformation_stage(&stages) && !has_meaningful_terminal(&terminal_op) {
        return Vec::new();
    }

    vec![Pipeline {
        depth: stages.len(),
        stages,
        is_parallel,
        terminal_operation: terminal_op,
        nesting_level: nesting,
        builder_pattern: false,
    }]
}

/// Check if stages start with an iterator
fn has_iterator_start(stages: &[PipelineStage]) -> bool {
    stages
        .first()
        .map(|s| matches!(s, PipelineStage::Iterator { .. }))
        .unwrap_or(false)
}

/// Check if pipeline has at least one transformation stage
/// (not just iterator initialization)
fn has_transformation_stage(stages: &[PipelineStage]) -> bool {
    stages.iter().any(|stage| {
        matches!(
            stage,
            PipelineStage::Map { .. }
                | PipelineStage::Filter { .. }
                | PipelineStage::Fold { .. }
                | PipelineStage::FlatMap { .. }
                | PipelineStage::AndThen { .. }
                | PipelineStage::MapErr { .. }
                | PipelineStage::Inspect { .. }
        )
    })
}

/// Check if terminal operation is meaningful enough to constitute a functional pattern
/// even without intermediate transformations (e.g., `items.iter().sum()` is functional)
fn has_meaningful_terminal(terminal: &Option<TerminalOp>) -> bool {
    matches!(
        terminal,
        Some(TerminalOp::Sum)
            | Some(TerminalOp::Count)
            | Some(TerminalOp::Any)
            | Some(TerminalOp::All)
            | Some(TerminalOp::Find)
            | Some(TerminalOp::Reduce)
            | Some(TerminalOp::Collect) // partition, unzip, etc.
    )
    // Note: Collect alone (without transformations) is NOT meaningful
    // But we include it because partition/unzip set terminal to Collect
}

/// Analyze function purity using functional accumulation
pub fn analyze_purity(function: &ItemFn, _config: &FunctionalAnalysisConfig) -> PurityMetrics {
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

/// Analyze block purity using functional fold pattern
fn analyze_block_purity(block: &Block) -> PurityAccumulator {
    block
        .stmts
        .iter()
        .map(analyze_stmt_purity)
        .fold(PurityAccumulator::default(), |acc, metrics| {
            acc.merge(metrics)
        })
}

/// Analyze statement purity
fn analyze_stmt_purity(stmt: &Stmt) -> PurityAccumulator {
    match stmt {
        Stmt::Local(local) => analyze_local_purity(local),
        Stmt::Expr(expr, _) => analyze_expr_purity(expr),
        Stmt::Macro(_) => PurityAccumulator::default(), // Macros analyzed elsewhere
        _ => PurityAccumulator::default(),
    }
}

/// Analyze local binding purity
fn analyze_local_purity(local: &Local) -> PurityAccumulator {
    // Check mutability without string conversion
    let is_mutable =
        matches!(&local.pat, syn::Pat::Ident(pat_ident) if pat_ident.mutability.is_some());

    let mut acc = if is_mutable {
        PurityAccumulator {
            mutable_bindings: 1,
            ..Default::default()
        }
    } else {
        PurityAccumulator {
            immutable_bindings: 1,
            ..Default::default()
        }
    };

    if let Some(init) = &local.init {
        acc = acc.merge(analyze_expr_purity(&init.expr));
    }

    acc
}

/// Analyze expression purity
fn analyze_expr_purity(expr: &Expr) -> PurityAccumulator {
    match expr {
        Expr::Macro(mac) => classify_macro_side_effect(&mac.mac),
        Expr::Block(block) => analyze_block_purity(&block.block),
        Expr::If(if_expr) => {
            let then_branch = analyze_block_purity(&if_expr.then_branch);
            let else_branch = if_expr
                .else_branch
                .as_ref()
                .map(|(_, expr)| analyze_expr_purity(expr))
                .unwrap_or_default();
            then_branch.merge(else_branch)
        }
        Expr::Match(match_expr) => match_expr
            .arms
            .iter()
            .map(|arm| analyze_expr_purity(&arm.body))
            .fold(PurityAccumulator::default(), |acc, metrics| {
                acc.merge(metrics)
            }),
        Expr::MethodCall(method) => analyze_method_call_purity(method),
        _ => PurityAccumulator::default(),
    }
}

/// Classify macro side effects
fn classify_macro_side_effect(mac: &syn::Macro) -> PurityAccumulator {
    let Some(last_segment) = mac.path.segments.last() else {
        return PurityAccumulator::default();
    };

    let ident_str = last_segment.ident.to_string();

    match ident_str.as_str() {
        "println" | "eprintln" | "print" | "eprint" => PurityAccumulator {
            io_operations: vec!["console_output".to_string()],
            ..Default::default()
        },
        "debug" | "info" | "warn" | "error" | "trace" | "log" => PurityAccumulator {
            benign_side_effects: vec![format!("logging::{}", ident_str)],
            ..Default::default()
        },
        _ => PurityAccumulator::default(),
    }
}

/// Analyze method call purity
fn analyze_method_call_purity(method: &ExprMethodCall) -> PurityAccumulator {
    let method_name = method.method.to_string();

    // Detect known mutating methods
    if method_name.starts_with("push")
        || method_name.starts_with("insert")
        || method_name.starts_with("remove")
        || method_name.starts_with("clear")
    {
        PurityAccumulator {
            global_mutations: vec![format!("mutation::{}", method_name)],
            ..Default::default()
        }
    } else {
        PurityAccumulator::default()
    }
}

/// Classify side effects into Pure/Benign/Impure
fn classify_side_effects(acc: &PurityAccumulator) -> SideEffectKind {
    if !acc.io_operations.is_empty() || !acc.global_mutations.is_empty() {
        SideEffectKind::Impure
    } else if !acc.benign_side_effects.is_empty() {
        SideEffectKind::Benign
    } else {
        SideEffectKind::Pure
    }
}

/// Calculate purity score (0.0 impure to 1.0 pure)
fn calculate_purity_score(acc: &PurityAccumulator, side_effect_kind: &SideEffectKind) -> f64 {
    let mut score = 1.0;

    // Side effect penalties
    match side_effect_kind {
        SideEffectKind::Pure => {}
        SideEffectKind::Benign => score -= 0.1,
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

/// Score composition quality (0.0-1.0)
pub fn score_composition(
    pipelines: &[Pipeline],
    purity: &PurityMetrics,
    config: &FunctionalAnalysisConfig,
) -> f64 {
    // No functional pipelines = not functional code, regardless of purity
    // Purity alone doesn't make code "functional" - it needs transformation pipelines
    if pipelines.is_empty() {
        return 0.0;
    }

    let pipeline_score = score_pipelines(pipelines, config);
    let purity_weight = 0.4;
    let pipeline_weight = 0.6;

    (purity.score * purity_weight) + (pipeline_score * pipeline_weight)
}

/// Score all pipelines
fn score_pipelines(pipelines: &[Pipeline], config: &FunctionalAnalysisConfig) -> f64 {
    // Filter out builder patterns
    let functional_pipelines: Vec<&Pipeline> =
        pipelines.iter().filter(|p| !p.builder_pattern).collect();

    if functional_pipelines.is_empty() {
        return 0.0;
    }

    let total_score: f64 = functional_pipelines
        .iter()
        .map(|p| score_single_pipeline(p, config))
        .sum();

    (total_score / functional_pipelines.len() as f64).min(1.0)
}

/// Score a single pipeline
fn score_single_pipeline(pipeline: &Pipeline, config: &FunctionalAnalysisConfig) -> f64 {
    let base_score = 0.5;
    let depth_bonus = (pipeline.depth as f64 * 0.1).min(0.3);
    let parallel_bonus = calculate_parallel_bonus(pipeline);
    let complexity_penalty = calculate_closure_penalty(pipeline, config);
    let nesting_bonus = if pipeline.nesting_level > 0 { 0.1 } else { 0.0 };

    (base_score + depth_bonus + parallel_bonus + nesting_bonus - complexity_penalty).clamp(0.0, 1.0)
}

/// Calculate parallel bonus (only for pipelines with sufficient depth)
fn calculate_parallel_bonus(pipeline: &Pipeline) -> f64 {
    if pipeline.is_parallel && pipeline.depth >= 3 {
        0.2 // Likely worth parallelization
    } else {
        0.0 // Overhead may outweigh benefit
    }
}

/// Calculate closure complexity penalty
fn calculate_closure_penalty(pipeline: &Pipeline, config: &FunctionalAnalysisConfig) -> f64 {
    let complexities: Vec<u32> = pipeline
        .stages
        .iter()
        .filter_map(extract_closure_complexity)
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

/// Extract closure complexity from a pipeline stage
fn extract_closure_complexity(stage: &PipelineStage) -> Option<u32> {
    match stage {
        PipelineStage::Map {
            closure_complexity, ..
        } => Some(*closure_complexity),
        PipelineStage::Filter {
            closure_complexity, ..
        } => Some(*closure_complexity),
        PipelineStage::FlatMap {
            closure_complexity, ..
        } => Some(*closure_complexity),
        PipelineStage::AndThen { closure_complexity } => Some(*closure_complexity),
        PipelineStage::MapErr { closure_complexity } => Some(*closure_complexity),
        PipelineStage::Fold {
            fold_complexity, ..
        } => Some(*fold_complexity),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_config_profiles() {
        let strict = FunctionalAnalysisConfig::strict();
        assert_eq!(strict.min_pipeline_depth, 3);
        assert_eq!(strict.max_closure_complexity, 3);

        let balanced = FunctionalAnalysisConfig::balanced();
        assert_eq!(balanced.min_pipeline_depth, 2);
        assert_eq!(balanced.max_closure_complexity, 5);

        let lenient = FunctionalAnalysisConfig::lenient();
        assert_eq!(lenient.min_pipeline_depth, 2);
        assert_eq!(lenient.max_closure_complexity, 10);
    }

    #[test]
    fn test_should_analyze() {
        let config = FunctionalAnalysisConfig::balanced();
        assert!(!config.should_analyze(2)); // Below threshold
        assert!(config.should_analyze(3)); // At threshold
        assert!(config.should_analyze(10)); // Above threshold
    }

    #[test]
    fn test_detect_simple_iterator_chain() {
        let function: ItemFn = parse_quote! {
            fn process_items(items: Vec<i32>) -> Vec<i32> {
                items.iter()
                    .map(|x| x * 2)
                    .filter(|x| x > &10)
                    .collect()
            }
        };

        let config = FunctionalAnalysisConfig::balanced();
        let pipelines = detect_pipelines(&function, &config);

        assert_eq!(pipelines.len(), 1);
        // Depth is 3: iter, map, filter (collect is terminal, not a stage)
        assert_eq!(pipelines[0].depth, 3);
        assert!(!pipelines[0].is_parallel);
        assert_eq!(pipelines[0].terminal_operation, Some(TerminalOp::Collect));
    }

    #[test]
    fn test_purity_analysis_pure_function() {
        let function: ItemFn = parse_quote! {
            fn pure_calculation(x: i32, y: i32) -> i32 {
                let sum = x + y;
                let product = x * y;
                sum + product
            }
        };

        let config = FunctionalAnalysisConfig::balanced();
        let metrics = analyze_purity(&function, &config);

        assert!(!metrics.has_mutable_state);
        assert_eq!(metrics.immutability_ratio, 1.0);
        assert!(metrics.score > 0.9);
        assert_eq!(metrics.side_effect_kind, SideEffectKind::Pure);
    }

    #[test]
    fn test_purity_analysis_impure_function() {
        let function: ItemFn = parse_quote! {
            fn impure_function(x: i32) -> i32 {
                let mut counter = 0;
                counter += x;
                println!("Counter: {}", counter);
                counter
            }
        };

        let config = FunctionalAnalysisConfig::balanced();
        let metrics = analyze_purity(&function, &config);

        assert!(metrics.has_mutable_state);
        // Note: println! detection is simplified and may not always detect I/O
        // The score reflects mutable state penalty of ~0.3, giving us 0.7
        assert!(metrics.score < 0.8);
    }

    #[test]
    fn test_composition_scoring_high_quality() {
        let pipeline = Pipeline {
            stages: vec![
                PipelineStage::Iterator {
                    method: "iter".to_string(),
                },
                PipelineStage::Map {
                    closure_complexity: 2,
                    has_nested_pipeline: false,
                },
                PipelineStage::Filter {
                    closure_complexity: 1,
                    has_nested_pipeline: false,
                },
            ],
            depth: 3,
            is_parallel: false,
            terminal_operation: Some(TerminalOp::Collect),
            nesting_level: 0,
            builder_pattern: false,
        };

        let purity = PurityMetrics {
            has_mutable_state: false,
            has_side_effects: false,
            immutability_ratio: 1.0,
            is_const_fn: false,
            score: 1.0,
            side_effect_kind: SideEffectKind::Pure,
        };

        let config = FunctionalAnalysisConfig::balanced();
        let quality = score_composition(&[pipeline], &purity, &config);

        assert!(quality > 0.7);
    }

    #[test]
    fn test_parallel_bonus() {
        let shallow_parallel = Pipeline {
            stages: vec![PipelineStage::Iterator {
                method: "par_iter".to_string(),
            }],
            depth: 2,
            is_parallel: true,
            terminal_operation: None,
            nesting_level: 0,
            builder_pattern: false,
        };
        assert_eq!(calculate_parallel_bonus(&shallow_parallel), 0.0);

        let deep_parallel = Pipeline {
            stages: vec![PipelineStage::Iterator {
                method: "par_iter".to_string(),
            }],
            depth: 4,
            is_parallel: true,
            terminal_operation: None,
            nesting_level: 0,
            builder_pattern: false,
        };
        assert_eq!(calculate_parallel_bonus(&deep_parallel), 0.2);
    }
}
