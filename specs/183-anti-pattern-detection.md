# Spec 183: Anti-Pattern Detection for Code Quality

**Status**: Draft
**Dependencies**: [181, 182]
**Related**: [178, 179, 180]

## Problem

Current god object recommendations can produce anti-patterns that violate idiomatic Rust and functional programming principles:

1. **Utilities Modules**: Every behavioral split creates catch-all utilities.rs with mixed responsibilities
2. **Technical Groupings**: Modules named by verbs (rendering.rs, calculate.rs) instead of data domains
3. **Parameter Passing**: Functions pass 4-5 parameters instead of encapsulating in types
4. **Mixed Data Types**: Single module operates on multiple unrelated types

**Example anti-patterns from current output**:
```
formatter/utilities.rs (9 methods)
  - format_source_location
  - format_method_list
  - format_severity
  - truncate_name
  [Mixed responsibilities, no clear data domain]

god_object_analysis/calculate.rs (8 methods)
  [Verb-based grouping instead of type ownership]
```

## Objective

Implement anti-pattern detection that identifies violations of idiomatic Rust/FP principles and provides corrective recommendations to transform anti-patterns into proper domain-driven module organization.

## Requirements

### 1. Utilities Module Detection

Identify catch-all modules with mixed responsibilities:

```rust
// src/organization/anti_pattern_detector.rs

use std::collections::{HashMap, HashSet};
use crate::organization::god_object_analysis::ModuleSplit;
use crate::organization::type_based_clustering::TypeInfo;

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

#[derive(Clone, Debug)]
pub struct AntiPattern {
    pub pattern_type: AntiPatternType,
    pub severity: AntiPatternSeverity,
    pub location: String,
    pub description: String,
    pub correction: String,
    pub affected_methods: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AntiPatternType {
    UtilitiesModule,
    TechnicalGrouping,
    ParameterPassing,
    MixedDataTypes,
    LackOfTypeOwnership,
}

#[derive(Clone, Debug, PartialEq, Ord, PartialOrd, Eq)]
pub enum AntiPatternSeverity {
    Critical,
    High,
    Medium,
    Low,
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
    pub fn detect_utilities_module(
        &self,
        split: &ModuleSplit,
    ) -> Option<AntiPattern> {
        let is_utilities = split.suggested_name.ends_with("utilities")
            || split.suggested_name.ends_with("utils")
            || split.suggested_name.ends_with("helpers")
            || split.suggested_name.ends_with("common");

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

    fn suggest_utilities_correction(&self, split: &ModuleSplit) -> String {
        format!(
            "Split utilities into domain-specific modules:\n\
             1. Group methods by the primary type they operate on\n\
             2. Move formatting methods to the type they format (e.g., PriorityItem::display)\n\
             3. Extract parameter clumps into new types with methods\n\
             4. Consider trait implementations (Display, From, TryFrom) instead of utility functions"
        )
    }
}
```

### 2. Technical Grouping Detection

Identify verb-based module names instead of type/domain-based:

