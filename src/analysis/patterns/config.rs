//! Pattern recognition configuration
//!
//! Provides user-configurable pattern matching rules and thresholds
//! for design pattern detection.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main pattern recognition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    /// Enable/disable pattern recognition globally
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Observer pattern configuration
    #[serde(default)]
    pub observer: ObserverConfig,

    /// Singleton pattern configuration
    #[serde(default)]
    pub singleton: SingletonConfig,

    /// Factory pattern configuration
    #[serde(default)]
    pub factory: FactoryConfig,

    /// Strategy pattern configuration
    #[serde(default)]
    pub strategy: StrategyConfig,

    /// Callback pattern configuration
    #[serde(default)]
    pub callback: CallbackConfig,

    /// Template method pattern configuration
    #[serde(default)]
    pub template_method: TemplateMethodConfig,

    /// Custom pattern rules
    #[serde(default)]
    pub custom_rules: Vec<CustomPatternRule>,

    /// Minimum confidence threshold for pattern detection (0.0 - 1.0)
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f32,
}

fn default_enabled() -> bool {
    true
}

fn default_confidence_threshold() -> f32 {
    0.7
}

/// Observer pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverConfig {
    /// Markers for identifying observer interfaces
    #[serde(default = "default_interface_markers")]
    pub interface_markers: Vec<String>,

    /// Method names that indicate observer registration
    #[serde(default = "default_registration_methods")]
    pub registration_methods: Vec<String>,

    /// Prefixes for observer notification methods
    #[serde(default = "default_method_prefixes")]
    pub method_prefixes: Vec<String>,
}

fn default_interface_markers() -> Vec<String> {
    vec![
        "ABC".to_string(),
        "Protocol".to_string(),
        "Interface".to_string(),
    ]
}

fn default_registration_methods() -> Vec<String> {
    vec![
        "add_observer".to_string(),
        "register".to_string(),
        "subscribe".to_string(),
    ]
}

fn default_method_prefixes() -> Vec<String> {
    vec![
        "on_".to_string(),
        "handle_".to_string(),
        "notify_".to_string(),
    ]
}

impl Default for ObserverConfig {
    fn default() -> Self {
        Self {
            interface_markers: default_interface_markers(),
            registration_methods: default_registration_methods(),
            method_prefixes: default_method_prefixes(),
        }
    }
}

/// Singleton pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingletonConfig {
    /// Detect module-level singletons
    #[serde(default = "default_true")]
    pub detect_module_level: bool,

    /// Detect singletons using __new__ override
    #[serde(default = "default_true")]
    pub detect_new_override: bool,

    /// Detect singletons using decorators
    #[serde(default = "default_true")]
    pub detect_decorator: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SingletonConfig {
    fn default() -> Self {
        Self {
            detect_module_level: true,
            detect_new_override: true,
            detect_decorator: true,
        }
    }
}

/// Factory pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryConfig {
    /// Detect factory functions
    #[serde(default = "default_true")]
    pub detect_functions: bool,

    /// Detect factory registries
    #[serde(default = "default_true")]
    pub detect_registries: bool,

    /// Name patterns that indicate factory functions
    #[serde(default = "default_factory_name_patterns")]
    pub name_patterns: Vec<String>,
}

fn default_factory_name_patterns() -> Vec<String> {
    vec![
        "create_".to_string(),
        "make_".to_string(),
        "build_".to_string(),
        "_factory".to_string(),
    ]
}

impl Default for FactoryConfig {
    fn default() -> Self {
        Self {
            detect_functions: true,
            detect_registries: true,
            name_patterns: default_factory_name_patterns(),
        }
    }
}

/// Strategy pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyConfig {
    /// Enable strategy pattern detection
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Callback pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackConfig {
    /// Decorator patterns that indicate callbacks
    #[serde(default = "default_callback_decorators")]
    pub decorator_patterns: Vec<String>,
}

fn default_callback_decorators() -> Vec<String> {
    vec![
        "route".to_string(),
        "handler".to_string(),
        "app.".to_string(),
    ]
}

impl Default for CallbackConfig {
    fn default() -> Self {
        Self {
            decorator_patterns: default_callback_decorators(),
        }
    }
}

/// Template method pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateMethodConfig {
    /// Enable template method pattern detection
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Custom pattern rule for user-defined patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPatternRule {
    /// Name of the custom pattern
    pub name: String,

    /// Regex pattern for matching method names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_pattern: Option<String>,

    /// Regex pattern for matching class names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_pattern: Option<String>,

    /// Regex pattern for matching decorators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decorator_pattern: Option<String>,

    /// Confidence score for this pattern (0.0 - 1.0)
    #[serde(default = "default_confidence_threshold")]
    pub confidence: f32,
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            observer: ObserverConfig::default(),
            singleton: SingletonConfig::default(),
            factory: FactoryConfig::default(),
            strategy: StrategyConfig::default(),
            callback: CallbackConfig::default(),
            template_method: TemplateMethodConfig::default(),
            custom_rules: Vec::new(),
            confidence_threshold: 0.7,
        }
    }
}

impl PatternConfig {
    /// Load configuration from .debtmap.toml
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from project directory or use defaults
    pub fn load_or_default(project_path: &Path) -> Self {
        let config_path = project_path.join(".debtmap.toml");
        if config_path.exists() {
            Self::load(&config_path).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.confidence_threshold) {
            return Err(format!(
                "confidence_threshold must be between 0.0 and 1.0, got {}",
                self.confidence_threshold
            ));
        }

        for rule in &self.custom_rules {
            if !(0.0..=1.0).contains(&rule.confidence) {
                return Err(format!(
                    "Custom rule '{}' confidence must be between 0.0 and 1.0, got {}",
                    rule.name, rule.confidence
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PatternConfig::default();
        assert!(config.enabled);
        assert_eq!(config.confidence_threshold, 0.7);
        assert!(config
            .observer
            .interface_markers
            .contains(&"ABC".to_string()));
    }

    #[test]
    fn test_load_config_from_toml() {
        let config_content = r#"
enabled = true
confidence_threshold = 0.8

[observer]
interface_markers = ["ABC", "Protocol", "CustomInterface"]
method_prefixes = ["on_", "handle_", "process_"]

[[custom_rules]]
name = "event_handler"
method_pattern = "^handle_.*_event$"
confidence = 0.85
        "#;

        let config: Result<PatternConfig, _> = toml::from_str(config_content);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.confidence_threshold, 0.8);
        assert_eq!(config.observer.interface_markers.len(), 3);
        assert_eq!(config.custom_rules.len(), 1);
        assert_eq!(config.custom_rules[0].name, "event_handler");
    }

    #[test]
    fn test_validate_valid_config() {
        let config = PatternConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_threshold() {
        let config = PatternConfig {
            confidence_threshold: 1.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_custom_rule_confidence() {
        let mut config = PatternConfig::default();
        config.custom_rules.push(CustomPatternRule {
            name: "test".to_string(),
            method_pattern: None,
            class_pattern: None,
            decorator_pattern: None,
            confidence: 2.0,
        });
        assert!(config.validate().is_err());
    }
}
