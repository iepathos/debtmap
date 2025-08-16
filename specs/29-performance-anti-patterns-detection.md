---
number: 29
title: Performance Anti-Patterns Detection
category: feature
priority: high
status: draft
dependencies: []
created: 2025-08-16
---

# Specification 29: Performance Anti-Patterns Detection

**Category**: feature
**Priority**: high
**Status**: draft
**Dependencies**: []

## Context

Performance issues often stem from common anti-patterns that can be detected through static analysis. While the current debtmap system identifies complexity, it lacks specific detection for performance-related technical debt patterns that can significantly impact application performance:

- **O(n²) Nested Loops** - Quadratic complexity from nested iteration patterns
- **Inefficient Data Structures** - Using Vec::contains() in loops, linear searches on large datasets
- **String Concatenation in Loops** - Repeated string allocation instead of using StringBuilder patterns
- **Redundant Allocations** - Unnecessary cloning, frequent temporary allocations
- **Inefficient I/O Patterns** - Synchronous I/O in loops, missing batching opportunities

These patterns represent high-impact technical debt as they directly affect application performance and user experience.

## Objective

Implement performance-focused pattern detection that identifies algorithmic and structural performance issues not caught by existing language tooling:

1. **Algorithmic Complexity Analysis**: Detect O(n²) and higher complexity patterns (not detected by any existing tools)
2. **Data Structure Misuse**: Identify algorithmic inefficiencies beyond basic clippy checks
3. **Language-Specific Performance Patterns**:
   - **Rust**: Skip patterns caught by clippy (`needless_collect`, `inefficient_to_string`, `string_add_assign`)
   - **Python**: Detect all patterns including list comprehension abuse, repeated sorting, global access in loops
   - **JavaScript/TypeScript**: Detect DOM manipulation in loops, synchronous operations in async contexts

## Requirements

### Functional Requirements

1. **Language-Aware Performance Detection**
   - For Rust: Focus on algorithmic complexity not caught by clippy
   - For Python/JS/TS: Detect all performance anti-patterns
   - Maintain language-specific performance pattern databases

2. **Algorithmic Complexity Detection**
   - Identify nested loops with O(n²) or higher complexity
   - Detect collection operations within loops that increase complexity
   - Calculate estimated time complexity for nested operations
   - Flag algorithms that could use better data structures (e.g., Vec search → HashSet)

3. **Data Structure Efficiency Detection** (Focus on patterns not caught by clippy)
   - Vec::contains() usage in loops (O(n) per iteration)
   - Linear search patterns on large collections
   - Inappropriate data structure choices (Vec vs. HashSet/HashMap)
   - Missing indexing opportunities

3. **Memory Allocation Analysis**
   - Excessive cloning in loops or hot paths
   - Temporary collection allocations in iterations
   - String concatenation patterns causing repeated allocation
   - Large stack allocations that should be heap-allocated

4. **I/O Performance Detection**
   - Synchronous I/O operations in loops
   - Missing batch processing opportunities
   - File I/O without buffering
   - Database queries in loops (N+1 problem)

5. **String Processing Anti-Patterns**
   - String concatenation in loops using + operator
   - Format string usage in hot paths
   - Regular expression compilation in loops
   - Inefficient string parsing patterns

### Non-Functional Requirements

1. **Performance**
   - Performance analysis adds <20% overhead to total analysis time
   - Efficient complexity analysis using AST traversal
   - Scalable to large codebases with incremental analysis

2. **Accuracy**
   - >85% precision for performance anti-pattern detection
   - >75% recall for significant performance issues
   - Configurable complexity thresholds

3. **Actionability**
   - Provide specific recommendations for each anti-pattern
   - Estimate performance impact and improvement potential
   - Suggest alternative implementations

## Acceptance Criteria