```rust
impl AntiPatternDetector {
    /// Detect technical/verb-based grouping anti-pattern
    ///
    /// Uses semantic analysis instead of hardcoded verb list:
    /// 1. Check if module name is a verb (ends in -ing, -tion, -ment, etc.)
    /// 2. Check if methods in module share same verb prefix
    /// 3. Compare against known domain terms
    pub fn detect_technical_grouping(
        &self,
        split: &ModuleSplit,
    ) -> Option<AntiPattern> {
        let module_name = split.suggested_name
            .split('/')
            .last()
            .unwrap_or(&split.suggested_name)
            .trim_end_matches(".rs");

        // Check 1: Module name looks like a verb/action
        let is_verb_name = self.is_likely_verb(module_name);

        // Check 2: Methods share same verb prefix
        let method_verbs: Vec<_> = split.methods_to_move.iter()
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
        word.ends_with("ing") ||     // rendering, parsing
        word.ends_with("tion") ||    // calculation, validation
        word.ends_with("ment") ||    // management, placement
        word.ends_with("sion") ||    // conversion, extension
        word.ends_with("ance") ||    // performance, maintenance
        word.ends_with("ence") ||    // reference, persistence
        // Known action words
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
        counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(item, _)| item.to_string())
            .unwrap_or_default()
    }

    /// Check if word is a known domain term (not an action)
    fn is_domain_term(&self, word: &str) -> bool {
        // Common domain suffixes
        word.ends_with("metrics") ||
        word.ends_with("data") ||
        word.ends_with("config") ||
        word.ends_with("settings") ||
        word.ends_with("context") ||
        word.ends_with("item") ||
        word.ends_with("result") ||
        word.ends_with("info") ||
        word.ends_with("details") ||
        // Plural nouns (likely domain objects)
        (word.ends_with('s') && !word.ends_with("ss")) ||
        // Single words that are nouns
        matches!(
            word,
            "priority" | "god_object" | "debt" | "complexity" |
            "coverage" | "analysis" | "report" | "summary"
        )
    }

    fn suggest_type_based_grouping(&self, split: &ModuleSplit) -> String {
        format!(
            "Reorganize by data types:\n\
             1. Identify the primary types these methods operate on\n\
             2. Create modules named after those types (e.g., priority_item.rs, god_object_section.rs)\n\
             3. Move all methods operating on a type to its module\n\
             4. Use impl blocks to associate methods with their types\n\
             \n\
             Example:\n\
             Instead of: calculate/calculate_score.rs\n\
             Use: god_object_metrics.rs with impl GodObjectMetrics {{ fn score() }}"
        )
    }
}
```

### 3. Parameter Passing Detection

Identify functions passing many parameters instead of using types:

```rust
use crate::organization::type_based_clustering::MethodSignature;

impl AntiPatternDetector {
    /// Detect parameter passing anti-pattern
    pub fn detect_parameter_passing(
        &self,
        signatures: &[MethodSignature],
        split: &ModuleSplit,
    ) -> Vec<AntiPattern> {
        let mut anti_patterns = Vec::new();

        for sig in signatures {
            if !split.methods_to_move.contains(&sig.name) {
                continue;
            }

            // Check against configured threshold
            if sig.param_types.len() >= self.config.max_parameters {
                anti_patterns.push(AntiPattern {
                    pattern_type: AntiPatternType::ParameterPassing,
                    severity: AntiPatternSeverity::Medium,
                    location: format!("{}::{}", split.suggested_name, sig.name),
                    description: format!(
                        "Function '{}' takes {} parameters. Consider encapsulating in a struct.",
                        sig.name,
                        sig.param_types.len()
                    ),
                    correction: self.suggest_parameter_encapsulation(sig),
                    affected_methods: vec![sig.name.clone()],
                });
            }
        }

        anti_patterns
    }

    fn suggest_parameter_encapsulation(&self, sig: &MethodSignature) -> String {
        let param_names: Vec<_> = sig.param_types.iter()
            .map(|t| t.name.as_str())
            .collect();

        format!(
            "Create a struct to encapsulate parameters:\n\
             \n\
             pub struct {}Params {{\n\
             {}\n\
             }}\n\
             \n\
             impl {} {{\n\
             \    pub fn {}(params: {}Params) -> ... {{\n\
             \        // Use params.field instead of individual parameters\n\
             \    }}\n\
             }}",
            to_pascal_case(&sig.name),
            param_names.iter()
                .enumerate()
                .map(|(i, name)| format!("    pub param{}: {},", i, name))
                .collect::<Vec<_>>()
                .join("\n"),
            to_pascal_case(&sig.name),
            sig.name,
            to_pascal_case(&sig.name),
        )
    }
}

fn to_pascal_case(s: &str) -> String {
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
```

### 4. Mixed Data Types Detection

Identify modules operating on multiple unrelated types:

