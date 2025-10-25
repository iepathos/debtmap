use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Scoring weights configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for coverage factor (0.0-1.0)
    #[serde(default = "default_coverage_weight")]
    pub coverage: f64,

    /// Weight for complexity factor (0.0-1.0)
    #[serde(default = "default_complexity_weight")]
    pub complexity: f64,

    /// Weight for semantic factor (0.0-1.0)
    #[serde(default = "default_semantic_weight")]
    pub semantic: f64,

    /// Weight for dependency criticality factor (0.0-1.0)
    #[serde(default = "default_dependency_weight")]
    pub dependency: f64,

    /// Weight for security issues factor (0.0-1.0)
    #[serde(default = "default_security_weight")]
    pub security: f64,

    /// Weight for code organization issues factor (0.0-1.0)
    #[serde(default = "default_organization_weight")]
    pub organization: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            coverage: default_coverage_weight(),
            complexity: default_complexity_weight(),
            semantic: default_semantic_weight(),
            dependency: default_dependency_weight(),
            security: default_security_weight(),
            organization: default_organization_weight(),
        }
    }
}

impl ScoringWeights {
    // Pure function: Check if a weight is in valid range
    fn is_valid_weight(weight: f64) -> bool {
        (0.0..=1.0).contains(&weight)
    }

    // Pure function: Validate a single weight with name
    fn validate_weight(weight: f64, name: &str) -> Result<(), String> {
        if Self::is_valid_weight(weight) {
            Ok(())
        } else {
            Err(format!("{} weight must be between 0.0 and 1.0", name))
        }
    }

    // Pure function: Validate active weights sum to 1.0
    fn validate_active_weights_sum(
        coverage: f64,
        complexity: f64,
        dependency: f64,
    ) -> Result<(), String> {
        let sum = coverage + complexity + dependency;
        if (sum - 1.0).abs() > 0.001 {
            Err(format!(
                "Active scoring weights (coverage, complexity, dependency) must sum to 1.0, but sum to {:.3}",
                sum
            ))
        } else {
            Ok(())
        }
    }

    // Pure function: Collect all weight validations
    fn collect_weight_validations(&self) -> Vec<Result<(), String>> {
        vec![
            Self::validate_weight(self.coverage, "Coverage"),
            Self::validate_weight(self.complexity, "Complexity"),
            Self::validate_weight(self.semantic, "Semantic"),
            Self::validate_weight(self.dependency, "Dependency"),
            Self::validate_weight(self.security, "Security"),
            Self::validate_weight(self.organization, "Organization"),
        ]
    }

    /// Validate that weights sum to 1.0 (with small tolerance for floating point)
    pub fn validate(&self) -> Result<(), String> {
        // Validate active weights sum
        Self::validate_active_weights_sum(self.coverage, self.complexity, self.dependency)?;

        // Validate all individual weights
        for validation in self.collect_weight_validations() {
            validation?;
        }

        Ok(())
    }

    /// Normalize weights to ensure they sum to 1.0
    pub fn normalize(&mut self) {
        // Only normalize the weights we actually use
        let sum = self.coverage + self.complexity + self.dependency;
        if sum > 0.0 && (sum - 1.0).abs() > 0.001 {
            self.coverage /= sum;
            self.complexity /= sum;
            self.dependency /= sum;
            // Set unused weights to 0
            self.semantic = 0.0;
            self.security = 0.0;
            self.organization = 0.0;
        }
    }
}

// Default weights for weighted sum model - prioritizing coverage gaps
fn default_coverage_weight() -> f64 {
    0.50 // 50% weight to prioritize untested code
}
fn default_complexity_weight() -> f64 {
    0.35 // 35% weight for complexity within coverage tiers
}
fn default_semantic_weight() -> f64 {
    0.00 // Not used in weighted sum model
}
fn default_dependency_weight() -> f64 {
    0.15 // 15% weight for impact radius
}
fn default_security_weight() -> f64 {
    0.00 // Not used in weighted sum model
}
fn default_organization_weight() -> f64 {
    0.00 // Not used in weighted sum model
}

/// Role multipliers configuration for semantic classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMultipliers {
    /// Multiplier for PureLogic functions (default: 1.5)
    #[serde(default = "default_pure_logic_multiplier")]
    pub pure_logic: f64,

    /// Multiplier for Orchestrator functions (default: 0.6)
    #[serde(default = "default_orchestrator_multiplier")]
    pub orchestrator: f64,

    /// Multiplier for IOWrapper functions (default: 0.5)
    #[serde(default = "default_io_wrapper_multiplier")]
    pub io_wrapper: f64,

    /// Multiplier for EntryPoint functions (default: 0.8)
    #[serde(default = "default_entry_point_multiplier")]
    pub entry_point: f64,

    /// Multiplier for PatternMatch functions (default: 0.4)
    #[serde(default = "default_pattern_match_multiplier")]
    pub pattern_match: f64,

    /// Multiplier for Unknown functions (default: 1.0)
    #[serde(default = "default_unknown_multiplier")]
    pub unknown: f64,
}

impl Default for RoleMultipliers {
    fn default() -> Self {
        Self {
            pure_logic: default_pure_logic_multiplier(),
            orchestrator: default_orchestrator_multiplier(),
            io_wrapper: default_io_wrapper_multiplier(),
            entry_point: default_entry_point_multiplier(),
            pattern_match: default_pattern_match_multiplier(),
            unknown: default_unknown_multiplier(),
        }
    }
}

fn default_pure_logic_multiplier() -> f64 {
    1.2 // Prioritized but not extreme (was 1.5)
}

fn default_orchestrator_multiplier() -> f64 {
    0.8 // Reduced but not severely (was 0.6)
}

fn default_io_wrapper_multiplier() -> f64 {
    0.7 // Minor reduction (was 0.5)
}

fn default_entry_point_multiplier() -> f64 {
    0.9 // Slight reduction (was 0.8)
}

fn default_pattern_match_multiplier() -> f64 {
    0.6 // Moderate reduction (was 0.4)
}

fn default_unknown_multiplier() -> f64 {
    1.0 // No adjustment for unknown functions
}

/// Complexity weights configuration (spec 121)
/// Controls how cyclomatic and cognitive complexity are combined in scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityWeightsConfig {
    /// Weight for cyclomatic complexity (default: 0.3)
    #[serde(default = "default_cyclomatic_weight")]
    pub cyclomatic: f64,

    /// Weight for cognitive complexity (default: 0.7)
    #[serde(default = "default_cognitive_weight")]
    pub cognitive: f64,

    /// Maximum cyclomatic complexity for normalization (default: 50)
    #[serde(default = "default_max_cyclomatic")]
    pub max_cyclomatic: f64,

    /// Maximum cognitive complexity for normalization (default: 100)
    #[serde(default = "default_max_cognitive")]
    pub max_cognitive: f64,
}

impl Default for ComplexityWeightsConfig {
    fn default() -> Self {
        Self {
            cyclomatic: default_cyclomatic_weight(),
            cognitive: default_cognitive_weight(),
            max_cyclomatic: default_max_cyclomatic(),
            max_cognitive: default_max_cognitive(),
        }
    }
}

fn default_cyclomatic_weight() -> f64 {
    0.3 // 30% weight - cyclomatic as proxy for test cases
}

fn default_cognitive_weight() -> f64 {
    0.7 // 70% weight - cognitive correlates better with bugs
}

fn default_max_cyclomatic() -> f64 {
    50.0 // Reasonable maximum for cyclomatic complexity
}

fn default_max_cognitive() -> f64 {
    100.0 // Cognitive complexity can go higher
}

