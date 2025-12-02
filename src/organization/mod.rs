use crate::common::SourceLocation;
use syn;

pub mod anti_pattern_detector;
pub mod architecture_utils;
pub mod behavioral_decomposition;
pub mod codebase_type_analyzer;
pub mod confidence;
pub mod data_flow_analyzer;
pub use behavioral_decomposition::{
    apply_hybrid_clustering, apply_production_ready_clustering,
    build_method_call_adjacency_matrix_with_functions, cluster_methods_by_behavior,
    suggest_trait_extraction, BehaviorCategory, BehavioralCategorizer, FieldAccessStats,
    FieldAccessTracker, MethodCluster,
};
pub mod boilerplate_detector;
pub mod builder_pattern;
pub mod call_graph_cohesion;
pub mod class_ownership;
pub mod clustering;
pub mod cohesion_calculator;
pub mod cohesion_priority;
pub mod complexity_weighting;
pub mod cycle_detector;
pub mod dependency_analyzer;
pub mod domain_classifier;
pub mod domain_diversity;
pub mod domain_patterns;
pub mod file_classifier;
pub mod god_object;
pub mod god_object_analysis;
pub mod god_object_metrics;
pub mod hidden_type_extractor;
pub mod integrated_analyzer;
pub mod language;
pub mod macro_recommendations;
pub mod module_function_classifier;
pub mod parallel_execution_pattern;
pub mod purity_analyzer;
pub mod registry_pattern;
pub mod split_validator;
pub mod struct_initialization;
pub mod struct_ownership;
pub mod trait_pattern_analyzer;
pub mod type_based_clustering;

pub use god_object_analysis::{
    calculate_domain_diversity_from_structs, calculate_god_object_score,
    calculate_god_object_score_weighted, calculate_struct_ratio, count_distinct_domains,
    determine_confidence, determine_cross_domain_severity, group_methods_by_responsibility,
    group_methods_by_responsibility_with_domain_patterns, infer_responsibility_with_confidence,
    recommend_module_splits, recommend_module_splits_enhanced,
    recommend_module_splits_with_evidence, suggest_module_splits_by_domain,
    suggest_splits_by_struct_grouping, ClassificationResult, DetectionType,
    EnhancedGodObjectAnalysis, GodObjectAnalysis, GodObjectConfidence, GodObjectThresholds,
    GodObjectType, ModuleSplit, Priority, PurityDistribution, RecommendationSeverity, SignalType,
    SplitAnalysisMethod, StageType, StructMetrics, StructWithMethods,
};

pub use domain_classifier::classify_struct_domain_enhanced;
pub use domain_diversity::{
    CrossDomainSeverity, DiversityScore, DomainDiversityMetrics, StructDomainClassification,
};
pub use split_validator::{
    validate_and_refine_splits, validate_and_refine_splits_with_config, SplitSizeConfig,
};
pub use struct_ownership::StructOwnershipAnalyzer;

pub use god_object_metrics::{
    FileMetricHistory, FileTrend, GodObjectMetrics, GodObjectSnapshot, MetricsSummary,
    TrendDirection,
};

pub use complexity_weighting::{
    aggregate_weighted_complexity, calculate_avg_complexity, calculate_complexity_penalty,
    calculate_complexity_weight, ComplexityWeight, ComplexityWeightedAnalysis,
    FunctionComplexityInfo,
};

pub use confidence::{
    emit_classification_metrics, ClassificationMetrics, MINIMUM_CONFIDENCE, MIN_METHODS_FOR_SPLIT,
    MODULE_SPLIT_CONFIDENCE, UTILITIES_THRESHOLD,
};

pub use purity_analyzer::{PurityAnalyzer, PurityIndicators, PurityLevel};

pub use builder_pattern::{
    adjust_builder_score, BuilderPattern, BuilderPatternDetector, MethodInfo, MethodReturnType,
};

pub use registry_pattern::{
    adjust_registry_score, RegistryPattern, RegistryPatternDetector, TraitImplInfo,
};

