use serde::{Deserialize, Serialize};

use super::{AccessorDetectionConfig, ConstructorDetectionConfig, DataFlowClassificationConfig};

/// Configuration for caller/callee display in output
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