/// Role-based coverage weight multipliers for scoring adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleCoverageWeights {
    /// Coverage weight multiplier for EntryPoint functions (default: 0.6)
    #[serde(default = "default_entry_point_coverage_weight")]
    pub entry_point: f64,

    /// Coverage weight multiplier for Orchestrator functions (default: 0.8)
    #[serde(default = "default_orchestrator_coverage_weight")]
    pub orchestrator: f64,

    /// Coverage weight multiplier for PureLogic functions (default: 1.0)
    #[serde(default = "default_pure_logic_coverage_weight")]
    pub pure_logic: f64,

    /// Coverage weight multiplier for IOWrapper functions (default: 1.0)
    #[serde(default = "default_io_wrapper_coverage_weight")]
    pub io_wrapper: f64,

    /// Coverage weight multiplier for PatternMatch functions (default: 1.0)
    #[serde(default = "default_pattern_match_coverage_weight")]
    pub pattern_match: f64,

    /// Coverage weight multiplier for Unknown functions (default: 1.0)
    #[serde(default = "default_unknown_coverage_weight")]
    pub unknown: f64,
}

/// Configuration for caller/callee display in output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallerCalleeConfig {
    /// Maximum number of callers to display (default: 5)
    #[serde(default = "default_max_callers")]
    pub max_callers: usize,

    /// Maximum number of callees to display (default: 5)
    #[serde(default = "default_max_callees")]
    pub max_callees: usize,

    /// Whether to show external crate calls (default: false)
    #[serde(default = "default_show_external")]
    pub show_external: bool,

    /// Whether to show standard library calls (default: false)
    #[serde(default = "default_show_std_lib")]
    pub show_std_lib: bool,
}

impl Default for CallerCalleeConfig {
    fn default() -> Self {
        Self {
            max_callers: default_max_callers(),
            max_callees: default_max_callees(),
            show_external: default_show_external(),
            show_std_lib: default_show_std_lib(),
        }
    }
}

fn default_max_callers() -> usize {
    5
}

fn default_max_callees() -> usize {
    5
}

fn default_show_external() -> bool {
    false
}

fn default_show_std_lib() -> bool {
    false
}

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

// Pure mapping pattern detection config (spec 118)
pub use crate::complexity::pure_mapping_patterns::MappingPatternConfig;

impl Default for RoleCoverageWeights {
    fn default() -> Self {
        Self {
            entry_point: default_entry_point_coverage_weight(),
            orchestrator: default_orchestrator_coverage_weight(),
            pure_logic: default_pure_logic_coverage_weight(),
            io_wrapper: default_io_wrapper_coverage_weight(),
            pattern_match: default_pattern_match_coverage_weight(),
            unknown: default_unknown_coverage_weight(),
        }
    }
}

fn default_entry_point_coverage_weight() -> f64 {
    0.6 // Entry points are often integration tested, reduce unit coverage penalty
}

fn default_orchestrator_coverage_weight() -> f64 {
    0.8 // Orchestrators are often tested via higher-level tests
}

fn default_pure_logic_coverage_weight() -> f64 {
    1.0 // Pure logic should have unit tests, no reduction
}

fn default_io_wrapper_coverage_weight() -> f64 {
    0.5 // I/O wrappers are integration tested, reduce unit coverage penalty (spec 119)
}

fn default_pattern_match_coverage_weight() -> f64 {
    1.0 // Pattern matching should have unit tests, no reduction
}

fn default_unknown_coverage_weight() -> f64 {
    1.0 // Unknown functions get normal coverage expectations
}

/// Role multiplier clamping configuration (spec 119)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMultiplierConfig {
    /// Minimum clamp value for role multipliers (default: 0.3)
    #[serde(default = "default_role_clamp_min")]
    pub clamp_min: f64,

    /// Maximum clamp value for role multipliers (default: 1.8)
    #[serde(default = "default_role_clamp_max")]
    pub clamp_max: f64,

    /// Whether to enable clamping (default: true)
    #[serde(default = "default_enable_role_clamping")]
    pub enable_clamping: bool,
}

impl Default for RoleMultiplierConfig {
    fn default() -> Self {
        Self {
            clamp_min: default_role_clamp_min(),
            clamp_max: default_role_clamp_max(),
            enable_clamping: default_enable_role_clamping(),
        }
    }
}

fn default_role_clamp_min() -> f64 {
    0.3 // Allow IOWrapper to get 50% reduction (0.5 multiplier)
}

fn default_role_clamp_max() -> f64 {
    1.8 // Allow EntryPoint to get 50% increase (1.5 multiplier)
}

fn default_enable_role_clamping() -> bool {
    true // Enable by default
}

/// Score normalization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationConfig {
    /// Threshold for linear scaling (default: 10.0)
    #[serde(default = "default_linear_threshold")]
    pub linear_threshold: f64,

    /// Threshold for logarithmic scaling (default: 100.0)
    #[serde(default = "default_logarithmic_threshold")]
    pub logarithmic_threshold: f64,

    /// Multiplier for square root scaling (default: 3.33)
    #[serde(default = "default_sqrt_multiplier")]
    pub sqrt_multiplier: f64,

    /// Multiplier for logarithmic scaling (default: 10.0)
    #[serde(default = "default_log_multiplier")]
    pub log_multiplier: f64,

    /// Whether to show raw scores alongside normalized scores
    #[serde(default = "default_show_raw_scores")]
    pub show_raw_scores: bool,
}

impl Default for NormalizationConfig {
    fn default() -> Self {
        Self {
            linear_threshold: default_linear_threshold(),
            logarithmic_threshold: default_logarithmic_threshold(),
            sqrt_multiplier: default_sqrt_multiplier(),
            log_multiplier: default_log_multiplier(),
            show_raw_scores: default_show_raw_scores(),
        }
    }
}

fn default_linear_threshold() -> f64 {
    10.0
}

fn default_logarithmic_threshold() -> f64 {
    100.0
}

fn default_sqrt_multiplier() -> f64 {
    3.33
}

fn default_log_multiplier() -> f64 {
    10.0
}

fn default_show_raw_scores() -> bool {
    true
}

/// Context-aware detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Whether context-aware detection is enabled
    #[serde(default = "default_context_enabled")]
    pub enabled: bool,

    /// Custom context rules
    #[serde(default)]
    pub rules: Vec<ContextRuleConfig>,

    /// Function role patterns
    #[serde(default)]
    pub function_patterns: Option<FunctionPatternConfig>,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            enabled: default_context_enabled(),
            rules: Vec::new(),
            function_patterns: None,
        }
    }
}

fn default_context_enabled() -> bool {
    false // Opt-in by default
}

/// Configuration for a context rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRuleConfig {
    /// Name of the rule
    pub name: String,

    /// Pattern to match (e.g., "blocking_io", "security", "performance")
    pub pattern: String,

    /// Context matcher configuration
    pub context: ContextMatcherConfig,

    /// Action to take (allow, skip, warn, reduce_severity)
    pub action: String,

    /// Priority (higher number = higher priority)
    #[serde(default = "default_rule_priority")]
    pub priority: i32,

    /// Optional reason for the rule
    pub reason: Option<String>,
}

fn default_rule_priority() -> i32 {
    50
}

/// Context matcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMatcherConfig {
    /// Function role (main, test, handler, config_loader, etc.)
    pub role: Option<String>,

    /// File type (production, test, benchmark, example, etc.)
    pub file_type: Option<String>,

    /// Whether function is async
    pub is_async: Option<bool>,

    /// Framework pattern (rust_main, web_handler, cli_handler, etc.)
    pub framework_pattern: Option<String>,

    /// Function name pattern (regex)
    pub name_pattern: Option<String>,
}

/// Function pattern configuration for detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionPatternConfig {
    /// Additional test function patterns
    #[serde(default)]
    pub test_patterns: Vec<String>,

    /// Additional config loader patterns
    #[serde(default)]
    pub config_patterns: Vec<String>,

    /// Additional handler patterns
    #[serde(default)]
    pub handler_patterns: Vec<String>,

    /// Additional initialization patterns
    #[serde(default)]
    pub init_patterns: Vec<String>,
}