```rust
impl AntiPatternDetector {
    /// Detect mixed data types anti-pattern
    pub fn detect_mixed_data_types(
        &self,
        signatures: &[MethodSignature],
        split: &ModuleSplit,
    ) -> Option<AntiPattern> {
        // Collect all types used by methods in this split
        let mut type_counts: HashMap<String, usize> = HashMap::new();

        for sig in signatures {
            if !split.methods_to_move.contains(&sig.name) {
                continue;
            }

            for param in &sig.param_types {
                *type_counts.entry(param.name.clone()).or_insert(0) += 1;
            }

            if let Some(ret) = &sig.return_type {
                *type_counts.entry(ret.name.clone()).or_insert(0) += 1;
            }
        }

        // Remove common types (String, usize, bool, etc.)
        type_counts.retain(|k, _| !is_primitive(k));

        // Check against configured threshold
        if type_counts.len() >= self.config.max_mixed_types {
            let types: Vec<_> = type_counts.keys().cloned().collect();

            return Some(AntiPattern {
                pattern_type: AntiPatternType::MixedDataTypes,
                severity: AntiPatternSeverity::High,
                location: split.suggested_name.clone(),
                description: format!(
                    "Module '{}' operates on {} distinct types: {}. \
                     Each type should have its own module.",
                    split.suggested_name,
                    types.len(),
                    types.join(", ")
                ),
                correction: format!(
                    "Split into type-specific modules:\n{}",
                    types.iter()
                        .map(|t| format!("- {}.rs (methods operating on {})", t.to_lowercase(), t))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
                affected_methods: split.methods_to_move.clone(),
            });
        }

        None
    }
}

/// Determine if a type is primitive/stdlib (context-aware)
///
/// Path/PathBuf are considered domain types in file-handling contexts,
/// but primitives in general analysis.
fn is_primitive_contextaware(type_name: &str, context: &ModuleSplit) -> bool {
    // Core primitives (always)
    if matches!(
        type_name,
        "String" | "str" | "usize" | "isize" | "u32" | "i32" | "u64" | "i64" |
        "u8" | "i8" | "u16" | "i16" | "u128" | "i128" |
        "f32" | "f64" | "bool" | "char" | "()" |
        "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc" |
        "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" |
        "VecDeque" | "LinkedList" | "BinaryHeap" |
        "File" | "BufReader" | "BufWriter" |
        "Cow" | "RefCell" | "Cell" | "Mutex" | "RwLock"
    ) || type_name.starts_with('&') {
        return true;
    }

    // Context-aware decisions
    match type_name {
        "Path" | "PathBuf" | "OsString" | "OsStr" => {
            // If module deals with file paths, these are domain types
            let module_name = &context.suggested_name.to_lowercase();
            let is_path_domain = module_name.contains("path") ||
                                 module_name.contains("file") ||
                                 module_name.contains("location") ||
                                 module_name.contains("source");
            !is_path_domain  // Primitive if NOT in path domain
        }
        "Error" => {
            // Error is a primitive type in error-handling contexts
            let is_error_domain = context.suggested_name.to_lowercase().contains("error");
            !is_error_domain
        }
        _ => false
    }
}

/// Simplified version for non-context-aware usage
fn is_primitive(type_name: &str) -> bool {
    matches!(
        type_name,
        "String" | "str" | "usize" | "isize" | "u32" | "i32" | "u64" | "i64" |
        "u8" | "i8" | "u16" | "i16" | "u128" | "i128" |
        "f32" | "f64" | "bool" | "char" | "()" |
        "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc" |
        "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" |
        "VecDeque" | "LinkedList" | "BinaryHeap" |
        "Path" | "PathBuf" | "OsString" | "OsStr" |
        "File" | "BufReader" | "BufWriter" |
        "Cow" | "RefCell" | "Cell" | "Mutex" | "RwLock" |
        "Error"
    ) || type_name.starts_with("&")  // References
      || type_name.starts_with("&mut")  // Mutable references
}
```

### 5. Comprehensive Analysis

Combine all detectors:

