//! Debt type definitions and related implementations.
//!
//! This module defines the core debt types used throughout debtmap analysis.
//! Following Stillwater principles: pure data types with derived implementations.

use crate::core::Priority;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Unified macro to hash enum variant fields.
///
/// Handles both regular Hash-implementing fields and f64 fields (via bit conversion).
/// Use `@f64 field` for f64 values, regular identifiers for everything else.
///
/// # Examples
/// ```ignore
/// hash_fields!(state, name, age);                    // Regular fields
/// hash_fields!(state, @f64 score, count);           // f64 + regular
/// hash_fields!(state, a, @f64 b, c, @f64 d);        // Mixed order
/// ```
macro_rules! hash_fields {
    // Base case: done
    ($state:expr $(,)?) => {};

    // f64 field (marked with @f64)
    ($state:expr, @f64 $field:expr $(, $($rest:tt)*)?) => {{
        $field.to_bits().hash($state);
        hash_fields!($state $(, $($rest)*)?);
    }};

    // Regular field (implements Hash)
    ($state:expr, $field:expr $(, $($rest:tt)*)?) => {{
        $field.hash($state);
        hash_fields!($state $(, $($rest)*)?);
    }};
}

/// Visibility level for functions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionVisibility {
    Private,
    Crate,
    Public,
}

/// Types of technical debt that debtmap can identify.
///
/// Each variant captures specific metrics relevant to that type of debt.
/// The enum is designed to be:
/// - Exhaustive: covers all detected debt patterns
/// - Self-describing: variant names indicate the issue type
/// - Data-rich: captures relevant metrics for prioritization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DebtType {
    // Legacy variants from core::DebtType (spec 203)
    Todo {
        reason: Option<String>,
    },
    Fixme {
        reason: Option<String>,
    },
    CodeSmell {
        smell_type: Option<String>,
    },
    Complexity {
        cyclomatic: u32,
        cognitive: u32,
    },
    Dependency {
        dependency_type: Option<String>,
    },
    ResourceManagement {
        issue_type: Option<String>,
    },
    CodeOrganization {
        issue_type: Option<String>,
    },
    TestComplexity {
        cyclomatic: u32,
        cognitive: u32,
    },
    TestQuality {
        issue_type: Option<String>,
    },
    // Priority-specific variants
    TestingGap {
        coverage: f64,
        cyclomatic: u32,
        cognitive: u32,
    },
    ComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
    },
    DeadCode {
        visibility: FunctionVisibility,
        cyclomatic: u32,
        cognitive: u32,
        usage_hints: Vec<String>,
    },
    Duplication {
        instances: u32,
        total_lines: u32,
    },
    Risk {
        risk_score: f64,
        factors: Vec<String>,
    },
    // Test-specific debt types
    TestComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
        threshold: u32,
    },
    TestTodo {
        priority: Priority,
        reason: Option<String>,
    },
    TestDuplication {
        instances: u32,
        total_lines: u32,
        similarity: f64,
    },
    ErrorSwallowing {
        pattern: String,
        context: Option<String>,
    },
    // Resource Management debt types
    AllocationInefficiency {
        pattern: String,
        impact: String,
    },
    StringConcatenation {
        loop_type: String,
        iterations: Option<u32>,
    },
    NestedLoops {
        depth: u32,
        complexity_estimate: String,
    },
    BlockingIO {
        operation: String,
        context: String,
    },
    SuboptimalDataStructure {
        current_type: String,
        recommended_type: String,
    },
    // Organization debt types
    /// Unified god object variant representing all detection types (GodClass, GodFile, GodModule)
    /// The `god_object_indicators.detection_type` field distinguishes between these types
    GodObject {
        /// Number of methods (for GodClass) or functions (for GodFile/GodModule)
        methods: u32,
        /// Number of fields - Some for GodClass, None for GodFile/GodModule
        fields: Option<u32>,
        responsibilities: u32,
        god_object_score: f64,
        /// Total lines of code
        lines: u32,
    },
    FeatureEnvy {
        external_class: String,
        usage_ratio: f64,
    },
    PrimitiveObsession {
        primitive_type: String,
        domain_concept: String,
    },
    MagicValues {
        value: String,
        occurrences: u32,
    },
    // Testing quality debt types
    AssertionComplexity {
        assertion_count: u32,
        complexity_score: f64,
    },
    FlakyTestPattern {
        pattern_type: String,
        reliability_impact: String,
    },
    // Resource management debt types
    AsyncMisuse {
        pattern: String,
        performance_impact: String,
    },
    ResourceLeak {
        resource_type: String,
        cleanup_missing: String,
    },
    CollectionInefficiency {
        collection_type: String,
        inefficiency_type: String,
    },
    // Type organization debt types (Spec 187)
    ScatteredType {
        type_name: String,
        total_methods: usize,
        file_count: usize,
        severity: String,
    },
    OrphanedFunctions {
        target_type: String,
        function_count: usize,
        file_count: usize,
    },
    UtilitiesSprawl {
        function_count: usize,
        distinct_types: usize,
    },
}

