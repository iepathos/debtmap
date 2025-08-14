---
number: 26
title: Language-Specific Call Graph Architecture
category: foundation
priority: high
status: draft
dependencies: [23]
created: 2025-01-14
---

# Specification 26: Language-Specific Call Graph Architecture

**Category**: foundation
**Priority**: high  
**Status**: draft
**Dependencies**: [23 - Enhanced Call Graph Analysis]

## Context

The current `EnhancedCallGraph` in `src/analysis/call_graph/` is misnamed as it contains Rust-specific analysis features like trait dispatch, function pointers, and Rust framework patterns. As the project expands to support multiple languages (Python already supported, more planned), we need a clearer architecture that separates language-agnostic call graph functionality from language-specific enhancements.

The current naming creates confusion about the purpose and scope of the enhanced call graph, and doesn't scale well for adding language-specific call graph analysis for Python, JavaScript, TypeScript, and other languages in the future.

## Objective

Rename `EnhancedCallGraph` to `RustCallGraph` to accurately reflect its language-specific nature, and establish a clear architectural pattern for language-specific call graph implementations that will scale as more languages are added to the project.

## Requirements

### Functional Requirements
- Rename `EnhancedCallGraph` struct to `RustCallGraph` throughout the codebase
- Rename `EnhancedCallGraphBuilder` to `RustCallGraphBuilder`
- Update all imports and usages to reflect the new naming
- Preserve all existing functionality without any behavioral changes
- Update documentation to clarify the language-specific nature of the implementation

### Non-Functional Requirements
- Zero impact on existing functionality - pure refactoring
- Maintain backward compatibility of public APIs if exposed
- Clear naming that immediately communicates language specificity
- Establish naming pattern for future language-specific implementations

## Acceptance Criteria

- [ ] All references to `EnhancedCallGraph` renamed to `RustCallGraph`
- [ ] All references to `EnhancedCallGraphBuilder` renamed to `RustCallGraphBuilder`
- [ ] Code compiles without errors after rename
- [ ] All existing tests pass without modification (except for name updates)
- [ ] Documentation updated to reflect new naming and architecture
- [ ] Module structure clearly shows language-specific nature
- [ ] No functional changes - pure refactoring verified through test suite

## Technical Details

### Implementation Approach
1. Rename the core struct and builder in `src/analysis/call_graph/mod.rs`
2. Update all imports throughout the codebase
3. Update documentation comments to reflect language-specific purpose
4. Consider moving to `src/analysis/rust_call_graph/` for clearer module organization

### Architecture Changes
- Current: `src/analysis/call_graph/` contains "enhanced" (Rust-specific) implementation
- Proposed: Either rename within existing location or restructure to:
  - `src/analysis/call_graph/` - Base call graph types and traits
  - `src/analysis/rust_call_graph/` - Rust-specific implementation
  - `src/analysis/python_call_graph/` - Future Python-specific implementation

### Data Structures
No changes to data structures, only naming:
- `EnhancedCallGraph` → `RustCallGraph`
- `EnhancedCallGraphBuilder` → `RustCallGraphBuilder`

### APIs and Interfaces
All public APIs remain the same, only type names change. This ensures compatibility while improving clarity.

## Dependencies

- **Prerequisites**: Specification 23 (Enhanced Call Graph Analysis) must remain implemented
- **Affected Components**: 
  - `src/analysis/call_graph/mod.rs`
  - `src/main.rs` (imports and usage)
  - Any other files importing or using `EnhancedCallGraph`
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Update test names to reflect new struct names
- **Integration Tests**: Ensure all integration tests pass without functional changes
- **Performance Tests**: Verify no performance regression from refactoring
- **User Acceptance**: CLI behavior remains identical

## Documentation Requirements

- **Code Documentation**: Update all doc comments to clarify Rust-specific nature
- **User Documentation**: Update any user-facing docs mentioning enhanced call graph
- **Architecture Updates**: Update ARCHITECTURE.md to reflect new naming and future language support pattern

## Implementation Notes

### Future Language Support Pattern
This refactoring establishes a pattern for future language-specific call graph implementations:
- `RustCallGraph` - Rust-specific features (traits, lifetimes, macros)
- `PythonCallGraph` - Python-specific features (duck typing, decorators, metaclasses)
- `JavaScriptCallGraph` - JS-specific features (prototypes, async/await, modules)
- `TypeScriptCallGraph` - TS-specific features (interfaces, generics, type guards)

Each language-specific implementation would:
1. Wrap the base `CallGraph` structure
2. Add language-specific analysis components
3. Implement a common trait/interface for polymorphic usage
4. Provide language-specific pattern detection

### Naming Conventions
- Use `{Language}CallGraph` for language-specific implementations
- Use `{Language}CallGraphBuilder` for corresponding builders
- Keep base `CallGraph` in `src/priority/call_graph.rs` as language-agnostic

## Migration and Compatibility

- **Breaking Changes**: None for end users - CLI interface unchanged
- **Migration Requirements**: None - pure internal refactoring
- **Compatibility Considerations**: 
  - If library is used as a dependency, type names will change
  - Consider type aliases for transition period if needed
  - Update any external documentation or examples

## Benefits

1. **Clarity**: Name immediately conveys language-specific purpose
2. **Scalability**: Clear pattern for adding more language-specific implementations
3. **Maintainability**: Easier to understand and modify language-specific logic
4. **Discoverability**: Developers can easily find language-specific features
5. **Future-Proofing**: Architecture ready for multi-language expansion