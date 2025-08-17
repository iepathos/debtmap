use crate::performance::collected_data::*;
use crate::performance::{
    ComplexityClass, LoopOperation, PerformanceAntiPattern, PerformanceImpact,
};
use std::path::Path;

/// Trait for performance detectors that analyze pre-collected data
pub trait OptimizedPerformanceDetector {
    /// Analyze the collected data and return detected anti-patterns
    fn analyze_collected_data(
        &self,
        data: &CollectedPerformanceData,
        path: &Path,
    ) -> Vec<PerformanceAntiPattern>;

    /// Get the name of this detector
    fn detector_name(&self) -> &'static str;

    /// Estimate the impact of a detected pattern
    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact;
}

/// Adapter for the NestedLoopDetector to use collected data
pub struct OptimizedNestedLoopDetector;

impl OptimizedNestedLoopDetector {
    pub fn new() -> Self {
        Self
    }

    fn analyze_loop(
        &self,
        loop_info: &LoopInfo,
        data: &CollectedPerformanceData,
    ) -> Option<PerformanceAntiPattern> {
        // Only report nested loops (depth > 1)
        if loop_info.nesting_level < 2 {
            return None;
        }

        // Get operations within this loop
        let loop_ops = data.get_loop_operations(loop_info.id);

        // Classify inner operations
        let mut inner_operations = Vec::new();

        if !loop_ops.io_operations.is_empty() {
            inner_operations.push(LoopOperation::FileIO);
        }

        if loop_ops.data_structure_ops.iter().any(|op| {
            matches!(
                op.operation_type,
                DataStructureOpType::VecContains | DataStructureOpType::VecLinearSearch
            )
        }) {
            inner_operations.push(LoopOperation::CollectionIteration);
        }

        if !loop_ops.string_operations.is_empty() {
            inner_operations.push(LoopOperation::StringOperation);
        }

        if inner_operations.is_empty() {
            inner_operations.push(LoopOperation::Computation);
        }

        // Estimate complexity based on nesting level
        let complexity = match loop_info.nesting_level {
            2 => ComplexityClass::Quadratic,
            3 => ComplexityClass::Cubic,
            n if n > 3 => ComplexityClass::Exponential,
            _ => ComplexityClass::Unknown,
        };

        // Check if parallelizable (simple heuristic)
        let can_parallelize =
            !loop_ops.io_operations.iter().any(|op| !op.is_async) && !loop_info.has_early_exit;

        Some(PerformanceAntiPattern::NestedLoop {
            nesting_level: loop_info.nesting_level as u32,
            estimated_complexity: complexity,
            inner_operations,
            can_parallelize,
            location: loop_info.location.clone(),
        })
    }
}

impl OptimizedPerformanceDetector for OptimizedNestedLoopDetector {
    fn analyze_collected_data(
        &self,
        data: &CollectedPerformanceData,
        _path: &Path,
    ) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();

        // Analyze each loop
        for loop_info in &data.loops {
            if let Some(pattern) = self.analyze_loop(loop_info, data) {
                patterns.push(pattern);
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "OptimizedNestedLoopDetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::NestedLoop {
                estimated_complexity,
                ..
            } => match estimated_complexity {
                ComplexityClass::Exponential => PerformanceImpact::Critical,
                ComplexityClass::Cubic => PerformanceImpact::High,
                ComplexityClass::Quadratic => PerformanceImpact::Medium,
                _ => PerformanceImpact::Low,
            },
            _ => PerformanceImpact::Low,
        }
    }
}

/// Adapter for the IOPerformanceDetector to use collected data
pub struct OptimizedIODetector;

impl OptimizedIODetector {
    pub fn new() -> Self {
        Self
    }
}

