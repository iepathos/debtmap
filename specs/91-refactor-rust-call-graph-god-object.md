---
number: 91
title: Refactor rust_call_graph.rs God Object
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-09-07
---

# Specification 91: Refactor rust_call_graph.rs God Object

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The `src/analyzers/rust_call_graph.rs` file has grown to 3,860 lines with 270 functions, making it the largest single file in the codebase. This represents a classic "God Object" anti-pattern that significantly impacts maintainability, testability, and contributes heavily to the technical debt score. The file handles multiple responsibilities including macro expansion, call resolution, graph construction, and trait handling, all of which should be separated into focused modules.

## Objective

Break down the monolithic `rust_call_graph.rs` file into multiple focused, cohesive modules that follow the Single Responsibility Principle, reducing complexity and improving the overall debt score by an estimated 50-100 points.

## Requirements

### Functional Requirements
- Preserve all existing functionality of the call graph analyzer
- Maintain backward compatibility with existing API consumers
- Keep performance characteristics equivalent or better
- Ensure all existing tests continue to pass

### Non-Functional Requirements
- Each new module should be under 500 lines
- Each module should have a single, clear responsibility
- Module interfaces should be well-defined and minimal
- Code should follow existing project conventions and style

## Acceptance Criteria

- [ ] The original `rust_call_graph.rs` file is reduced to under 500 lines
- [ ] At least 4 new focused modules are created from the extracted code
- [ ] All existing tests pass without modification
- [ ] Each new module has clear documentation of its responsibility
- [ ] The cyclomatic complexity of individual functions is reduced by at least 30%
- [ ] The technical debt score for this component drops by at least 50 points
- [ ] No performance regression in call graph analysis (benchmark before/after)

## Technical Details

### Implementation Approach

1. **Module Structure**:
   ```
   src/analyzers/call_graph/
   ├── mod.rs              // Public API and coordination
   ├── macro_expansion.rs  // MacroExpansionStats, MacroHandlingConfig
   ├── call_resolution.rs  // UnresolvedCall, resolution logic
   ├── graph_builder.rs    // Graph construction and traversal
   ├── trait_handling.rs   // Trait resolution and method dispatch
   └── utils.rs           // Shared utilities and helpers
   ```

2. **Extraction Strategy**:
   - Start with clearly defined structs and their implementations
   - Extract macro-related functionality first (least coupled)
   - Move trait resolution logic next
   - Extract graph building operations
   - Leave coordination logic in main module

### Architecture Changes
- Create new `call_graph` submodule under `analyzers`
- Establish clear interfaces between modules using traits
- Use dependency injection for cross-module dependencies
- Implement facade pattern in main module for backward compatibility

### Data Structures
- Keep existing public structs but organize into appropriate modules
- Create new internal structs for module communication if needed
- Use builder pattern for complex object construction

### APIs and Interfaces
```rust
// Main public interface remains in mod.rs
pub use self::macro_expansion::{MacroExpansionStats, MacroHandlingConfig};
pub use self::graph_builder::CallGraphExtractor;

// Internal traits for module communication
trait CallResolver {
    fn resolve_call(&self, call: &UnresolvedCall) -> Option<ResolvedCall>;
}

trait GraphBuilder {
    fn add_edge(&mut self, from: NodeId, to: NodeId, call_type: CallType);
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - All modules that import from `rust_call_graph.rs`
  - Test files referencing the module (13 test files identified)
- **External Dependencies**: No new dependencies required

## Testing Strategy

- **Unit Tests**: Create focused unit tests for each new module
- **Integration Tests**: Ensure existing integration tests pass
- **Performance Tests**: Benchmark call graph analysis before and after
- **Regression Tests**: Run full test suite to ensure no breakage

## Documentation Requirements

- **Code Documentation**: 
  - Module-level documentation explaining responsibility
  - Public API documentation with examples
  - Internal trait documentation
- **Architecture Updates**: Update ARCHITECTURE.md with new module structure
- **Migration Guide**: Document import path changes for consumers

## Implementation Notes

1. **Phased Approach**:
   - Phase 1: Create module structure and move structs
   - Phase 2: Extract macro expansion logic
   - Phase 3: Extract trait handling
   - Phase 4: Extract graph building
   - Phase 5: Clean up and optimize

2. **Risk Mitigation**:
   - Keep original file as backup during refactoring
   - Use feature flag to switch between old and new implementation
   - Extensive testing at each phase

3. **Performance Considerations**:
   - Minimize allocations during module communication
   - Use references where possible
   - Consider caching for frequently accessed data

## Migration and Compatibility

During the prototype phase, we can make breaking changes if needed for optimal design. However, we should:
- Provide clear migration path for existing code
- Update all internal usage sites
- Document all breaking changes clearly