- [ ] **Nested Loop Detection**: All O(n²) and higher patterns identified with complexity estimates
- [ ] **Data Structure Anti-Patterns**: Inefficient collection usage flagged with alternatives
- [ ] **Allocation Analysis**: Excessive allocation patterns detected and quantified
- [ ] **I/O Pattern Detection**: Inefficient I/O usage identified with batch processing suggestions
- [ ] **String Anti-Patterns**: Inefficient string operations flagged with efficient alternatives
- [ ] **Performance Impact**: Each issue includes estimated performance impact
- [ ] **Actionable Recommendations**: Specific improvement suggestions for each pattern
- [ ] **Complexity Estimation**: Time and space complexity analysis for detected patterns

## Technical Details

### Implementation Approach

#### 1. Performance Analysis Framework (`src/performance/`)

```rust
/// Performance anti-pattern detection framework
pub mod performance {
    use crate::core::ast::AstNode;
    use crate::core::{DebtItem, Priority};
    
    #[derive(Debug, Clone, PartialEq)]
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
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ComplexityClass {
        Linear,        // O(n)
        Quadratic,     // O(n²)
        Cubic,         // O(n³)
        Exponential,   // O(2^n)
        Unknown,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum LoopOperation {
        CollectionIteration,
        DatabaseQuery,
        FileIO,
        NetworkRequest,
        Computation,
        StringOperation,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum DataStructureOperation {
        Contains,
        LinearSearch,
        FrequentInsertion,
        FrequentDeletion,
        RandomAccess,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum AllocationType {
        Clone,
        StringConcatenation,
        TemporaryCollection,
        LargeStackAllocation,
        RepeatedBoxing,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum AllocationFrequency {
        InLoop,
        InHotPath,
        Recursive,
        Occasional,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum IOPattern {
        SyncInLoop,
        UnbatchedQueries,
        UnbufferedIO,
        ExcessiveConnections,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum StringAntiPattern {
        ConcatenationInLoop,
        RepeatedFormatting,
        RegexInLoop,
        InefficientParsing,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum PerformanceImpact {
        Critical,  // 10x+ performance impact
        High,      // 3-10x performance impact
        Medium,    // 1.5-3x performance impact
        Low,       // <1.5x performance impact
    }
    
    pub trait PerformanceDetector {
        fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern>;
        fn detector_name(&self) -> &'static str;
        fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact;
    }
}
```

#### 2. Nested Loop Detector (`src/performance/nested_loop_detector.rs`)

