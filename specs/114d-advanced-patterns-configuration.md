---
number: 114d
title: Advanced Patterns and Configuration
category: foundation
priority: high
status: draft
dependencies: [114a, 114b, 114c]
created: 2025-10-16
---

# Specification 114d: Advanced Patterns and Configuration

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 114a (Parser Enhancements), Spec 114b (Local Pattern Detection), Spec 114c (Cross-File Integration)

## Context

This is Phase 4 (Final) of the Design Pattern Recognition feature (Spec 114). With core pattern detection working, we now add:
- Additional pattern types (Strategy, Template Method, Dependency Injection)
- User-configurable pattern matching rules
- Pattern confidence scoring and warnings
- Enhanced output with pattern reasoning

**Why Configuration Matters**: Every project has unique patterns:
- Custom decorators for callbacks
- Project-specific naming conventions
- Framework-specific patterns (Django, Flask, FastAPI)
- Domain-specific design patterns

Users need to configure pattern detection for their specific codebase.

## Objective

Implement Strategy, Template Method, and Dependency Injection pattern detection, add user-configurable pattern rules, provide confidence scores, and enhance output to show pattern usage reasoning.

## Requirements

### Functional Requirements

1. **Strategy Pattern Detection**
   - Identify strategy interfaces (Protocol, ABC)
   - Detect strategy implementations
   - Track strategy injection (constructor, setter)
   - Mark strategy methods as "used via pattern"

2. **Template Method Pattern Detection**
   - Identify template method base classes
   - Detect overridden template methods
   - Mark overridden methods as "used via inheritance"

3. **Dependency Injection Detection**
   - Detect constructor injection patterns
   - Identify injected dependencies
   - Track framework-based DI decorators
   - Mark injected methods as "potentially used"

4. **Pattern Configuration**
   - User-defined pattern matching rules
   - Configurable pattern thresholds
   - Custom decorator patterns
   - Project-specific naming conventions

5. **Confidence Scoring**
   - Confidence score for each pattern detection (0.0 - 1.0)
   - Threshold for marking as "used"
   - Warning mode for uncertain patterns

6. **Enhanced Output**
   - Show pattern type in dead code report
   - Display pattern reasoning
   - List pattern usage sites
   - Confidence scores for uncertain detections

### Non-Functional Requirements

1. **Accuracy**
   - Pattern detection precision > 90%
   - False positive reduction: 20% → < 2%
   - No false negatives for configured patterns

2. **Performance**
   - Total pattern detection overhead < 10%
   - Efficient configuration parsing
   - Cached pattern rules

3. **Usability**
   - Simple configuration format (TOML)
   - Clear error messages for invalid config
   - Sensible defaults
   - Easy to add custom patterns

## Acceptance Criteria

- [ ] Strategy pattern detected (protocol implementations)
- [ ] Template method pattern detected (overridden methods)
- [ ] Dependency injection detected (constructor injection)
- [ ] Configuration file format defined (`.debtmap.toml`)
- [ ] Custom pattern rules supported
- [ ] Confidence scores computed for all patterns
- [ ] Warning mode for uncertain patterns
- [ ] Enhanced output shows pattern reasoning
- [ ] CLI option to disable pattern detection
- [ ] CLI option to enable specific patterns only
- [ ] False positive rate < 2% on promptconstruct-frontend
- [ ] Performance overhead < 10% total
- [ ] Documentation with configuration examples
- [ ] Integration tests with custom patterns

## Technical Details

### Strategy Pattern Recognizer

