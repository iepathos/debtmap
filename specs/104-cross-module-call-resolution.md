---
number: 104
title: Enhanced Cross-Module Call Resolution for Python
category: optimization
priority: high
status: draft
dependencies: [103]
created: 2025-09-28
---

# Specification 104: Enhanced Cross-Module Call Resolution for Python

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [103 - Cross-Module Python Call Graph Analysis]

## Context

Specification 103 successfully established the foundation for cross-module Python analysis with ImportTracker, CrossModuleContext, and GlobalSymbolTable components. Functions are now correctly detected across module boundaries. However, the actual call resolution between modules remains incomplete - imported functions are not properly linked to their call sites.

Current implementation correctly:
- Detects and registers all functions across modules
- Tracks import statements and module dependencies
- Builds a global symbol table
- Merges call graphs with proper node preservation

Current implementation fails to:
- Resolve calls to imported functions (e.g., `log_message()` called from `process_data()`)
- Track calls through imported class instances (e.g., `manager.process()`)
- Handle aliased imports (e.g., `import numpy as np`)
- Resolve calls through chained imports (A imports from B, B imports from C)

## Objective

Complete the cross-module call resolution implementation to accurately track all function and method calls across module boundaries, eliminating false positives in dead code detection for multi-file Python projects.

## Requirements

### Functional Requirements

1. **Import Resolution Enhancement**
   - Track the relationship between import statements and their usage in code
   - Map imported names to their source module functions
   - Handle various import styles (from X import Y, import X, from X import Y as Z)
   - Support wildcard imports (from X import *)

2. **Call Site Analysis**
   - Identify when an imported function is called
   - Link the call site to the actual function definition
   - Track the calling context (which function contains the call)
   - Preserve call type information (direct, method, etc.)

3. **Namespace Resolution**
   - Build import namespace for each module
   - Track how imported names are used within their scope
   - Handle shadowing and redefinition of imported names
   - Support module-level and function-level imports

4. **Alias Tracking**
   - Map aliased imports to their original names
   - Track usage of aliases throughout the module
   - Support chained aliases (import as, then assign to another name)
   - Handle common patterns like `import numpy as np`

5. **Type-Aware Resolution**
   - Use type information to resolve method calls on imported classes
   - Track instance creation from imported classes
   - Follow method calls through typed parameters
   - Leverage type hints for better resolution

### Non-Functional Requirements

1. **Performance**
   - Maintain O(n) complexity for import resolution
   - Cache resolved imports for repeated lookups
   - Minimal memory overhead for namespace tracking

2. **Accuracy**
   - Zero false negatives for standard import patterns
   - Minimize false positives through conservative resolution
   - Handle edge cases gracefully with fallback heuristics

3. **Maintainability**
   - Clear separation between import tracking and call resolution
   - Well-documented resolution algorithm
   - Comprehensive test coverage for import patterns

## Acceptance Criteria

- [ ] Imported function calls are correctly linked to their definitions
- [ ] Method calls on imported class instances show correct callers
- [ ] Aliased imports (as keyword) are properly resolved
- [ ] Wildcard imports link calls to correct source functions
- [ ] Chained imports (A→B→C) are resolved transitively
- [ ] Type hints improve method resolution accuracy
- [ ] All existing cross-module tests pass
- [ ] Performance remains within 10% of current implementation
- [ ] Test coverage for new code exceeds 90%

## Technical Details

### Implementation Approach

1. **Enhanced Import Namespace Building**
   ```rust
   pub struct ModuleNamespace {
       /// Direct imports: name -> (module_path, original_name)
       imports: HashMap<String, (PathBuf, String)>,
       /// Wildcard imports: module_path -> all exported names
       wildcard_imports: Vec<PathBuf>,
       /// Import aliases: alias -> original_name
       aliases: HashMap<String, String>,
       /// Scope-specific imports (function-level)
       scoped_imports: HashMap<String, ModuleNamespace>,
   }
   ```

2. **Two-Phase Resolution Strategy**
   - Phase 1: Build complete import namespaces for all modules
   - Phase 2: Resolve calls using namespace information

3. **Call Resolution Pipeline**
   ```rust
   fn resolve_imported_call(
       &self,
       call_name: &str,
       module_namespace: &ModuleNamespace,
       global_context: &CrossModuleContext,
   ) -> Option<FunctionId> {
       // 1. Check direct imports
       if let Some((source_module, original_name)) =
           module_namespace.resolve_import(call_name) {
           return global_context.resolve_function(&source_module, &original_name);
       }

       // 2. Check wildcard imports
       for wildcard_module in &module_namespace.wildcard_imports {
           if let Some(func_id) =
               global_context.resolve_function(wildcard_module, call_name) {
               return Some(func_id);
           }
       }

       // 3. Check aliases
       if let Some(original) = module_namespace.aliases.get(call_name) {
           return self.resolve_imported_call(original, module_namespace, global_context);
       }

       None
   }
   ```

### Architecture Changes

