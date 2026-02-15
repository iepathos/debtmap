//! Advanced Analysis Module
//!
//! This module provides advanced analysis capabilities including:
//! - Rust-specific call graph analysis with trait dispatch and function pointers
//! - Design pattern recognition (Observer, Factory, Callback)
//! - Cross-module dependency tracking
//! - Multi-pass complexity analysis with attribution
//! - Diagnostic reporting and insights generation
//! - I/O and side effect detection for responsibility classification
//! - Call graph metrics and pattern detection for responsibility analysis
//! - Purity analysis for function classification and refactoring guidance
//! - Multi-signal aggregation for high-accuracy responsibility classification
//! - Type signature-based classification for improved responsibility detection
//! - Effect-based analysis patterns for testability and composability (Spec 207)
//! - Data flow analysis for state transition and mutation tracking (Spec 201)
//! - Analysis workflow state machine with checkpoint/resume support (Spec 202)

/// Complexity attribution analysis for source-level insights.
///
/// Maps complexity metrics back to specific code constructs, identifying
/// whether complexity comes from logical structure, formatting artifacts,
/// or recognized patterns.
pub mod attribution;
/// Rust call graph analysis with trait dispatch and function pointer tracking.
///
/// Builds a call graph from Rust source code, handling static dispatch,
/// dynamic dispatch through trait objects, and function pointer calls.
/// Supports cross-module dependency tracking and dead code analysis.
pub mod call_graph;
/// Context detection for function and method analysis.
///
/// Identifies the execution context of functions, such as async, test,
/// main entry point, or callback contexts, for more accurate analysis.
pub mod context_detection;
/// Data flow analysis for Rust code.
///
/// Provides control flow graph construction and reaching definitions analysis
/// for tracking variable definitions, uses, and function purity.
pub mod data_flow;
/// Diagnostic reporting for complexity analysis.
///
/// Generates detailed reports with summaries, attributions, recommendations,
/// and comparative analysis for multi-pass complexity results.
pub mod diagnostics;
/// Effect utilities for analysis modules.
///
/// Provides effect wrappers enabling testable, composable analysis operations
/// while maintaining compatibility with existing `anyhow::Result` based code.
pub mod effects;
/// File-level context detection for analysis.
///
/// Detects whether a file is a test file, benchmark, or production code
/// based on path patterns, content analysis, and project structure.
pub mod file_context;
/// Framework pattern detection for Rust applications.
///
/// Recognizes common framework patterns like Actix-web handlers, Tokio async
/// patterns, and other framework-specific constructs for context-aware analysis.
pub mod framework_patterns;
/// Multi-language framework pattern detection.
///
/// Extends framework detection to JavaScript, TypeScript, and Python frameworks
/// like React, Express, Django, and FastAPI.
pub mod framework_patterns_multi;
/// Function body visitor for statement analysis.
///
/// Provides utilities for traversing and classifying statements within
/// function bodies, useful for complexity analysis and pattern detection.
pub mod function_visitor;
/// Functional composition analysis.
///
/// Detects iterator chains, method pipelines, and functional programming
/// patterns. Analyzes purity and identifies terminal operations.
pub mod functional_composition;
/// Call graph metrics and pattern detection.
///
/// Computes graph-theoretic metrics like betweenness centrality and
/// clustering coefficient. Detects architectural patterns for responsibility
/// classification.
pub mod graph_metrics;
/// I/O and side effect detection.
///
/// Identifies I/O operations (file, network, database, environment access)
/// and classifies function responsibility based on side effect profiles.
pub mod io_detection;
/// Module structure analysis.
///
/// Analyzes module organization, component coupling, and provides
/// recommendations for splitting large or poorly-organized modules.
pub mod module_structure;
/// Multi-pass complexity analysis engine.
///
/// Performs raw and normalized complexity analysis with attribution,
/// generating comprehensive diagnostic reports with recommendations.
pub mod multi_pass;
/// Effect-based multi-pass analysis.
///
/// Wraps multi-pass analysis with effect handlers for testable,
/// composable workflows that separate pure computation from I/O.
pub mod multi_pass_effects;
/// Multi-signal responsibility aggregation.
///
/// Combines signals from I/O detection, graph metrics, purity analysis,
/// and type signatures for high-accuracy responsibility classification.
pub mod multi_signal_aggregation;
/// Design pattern recognition.
///
/// Detects common design patterns like Observer, Factory, and Callback
/// patterns for context-aware complexity scoring.
pub mod patterns;
/// Purity analysis for function classification.
///
/// Classifies functions by their purity level (pure, impure, unknown)
/// and identifies refactoring opportunities to extract pure cores.
pub mod purity_analysis;
/// Cross-function purity propagation.
///
/// Propagates purity information through the call graph, accounting
/// for transitive dependencies and caching results.
pub mod purity_propagation;
/// Rust-specific pattern detection.
///
/// Recognizes Rust idioms and patterns like `impl` blocks, trait
/// implementations, and async patterns for accurate classification.
pub mod rust_patterns;
/// Type flow tracking for analysis.
///
/// Tracks type information through expressions and statements,
/// supporting generic type resolution and collection operation detection.
pub mod type_flow_tracker;
/// Type signature-based classification.
///
/// Analyzes function signatures to extract type patterns and infer
/// responsibility categories from parameter and return types.
pub mod type_signatures;
/// Analysis workflow state machine.
///
/// Manages multi-phase analysis workflows with checkpoint/resume
/// support for long-running analysis operations.
pub mod workflow;

