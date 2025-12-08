//! # Struct Pattern Detection (Pure Core)
//!
//! Pure functions for detecting common Rust patterns that should not be
//! flagged as god objects. Implements pattern recognition to reduce false
//! positives.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - all functions are deterministic
//! with no side effects. Pattern detection is a pure transformation of metrics.
//!
//! ## Recognized Patterns
//!
//! - **Config Pattern**: Builder/factory methods for configuration presets
//! - **DTO Pattern**: Data Transfer Objects with minimal behavior
//! - **Aggregate Root**: Domain entities with many fields but cohesive responsibility
//!
//! ## Parallel to builder_pattern.rs
//!
//! This module follows the same architectural pattern as `builder_pattern.rs`,
//! providing organization-level pattern detection that can be used by multiple
//! analyzers (currently used by god_object detector).

use crate::organization::god_object::ast_visitor::TypeAnalysis;

/// Pattern classification for struct types.
///
/// Used to distinguish acceptable patterns from genuine god objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructPattern {
    /// Configuration struct with factory methods
    Config,
    /// Data Transfer Object - data container with minimal behavior
    DataTransferObject,
    /// Aggregate Root - domain entity with many fields but single responsibility
    AggregateRoot,
    /// No recognizable pattern - standard struct
    Standard,
}

/// Comprehensive pattern analysis result.
#[derive(Debug, Clone)]
pub struct PatternAnalysis {
    /// Detected pattern type
    pub pattern: StructPattern,
    /// Confidence in detection (0.0 - 1.0)
    pub confidence: f64,
    /// Evidence supporting this classification
    pub evidence: Vec<String>,
    /// Whether this pattern should skip god object detection
    pub skip_god_object_check: bool,
}

/// Detect struct pattern from metrics (pure function).
///
/// Analyzes method names, field count, and method-to-field ratio to identify
/// common Rust patterns that are not god objects despite high metric counts.
///
/// # Arguments
///
/// * `type_info` - Collected type information from AST
/// * `responsibilities` - Number of detected responsibilities
///
/// # Returns
///
/// Pattern analysis with classification and confidence score
///
/// # Examples
///
/// ```
/// use debtmap::organization::struct_patterns::{detect_pattern, StructPattern};
/// use debtmap::organization::god_object::ast_visitor::TypeAnalysis;
/// use debtmap::common::{SourceLocation, LocationConfidence};
///
/// let type_analysis = TypeAnalysis {
///     name: "AppConfig".to_string(),
///     method_count: 5,
///     field_count: 8,
///     methods: vec!["strict".into(), "balanced".into(), "lenient".into()],
///     fields: vec![],
///     responsibilities: vec![],
///     trait_implementations: 0,
///     location: SourceLocation {
///         line: 1,
///         column: Some(1),
///         end_line: None,
///         end_column: None,
///         confidence: LocationConfidence::Exact,
///     },
/// };
///
/// let analysis = detect_pattern(&type_analysis, 1);
/// assert_eq!(analysis.pattern, StructPattern::Config);
/// assert!(analysis.skip_god_object_check);
/// ```
pub fn detect_pattern(type_analysis: &TypeAnalysis, responsibilities: usize) -> PatternAnalysis {
    // Try patterns in order of specificity
    if let Some(analysis) = detect_config_pattern(type_analysis, responsibilities) {
        return analysis;
    }

    if let Some(analysis) = detect_dto_pattern(type_analysis, responsibilities) {
        return analysis;
    }

    if let Some(analysis) = detect_aggregate_root_pattern(type_analysis, responsibilities) {
        return analysis;
    }

    // Default: standard struct, no special handling
    PatternAnalysis {
        pattern: StructPattern::Standard,
        confidence: 1.0,
        evidence: vec![],
        skip_god_object_check: false,
    }
}