```rust
pub struct NestedLoopDetector {
    max_acceptable_nesting: u32,
    complexity_analysis: ComplexityAnalyzer,
}

impl PerformanceDetector for NestedLoopDetector {
    fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        let nested_loops = self.find_nested_loops(ast);
        
        for nested_loop in nested_loops {
            let nesting_level = self.calculate_nesting_level(&nested_loop);
            
            if nesting_level > self.max_acceptable_nesting {
                let inner_operations = self.analyze_inner_operations(&nested_loop);
                let estimated_complexity = self.estimate_complexity(&nested_loop, &inner_operations);
                let can_parallelize = self.analyze_parallelization_potential(&nested_loop);
                
                patterns.push(PerformanceAntiPattern::NestedLoop {
                    nesting_level,
                    estimated_complexity,
                    inner_operations,
                    can_parallelize,
                });
            }
        }
        
        patterns
    }
}

impl NestedLoopDetector {
    fn find_nested_loops(&self, ast: &AstNode) -> Vec<NestedLoopStructure> {
        let mut nested_loops = Vec::new();
        
        ast.traverse_depth_first(|node| {
            if let AstNode::Loop(loop_node) = node {
                let inner_loops = self.find_loops_within(loop_node);
                if !inner_loops.is_empty() {
                    nested_loops.push(NestedLoopStructure {
                        outer_loop: loop_node.clone(),
                        inner_loops,
                    });
                }
            }
        });
        
        nested_loops
    }
    
    fn analyze_inner_operations(&self, nested_loop: &NestedLoopStructure) -> Vec<LoopOperation> {
        let mut operations = Vec::new();
        
        for inner_loop in &nested_loop.inner_loops {
            let loop_operations = self.identify_operations_in_loop(inner_loop);
            operations.extend(loop_operations);
        }
        
        operations
    }
    
    fn estimate_complexity(
        &self, 
        nested_loop: &NestedLoopStructure, 
        operations: &[LoopOperation]
    ) -> ComplexityClass {
        let nesting_level = nested_loop.inner_loops.len() + 1;
        
        // Base complexity from nesting
        let base_complexity = match nesting_level {
            1 => ComplexityClass::Linear,
            2 => ComplexityClass::Quadratic,
            3 => ComplexityClass::Cubic,
            _ => ComplexityClass::Exponential,
        };
        
        // Adjust based on inner operations
        let has_expensive_operations = operations.iter().any(|op| {
            matches!(op, LoopOperation::DatabaseQuery | LoopOperation::NetworkRequest)
        });
        
        if has_expensive_operations {
            // Each expensive operation effectively adds another level
            match base_complexity {
                ComplexityClass::Linear => ComplexityClass::Quadratic,
                ComplexityClass::Quadratic => ComplexityClass::Cubic,
                _ => ComplexityClass::Exponential,
            }
        } else {
            base_complexity
        }
    }
    
    fn analyze_parallelization_potential(&self, nested_loop: &NestedLoopStructure) -> bool {
        // Check for data dependencies between iterations
        let has_dependencies = self.has_data_dependencies(&nested_loop.outer_loop);
        let has_shared_mutable_state = self.has_shared_mutable_state(&nested_loop.outer_loop);
        let has_io_operations = self.has_io_operations(&nested_loop.outer_loop);
        
        !has_dependencies && !has_shared_mutable_state && !has_io_operations
    }
    
    fn identify_operations_in_loop(&self, loop_node: &LoopNode) -> Vec<LoopOperation> {
        let mut operations = Vec::new();
        
        loop_node.body.traverse(|node| {
            match node {
                AstNode::MethodCall(call) if self.is_collection_operation(call) => {
                    operations.push(LoopOperation::CollectionIteration);
                }
                AstNode::FunctionCall(call) if self.is_database_operation(call) => {
                    operations.push(LoopOperation::DatabaseQuery);
                }
                AstNode::FunctionCall(call) if self.is_file_operation(call) => {
                    operations.push(LoopOperation::FileIO);
                }
                AstNode::FunctionCall(call) if self.is_network_operation(call) => {
                    operations.push(LoopOperation::NetworkRequest);
                }
                AstNode::StringConcatenation(_) => {
                    operations.push(LoopOperation::StringOperation);
                }
                _ => {}
            }
        });
        
        operations
    }
}
```

#### 3. Inefficient Data Structure Detector (`src/performance/data_structure_detector.rs`)

