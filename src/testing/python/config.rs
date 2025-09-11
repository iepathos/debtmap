//! Configuration module for Python test quality analysis
//!
//! This module provides configuration options for customizing the behavior
//! of test quality analyzers, including thresholds and detection settings.

use serde::{Deserialize, Serialize};

/// Configuration for Python test quality analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct PythonTestConfig {
    /// Configuration for complexity analysis
    pub complexity: ComplexityConfig,

    /// Configuration for assertion detection
    pub assertions: AssertionConfig,

    /// Configuration for flaky pattern detection
    pub flaky_patterns: FlakyPatternConfig,

    /// Configuration for excessive mocking detection
    pub mocking: MockingConfig,
}


/// Configuration for test complexity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityConfig {
    /// Base complexity threshold (default: 10)
    pub threshold: u32,

    /// Weight for conditional statements (default: 2)
    pub conditional_weight: u32,

    /// Weight for loops (default: 3)
    pub loop_weight: u32,

    /// Number of assertions before penalty (default: 5)
    pub assertion_threshold: u32,

    /// Weight for each assertion over threshold (default: 1)
    pub assertion_weight: u32,

    /// Weight for each mock/patch (default: 2)
    pub mock_weight: u32,

    /// Nesting depth before penalty (default: 2)
    pub nesting_threshold: u32,

    /// Weight for each level over nesting threshold (default: 2)
    pub nesting_weight: u32,

    /// Line count before penalty (default: 20)
    pub line_threshold: u32,

    /// Divisor for lines over threshold (default: 5)
    pub line_divisor: u32,
}

impl Default for ComplexityConfig {
    fn default() -> Self {
        Self {
            threshold: 10,
            conditional_weight: 2,
            loop_weight: 3,
            assertion_threshold: 5,
            assertion_weight: 1,
            mock_weight: 2,
            nesting_threshold: 2,
            nesting_weight: 2,
            line_threshold: 20,
            line_divisor: 5,
        }
    }
}

/// Configuration for assertion detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionConfig {
    /// Whether to report tests with only setup code (default: true)
    pub report_setup_only: bool,

    /// Whether to suggest assertions for variables (default: true)
    pub suggest_assertions: bool,

    /// Minimum number of lines to consider as "has setup" (default: 1)
    pub min_setup_lines: usize,
}

impl Default for AssertionConfig {
    fn default() -> Self {
        Self {
            report_setup_only: true,
            suggest_assertions: true,
            min_setup_lines: 1,
        }
    }
}

/// Configuration for flaky pattern detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlakyPatternConfig {
    /// Whether to detect timing dependencies (default: true)
    pub detect_timing: bool,

    /// Whether to detect random value usage (default: true)
    pub detect_random: bool,

    /// Whether to detect external dependencies (default: true)
    pub detect_external: bool,

    /// Whether to detect filesystem dependencies (default: true)
    pub detect_filesystem: bool,

    /// Whether to detect network dependencies (default: true)
    pub detect_network: bool,

    /// Whether to detect threading issues (default: true)
    pub detect_threading: bool,

    /// Allow filesystem operations with temp directories (default: true)
    pub allow_temp_filesystem: bool,

    /// Allow threading with proper synchronization (default: true)
    pub allow_synchronized_threading: bool,
}

impl Default for FlakyPatternConfig {
    fn default() -> Self {
        Self {
            detect_timing: true,
            detect_random: true,
            detect_external: true,
            detect_filesystem: true,
            detect_network: true,
            detect_threading: true,
            allow_temp_filesystem: true,
            allow_synchronized_threading: true,
        }
    }
}

/// Configuration for mocking detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockingConfig {
    /// Maximum number of mocks before reporting (default: 5)
    pub max_mocks: usize,

    /// Whether to count decorators (default: true)
    pub count_decorators: bool,

    /// Whether to count inline mocks (default: true)
    pub count_inline: bool,

    /// Whether to count context managers (default: true)
    pub count_context_managers: bool,
}

impl Default for MockingConfig {
    fn default() -> Self {
        Self {
            max_mocks: 5,
            count_decorators: true,
            count_inline: true,
            count_context_managers: true,
        }
    }
}

impl PythonTestConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Create a configuration with strict settings (lower thresholds)
    pub fn strict() -> Self {
        Self {
            complexity: ComplexityConfig {
                threshold: 5,
                ..ComplexityConfig::default()
            },
            mocking: MockingConfig {
                max_mocks: 3,
                ..MockingConfig::default()
            },
            ..Self::default()
        }
    }

    /// Create a configuration with relaxed settings (higher thresholds)
    pub fn relaxed() -> Self {
        Self {
            complexity: ComplexityConfig {
                threshold: 20,
                ..ComplexityConfig::default()
            },
            mocking: MockingConfig {
                max_mocks: 10,
                ..MockingConfig::default()
            },
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PythonTestConfig::default();
        assert_eq!(config.complexity.threshold, 10);
        assert_eq!(config.mocking.max_mocks, 5);
        assert!(config.flaky_patterns.detect_timing);
    }

    #[test]
    fn test_strict_config() {
        let config = PythonTestConfig::strict();
        assert_eq!(config.complexity.threshold, 5);
        assert_eq!(config.mocking.max_mocks, 3);
    }

    #[test]
    fn test_relaxed_config() {
        let config = PythonTestConfig::relaxed();
        assert_eq!(config.complexity.threshold, 20);
        assert_eq!(config.mocking.max_mocks, 10);
    }

    #[test]
    fn test_config_serialization() {
        let config = PythonTestConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: PythonTestConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.complexity.threshold, config.complexity.threshold);
    }
}