/// Detect configuration pattern (pure function).
///
/// Indicators:
/// - Name contains "Config", "Settings", "Options"
/// - Factory methods: strict(), balanced(), lenient(), default()
/// - Low field count (< 10)
/// - Low method count (< 10)
/// - Single responsibility
fn detect_config_pattern(
    type_analysis: &TypeAnalysis,
    responsibilities: usize,
) -> Option<PatternAnalysis> {
    let mut evidence = Vec::new();
    let mut confidence = 0.0;

    // Check name
    let name_lower = type_analysis.name.to_lowercase();
    if name_lower.contains("config")
        || name_lower.contains("settings")
        || name_lower.contains("options")
    {
        evidence.push("Name indicates configuration struct".to_string());
        confidence += 0.3;
    }

    // Check for factory methods
    let factory_methods = ["strict", "balanced", "lenient", "default", "new"];
    let has_factory = type_analysis
        .methods
        .iter()
        .any(|m| factory_methods.contains(&m.as_str()));

    if has_factory {
        let found: Vec<_> = type_analysis
            .methods
            .iter()
            .filter(|m| factory_methods.contains(&m.as_str()))
            .collect();
        evidence.push(format!("Has factory methods: {:?}", found));
        confidence += 0.4;
    }

    // Check metric constraints
    if type_analysis.field_count <= 10 && type_analysis.method_count <= 10 {
        evidence.push(format!(
            "Reasonable size: {} fields, {} methods",
            type_analysis.field_count, type_analysis.method_count
        ));
        confidence += 0.2;
    }

    // Single responsibility is expected for config
    if responsibilities <= 1 {
        evidence.push("Single responsibility (configuration)".to_string());
        confidence += 0.1;
    }

    // Need strong evidence (>= 0.6) to classify as config
    if confidence >= 0.6 {
        Some(PatternAnalysis {
            pattern: StructPattern::Config,
            confidence,
            evidence,
            skip_god_object_check: true,
        })
    } else {
        None
    }
}

/// Detect Data Transfer Object pattern (pure function).
///
/// Indicators:
/// - Many fields (>= 15)
/// - Minimal methods (<= 3)
/// - Low method-to-field ratio (< 0.2)
/// - Single responsibility
/// - Name patterns: *Data, *Dto, *Item, *Record, *Result, *Metrics
fn detect_dto_pattern(
    type_analysis: &TypeAnalysis,
    responsibilities: usize,
) -> Option<PatternAnalysis> {
    let mut evidence = Vec::new();
    let mut confidence = 0.0;

    // High field count is expected for DTOs
    if type_analysis.field_count >= 15 {
        evidence.push(format!("High field count: {}", type_analysis.field_count));
        confidence += 0.3;
    }

    // Minimal behavior
    if type_analysis.method_count <= 3 {
        evidence.push(format!(
            "Minimal methods ({}), indicating data container",
            type_analysis.method_count
        ));
        confidence += 0.3;
    }

    // Check method-to-field ratio
    let ratio = type_analysis.method_count as f64 / type_analysis.field_count.max(1) as f64;
    if ratio < 0.2 {
        evidence.push(format!(
            "Low method-to-field ratio ({:.2}), data-heavy",
            ratio
        ));
        confidence += 0.2;
    }

    // Single responsibility (all fields relate to one concept)
    if responsibilities <= 1 {
        evidence.push("Single conceptual responsibility".to_string());
        confidence += 0.2;
    }

    // Check name patterns
    let name_lower = type_analysis.name.to_lowercase();
    let dto_suffixes = [
        "data", "dto", "item", "record", "result", "metrics", "analysis",
    ];
    if dto_suffixes.iter().any(|s| name_lower.ends_with(s)) {
        evidence.push(format!("Name pattern suggests DTO: {}", type_analysis.name));
        confidence += 0.1;
    }

    // Need strong evidence (>= 0.7) for DTO classification
    if confidence >= 0.7 {
        Some(PatternAnalysis {
            pattern: StructPattern::DataTransferObject,
            confidence,
            evidence,
            skip_god_object_check: true,
        })
    } else {
        None
    }
}