```rust
pub struct DataStructureDetector {
    collection_patterns: HashMap<String, DataStructurePattern>,
}

impl PerformanceDetector for DataStructureDetector {
    fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        
        // Detect Vec::contains() in loops
        let contains_in_loops = self.find_contains_in_loops(ast);
        patterns.extend(contains_in_loops);
        
        // Detect linear search patterns
        let linear_searches = self.find_linear_search_patterns(ast);
        patterns.extend(linear_searches);
        
        // Detect frequent insertion/deletion on Vec
        let inefficient_modifications = self.find_inefficient_modifications(ast);
        patterns.extend(inefficient_modifications);
        
        patterns
    }
}

impl DataStructureDetector {
    fn find_contains_in_loops(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        
        ast.traverse_depth_first(|node| {
            if let AstNode::Loop(loop_node) = node {
                let contains_calls = self.find_contains_calls_in_loop(loop_node);
                
                for contains_call in contains_calls {
                    let collection_type = self.infer_collection_type(&contains_call.receiver);
                    
                    if collection_type == "Vec" {
                        patterns.push(PerformanceAntiPattern::InefficientDataStructure {
                            operation: DataStructureOperation::Contains,
                            collection_type: collection_type.clone(),
                            recommended_alternative: "HashSet or HashMap".to_string(),
                            performance_impact: PerformanceImpact::High,
                        });
                    }
                }
            }
        });
        
        patterns
    }
    
    fn find_linear_search_patterns(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        
        // Look for .iter().find() patterns in loops
        ast.traverse_depth_first(|node| {
            if let AstNode::Loop(loop_node) = node {
                let search_patterns = self.find_search_patterns_in_loop(loop_node);
                
                for pattern in search_patterns {
                    if self.is_linear_search(&pattern) {
                        patterns.push(PerformanceAntiPattern::InefficientDataStructure {
                            operation: DataStructureOperation::LinearSearch,
                            collection_type: pattern.collection_type,
                            recommended_alternative: self.suggest_indexed_alternative(&pattern),
                            performance_impact: self.estimate_search_impact(&pattern),
                        });
                    }
                }
            }
        });
        
        patterns
    }
    
    fn find_inefficient_modifications(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        
        // Detect frequent Vec::insert(0, _) or Vec::remove(0) patterns
        ast.traverse_depth_first(|node| {
            if let AstNode::Loop(loop_node) = node {
                let modifications = self.find_collection_modifications(loop_node);
                
                for modification in modifications {
                    if self.is_inefficient_modification(&modification) {
                        patterns.push(PerformanceAntiPattern::InefficientDataStructure {
                            operation: self.classify_modification(&modification),
                            collection_type: modification.collection_type,
                            recommended_alternative: self.suggest_modification_alternative(&modification),
                            performance_impact: PerformanceImpact::Medium,
                        });
                    }
                }
            }
        });
        
        patterns
    }
    
    fn suggest_indexed_alternative(&self, pattern: &SearchPattern) -> String {
        match pattern.search_criteria {
            SearchCriteria::ByKey => "HashMap for key-based lookups".to_string(),
            SearchCriteria::ByValue => "HashSet for membership testing".to_string(),
            SearchCriteria::ByPredicate => "BTreeMap or indexed structure".to_string(),
        }
    }
    
    fn estimate_search_impact(&self, pattern: &SearchPattern) -> PerformanceImpact {
        match pattern.estimated_collection_size {
            0..=10 => PerformanceImpact::Low,
            11..=100 => PerformanceImpact::Medium,
            101..=1000 => PerformanceImpact::High,
            _ => PerformanceImpact::Critical,
        }
    }
}
```

#### 4. Memory Allocation Detector (`src/performance/allocation_detector.rs`)

