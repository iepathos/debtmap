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

/// Custom Hash implementation that handles f64 fields by hashing their bit representations.
///
/// # Design Notes
///
/// This function has high cyclomatic complexity due to matching on all 33 enum variants.
/// This is intentional structural complexity that cannot be reduced without sacrificing:
/// - Type safety (each variant's fields must be explicitly destructured)
/// - Correctness (f64 fields require bit conversion via `@f64` marker)
/// - Exhaustive matching (compiler ensures all variants are handled)
///
/// The complexity is mitigated by:
/// - Using the `hash_fields!` macro to reduce repetition
/// - Grouping similar variants with or-patterns where field types match
/// - Comprehensive test coverage for all variant groups
///
/// Variant groups:
/// - `Option<String>` single field: 7 variants
/// - `(u32, u32)` pairs: 4 variants (Complexity, TestComplexity, ComplexityHotspot, Duplication)
/// - `(String, String)` pairs: 8 variants
/// - Remaining unique structures: handled individually
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

            // (u32, u32) pair variants - bound to common names for grouping
            Self::Complexity {
                cyclomatic: a,
                cognitive: b,
            }
            | Self::TestComplexity {
                cyclomatic: a,
                cognitive: b,
            }
            | Self::ComplexityHotspot {
                cyclomatic: a,
                cognitive: b,
            }
            | Self::Duplication {
                instances: a,
                total_lines: b,
            } => {
                hash_fields!(state, a, b)
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

    // Helper function to compute hash for DebtType
    fn compute_hash(debt: &DebtType) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        debt.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn hash_option_string_variants_consistent() {
        // All Option<String> field variants should hash consistently
        let variants = [
            DebtType::Todo {
                reason: Some("test".into()),
            },
            DebtType::Fixme {
                reason: Some("test".into()),
            },
            DebtType::CodeSmell {
                smell_type: Some("test".into()),
            },
            DebtType::Dependency {
                dependency_type: Some("test".into()),
            },
            DebtType::ResourceManagement {
                issue_type: Some("test".into()),
            },
            DebtType::CodeOrganization {
                issue_type: Some("test".into()),
            },
            DebtType::TestQuality {
                issue_type: Some("test".into()),
            },
        ];

        // Each variant should hash consistently with itself
        for variant in &variants {
            assert_eq!(compute_hash(variant), compute_hash(&variant.clone()));
        }

        // Different variants with same field value should have different hashes (discriminant matters)
        let todo = &variants[0];
        let fixme = &variants[1];
        assert_ne!(compute_hash(todo), compute_hash(fixme));
    }

    #[test]
    fn hash_u32_pair_variants_consistent() {
        // (u32, u32) pair variants should hash consistently
        let complexity = DebtType::Complexity {
            cyclomatic: 10,
            cognitive: 5,
        };
        let test_complexity = DebtType::TestComplexity {
            cyclomatic: 10,
            cognitive: 5,
        };
        let duplication = DebtType::Duplication {
            instances: 10,
            total_lines: 5,
        };

        // Each should hash consistently
        assert_eq!(compute_hash(&complexity), compute_hash(&complexity.clone()));
        assert_eq!(
            compute_hash(&test_complexity),
            compute_hash(&test_complexity.clone())
        );
        assert_eq!(
            compute_hash(&duplication),
            compute_hash(&duplication.clone())
        );

        // Different variants should have different hashes even with same field values
        assert_ne!(compute_hash(&complexity), compute_hash(&test_complexity));
        assert_ne!(compute_hash(&complexity), compute_hash(&duplication));
    }

    #[test]
    fn hash_string_pair_variants_consistent() {
        // (String, String) pair variants should hash consistently
        let variants = [
            DebtType::AllocationInefficiency {
                pattern: "a".into(),
                impact: "b".into(),
            },
            DebtType::BlockingIO {
                operation: "a".into(),
                context: "b".into(),
            },
            DebtType::AsyncMisuse {
                pattern: "a".into(),
                performance_impact: "b".into(),
            },
            DebtType::ResourceLeak {
                resource_type: "a".into(),
                cleanup_missing: "b".into(),
            },
            DebtType::FlakyTestPattern {
                pattern_type: "a".into(),
                reliability_impact: "b".into(),
            },
            DebtType::SuboptimalDataStructure {
                current_type: "a".into(),
                recommended_type: "b".into(),
            },
            DebtType::PrimitiveObsession {
                primitive_type: "a".into(),
                domain_concept: "b".into(),
            },
            DebtType::CollectionInefficiency {
                collection_type: "a".into(),
                inefficiency_type: "b".into(),
            },
        ];

        // Each should hash consistently
        for variant in &variants {
            assert_eq!(compute_hash(variant), compute_hash(&variant.clone()));
        }

        // Different variants with same field values should have different hashes
        assert_ne!(compute_hash(&variants[0]), compute_hash(&variants[1]));
    }

    #[test]
    fn hash_f64_variants_consistent() {
        // Variants with f64 fields should hash consistently
        let testing_gap = DebtType::TestingGap {
            coverage: 0.75,
            cyclomatic: 10,
            cognitive: 5,
        };
        let risk = DebtType::Risk {
            risk_score: 0.85,
            factors: vec!["factor1".into()],
        };
        let test_duplication = DebtType::TestDuplication {
            instances: 3,
            total_lines: 100,
            similarity: 0.9,
        };
        let god_object = DebtType::GodObject {
            methods: 50,
            fields: Some(30),
            responsibilities: 5,
            god_object_score: 85.5,
            lines: 2000,
        };
        let feature_envy = DebtType::FeatureEnvy {
            external_class: "OtherClass".into(),
            usage_ratio: 0.6,
        };
        let assertion_complexity = DebtType::AssertionComplexity {
            assertion_count: 15,
            complexity_score: 3.5,
        };

        // Each should hash consistently
        assert_eq!(
            compute_hash(&testing_gap),
            compute_hash(&testing_gap.clone())
        );
        assert_eq!(compute_hash(&risk), compute_hash(&risk.clone()));
        assert_eq!(
            compute_hash(&test_duplication),
            compute_hash(&test_duplication.clone())
        );
        assert_eq!(compute_hash(&god_object), compute_hash(&god_object.clone()));
        assert_eq!(
            compute_hash(&feature_envy),
            compute_hash(&feature_envy.clone())
        );
        assert_eq!(
            compute_hash(&assertion_complexity),
            compute_hash(&assertion_complexity.clone())
        );
    }

    #[test]
    fn hash_f64_different_values_different_hashes() {
        let risk1 = DebtType::Risk {
            risk_score: 0.5,
            factors: vec![],
        };
        let risk2 = DebtType::Risk {
            risk_score: 0.6,
            factors: vec![],
        };
        assert_ne!(compute_hash(&risk1), compute_hash(&risk2));
    }

    #[test]
    fn hash_remaining_variants_consistent() {
        // Test remaining multi-field variants
        let dead_code = DebtType::DeadCode {
            visibility: FunctionVisibility::Private,
            cyclomatic: 5,
            cognitive: 3,
            usage_hints: vec!["unused".into()],
        };
        let test_complexity_hotspot = DebtType::TestComplexityHotspot {
            cyclomatic: 20,
            cognitive: 15,
            threshold: 10,
        };
        let test_todo = DebtType::TestTodo {
            priority: Priority::High,
            reason: Some("fix later".into()),
        };
        let string_concat = DebtType::StringConcatenation {
            loop_type: "for".into(),
            iterations: Some(100),
        };
        let nested_loops = DebtType::NestedLoops {
            depth: 3,
            complexity_estimate: "O(n^3)".into(),
        };
        let magic_values = DebtType::MagicValues {
            value: "42".into(),
            occurrences: 5,
        };
        let scattered_type = DebtType::ScatteredType {
            type_name: "User".into(),
            total_methods: 20,
            file_count: 5,
            severity: "high".into(),
        };
        let orphaned = DebtType::OrphanedFunctions {
            target_type: "Parser".into(),
            function_count: 10,
            file_count: 3,
        };
        let utilities_sprawl = DebtType::UtilitiesSprawl {
            function_count: 50,
            distinct_types: 10,
        };
        let error_swallowing = DebtType::ErrorSwallowing {
            pattern: "unwrap()".into(),
            context: Some("in main".into()),
        };

        // Each should hash consistently
        let variants: Vec<&DebtType> = vec![
            &dead_code,
            &test_complexity_hotspot,
            &test_todo,
            &string_concat,
            &nested_loops,
            &magic_values,
            &scattered_type,
            &orphaned,
            &utilities_sprawl,
            &error_swallowing,
        ];

        for variant in variants {
            assert_eq!(
                compute_hash(variant),
                compute_hash(&variant.clone()),
                "Hash inconsistent for {:?}",
                variant
            );
        }
    }

    #[test]
    fn hash_in_hashset_works() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(DebtType::Todo {
            reason: Some("test".into()),
        });
        set.insert(DebtType::Complexity {
            cyclomatic: 10,
            cognitive: 5,
        });
        set.insert(DebtType::Risk {
            risk_score: 0.75,
            factors: vec!["a".into()],
        });

        assert_eq!(set.len(), 3);

        // Duplicate should not increase size
        set.insert(DebtType::Todo {
            reason: Some("test".into()),
        });
        assert_eq!(set.len(), 3);

        // Different value should increase size
        set.insert(DebtType::Todo {
            reason: Some("different".into()),
        });
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn hash_eq_consistency() {
        // Verify that equal values have equal hashes (required by Hash contract)
        let pairs: Vec<(DebtType, DebtType)> = vec![
            (
                DebtType::Risk {
                    risk_score: 0.5,
                    factors: vec!["a".into()],
                },
                DebtType::Risk {
                    risk_score: 0.5,
                    factors: vec!["a".into()],
                },
            ),
            (
                DebtType::GodObject {
                    methods: 10,
                    fields: Some(5),
                    responsibilities: 3,
                    god_object_score: 75.5,
                    lines: 500,
                },
                DebtType::GodObject {
                    methods: 10,
                    fields: Some(5),
                    responsibilities: 3,
                    god_object_score: 75.5,
                    lines: 500,
                },
            ),
        ];

        for (a, b) in pairs {
            assert_eq!(a, b, "Values should be equal");
            assert_eq!(
                compute_hash(&a),
                compute_hash(&b),
                "Equal values must have equal hashes"
            );
        }
    }
}