1. **Enhanced TwoPassExtractor**
   - Add import namespace building in phase one
   - Use namespace for call resolution in phase two
   - Track import context per function scope

2. **Improved CrossModuleContext**
   - Add namespace management methods
   - Implement transitive import resolution
   - Cache resolved imports for performance

3. **Modified Call Resolution**
   - Check import namespace before local resolution
   - Use type information for method resolution
   - Add fallback heuristics for unresolved imports

### Data Structures

```rust
/// Enhanced unresolved call with import context
pub struct UnresolvedCallWithContext {
    pub base: UnresolvedCall,
    pub import_context: Option<String>,  // The import used for this call
    pub is_imported: bool,
    pub module_alias: Option<String>,
}

/// Import usage tracking
pub struct ImportUsage {
    pub import_stmt: ImportStatement,
    pub usage_sites: Vec<Location>,
    pub resolved_targets: HashMap<String, FunctionId>,
}

/// Enhanced cross-module context
pub struct EnhancedCrossModuleContext {
    pub base: CrossModuleContext,
    pub namespaces: HashMap<PathBuf, ModuleNamespace>,
    pub import_usage: HashMap<PathBuf, Vec<ImportUsage>>,
    pub resolution_cache: HashMap<(PathBuf, String), Option<FunctionId>>,
}
```

### APIs and Interfaces

```rust
/// Resolve a call considering imports
pub fn resolve_call_with_imports(
    &self,
    call: &UnresolvedCall,
    module_path: &Path,
    context: &EnhancedCrossModuleContext,
) -> Option<FunctionId>;

/// Build module namespace from AST
pub fn build_module_namespace(
    module: &ast::Mod,
    module_path: &Path,
) -> ModuleNamespace;

/// Check if a name is imported
pub fn is_imported_name(
    &self,
    name: &str,
    scope: &FunctionScope,
) -> Option<ImportInfo>;
```

## Dependencies

- **Prerequisites**:
  - Specification 103 must be fully implemented
  - Basic cross-module infrastructure must be in place

- **Affected Components**:
  - `python_type_tracker.rs` - Enhanced resolution logic
  - `cross_module.rs` - Namespace management
  - `import_tracker.rs` - Import usage tracking
  - `analyze.rs` - Integration of enhanced resolution

- **External Dependencies**: None additional

## Testing Strategy

- **Unit Tests**:
  - Test namespace building for various import styles
  - Test import resolution with aliases
  - Test wildcard import resolution
  - Test scoped import handling

- **Integration Tests**:
  - Multi-module projects with complex import graphs
  - Circular import handling
  - Dynamic import patterns
  - Real-world Python packages (numpy, pandas usage patterns)

- **Performance Tests**:
  - Benchmark namespace building overhead
  - Test resolution cache effectiveness
  - Memory usage with large import graphs

- **Regression Tests**:
  - Ensure all spec 103 tests continue to pass
  - Verify no performance degradation
  - Check backward compatibility

## Documentation Requirements

- **Code Documentation**:
  - Document the import resolution algorithm
  - Explain namespace building process
  - Add examples of supported import patterns

- **User Documentation**:
  - Update README with cross-module capabilities
  - Add troubleshooting for import resolution issues
  - Document limitations with dynamic imports

- **Architecture Updates**:
  - Update ARCHITECTURE.md with namespace resolution flow
  - Document the two-phase resolution strategy
  - Add sequence diagrams for import tracking

## Implementation Notes

### Resolution Priority

1. Direct imports (explicit from X import Y)
2. Aliased imports (import X as Y)
3. Wildcard imports (from X import *)
4. Qualified access (module.function)
5. Heuristic matching (last resort)

### Edge Cases to Handle

1. **Circular imports**: A imports B, B imports A
2. **Re-exports**: Module imports and re-exports symbols
3. **Dynamic imports**: Using importlib or __import__
4. **Conditional imports**: Imports inside if statements
5. **Package imports**: Importing from __init__.py

### Performance Optimizations

1. **Lazy namespace building**: Build only when needed
2. **Resolution caching**: Cache successful resolutions
3. **Early termination**: Stop searching once found
4. **Batch processing**: Resolve multiple calls together

## Migration and Compatibility

### Breaking Changes
- None expected - this is an enhancement to existing functionality

### Migration Path
- Existing cross-module analysis will automatically benefit
- No changes required to user code or configuration
- Backward compatible with spec 103 implementation

### Configuration
- Optional flag to disable enhanced resolution
- Tunable cache size for resolution cache
- Configurable depth for transitive import resolution

## Success Metrics

- **Accuracy**: >95% correct resolution of imported function calls
- **Performance**: <10% overhead compared to spec 103 baseline
- **Coverage**: Support for all standard Python import patterns
- **Reliability**: Zero crashes on malformed imports

## Future Enhancements

- Support for Python stub files (.pyi)
- Integration with Python type checkers (mypy, pyright)
- Machine learning-based import resolution for ambiguous cases
- Support for namespace packages (PEP 420)
- Integration with Python package managers for third-party imports