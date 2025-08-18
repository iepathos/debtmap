---
number: 51
title: Data Flow Analysis for Input Validation Detection
category: foundation
priority: high
status: draft
dependencies: [43]
created: 2025-01-18
---

# Specification 51: Data Flow Analysis for Input Validation Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [43 - Context-Aware False Positive Reduction]

## Context

The current Input Validation detector in `src/security/input_validation_detector.rs` generates an extremely high rate of false positives. Analysis reveals that all 10 top-priority "security vulnerabilities" flagged by the system are false positives - functions that merely check for input patterns or perform analysis, rather than actually handling external input.

The core issue is that the detector uses simplistic pattern matching on variable names and function calls, without understanding actual data flow. It conflates:
- Functions that **detect** input sources with functions that **handle** input
- Variable names containing "input" with actual external input
- Analysis/utility functions with input-handling functions
- Security scanning code with security vulnerabilities

This makes the detector essentially unusable in real-world codebases, particularly in security tools, parsers, compilers, and any code that analyzes or documents input patterns.

## Objective

Replace the current pattern-matching approach with proper data flow analysis that tracks actual input from sources to sinks, eliminating false positives while maintaining detection of real input validation gaps.

## Requirements

### Functional Requirements

1. **Data Flow Graph Construction**
   - Build a data flow graph tracking variable assignments and propagation
   - Track data from actual input sources through transformations to usage
   - Support inter-procedural analysis across function boundaries
   - Handle control flow (if/else, match, loops) in taint propagation

2. **Accurate Source Detection**
   - Distinguish between actual input operations and pattern checking
   - Identify real external input sources:
     - File I/O operations that read data
     - Network operations receiving data
     - CLI arguments and environment variables
     - User input from stdin
   - Exclude analysis functions that check for patterns

3. **Taint Propagation**
   - Track tainted data through variable assignments
   - Propagate taint through function parameters and returns
   - Handle field access and struct/enum construction
   - Support collection operations (Vec, HashMap, etc.)

4. **Validation Detection**
   - Identify actual validation operations on tainted data
   - Recognize sanitization patterns (parsing with error handling)
   - Detect validation frameworks and libraries
   - Track validation state through the data flow

5. **Context Integration**
   - Leverage existing context detection from spec 43
   - Skip test functions and test utilities
   - Understand function roles (utility, analysis, handler)
   - Respect framework patterns

### Non-Functional Requirements

1. **Performance**
   - Incremental analysis capability for large codebases
   - Lazy evaluation of data flow paths
   - Efficient graph representation and traversal
   - Cache analysis results between runs

2. **Accuracy**
   - Zero false positives for analysis/detection functions
   - Maintain high recall for actual input validation gaps
   - Clear explanation of detected issues with data flow path

3. **Maintainability**
   - Modular design separating graph construction from analysis
   - Extensible framework for adding new source/sink patterns
   - Clear separation between language-specific and generic logic

## Acceptance Criteria

- [ ] Data flow graph correctly tracks variable assignments and propagation
- [ ] Functions that check for patterns (like `is_cli_argument_source`) are not flagged
- [ ] Functions that format strings (like `generate_message`) are not flagged
- [ ] Actual input handling without validation is correctly detected
- [ ] Test functions are excluded from validation requirements
- [ ] Utility functions without external input are not flagged
- [ ] Clear data flow path is provided for each detected issue
- [ ] Performance impact is less than 20% on analysis time
- [ ] All existing test cases pass after implementation
- [ ] New test cases demonstrate elimination of false positives

## Technical Details

### Implementation Approach

1. **Phase 1: Data Flow Infrastructure**
   - Create `data_flow` module with graph construction
   - Implement `DataFlowGraph` struct with nodes and edges
   - Add visitor pattern for AST traversal and graph building

2. **Phase 2: Source and Sink Analysis**
   - Refactor source detection to check actual operations
   - Implement operation classification (read vs check)
   - Create sink detection for dangerous operations

3. **Phase 3: Taint Propagation Engine**
   - Implement forward taint analysis algorithm
   - Add support for inter-procedural analysis
   - Handle control flow and merging taint states