```rust
// src/analysis/patterns/strategy.rs
use super::{PatternInstance, PatternRecognizer, PatternType, Implementation};
use crate::core::{FileMetrics, FunctionDef, ClassDef};

pub struct StrategyPatternRecognizer;

impl StrategyPatternRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if class is a strategy interface
    fn is_strategy_interface(&self, class: &ClassDef) -> bool {
        // Strategy interfaces often use Protocol or ABC
        let has_protocol_base = class.base_classes.iter().any(|b| {
            b.contains("Protocol") || b.contains("ABC") || b.contains("Strategy")
        });

        // Strategy interfaces typically have one or more abstract methods
        let has_abstract_methods = class.methods.iter().any(|m| m.is_abstract);

        has_protocol_base && has_abstract_methods
    }

    /// Find strategy implementations
    fn find_strategy_implementations(&self, interface: &ClassDef, file_metrics: &FileMetrics) -> Vec<Implementation> {
        file_metrics
            .classes
            .iter()
            .filter(|class| class.base_classes.contains(&interface.name))
            .flat_map(|class| {
                class.methods.iter().filter_map(|method| {
                    if interface.methods.iter().any(|m| m.name == method.name) {
                        Some(Implementation {
                            file: file_metrics.path.clone(),
                            class_name: Some(class.name.clone()),
                            function_name: method.name.clone(),
                            line: method.line,
                        })
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// Detect strategy injection (constructor parameter with strategy type)
    fn find_strategy_injections(&self, interface_name: &str, file_metrics: &FileMetrics) -> Vec<UsageSite> {
        let mut usage_sites = Vec::new();

        for class in &file_metrics.classes {
            // Look for __init__ methods with strategy parameter
            for method in &class.methods {
                if method.name == "__init__" {
                    // Check method parameters (simplified - would need AST parsing)
                    // If parameter type annotation matches interface, it's injection
                    usage_sites.push(UsageSite {
                        file: file_metrics.path.clone(),
                        line: method.line,
                        context: format!("Strategy injection in {}.{}()", class.name, method.name),
                    });
                }
            }
        }

        usage_sites
    }
}

impl PatternRecognizer for StrategyPatternRecognizer {
    fn name(&self) -> &str {
        "Strategy"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        for class in &file_metrics.classes {
            if self.is_strategy_interface(class) {
                let implementations = self.find_strategy_implementations(class, file_metrics);
                let injections = self.find_strategy_injections(&class.name, file_metrics);

                if !implementations.is_empty() || !injections.is_empty() {
                    patterns.push(PatternInstance {
                        pattern_type: PatternType::Strategy,
                        confidence: 0.85,
                        base_class: Some(class.name.clone()),
                        implementations,
                        usage_sites: injections,
                        reasoning: format!("Strategy interface {} with injection points", class.name),
                    });
                }
            }
        }

        patterns
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionDef,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        let class_name = function.class_name.as_ref()?;
        let class = file_metrics.classes.iter().find(|c| &c.name == class_name)?;

        // Check if class implements a strategy interface
        for base_name in &class.base_classes {
            if let Some(base_class) = file_metrics.classes.iter().find(|c| &c.name == base_name) {
                if self.is_strategy_interface(base_class) {
                    return Some(PatternInstance {
                        pattern_type: PatternType::Strategy,
                        confidence: 0.8,
                        base_class: Some(base_name.clone()),
                        implementations: vec![Implementation {
                            file: file_metrics.path.clone(),
                            class_name: Some(class_name.clone()),
                            function_name: function.name.clone(),
                            line: function.start_line,
                        }],
                        usage_sites: Vec::new(),
                        reasoning: format!("Implements strategy method {} from {}", function.name, base_name),
                    });
                }
            }
        }

        None
    }
}
```

### Template Method Pattern Recognizer