impl OptimizedPerformanceDetector for OptimizedIODetector {
    fn analyze_collected_data(
        &self,
        data: &CollectedPerformanceData,
        _path: &Path,
    ) -> Vec<PerformanceAntiPattern> {
        use crate::performance::IOPattern;

        let mut patterns = Vec::new();

        for io_op in &data.io_operations {
            // Check if I/O is in a loop
            if io_op.context.loop_depth > 0 && !io_op.is_async {
                patterns.push(PerformanceAntiPattern::InefficientIO {
                    io_pattern: IOPattern::SyncInLoop,
                    batching_opportunity: true,
                    async_opportunity: true,
                    location: io_op.context.location.clone(),
                });
            } else if !io_op.is_buffered
                && matches!(io_op.operation_type, IOType::FileRead | IOType::FileWrite)
            {
                patterns.push(PerformanceAntiPattern::InefficientIO {
                    io_pattern: IOPattern::UnbufferedIO,
                    batching_opportunity: false,
                    async_opportunity: io_op.is_async,
                    location: io_op.context.location.clone(),
                });
            } else if matches!(io_op.operation_type, IOType::DatabaseQuery)
                && io_op.context.loop_depth > 0
            {
                patterns.push(PerformanceAntiPattern::InefficientIO {
                    io_pattern: IOPattern::UnbatchedQueries,
                    batching_opportunity: true,
                    async_opportunity: !io_op.is_async,
                    location: io_op.context.location.clone(),
                });
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "OptimizedIODetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::InefficientIO { io_pattern, .. } => {
                use crate::performance::IOPattern;
                match io_pattern {
                    IOPattern::SyncInLoop | IOPattern::UnbatchedQueries => PerformanceImpact::High,
                    IOPattern::UnbufferedIO => PerformanceImpact::Medium,
                    _ => PerformanceImpact::Low,
                }
            }
            _ => PerformanceImpact::Low,
        }
    }
}

/// Adapter for the AllocationDetector to use collected data
pub struct OptimizedAllocationDetector;

impl OptimizedAllocationDetector {
    pub fn new() -> Self {
        Self
    }
}

impl OptimizedPerformanceDetector for OptimizedAllocationDetector {
    fn analyze_collected_data(
        &self,
        data: &CollectedPerformanceData,
        _path: &Path,
    ) -> Vec<PerformanceAntiPattern> {
        use crate::performance::{AllocationFrequency, AllocationType as PatternAllocationType};

        let mut patterns = Vec::new();

        for alloc in &data.allocations {
            let frequency = if alloc.context.loop_depth > 0 {
                AllocationFrequency::InLoop
            } else if alloc.is_in_hot_path {
                AllocationFrequency::InHotPath
            } else {
                AllocationFrequency::Occasional
            };

            let alloc_type = match alloc.allocation_type {
                AllocationType::Clone => PatternAllocationType::Clone,
                AllocationType::StringConcat => PatternAllocationType::StringConcatenation,
                AllocationType::VecNew | AllocationType::Collect => {
                    PatternAllocationType::TemporaryCollection
                }
                AllocationType::BoxNew => PatternAllocationType::RepeatedBoxing,
                AllocationType::ToString | AllocationType::Format => {
                    PatternAllocationType::StringConcatenation
                }
            };

            let suggested_optimization =
                match (&alloc.allocation_type, alloc.context.loop_depth > 0) {
                    (AllocationType::Clone, true) => {
                        "Consider using references or Rc/Arc for shared ownership".to_string()
                    }
                    (AllocationType::StringConcat, true) => {
                        "Use String::with_capacity() and push_str()".to_string()
                    }
                    (AllocationType::VecNew, true) => {
                        "Pre-allocate with Vec::with_capacity()".to_string()
                    }
                    (AllocationType::Format, true) => {
                        "Use a String builder or write! macro".to_string()
                    }
                    _ => "Consider caching or pre-allocation".to_string(),
                };

            // Only report allocations in loops or hot paths
            if alloc.context.loop_depth > 0 || alloc.is_in_hot_path {
                patterns.push(PerformanceAntiPattern::ExcessiveAllocation {
                    allocation_type: alloc_type,
                    frequency,
                    suggested_optimization,
                    location: alloc.context.location.clone(),
                });
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "OptimizedAllocationDetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::ExcessiveAllocation { frequency, .. } => {
                use crate::performance::AllocationFrequency;
                match frequency {
                    AllocationFrequency::InLoop | AllocationFrequency::Recursive => {
                        PerformanceImpact::High
                    }
                    AllocationFrequency::InHotPath => PerformanceImpact::Medium,
                    AllocationFrequency::Occasional => PerformanceImpact::Low,
                }
            }
            _ => PerformanceImpact::Low,
        }
    }
}