```rust
pub struct AllocationDetector {
    clone_tracking: CloneTracker,
    string_analysis: StringAllocationAnalyzer,
}

impl PerformanceDetector for AllocationDetector {
    fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        
        // Detect excessive cloning
        let clone_patterns = self.find_excessive_cloning(ast);
        patterns.extend(clone_patterns);
        
        // Detect string concatenation in loops
        let string_patterns = self.find_string_allocation_patterns(ast);
        patterns.extend(string_patterns);
        
        // Detect temporary collection allocations
        let temp_allocations = self.find_temporary_allocations(ast);
        patterns.extend(temp_allocations);
        
        patterns
    }
}

impl AllocationDetector {
    fn find_excessive_cloning(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        let clone_sites = self.clone_tracking.find_all_clones(ast);
        
        for clone_site in clone_sites {
            let frequency = self.analyze_clone_frequency(&clone_site);
            
            if self.is_excessive_cloning(&clone_site, &frequency) {
                patterns.push(PerformanceAntiPattern::ExcessiveAllocation {
                    allocation_type: AllocationType::Clone,
                    frequency,
                    suggested_optimization: self.suggest_clone_optimization(&clone_site),
                });
            }
        }
        
        patterns
    }
    
    fn find_string_allocation_patterns(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        
        // Find string concatenation in loops
        ast.traverse_depth_first(|node| {
            if let AstNode::Loop(loop_node) = node {
                let string_ops = self.string_analysis.find_string_operations(loop_node);
                
                for op in string_ops {
                    if op.causes_allocation && op.frequency == AllocationFrequency::InLoop {
                        patterns.push(PerformanceAntiPattern::ExcessiveAllocation {
                            allocation_type: AllocationType::StringConcatenation,
                            frequency: AllocationFrequency::InLoop,
                            suggested_optimization: "Use String::with_capacity() or format! macro".to_string(),
                        });
                    }
                }
            }
        });
        
        patterns
    }
    
    fn suggest_clone_optimization(&self, clone_site: &CloneSite) -> String {
        match &clone_site.context {
            CloneContext::ReturnValue => "Consider returning a reference or using Cow<>".to_string(),
            CloneContext::FunctionArgument => "Consider taking a reference parameter".to_string(),
            CloneContext::InLoop => "Move clone outside loop or use references".to_string(),
            CloneContext::Temporary => "Consider borrowing instead of cloning".to_string(),
        }
    }
    
    fn is_excessive_cloning(&self, clone_site: &CloneSite, frequency: &AllocationFrequency) -> bool {
        match frequency {
            AllocationFrequency::InLoop => true,
            AllocationFrequency::InHotPath => clone_site.data_size > 1024, // Large data structures
            AllocationFrequency::Recursive => true,
            AllocationFrequency::Occasional => false,
        }
    }
}
```

#### 5. I/O Performance Detector (`src/performance/io_detector.rs`)

```rust
pub struct IOPerformanceDetector {
    io_patterns: IOPatternAnalyzer,
}

impl PerformanceDetector for IOPerformanceDetector {
    fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        
        // Detect synchronous I/O in loops
        let sync_io_patterns = self.find_sync_io_in_loops(ast);
        patterns.extend(sync_io_patterns);
        
        // Detect unbatched database queries
        let unbatched_queries = self.find_unbatched_queries(ast);
        patterns.extend(unbatched_queries);
        
        // Detect unbuffered I/O operations
        let unbuffered_io = self.find_unbuffered_io(ast);
        patterns.extend(unbuffered_io);
        
        patterns
    }
}

impl IOPerformanceDetector {
    fn find_sync_io_in_loops(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        
        ast.traverse_depth_first(|node| {
            if let AstNode::Loop(loop_node) = node {
                let io_operations = self.io_patterns.find_io_operations(loop_node);
                
                for io_op in io_operations {
                    if io_op.is_synchronous && !io_op.is_batched {
                        patterns.push(PerformanceAntiPattern::InefficientIO {
                            io_pattern: IOPattern::SyncInLoop,
                            batching_opportunity: self.can_batch_operation(&io_op),
                            async_opportunity: self.can_make_async(&io_op),
                        });
                    }
                }
            }
        });
        
        patterns
    }
    
    fn find_unbatched_queries(&self, ast: &AstNode) -> Vec<PerformanceAntiPattern> {
        let mut patterns = Vec::new();
        let query_sites = self.io_patterns.find_database_queries(ast);
        
        // Group queries by loop context
        let queries_in_loops = query_sites.into_iter()
            .filter(|query| query.is_in_loop)
            .collect::<Vec<_>>();
            
        for query in queries_in_loops {
            if !query.is_batched && self.can_batch_query(&query) {
                patterns.push(PerformanceAntiPattern::InefficientIO {
                    io_pattern: IOPattern::UnbatchedQueries,
                    batching_opportunity: true,
                    async_opportunity: query.supports_async,
                });
            }
        }
        
        patterns
    }
    
    fn can_batch_operation(&self, io_op: &IOOperation) -> bool {
        match io_op.operation_type {
            IOOperationType::DatabaseQuery => true,
            IOOperationType::FileRead => io_op.file_size < 1024 * 1024, // Small files can be batched
            IOOperationType::NetworkRequest => true,
            IOOperationType::FileWrite => true,
        }
    }
    
    fn can_make_async(&self, io_op: &IOOperation) -> bool {
        // Most I/O operations can be made async, but some have constraints
        match io_op.operation_type {
            IOOperationType::DatabaseQuery => true,
            IOOperationType::NetworkRequest => true,
            IOOperationType::FileRead | IOOperationType::FileWrite => {
                !io_op.requires_immediate_consistency
            }
        }
    }
}
```