/// God object detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectConfig {
    /// Whether god object detection is enabled
    #[serde(default = "default_god_object_enabled")]
    pub enabled: bool,

    /// Language-specific thresholds
    #[serde(default)]
    pub rust: GodObjectThresholds,

    #[serde(default)]
    pub python: GodObjectThresholds,

    #[serde(default)]
    pub javascript: GodObjectThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectThresholds {
    #[serde(default = "default_max_methods")]
    pub max_methods: usize,

    #[serde(default = "default_max_fields")]
    pub max_fields: usize,

    #[serde(default = "default_max_traits")]
    pub max_traits: usize,

    #[serde(default = "default_max_lines")]
    pub max_lines: usize,

    #[serde(default = "default_max_complexity")]
    pub max_complexity: u32,
}

impl Default for GodObjectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rust: GodObjectThresholds::rust_defaults(),
            python: GodObjectThresholds::python_defaults(),
            javascript: GodObjectThresholds::javascript_defaults(),
        }
    }
}

impl Default for GodObjectThresholds {
    fn default() -> Self {
        Self {
            max_methods: default_max_methods(),
            max_fields: default_max_fields(),
            max_traits: default_max_traits(),
            max_lines: default_max_lines(),
            max_complexity: default_max_complexity(),
        }
    }
}

impl GodObjectThresholds {
    fn rust_defaults() -> Self {
        Self {
            max_methods: 20,
            max_fields: 15,
            max_traits: 5,
            max_lines: 1000,
            max_complexity: 200,
        }
    }

    fn python_defaults() -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_traits: 3,
            max_lines: 500,
            max_complexity: 150,
        }
    }

    fn javascript_defaults() -> Self {
        Self {
            max_methods: 15,
            max_fields: 20,
            max_traits: 3,
            max_lines: 500,
            max_complexity: 150,
        }
    }
}

fn default_god_object_enabled() -> bool {
    true
}
fn default_max_methods() -> usize {
    20
}
fn default_max_fields() -> usize {
    15
}
fn default_max_traits() -> usize {
    5
}
fn default_max_lines() -> usize {
    1000
}
fn default_max_complexity() -> u32 {
    200
}

/// Display configuration for output formatting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Whether to use tiered priority display
    #[serde(default = "default_tiered_display")]
    pub tiered: bool,

    /// Maximum items to show per tier (default: 5)
    #[serde(default = "default_items_per_tier")]
    pub items_per_tier: usize,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            tiered: default_tiered_display(),
            items_per_tier: default_items_per_tier(),
        }
    }
}

fn default_tiered_display() -> bool {
    true // Enable tiered display by default
}

fn default_items_per_tier() -> usize {
    5
}

/// Root configuration structure for debtmap
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebtmapConfig {
    /// Scoring weights configuration
    #[serde(default)]
    pub scoring: Option<ScoringWeights>,

    /// Display configuration for output formatting
    #[serde(default)]
    pub display: Option<DisplayConfig>,

    /// External API detection configuration
    #[serde(default)]
    pub external_api: Option<crate::priority::external_api_detector::ExternalApiConfig>,

    /// God object detection configuration
    #[serde(default)]
    pub god_object_detection: Option<GodObjectConfig>,

    /// Thresholds configuration
    #[serde(default)]
    pub thresholds: Option<ThresholdsConfig>,

    /// Language configuration
    #[serde(default)]
    pub languages: Option<LanguagesConfig>,

    /// Ignore patterns
    #[serde(default)]
    pub ignore: Option<IgnoreConfig>,

    /// Output configuration
    #[serde(default)]
    pub output: Option<OutputConfig>,

    /// Context-aware detection configuration
    #[serde(default)]
    pub context: Option<ContextConfig>,

    /// Entropy-based complexity scoring configuration
    #[serde(default)]
    pub entropy: Option<EntropyConfig>,

    /// Role multipliers for semantic classification
    #[serde(default)]
    pub role_multipliers: Option<RoleMultipliers>,

    /// Complexity thresholds for enhanced detection
    #[serde(default)]
    pub complexity_thresholds: Option<crate::complexity::threshold_manager::ComplexityThresholds>,

    /// Error handling detection configuration
    #[serde(default)]
    pub error_handling: Option<ErrorHandlingConfig>,

    /// Score normalization configuration
    #[serde(default)]
    pub normalization: Option<NormalizationConfig>,

    /// Lines of code counting configuration
    #[serde(default)]
    pub loc: Option<crate::metrics::LocCountingConfig>,

    /// Tier configuration for prioritization
    #[serde(default)]
    pub tiers: Option<crate::priority::TierConfig>,

    /// Role-based coverage weight multipliers
    #[serde(default)]
    pub role_coverage_weights: Option<RoleCoverageWeights>,

    /// Role multiplier clamping configuration (spec 119)
    #[serde(default)]
    pub role_multiplier_config: Option<RoleMultiplierConfig>,

    /// Orchestrator detection configuration
    #[serde(default)]
    pub orchestrator_detection: Option<OrchestratorDetectionConfig>,

    /// Orchestration score adjustment configuration (spec 110)
    #[serde(default)]
    pub orchestration_adjustment:
        Option<crate::priority::scoring::orchestration_adjustment::OrchestrationAdjustmentConfig>,

    /// Constructor detection configuration (spec 117)
    #[serde(default, rename = "classification")]
    pub classification: Option<ClassificationConfig>,

    /// Pure mapping pattern detection configuration (spec 118)
    #[serde(default)]
    pub mapping_patterns: Option<MappingPatternConfig>,

    /// Complexity weights configuration (spec 121)
    #[serde(default)]
    pub complexity_weights: Option<ComplexityWeightsConfig>,
}

/// Classification configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassificationConfig {
    /// Constructor detection configuration
    #[serde(default)]
    pub constructors: Option<ConstructorDetectionConfig>,

    /// Accessor detection configuration (spec 125)
    #[serde(default)]
    pub accessors: Option<AccessorDetectionConfig>,

    /// Data flow classification configuration (spec 126)
    #[serde(default)]
    pub data_flow: Option<DataFlowClassificationConfig>,
}