/// Adapter for the StringPerformanceDetector to use collected data
pub struct OptimizedStringDetector;

impl OptimizedStringDetector {
    pub fn new() -> Self {
        Self
    }
}

impl OptimizedPerformanceDetector for OptimizedStringDetector {
    fn analyze_collected_data(
        &self,
        data: &CollectedPerformanceData,
        _path: &Path,
    ) -> Vec<PerformanceAntiPattern> {
        use crate::performance::StringAntiPattern;

        let mut patterns = Vec::new();

        for str_op in &data.string_operations {
            // Only report string operations in loops
            if str_op.context.loop_depth == 0 {
                continue;
            }

            let pattern_type = match str_op.operation_type {
                StringOperationType::Concatenation => StringAntiPattern::ConcatenationInLoop,
                StringOperationType::Format => StringAntiPattern::RepeatedFormatting,
                StringOperationType::Parse => StringAntiPattern::InefficientParsing,
                _ => continue,
            };

            let recommended_approach = match str_op.operation_type {
                StringOperationType::Concatenation => {
                    "Use String::with_capacity() and push_str()".to_string()
                }
                StringOperationType::Format => "Cache formatted strings or use write!".to_string(),
                StringOperationType::Parse => "Parse once and reuse the result".to_string(),
                _ => "Optimize string processing".to_string(),
            };

            let impact = if str_op.context.loop_depth > 1 {
                PerformanceImpact::High
            } else {
                PerformanceImpact::Medium
            };

            patterns.push(PerformanceAntiPattern::StringProcessingAntiPattern {
                pattern_type,
                performance_impact: impact,
                recommended_approach,
                location: str_op.context.location.clone(),
            });
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "OptimizedStringDetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::StringProcessingAntiPattern {
                performance_impact, ..
            } => *performance_impact,
            _ => PerformanceImpact::Low,
        }
    }
}

/// Adapter for the DataStructureDetector to use collected data
pub struct OptimizedDataStructureDetector;

impl OptimizedDataStructureDetector {
    pub fn new() -> Self {
        Self
    }
}

impl OptimizedPerformanceDetector for OptimizedDataStructureDetector {
    fn analyze_collected_data(
        &self,
        data: &CollectedPerformanceData,
        _path: &Path,
    ) -> Vec<PerformanceAntiPattern> {
        use crate::performance::DataStructureOperation;

        let mut patterns = Vec::new();

        for ds_op in &data.data_structure_ops {
            // Skip operations not in loops or hot paths
            if ds_op.context.loop_depth == 0 && !ds_op.is_in_hot_path {
                continue;
            }

            let (operation, recommended) = match ds_op.operation_type {
                DataStructureOpType::VecContains => {
                    (DataStructureOperation::Contains, "HashSet or BTreeSet")
                }
                DataStructureOpType::VecLinearSearch => {
                    (DataStructureOperation::LinearSearch, "HashMap or BTreeMap")
                }
                DataStructureOpType::VecInsert => (
                    DataStructureOperation::FrequentInsertion,
                    "VecDeque or LinkedList",
                ),
                DataStructureOpType::VecRemove => (
                    DataStructureOperation::FrequentDeletion,
                    "VecDeque or LinkedList",
                ),
                _ => continue,
            };

            let impact = if ds_op.context.loop_depth > 1 {
                PerformanceImpact::High
            } else if ds_op.is_in_hot_path {
                PerformanceImpact::Medium
            } else {
                PerformanceImpact::Low
            };

            patterns.push(PerformanceAntiPattern::InefficientDataStructure {
                operation,
                collection_type: ds_op.collection_type.clone(),
                recommended_alternative: recommended.to_string(),
                performance_impact: impact,
                location: ds_op.context.location.clone(),
            });
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "OptimizedDataStructureDetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::InefficientDataStructure {
                performance_impact, ..
            } => *performance_impact,
            _ => PerformanceImpact::Low,
        }
    }
}