4. **Phase 4: Integration**
   - Replace pattern matching in `input_validation_detector.rs`
   - Integrate with context detection from spec 43
   - Update scoring and reporting

### Architecture Changes

```rust
// New module structure
src/
  data_flow/
    mod.rs           // Public API
    graph.rs         // Data flow graph representation
    builder.rs       // Graph construction from AST
    taint.rs         // Taint propagation algorithms
    sources.rs       // Input source detection
    sinks.rs         // Dangerous operation detection
    validation.rs    // Validation pattern recognition
```

### Data Structures

```rust
pub struct DataFlowGraph {
    nodes: HashMap<NodeId, DataFlowNode>,
    edges: Vec<DataFlowEdge>,
    entry_points: Vec<NodeId>,
}

pub enum DataFlowNode {
    Variable { name: String, location: SourceLocation },
    Expression { kind: ExpressionKind, location: SourceLocation },
    Parameter { function: String, index: usize },
    Return { function: String },
}

pub struct TaintState {
    tainted_nodes: HashSet<NodeId>,
    sources: HashMap<NodeId, InputSource>,
    validations: HashMap<NodeId, ValidationType>,
}

pub enum OperationType {
    Read,      // Actually reads external input
    Check,     // Checks for pattern/existence
    Transform, // Transforms data
    Validate,  // Validates/sanitizes data
}
```

### APIs and Interfaces

```rust
pub trait DataFlowAnalyzer {
    fn build_graph(&self, file: &syn::File) -> DataFlowGraph;
    fn analyze_taint(&self, graph: &DataFlowGraph) -> TaintAnalysis;
    fn find_validation_gaps(&self, analysis: &TaintAnalysis) -> Vec<ValidationGap>;
}

pub struct ValidationGap {
    pub source: InputSource,
    pub sink: Option<SinkOperation>,
    pub path: Vec<NodeId>,
    pub location: SourceLocation,
    pub severity: Severity,
    pub explanation: String,
}
```

## Dependencies

- **Prerequisites**: Spec 43 (Context-Aware False Positive Reduction) for context detection
- **Affected Components**: 
  - `src/security/input_validation_detector.rs` - Complete rewrite
  - `src/security/taint_analysis.rs` - Enhance and integrate
  - `src/analyzer/mod.rs` - Update to use new analysis
- **External Dependencies**: 
  - Consider `petgraph` for efficient graph operations
  - Reuse existing `syn` visitor patterns

## Testing Strategy

- **Unit Tests**:
  - Graph construction from various AST patterns
  - Taint propagation through different control flows
  - Source/sink classification accuracy
  - Validation detection patterns

- **Integration Tests**:
  - Test with security analysis tools (should have zero false positives)
  - Test with actual vulnerable code (should detect issues)
  - Test with various frameworks and patterns
  - Performance benchmarks on large codebases

- **Regression Tests**:
  - Ensure all previously detected true positives are still found
  - Verify false positives from issue report are eliminated
  - Test with debtmap's own codebase

## Documentation Requirements

- **Code Documentation**:
  - Document data flow algorithm and assumptions
  - Explain taint propagation rules
  - Provide examples of detected vs non-detected patterns

- **User Documentation**:
  - Update README with new detection capabilities
  - Explain how to interpret data flow paths
  - Provide guidance on fixing detected issues

- **Architecture Updates**:
  - Add data flow module to ARCHITECTURE.md
  - Document integration with existing security detectors
  - Update detector interaction diagrams

## Implementation Notes

1. **Incremental Migration**: Start with a parallel implementation alongside existing detector, then switch over after validation

2. **Language Agnostic Core**: Design data flow graph to be language-agnostic, with language-specific builders

3. **Explainability**: Each detection should include the full data flow path for debugging

4. **Configuration**: Allow users to configure custom source/sink patterns for domain-specific cases

5. **Caching Strategy**: Cache data flow graphs at the file level, invalidate on file changes

## Migration and Compatibility

- The new implementation will be backwards compatible in terms of CLI interface
- Output format remains the same, but with more accurate results
- Existing suppressions will continue to work
- Consider providing a flag to use old detector during transition period