```rust
// src/analysis/patterns/template_method.rs
use super::{PatternInstance, PatternRecognizer, PatternType, Implementation};
use crate::core::{FileMetrics, FunctionDef, ClassDef};

pub struct TemplateMethodPatternRecognizer;

impl TemplateMethodPatternRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if method is a template method (calls other methods that can be overridden)
    fn is_template_method(&self, method: &MethodDef, class: &ClassDef) -> bool {
        // Template methods typically call other methods in the same class
        // This would require AST analysis to detect method calls
        // For now, heuristic: non-abstract method in class with abstract methods
        !method.is_abstract && class.methods.iter().any(|m| m.is_abstract)
    }

    /// Find overridden template methods
    fn find_overridden_methods(&self, base_class: &ClassDef, file_metrics: &FileMetrics) -> Vec<Implementation> {
        file_metrics
            .classes
            .iter()
            .filter(|class| class.base_classes.contains(&base_class.name))
            .flat_map(|class| {
                class.methods.iter().filter_map(|method| {
                    // Check if method overrides a base class method
                    if base_class.methods.iter().any(|m| m.name == method.name) {
                        Some(Implementation {
                            file: file_metrics.path.clone(),
                            class_name: Some(class.name.clone()),
                            function_name: method.name.clone(),
                            line: method.line,
                        })
                    } else {
                        None
                    }
                })
            })
            .collect()
    }
}

impl PatternRecognizer for TemplateMethodPatternRecognizer {
    fn name(&self) -> &str {
        "TemplateMethod"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        for class in &file_metrics.classes {
            // Look for classes with template methods
            let has_template_method = class.methods.iter().any(|m| self.is_template_method(m, class));

            if has_template_method {
                let overridden = self.find_overridden_methods(class, file_metrics);

                if !overridden.is_empty() {
                    patterns.push(PatternInstance {
                        pattern_type: PatternType::TemplateMethod,
                        confidence: 0.75, // Lower confidence (heuristic-based)
                        base_class: Some(class.name.clone()),
                        implementations: overridden,
                        usage_sites: Vec::new(),
                        reasoning: format!("Template method pattern in {}", class.name),
                    });
                }
            }
        }

        patterns
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionDef,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        let class_name = function.class_name.as_ref()?;
        let class = file_metrics.classes.iter().find(|c| &c.name == class_name)?;

        // Check if method overrides a template method
        for base_name in &class.base_classes {
            if let Some(base_class) = file_metrics.classes.iter().find(|c| &c.name == base_name) {
                if base_class.methods.iter().any(|m| m.name == function.name) {
                    return Some(PatternInstance {
                        pattern_type: PatternType::TemplateMethod,
                        confidence: 0.7,
                        base_class: Some(base_name.clone()),
                        implementations: vec![Implementation {
                            file: file_metrics.path.clone(),
                            class_name: Some(class_name.clone()),
                            function_name: function.name.clone(),
                            line: function.start_line,
                        }],
                        usage_sites: Vec::new(),
                        reasoning: format!("Overrides template method {} from {}", function.name, base_name),
                    });
                }
            }
        }

        None
    }
}
```

### Pattern Configuration

```rust
// src/analysis/patterns/config.rs
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default)]
    pub observer: ObserverConfig,

    #[serde(default)]
    pub singleton: SingletonConfig,

    #[serde(default)]
    pub factory: FactoryConfig,

    #[serde(default)]
    pub strategy: StrategyConfig,

    #[serde(default)]
    pub callback: CallbackConfig,

    #[serde(default)]
    pub template_method: TemplateMethodConfig,

    #[serde(default)]
    pub custom_rules: Vec<CustomPatternRule>,

    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f32,
}

fn default_enabled() -> bool { true }
fn default_confidence_threshold() -> f32 { 0.7 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverConfig {
    #[serde(default = "default_interface_markers")]
    pub interface_markers: Vec<String>,

    #[serde(default = "default_registration_methods")]
    pub registration_methods: Vec<String>,

    #[serde(default = "default_method_prefixes")]
    pub method_prefixes: Vec<String>,
}

fn default_interface_markers() -> Vec<String> {
    vec!["ABC".to_string(), "Protocol".to_string(), "Interface".to_string()]
}

fn default_registration_methods() -> Vec<String> {
    vec!["add_observer".to_string(), "register".to_string(), "subscribe".to_string()]
}

fn default_method_prefixes() -> Vec<String> {
    vec!["on_".to_string(), "handle_".to_string(), "notify_".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SingletonConfig {
    #[serde(default = "default_true")]
    pub detect_module_level: bool,

    #[serde(default = "default_true")]
    pub detect_new_override: bool,

    #[serde(default = "default_true")]
    pub detect_decorator: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryConfig {
    #[serde(default = "default_true")]
    pub detect_functions: bool,

    #[serde(default = "default_true")]
    pub detect_registries: bool,

    #[serde(default = "default_factory_name_patterns")]
    pub name_patterns: Vec<String>,
}

fn default_factory_name_patterns() -> Vec<String> {
    vec!["create_".to_string(), "make_".to_string(), "build_".to_string(), "_factory".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyConfig {
    // Strategy-specific config
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CallbackConfig {
    #[serde(default = "default_callback_decorators")]
    pub decorator_patterns: Vec<String>,
}

fn default_callback_decorators() -> Vec<String> {
    vec!["route".to_string(), "handler".to_string(), "app.".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateMethodConfig {
    // Template method config
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPatternRule {
    pub name: String,
    pub method_pattern: Option<String>, // Regex pattern
    pub class_pattern: Option<String>,
    pub decorator_pattern: Option<String>,
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

impl Default for ObserverConfig {
    fn default() -> Self {
        Self {
            interface_markers: default_interface_markers(),
            registration_methods: default_registration_methods(),
            method_prefixes: default_method_prefixes(),
        }
    }
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

impl PatternConfig {
    /// Load configuration from .debtmap.toml
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: PatternConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Merge with default configuration
    pub fn with_defaults(self) -> Self {
        // Ensure defaults are filled in
        self
    }
}
```