pub use struct_initialization::{
    FieldDependency, ReturnAnalysis, StructInitPattern, StructInitPatternDetector,
};

pub use parallel_execution_pattern::{
    adjust_parallel_score, ClosureInfo, ParallelLibrary, ParallelPattern, ParallelPatternDetector,
};

pub use boilerplate_detector::{
    BoilerplateAnalysis, BoilerplateDetectionConfig, BoilerplateDetector, BoilerplatePattern,
    DetectionSignal,
};

pub use trait_pattern_analyzer::{TraitPatternAnalyzer, TraitPatternMetrics};

pub use type_based_clustering::{
    MethodSignature, TypeAffinityAnalyzer, TypeCluster, TypeInfo, TypeSignatureAnalyzer,
};

pub use domain_patterns::{
    cluster_methods_by_domain, DomainPattern, DomainPatternDetector, DomainPatternMatch,
    PatternEvidence, DOMAIN_PATTERN_THRESHOLD, MIN_DOMAIN_CLUSTER_SIZE,
};

pub use hidden_type_extractor::{
    HiddenType, HiddenTypeConfig, HiddenTypeExtractor, MethodPurpose, ParameterClump, TupleReturn,
    TypeField, TypeMethod, Visibility,
};

pub use data_flow_analyzer::{
    DataFlowAnalyzer, PipelineStage, TransformationType, TypeFlowEdge, TypeFlowGraph,
};

pub use macro_recommendations::MacroRecommendationEngine;

pub mod module_recommendations;
pub use module_recommendations::{
    generate_decomposition_plan, is_generic_name, DecompositionLevel, DecompositionPlan,
    ModuleRecommendation,
};

pub mod semantic_naming;
pub use semantic_naming::{
    DomainTermExtractor, NameCandidate, NameUniquenessValidator, NamingStrategy, PatternRecognizer,
    SemanticNameGenerator, SpecificityScorer,
};

pub use anti_pattern_detector::{
    is_primitive, AntiPattern, AntiPatternConfig, AntiPatternDetector, AntiPatternSeverity,
    AntiPatternType, SplitQualityReport,
};

pub use integrated_analyzer::{
    AnalysisConfig, AnalysisError, AnalysisMetadata, AntiPatternReport, ConflictResolutionStrategy,
    EnabledAnalyzers, IntegratedAnalysisResult, IntegratedArchitectureAnalyzer,
};

pub use architecture_utils::{
    extract_base_type, extract_noun, is_domain_term, is_domain_type, is_likely_verb,
    is_primitive_type, most_common, to_pascal_case, to_snake_case, types_equivalent,
};

pub use codebase_type_analyzer::{
    ActionType, CodebaseAnalysisConfig, CodebaseRecommendation, CodebaseSnapshot,
    CodebaseTypeAnalysis, CodebaseTypeAnalyzer, ComplexityLevel, EffortEstimate, FileSnapshot,
    OrphanedFunctionGroup, RefactoringAction, RiskLevel, ScatteredType, ScatteringSeverity,
    UtilitiesModule,
};

pub use file_classifier::{
    calculate_reduction_target, classify_file, get_threshold, recommendation_level, ConfigType,
    FileSizeAnalysis, FileSizeThresholds, FileType, RecommendationLevel, ReductionTarget, TestType,
};

#[derive(Debug, Clone, PartialEq)]
pub enum OrganizationAntiPattern {
    GodObject {
        type_name: String,
        method_count: usize,
        field_count: usize,
        responsibility_count: usize,
        suggested_split: Vec<ResponsibilityGroup>,
        location: SourceLocation,
    },
    MagicValue {
        value_type: MagicValueType,
        value: String,
        occurrence_count: usize,
        suggested_constant_name: String,
        context: ValueContext,
        locations: Vec<SourceLocation>,
    },
    LongParameterList {
        function_name: String,
        parameter_count: usize,
        data_clumps: Vec<ParameterGroup>,
        suggested_refactoring: ParameterRefactoring,
        location: SourceLocation,
    },
    FeatureEnvy {
        method_name: String,
        envied_type: String,
        external_calls: usize,
        internal_calls: usize,
        suggested_move: bool,
        location: SourceLocation,
    },
    PrimitiveObsession {
        primitive_type: String,
        usage_context: PrimitiveUsageContext,
        occurrence_count: usize,
        suggested_domain_type: String,
        locations: Vec<SourceLocation>,
    },
    DataClump {
        parameter_group: ParameterGroup,
        occurrence_count: usize,
        suggested_struct_name: String,
        locations: Vec<SourceLocation>,
    },
    StructInitialization {
        function_name: String,
        struct_name: String,
        field_count: usize,
        cyclomatic_complexity: usize,
        field_based_complexity: f64,
        confidence: f64,
        recommendation: String,
        location: SourceLocation,
    },
}

