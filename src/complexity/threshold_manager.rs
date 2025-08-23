use crate::core::FunctionMetrics;
use serde::{Deserialize, Serialize};

/// Preset threshold configurations
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ThresholdPreset {
    /// Strict thresholds for high code quality standards
    Strict,
    /// Balanced thresholds for typical projects
    Balanced,
    /// Lenient thresholds for legacy or complex domains
    Lenient,
}

/// Complexity threshold configuration for determining when to flag functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityThresholds {
    /// Minimum total complexity to flag (cyclomatic + cognitive)
    #[serde(default = "default_minimum_total_complexity")]
    pub minimum_total_complexity: u32,

    /// Minimum cyclomatic complexity to flag
    #[serde(default = "default_minimum_cyclomatic_complexity")]
    pub minimum_cyclomatic_complexity: u32,

    /// Minimum cognitive complexity to flag
    #[serde(default = "default_minimum_cognitive_complexity")]
    pub minimum_cognitive_complexity: u32,

    /// Minimum number of match arms to flag
    #[serde(default = "default_minimum_match_arms")]
    pub minimum_match_arms: usize,

    /// Minimum if-else chain length to flag
    #[serde(default = "default_minimum_if_else_chain")]
    pub minimum_if_else_chain: usize,

    /// Minimum function length (lines) to flag
    #[serde(default = "default_minimum_function_length")]
    pub minimum_function_length: usize,

    // Role-based multipliers
    /// Multiplier for entry point functions (main, handlers)
    #[serde(default = "default_entry_point_multiplier")]
    pub entry_point_multiplier: f64,

    /// Multiplier for core logic functions
    #[serde(default = "default_core_logic_multiplier")]
    pub core_logic_multiplier: f64,

    /// Multiplier for utility functions (getters, setters)
    #[serde(default = "default_utility_multiplier")]
    pub utility_multiplier: f64,

    /// Multiplier for test functions
    #[serde(default = "default_test_function_multiplier")]
    pub test_function_multiplier: f64,
}

impl Default for ComplexityThresholds {
    fn default() -> Self {
        Self {
            minimum_total_complexity: default_minimum_total_complexity(),
            minimum_cyclomatic_complexity: default_minimum_cyclomatic_complexity(),
            minimum_cognitive_complexity: default_minimum_cognitive_complexity(),
            minimum_match_arms: default_minimum_match_arms(),
            minimum_if_else_chain: default_minimum_if_else_chain(),
            minimum_function_length: default_minimum_function_length(),
            entry_point_multiplier: default_entry_point_multiplier(),
            core_logic_multiplier: default_core_logic_multiplier(),
            utility_multiplier: default_utility_multiplier(),
            test_function_multiplier: default_test_function_multiplier(),
        }
    }
}

// Default values
fn default_minimum_total_complexity() -> u32 {
    8
}
fn default_minimum_cyclomatic_complexity() -> u32 {
    5
}
fn default_minimum_cognitive_complexity() -> u32 {
    10
}
fn default_minimum_match_arms() -> usize {
    4
}
fn default_minimum_if_else_chain() -> usize {
    3
}
fn default_minimum_function_length() -> usize {
    20
}
fn default_entry_point_multiplier() -> f64 {
    1.5
}
fn default_core_logic_multiplier() -> f64 {
    1.0
}
fn default_utility_multiplier() -> f64 {
    0.8
}
fn default_test_function_multiplier() -> f64 {
    2.0
}

/// Complexity level classification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Trivial,
    Moderate,
    High,
    Excessive,
}

impl ComplexityThresholds {
    /// Create thresholds from a preset configuration
    pub fn from_preset(preset: ThresholdPreset) -> Self {
        match preset {
            ThresholdPreset::Strict => Self {
                minimum_total_complexity: 5,
                minimum_cyclomatic_complexity: 3,
                minimum_cognitive_complexity: 7,
                minimum_match_arms: 3,
                minimum_if_else_chain: 2,
                minimum_function_length: 15,
                entry_point_multiplier: 1.2,
                core_logic_multiplier: 1.0,
                utility_multiplier: 0.6,
                test_function_multiplier: 3.0,  // More lenient for test functions
            },
            ThresholdPreset::Balanced => Self::default(),
            ThresholdPreset::Lenient => Self {
                minimum_total_complexity: 15,
                minimum_cyclomatic_complexity: 10,
                minimum_cognitive_complexity: 20,
                minimum_match_arms: 8,
                minimum_if_else_chain: 5,
                minimum_function_length: 50,
                entry_point_multiplier: 2.0,
                core_logic_multiplier: 1.0,
                utility_multiplier: 1.0,
                test_function_multiplier: 3.0,
            },
        }
    }

