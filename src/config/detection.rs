use serde::{Deserialize, Serialize};

/// Orchestrator detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorDetectionConfig {
    /// Maximum cyclomatic complexity for orchestrators (default: 5)
    /// Allows for error handling patterns (Result chains)
    #[serde(default = "default_orchestrator_max_cyclomatic")]
    pub max_cyclomatic: u32,

    /// Minimum delegation ratio (default: 0.2)
    /// Percentage of statements that are function calls
    #[serde(default = "default_orchestrator_min_delegation_ratio")]
    pub min_delegation_ratio: f64,

    /// Minimum meaningful callees (default: 2)
    /// Must coordinate at least 2 non-stdlib functions
    #[serde(default = "default_orchestrator_min_meaningful_callees")]
    pub min_meaningful_callees: usize,

    /// Cognitive complexity weight (default: 0.7)
    /// Weight given to cognitive vs cyclomatic complexity for orchestrators
    #[serde(default = "default_orchestrator_cognitive_weight")]
    pub cognitive_weight: f64,
}

impl Default for OrchestratorDetectionConfig {
    fn default() -> Self {
        Self {
            max_cyclomatic: default_orchestrator_max_cyclomatic(),
            min_delegation_ratio: default_orchestrator_min_delegation_ratio(),
            min_meaningful_callees: default_orchestrator_min_meaningful_callees(),
            cognitive_weight: default_orchestrator_cognitive_weight(),
        }
    }
}

fn default_orchestrator_max_cyclomatic() -> u32 {
    5 // Allow complexity up to 5 for error handling
}

fn default_orchestrator_min_delegation_ratio() -> f64 {
    0.2 // 20% of statements should be function calls
}

fn default_orchestrator_min_meaningful_callees() -> usize {
    2 // Must coordinate at least 2 functions
}

fn default_orchestrator_cognitive_weight() -> f64 {
    0.7 // 70% weight for cognitive complexity in orchestrators
}

/// Constructor detection configuration (spec 117, enhanced by spec 122)
/// Also used for enum converter detection (spec 124)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorDetectionConfig {
    /// Name patterns for constructor functions
    #[serde(default = "default_constructor_patterns")]
    pub patterns: Vec<String>,

    /// Maximum cyclomatic complexity for simple constructors (default: 2)
    #[serde(default = "default_constructor_max_cyclomatic")]
    pub max_cyclomatic: u32,

    /// Maximum cognitive complexity for simple constructors and enum converters (default: 3)
    /// Used by spec 124 to filter enum converters
    #[serde(default = "default_constructor_max_cognitive")]
    pub max_cognitive: u32,

    /// Maximum lines for simple constructors (default: 15)
    #[serde(default = "default_constructor_max_length")]
    pub max_length: usize,

    /// Maximum nesting depth for simple constructors (default: 1)
    #[serde(default = "default_constructor_max_nesting")]
    pub max_nesting: u32,

    /// Enable AST-based constructor detection (spec 122)
    /// When enabled, analyzes function return types and body patterns
    /// to detect non-standard constructors (default: true)
    #[serde(default = "default_constructor_ast_detection")]
    pub ast_detection: bool,
}

impl Default for ConstructorDetectionConfig {
    fn default() -> Self {
        Self {
            patterns: default_constructor_patterns(),
            max_cyclomatic: default_constructor_max_cyclomatic(),
            max_cognitive: default_constructor_max_cognitive(),
            max_length: default_constructor_max_length(),
            max_nesting: default_constructor_max_nesting(),
            ast_detection: default_constructor_ast_detection(),
        }
    }
}

fn default_constructor_patterns() -> Vec<String> {
    vec![
        "new".to_string(),
        "default".to_string(),
        "from_".to_string(),
        "with_".to_string(),
        "create_".to_string(),
        "make_".to_string(),
        "build_".to_string(),
        "of_".to_string(),
        "empty".to_string(),
        "zero".to_string(),
        "any".to_string(),
    ]
}

fn default_constructor_max_cyclomatic() -> u32 {
    2
}

fn default_constructor_max_cognitive() -> u32 {
    3
}

fn default_constructor_max_length() -> usize {
    15
}

fn default_constructor_max_nesting() -> u32 {
    1
}

fn default_constructor_ast_detection() -> bool {
    true
}

// Accessor detection config defaults (spec 125)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessorDetectionConfig {
    /// Enable accessor method detection
    #[serde(default = "default_accessor_enabled")]
    pub enabled: bool,

    /// Single-word accessor names
    #[serde(default = "default_accessor_single_word_patterns")]
    pub single_word_patterns: Vec<String>,

    /// Prefix patterns for accessors
    #[serde(default = "default_accessor_prefix_patterns")]
    pub prefix_patterns: Vec<String>,

    /// Maximum cyclomatic complexity
    #[serde(default = "default_accessor_max_cyclomatic")]
    pub max_cyclomatic: u32,

    /// Maximum cognitive complexity
    #[serde(default = "default_accessor_max_cognitive")]
    pub max_cognitive: u32,

    /// Maximum function length
    #[serde(default = "default_accessor_max_length")]
    pub max_length: usize,

    /// Maximum nesting depth
    #[serde(default = "default_accessor_max_nesting")]
    pub max_nesting: u32,
}