### Enhanced Output Formatter

```rust
// src/io/pattern_output.rs
use crate::analysis::patterns::{PatternInstance, PatternType};
use colored::*;

pub struct PatternOutputFormatter;

impl PatternOutputFormatter {
    pub fn format_pattern_usage(&self, pattern: &PatternInstance) -> String {
        let mut output = String::new();

        // Pattern type and confidence
        output.push_str(&format!(
            "[{}] Confidence: {:.0}%\n",
            self.format_pattern_type(&pattern.pattern_type),
            pattern.confidence * 100.0
        ));

        // Reasoning
        output.push_str(&format!("  Reasoning: {}\n", pattern.reasoning));

        // Implementations
        if !pattern.implementations.is_empty() {
            output.push_str("  Implementations:\n");
            for impl_info in &pattern.implementations {
                output.push_str(&format!(
                    "    - {}:{}:{}\n",
                    impl_info.file.display(),
                    impl_info.line,
                    impl_info.function_name
                ));
            }
        }

        // Usage sites
        if !pattern.usage_sites.is_empty() {
            output.push_str("  Usage sites:\n");
            for usage in &pattern.usage_sites {
                output.push_str(&format!(
                    "    - {}:{} {}\n",
                    usage.file.display(),
                    usage.line,
                    usage.context
                ));
            }
        }

        output
    }

    fn format_pattern_type(&self, pattern_type: &PatternType) -> ColoredString {
        match pattern_type {
            PatternType::Observer => "OBSERVER PATTERN".green(),
            PatternType::Singleton => "SINGLETON PATTERN".blue(),
            PatternType::Factory => "FACTORY PATTERN".yellow(),
            PatternType::Strategy => "STRATEGY PATTERN".cyan(),
            PatternType::Callback => "CALLBACK PATTERN".magenta(),
            PatternType::TemplateMethod => "TEMPLATE METHOD".bright_blue(),
            PatternType::DependencyInjection => "DEPENDENCY INJECTION".bright_green(),
        }
    }

    pub fn format_dead_code_with_pattern(&self, function: &FunctionDef, pattern: Option<&PatternInstance>) -> String {
        if let Some(pattern) = pattern {
            format!(
                "❌ {} ({}:{})\n   ✅ USED VIA PATTERN: {}\n{}",
                function.name,
                function.file.display(),
                function.start_line,
                self.format_pattern_type(&pattern.pattern_type),
                self.format_pattern_usage(pattern)
            )
        } else {
            format!(
                "❌ {} ({}:{})\n   ⚠️  No pattern usage detected - likely dead code",
                function.name,
                function.file.display(),
                function.start_line
            )
        }
    }
}
```

### CLI Integration

```rust
// src/cli.rs - Add pattern-related options
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "debtmap")]
#[command(about = "Code complexity and technical debt analyzer")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Analyze {
        /// Directory to analyze
        path: PathBuf,

        /// Disable pattern recognition
        #[arg(long = "no-pattern-detection")]
        no_pattern_detection: bool,

        /// Enable specific patterns only (comma-separated: observer,singleton,factory)
        #[arg(long = "patterns")]
        patterns: Option<String>,

        /// Pattern confidence threshold (0.0 - 1.0)
        #[arg(long = "pattern-threshold", default_value = "0.7")]
        pattern_threshold: f32,

        /// Show pattern warnings for uncertain detections
        #[arg(long = "show-pattern-warnings")]
        show_pattern_warnings: bool,
    },
}
```

## Dependencies

- **Prerequisites**: Spec 114a (Parser Enhancements), Spec 114b (Local Pattern Detection), Spec 114c (Cross-File Integration)
- **Affected Components**:
  - `src/analysis/patterns/strategy.rs` - **New**
  - `src/analysis/patterns/template_method.rs` - **New**
  - `src/analysis/patterns/config.rs` - **New**
  - `src/io/pattern_output.rs` - **New**
  - `src/cli.rs` - Add pattern options