#### 6. Integration with Main Analysis Pipeline

```rust
// In src/analyzers/rust.rs
use crate::performance::{
    PerformanceDetector, NestedLoopDetector, DataStructureDetector, 
    AllocationDetector, IOPerformanceDetector, StringPerformanceDetector
};

fn analyze_performance_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    let detectors: Vec<Box<dyn PerformanceDetector>> = vec![
        Box::new(NestedLoopDetector::new()),
        Box::new(DataStructureDetector::new()),
        Box::new(AllocationDetector::new()),
        Box::new(IOPerformanceDetector::new()),
        Box::new(StringPerformanceDetector::new()),
    ];
    
    let ast_node = convert_syn_to_ast_node(file);
    let mut performance_items = Vec::new();
    
    for detector in detectors {
        let anti_patterns = detector.detect_anti_patterns(&ast_node);
        
        for pattern in anti_patterns {
            let impact = detector.estimate_impact(&pattern);
            let debt_item = convert_performance_pattern_to_debt_item(pattern, impact, path);
            performance_items.push(debt_item);
        }
    }
    
    performance_items
}

fn convert_performance_pattern_to_debt_item(
    pattern: PerformanceAntiPattern,
    impact: PerformanceImpact,
    path: &Path
) -> DebtItem {
    let (priority, message, recommendation) = match pattern {
        PerformanceAntiPattern::NestedLoop { nesting_level, estimated_complexity, .. } => {
            let priority = match estimated_complexity {
                ComplexityClass::Exponential => Priority::Critical,
                ComplexityClass::Cubic => Priority::High,
                ComplexityClass::Quadratic => Priority::Medium,
                _ => Priority::Low,
            };
            (
                priority,
                format!("Nested loop with {} levels ({:?} complexity)", nesting_level, estimated_complexity),
                "Consider algorithm optimization, caching, or parallelization".to_string()
            )
        }
        PerformanceAntiPattern::InefficientDataStructure { operation, collection_type, recommended_alternative, .. } => {
            (
                impact_to_priority(impact),
                format!("{:?} operation on {} in performance-critical code", operation, collection_type),
                format!("Consider using {} for better performance", recommended_alternative)
            )
        }
        PerformanceAntiPattern::ExcessiveAllocation { allocation_type, frequency, suggested_optimization } => {
            (
                impact_to_priority(impact),
                format!("{:?} allocation {:?}", allocation_type, frequency),
                suggested_optimization
            )
        }
        PerformanceAntiPattern::InefficientIO { io_pattern, batching_opportunity, async_opportunity } => {
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
                format!("Consider: {}", recommendations.join(", "))
            )
        }
        PerformanceAntiPattern::StringProcessingAntiPattern { pattern_type, recommended_approach, .. } => {
            (
                impact_to_priority(impact),
                format!("Inefficient string processing: {:?}", pattern_type),
                recommended_approach
            )
        }
    };
    
    DebtItem {
        id: format!("performance-{}-{}", path.display(), get_line_from_pattern(&pattern)),
        debt_type: DebtType::Performance, // New debt type
        priority,
        file: path.to_path_buf(),
        line: get_line_from_pattern(&pattern),
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
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nested_loop_detection() {
        let source = r#"
            fn inefficient_search(items: &[Vec<i32>], target: i32) -> bool {
                for outer_vec in items {
                    for &item in outer_vec {
                        if item == target {
                            return true;
                        }
                    }
                }
                false
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = NestedLoopDetector::new();
        let patterns = detector.detect_anti_patterns(&ast);
        
        assert_eq!(patterns.len(), 1);
        if let PerformanceAntiPattern::NestedLoop { nesting_level, estimated_complexity, .. } = &patterns[0] {
            assert_eq!(*nesting_level, 2);
            assert_eq!(*estimated_complexity, ComplexityClass::Quadratic);
        } else {
            panic!("Expected nested loop pattern");
        }
    }
    
    #[test]
    fn test_vec_contains_in_loop() {
        let source = r#"
            fn filter_items(all_items: &[String], allowed: &[String]) -> Vec<String> {
                let mut result = Vec::new();
                for item in all_items {
                    if allowed.contains(item) {
                        result.push(item.clone());
                    }
                }
                result
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = DataStructureDetector::new();
        let patterns = detector.detect_anti_patterns(&ast);
        
        assert!(!patterns.is_empty());
        let contains_pattern = patterns.iter().find(|p| {
            matches!(p, PerformanceAntiPattern::InefficientDataStructure { 
                operation: DataStructureOperation::Contains, 
                .. 
            })
        });
        assert!(contains_pattern.is_some());
    }
    
    #[test]
    fn test_string_concatenation_in_loop() {
        let source = r#"
            fn build_string(items: &[&str]) -> String {
                let mut result = String::new();
                for item in items {
                    result = result + item + ",";
                }
                result
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = AllocationDetector::new();
        let patterns = detector.detect_anti_patterns(&ast);
        
        assert!(!patterns.is_empty());
        let string_pattern = patterns.iter().find(|p| {
            matches!(p, PerformanceAntiPattern::ExcessiveAllocation { 
                allocation_type: AllocationType::StringConcatenation, 
                .. 
            })
        });
        assert!(string_pattern.is_some());
    }
    
    #[test]
    fn test_io_in_loop_detection() {
        let source = r#"
            fn process_files(filenames: &[String]) -> Vec<String> {
                let mut contents = Vec::new();
                for filename in filenames {
                    let content = std::fs::read_to_string(filename).unwrap();
                    contents.push(content);
                }
                contents
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = IOPerformanceDetector::new();
        let patterns = detector.detect_anti_patterns(&ast);
        
        assert!(!patterns.is_empty());
        let io_pattern = patterns.iter().find(|p| {
            matches!(p, PerformanceAntiPattern::InefficientIO { 
                io_pattern: IOPattern::SyncInLoop, 
                .. 
            })
        });
        assert!(io_pattern.is_some());
    }
}
```

## Configuration

```toml
[performance]
enabled = true
detectors = ["nested_loops", "data_structures", "allocations", "io", "strings"]

[performance.nested_loops]
max_acceptable_nesting = 2
analyze_parallelization = true
complexity_threshold = "quadratic"

[performance.data_structures]
track_collection_size = true
suggest_alternatives = true
performance_impact_threshold = "medium"

[performance.allocations]
track_clone_frequency = true
string_concatenation_threshold = 3
large_allocation_threshold = 1024

[performance.io]
detect_sync_in_loops = true
suggest_batching = true
suggest_async = true

[performance.strings]
concatenation_in_loops = true
regex_compilation = true
formatting_in_loops = true
```

## Expected Impact

After implementation:

1. **Performance Optimization Guidance**: Automatic identification of performance bottlenecks
2. **Algorithm Improvement**: Specific suggestions for better algorithms and data structures
3. **Resource Efficiency**: Reduced memory allocations and I/O operations
4. **Scalability Improvements**: Early detection of patterns that don't scale
5. **Developer Education**: Performance awareness through automated analysis

This performance-focused detection capability significantly enhances technical debt assessment by identifying high-impact performance issues that can dramatically improve application responsiveness and resource usage.