impl DebtType {
    /// Pure function: returns the display name for this debt type.
    ///
    /// For most variants, this returns a static string. The `ErrorSwallowing`
    /// variant is the exception - it requires dynamic content and is handled
    /// separately in the `Display` impl.
    ///
    /// This separation follows the Stillwater pattern: pure core logic in
    /// a helper function, I/O (formatting) at the boundary.
    pub fn display_name(&self) -> &'static str {
        match self {
            // Legacy variants
            Self::Todo { .. } => "TODO",
            Self::Fixme { .. } => "FIXME",
            Self::CodeSmell { .. } => "Code Smell",
            Self::Complexity { .. } => "Complexity",
            Self::Dependency { .. } => "Dependency",
            Self::ResourceManagement { .. } => "Resource Management",
            Self::CodeOrganization { .. } => "Code Organization",
            Self::TestComplexity { .. } => "Test Complexity",
            Self::TestQuality { .. } => "Test Quality",
            // Priority-specific variants
            Self::TestingGap { .. } => "Testing Gap",
            Self::ComplexityHotspot { .. } => "Complexity Hotspot",
            Self::DeadCode { .. } => "Dead Code",
            Self::Duplication { .. } => "Duplication",
            Self::Risk { .. } => "Risk",
            Self::TestComplexityHotspot { .. } => "Test Complexity Hotspot",
            Self::TestTodo { .. } => "Test TODO",
            Self::TestDuplication { .. } => "Test Duplication",
            // ErrorSwallowing has dynamic content - use placeholder
            Self::ErrorSwallowing { .. } => "Error Swallowing",
            Self::AllocationInefficiency { .. } => "Allocation Inefficiency",
            Self::StringConcatenation { .. } => "String Concatenation",
            Self::NestedLoops { .. } => "Nested Loops",
            Self::BlockingIO { .. } => "Blocking I/O",
            Self::SuboptimalDataStructure { .. } => "Suboptimal Data Structure",
            Self::GodObject { .. } => "God Object",
            Self::FeatureEnvy { .. } => "Feature Envy",
            Self::PrimitiveObsession { .. } => "Primitive Obsession",
            Self::MagicValues { .. } => "Magic Values",
            Self::AssertionComplexity { .. } => "Assertion Complexity",
            Self::FlakyTestPattern { .. } => "Flaky Test Pattern",
            Self::AsyncMisuse { .. } => "Async Misuse",
            Self::ResourceLeak { .. } => "Resource Leak",
            Self::CollectionInefficiency { .. } => "Collection Inefficiency",
            Self::ScatteredType { .. } => "Scattered Type",
            Self::OrphanedFunctions { .. } => "Orphaned Functions",
            Self::UtilitiesSprawl { .. } => "Utilities Sprawl",
        }
    }
}

// Custom Eq implementation that handles f64 fields by comparing their bit representations
impl Eq for DebtType {}