impl DebtmapConfig {
    /// Get ignore patterns from configuration
    ///
    /// Returns a vector of glob patterns that should be excluded from analysis.
    /// If no configuration is found or no patterns are specified, returns an empty vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::config::DebtmapConfig;
    /// let config = DebtmapConfig::default();
    /// let patterns = config.get_ignore_patterns();
    /// // patterns might contain ["tests/**/*", "*.test.rs"]
    /// ```
    pub fn get_ignore_patterns(&self) -> Vec<String> {
        self.ignore
            .as_ref()
            .map(|ig| ig.patterns.clone())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdsConfig {
    pub complexity: Option<u32>,
    pub duplication: Option<u32>,
    pub max_file_length: Option<usize>,
    pub max_function_length: Option<usize>,

    /// Minimum thresholds for including items in debt analysis
    #[serde(default)]
    pub minimum_debt_score: Option<f64>,

    /// Minimum complexity thresholds for considering something as debt
    #[serde(default)]
    pub minimum_cyclomatic_complexity: Option<u32>,
    #[serde(default)]
    pub minimum_cognitive_complexity: Option<u32>,

    /// Minimum risk score for including items (0.0-10.0)
    #[serde(default)]
    pub minimum_risk_score: Option<f64>,

    /// Validation thresholds - used by `debtmap validate` command
    #[serde(default)]
    pub validation: Option<ValidationThresholds>,
}

/// Validation thresholds for the validate command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationThresholds {
    /// Maximum allowed average complexity (default: 10.0)
    #[serde(default = "default_max_avg_complexity")]
    pub max_average_complexity: f64,

    /// Maximum allowed high complexity function count (default: 100)
    #[serde(default = "default_max_high_complexity_count")]
    pub max_high_complexity_count: usize,

    /// Maximum allowed technical debt items (default: 2000)
    #[serde(default = "default_max_debt_items")]
    pub max_debt_items: usize,

    /// Maximum allowed total debt score (default: 1000)
    /// Note: Uses unified scoring where each item is capped at 10.0
    #[serde(default = "default_max_total_debt_score")]
    pub max_total_debt_score: u32,

    /// Maximum allowed codebase risk score (default: 7.0)
    #[serde(default = "default_max_codebase_risk")]
    pub max_codebase_risk_score: f64,

    /// Maximum allowed high-risk function count (default: 50)
    #[serde(default = "default_max_high_risk_count")]
    pub max_high_risk_functions: usize,

    /// Minimum required code coverage percentage (default: 0.0 - no minimum)
    #[serde(default = "default_min_coverage")]
    pub min_coverage_percentage: f64,

    /// Maximum allowed debt density per 1000 LOC (default: 50.0)
    #[serde(default = "default_max_debt_density")]
    pub max_debt_density: f64,
}

impl Default for ValidationThresholds {
    fn default() -> Self {
        Self {
            max_average_complexity: default_max_avg_complexity(),
            max_high_complexity_count: default_max_high_complexity_count(),
            max_debt_items: default_max_debt_items(),
            max_total_debt_score: default_max_total_debt_score(),
            max_codebase_risk_score: default_max_codebase_risk(),
            max_high_risk_functions: default_max_high_risk_count(),
            min_coverage_percentage: default_min_coverage(),
            max_debt_density: default_max_debt_density(),
        }
    }
}

// Default validation threshold values
fn default_max_avg_complexity() -> f64 {
    10.0
}
fn default_max_high_complexity_count() -> usize {
    100
}
fn default_max_debt_items() -> usize {
    2000
}
fn default_max_total_debt_score() -> u32 {
    1000 // Unified scoring: each item capped at 10.0, so ~100 high-priority items
}
fn default_max_codebase_risk() -> f64 {
    7.0
}
fn default_max_high_risk_count() -> usize {
    50
}
fn default_min_coverage() -> f64 {
    0.0
}
fn default_max_debt_density() -> f64 {
    50.0 // 50 debt points per 1000 LOC - reasonable default for most projects
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguagesConfig {
    pub enabled: Vec<String>,

    /// Rust-specific configuration
    #[serde(default)]
    pub rust: Option<LanguageFeatures>,

    /// Python-specific configuration
    #[serde(default)]
    pub python: Option<LanguageFeatures>,

    /// JavaScript-specific configuration
    #[serde(default)]
    pub javascript: Option<LanguageFeatures>,

    /// TypeScript-specific configuration
    #[serde(default)]
    pub typescript: Option<LanguageFeatures>,
}

/// Language-specific feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageFeatures {
    /// Whether to detect dead code for this language
    #[serde(default = "default_detect_dead_code")]
    pub detect_dead_code: bool,

    /// Whether to detect complexity issues for this language
    #[serde(default = "default_detect_complexity")]
    pub detect_complexity: bool,

    /// Whether to detect duplication for this language
    #[serde(default = "default_detect_duplication")]
    pub detect_duplication: bool,
}

impl Default for LanguageFeatures {
    fn default() -> Self {
        Self {
            detect_dead_code: default_detect_dead_code(),
            detect_complexity: default_detect_complexity(),
            detect_duplication: default_detect_duplication(),
        }
    }
}

// Default feature flags - all enabled except Rust dead code detection
fn default_detect_dead_code() -> bool {
    true // Will be overridden for Rust
}

fn default_detect_complexity() -> bool {
    true
}

fn default_detect_duplication() -> bool {
    true
}

/// Entropy-based complexity scoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyConfig {
    /// Whether entropy-based scoring is enabled
    #[serde(default = "default_entropy_enabled")]
    pub enabled: bool,

    /// Weight of entropy in complexity adjustment (0.0-1.0)
    #[serde(default = "default_entropy_weight")]
    pub weight: f64,

    /// Minimum tokens required for entropy calculation
    #[serde(default = "default_entropy_min_tokens")]
    pub min_tokens: usize,

    /// Pattern similarity threshold for repetition detection (0.0-1.0)
    #[serde(default = "default_entropy_pattern_threshold")]
    pub pattern_threshold: f64,

    /// Entropy threshold for low entropy detection (0.0-1.0)
    #[serde(default = "default_entropy_threshold")]
    pub entropy_threshold: f64,

    /// Whether to use smarter token classification (false by default for backward compatibility)
    #[serde(default)]
    pub use_classification: Option<bool>,

    /// Branch similarity threshold for detection (0.0-1.0)
    #[serde(default = "default_branch_threshold")]
    pub branch_threshold: f64,

    /// Maximum reduction for high repetition (0.0-1.0)
    #[serde(default = "default_max_repetition_reduction")]
    pub max_repetition_reduction: f64,

    /// Maximum reduction for low entropy (0.0-1.0)
    #[serde(default = "default_max_entropy_reduction")]
    pub max_entropy_reduction: f64,

    /// Maximum reduction for similar branches (0.0-1.0)
    #[serde(default = "default_max_branch_reduction")]
    pub max_branch_reduction: f64,

    /// Maximum combined reduction (0.0-1.0)
    #[serde(default = "default_max_combined_reduction")]
    pub max_combined_reduction: f64,
}

impl Default for EntropyConfig {
    fn default() -> Self {
        Self {
            enabled: default_entropy_enabled(),
            weight: default_entropy_weight(),
            min_tokens: default_entropy_min_tokens(),
            pattern_threshold: default_entropy_pattern_threshold(),
            entropy_threshold: default_entropy_threshold(),
            use_classification: None, // Default to None for backward compatibility
            branch_threshold: default_branch_threshold(),
            max_repetition_reduction: default_max_repetition_reduction(),
            max_entropy_reduction: default_max_entropy_reduction(),
            max_branch_reduction: default_max_branch_reduction(),
            max_combined_reduction: default_max_combined_reduction(),
        }
    }
}

fn default_entropy_enabled() -> bool {
    true // Enabled by default for better match statement handling
}

fn default_entropy_weight() -> f64 {
    1.0 // Full weight when enabled (user can adjust)
}

fn default_entropy_min_tokens() -> usize {
    20
}

fn default_entropy_pattern_threshold() -> f64 {
    0.7
}

fn default_entropy_threshold() -> f64 {
    0.4 // Below 0.4 entropy is considered low
}

fn default_branch_threshold() -> f64 {
    0.8 // Above 80% branch similarity triggers dampening
}

fn default_max_repetition_reduction() -> f64 {
    0.20 // Max 20% reduction for high repetition
}

fn default_max_entropy_reduction() -> f64 {
    0.15 // Max 15% reduction for low entropy
}

fn default_max_branch_reduction() -> f64 {
    0.25 // Max 25% reduction for similar branches
}

fn default_max_combined_reduction() -> f64 {
    0.30 // Max 30% total reduction (cap)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub default_format: Option<String>,
}

/// Cache the configuration
static CONFIG: OnceLock<DebtmapConfig> = OnceLock::new();
static SCORING_WEIGHTS: OnceLock<ScoringWeights> = OnceLock::new();

/// Load configuration from .debtmap.toml if it exists
/// Pure function to read and parse config file contents
fn read_config_file(path: &Path) -> Result<String, std::io::Error> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Pure function to parse and validate config from TOML string
#[cfg(test)]
pub(crate) fn parse_and_validate_config(contents: &str) -> Result<DebtmapConfig, String> {
    parse_and_validate_config_impl(contents)
}

fn parse_and_validate_config_impl(contents: &str) -> Result<DebtmapConfig, String> {
    let mut config = toml::from_str::<DebtmapConfig>(contents)
        .map_err(|e| format!("Failed to parse .debtmap.toml: {}", e))?;

    // Validate and normalize scoring weights if present
    if let Some(ref mut scoring) = config.scoring {
        if let Err(e) = scoring.validate() {
            eprintln!("Warning: Invalid scoring weights: {}. Using defaults.", e);
            config.scoring = Some(ScoringWeights::default());
        } else {
            scoring.normalize(); // Ensure exact sum of 1.0
        }
    }

    Ok(config)
}

/// Pure function to try loading config from a specific path
fn try_load_config_from_path(config_path: &Path) -> Option<DebtmapConfig> {
    let contents = match read_config_file(config_path) {
        Ok(contents) => contents,
        Err(e) => {
            handle_read_error(config_path, &e);
            return None;
        }
    };

    match parse_and_validate_config_impl(&contents) {
        Ok(config) => {
            log::debug!("Loaded config from {}", config_path.display());
            Some(config)
        }
        Err(e) => {
            eprintln!("Warning: {}. Using defaults.", e);
            None
        }
    }
}