    /// Check if a function should be flagged based on thresholds
    pub fn should_flag_function(&self, metrics: &FunctionMetrics, role: FunctionRole) -> bool {
        let multiplier = self.get_role_multiplier(role);

        // Apply multiplier to thresholds (higher multiplier = more lenient)
        let adjusted_cyclomatic_threshold = (self.minimum_cyclomatic_complexity as f64 * multiplier) as u32;
        let adjusted_cognitive_threshold = (self.minimum_cognitive_complexity as f64 * multiplier) as u32;
        let adjusted_total_threshold = (self.minimum_total_complexity as f64 * multiplier) as u32;

        // Must exceed ALL adjusted thresholds to be flagged
        metrics.cyclomatic >= adjusted_cyclomatic_threshold
            && metrics.cognitive >= adjusted_cognitive_threshold
            && metrics.length >= self.minimum_function_length
            && (metrics.cyclomatic + metrics.cognitive) >= adjusted_total_threshold
    }

    /// Get the complexity level for given metrics
    pub fn get_complexity_level(&self, metrics: &FunctionMetrics) -> ComplexityLevel {
        let total = metrics.cyclomatic + metrics.cognitive;
        match total {
            t if t < self.minimum_total_complexity => ComplexityLevel::Trivial,
            t if t < self.minimum_total_complexity * 2 => ComplexityLevel::Moderate,
            t if t < self.minimum_total_complexity * 3 => ComplexityLevel::High,
            _ => ComplexityLevel::Excessive,
        }
    }

    /// Get role-based multiplier
    pub fn get_role_multiplier(&self, role: FunctionRole) -> f64 {
        match role {
            FunctionRole::EntryPoint => self.entry_point_multiplier,
            FunctionRole::CoreLogic => self.core_logic_multiplier,
            FunctionRole::Utility => self.utility_multiplier,
            FunctionRole::Test => self.test_function_multiplier,
            FunctionRole::Unknown => self.core_logic_multiplier,
        }
    }

    /// Validate thresholds are reasonable
    pub fn validate(&self) -> Result<(), String> {
        if self.minimum_total_complexity == 0 {
            return Err("minimum_total_complexity must be greater than 0".to_string());
        }
        if self.minimum_cyclomatic_complexity == 0 {
            return Err("minimum_cyclomatic_complexity must be greater than 0".to_string());
        }
        if self.minimum_cognitive_complexity == 0 {
            return Err("minimum_cognitive_complexity must be greater than 0".to_string());
        }

        // Check multipliers are positive
        if self.entry_point_multiplier <= 0.0 {
            return Err("entry_point_multiplier must be positive".to_string());
        }
        if self.core_logic_multiplier <= 0.0 {
            return Err("core_logic_multiplier must be positive".to_string());
        }
        if self.utility_multiplier <= 0.0 {
            return Err("utility_multiplier must be positive".to_string());
        }
        if self.test_function_multiplier <= 0.0 {
            return Err("test_function_multiplier must be positive".to_string());
        }

        Ok(())
    }

    /// Get preset configurations
    pub fn preset(name: &str) -> Option<Self> {
        match name {
            "strict" => Some(Self {
                minimum_total_complexity: 5,
                minimum_cyclomatic_complexity: 3,
                minimum_cognitive_complexity: 7,
                minimum_match_arms: 3,
                minimum_if_else_chain: 2,
                minimum_function_length: 15,
                entry_point_multiplier: 1.2,
                core_logic_multiplier: 1.0,
                utility_multiplier: 0.6,
                test_function_multiplier: 1.5,
            }),
            "balanced" => Some(Self::default()),
            "lenient" => Some(Self {
                minimum_total_complexity: 15,
                minimum_cyclomatic_complexity: 10,
                minimum_cognitive_complexity: 20,
                minimum_match_arms: 8,
                minimum_if_else_chain: 5,
                minimum_function_length: 50,
                entry_point_multiplier: 2.0,
                core_logic_multiplier: 1.0,
                utility_multiplier: 1.0,
                test_function_multiplier: 3.0,
            }),
            _ => None,
        }
    }
}

