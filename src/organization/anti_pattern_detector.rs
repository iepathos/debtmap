//! Anti-Pattern Detection for Code Quality
//!
//! Detects violations of idiomatic Rust and functional programming principles
//! in module split recommendations, providing corrective guidance.
//!
//! This module implements Spec 183, identifying anti-patterns such as:
//! - Utilities modules (catch-all modules with mixed responsibilities)
//! - Technical groupings (verb-based instead of type-based organization)
//! - Parameter passing (functions with too many parameters)
//! - Mixed data types (modules operating on unrelated types)

use crate::analyzers::type_registry::MethodSignature;
use crate::organization::god_object_analysis::ModuleSplit;
use std::collections::HashMap;

/// Anti-pattern detector for analyzing module splits
pub struct AntiPatternDetector {
    config: AntiPatternConfig,
}

/// Configuration for anti-pattern detection thresholds
#[derive(Clone, Debug)]
pub struct AntiPatternConfig {
    /// Minimum parameter count to flag as anti-pattern (default: 4)
    pub max_parameters: usize,

    /// Minimum distinct types to flag as mixed (default: 3)
    pub max_mixed_types: usize,

    /// Quality score penalty for critical anti-patterns (default: 20.0)
    pub critical_penalty: f64,

    /// Quality score penalty for high severity anti-patterns (default: 10.0)
    pub high_penalty: f64,

    /// Quality score penalty for medium severity anti-patterns (default: 5.0)
    pub medium_penalty: f64,

    /// Quality score penalty for low severity anti-patterns (default: 2.0)
    pub low_penalty: f64,
}

impl Default for AntiPatternConfig {
    fn default() -> Self {
        Self {
            max_parameters: 4,
            max_mixed_types: 3,
            critical_penalty: 20.0,
            high_penalty: 10.0,
            medium_penalty: 5.0,
            low_penalty: 2.0,
        }
    }
}

/// An identified anti-pattern
#[derive(Clone, Debug)]
pub struct AntiPattern {
    pub pattern_type: AntiPatternType,
    pub severity: AntiPatternSeverity,
    pub location: String,
    pub description: String,
    pub correction: String,
    pub affected_methods: Vec<String>,
}

/// Types of anti-patterns that can be detected
#[derive(Clone, Debug, PartialEq)]
pub enum AntiPatternType {
    UtilitiesModule,
    TechnicalGrouping,
    ParameterPassing,
    MixedDataTypes,
    LackOfTypeOwnership,
}

/// Severity levels for anti-patterns
#[derive(Clone, Debug, PartialEq, Ord, PartialOrd, Eq)]
pub enum AntiPatternSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Quality report for a set of module splits
#[derive(Clone, Debug)]
pub struct SplitQualityReport {
    pub quality_score: f64,
    pub anti_patterns: Vec<AntiPattern>,
    pub total_splits: usize,
    pub idiomatic_splits: usize,
}

impl AntiPatternDetector {
    /// Create a new detector with default configuration
    pub fn new() -> Self {
        Self {
            config: AntiPatternConfig::default(),
        }
    }

    /// Create a new detector with custom configuration
    pub fn with_config(config: AntiPatternConfig) -> Self {
        Self { config }
    }

    /// Detect utilities module anti-pattern
    ///
    /// Identifies catch-all modules named "utilities", "utils", "helpers", or "common"
    /// that typically indicate mixed responsibilities.
    pub fn detect_utilities_module(&self, split: &ModuleSplit) -> Option<AntiPattern> {
        // Extract the module name, stripping .rs extension if present
        let module_name = split
            .suggested_name
            .trim_end_matches(".rs")
            .rsplit('/')
            .next()
            .unwrap_or(&split.suggested_name);

        let is_utilities = module_name == "utilities"
            || module_name == "utils"
            || module_name == "helpers"
            || module_name == "common";

        if !is_utilities {
            return None;
        }

        Some(AntiPattern {
            pattern_type: AntiPatternType::UtilitiesModule,
            severity: AntiPatternSeverity::Critical,
            location: split.suggested_name.clone(),
            description: format!(
                "Utilities module '{}' is a catch-all with {} mixed responsibilities. \
                 This violates Single Responsibility Principle and creates unclear ownership.",
                split.suggested_name,
                split.methods_to_move.len()
            ),
            correction: self.suggest_utilities_correction(split),
            affected_methods: split.methods_to_move.clone(),
        })
    }