/// Detect Aggregate Root pattern (pure function).
///
/// Indicators:
/// - Many fields but single cohesive responsibility
/// - Moderate method count (related to managing the aggregate)
/// - Name patterns: domain entities
///
/// This is more lenient than DTO - represents a valid domain design
/// where a single entity legitimately needs many fields.
fn detect_aggregate_root_pattern(
    type_analysis: &TypeAnalysis,
    responsibilities: usize,
) -> Option<PatternAnalysis> {
    let mut evidence = Vec::new();
    let mut confidence = 0.0;

    // Must have single responsibility
    if responsibilities > 1 {
        return None; // Multiple responsibilities = not an aggregate root
    }

    evidence.push("Single responsibility detected".to_string());
    confidence += 0.4;

    // High field count is REQUIRED for aggregate root (not optional)
    if type_analysis.field_count < 10 {
        return None; // Too few fields to be an aggregate root
    }

    evidence.push(format!(
        "Complex domain entity: {} fields",
        type_analysis.field_count
    ));
    confidence += 0.2;

    // Moderate methods (domain operations)
    if type_analysis.method_count >= 5 && type_analysis.method_count <= 20 {
        evidence.push(format!(
            "Moderate method count ({}), within domain complexity",
            type_analysis.method_count
        ));
        confidence += 0.2;
    }

    // Higher tolerance for aggregate roots - they're often complex
    // but if single responsibility, likely valid domain design
    if confidence >= 0.6 {
        Some(PatternAnalysis {
            pattern: StructPattern::AggregateRoot,
            confidence,
            evidence,
            skip_god_object_check: false, // Still check, but with context
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{LocationConfidence, SourceLocation};

    fn make_type_analysis(
        name: &str,
        methods: Vec<&str>,
        method_count: usize,
        field_count: usize,
    ) -> TypeAnalysis {
        TypeAnalysis {
            name: name.to_string(),
            method_count,
            field_count,
            methods: methods.into_iter().map(String::from).collect(),
            fields: vec![],
            responsibilities: vec![],
            trait_implementations: 0,
            location: SourceLocation {
                line: 1,
                column: Some(1),
                end_line: None,
                end_column: None,
                confidence: LocationConfidence::Exact,
            },
        }
    }

    #[test]
    fn test_config_pattern_detected() {
        let type_analysis = make_type_analysis(
            "FunctionalAnalysisConfig",
            vec!["strict", "balanced", "lenient", "should_analyze"],
            4,
            5,
        );

        let analysis = detect_pattern(&type_analysis, 1);
        assert_eq!(analysis.pattern, StructPattern::Config);
        assert!(analysis.confidence >= 0.6);
        assert!(analysis.skip_god_object_check);
    }

    #[test]
    fn test_dto_pattern_detected() {
        let type_analysis =
            make_type_analysis("UnifiedDebtItem", vec!["with_pattern_analysis"], 1, 35);

        let analysis = detect_pattern(&type_analysis, 1);
        assert_eq!(analysis.pattern, StructPattern::DataTransferObject);
        assert!(analysis.confidence >= 0.7);
        assert!(analysis.skip_god_object_check);
    }

    #[test]
    fn test_genuine_god_object_not_dto() {
        // Many fields AND many methods AND multiple responsibilities
        let type_analysis = make_type_analysis(
            "UserManager",
            vec![
                "create_user",
                "delete_user",
                "send_email",
                "log_activity",
                "validate_input",
                "render_template",
            ],
            25,
            20,
        );

        let analysis = detect_pattern(&type_analysis, 5); // Multiple responsibilities
        assert_eq!(analysis.pattern, StructPattern::Standard);
        assert!(!analysis.skip_god_object_check);
    }

    #[test]
    fn test_aggregate_root_pattern() {
        let type_analysis = make_type_analysis(
            "Order",
            vec!["add_item", "calculate_total", "apply_discount", "validate"],
            8,
            12,
        );

        let analysis = detect_pattern(&type_analysis, 1);
        assert_eq!(analysis.pattern, StructPattern::AggregateRoot);
        assert!(!analysis.skip_god_object_check); // Still check but with context
    }

    #[test]
    fn test_standard_struct() {
        let type_analysis = make_type_analysis("Helper", vec!["process"], 5, 3);

        let analysis = detect_pattern(&type_analysis, 1);
        assert_eq!(analysis.pattern, StructPattern::Standard);
    }
}
