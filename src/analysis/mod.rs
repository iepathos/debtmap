//! Advanced Analysis Module
//!
//! This module provides advanced analysis capabilities including:
//! - Rust-specific call graph analysis with trait dispatch and function pointers
//! - Python-specific call graph analysis with instance method tracking
//! - Python type tracking and inference for improved method resolution
//! - Python-aware dead code detection with magic methods and framework patterns
//! - Framework pattern detection (Python-specific and multi-language)
//! - Design pattern recognition (Observer, Factory, Callback)
//! - Cross-module dependency tracking
//! - Multi-pass complexity analysis with attribution
//! - Diagnostic reporting and insights generation
//! - I/O and side effect detection for responsibility classification
//! - Call graph metrics and pattern detection for responsibility analysis
//! - Purity analysis for function classification and refactoring guidance
//! - Multi-signal aggregation for high-accuracy responsibility classification

pub mod attribution;
pub mod call_graph;
pub mod context_detection;
pub mod diagnostics;
pub mod framework_patterns;
pub mod framework_patterns_multi;
pub mod function_visitor;
pub mod functional_composition;
pub mod graph_metrics;
pub mod io_detection;
pub mod module_structure;
pub mod multi_pass;
pub mod multi_signal_aggregation;
pub mod patterns;
pub mod purity_analysis;
pub mod python_call_graph;
pub mod python_dead_code;
pub mod python_dead_code_enhanced;
pub mod python_imports;
pub mod python_static_errors;
pub mod python_type_tracker;
pub mod rust_patterns;
pub mod type_flow_tracker;

pub use call_graph::{
    AnalysisConfig, CrossModuleTracker, DeadCodeAnalysis, FrameworkPatternDetector,
    FunctionPointerTracker, RustCallGraph, RustCallGraphBuilder, TraitRegistry,
};
pub use context_detection::{ContextAnalysis, ContextDetector, FunctionContext};
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
    ModuleComponent, ModuleStructure, ModuleStructureAnalyzer, SplitRecommendation,
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
pub use python_dead_code::{FrameworkPattern, PythonDeadCodeDetector, RemovalConfidence};
pub use python_dead_code_enhanced::{
    DeadCodeConfidence, DeadCodeReason, DeadCodeResult, EnhancedDeadCodeAnalyzer, LiveCodeReason,
    RemovalSuggestion,
};
pub use python_imports::{
    EnhancedImportResolver, ExportedSymbol as ImportExportedSymbol, ImportGraph, ImportType,
    ModuleSymbols, ResolvedSymbol,
};
pub use python_static_errors::{
    analyze_static_errors, errors_to_debt_items, LocalSymbols, StaticAnalysisResult, StaticError,
};
pub use python_type_tracker::{
    ClassInfo, FunctionSignature, PythonType, PythonTypeTracker, TwoPassExtractor,
};
pub use rust_patterns::{
    ImplContext, RustFunctionContext, RustPattern, RustPatternDetector, RustPatternResult,
    RustSpecificClassification,
};
pub use type_flow_tracker::{CollectionOp, Location, TypeFlowTracker, TypeId, TypeInfo};
