use crate::common::SourceLocation;
use syn;

pub mod complexity_weighting;
pub mod god_object_analysis;
pub mod god_object_metrics;
pub mod purity_analyzer;

pub use god_object_analysis::{
    calculate_god_object_score, calculate_god_object_score_weighted, determine_confidence,
    group_methods_by_responsibility, recommend_module_splits, GodObjectAnalysis,
    GodObjectConfidence, GodObjectThresholds, ModuleSplit, PurityDistribution,
};

pub use god_object_metrics::{
    FileMetricHistory, FileTrend, GodObjectMetrics, GodObjectSnapshot, MetricsSummary,
    TrendDirection,
};

pub use complexity_weighting::{
    aggregate_weighted_complexity, calculate_avg_complexity, calculate_complexity_penalty,
    calculate_complexity_weight, ComplexityWeight, ComplexityWeightedAnalysis,
    FunctionComplexityInfo,
};

pub use purity_analyzer::{PurityAnalyzer, PurityIndicators, PurityLevel};

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

pub mod python;

pub use feature_envy_detector::FeatureEnvyDetector;
pub use god_object_detector::GodObjectDetector;
pub use magic_value_detector::MagicValueDetector;
pub use parameter_analyzer::ParameterAnalyzer;
pub use primitive_obsession_detector::PrimitiveObsessionDetector;