    fn suggest_utilities_correction(&self, _split: &ModuleSplit) -> String {
        "Split utilities into domain-specific modules:\n\
         1. Group methods by the primary type they operate on\n\
         2. Move formatting methods to the type they format (e.g., PriorityItem::display)\n\
         3. Extract parameter clumps into new types with methods\n\
         4. Consider trait implementations (Display, From, TryFrom) instead of utility functions"
            .to_string()
    }

    /// Detect technical/verb-based grouping anti-pattern
    ///
    /// Uses semantic analysis to identify:
    /// 1. Module names that are verbs (ends in -ing, -tion, -ment, etc.)
    /// 2. Methods that share the same verb prefix
    /// 3. Modules not recognized as domain terms
    pub fn detect_technical_grouping(&self, split: &ModuleSplit) -> Option<AntiPattern> {
        let module_name = split
            .suggested_name
            .rsplit('/')
            .next()
            .unwrap_or(&split.suggested_name)
            .trim_end_matches(".rs");

        // Check 1: Module name looks like a verb/action
        let is_verb_name = self.is_likely_verb(module_name);

        // Check 2: Methods share same verb prefix
        let method_verbs: Vec<_> = split
            .methods_to_move
            .iter()
            .filter_map(|m| self.extract_leading_verb(m))
            .collect();

        let shared_verb = if method_verbs.len() >= split.methods_to_move.len() / 2 {
            // More than half share a verb prefix
            let most_common = self.most_common_element(&method_verbs);
            Some(most_common)
        } else {
            None
        };

        // Check 3: Not a known domain term
        let is_domain_term = self.is_domain_term(module_name);

        if (is_verb_name || shared_verb.is_some()) && !is_domain_term {
            Some(AntiPattern {
                pattern_type: AntiPatternType::TechnicalGrouping,
                severity: AntiPatternSeverity::High,
                location: split.suggested_name.clone(),
                description: format!(
                    "Module '{}' is grouped by technical operation (verb) instead of data domain. \
                     This scatters type-related behavior across multiple modules.",
                    module_name
                ),
                correction: self.suggest_type_based_grouping(split),
                affected_methods: split.methods_to_move.clone(),
            })
        } else {
            None
        }
    }

    /// Check if word is likely a verb based on linguistic patterns
    fn is_likely_verb(&self, word: &str) -> bool {
        // Verbal noun suffixes
        word.ends_with("ing")
            || word.ends_with("tion")
            || word.ends_with("ment")
            || word.ends_with("sion")
            || word.ends_with("ance")
            || word.ends_with("ence")
            || // Known action words
        matches!(
            word,
            "calculate" | "compute" | "process" | "handle" | "manage" |
            "render" | "format" | "display" | "show" | "print" |
            "validate" | "check" | "verify" | "ensure" |
            "parse" | "transform" | "convert" | "serialize" | "deserialize" |
            "get" | "set" | "update" | "modify" | "create" | "delete" |
            "authenticate" | "authorize" | "encrypt" | "decrypt"
        )
    }

    /// Extract leading verb from method name (e.g., "format_header" → "format")
    fn extract_leading_verb(&self, method_name: &str) -> Option<String> {
        method_name.split('_').next().map(|s| s.to_string())
    }

