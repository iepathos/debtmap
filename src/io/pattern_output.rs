//! Enhanced output formatting for pattern detection
//!
//! Provides colored, informative output for detected design patterns
//! with confidence scores and reasoning.

use crate::analysis::patterns::{PatternInstance, PatternType};
use colored::*;

/// Formatter for pattern detection output
pub struct PatternOutputFormatter {
    /// Use plain output (no colors or emoji)
    plain: bool,
}

impl PatternOutputFormatter {
    /// Create a new formatter
    pub fn new(plain: bool) -> Self {
        Self { plain }
    }

    /// Format a pattern instance with full details
    pub fn format_pattern_usage(&self, pattern: &PatternInstance) -> String {
        let mut output = String::new();

        // Pattern type and confidence
        let pattern_label = self.format_pattern_type(&pattern.pattern_type);
        let confidence_pct = (pattern.confidence * 100.0) as u32;

        output.push_str(&format!(
            "[{}] Confidence: {}%\n",
            pattern_label, confidence_pct
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

    /// Format pattern type with color
    fn format_pattern_type(&self, pattern_type: &PatternType) -> String {
        let label = match pattern_type {
            PatternType::Observer => "OBSERVER PATTERN",
            PatternType::Singleton => "SINGLETON PATTERN",
            PatternType::Factory => "FACTORY PATTERN",
            PatternType::Strategy => "STRATEGY PATTERN",
            PatternType::Callback => "CALLBACK PATTERN",
            PatternType::TemplateMethod => "TEMPLATE METHOD",
            PatternType::DependencyInjection => "DEPENDENCY INJECTION",
        };

        if self.plain {
            label.to_string()
        } else {
            match pattern_type {
                PatternType::Observer => label.green().to_string(),
                PatternType::Singleton => label.blue().to_string(),
                PatternType::Factory => label.yellow().to_string(),
                PatternType::Strategy => label.cyan().to_string(),
                PatternType::Callback => label.magenta().to_string(),
                PatternType::TemplateMethod => label.bright_blue().to_string(),
                PatternType::DependencyInjection => label.bright_green().to_string(),
            }
        }
    }

    /// Format a function with pattern information
    pub fn format_with_pattern(
        &self,
        function_name: &str,
        file_path: &str,
        line: usize,
        pattern: Option<&PatternInstance>,
    ) -> String {
        if let Some(pattern) = pattern {
            let mut output = if self.plain {
                format!(
                    "X {} ({}:{})\n   USED VIA PATTERN: {}\n",
                    function_name,
                    file_path,
                    line,
                    self.format_pattern_type(&pattern.pattern_type)
                )
            } else {
                format!(
                    "❌ {} ({}:{})\n   ✅ USED VIA PATTERN: {}\n",
                    function_name,
                    file_path,
                    line,
                    self.format_pattern_type(&pattern.pattern_type)
                )
            };
            output.push_str(&self.format_pattern_usage(pattern));
            output
        } else if self.plain {
            format!(
                "X {} ({}:{})\n   ! No pattern usage detected - likely dead code\n",
                function_name, file_path, line
            )
        } else {
            format!(
                "❌ {} ({}:{})\n   ⚠️  No pattern usage detected - likely dead code\n",
                function_name, file_path, line
            )
        }
    }

    /// Format a confidence warning
    pub fn format_warning(&self, pattern: &PatternInstance, threshold: f32) -> String {
        if pattern.confidence < threshold {
            if self.plain {
                format!(
                    "  ! WARNING: Low confidence pattern detection ({}% < {}%)\n",
                    (pattern.confidence * 100.0) as u32,
                    (threshold * 100.0) as u32
                )
            } else {
                format!(
                    "  ⚠️  WARNING: Low confidence pattern detection ({}% < {}%)\n",
                    (pattern.confidence * 100.0) as u32,
                    (threshold * 100.0) as u32
                )
            }
        } else {
            String::new()
        }
    }
}

impl Default for PatternOutputFormatter {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::patterns::{Implementation, UsageSite};
    use std::path::PathBuf;

    fn create_test_pattern() -> PatternInstance {
        PatternInstance {
            pattern_type: PatternType::Observer,
            confidence: 0.95,
            base_class: Some("Observer".to_string()),
            implementations: vec![Implementation {
                file: PathBuf::from("test.py"),
                class_name: Some("ConcreteObserver".to_string()),
                function_name: "on_event".to_string(),
                line: 10,
            }],
            usage_sites: vec![UsageSite {
                file: PathBuf::from("test.py"),
                line: 20,
                context: "observer.on_event() call".to_string(),
            }],
            reasoning: "Observer pattern detected".to_string(),
        }
    }

    #[test]
    fn test_format_pattern_usage() {
        let formatter = PatternOutputFormatter::new(true);
        let pattern = create_test_pattern();
        let output = formatter.format_pattern_usage(&pattern);

        assert!(output.contains("OBSERVER PATTERN"));
        assert!(output.contains("Confidence: 95%"));
        assert!(output.contains("Reasoning: Observer pattern detected"));
        assert!(output.contains("Implementations:"));
        assert!(output.contains("on_event"));
    }

    #[test]
    fn test_format_with_pattern() {
        let formatter = PatternOutputFormatter::new(true);
        let pattern = create_test_pattern();
        let output = formatter.format_with_pattern("on_event", "test.py", 10, Some(&pattern));

        assert!(output.contains("on_event"));
        assert!(output.contains("USED VIA PATTERN"));
    }

    #[test]
    fn test_format_without_pattern() {
        let formatter = PatternOutputFormatter::new(true);
        let output = formatter.format_with_pattern("unused_func", "test.py", 10, None);

        assert!(output.contains("unused_func"));
        assert!(output.contains("likely dead code"));
    }

    #[test]
    fn test_format_warning() {
        let formatter = PatternOutputFormatter::new(true);
        let mut pattern = create_test_pattern();
        pattern.confidence = 0.6;

        let warning = formatter.format_warning(&pattern, 0.7);
        assert!(warning.contains("WARNING"));
        assert!(warning.contains("60%"));
        assert!(warning.contains("70%"));
    }

    #[test]
    fn test_format_no_warning_high_confidence() {
        let formatter = PatternOutputFormatter::new(true);
        let pattern = create_test_pattern();

        let warning = formatter.format_warning(&pattern, 0.7);
        assert!(warning.is_empty());
    }
}
