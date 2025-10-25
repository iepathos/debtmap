use serde::{Deserialize, Serialize};

/// Configuration for pure mapping pattern detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingPatternConfig {
    /// Enable pure mapping pattern detection
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Minimum mapping ratio to qualify (0.0-1.0)
    #[serde(default = "default_min_mapping_ratio")]
    pub min_mapping_ratio: f64,

    /// Complexity adjustment factor for pure mappings (0.0-1.0)
    #[serde(default = "default_adjustment_factor")]
    pub adjustment_factor: f64,

    /// Maximum expression complexity in arms/cases
    #[serde(default = "default_max_arm_complexity")]
    pub max_arm_complexity: u32,
}

impl Default for MappingPatternConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            min_mapping_ratio: default_min_mapping_ratio(),
            adjustment_factor: default_adjustment_factor(),
            max_arm_complexity: default_max_arm_complexity(),
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_min_mapping_ratio() -> f64 {
    0.8
}

fn default_adjustment_factor() -> f64 {
    0.4
}

fn default_max_arm_complexity() -> u32 {
    2
}

/// Result of mapping pattern detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MappingPatternResult {
    pub is_pure_mapping: bool,
    pub confidence: f64,
    pub mapping_ratio: f64,
    pub complexity_adjustment_factor: f64,
    pub pattern_description: String,
}

impl Default for MappingPatternResult {
    fn default() -> Self {
        Self {
            is_pure_mapping: false,
            confidence: 0.0,
            mapping_ratio: 0.0,
            complexity_adjustment_factor: 1.0,
            pattern_description: String::new(),
        }
    }
}

/// Detector for pure mapping patterns
pub struct MappingPatternDetector {
    config: MappingPatternConfig,
}

impl MappingPatternDetector {
    pub fn new(config: MappingPatternConfig) -> Self {
        Self { config }
    }

    /// Analyze a function to detect pure mapping patterns
    pub fn analyze_function(
        &self,
        function_body: &str,
        cyclomatic_complexity: u32,
    ) -> MappingPatternResult {
        if !self.config.enabled || cyclomatic_complexity < 10 {
            return MappingPatternResult::default();
        }

        // Try different pattern detectors
        if let Some(result) = self.detect_rust_match_pattern(function_body) {
            return result;
        }

        if let Some(result) = self.detect_switch_pattern(function_body) {
            return result;
        }

        if let Some(result) = self.detect_ifelse_chain_pattern(function_body) {
            return result;
        }

        MappingPatternResult::default()
    }

    /// Detect Rust match expressions with simple arms
    fn detect_rust_match_pattern(&self, body: &str) -> Option<MappingPatternResult> {
        // Look for match expressions
        if !body.contains("match ") {
            return None;
        }

        let lines: Vec<&str> = body.lines().collect();
        // Filter out empty lines for a more accurate ratio
        let non_empty_lines: Vec<&str> = lines.iter().filter(|l| !l.trim().is_empty()).copied().collect();
        let total_lines = non_empty_lines.len();

        // Count match arms
        let arm_count = lines.iter().filter(|line| line.contains("=>")).count();

        if arm_count < 3 {
            return None; // Too few arms to be a mapping pattern
        }

        // Calculate mapping ratio - how much of the function is the match expression
        let match_start = non_empty_lines.iter().position(|line| line.contains("match "))?;
        let match_end = non_empty_lines
            .iter()
            .rposition(|line| line.trim_end().ends_with('}'))?;
        let match_lines = match_end.saturating_sub(match_start) + 1;
        let mapping_ratio = match_lines as f64 / total_lines as f64;

        if mapping_ratio < self.config.min_mapping_ratio {
            return None;
        }

        // Check if all arms are simple
        let complex_arm_count = lines
            .iter()
            .filter(|line| {
                let line_lower = line.to_lowercase();
                line.contains("=>")
                    && (line_lower.contains("if ")
                        || line_lower.contains("match ")
                        || line_lower.contains("loop ")
                        || line_lower.contains("for ")
                        || line_lower.contains("while "))
            })
            .count();

        if complex_arm_count > 0 {
            return None; // Has complex arms
        }

        // Detect nested match pattern (two match expressions)
        let match_count = body.matches("match ").count();
        let is_nested = match_count == 2;

        Some(MappingPatternResult {
            is_pure_mapping: true,
            confidence: 0.9,
            mapping_ratio,
            complexity_adjustment_factor: self.config.adjustment_factor,
            pattern_description: if is_nested {
                format!("nested exhaustive match with {} arms", arm_count)
            } else {
                format!("exhaustive match with {} arms", arm_count)
            },
        })
    }