```rust
impl AntiPatternDetector {
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
        let critical_count = all_anti_patterns.iter()
            .filter(|p| p.severity == AntiPatternSeverity::Critical)
            .count();
        let high_count = all_anti_patterns.iter()
            .filter(|p| p.severity == AntiPatternSeverity::High)
            .count();
        let medium_count = all_anti_patterns.iter()
            .filter(|p| p.severity == AntiPatternSeverity::Medium)
            .count();
        let low_count = all_anti_patterns.iter()
            .filter(|p| p.severity == AntiPatternSeverity::Low)
            .count();

        /// Quality Score Formula:
        ///
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

#[derive(Clone, Debug)]
pub struct SplitQualityReport {
    pub quality_score: f64,
    pub anti_patterns: Vec<AntiPattern>,
    pub total_splits: usize,
    pub idiomatic_splits: usize,
}
```

### 6. Shared Utilities

Common helper functions used across anti-pattern detection and specs 181-184:

```rust
/// Convert string to PascalCase (shared with Spec 184)
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
```

Note: The `is_primitive` function is defined inline in the "Mixed Data Types Detection" section above.

### 7. Integration with God Object Detector

```rust
// src/organization/god_object_detector.rs

fn validate_and_improve_splits(
    splits: Vec<ModuleSplit>,
    signatures: &[MethodSignature],
) -> (Vec<ModuleSplit>, SplitQualityReport) {
    use crate::organization::anti_pattern_detector::AntiPatternDetector;

    let detector = AntiPatternDetector;
    let quality_report = detector.calculate_split_quality(&splits, signatures);

    // Filter out splits with critical anti-patterns
    let improved_splits: Vec<_> = splits.into_iter()
        .filter(|split| {
            let patterns = detector.analyze_split(split, signatures);
            !patterns.iter().any(|p| p.severity == AntiPatternSeverity::Critical)
        })
        .collect();

    (improved_splits, quality_report)
}
```

## Enhanced Output Format

```
#4 SCORE: 62.0 [CRITICAL] god_object_analysis.rs (27 methods, 15 structs)

Split Quality Analysis:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Quality Score: 45/100 (Needs Improvement)
  Total Splits: 5
  Idiomatic Splits: 2 ✓
  Anti-Patterns Detected: 3 ✗

Anti-Patterns Found:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[CRITICAL] Utilities Module
  Location: god_object_analysis/utilities.rs
  Problem: Utilities module is a catch-all with 12 mixed responsibilities.
           This violates Single Responsibility Principle and creates unclear ownership.

  Affected Methods:
    - format_header, format_method_list, format_severity
    - calculate_helper, validate_input, transform_data
    ...

  Correction:
    Split utilities into domain-specific modules:
    1. Group methods by the primary type they operate on
    2. Move formatting methods to the type they format (e.g., PriorityItem::display)
    3. Extract parameter clumps into new types with methods
    4. Consider trait implementations (Display, From, TryFrom) instead of utility functions

[HIGH] Technical Grouping
  Location: god_object_analysis/calculate.rs
  Problem: Module 'calculate.rs' is grouped by technical operation (verb) instead of data domain.
           This scatters type-related behavior across multiple modules.

  Affected Methods:
    - calculate_god_object_score, calculate_domain_diversity
    - calculate_struct_ratio

  Correction:
    Reorganize by data types:
    1. Identify the primary types these methods operate on
    2. Create modules named after those types (e.g., god_object_metrics.rs)
    3. Move all methods operating on a type to its module
    4. Use impl blocks to associate methods with their types

    Example:
    Instead of: calculate/calculate_score.rs
    Use: god_object_metrics.rs with impl GodObjectMetrics { fn score() }

[HIGH] Mixed Data Types
  Location: god_object_analysis/recommend.rs
  Problem: Module 'recommend.rs' operates on 4 distinct types: ModuleSplit,
           DomainDiversityMetrics, GodObjectConfidence, StructWithMethods.
           Each type should have its own module.

  Correction:
    Split into type-specific modules:
    - modulesplit.rs (methods operating on ModuleSplit)
    - domaindiversitymetrics.rs (methods operating on DomainDiversityMetrics)
    - godobjectconfidence.rs (methods operating on GodObjectConfidence)
    - structwithmethods.rs (methods operating on StructWithMethods)

Recommended Action:
  Use type-based clustering (Spec 181) or data flow analysis (Spec 182)
  to generate idiomatic splits without these anti-patterns.
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utilities_module_detection() {
        let split = ModuleSplit {
            suggested_name: "god_object/utilities.rs".to_string(),
            methods_to_move: vec!["foo".to_string(), "bar".to_string()],
            ..Default::default()
        };

        let detector = AntiPatternDetector;
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
            ..Default::default()
        };

        let detector = AntiPatternDetector;
        let pattern = detector.detect_technical_grouping(&split);

        assert!(pattern.is_some());
        let pattern = pattern.unwrap();
        assert_eq!(pattern.pattern_type, AntiPatternType::TechnicalGrouping);
        assert_eq!(pattern.severity, AntiPatternSeverity::High);
    }

    #[test]
    fn test_parameter_passing_detection() {
        let signature = MethodSignature {
            name: "format_header".to_string(),
            param_types: vec![
                TypeInfo { name: "Score".to_string(), .. },
                TypeInfo { name: "Location".to_string(), .. },
                TypeInfo { name: "Metrics".to_string(), .. },
                TypeInfo { name: "Verbosity".to_string(), .. },
            ],
            return_type: Some(TypeInfo { name: "String".to_string(), .. }),
            self_type: None,
        };

        let split = ModuleSplit {
            methods_to_move: vec!["format_header".to_string()],
            ..Default::default()
        };

        let detector = AntiPatternDetector;
        let patterns = detector.detect_parameter_passing(&[signature], &split);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, AntiPatternType::ParameterPassing);
    }

    #[test]
    fn test_quality_score_calculation() {
        let splits = vec![
            ModuleSplit {
                suggested_name: "good_module.rs".to_string(),
                methods_to_move: vec!["foo".to_string()],
                ..Default::default()
            },
            ModuleSplit {
                suggested_name: "utilities.rs".to_string(),
                methods_to_move: vec!["bar".to_string()],
                ..Default::default()
            },
        ];

        let detector = AntiPatternDetector;
        let report = detector.calculate_split_quality(&splits, &[]);

        assert!(report.quality_score < 100.0);
        assert!(report.anti_patterns.len() > 0);
    }
}
```

