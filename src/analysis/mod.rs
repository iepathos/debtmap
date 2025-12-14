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
//! - Dependency Structure Matrix (DSM) for architectural visualization (Spec 205)
//! - Analysis workflow state machine with checkpoint/resume support (Spec 202)

pub mod attribution;
pub mod call_graph;
pub mod context_detection;
pub mod data_flow;
pub mod diagnostics;
pub mod dsm;
pub mod effects;
pub mod file_context;
pub mod framework_patterns;
pub mod framework_patterns_multi;
pub mod function_visitor;
pub mod functional_composition;
pub mod graph_metrics;
pub mod io_detection;
pub mod module_structure;
pub mod multi_pass;
pub mod multi_pass_effects;
pub mod multi_signal_aggregation;
pub mod patterns;
pub mod purity_analysis;
pub mod purity_propagation;
pub mod rust_patterns;
pub mod type_flow_tracker;
pub mod type_signatures;
pub mod workflow;

pub use call_graph::{
    AnalysisConfig, CrossModuleTracker, DeadCodeAnalysis, FrameworkPatternDetector,
    FunctionPointerTracker, RustCallGraph, RustCallGraphBuilder, TraitRegistry,
};
pub use context_detection::{ContextAnalysis, ContextDetector, FunctionContext};
pub use data_flow::{ControlFlowGraph, DataFlowAnalysis, VarId};
pub use dsm::{CycleInfo, CycleSeverity, DependencyMatrix, DsmCell, DsmMetrics};
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
