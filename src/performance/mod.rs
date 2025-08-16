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
    },
    InefficientDataStructure {
        operation: DataStructureOperation,
        collection_type: String,
        recommended_alternative: String,
        performance_impact: PerformanceImpact,
    },
    ExcessiveAllocation {
        allocation_type: AllocationType,
        frequency: AllocationFrequency,
        suggested_optimization: String,
    },
    InefficientIO {
        io_pattern: IOPattern,
        batching_opportunity: bool,
        async_opportunity: bool,
    },
    StringProcessingAntiPattern {
        pattern_type: StringAntiPattern,
        performance_impact: PerformanceImpact,
        recommended_approach: String,
    },
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
pub mod nested_loop_detector;
pub mod string_detector;

pub use allocation_detector::AllocationDetector;
pub use data_structure_detector::DataStructureDetector;
pub use io_detector::IOPerformanceDetector;
pub use nested_loop_detector::NestedLoopDetector;
pub use string_detector::StringPerformanceDetector;

pub fn convert_performance_pattern_to_debt_item(
    pattern: PerformanceAntiPattern,
    impact: PerformanceImpact,
    path: &Path,
    line: usize,
) -> DebtItem {
    let (priority, message, recommendation) = match pattern {
        PerformanceAntiPattern::NestedLoop {
            nesting_level,
            estimated_complexity,
            can_parallelize,
            ..
        } => {
            let priority = match estimated_complexity {
                ComplexityClass::Exponential => Priority::Critical,
                ComplexityClass::Cubic => Priority::High,
                ComplexityClass::Quadratic => Priority::Medium,
                _ => Priority::Low,
            };
            let mut rec = "Consider algorithm optimization or caching".to_string();
            if can_parallelize {
                rec.push_str(" (parallelization possible)");
            }
            (
                priority,
                format!(
                    "Nested loop with {} levels ({:?} complexity)",
                    nesting_level, estimated_complexity
                ),
                rec,
            )
        }
        PerformanceAntiPattern::InefficientDataStructure {
            operation,
            collection_type,
            recommended_alternative,
            ..
        } => (
            impact_to_priority(impact),
            format!(
                "{:?} operation on {} in performance-critical code",
                operation, collection_type
            ),
            format!(
                "Consider using {} for better performance",
                recommended_alternative
            ),
        ),
        PerformanceAntiPattern::ExcessiveAllocation {
            allocation_type,
            frequency,
            suggested_optimization,
        } => (
            impact_to_priority(impact),
            format!("{:?} allocation {:?}", allocation_type, frequency),
            suggested_optimization,
        ),
        PerformanceAntiPattern::InefficientIO {
            io_pattern,
            batching_opportunity,
            async_opportunity,
        } => {
            let mut recommendations = Vec::new();
            if batching_opportunity {
                recommendations.push("batch operations");
            }
            if async_opportunity {
                recommendations.push("use async I/O");
            }

            (
                Priority::High,
                format!("Inefficient I/O pattern: {:?}", io_pattern),
                format!("Consider: {}", recommendations.join(", ")),
            )
        }
        PerformanceAntiPattern::StringProcessingAntiPattern {
            pattern_type,
            recommended_approach,
            ..
        } => (
            impact_to_priority(impact),
            format!("Inefficient string processing: {:?}", pattern_type),
            recommended_approach,
        ),
    };

    DebtItem {
        id: format!("performance-{}-{}", path.display(), line),
        debt_type: DebtType::Performance,
        priority,
        file: path.to_path_buf(),
        line,
        column: None,
        message,
        context: Some(recommendation),
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