/// Function role classification (duplicated here to avoid circular dependency)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionRole {
    EntryPoint,
    CoreLogic,
    Utility,
    Test,
    Unknown,
}

impl FunctionRole {
    /// Determine role from function name
    pub fn from_name(name: &str) -> Self {
        if name == "main" || name.ends_with("_handler") || name.starts_with("handle_") {
            Self::EntryPoint
        } else if name.starts_with("test_") || name.ends_with("_test") {
            Self::Test
        } else if name.starts_with("get_")
            || name.starts_with("set_")
            || name.starts_with("is_")
            || name.starts_with("has_")
        {
            Self::Utility
        } else {
            Self::CoreLogic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_thresholds() {
        let thresholds = ComplexityThresholds::default();
        assert_eq!(thresholds.minimum_total_complexity, 8);
        assert_eq!(thresholds.minimum_cyclomatic_complexity, 5);
        assert_eq!(thresholds.minimum_cognitive_complexity, 10);
    }

    #[test]
    fn test_should_flag_function() {
        let thresholds = ComplexityThresholds::default();

        let mut simple_metrics =
            FunctionMetrics::new("simple".to_string(), std::path::PathBuf::from("test.rs"), 1);
        simple_metrics.cyclomatic = 2;
        simple_metrics.cognitive = 3;
        simple_metrics.length = 10;
        simple_metrics.is_pure = Some(true);
        simple_metrics.purity_confidence = Some(0.9);

        // Simple function should not be flagged
        assert!(!thresholds.should_flag_function(&simple_metrics, FunctionRole::CoreLogic));

        let mut complex_metrics = FunctionMetrics::new(
            "complex".to_string(),
            std::path::PathBuf::from("test.rs"),
            1,
        );
        complex_metrics.cyclomatic = 10;
        complex_metrics.cognitive = 15;
        complex_metrics.length = 50;
        complex_metrics.is_pure = Some(false);
        complex_metrics.purity_confidence = Some(0.9);

        // Complex function should be flagged
        assert!(thresholds.should_flag_function(&complex_metrics, FunctionRole::CoreLogic));
    }

    #[test]
    fn test_complexity_levels() {
        let thresholds = ComplexityThresholds::default();

        let mut trivial = FunctionMetrics::new(
            "trivial".to_string(),
            std::path::PathBuf::from("test.rs"),
            1,
        );
        trivial.cyclomatic = 2;
        trivial.cognitive = 3;
        assert_eq!(
            thresholds.get_complexity_level(&trivial),
            ComplexityLevel::Trivial
        );

        let mut moderate = FunctionMetrics::new(
            "moderate".to_string(),
            std::path::PathBuf::from("test.rs"),
            1,
        );
        moderate.cyclomatic = 5;
        moderate.cognitive = 8;
        assert_eq!(
            thresholds.get_complexity_level(&moderate),
            ComplexityLevel::Moderate
        );

        let mut high =
            FunctionMetrics::new("high".to_string(), std::path::PathBuf::from("test.rs"), 1);
        high.cyclomatic = 8;
        high.cognitive = 12;
        assert_eq!(
            thresholds.get_complexity_level(&high),
            ComplexityLevel::High
        );

        let mut excessive = FunctionMetrics::new(
            "excessive".to_string(),
            std::path::PathBuf::from("test.rs"),
            1,
        );
        excessive.cyclomatic = 20;
        excessive.cognitive = 30;
        assert_eq!(
            thresholds.get_complexity_level(&excessive),
            ComplexityLevel::Excessive
        );
    }

    #[test]
    fn test_presets() {
        let strict = ComplexityThresholds::preset("strict").unwrap();
        assert_eq!(strict.minimum_total_complexity, 5);

        let lenient = ComplexityThresholds::preset("lenient").unwrap();
        assert_eq!(lenient.minimum_total_complexity, 15);

        let balanced = ComplexityThresholds::preset("balanced").unwrap();
        assert_eq!(balanced.minimum_total_complexity, 8);
    }

    #[test]
    fn test_role_multipliers() {
        let thresholds = ComplexityThresholds::default();

        assert_eq!(
            thresholds.get_role_multiplier(FunctionRole::EntryPoint),
            1.5
        );
        assert_eq!(thresholds.get_role_multiplier(FunctionRole::CoreLogic), 1.0);
        assert_eq!(thresholds.get_role_multiplier(FunctionRole::Utility), 0.8);
        assert_eq!(thresholds.get_role_multiplier(FunctionRole::Test), 2.0);
    }
}