    /// Find most common element in vector
    fn most_common_element(&self, items: &[String]) -> String {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for item in items {
            *counts.entry(item.as_str()).or_insert(0) += 1;
        }
        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(item, _)| item.to_string())
            .unwrap_or_default()
    }

    /// Check if word is a known domain term (not an action)
    fn is_domain_term(&self, word: &str) -> bool {
        // Common domain suffixes
        word.ends_with("metrics")
            || word.ends_with("data")
            || word.ends_with("config")
            || word.ends_with("settings")
            || word.ends_with("context")
            || word.ends_with("item")
            || word.ends_with("result")
            || word.ends_with("info")
            || word.ends_with("details")
            || // Plural nouns (likely domain objects)
        (word.ends_with('s') && !word.ends_with("ss"))
            || // Single words that are nouns
        matches!(
            word,
            "priority" | "god_object" | "debt" | "complexity" |
            "coverage" | "analysis" | "report" | "summary"
        )
    }

    fn suggest_type_based_grouping(&self, _split: &ModuleSplit) -> String {
        "Reorganize by data types:\n\
         1. Identify the primary types these methods operate on\n\
         2. Create modules named after those types (e.g., priority_item.rs, god_object_section.rs)\n\
         3. Move all methods operating on a type to its module\n\
         4. Use impl blocks to associate methods with their types\n\
         \n\
         Example:\n\
         Instead of: calculate/calculate_score.rs\n\
         Use: god_object_metrics.rs with impl GodObjectMetrics { fn score() }"
            .to_string()
    }

    /// Detect parameter passing anti-pattern
    ///
    /// Identifies functions with excessive parameters that should be encapsulated
    /// in a struct. Flags methods with 4+ parameters as candidates for parameter object pattern.
    pub fn detect_parameter_passing(
        &self,
        signatures: &[MethodSignature],
        split: &ModuleSplit,
    ) -> Vec<AntiPattern> {
        let mut anti_patterns = Vec::new();

        // Filter signatures to those in this split
        let split_methods: Vec<_> = signatures
            .iter()
            .filter(|sig| split.methods_to_move.contains(&sig.name))
            .collect();

        for signature in split_methods {
            if signature.param_types.len() >= self.config.max_parameters {
                anti_patterns.push(AntiPattern {
                    pattern_type: AntiPatternType::ParameterPassing,
                    severity: AntiPatternSeverity::Medium,
                    location: format!("{}::{}", split.suggested_name, signature.name),
                    description: format!(
                        "Method '{}' has {} parameters. Functions with 4+ parameters are hard to call and maintain.",
                        signature.name,
                        signature.param_types.len()
                    ),
                    correction: format!(
                        "Encapsulate related parameters into a struct:\n\
                         1. Identify parameter clumps (params that are always passed together)\n\
                         2. Create a new struct to hold these parameters\n\
                         3. Update method signature to take the struct\n\
                         \n\
                         Example:\n\
                         Instead of: fn {}({}) {{\n\
                         Use: struct {}Params {{ ... }}\n\
                         fn {}(params: {}Params) {{",
                        signature.name,
                        signature.param_types.join(", "),
                        to_pascal_case(&signature.name),
                        signature.name,
                        to_pascal_case(&signature.name)
                    ),
                    affected_methods: vec![signature.name.clone()],
                });
            }
        }

        anti_patterns
    }

    /// Detect mixed data types anti-pattern
    ///
    /// Identifies modules operating on multiple unrelated types (3+ distinct non-primitive types).
    pub fn detect_mixed_data_types(
        &self,
        signatures: &[MethodSignature],
        split: &ModuleSplit,
    ) -> Option<AntiPattern> {
        // Filter signatures to those in this split
        let split_methods: Vec<_> = signatures
            .iter()
            .filter(|sig| split.methods_to_move.contains(&sig.name))
            .collect();

        // Collect all distinct non-primitive types from parameters and return types
        let mut types = std::collections::HashSet::new();

        for signature in &split_methods {
            // Collect parameter types
            for param_type in &signature.param_types {
                if !is_primitive(param_type) {
                    types.insert(param_type.clone());
                }
            }

            // Collect return type
            if let Some(return_type) = &signature.return_type {
                if !is_primitive(return_type) {
                    types.insert(return_type.clone());
                }
            }
        }

        // Flag if we have 3+ distinct non-primitive types
        if types.len() >= self.config.max_mixed_types {
            let type_list: Vec<_> = types.iter().cloned().collect();
            Some(AntiPattern {
                pattern_type: AntiPatternType::MixedDataTypes,
                severity: AntiPatternSeverity::High,
                location: split.suggested_name.clone(),
                description: format!(
                    "Module '{}' operates on {} distinct non-primitive types: {}. \
                     This indicates mixed responsibilities and unclear domain boundaries.",
                    split.suggested_name,
                    types.len(),
                    type_list.join(", ")
                ),
                correction: format!(
                    "Split module by primary data type:\n\
                     1. Group methods by the main type they operate on\n\
                     2. Create separate modules for each type (e.g., {}.rs, {}.rs)\n\
                     3. Move cross-cutting concerns to trait implementations\n\
                     \n\
                     Detected types: {}",
                    type_list.get(0).unwrap_or(&"type1".to_string()),
                    type_list.get(1).unwrap_or(&"type2".to_string()),
                    type_list.join(", ")
                ),
                affected_methods: split.methods_to_move.clone(),
            })
        } else {
            None
        }
    }

    /// Analyze module split for all anti-patterns
    pub fn analyze_split(
        &self,
        split: &ModuleSplit,
        signatures: &[MethodSignature],
    ) -> Vec<AntiPattern> {
        let mut anti_patterns = Vec::new();

        // Check for utilities module
        if let Some(pattern) = self.detect_utilities_module(split) {
            anti_patterns.push(pattern);
        }

        // Check for technical grouping
        if let Some(pattern) = self.detect_technical_grouping(split) {
            anti_patterns.push(pattern);
        }

        // Check for parameter passing
        anti_patterns.extend(self.detect_parameter_passing(signatures, split));

        // Check for mixed data types
        if let Some(pattern) = self.detect_mixed_data_types(signatures, split) {
            anti_patterns.push(pattern);
        }

        anti_patterns.sort_by(|a, b| b.severity.cmp(&a.severity));
        anti_patterns
    }

    /// Analyze all splits and return quality score
    ///
    /// Quality Score Formula:
    /// Starting from 100 (perfect), subtract penalties for each anti-pattern:
    /// - Critical: -20 points (utilities modules, major violations)
    /// - High: -10 points (technical groupings, mixed types)
    /// - Medium: -5 points (parameter passing, minor issues)
    /// - Low: -2 points (style issues, suggestions)
    ///
    /// Score interpretation:
    /// - 90-100: Excellent (idiomatic Rust/FP)
    /// - 70-89: Good (minor improvements needed)
    /// - 50-69: Needs Improvement (several anti-patterns)
    /// - 0-49: Poor (major refactoring needed)
    pub fn calculate_split_quality(
        &self,
        splits: &[ModuleSplit],
        signatures: &[MethodSignature],
    ) -> SplitQualityReport {
        let mut all_anti_patterns = Vec::new();

        for split in splits {
            let patterns = self.analyze_split(split, signatures);
            all_anti_patterns.extend(patterns);
        }

        // Count anti-patterns by severity
        let critical_count = all_anti_patterns
            .iter()
            .filter(|p| p.severity == AntiPatternSeverity::Critical)
            .count();
        let high_count = all_anti_patterns
            .iter()
            .filter(|p| p.severity == AntiPatternSeverity::High)
            .count();
        let medium_count = all_anti_patterns
            .iter()
            .filter(|p| p.severity == AntiPatternSeverity::Medium)
            .count();
        let low_count = all_anti_patterns
            .iter()
            .filter(|p| p.severity == AntiPatternSeverity::Low)
            .count();

        let quality_score = 100.0
            - (critical_count as f64 * self.config.critical_penalty)
            - (high_count as f64 * self.config.high_penalty)
            - (medium_count as f64 * self.config.medium_penalty)
            - (low_count as f64 * self.config.low_penalty);

        SplitQualityReport {
            quality_score: quality_score.max(0.0),
            anti_patterns: all_anti_patterns,
            total_splits: splits.len(),
            idiomatic_splits: splits.len() - critical_count - high_count,
        }
    }
}

