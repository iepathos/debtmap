use crate::common::{LocationConfidence, SourceLocation};
use crate::core::{DebtItem, DebtType, Priority};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PerformanceAntiPattern {
    NestedLoop {
        nesting_level: u32,
        estimated_complexity: ComplexityClass,
        inner_operations: Vec<LoopOperation>,
        can_parallelize: bool,
        location: SourceLocation,
    },
    InefficientDataStructure {
        operation: DataStructureOperation,
        collection_type: String,
        recommended_alternative: String,
        performance_impact: PerformanceImpact,
        location: SourceLocation,
    },
    ExcessiveAllocation {
        allocation_type: AllocationType,
        frequency: AllocationFrequency,
        suggested_optimization: String,
        location: SourceLocation,
    },
    InefficientIO {
        io_pattern: IOPattern,
        batching_opportunity: bool,
        async_opportunity: bool,
        location: SourceLocation,
    },
    StringProcessingAntiPattern {
        pattern_type: StringAntiPattern,
        performance_impact: PerformanceImpact,
        recommended_approach: String,
        location: SourceLocation,
    },
}

impl PerformanceAntiPattern {
    pub fn location(&self) -> &SourceLocation {
        match self {
            PerformanceAntiPattern::NestedLoop { location, .. } => location,
            PerformanceAntiPattern::InefficientDataStructure { location, .. } => location,
            PerformanceAntiPattern::ExcessiveAllocation { location, .. } => location,
            PerformanceAntiPattern::InefficientIO { location, .. } => location,
            PerformanceAntiPattern::StringProcessingAntiPattern { location, .. } => location,
        }
    }