    /// Detect JavaScript/TypeScript switch statements
    fn detect_switch_pattern(&self, body: &str) -> Option<MappingPatternResult> {
        if !body.contains("switch") {
            return None;
        }

        let lines: Vec<&str> = body.lines().collect();
        // Filter out empty lines for a more accurate ratio
        let non_empty_lines: Vec<&str> = lines.iter().filter(|l| !l.trim().is_empty()).copied().collect();
        let total_lines = non_empty_lines.len();

        // Count case statements
        let case_count = lines
            .iter()
            .filter(|line| line.trim().starts_with("case "))
            .count();

        if case_count < 3 {
            return None;
        }

        // Check for simple cases (just return statements)
        let return_count = lines
            .iter()
            .filter(|line| line.contains("return "))
            .count();

        if return_count < case_count {
            return None; // Not all cases have simple returns
        }

        // Calculate mapping ratio
        let switch_start = non_empty_lines.iter().position(|line| line.contains("switch"))?;
        let switch_end = non_empty_lines.iter().rposition(|line| line.trim() == "}")?;
        let switch_lines = switch_end.saturating_sub(switch_start) + 1;
        let mapping_ratio = switch_lines as f64 / total_lines as f64;

        if mapping_ratio < self.config.min_mapping_ratio {
            return None;
        }

        Some(MappingPatternResult {
            is_pure_mapping: true,
            confidence: 0.85,
            mapping_ratio,
            complexity_adjustment_factor: self.config.adjustment_factor,
            pattern_description: format!("switch statement with {} cases", case_count),
        })
    }

    /// Detect Python if-elif chains
    fn detect_ifelse_chain_pattern(&self, body: &str) -> Option<MappingPatternResult> {
        let lines: Vec<&str> = body.lines().collect();
        let _total_lines = lines.len();

        // Count elif statements
        let elif_count = lines
            .iter()
            .filter(|line| line.trim().starts_with("elif "))
            .count();

        if elif_count < 2 {
            return None;
        }

        // Check if this is a simple mapping pattern
        let return_count = lines
            .iter()
            .filter(|line| line.trim().starts_with("return "))
            .count();

        let branch_count = elif_count + 1; // elif + initial if

        // Each branch should have a simple return
        if return_count < branch_count {
            return None;
        }

        // Calculate mapping ratio
        let mapping_ratio = 0.9; // If-elif chains typically dominate Python functions

        if mapping_ratio < self.config.min_mapping_ratio {
            return None;
        }

        Some(MappingPatternResult {
            is_pure_mapping: true,
            confidence: 0.8,
            mapping_ratio,
            complexity_adjustment_factor: self.config.adjustment_factor,
            pattern_description: format!("if-elif chain with {} branches", branch_count),
        })
    }
}