impl Default for AccessorDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: default_accessor_enabled(),
            single_word_patterns: default_accessor_single_word_patterns(),
            prefix_patterns: default_accessor_prefix_patterns(),
            max_cyclomatic: default_accessor_max_cyclomatic(),
            max_cognitive: default_accessor_max_cognitive(),
            max_length: default_accessor_max_length(),
            max_nesting: default_accessor_max_nesting(),
        }
    }
}

fn default_accessor_enabled() -> bool {
    true
}

fn default_accessor_single_word_patterns() -> Vec<String> {
    vec![
        "id".to_string(),
        "name".to_string(),
        "value".to_string(),
        "kind".to_string(),
        "type".to_string(),
        "status".to_string(),
        "code".to_string(),
        "key".to_string(),
        "index".to_string(),
    ]
}

fn default_accessor_prefix_patterns() -> Vec<String> {
    vec![
        "get_".to_string(),
        "is_".to_string(),
        "has_".to_string(),
        "can_".to_string(),
        "should_".to_string(),
        "as_".to_string(),
        "to_".to_string(),
        "into_".to_string(),
    ]
}

fn default_accessor_max_cyclomatic() -> u32 {
    2
}

fn default_accessor_max_cognitive() -> u32 {
    1
}

fn default_accessor_max_length() -> usize {
    10
}

fn default_accessor_max_nesting() -> u32 {
    1
}

// Data flow classification config (spec 126)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowClassificationConfig {
    /// Enable data flow classification (default: false - opt-in)
    #[serde(default = "default_data_flow_enabled")]
    pub enabled: bool,

    /// Minimum confidence required (0.0 - 1.0)
    #[serde(default = "default_data_flow_min_confidence")]
    pub min_confidence: f64,

    /// Minimum transformation ratio to classify as Orchestrator
    #[serde(default = "default_data_flow_min_transformation_ratio")]
    pub min_transformation_ratio: f64,

    /// Maximum business logic ratio for Orchestrator classification
    #[serde(default = "default_data_flow_max_business_logic_ratio")]
    pub max_business_logic_ratio: f64,
}

impl Default for DataFlowClassificationConfig {
    fn default() -> Self {
        Self {
            enabled: default_data_flow_enabled(),
            min_confidence: default_data_flow_min_confidence(),
            min_transformation_ratio: default_data_flow_min_transformation_ratio(),
            max_business_logic_ratio: default_data_flow_max_business_logic_ratio(),
        }
    }
}

fn default_data_flow_enabled() -> bool {
    false // OPT-IN (spec 126)
}

fn default_data_flow_min_confidence() -> f64 {
    0.8
}

fn default_data_flow_min_transformation_ratio() -> f64 {
    0.7
}

fn default_data_flow_max_business_logic_ratio() -> f64 {
    0.3
}

/// Error handling configuration for pattern detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandlingConfig {
    /// Enable async error detection
    #[serde(default = "default_detect_async_errors")]
    pub detect_async_errors: bool,

    /// Enable error context loss detection
    #[serde(default = "default_detect_context_loss")]
    pub detect_context_loss: bool,

    /// Enable error propagation analysis
    #[serde(default = "default_detect_propagation")]
    pub detect_propagation: bool,

    /// Enable panic pattern detection
    #[serde(default = "default_detect_panic_patterns")]
    pub detect_panic_patterns: bool,

    /// Enable error swallowing detection
    #[serde(default = "default_detect_swallowing")]
    pub detect_swallowing: bool,

    /// Custom error patterns to detect
    #[serde(default)]
    pub custom_patterns: Vec<ErrorPatternConfig>,

    /// Severity overrides for specific patterns
    #[serde(default)]
    pub severity_overrides: Vec<SeverityOverride>,
}

impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        Self {
            detect_async_errors: default_detect_async_errors(),
            detect_context_loss: default_detect_context_loss(),
            detect_propagation: default_detect_propagation(),
            detect_panic_patterns: default_detect_panic_patterns(),
            detect_swallowing: default_detect_swallowing(),
            custom_patterns: Vec::new(),
            severity_overrides: Vec::new(),
        }
    }
}

fn default_detect_async_errors() -> bool {
    true
}

fn default_detect_context_loss() -> bool {
    true
}

fn default_detect_propagation() -> bool {
    true
}

fn default_detect_panic_patterns() -> bool {
    true
}

fn default_detect_swallowing() -> bool {
    true
}

/// Custom error pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPatternConfig {
    /// Pattern name/identifier
    pub name: String,

    /// Pattern regex or matcher
    pub pattern: String,

    /// Pattern type (function_name, macro_name, method_call, etc.)
    pub pattern_type: String,

    /// Severity level (low, medium, high, critical)
    pub severity: String,

    /// Description of what this pattern detects
    pub description: String,

    /// Suggested remediation
    pub remediation: Option<String>,
}

/// Severity override for specific patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityOverride {
    /// Pattern to match (e.g., "unwrap", "panic", "todo")
    pub pattern: String,

    /// Context where override applies (e.g., "test", "benchmark", "example")
    pub context: String,

    /// New severity level
    pub severity: String,
}