// Custom Hash implementation that handles f64 fields by hashing their bit representations.
// Uses `hash_fields!` macro with `@f64` marker for float fields.
impl Hash for DebtType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);

        match self {
            // Single Option<String> field variants
            Self::Todo { reason }
            | Self::Fixme { reason }
            | Self::CodeSmell { smell_type: reason }
            | Self::Dependency {
                dependency_type: reason,
            }
            | Self::ResourceManagement { issue_type: reason }
            | Self::CodeOrganization { issue_type: reason }
            | Self::TestQuality { issue_type: reason } => hash_fields!(state, reason),

            // Complexity pair variants (cyclomatic, cognitive)
            Self::Complexity {
                cyclomatic,
                cognitive,
            }
            | Self::TestComplexity {
                cyclomatic,
                cognitive,
            }
            | Self::ComplexityHotspot {
                cyclomatic,
                cognitive,
            } => {
                hash_fields!(state, cyclomatic, cognitive)
            }

            // Variants with f64 fields
            Self::TestingGap {
                coverage,
                cyclomatic,
                cognitive,
            } => {
                hash_fields!(state, @f64 coverage, cyclomatic, cognitive)
            }
            Self::Risk {
                risk_score,
                factors,
            } => hash_fields!(state, @f64 risk_score, factors),
            Self::TestDuplication {
                instances,
                total_lines,
                similarity,
            } => {
                hash_fields!(state, instances, total_lines, @f64 similarity)
            }
            Self::GodObject {
                methods,
                fields,
                responsibilities,
                god_object_score,
                lines,
            } => {
                hash_fields!(state, methods, fields, responsibilities, @f64 god_object_score, lines)
            }
            Self::FeatureEnvy {
                external_class,
                usage_ratio,
            } => {
                hash_fields!(state, external_class, @f64 usage_ratio)
            }
            Self::AssertionComplexity {
                assertion_count,
                complexity_score,
            } => {
                hash_fields!(state, assertion_count, @f64 complexity_score)
            }

            // (String, Option<String>) variant
            Self::ErrorSwallowing { pattern, context } => hash_fields!(state, pattern, context),

            // (String, String) pair variants
            Self::AllocationInefficiency { pattern, impact }
            | Self::BlockingIO {
                operation: pattern,
                context: impact,
            }
            | Self::AsyncMisuse {
                pattern,
                performance_impact: impact,
            }
            | Self::ResourceLeak {
                resource_type: pattern,
                cleanup_missing: impact,
            }
            | Self::FlakyTestPattern {
                pattern_type: pattern,
                reliability_impact: impact,
            }
            | Self::SuboptimalDataStructure {
                current_type: pattern,
                recommended_type: impact,
            }
            | Self::PrimitiveObsession {
                primitive_type: pattern,
                domain_concept: impact,
            }
            | Self::CollectionInefficiency {
                collection_type: pattern,
                inefficiency_type: impact,
            } => {
                hash_fields!(state, pattern, impact)
            }

            // Remaining multi-field variants
            Self::DeadCode {
                visibility,
                cyclomatic,
                cognitive,
                usage_hints,
            } => {
                hash_fields!(state, visibility, cyclomatic, cognitive, usage_hints)
            }
            Self::Duplication {
                instances,
                total_lines,
            } => {
                hash_fields!(state, instances, total_lines)
            }
            Self::TestComplexityHotspot {
                cyclomatic,
                cognitive,
                threshold,
            } => {
                hash_fields!(state, cyclomatic, cognitive, threshold)
            }
            Self::TestTodo { priority, reason } => hash_fields!(state, priority, reason),
            Self::StringConcatenation {
                loop_type,
                iterations,
            } => {
                hash_fields!(state, loop_type, iterations)
            }
            Self::NestedLoops {
                depth,
                complexity_estimate,
            } => {
                hash_fields!(state, depth, complexity_estimate)
            }
            Self::MagicValues { value, occurrences } => hash_fields!(state, value, occurrences),
            Self::ScatteredType {
                type_name,
                total_methods,
                file_count,
                severity,
            } => {
                hash_fields!(state, type_name, total_methods, file_count, severity)
            }
            Self::OrphanedFunctions {
                target_type,
                function_count,
                file_count,
            } => {
                hash_fields!(state, target_type, function_count, file_count)
            }
            Self::UtilitiesSprawl {
                function_count,
                distinct_types,
            } => {
                hash_fields!(state, function_count, distinct_types)
            }
        }
    }
}

impl std::fmt::Display for DebtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // ErrorSwallowing includes dynamic content - handle separately
        if let Self::ErrorSwallowing { pattern, .. } = self {
            return write!(f, "Error Swallowing: {}", pattern);
        }
        // All other variants use the pure display_name() helper
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_todo() {
        let debt = DebtType::Todo {
            reason: Some("fix later".into()),
        };
        assert_eq!(debt.to_string(), "TODO");
    }

    #[test]
    fn display_error_swallowing_includes_pattern() {
        let debt = DebtType::ErrorSwallowing {
            pattern: "unwrap()".into(),
            context: None,
        };
        assert_eq!(debt.to_string(), "Error Swallowing: unwrap()");
    }

    #[test]
    fn hash_consistency() {
        use std::collections::hash_map::DefaultHasher;

        let debt1 = DebtType::GodObject {
            methods: 50,
            fields: Some(30),
            responsibilities: 5,
            god_object_score: 85.0,
            lines: 2000,
        };
        let debt2 = debt1.clone();

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        debt1.hash(&mut hasher1);
        debt2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn hash_different_for_different_values() {
        use std::collections::hash_map::DefaultHasher;

        let debt1 = DebtType::Complexity {
            cyclomatic: 10,
            cognitive: 5,
        };
        let debt2 = DebtType::Complexity {
            cyclomatic: 10,
            cognitive: 6,
        };

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        debt1.hash(&mut hasher1);
        debt2.hash(&mut hasher2);

        assert_ne!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn display_name_returns_non_empty_for_all_variants() {
        let variants: Vec<DebtType> = vec![
            DebtType::Todo { reason: None },
            DebtType::Fixme { reason: None },
            DebtType::CodeSmell { smell_type: None },
            DebtType::Complexity {
                cyclomatic: 0,
                cognitive: 0,
            },
            DebtType::GodObject {
                methods: 0,
                fields: None,
                responsibilities: 0,
                god_object_score: 0.0,
                lines: 0,
            },
        ];

        for variant in variants {
            let display = variant.to_string();
            assert!(!display.is_empty(), "Empty display for {:?}", variant);
        }
    }
}
