---
number: 47
title: Unified AST Traversal for Performance Detection
category: optimization
priority: critical
status: draft
dependencies: [42, 44]
created: 2024-01-17
---

# Specification 47: Unified AST Traversal for Performance Detection

**Category**: optimization  
**Priority**: critical  
**Status**: draft  
**Dependencies**: [42 (Smart Pattern Matching), 44 (Enhanced Scoring)]

## Context

The current performance detection system suffers from significant inefficiency due to redundant AST traversals. Each of the five performance detectors (NestedLoopDetector, IOPerformanceDetector, AllocationDetector, DataStructureDetector, StringPerformanceDetector) independently traverses the entire AST of each file, resulting in 5x the necessary computational overhead. Additionally, the smart performance detection and enhanced scoring systems add further traversals for context analysis.

Performance profiling shows this causes a 5-10x slowdown in analysis time:
- Old version (without smart detection): ~17 seconds
- Current version: Times out or takes excessive time
- Each detector creates its own visitor and calls `visit_file()`
- No sharing of context data between detectors
- Location extraction happens multiple times
- Function boundary detection repeated for each pattern

This architectural inefficiency makes the tool unusable for large codebases and negates the benefits of the improved accuracy from smart detection.

## Objective

Create a unified AST traversal system that collects all performance-relevant data in a single pass, eliminating redundant traversals while maintaining or improving detection accuracy. The system should support efficient data sharing between detectors and enable better pattern correlation through shared context.

## Requirements

### Functional Requirements

1. **Single-Pass Data Collection**
   - Traverse AST exactly once per file for all performance detectors
   - Collect all relevant nodes and context in unified data structures
   - Track hierarchical relationships (functions, loops, blocks)
   - Maintain source location information for all collected items

2. **Unified Data Model**
   - Define comprehensive data structures capturing all detector needs:
     - Loop information (nesting, type, operations)
     - I/O operations (calls, patterns, context)
     - Memory allocations (types, sizes, patterns)
     - String operations (concatenation, formatting, parsing)
     - Data structure operations (lookups, insertions, iterations)
   - Include contextual information (containing function, loop depth, etc.)

3. **Detector Interface Adaptation**
   - Modify detectors to analyze collected data instead of traversing AST
   - Maintain existing `PerformanceDetector` trait compatibility
   - Support incremental migration of detectors

4. **Context Sharing**
   - Share function boundaries across all detectors
   - Provide loop context to all nested operations
   - Enable cross-pattern analysis with full context

5. **Performance Metrics**
   - Track traversal time and memory usage
   - Compare with current multi-traversal approach
   - Provide profiling data for optimization

### Non-Functional Requirements

1. **Performance**
   - Achieve 60-80% reduction in AST traversal overhead
   - Maintain sub-linear memory growth with file size
   - Support streaming for very large files

2. **Maintainability**
   - Clear separation between data collection and analysis
   - Well-documented data structures and interfaces
   - Extensible for new detector types

3. **Compatibility**
   - Preserve existing detector accuracy
   - Maintain current API for performance detection
   - Support gradual migration path

## Acceptance Criteria

- [ ] Single AST traversal collects all performance-relevant data
- [ ] All five existing detectors work with collected data
- [ ] Performance analysis time reduced by at least 60%
- [ ] Memory overhead less than 20% increase
- [ ] All existing tests pass without modification
- [ ] Smart detection and enhanced scoring use unified data
- [ ] Pattern correlation improved through shared context
- [ ] Location extraction happens exactly once per file
- [ ] Function boundaries calculated once and reused
- [ ] Documentation updated with new architecture

## Technical Details

### Implementation Approach

1. **Create Unified Visitor**
```rust
pub struct UnifiedPerformanceVisitor<'a> {
    // Core tracking
    source_content: &'a str,
    location_extractor: LocationExtractor,
    
    // Function context
    current_function: Option<FunctionContext>,
    functions: Vec<FunctionInfo>,
    
    // Loop tracking
    loop_stack: Vec<LoopContext>,
    loop_info: Vec<LoopInfo>,
    
    // Collected data
    io_operations: Vec<IOOperation>,
    allocations: Vec<AllocationInfo>,
    string_operations: Vec<StringOperation>,
    data_structure_ops: Vec<DataStructureOp>,
    
    // Shared context
    call_graph: Vec<CallSite>,
    block_depth: usize,
}
```

2. **Define Collected Data Types**
```rust
pub struct CollectedPerformanceData {
    pub functions: Vec<FunctionInfo>,
    pub loops: Vec<LoopInfo>,
    pub io_operations: Vec<IOOperation>,
    pub allocations: Vec<AllocationInfo>,
    pub string_operations: Vec<StringOperation>,
    pub data_structure_ops: Vec<DataStructureOp>,
    pub call_sites: Vec<CallSite>,
}

pub struct ContextualOperation {
    pub location: SourceLocation,
    pub containing_function: Option<FunctionId>,
    pub loop_depth: usize,
    pub in_conditional: bool,
    pub in_error_handler: bool,
}
```

3. **Adapt Detector Interface**
```rust
pub trait OptimizedPerformanceDetector {
    fn analyze_collected_data(
        &self,
        data: &CollectedPerformanceData,
        path: &Path,
    ) -> Vec<PerformanceAntiPattern>;
}
```