impl OrganizationAntiPattern {
    pub fn primary_location(&self) -> &SourceLocation {
        match self {
            OrganizationAntiPattern::GodObject { location, .. } => location,
            OrganizationAntiPattern::MagicValue { locations, .. } => &locations[0],
            OrganizationAntiPattern::LongParameterList { location, .. } => location,
            OrganizationAntiPattern::FeatureEnvy { location, .. } => location,
            OrganizationAntiPattern::PrimitiveObsession { locations, .. } => &locations[0],
            OrganizationAntiPattern::DataClump { locations, .. } => &locations[0],
            OrganizationAntiPattern::StructInitialization { location, .. } => location,
        }
    }

    pub fn all_locations(&self) -> Vec<&SourceLocation> {
        match self {
            OrganizationAntiPattern::GodObject { location, .. } => vec![location],
            OrganizationAntiPattern::MagicValue { locations, .. } => locations.iter().collect(),
            OrganizationAntiPattern::LongParameterList { location, .. } => vec![location],
            OrganizationAntiPattern::FeatureEnvy { location, .. } => vec![location],
            OrganizationAntiPattern::PrimitiveObsession { locations, .. } => {
                locations.iter().collect()
            }
            OrganizationAntiPattern::DataClump { locations, .. } => locations.iter().collect(),
            OrganizationAntiPattern::StructInitialization { location, .. } => vec![location],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MagicValueType {
    NumericLiteral,
    StringLiteral,
    ArraySize,
    ConfigurationValue,
    BusinessRule,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueContext {
    Comparison,
    ArrayIndexing,
    Calculation,
    Timeout,
    BufferSize,
    BusinessLogic,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParameterRefactoring {
    ExtractStruct,
    UseBuilder,
    SplitFunction,
    UseConfiguration,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrimitiveUsageContext {
    Identifier,
    Measurement,
    Status,
    Category,
    BusinessRule,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResponsibilityGroup {
    pub name: String,
    pub methods: Vec<String>,
    pub fields: Vec<String>,
    pub responsibility: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParameterGroup {
    pub parameters: Vec<Parameter>,
    pub group_name: String,
    pub semantic_relationship: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_name: String,
    pub position: usize,
}

pub trait OrganizationDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern>;
    fn detector_name(&self) -> &'static str;
    fn estimate_maintainability_impact(
        &self,
        pattern: &OrganizationAntiPattern,
    ) -> MaintainabilityImpact;
}

#[derive(Debug, Clone, PartialEq)]
pub enum MaintainabilityImpact {
    Critical,
    High,
    Medium,
    Low,
}

mod feature_envy_detector;
mod god_object_detector;
mod magic_value_detector;
mod parameter_analyzer;
mod primitive_obsession_detector;
mod struct_init_detector;

pub use feature_envy_detector::FeatureEnvyDetector;
pub use god_object_detector::GodObjectDetector;
pub use magic_value_detector::MagicValueDetector;
pub use parameter_analyzer::ParameterAnalyzer;
pub use primitive_obsession_detector::PrimitiveObsessionDetector;
pub use struct_init_detector::StructInitOrganizationDetector;

// Multi-language support exports
pub use class_ownership::{ClassOwnership, ClassOwnershipAnalyzer};
pub use language::Language;
