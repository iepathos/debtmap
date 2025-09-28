---
number: 103
title: Cross-Module Python Call Graph Analysis
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-09-28
---

# Specification 103: Cross-Module Python Call Graph Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current Python call graph analysis in debtmap successfully detects many patterns within single modules but fails to properly track calls across module boundaries. This leads to false positives where methods are incorrectly flagged as dead code despite being called from other modules.

Recent improvements have addressed several Python-specific patterns:
- Event binding detection (Bind, connect, etc.) within the same module
- Module-level execution patterns (`if __name__ == "__main__"`)
- Framework method exclusions

However, cross-module analysis remains problematic, particularly for:
- Methods called via imported instances (e.g., `conversation_manager.register_observer()`)
- Framework methods like `OnInit` that should be recognized regardless of module
- Observer pattern implementations split across files
- Dependency injection patterns where objects are passed between modules

## Objective

Enhance the Python call graph analyzer to accurately track function and method calls across module boundaries, eliminating false positives for inter-module dependencies while maintaining performance for large codebases.

## Requirements

### Functional Requirements

1. **Cross-Module Import Tracking**
   - Track Python imports (`import`, `from ... import`) to understand module relationships
   - Build a module dependency graph during analysis
   - Resolve qualified names across module boundaries

2. **Instance Method Resolution**
   - Track when instances are passed as parameters between modules
   - Resolve method calls on parameters based on type hints or inferred types
   - Handle common patterns like dependency injection and observer registration

3. **Framework Pattern Recognition**
   - Maintain global framework pattern detection across all analyzed modules
   - Share framework context between module analyses
   - Recognize framework methods regardless of import structure

4. **Type Inference Improvements**
   - Enhance type tracking for parameters and return values
   - Use type hints when available for better resolution
   - Track instance types across module boundaries

5. **Call Graph Merging**
   - Properly merge call graphs from multiple modules
   - Resolve cross-references after all modules are analyzed
   - Handle circular dependencies gracefully

### Non-Functional Requirements

1. **Performance**
   - Analysis should remain efficient for projects with 100+ Python files
   - Incremental analysis capability for changed files only
   - Memory usage should scale linearly with project size

2. **Accuracy**
   - False positive rate should decrease by at least 50% for cross-module patterns
   - Maintain current accuracy for single-module analysis
   - No increase in false negatives

3. **Compatibility**
   - Support Python 3.6+ syntax and patterns
   - Handle both absolute and relative imports
   - Work with common Python project structures

## Acceptance Criteria

- [ ] Cross-module method calls are properly tracked in the call graph
- [ ] Framework methods (OnInit, setUp, etc.) are recognized across modules
- [ ] Observer pattern implementations show correct caller relationships
- [ ] Instance methods called via parameters show callers
- [ ] Import statements are parsed and module dependencies tracked
- [ ] Type hints are utilized for better method resolution
- [ ] Analysis performance remains within 2x of current speed
- [ ] Test coverage includes multi-module scenarios

## Technical Details

### Implementation Approach

1. **Two-Phase Analysis Enhancement**
   - Phase 1a: Parse all modules and extract signatures, types, and imports
   - Phase 1b: Build global symbol table and module dependency graph
   - Phase 2a: Analyze each module with access to global context
   - Phase 2b: Resolve cross-module references and merge call graphs

2. **Global Symbol Table**
   - Maintain a project-wide symbol table during analysis
   - Map fully qualified names to FunctionIds
   - Track module exports and imports

3. **Enhanced Type Tracker**
   - Extend `PythonTypeTracker` to handle imports
   - Track parameter types across function boundaries
   - Use type hints and docstrings for type inference

### Architecture Changes

1. **New Components**
   - `CrossModuleAnalyzer`: Coordinates multi-module analysis
   - `ImportTracker`: Parses and tracks import statements
   - `GlobalSymbolTable`: Project-wide symbol resolution

2. **Modified Components**
   - `TwoPassExtractor`: Enhanced to use global context
   - `PythonTypeTracker`: Extended type inference across modules
   - `CallGraph`: Improved merging capabilities

### Data Structures

```rust
pub struct CrossModuleContext {
    /// Global symbol table for all modules
    pub symbols: HashMap<String, FunctionId>,
    /// Module dependency graph
    pub dependencies: HashMap<PathBuf, Vec<PathBuf>>,
    /// Imported symbols per module
    pub imports: HashMap<PathBuf, Vec<ImportedSymbol>>,
    /// Exported symbols per module
    pub exports: HashMap<PathBuf, Vec<ExportedSymbol>>,
}

pub struct ImportedSymbol {
    pub module: String,
    pub name: String,
    pub alias: Option<String>,
    pub is_wildcard: bool,
}
```

### APIs and Interfaces

```rust
/// Analyze multiple Python modules with cross-module resolution
pub fn analyze_python_project(
    files: &[PathBuf],
    call_graph: &mut CallGraph,
) -> Result<()>;

/// Build cross-module context from Python files
pub fn build_cross_module_context(
    files: &[PathBuf],
) -> Result<CrossModuleContext>;
```

## Dependencies

- **Prerequisites**: None (builds on existing Python analysis)
- **Affected Components**:
  - `python_type_tracker.rs`
  - `python_call_graph/mod.rs`
  - `priority/scoring/classification.rs`
- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - Test import parsing and symbol resolution
  - Test type inference across modules
  - Test call graph merging

- **Integration Tests**:
  - Multi-file Python projects with various import patterns
  - Framework applications (Django, Flask, wxPython)
  - Observer pattern implementations

- **Performance Tests**:
  - Benchmark on projects with 10, 50, 100+ files
  - Memory usage profiling
  - Incremental analysis performance

- **User Acceptance**:
  - Test on real projects like promptconstruct-frontend
  - Verify false positive reduction
  - Validate no regression in single-module analysis

## Documentation Requirements

- **Code Documentation**:
  - Document cross-module analysis algorithm
  - Explain symbol resolution strategy
  - Add examples of supported patterns

- **User Documentation**:
  - Update README with multi-module capabilities
  - Add troubleshooting guide for cross-module issues
  - Document limitations and edge cases

- **Architecture Updates**:
  - Update ARCHITECTURE.md with cross-module flow
  - Document new components and their interactions
  - Add sequence diagrams for multi-phase analysis

## Implementation Notes

### Known Challenges

1. **Circular Dependencies**: Python allows circular imports which complicate analysis
2. **Dynamic Imports**: `importlib` and dynamic imports are harder to track
3. **Module Aliases**: The same module may be imported under different names
4. **Performance**: Analyzing all modules together may impact performance

### Suggested Solutions

1. Use topological sort for module analysis order
2. Fall back to heuristics for dynamic imports
3. Normalize module names to canonical paths
4. Implement caching and incremental analysis

### Future Enhancements

- Support for Python stub files (.pyi)
- Integration with Python type checkers (mypy, pyright)
- Support for virtual environments and installed packages
- Machine learning-based type inference

## Migration and Compatibility

### Breaking Changes
- None expected for existing users

### Migration Path
- The enhancement is backward compatible
- Existing single-module analysis continues to work
- Cross-module analysis is opt-in via new API

### Configuration
- Add option to enable/disable cross-module analysis
- Configure depth of import following
- Set performance vs accuracy trade-offs