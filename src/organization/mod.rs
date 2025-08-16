use syn;

#[derive(Debug, Clone, PartialEq)]
pub enum OrganizationAntiPattern {
    GodObject {
        type_name: String,
        method_count: usize,
        field_count: usize,
        responsibility_count: usize,
        suggested_split: Vec<ResponsibilityGroup>,
    },
    MagicValue {
        value_type: MagicValueType,
        value: String,
        occurrence_count: usize,
        suggested_constant_name: String,
        context: ValueContext,
    },
    LongParameterList {
        function_name: String,
        parameter_count: usize,
        data_clumps: Vec<ParameterGroup>,
        suggested_refactoring: ParameterRefactoring,
    },
    FeatureEnvy {
        method_name: String,
        envied_type: String,
        external_calls: usize,
        internal_calls: usize,
        suggested_move: bool,
    },
    PrimitiveObsession {
        primitive_type: String,
        usage_context: PrimitiveUsageContext,
        occurrence_count: usize,
        suggested_domain_type: String,
    },
    DataClump {
        parameter_group: ParameterGroup,
        occurrence_count: usize,
        suggested_struct_name: String,
    },
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

pub use feature_envy_detector::FeatureEnvyDetector;
pub use god_object_detector::GodObjectDetector;
pub use magic_value_detector::MagicValueDetector;
pub use parameter_analyzer::ParameterAnalyzer;
pub use primitive_obsession_detector::PrimitiveObsessionDetector;