### Integration Tests

```rust
// tests/anti_pattern_detection_integration.rs

#[test]
fn test_detect_behavioral_split_anti_patterns() {
    // Current behavioral split output
    let splits = vec![
        ModuleSplit {
            suggested_name: "formatter/utilities.rs".to_string(),
            methods_to_move: vec![
                "format_source_location".to_string(),
                "format_method_list".to_string(),
                "truncate_name".to_string(),
            ],
            ..Default::default()
        },
        ModuleSplit {
            suggested_name: "formatter/rendering.rs".to_string(),
            methods_to_move: vec!["render_section".to_string()],
            ..Default::default()
        },
    ];

    let detector = AntiPatternDetector;
    let report = detector.calculate_split_quality(&splits, &[]);

    // Should detect utilities module
    assert!(report.anti_patterns.iter().any(|p| {
        p.pattern_type == AntiPatternType::UtilitiesModule
    }));

    // Should detect technical grouping
    assert!(report.anti_patterns.iter().any(|p| {
        p.pattern_type == AntiPatternType::TechnicalGrouping
    }));

    // Quality score should be low
    assert!(report.quality_score < 60.0);
}
```

## Dependencies

- **Spec 181**: Type signature extraction for parameter analysis
- **Spec 182**: Data flow analysis for detecting mixed types
- Existing `ModuleSplit` structure for analysis

## Migration Path

1. **Phase 1**: Implement individual anti-pattern detectors
2. **Phase 2**: Add comprehensive analysis function
3. **Phase 3**: Integrate with god object detector output
4. **Phase 4**: Add quality score calculation
5. **Phase 5**: Enhance output format with anti-pattern warnings

## Success Criteria

- Detects utilities modules with 100% accuracy
- Identifies technical groupings (verb-based modules)
- Flags functions with 4+ parameters
- Detects mixed data types (3+ types per module)
- Provides actionable corrections for each anti-pattern
- Quality score accurately reflects idiomatic Rust adherence