/// Handle file read errors with appropriate logging
fn handle_read_error(config_path: &Path, error: &std::io::Error) {
    // Only log actual errors, not "file not found"
    if error.kind() != std::io::ErrorKind::NotFound {
        log::warn!(
            "Failed to read config file {}: {}",
            config_path.display(),
            error
        );
    }
}

/// Pure function to generate directory ancestors up to a depth limit
#[cfg(test)]
pub(crate) fn directory_ancestors(
    start: PathBuf,
    max_depth: usize,
) -> impl Iterator<Item = PathBuf> {
    directory_ancestors_impl(start, max_depth)
}

fn directory_ancestors_impl(start: PathBuf, max_depth: usize) -> impl Iterator<Item = PathBuf> {
    std::iter::successors(Some(start), |dir| {
        let mut parent = dir.clone();
        if parent.pop() {
            Some(parent)
        } else {
            None
        }
    })
    .take(max_depth)
}

pub fn load_config() -> DebtmapConfig {
    const MAX_TRAVERSAL_DEPTH: usize = 10;

    // Get current directory or return default
    let current = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!(
                "Failed to get current directory: {}. Using default config.",
                e
            );
            return DebtmapConfig::default();
        }
    };

    // Search for config file in directory hierarchy
    directory_ancestors_impl(current, MAX_TRAVERSAL_DEPTH)
        .map(|dir| dir.join(".debtmap.toml"))
        .find_map(|path| try_load_config_from_path(&path))
        .unwrap_or_else(|| {
            log::debug!(
                "No config found after checking {} directories. Using default config.",
                MAX_TRAVERSAL_DEPTH
            );
            DebtmapConfig::default()
        })
}

/// Get the cached configuration
pub fn get_config() -> &'static DebtmapConfig {
    CONFIG.get_or_init(load_config)
}

/// Get the configuration without panicking on errors
pub fn get_config_safe() -> Result<DebtmapConfig, std::io::Error> {
    Ok(load_config())
}

/// Get the scoring weights (with defaults if not configured)
pub fn get_scoring_weights() -> &'static ScoringWeights {
    SCORING_WEIGHTS.get_or_init(|| get_config().scoring.clone().unwrap_or_default())
}

/// Get entropy-based complexity scoring configuration
pub fn get_entropy_config() -> EntropyConfig {
    get_config().entropy.clone().unwrap_or_default()
}

/// Get role multipliers configuration
pub fn get_role_multipliers() -> RoleMultipliers {
    get_config().role_multipliers.clone().unwrap_or_default()
}

/// Get minimum debt score threshold (default: 1.0)
pub fn get_minimum_debt_score() -> f64 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_debt_score)
        .unwrap_or(1.0)
}

/// Get minimum cyclomatic complexity threshold (default: 2)
pub fn get_minimum_cyclomatic_complexity() -> u32 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_cyclomatic_complexity)
        .unwrap_or(2)
}

/// Get minimum cognitive complexity threshold (default: 3)
pub fn get_minimum_cognitive_complexity() -> u32 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_cognitive_complexity)
        .unwrap_or(3)
}

/// Get minimum risk score threshold (default: 1.0)
pub fn get_minimum_risk_score() -> f64 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_risk_score)
        .unwrap_or(1.0)
}

/// Get display configuration (with defaults)
pub fn get_display_config() -> DisplayConfig {
    get_config().display.clone().unwrap_or_default()
}

/// Get validation thresholds (with defaults)
pub fn get_validation_thresholds() -> ValidationThresholds {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.validation.clone())
        .unwrap_or_default()
}

/// Get language-specific feature configuration
pub fn get_language_features(language: &crate::core::Language) -> LanguageFeatures {
    use crate::core::Language;

    let config = get_config();
    let languages_config = config.languages.as_ref();

    match language {
        Language::Rust => {
            languages_config
                .and_then(|lc| lc.rust.clone())
                .unwrap_or(LanguageFeatures {
                    detect_dead_code: false, // Rust dead code detection disabled by default
                    detect_complexity: true,
                    detect_duplication: true,
                })
        }
        Language::Python => languages_config
            .and_then(|lc| lc.python.clone())
            .unwrap_or_default(),
        Language::JavaScript => languages_config
            .and_then(|lc| lc.javascript.clone())
            .unwrap_or_default(),
        Language::TypeScript => languages_config
            .and_then(|lc| lc.typescript.clone())
            .unwrap_or_default(),
        Language::Unknown => LanguageFeatures::default(),
    }
}