impl Default for AntiPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Determine if a type is primitive/stdlib
pub fn is_primitive(type_name: &str) -> bool {
    matches!(
        type_name,
        "String"
            | "str"
            | "usize"
            | "isize"
            | "u32"
            | "i32"
            | "u64"
            | "i64"
            | "u8"
            | "i8"
            | "u16"
            | "i16"
            | "u128"
            | "i128"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "()"
            | "Vec"
            | "Option"
            | "Result"
            | "Box"
            | "Rc"
            | "Arc"
            | "HashMap"
            | "HashSet"
            | "BTreeMap"
            | "BTreeSet"
            | "VecDeque"
            | "LinkedList"
            | "BinaryHeap"
            | "Path"
            | "PathBuf"
            | "OsString"
            | "OsStr"
            | "File"
            | "BufReader"
            | "BufWriter"
            | "Cow"
            | "RefCell"
            | "Cell"
            | "Mutex"
            | "RwLock"
            | "Error"
    ) || type_name.starts_with('&')
        || type_name.starts_with("&mut")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utilities_module_detection() {
        let split = ModuleSplit {
            suggested_name: "god_object/utilities.rs".to_string(),
            methods_to_move: vec!["foo".to_string(), "bar".to_string()],
            responsibility: "utilities".to_string(),
            estimated_lines: 100,
            method_count: 2,
            ..Default::default()
        };

        let detector = AntiPatternDetector::new();
        let pattern = detector.detect_utilities_module(&split);

        assert!(pattern.is_some());
        let pattern = pattern.unwrap();
        assert_eq!(pattern.pattern_type, AntiPatternType::UtilitiesModule);
        assert_eq!(pattern.severity, AntiPatternSeverity::Critical);
    }

    #[test]
    fn test_technical_grouping_detection() {
        let split = ModuleSplit {
            suggested_name: "god_object/calculate.rs".to_string(),
            methods_to_move: vec!["calculate_score".to_string()],
            responsibility: "calculation".to_string(),
            estimated_lines: 50,
            method_count: 1,
            ..Default::default()
        };

        let detector = AntiPatternDetector::new();
        let pattern = detector.detect_technical_grouping(&split);

        assert!(pattern.is_some());
        let pattern = pattern.unwrap();
        assert_eq!(pattern.pattern_type, AntiPatternType::TechnicalGrouping);
        assert_eq!(pattern.severity, AntiPatternSeverity::High);
    }

    #[test]
    fn test_quality_score_calculation() {
        let splits = vec![
            ModuleSplit {
                suggested_name: "good_module.rs".to_string(),
                methods_to_move: vec!["foo".to_string()],
                responsibility: "domain".to_string(),
                estimated_lines: 50,
                method_count: 1,
                ..Default::default()
            },
            ModuleSplit {
                suggested_name: "utilities.rs".to_string(),
                methods_to_move: vec!["bar".to_string()],
                responsibility: "utilities".to_string(),
                estimated_lines: 50,
                method_count: 1,
                ..Default::default()
            },
        ];

        let detector = AntiPatternDetector::new();
        let report = detector.calculate_split_quality(&splits, &[]);

        assert!(report.quality_score < 100.0);
        assert!(!report.anti_patterns.is_empty());
    }

    #[test]
    fn test_is_likely_verb() {
        let detector = AntiPatternDetector::new();

        assert!(detector.is_likely_verb("rendering"));
        assert!(detector.is_likely_verb("calculation"));
        assert!(detector.is_likely_verb("management"));
        assert!(detector.is_likely_verb("format"));
        assert!(detector.is_likely_verb("calculate"));

        assert!(!detector.is_likely_verb("metrics"));
        assert!(!detector.is_likely_verb("config"));
        assert!(!detector.is_likely_verb("data"));
    }

    #[test]
    fn test_is_domain_term() {
        let detector = AntiPatternDetector::new();

        assert!(detector.is_domain_term("metrics"));
        assert!(detector.is_domain_term("config"));
        assert!(detector.is_domain_term("priority"));
        assert!(detector.is_domain_term("results"));

        assert!(!detector.is_domain_term("rendering"));
        assert!(!detector.is_domain_term("calculate"));
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(to_pascal_case("format_header"), "FormatHeader");
        assert_eq!(to_pascal_case("simple"), "Simple");
    }

    #[test]
    fn test_is_primitive() {
        assert!(is_primitive("String"));
        assert!(is_primitive("usize"));
        assert!(is_primitive("Vec"));
        assert!(is_primitive("Option"));
        assert!(is_primitive("&str"));

        assert!(!is_primitive("CustomType"));
        assert!(!is_primitive("MyStruct"));
    }
}

