// Export modules for library usage
pub mod analysis;
pub mod analysis_utils;
pub mod analyzers;
pub mod cli;
pub mod common;
pub mod complexity;
pub mod config;
pub mod context;
pub mod core;
pub mod data_flow;
pub mod database;
pub mod debt;
pub mod io;
pub mod organization;
pub mod priority;
pub mod refactoring;
pub mod resource;
pub mod risk;
pub mod scoring;
pub mod security;
pub mod testing;
pub mod transformers;

// Re-export commonly used types
pub use crate::core::{
    AnalysisResults, CircularDependency, ComplexityMetrics, ComplexityReport, ComplexitySummary,
    DebtItem, DebtType, Dependency, DependencyKind, DependencyReport, DuplicationBlock,
    DuplicationLocation, FileMetrics, FunctionMetrics, Language, ModuleDependency, Priority,
    TechnicalDebtReport,
};

pub use crate::debt::{
    circular::{analyze_module_dependencies, DependencyGraph},
    coupling::{calculate_coupling_metrics, identify_coupling_issues, CouplingMetrics},
    duplication::detect_duplication,
    patterns::{
        detect_duplicate_strings, find_code_smells, find_code_smells_with_suppression,
        find_todos_and_fixmes, find_todos_and_fixmes_with_suppression,
    },
    smells::{
        analyze_function_smells, analyze_module_smells, detect_deep_nesting, detect_long_method,
        detect_long_parameter_list, CodeSmell, SmellType,
    },
    suppression::{parse_suppression_comments, SuppressionContext, SuppressionStats},
};

pub use crate::core::metrics::{
    calculate_average_complexity, count_high_complexity, find_max_complexity,
};

pub use crate::io::output::{create_writer, OutputFormat, OutputWriter};

pub use crate::analyzers::{analyze_file, get_analyzer, Analyzer};

pub use crate::risk::{
    insights::generate_risk_insights, lcov::parse_lcov_file, FunctionRisk, RiskAnalyzer,
    RiskCategory, RiskInsight, TestingRecommendation,
};

pub use crate::analysis::{
    AnalysisConfig, CrossModuleTracker, DeadCodeAnalysis, FrameworkPatternDetector,
    FunctionPointerTracker, RustCallGraph, RustCallGraphBuilder, TraitRegistry,
};