    pub fn primary_line(&self) -> usize {
        self.location().line
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplexityClass {
    Linear,      // O(n)
    Quadratic,   // O(n²)
    Cubic,       // O(n³)
    Exponential, // O(2^n)
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoopOperation {
    CollectionIteration,
    DatabaseQuery,
    FileIO,
    NetworkRequest,
    Computation,
    StringOperation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataStructureOperation {
    Contains,
    LinearSearch,
    FrequentInsertion,
    FrequentDeletion,
    RandomAccess,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AllocationType {
    Clone,
    StringConcatenation,
    TemporaryCollection,
    LargeStackAllocation,
    RepeatedBoxing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AllocationFrequency {
    InLoop,
    InHotPath,
    Recursive,
    Occasional,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IOPattern {
    SyncInLoop,
    UnbatchedQueries,
    UnbufferedIO,
    ExcessiveConnections,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StringAntiPattern {
    ConcatenationInLoop,
    RepeatedFormatting,
    RegexInLoop,
    InefficientParsing,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PerformanceImpact {
    Critical, // 10x+ performance impact
    High,     // 3-10x performance impact
    Medium,   // 1.5-3x performance impact
    Low,      // <1.5x performance impact
}

pub trait PerformanceDetector {
    fn detect_anti_patterns(&self, file: &syn::File, path: &Path) -> Vec<PerformanceAntiPattern>;
    fn detector_name(&self) -> &'static str;
    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact;
}

pub mod allocation_detector;
pub mod data_structure_detector;
pub mod io_detector;
pub mod location_extractor;
pub mod nested_loop_detector;
pub mod string_detector;

pub use allocation_detector::AllocationDetector;
pub use data_structure_detector::DataStructureDetector;
pub use io_detector::IOPerformanceDetector;
pub use location_extractor::LocationExtractor;
pub use nested_loop_detector::NestedLoopDetector;
pub use string_detector::StringPerformanceDetector;

pub fn convert_performance_pattern_to_debt_item(
    pattern: PerformanceAntiPattern,
    impact: PerformanceImpact,
    path: &Path,
) -> DebtItem {
    let location = pattern.location();
    let line = location.line;
    let priority = match &pattern {
        PerformanceAntiPattern::NestedLoop {
            estimated_complexity,
            ..
        } => classify_nested_loop_priority(estimated_complexity),
        PerformanceAntiPattern::InefficientIO { .. } => Priority::High,
        _ => impact_to_priority(impact),
    };

    let message = format_pattern_message(&pattern);
    let recommendation = generate_pattern_recommendation(&pattern);

    DebtItem {
        id: format!("performance-{}-{}", path.display(), line),
        debt_type: DebtType::Performance,
        priority,
        file: path.to_path_buf(),
        line,
        column: location.column,
        message,
        context: Some(format!(
            "{}
Location confidence: {:?}",
            recommendation, location.confidence
        )),
    }
}

fn impact_to_priority(impact: PerformanceImpact) -> Priority {
    match impact {
        PerformanceImpact::Critical => Priority::Critical,
        PerformanceImpact::High => Priority::High,
        PerformanceImpact::Medium => Priority::Medium,
        PerformanceImpact::Low => Priority::Low,
    }
}

/// Classify nested loop complexity into priority level
fn classify_nested_loop_priority(complexity: &ComplexityClass) -> Priority {
    match complexity {
        ComplexityClass::Exponential => Priority::Critical,
        ComplexityClass::Cubic => Priority::High,
        ComplexityClass::Quadratic => Priority::Medium,
        _ => Priority::Low,
    }
}

/// Format a performance pattern into a human-readable message
fn format_pattern_message(pattern: &PerformanceAntiPattern) -> String {
    match pattern {
        PerformanceAntiPattern::NestedLoop {
            nesting_level,
            estimated_complexity,
            ..
        } => format!(
            "Nested loop with {} levels ({:?} complexity)",
            nesting_level, estimated_complexity
        ),
        PerformanceAntiPattern::InefficientDataStructure {
            operation,
            collection_type,
            ..
        } => format!(
            "{:?} operation on {} in performance-critical code",
            operation, collection_type
        ),
        PerformanceAntiPattern::ExcessiveAllocation {
            allocation_type,
            frequency,
            ..
        } => format!("{:?} allocation {:?}", allocation_type, frequency),
        PerformanceAntiPattern::InefficientIO { io_pattern, .. } => {
            format!("Inefficient I/O pattern: {:?}", io_pattern)
        }
        PerformanceAntiPattern::StringProcessingAntiPattern { pattern_type, .. } => {
            format!("Inefficient string processing: {:?}", pattern_type)
        }
    }
}

/// Generate recommendation for a performance pattern
fn generate_pattern_recommendation(pattern: &PerformanceAntiPattern) -> String {
    match pattern {
        PerformanceAntiPattern::NestedLoop {
            can_parallelize, ..
        } => {
            let mut rec = "Consider algorithm optimization or caching".to_string();
            if *can_parallelize {
                rec.push_str(" (parallelization possible)");
            }
            rec
        }
        PerformanceAntiPattern::InefficientDataStructure {
            recommended_alternative,
            ..
        } => format!(
            "Consider using {} for better performance",
            recommended_alternative
        ),
        PerformanceAntiPattern::ExcessiveAllocation {
            suggested_optimization,
            ..
        } => suggested_optimization.clone(),
        PerformanceAntiPattern::InefficientIO {
            batching_opportunity,
            async_opportunity,
            ..
        } => {
            let mut recommendations = Vec::new();
            if *batching_opportunity {
                recommendations.push("batch operations");
            }
            if *async_opportunity {
                recommendations.push("use async I/O");
            }
            format!("Consider: {}", recommendations.join(", "))
        }
        PerformanceAntiPattern::StringProcessingAntiPattern {
            recommended_approach,
            ..
        } => recommended_approach.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DebtType;
    use crate::Priority;
    use std::path::PathBuf;

    // Helper function to create a default SourceLocation for tests
    fn default_location() -> SourceLocation {
        SourceLocation {
            line: 1,
            column: None,
            end_line: None,
            end_column: None,
            confidence: LocationConfidence::Unavailable,
        }
    }

    #[test]
    fn test_classify_nested_loop_priority() {
        assert_eq!(
            classify_nested_loop_priority(&ComplexityClass::Exponential),
            Priority::Critical
        );
        assert_eq!(
            classify_nested_loop_priority(&ComplexityClass::Cubic),
            Priority::High
        );
        assert_eq!(
            classify_nested_loop_priority(&ComplexityClass::Quadratic),
            Priority::Medium
        );
        assert_eq!(
            classify_nested_loop_priority(&ComplexityClass::Linear),
            Priority::Low
        );
        assert_eq!(
            classify_nested_loop_priority(&ComplexityClass::Unknown),
            Priority::Low
        );
    }

    #[test]
    fn test_impact_to_priority() {
        assert_eq!(
            impact_to_priority(PerformanceImpact::Critical),
            Priority::Critical
        );
        assert_eq!(impact_to_priority(PerformanceImpact::High), Priority::High);
        assert_eq!(
            impact_to_priority(PerformanceImpact::Medium),
            Priority::Medium
        );
        assert_eq!(impact_to_priority(PerformanceImpact::Low), Priority::Low);
    }

    #[test]
    fn test_format_pattern_message_nested_loop() {
        let pattern = PerformanceAntiPattern::NestedLoop {
            nesting_level: 3,
            estimated_complexity: ComplexityClass::Cubic,
            can_parallelize: true,
            inner_operations: vec![LoopOperation::Computation],
            location: default_location(),
        };
        let message = format_pattern_message(&pattern);
        assert_eq!(message, "Nested loop with 3 levels (Cubic complexity)");
    }

    #[test]
    fn test_format_pattern_message_inefficient_data_structure() {
        let pattern = PerformanceAntiPattern::InefficientDataStructure {
            operation: DataStructureOperation::LinearSearch,
            collection_type: "Vec".to_string(),
            recommended_alternative: "HashSet".to_string(),
            performance_impact: PerformanceImpact::High,
            location: default_location(),
        };
        let message = format_pattern_message(&pattern);
        assert_eq!(
            message,
            "LinearSearch operation on Vec in performance-critical code"
        );
    }

    #[test]
    fn test_format_pattern_message_excessive_allocation() {
        let pattern = PerformanceAntiPattern::ExcessiveAllocation {
            allocation_type: AllocationType::StringConcatenation,
            frequency: AllocationFrequency::InLoop,
            suggested_optimization: "Use String::with_capacity()".to_string(),
            location: default_location(),
        };
        let message = format_pattern_message(&pattern);
        assert_eq!(message, "StringConcatenation allocation InLoop");
    }

    #[test]
    fn test_format_pattern_message_inefficient_io() {
        let pattern = PerformanceAntiPattern::InefficientIO {
            io_pattern: IOPattern::UnbufferedIO,
            batching_opportunity: true,
            async_opportunity: false,
            location: default_location(),
        };
        let message = format_pattern_message(&pattern);
        assert_eq!(message, "Inefficient I/O pattern: UnbufferedIO");
    }

    #[test]
    fn test_format_pattern_message_string_processing() {
        let pattern = PerformanceAntiPattern::StringProcessingAntiPattern {
            pattern_type: StringAntiPattern::ConcatenationInLoop,
            recommended_approach: "Use a String builder".to_string(),
            performance_impact: PerformanceImpact::Medium,
            location: default_location(),
        };
        let message = format_pattern_message(&pattern);
        assert_eq!(
            message,
            "Inefficient string processing: ConcatenationInLoop"
        );
    }

    #[test]
    fn test_generate_pattern_recommendation_nested_loop_parallel() {
        let pattern = PerformanceAntiPattern::NestedLoop {
            nesting_level: 3,
            estimated_complexity: ComplexityClass::Cubic,
            can_parallelize: true,
            inner_operations: vec![LoopOperation::Computation],
            location: default_location(),
        };
        let rec = generate_pattern_recommendation(&pattern);
        assert_eq!(
            rec,
            "Consider algorithm optimization or caching (parallelization possible)"
        );
    }

    #[test]
    fn test_generate_pattern_recommendation_nested_loop_no_parallel() {
        let pattern = PerformanceAntiPattern::NestedLoop {
            nesting_level: 2,
            estimated_complexity: ComplexityClass::Quadratic,
            can_parallelize: false,
            inner_operations: vec![LoopOperation::CollectionIteration],
            location: default_location(),
        };
        let rec = generate_pattern_recommendation(&pattern);
        assert_eq!(rec, "Consider algorithm optimization or caching");
    }

    #[test]
    fn test_generate_pattern_recommendation_data_structure() {
        let pattern = PerformanceAntiPattern::InefficientDataStructure {
            operation: DataStructureOperation::FrequentInsertion,
            collection_type: "Vec".to_string(),
            recommended_alternative: "HashMap".to_string(),
            performance_impact: PerformanceImpact::High,
            location: default_location(),
        };
        let rec = generate_pattern_recommendation(&pattern);
        assert_eq!(rec, "Consider using HashMap for better performance");
    }

    #[test]
    fn test_generate_pattern_recommendation_allocation() {
        let pattern = PerformanceAntiPattern::ExcessiveAllocation {
            allocation_type: AllocationType::TemporaryCollection,
            frequency: AllocationFrequency::InHotPath,
            suggested_optimization: "Pre-allocate with Vec::with_capacity()".to_string(),
            location: default_location(),
        };
        let rec = generate_pattern_recommendation(&pattern);
        assert_eq!(rec, "Pre-allocate with Vec::with_capacity()");
    }

    #[test]
    fn test_generate_pattern_recommendation_io_both_opportunities() {
        let pattern = PerformanceAntiPattern::InefficientIO {
            io_pattern: IOPattern::UnbufferedIO,
            batching_opportunity: true,
            async_opportunity: true,
            location: default_location(),
        };
        let rec = generate_pattern_recommendation(&pattern);
        assert_eq!(rec, "Consider: batch operations, use async I/O");
    }

    #[test]
    fn test_generate_pattern_recommendation_io_batch_only() {
        let pattern = PerformanceAntiPattern::InefficientIO {
            io_pattern: IOPattern::UnbufferedIO,
            batching_opportunity: true,
            async_opportunity: false,
            location: default_location(),
        };
        let rec = generate_pattern_recommendation(&pattern);
        assert_eq!(rec, "Consider: batch operations");
    }

    #[test]
    fn test_generate_pattern_recommendation_io_async_only() {
        let pattern = PerformanceAntiPattern::InefficientIO {
            io_pattern: IOPattern::SyncInLoop,
            batching_opportunity: false,
            async_opportunity: true,
            location: default_location(),
        };
        let rec = generate_pattern_recommendation(&pattern);
        assert_eq!(rec, "Consider: use async I/O");
    }

    #[test]
    fn test_generate_pattern_recommendation_string() {
        let pattern = PerformanceAntiPattern::StringProcessingAntiPattern {
            pattern_type: StringAntiPattern::InefficientParsing,
            recommended_approach: "Use a dedicated parser library".to_string(),
            performance_impact: PerformanceImpact::High,
            location: default_location(),
        };
        let rec = generate_pattern_recommendation(&pattern);
        assert_eq!(rec, "Use a dedicated parser library");
    }

    #[test]
    fn test_convert_performance_pattern_nested_loop_critical() {
        let pattern = PerformanceAntiPattern::NestedLoop {
            nesting_level: 4,
            estimated_complexity: ComplexityClass::Exponential,
            can_parallelize: true,
            inner_operations: vec![LoopOperation::DatabaseQuery, LoopOperation::Computation],
            location: SourceLocation {
                line: 100,
                column: None,
                end_line: None,
                end_column: None,
                confidence: LocationConfidence::Unavailable,
            },
        };
        let path = PathBuf::from("src/test.rs");
        let debt =
            convert_performance_pattern_to_debt_item(pattern, PerformanceImpact::Critical, &path);

        assert_eq!(debt.priority, Priority::Critical);
        assert_eq!(debt.debt_type, DebtType::Performance);
        assert_eq!(
            debt.message,
            "Nested loop with 4 levels (Exponential complexity)"
        );
        assert_eq!(
            debt.context,
            Some(
                "Consider algorithm optimization or caching (parallelization possible)\nLocation confidence: Unavailable".to_string()
            )
        );
        assert_eq!(debt.line, 100);
        assert_eq!(debt.file, path);
    }

    #[test]
    fn test_convert_performance_pattern_inefficient_io() {
        let pattern = PerformanceAntiPattern::InefficientIO {
            io_pattern: IOPattern::UnbufferedIO,
            batching_opportunity: true,
            async_opportunity: false,
            location: SourceLocation {
                line: 50,
                column: None,
                end_line: None,
                end_column: None,
                confidence: LocationConfidence::Unavailable,
            },
        };
        let path = PathBuf::from("src/io.rs");
        let debt =
            convert_performance_pattern_to_debt_item(pattern, PerformanceImpact::Medium, &path);

        assert_eq!(debt.priority, Priority::High); // IO always gets High priority
        assert_eq!(debt.debt_type, DebtType::Performance);
        assert_eq!(debt.message, "Inefficient I/O pattern: UnbufferedIO");
        assert_eq!(
            debt.context,
            Some("Consider: batch operations\nLocation confidence: Unavailable".to_string())
        );
        assert_eq!(debt.line, 50);
    }

    #[test]
    fn test_convert_performance_pattern_uses_impact_for_other_patterns() {
        let pattern = PerformanceAntiPattern::ExcessiveAllocation {
            allocation_type: AllocationType::StringConcatenation,
            frequency: AllocationFrequency::InLoop,
            suggested_optimization: "Use String::with_capacity()".to_string(),
            location: SourceLocation {
                line: 75,
                column: None,
                end_line: None,
                end_column: None,
                confidence: LocationConfidence::Unavailable,
            },
        };
        let path = PathBuf::from("src/alloc.rs");
        let debt =
            convert_performance_pattern_to_debt_item(pattern, PerformanceImpact::Medium, &path);

        assert_eq!(debt.priority, Priority::Medium); // Uses impact_to_priority
        assert_eq!(debt.debt_type, DebtType::Performance);
        assert_eq!(debt.message, "StringConcatenation allocation InLoop");
        assert_eq!(
            debt.context,
            Some("Use String::with_capacity()\nLocation confidence: Unavailable".to_string())
        );
        assert_eq!(debt.line, 75);
    }
}