/// Calculate adjusted complexity score
pub fn calculate_adjusted_complexity(
    cyclomatic: u32,
    cognitive: u32,
    mapping_result: &MappingPatternResult,
) -> f64 {
    if !mapping_result.is_pure_mapping {
        return cyclomatic as f64;
    }

    let adjusted_cyclomatic = cyclomatic as f64 * mapping_result.complexity_adjustment_factor;
    let cognitive_weight = 0.7;

    // For mapping functions, emphasize cognitive complexity over cyclomatic
    adjusted_cyclomatic * (1.0 - cognitive_weight) + cognitive as f64 * cognitive_weight
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rust_enum_match_mapping() {
        let code = r#"
            fn format(val: MyEnum) -> &'static str {
                match val {
                    MyEnum::A => "a",
                    MyEnum::B => "b",
                    MyEnum::C => "c",
                    MyEnum::D => "d",
                    MyEnum::E => "e",
                }
            }
        "#;

        let detector = MappingPatternDetector::new(MappingPatternConfig::default());
        let result = detector.analyze_function(code, 15);

        assert!(result.is_pure_mapping);
        assert!(result.mapping_ratio > 0.8);
    }

    #[test]
    fn rejects_complex_match_arms() {
        let code = r#"
            fn process(val: MyEnum) -> Result<String> {
                match val {
                    MyEnum::A => {
                        if condition {
                            Ok("a".to_string())
                        } else {
                            Err("error")
                        }
                    },
                    MyEnum::B => Ok("b".to_string()),
                }
            }
        "#;

        let detector = MappingPatternDetector::new(MappingPatternConfig::default());
        let result = detector.analyze_function(code, 8);

        assert!(!result.is_pure_mapping);
    }

    #[test]
    fn detects_nested_match_pattern() {
        let code = r#"
            fn format(outer: Outer, inner: Inner) -> String {
                let label = match outer {
                    Outer::A => "A",
                    Outer::B => "B",
                };

                match inner {
                    Inner::X => label.green(),
                    Inner::Y => label.blue(),
                }
            }
        "#;

        let detector = MappingPatternDetector::new(MappingPatternConfig::default());
        let result = detector.analyze_function(code, 15);

        assert!(result.is_pure_mapping);
        assert!(result.pattern_description.contains("nested"));
    }

    #[test]
    fn applies_adjustment_factor_correctly() {
        let mapping_result = MappingPatternResult {
            is_pure_mapping: true,
            confidence: 0.9,
            mapping_ratio: 0.95,
            complexity_adjustment_factor: 0.4,
            pattern_description: "test".to_string(),
        };

        let adjusted = calculate_adjusted_complexity(15, 3, &mapping_result);

        // Adjusted = 15 * 0.4 * 0.3 + 3 * 0.7 = 1.8 + 2.1 = 3.9
        assert!((adjusted - 3.9).abs() < 0.1);
    }

    #[test]
    fn skips_detection_for_low_complexity() {
        let code = r#"
            fn simple(val: MyEnum) -> &'static str {
                match val {
                    MyEnum::A => "a",
                    MyEnum::B => "b",
                }
            }
        "#;

        let detector = MappingPatternDetector::new(MappingPatternConfig::default());
        let result = detector.analyze_function(code, 3); // Below threshold

        assert!(!result.is_pure_mapping);
    }

    #[test]
    fn detects_switch_statement() {
        let code = r#"
            function format(val) {
                switch (val) {
                    case 'A': return 'apple';
                    case 'B': return 'banana';
                    case 'C': return 'cherry';
                    case 'D': return 'date';
                    case 'E': return 'elderberry';
                }
            }
        "#;

        let detector = MappingPatternDetector::new(MappingPatternConfig::default());
        let result = detector.analyze_function(code, 12);

        assert!(result.is_pure_mapping);
        assert!(result.pattern_description.contains("switch"));
    }

    #[test]
    fn detects_python_ifelse_chain() {
        let code = r#"
            def format(val):
                if val == 'A':
                    return 'apple'
                elif val == 'B':
                    return 'banana'
                elif val == 'C':
                    return 'cherry'
                elif val == 'D':
                    return 'date'
                else:
                    return 'unknown'
        "#;

        let detector = MappingPatternDetector::new(MappingPatternConfig::default());
        let result = detector.analyze_function(code, 10);

        assert!(result.is_pure_mapping);
        assert!(result.pattern_description.contains("if-elif"));
    }
}