### Architecture Changes

1. **New Module Structure**
   - `performance/unified_visitor.rs` - Single-pass AST visitor
   - `performance/collected_data.rs` - Data structures for collected information
   - `performance/detector_adapter.rs` - Adapters for existing detectors

2. **Modified Detection Flow**
   - Phase 1: Unified AST traversal and data collection
   - Phase 2: Parallel pattern detection on collected data
   - Phase 3: Context analysis using pre-collected function data
   - Phase 4: Pattern correlation with full context available

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub id: FunctionId,
    pub name: String,
    pub location: SourceLocation,
    pub span: (usize, usize), // line range
    pub complexity: ComplexityMetrics,
    pub is_test: bool,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct LoopInfo {
    pub id: LoopId,
    pub loop_type: LoopType,
    pub location: SourceLocation,
    pub nesting_level: usize,
    pub containing_function: Option<FunctionId>,
    pub operations: Vec<OperationId>,
    pub estimated_iterations: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct IOOperation {
    pub id: OperationId,
    pub operation_type: IOType,
    pub location: SourceLocation,
    pub context: ContextualOperation,
    pub is_async: bool,
    pub in_loop: bool,
}
```

### APIs and Interfaces

```rust
pub fn analyze_performance_patterns_optimized(
    file: &syn::File,
    path: &Path,
) -> Vec<DebtItem> {
    // Phase 1: Single-pass data collection
    let collected_data = {
        let source_content = std::fs::read_to_string(path)?;
        let mut visitor = UnifiedPerformanceVisitor::new(&source_content);
        visitor.visit_file(file);
        visitor.into_collected_data()
    };
    
    // Phase 2: Parallel pattern detection
    let patterns = DETECTORS
        .par_iter()
        .flat_map(|detector| {
            detector.analyze_collected_data(&collected_data, path)
        })
        .collect();
    
    // Phase 3: Smart analysis (if enabled)
    let analyzed_patterns = if config.smart_detection_enabled {
        apply_smart_analysis(&patterns, &collected_data)
    } else {
        patterns
    };
    
    // Phase 4: Convert to debt items
    convert_patterns_to_debt_items(analyzed_patterns, path)
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 42: Smart Pattern Matching (provides context analysis framework)
  - Spec 44: Enhanced Scoring (uses function context data)
- **Affected Components**:
  - All performance detectors in `src/performance/`
  - Smart detector in `src/performance/smart_detector.rs`
  - Performance analysis in `src/analyzers/rust.rs`
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: 
  - Test unified visitor collects all expected data
  - Verify each detector produces same results with collected data
  - Test memory usage stays within bounds
  
- **Integration Tests**:
  - Compare detection results with current implementation
  - Verify performance improvements on sample codebases
  - Test with large files to ensure scalability
  
- **Performance Tests**:
  - Benchmark AST traversal time reduction
  - Measure memory overhead
  - Profile with various file sizes and complexities
  
- **Regression Tests**:
  - Ensure all existing patterns still detected
  - Verify no false negatives introduced
  - Check location accuracy maintained

## Documentation Requirements

- **Code Documentation**:
  - Document unified visitor architecture
  - Explain data collection strategy
  - Provide migration guide for new detectors
  
- **Architecture Updates**:
  - Update ARCHITECTURE.md with new traversal approach
  - Document performance optimization strategy
  - Add sequence diagrams for new flow
  
- **User Documentation**:
  - Note performance improvements in README
  - Update troubleshooting guide if needed

## Implementation Notes

### Optimization Opportunities

1. **Lazy Evaluation**
   - Only collect data needed by enabled detectors
   - Use iterators for memory-efficient processing
   - Defer expensive computations until needed

2. **Incremental Processing**
   - Cache collected data for unchanged files
   - Support partial re-analysis
   - Use file hashes for cache invalidation

3. **Parallel Analysis**
   - Run detectors in parallel after collection
   - Use rayon for work-stealing parallelism
   - Share immutable collected data safely

### Migration Strategy

1. **Phase 1**: Implement unified visitor alongside existing system
2. **Phase 2**: Migrate one detector at a time to use collected data
3. **Phase 3**: Run both systems in parallel for validation
4. **Phase 4**: Remove old traversal code after validation
5. **Phase 5**: Optimize data structures based on profiling

### Potential Challenges

1. **Memory Usage**: Storing all collected data may increase memory
   - Mitigation: Use compact representations, intern strings
   
2. **Data Structure Design**: Balancing completeness with efficiency
   - Mitigation: Profile actual usage patterns, optimize common cases
   
3. **Backward Compatibility**: Maintaining existing behavior
   - Mitigation: Comprehensive testing, gradual migration

## Migration and Compatibility

### Breaking Changes
- Internal API changes for performance detectors
- No external API changes

### Migration Path
1. Existing detectors continue working during migration
2. New unified system runs in parallel for validation
3. Feature flag to enable/disable optimization
4. Gradual deprecation of old traversal methods

### Compatibility Considerations
- Maintain exact same detection patterns
- Preserve all existing command-line options
- Keep same output format and debt items