use serde::{Deserialize, Serialize};

/// God object detection thresholds
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
    pub fn rust_defaults() -> Self {
        Self {
            max_methods: 20,
            max_fields: 15,
            max_traits: 5,
            max_lines: 1000,
            max_complexity: 200,
        }
    }

    pub fn python_defaults() -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_traits: 3,
            max_lines: 500,
            max_complexity: 150,
        }
    }

    pub fn javascript_defaults() -> Self {
        Self {
            max_methods: 15,
            max_fields: 20,
            max_traits: 3,
            max_lines: 500,
            max_complexity: 150,
        }
    }
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

    /// Context-aware file size thresholds (spec 135)
    #[serde(default)]
    pub file_size: Option<FileSizeThresholds>,
}

/// Validation thresholds for the validate command
///
/// # Density-Based Validation
///
/// This configuration uses density-based metrics as primary validation
/// criteria. Debt density (debt per 1000 LOC) provides a stable quality
/// measure that doesn't require adjustment as the codebase grows.
///
/// ## Recommended Configuration
///
/// ```toml
/// [thresholds.validation]
/// max_debt_density = 50.0             # Primary quality metric
/// max_average_complexity = 10.0       # Per-function quality
/// max_codebase_risk_score = 7.0       # Overall risk level
/// ```
///
/// ## Migration from Scale-Dependent Metrics
///
/// Old scale-dependent metrics are deprecated:
/// - `max_high_complexity_count` → Use `max_debt_density`
/// - `max_debt_items` → Use `max_debt_density`
/// - `max_high_risk_functions` → Use `max_debt_density` + `max_codebase_risk_score`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationThresholds {
    // === PRIMARY QUALITY METRICS (Scale-Independent) ===
    /// Maximum allowed average complexity per function (default: 10.0)
    /// This measures typical function complexity across the codebase
    #[serde(default = "default_max_avg_complexity")]
    pub max_average_complexity: f64,

    /// Maximum allowed debt density per 1000 LOC (default: 50.0)
    /// This is the PRIMARY quality metric for validation
    /// Provides scale-independent quality measure that remains stable as codebase grows
    #[serde(default = "default_max_debt_density")]
    pub max_debt_density: f64,

    /// Maximum allowed codebase risk score (default: 7.0)
    /// Overall risk level combining complexity, coverage, and criticality
    #[serde(default = "default_max_codebase_risk")]
    pub max_codebase_risk_score: f64,

    // === OPTIONAL METRICS ===
    /// Minimum required code coverage percentage (default: 0.0 - disabled)
    /// Set to enforce minimum test coverage requirements
    #[serde(default = "default_min_coverage")]
    pub min_coverage_percentage: f64,

    // === SAFETY NET ===
    /// Maximum total debt score - safety net to catch extreme cases (default: 10000)
    /// High ceiling that rarely triggers in normal operation
    /// Prevents runaway growth even if density stays low
    #[serde(default = "default_max_total_debt_score_high")]
    pub max_total_debt_score: u32,

    // === DEPRECATED (Will be removed in v1.0) ===
    /// DEPRECATED: Use max_debt_density instead
    /// This scale-dependent metric grows linearly with codebase size
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.3.0", note = "Use max_debt_density instead")]
    pub max_high_complexity_count: Option<usize>,

    /// DEPRECATED: Use max_debt_density instead
    /// This scale-dependent metric grows linearly with codebase size
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.3.0", note = "Use max_debt_density instead")]
    pub max_debt_items: Option<usize>,

    /// DEPRECATED: Use max_debt_density and max_codebase_risk_score instead
    /// This scale-dependent metric grows linearly with codebase size
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(
        since = "0.3.0",
        note = "Use max_debt_density and max_codebase_risk_score instead"
    )]
    pub max_high_risk_functions: Option<usize>,
}

impl Default for ValidationThresholds {
    #[allow(deprecated)]
    fn default() -> Self {
        Self {
            // Primary quality metrics
            max_average_complexity: default_max_avg_complexity(),
            max_debt_density: default_max_debt_density(),
            max_codebase_risk_score: default_max_codebase_risk(),
            // Optional metrics
            min_coverage_percentage: default_min_coverage(),
            // Safety net
            max_total_debt_score: default_max_total_debt_score_high(),
            // Deprecated metrics (None by default)
            max_high_complexity_count: None,
            max_debt_items: None,
            max_high_risk_functions: None,
        }
    }
}

// Default validation threshold values
fn default_max_avg_complexity() -> f64 {
    10.0
}
fn default_max_codebase_risk() -> f64 {
    7.0
}
fn default_min_coverage() -> f64 {
    0.0
}
fn default_max_debt_density() -> f64 {
    50.0 // 50 debt points per 1000 LOC - reasonable default for most projects
}
fn default_max_total_debt_score_high() -> u32 {
    10000 // High ceiling - 5x typical project, acts as safety net for extreme cases
}

/// Context-aware file size thresholds (spec 135)
///
/// Provides different size limits based on file type and purpose.
/// This prevents unrealistic recommendations for generated code,
/// declarative configurations, and other special file types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSizeThresholds {
    /// Business logic files (strict)
    #[serde(default = "default_business_logic_threshold")]
    pub business_logic: usize,

    /// Test code files (moderate)
    #[serde(default = "default_test_code_threshold")]
    pub test_code: usize,

    /// Declarative configuration files (lenient)
    #[serde(default = "default_declarative_config_threshold")]
    pub declarative_config: usize,

    /// Generated code files (very lenient/suppressed)
    #[serde(default = "default_generated_code_threshold")]
    pub generated_code: usize,

    /// Procedural macro files (moderate-strict)
    #[serde(default = "default_proc_macro_threshold")]
    pub proc_macro: usize,

    /// Build scripts (strict)
    #[serde(default = "default_build_script_threshold")]
    pub build_script: usize,

    /// Minimum lines per function (safety threshold)
    #[serde(default = "default_min_lines_per_function")]
    pub min_lines_per_function: f32,

    /// File-specific overrides using glob patterns
    #[serde(default)]
    pub overrides: std::collections::HashMap<String, usize>,
}

impl Default for FileSizeThresholds {
    fn default() -> Self {
        Self {
            business_logic: default_business_logic_threshold(),
            test_code: default_test_code_threshold(),
            declarative_config: default_declarative_config_threshold(),
            generated_code: default_generated_code_threshold(),
            proc_macro: default_proc_macro_threshold(),
            build_script: default_build_script_threshold(),
            min_lines_per_function: default_min_lines_per_function(),
            overrides: std::collections::HashMap::new(),
        }
    }
}

// Default file size threshold values
fn default_business_logic_threshold() -> usize {
    400
}

fn default_test_code_threshold() -> usize {
    650
}

fn default_declarative_config_threshold() -> usize {
    1200
}

fn default_generated_code_threshold() -> usize {
    5000
}

fn default_proc_macro_threshold() -> usize {
    500
}

fn default_build_script_threshold() -> usize {
    300
}

fn default_min_lines_per_function() -> f32 {
    3.0
}