pub use call_graph::{
    AnalysisConfig, CrossModuleTracker, DeadCodeAnalysis, FrameworkPatternDetector,
    FunctionPointerTracker, RustCallGraph, RustCallGraphBuilder, TraitRegistry,
};
pub use context_detection::{ContextAnalysis, ContextDetector, FunctionContext};
pub use data_flow::{ControlFlowGraph, DataFlowAnalysis, VarId};
pub use file_context::{FileContext, FileContextDetector, TestFileConfidence};
pub use framework_patterns::{
    CustomPattern, FrameworkPattern as NewFrameworkPattern, FrameworkPatternRegistry, FrameworkType,
};
pub use framework_patterns_multi::{
    FrameworkDetector as MultiLangFrameworkDetector, FrameworkMatch, Language as FrameworkLanguage,
    PatternMatcher as MultiLangPatternMatcher,
};
pub use functional_composition::{
    analyze_composition, analyze_purity, detect_pipelines, score_composition, CompositionMetrics,
    FunctionalAnalysisConfig, Pipeline, PipelineStage, PurityMetrics, SideEffectKind, TerminalOp,
};
pub use graph_metrics::{
    compute_betweenness_centrality, compute_clustering_coefficient,
    compute_depth_from_entry_points, CallGraphPattern, GraphMetrics,
    PatternDetector as CallGraphPatternDetector, ResponsibilityClassification,
};
pub use io_detection::{
    CollectionOp as IoCollectionOp, IoDetector, IoOperation, IoPatternSet, IoProfile, Language,
    OutputStream, QueryType, Responsibility, SideEffect,
};
pub use module_structure::{
    ComponentCouplingAnalysis, ComponentDependencyGraph, Difficulty, FunctionCounts, FunctionGroup,
    ModuleComponent, ModuleFacadeInfo, ModuleStructure, ModuleStructureAnalyzer,
    OrganizationQuality, PathDeclaration, SplitRecommendation,
};
pub use multi_signal_aggregation::{
    AggregatedClassification, AggregationConfig, ConflictResolutionStrategy,
    ResponsibilityAggregator, ResponsibilityCategory, SignalEvidence, SignalSet, SignalType,
    SignalWeights,
};
pub use patterns::{
    callback::CallbackPatternRecognizer, factory::FactoryPatternRecognizer,
    observer::ObserverPatternRecognizer, Implementation, PatternDetector, PatternInstance,
    PatternRecognizer, PatternType, UsageSite,
};
pub use purity_analysis::{
    EffortLevel, PurityAnalysis, PurityAnalyzer, PurityLevel, PurityRefactoringOpportunity,
    PurityViolation, RefactoringType,
};
pub use purity_propagation::{
    PurityCache, PurityCallGraphAdapter, PurityPropagator, PurityReason, PurityResult,
};
pub use rust_patterns::{
    ImplContext, RustFunctionContext, RustPattern, RustPatternDetector, RustPatternResult,
    RustSpecificClassification,
};
pub use type_flow_tracker::{CollectionOp, Location, TypeFlowTracker, TypeId, TypeInfo};
pub use type_signatures::{
    extract_rust_signature, CanonicalType, GenericBound, Parameter, TypeBasedClassification,
    TypeMatcher, TypeNormalizer, TypePattern, TypePatternLibrary, TypeSignature,
    TypeSignatureAnalyzer,
};
pub use workflow::{
    load_checkpoint, run_analysis, save_checkpoint, AnalysisConfig as WorkflowConfig, AnalysisEnv,
    AnalysisPhase, AnalysisResults as WorkflowResults, AnalysisState, FileSystem, ProgressReporter,
    RealAnalysisEnv, WorkflowRunner,
};

#[cfg(test)]
mod effects_tests;