- **External Dependencies**: `toml` crate for configuration parsing, `colored` for output formatting

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_pattern_config() {
        let config_content = r#"
[patterns]
enabled = true
confidence_threshold = 0.8

[patterns.observer]
interface_markers = ["ABC", "Protocol", "CustomInterface"]
method_prefixes = ["on_", "handle_", "process_"]

[[patterns.custom_rules]]
name = "event_handler"
method_pattern = "^handle_.*_event$"
confidence = 0.85
        "#;

        let config: PatternConfig = toml::from_str(config_content).unwrap();

        assert_eq!(config.confidence_threshold, 0.8);
        assert_eq!(config.observer.interface_markers.len(), 3);
        assert_eq!(config.custom_rules.len(), 1);
        assert_eq!(config.custom_rules[0].name, "event_handler");
    }

    #[test]
    fn test_strategy_pattern_detection() {
        let file_metrics = create_test_file_metrics_with_strategy_pattern();
        let recognizer = StrategyPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::Strategy);
    }

    #[test]
    fn test_template_method_pattern_detection() {
        let file_metrics = create_test_file_metrics_with_template_method();
        let recognizer = TemplateMethodPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::TemplateMethod);
    }
}
```

### Integration Tests

Test with `.debtmap.toml` configuration:

```toml
# tests/fixtures/pattern_config/.debtmap.toml
[patterns]
enabled = true
confidence_threshold = 0.75

[patterns.observer]
interface_markers = ["ABC", "Protocol", "Observer"]
registration_methods = ["add_observer", "subscribe", "register"]
method_prefixes = ["on_", "handle_"]

[patterns.callback]
decorator_patterns = ["app.route", "api.handler", "event.on"]

[[patterns.custom_rules]]
name = "django_view"
method_pattern = "^(get|post|put|delete)$"
class_pattern = ".*View$"
confidence = 0.9
```

Test that custom patterns are detected correctly.

## Documentation Requirements

### User Documentation

```markdown
## Pattern Detection Configuration

Create `.debtmap.toml` in your project root:

```toml
[patterns]
enabled = true
confidence_threshold = 0.7

[patterns.observer]
interface_markers = ["ABC", "Protocol", "Interface"]
method_prefixes = ["on_", "handle_", "notify_"]

[[patterns.custom_rules]]
name = "event_handler"
method_pattern = "^handle_.*_event$"
confidence = 0.85
```

### CLI Usage

```bash
# Disable pattern detection
debtmap analyze src --no-pattern-detection

# Enable specific patterns only
debtmap analyze src --patterns observer,singleton

# Set custom confidence threshold
debtmap analyze src --pattern-threshold 0.8

# Show warnings for uncertain patterns
debtmap analyze src --show-pattern-warnings
```

### Output Example

```
#5 ConversationPanel.on_message_added (conversation_panel.py:583)
   ✅ USED VIA PATTERN: OBSERVER PATTERN
   [OBSERVER PATTERN] Confidence: 95%
     Reasoning: Implements abstract method on_message_added from ConversationObserver
     Implementations:
       - conversation_panel.py:583:on_message_added
     Usage sites:
       - conversation_manager.py:137 for observer in self.observers: observer.on_message_added()
```

## Implementation Notes

### Configuration Loading
- Load `.debtmap.toml` from project root
- Merge with default configuration
- Validate configuration values
- Provide clear error messages for invalid config

### Custom Pattern Matching
- Use regex for pattern matching
- Cache compiled regex patterns
- Support both method and class patterns
- Allow confidence score override

### Performance
- Load configuration once at startup
- Cache pattern detection results
- Use efficient regex matching
- Minimize repeated AST traversals

## Success Metrics

- [ ] Strategy pattern detection: 85%+ accuracy
- [ ] Template method detection: 75%+ accuracy
- [ ] Configuration parsing: 100% success rate
- [ ] Custom patterns work correctly
- [ ] Enhanced output is readable and informative
- [ ] False positive rate < 2% overall
- [ ] Performance overhead < 10% total
- [ ] User documentation complete with examples