/// Get complexity thresholds configuration
pub fn get_complexity_thresholds() -> crate::complexity::threshold_manager::ComplexityThresholds {
    get_config()
        .complexity_thresholds
        .clone()
        .unwrap_or_default()
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

/// Get error handling configuration
pub fn get_error_handling_config() -> ErrorHandlingConfig {
    get_config().error_handling.clone().unwrap_or_default()
}

/// Get role-based coverage weight multipliers
pub fn get_role_coverage_weights() -> RoleCoverageWeights {
    get_config()
        .role_coverage_weights
        .clone()
        .unwrap_or_default()
}

/// Get role multiplier clamping configuration (spec 119)
pub fn get_role_multiplier_config() -> RoleMultiplierConfig {
    get_config()
        .role_multiplier_config
        .clone()
        .unwrap_or_default()
}

/// Get orchestrator detection configuration
pub fn get_orchestrator_detection_config() -> OrchestratorDetectionConfig {
    get_config()
        .orchestrator_detection
        .clone()
        .unwrap_or_default()
}

/// Get orchestration adjustment configuration (spec 110)
pub fn get_orchestration_adjustment_config(
) -> crate::priority::scoring::orchestration_adjustment::OrchestrationAdjustmentConfig {
    get_config()
        .orchestration_adjustment
        .clone()
        .unwrap_or_default()
}

/// Get constructor detection configuration (spec 117)
pub fn get_constructor_detection_config() -> ConstructorDetectionConfig {
    get_config()
        .classification
        .as_ref()
        .and_then(|c| c.constructors.clone())
        .unwrap_or_default()
}

/// Get accessor detection configuration (spec 125)
pub fn get_accessor_detection_config() -> AccessorDetectionConfig {
    get_config()
        .classification
        .as_ref()
        .and_then(|c| c.accessors.clone())
        .unwrap_or_default()
}

/// Get data flow classification configuration (spec 126)
pub fn get_data_flow_classification_config() -> DataFlowClassificationConfig {
    get_config()
        .classification
        .as_ref()
        .and_then(|c| c.data_flow.clone())
        .unwrap_or_default()
}

/// Get smart performance configuration
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_ignore_patterns_with_patterns() {
        let config = DebtmapConfig {
            ignore: Some(IgnoreConfig {
                patterns: vec![
                    "tests/**/*".to_string(),
                    "*.test.rs".to_string(),
                    "**/fixtures/**".to_string(),
                ],
            }),
            ..Default::default()
        };

        let patterns = config.get_ignore_patterns();
        assert_eq!(patterns.len(), 3);
        assert!(patterns.contains(&"tests/**/*".to_string()));
        assert!(patterns.contains(&"*.test.rs".to_string()));
        assert!(patterns.contains(&"**/fixtures/**".to_string()));
    }

    #[test]
    fn test_get_ignore_patterns_without_config() {
        let config = DebtmapConfig::default();
        let patterns = config.get_ignore_patterns();
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn test_get_ignore_patterns_with_empty_patterns() {
        let config = DebtmapConfig {
            ignore: Some(IgnoreConfig { patterns: vec![] }),
            ..Default::default()
        };

        let patterns = config.get_ignore_patterns();
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn test_parse_and_validate_config_valid_toml() {
        let toml_content = r#"
[context]
critical_paths = ["/src/main.rs"]

[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15
"#;
        let result = super::parse_and_validate_config(toml_content);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.scoring.is_some());
        let scoring = config.scoring.unwrap();
        // Active weights should sum to 1.0
        let active_sum = scoring.coverage + scoring.complexity + scoring.dependency;
        assert!((active_sum - 1.0).abs() < 0.001);
        // Check the values with floating point tolerance
        assert!((scoring.coverage - 0.50).abs() < 0.001);
        assert!((scoring.complexity - 0.35).abs() < 0.001);
        assert!((scoring.dependency - 0.15).abs() < 0.001);
        // Unused weights should be 0
        assert!((scoring.semantic - 0.0).abs() < 0.001);
        assert!((scoring.security - 0.0).abs() < 0.001);
        assert!((scoring.organization - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_and_validate_config_invalid_toml() {
        let toml_content = "invalid toml [[ content";
        let result = super::parse_and_validate_config(toml_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse"));
    }

    #[test]
    fn test_parse_and_validate_config_invalid_weights_replaced_with_defaults() {
        let toml_content = r#"
[scoring]
coverage = 0.5
complexity = 0.5
semantic = 0.5
dependency = 0.5
security = 0.5
organization = 0.5
"#;
        let result = super::parse_and_validate_config(toml_content);
        assert!(result.is_ok());
        let config = result.unwrap();
        let scoring = config.scoring.unwrap();
        // Invalid weights (sum > 1.0) should be replaced with defaults
        assert_eq!(scoring.coverage, 0.50);
        assert_eq!(scoring.complexity, 0.35);
        assert_eq!(scoring.semantic, 0.00);
        assert_eq!(scoring.dependency, 0.15);
        assert_eq!(scoring.security, 0.00);
        assert_eq!(scoring.organization, 0.00);
        let active_sum = scoring.coverage + scoring.complexity + scoring.dependency;
        assert!((active_sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_directory_ancestors_generates_correct_sequence() {
        use std::path::PathBuf;

        let start = PathBuf::from("/a/b/c/d");
        let ancestors: Vec<PathBuf> = super::directory_ancestors(start, 3).collect();

        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0], PathBuf::from("/a/b/c/d"));
        assert_eq!(ancestors[1], PathBuf::from("/a/b/c"));
        assert_eq!(ancestors[2], PathBuf::from("/a/b"));
    }

    #[test]
    fn test_directory_ancestors_respects_max_depth() {
        use std::path::PathBuf;

        let start = PathBuf::from("/a/b/c/d/e/f/g/h");
        let ancestors: Vec<PathBuf> = super::directory_ancestors(start, 2).collect();

        assert_eq!(ancestors.len(), 2);
    }

    #[test]
    fn test_directory_ancestors_handles_root() {
        use std::path::PathBuf;

        let start = PathBuf::from("/");
        let ancestors: Vec<PathBuf> = super::directory_ancestors(start, 5).collect();

        // Root directory has no parent, so we only get the root itself
        assert_eq!(ancestors.len(), 1);
        assert_eq!(ancestors[0], PathBuf::from("/"));
    }

    #[test]
    fn test_try_load_config_from_path_with_valid_config() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("debtmap.toml");

        // Write a valid config file
        fs::write(
            &config_path,
            r#"
[thresholds]
complexity = 15
max_file_length = 1000

[scoring]
complexity_weight = 0.4
coverage_weight = 0.3
inheritance_weight = 0.15
interface_weight = 0.15
"#,
        )
        .unwrap();

        let result = try_load_config_from_path(&config_path);
        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.thresholds.as_ref().unwrap().complexity, Some(15));
        assert_eq!(
            config.thresholds.as_ref().unwrap().max_file_length,
            Some(1000)
        );
    }

    #[test]
    fn test_try_load_config_from_path_with_invalid_config() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("debtmap.toml");

        // Write an invalid config file
        fs::write(&config_path, "invalid toml content").unwrap();

        let result = try_load_config_from_path(&config_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_try_load_config_from_path_with_nonexistent_file() {
        use std::path::PathBuf;

        let config_path = PathBuf::from("/nonexistent/path/to/config.toml");
        let result = try_load_config_from_path(&config_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_read_error_with_not_found() {
        use std::io;
        use std::path::PathBuf;

        let path = PathBuf::from("/test/path");
        let error = io::Error::new(io::ErrorKind::NotFound, "File not found");

        // This should not panic and should not log a warning for NotFound
        handle_read_error(&path, &error);
    }

    #[test]
    fn test_handle_read_error_with_permission_denied() {
        use std::io;
        use std::path::PathBuf;

        let path = PathBuf::from("/test/path");
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");

        // This should log a warning but not panic
        handle_read_error(&path, &error);
    }

    #[test]
    fn test_get_validation_thresholds_with_defaults() {
        // Test that get_validation_thresholds returns expected values
        // The config might override these, so we test flexible values
        let thresholds = get_validation_thresholds();
        assert_eq!(thresholds.max_average_complexity, 10.0);
        assert_eq!(thresholds.max_high_complexity_count, 100);
        // max_debt_items can be 2000 (default) or 2500 (from config)
        assert!(thresholds.max_debt_items >= 2000);
        // max_total_debt_score can be 1000 (default) or 5000 (from config)
        assert!(thresholds.max_total_debt_score >= 1000);
        assert_eq!(thresholds.max_codebase_risk_score, 7.0);
        assert_eq!(thresholds.max_high_risk_functions, 50);
        assert_eq!(thresholds.min_coverage_percentage, 0.0);
    }

    #[test]
    fn test_default_linear_threshold() {
        assert_eq!(default_linear_threshold(), 10.0);
    }

    #[test]
    fn test_default_logarithmic_threshold() {
        assert_eq!(default_logarithmic_threshold(), 100.0);
    }

    #[test]
    fn test_default_sqrt_multiplier() {
        assert_eq!(default_sqrt_multiplier(), 3.33);
    }

    #[test]
    fn test_default_log_multiplier() {
        assert_eq!(default_log_multiplier(), 10.0);
    }

    #[test]
    fn test_default_show_raw_scores() {
        assert!(default_show_raw_scores());
    }

    #[test]
    fn test_god_object_thresholds_rust_defaults() {
        let thresholds = GodObjectThresholds::rust_defaults();
        assert_eq!(thresholds.max_methods, 20);
        assert_eq!(thresholds.max_fields, 15);
        assert_eq!(thresholds.max_traits, 5);
        assert_eq!(thresholds.max_lines, 1000);
        assert_eq!(thresholds.max_complexity, 200);
    }

    #[test]
    fn test_god_object_thresholds_python_defaults() {
        let thresholds = GodObjectThresholds::python_defaults();
        assert_eq!(thresholds.max_methods, 15);
        assert_eq!(thresholds.max_fields, 10);
        assert_eq!(thresholds.max_traits, 3);
        assert_eq!(thresholds.max_lines, 500);
        assert_eq!(thresholds.max_complexity, 150);
    }

    #[test]
    fn test_god_object_thresholds_javascript_defaults() {
        let thresholds = GodObjectThresholds::javascript_defaults();
        assert_eq!(thresholds.max_methods, 15);
        assert_eq!(thresholds.max_fields, 20);
        assert_eq!(thresholds.max_traits, 3);
        assert_eq!(thresholds.max_lines, 500);
        assert_eq!(thresholds.max_complexity, 150);
    }

    #[test]
    fn test_normalization_config_default() {
        let config = NormalizationConfig::default();
        assert_eq!(config.linear_threshold, 10.0);
        assert_eq!(config.logarithmic_threshold, 100.0);
        assert_eq!(config.sqrt_multiplier, 3.33);
        assert_eq!(config.log_multiplier, 10.0);
        assert!(config.show_raw_scores);
    }

    #[test]
    fn test_role_multipliers_default() {
        let multipliers = RoleMultipliers::default();
        assert_eq!(multipliers.pure_logic, 1.2);
        assert_eq!(multipliers.orchestrator, 0.8);
        assert_eq!(multipliers.io_wrapper, 0.7);
        assert_eq!(multipliers.entry_point, 0.9);
        assert_eq!(multipliers.pattern_match, 0.6);
        assert_eq!(multipliers.unknown, 1.0);
    }

    #[test]
    fn test_scoring_weights_default() {
        let weights = ScoringWeights::default();
        assert_eq!(weights.coverage, 0.50);
        assert_eq!(weights.complexity, 0.35);
        assert_eq!(weights.semantic, 0.00);
        assert_eq!(weights.dependency, 0.15);
        assert_eq!(weights.security, 0.00);
        assert_eq!(weights.organization, 0.00);
    }

    #[test]
    fn test_scoring_weights_validate_success() {
        let weights = ScoringWeights {
            coverage: 0.50,
            complexity: 0.35,
            semantic: 0.0,
            dependency: 0.15,
            security: 0.0,
            organization: 0.0,
        };
        assert!(weights.validate().is_ok());
    }

    #[test]
    fn test_scoring_weights_validate_invalid_sum() {
        let weights = ScoringWeights {
            coverage: 0.60,
            complexity: 0.60,
            semantic: 0.0,
            dependency: 0.0,
            security: 0.0,
            organization: 0.0,
        };
        assert!(weights.validate().is_err());
    }

    #[test]
    fn test_scoring_weights_normalize() {
        let mut weights = ScoringWeights {
            coverage: 0.40,
            complexity: 0.30,
            semantic: 0.0,
            dependency: 0.10,
            security: 0.0,
            organization: 0.0,
        };
        weights.normalize();
        // After normalization, active weights should sum to 1.0
        let sum = weights.coverage + weights.complexity + weights.dependency;
        assert!((sum - 1.0).abs() < 0.001);
        // Check proportions are maintained
        assert!((weights.coverage - 0.50).abs() < 0.001);
        assert!((weights.complexity - 0.375).abs() < 0.001);
        assert!((weights.dependency - 0.125).abs() < 0.001);
    }

    #[test]
    fn test_entropy_config_default() {
        let config = EntropyConfig::default();
        assert!(config.enabled);
        assert_eq!(config.weight, 1.0);
        assert_eq!(config.min_tokens, 20);
        assert_eq!(config.pattern_threshold, 0.7);
        assert_eq!(config.entropy_threshold, 0.4);
        assert_eq!(config.branch_threshold, 0.8);
        assert_eq!(config.max_repetition_reduction, 0.20);
        assert_eq!(config.max_entropy_reduction, 0.15);
        assert_eq!(config.max_branch_reduction, 0.25);
        assert_eq!(config.max_combined_reduction, 0.30);
    }

    #[test]
    fn test_error_handling_config_default() {
        let config = ErrorHandlingConfig::default();
        assert!(config.detect_async_errors);
        assert!(config.detect_context_loss);
        assert!(config.detect_propagation);
        assert!(config.detect_panic_patterns);
        assert!(config.detect_swallowing);
        assert_eq!(config.custom_patterns.len(), 0);
        assert_eq!(config.severity_overrides.len(), 0);
    }

    #[test]
    fn test_god_object_config_default() {
        let config = GodObjectConfig::default();
        assert!(config.enabled);
        // Test Rust defaults
        assert_eq!(config.rust.max_methods, 20);
        assert_eq!(config.rust.max_fields, 15);
        // Test Python defaults
        assert_eq!(config.python.max_methods, 15);
        assert_eq!(config.python.max_fields, 10);
        // Test JavaScript defaults
        assert_eq!(config.javascript.max_methods, 15);
        assert_eq!(config.javascript.max_fields, 20);
    }

    #[test]
    fn test_context_config_default() {
        let config = ContextConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.rules.len(), 0);
        assert!(config.function_patterns.is_none());
    }

    #[test]
    fn test_language_features_default() {
        let features = LanguageFeatures::default();
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_minimum_debt_score() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let score = get_minimum_debt_score();
        // The default is 1.0 but config might override it to 2.0
        assert!(score >= 1.0);
    }

    #[test]
    fn test_get_minimum_cyclomatic_complexity() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let complexity = get_minimum_cyclomatic_complexity();
        // The default is 2 but config might override it to 3
        assert!(complexity >= 2);
    }

    #[test]
    fn test_get_minimum_cognitive_complexity() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let complexity = get_minimum_cognitive_complexity();
        // The default is 3 but config might override it to 5
        assert!(complexity >= 3);
    }

    #[test]
    fn test_get_minimum_risk_score() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let score = get_minimum_risk_score();
        // The default is 1.0 but config might override it to 2.0
        assert!(score >= 1.0);
    }

    #[test]
    fn test_default_god_object_thresholds() {
        let thresholds = GodObjectThresholds::default();
        assert_eq!(thresholds.max_methods, 20);
        assert_eq!(thresholds.max_fields, 15);
        assert_eq!(thresholds.max_traits, 5);
        assert_eq!(thresholds.max_lines, 1000);
        assert_eq!(thresholds.max_complexity, 200);
    }

    #[test]
    fn test_validation_thresholds_default() {
        let thresholds = ValidationThresholds::default();
        assert_eq!(thresholds.max_average_complexity, 10.0);
        assert_eq!(thresholds.max_high_complexity_count, 100);
        assert_eq!(thresholds.max_debt_items, 2000);
        assert_eq!(thresholds.max_total_debt_score, 1000);
        assert_eq!(thresholds.max_codebase_risk_score, 7.0);
        assert_eq!(thresholds.max_high_risk_functions, 50);
        assert_eq!(thresholds.min_coverage_percentage, 0.0);
    }

    #[test]
    fn test_get_language_features_rust() {
        use crate::core::Language;
        let features = get_language_features(&Language::Rust);
        assert!(!features.detect_dead_code); // Rust has dead code detection disabled
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_language_features_python() {
        use crate::core::Language;
        let features = get_language_features(&Language::Python);
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_language_features_javascript() {
        use crate::core::Language;
        let features = get_language_features(&Language::JavaScript);
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_language_features_typescript() {
        use crate::core::Language;
        let features = get_language_features(&Language::TypeScript);
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_language_features_unknown() {
        use crate::core::Language;
        let features = get_language_features(&Language::Unknown);
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_entropy_config() {
        let config = get_entropy_config();
        // Config might override these values
        assert!(config.enabled);
        // Weight might be configured to 0.5 in .debtmap.toml
        assert!(config.weight > 0.0);
    }

    #[test]
    fn test_get_role_multipliers() {
        let multipliers = get_role_multipliers();
        assert_eq!(multipliers.pure_logic, 1.2);
        assert_eq!(multipliers.orchestrator, 0.8);
    }

    #[test]
    fn test_get_error_handling_config() {
        let config = get_error_handling_config();
        assert!(config.detect_async_errors);
        assert!(config.detect_context_loss);
    }

    #[test]
    fn test_get_scoring_weights() {
        let weights = get_scoring_weights();
        assert_eq!(weights.coverage, 0.50);
        assert_eq!(weights.complexity, 0.35);
        assert_eq!(weights.dependency, 0.15);
    }

    #[test]
    fn test_default_weight_functions() {
        assert_eq!(default_coverage_weight(), 0.50);
        assert_eq!(default_complexity_weight(), 0.35);
        assert_eq!(default_semantic_weight(), 0.00);
        assert_eq!(default_dependency_weight(), 0.15);
        assert_eq!(default_security_weight(), 0.00);
        assert_eq!(default_organization_weight(), 0.00);
    }

    #[test]
    fn test_default_multiplier_functions() {
        assert_eq!(default_pure_logic_multiplier(), 1.2);
        assert_eq!(default_orchestrator_multiplier(), 0.8);
        assert_eq!(default_io_wrapper_multiplier(), 0.7);
        assert_eq!(default_entry_point_multiplier(), 0.9);
        assert_eq!(default_pattern_match_multiplier(), 0.6);
        assert_eq!(default_unknown_multiplier(), 1.0);
    }

    #[test]
    fn test_default_threshold_functions() {
        assert_eq!(default_max_methods(), 20);
        assert_eq!(default_max_fields(), 15);
        assert_eq!(default_max_traits(), 5);
        assert_eq!(default_max_lines(), 1000);
        assert_eq!(default_max_complexity(), 200);
        assert!(default_god_object_enabled());
    }

    #[test]
    fn test_default_validation_threshold_functions() {
        assert_eq!(default_max_avg_complexity(), 10.0);
        assert_eq!(default_max_high_complexity_count(), 100);
        assert_eq!(default_max_debt_items(), 2000);
        assert_eq!(default_max_total_debt_score(), 1000);
        assert_eq!(default_max_codebase_risk(), 7.0);
        assert_eq!(default_max_high_risk_count(), 50);
        assert_eq!(default_min_coverage(), 0.0);
    }

    #[test]
    fn test_default_language_feature_functions() {
        assert!(default_detect_dead_code());
        assert!(default_detect_complexity());
        assert!(default_detect_duplication());
    }

    #[test]
    fn test_default_entropy_functions() {
        assert!(default_entropy_enabled());
        assert_eq!(default_entropy_weight(), 1.0);
        assert_eq!(default_entropy_min_tokens(), 20);
        assert_eq!(default_entropy_pattern_threshold(), 0.7);
        assert_eq!(default_entropy_threshold(), 0.4);
        assert_eq!(default_branch_threshold(), 0.8);
        assert_eq!(default_max_repetition_reduction(), 0.20);
        assert_eq!(default_max_entropy_reduction(), 0.15);
        assert_eq!(default_max_branch_reduction(), 0.25);
        assert_eq!(default_max_combined_reduction(), 0.30);
    }

    #[test]
    fn test_default_error_handling_functions() {
        assert!(default_detect_async_errors());
        assert!(default_detect_context_loss());
        assert!(default_detect_propagation());
        assert!(default_detect_panic_patterns());
        assert!(default_detect_swallowing());
    }

    #[test]
    fn test_default_context_functions() {
        assert!(!default_context_enabled());
        assert_eq!(default_rule_priority(), 50);
    }

    // Tests for extracted pure functions (spec 93)

    #[test]
    fn test_is_valid_weight() {
        // Test valid weights
        assert!(ScoringWeights::is_valid_weight(0.0));
        assert!(ScoringWeights::is_valid_weight(0.5));
        assert!(ScoringWeights::is_valid_weight(1.0));

        // Test invalid weights
        assert!(!ScoringWeights::is_valid_weight(-0.1));
        assert!(!ScoringWeights::is_valid_weight(1.1));
        assert!(!ScoringWeights::is_valid_weight(2.0));
        assert!(!ScoringWeights::is_valid_weight(-10.0));
    }

    #[test]
    fn test_validate_weight() {
        // Test valid weight
        assert!(ScoringWeights::validate_weight(0.5, "Test").is_ok());
        assert!(ScoringWeights::validate_weight(0.0, "Min").is_ok());
        assert!(ScoringWeights::validate_weight(1.0, "Max").is_ok());

        // Test invalid weight
        let result = ScoringWeights::validate_weight(1.5, "Invalid");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Invalid weight must be between 0.0 and 1.0"
        );
    }

    #[test]
    fn test_validate_active_weights_sum() {
        // Test valid sum (exactly 1.0)
        assert!(ScoringWeights::validate_active_weights_sum(0.5, 0.3, 0.2).is_ok());

        // Test valid sum (within tolerance)
        assert!(ScoringWeights::validate_active_weights_sum(0.5, 0.3, 0.2001).is_ok());
        assert!(ScoringWeights::validate_active_weights_sum(0.5, 0.3, 0.1999).is_ok());

        // Test invalid sum (too high)
        let result = ScoringWeights::validate_active_weights_sum(0.6, 0.5, 0.3);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must sum to 1.0, but sum to 1.400"));

        // Test invalid sum (too low)
        let result = ScoringWeights::validate_active_weights_sum(0.2, 0.2, 0.2);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must sum to 1.0, but sum to 0.600"));
    }

    #[test]
    fn test_collect_weight_validations() {
        // Test with all valid weights
        let weights = ScoringWeights {
            coverage: 0.5,
            complexity: 0.3,
            semantic: 0.0,
            dependency: 0.2,
            security: 0.0,
            organization: 0.0,
        };
        let validations = weights.collect_weight_validations();
        assert_eq!(validations.len(), 6);
        for validation in validations {
            assert!(validation.is_ok());
        }

        // Test with invalid weights
        let weights = ScoringWeights {
            coverage: 1.5,    // Invalid
            complexity: -0.1, // Invalid
            semantic: 0.0,
            dependency: 0.2,
            security: 2.0, // Invalid
            organization: 0.0,
        };
        let validations = weights.collect_weight_validations();
        assert_eq!(validations.len(), 6);
        assert!(validations[0].is_err()); // coverage
        assert!(validations[1].is_err()); // complexity
        assert!(validations[2].is_ok()); // semantic
        assert!(validations[3].is_ok()); // dependency
        assert!(validations[4].is_err()); // security
        assert!(validations[5].is_ok()); // organization
    }

    #[test]
    fn test_read_config_file() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        // Write test config
        fs::write(&config_path, "[thresholds]\ncomplexity = 15\n").unwrap();

        // Test reading existing file
        let contents = read_config_file(&config_path).unwrap();
        assert_eq!(contents, "[thresholds]\ncomplexity = 15\n");

        // Test reading non-existent file
        let non_existent = temp_dir.path().join("non_existent.toml");
        assert!(read_config_file(&non_existent).is_err());
    }

    #[test]
    fn test_parse_and_validate_config_impl() {
        // Test valid config
        let valid_toml = r#"
[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15
"#;
        let config = parse_and_validate_config_impl(valid_toml).unwrap();
        let scoring = config.scoring.unwrap();
        assert_eq!(scoring.coverage, 0.50);
        assert_eq!(scoring.complexity, 0.35);
        assert_eq!(scoring.dependency, 0.15);

        // Test invalid TOML
        let invalid_toml = "invalid [[ toml";
        assert!(parse_and_validate_config_impl(invalid_toml).is_err());

        // Test config with invalid weights (should be normalized)
        let invalid_weights = r#"
[scoring]
coverage = 0.6
complexity = 0.6
dependency = 0.6
"#;
        let config = parse_and_validate_config_impl(invalid_weights).unwrap();
        // Should use defaults due to invalid sum
        let scoring = config.scoring.unwrap();
        assert_eq!(scoring.coverage, 0.50);
        assert_eq!(scoring.complexity, 0.35);
        assert_eq!(scoring.dependency, 0.15);
    }

    #[test]
    fn test_directory_ancestors_impl() {
        use std::path::PathBuf;

        // Test normal path traversal
        let start = PathBuf::from("/a/b/c/d");
        let ancestors: Vec<PathBuf> = directory_ancestors_impl(start.clone(), 3).collect();
        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0], PathBuf::from("/a/b/c/d"));
        assert_eq!(ancestors[1], PathBuf::from("/a/b/c"));
        assert_eq!(ancestors[2], PathBuf::from("/a/b"));

        // Test with depth limit
        let ancestors: Vec<PathBuf> = directory_ancestors_impl(start.clone(), 2).collect();
        assert_eq!(ancestors.len(), 2);

        // Test with root path
        let root = PathBuf::from("/");
        let ancestors: Vec<PathBuf> = directory_ancestors_impl(root, 5).collect();
        assert_eq!(ancestors.len(), 1);
        assert_eq!(ancestors[0], PathBuf::from("/"));

        // Test with zero depth
        let ancestors: Vec<PathBuf> = directory_ancestors_impl(start, 0).collect();
        assert_eq!(ancestors.len(), 0);
    }

    #[test]
    fn test_handle_read_error() {
        use std::io;
        use std::path::PathBuf;

        let path = PathBuf::from("/test/path.toml");

        // Test NotFound error (should not log warning)
        let not_found = io::Error::new(io::ErrorKind::NotFound, "File not found");
        handle_read_error(&path, &not_found); // Should not panic

        // Test PermissionDenied error (should log warning)
        let permission = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        handle_read_error(&path, &permission); // Should not panic

        // Test other errors
        let other = io::Error::other("Unknown error");
        handle_read_error(&path, &other); // Should not panic
    }
}