// Display implementations for formatted output (Spec 183)
use std::fmt;

impl fmt::Display for SplitQualityReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "╔══════════════════════════════════════════════════════════════╗")?;
        writeln!(f, "║              Split Quality Analysis                          ║")?;
        writeln!(f, "╠══════════════════════════════════════════════════════════════╣")?;
        writeln!(f, "║ Quality Score: {:<46} ║", format!("{:.1}/100.0", self.quality_score))?;
        writeln!(f, "║ Total Splits: {:<47} ║", self.total_splits)?;
        writeln!(f, "║ Idiomatic Splits: {:<43} ║", self.idiomatic_splits)?;
        writeln!(f, "╚══════════════════════════════════════════════════════════════╝")?;

        if !self.anti_patterns.is_empty() {
            writeln!(f)?;
            writeln!(f, "╔══════════════════════════════════════════════════════════════╗")?;
            writeln!(f, "║              Anti-Patterns Found ({:<2})                        ║", self.anti_patterns.len())?;
            writeln!(f, "╚══════════════════════════════════════════════════════════════╝")?;

            for (i, pattern) in self.anti_patterns.iter().enumerate() {
                if i > 0 {
                    writeln!(f)?;
                }
                write!(f, "{}", pattern)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for AntiPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity_str = match self.severity {
            AntiPatternSeverity::Critical => "🔴 CRITICAL",
            AntiPatternSeverity::High => "🟠 HIGH",
            AntiPatternSeverity::Medium => "🟡 MEDIUM",
            AntiPatternSeverity::Low => "🟢 LOW",
        };

        let pattern_name = match self.pattern_type {
            AntiPatternType::UtilitiesModule => "Utilities Module",
            AntiPatternType::TechnicalGrouping => "Technical Grouping",
            AntiPatternType::ParameterPassing => "Parameter Passing",
            AntiPatternType::MixedDataTypes => "Mixed Data Types",
            AntiPatternType::LackOfTypeOwnership => "Lack of Type Ownership",
        };

        writeln!(f, "┌──────────────────────────────────────────────────────────────┐")?;
        writeln!(f, "│ {} - {:<40} │", severity_str, pattern_name)?;
        writeln!(f, "├──────────────────────────────────────────────────────────────┤")?;
        writeln!(f, "│ Location: {:<51} │", self.location)?;
        writeln!(f, "├──────────────────────────────────────────────────────────────┤")?;

        // Description - wrap text to fit width
        writeln!(f, "│ Description:                                                 │")?;
        for line in wrap_text(&self.description, 58) {
            writeln!(f, "│   {:<58} │", line)?;
        }

        writeln!(f, "├──────────────────────────────────────────────────────────────┤")?;
        writeln!(f, "│ Correction:                                                  │")?;
        for line in wrap_text(&self.correction, 58) {
            writeln!(f, "│   {:<58} │", line)?;
        }

        if !self.affected_methods.is_empty() {
            writeln!(f, "├──────────────────────────────────────────────────────────────┤")?;
            writeln!(f, "│ Affected Methods ({:<2}):                                     │", self.affected_methods.len())?;
            for method in self.affected_methods.iter().take(5) {
                writeln!(f, "│   • {:<56} │", method)?;
            }
            if self.affected_methods.len() > 5 {
                writeln!(f, "│   ... and {} more                                        │", self.affected_methods.len() - 5)?;
            }
        }

        writeln!(f, "└──────────────────────────────────────────────────────────────┘")?;

        Ok(())
    }
}

/// Wrap text to fit within a specified width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + word.len() + 1 <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}
