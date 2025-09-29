---
number: 106
title: Python Import Resolution Improvements
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-09-29
---

# Specification 106: Python Import Resolution Improvements

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current Python import resolution system has significant gaps that prevent accurate cross-module call graph construction. Many function calls across modules are not detected, leading to incomplete dependency analysis and false dead code detection.

Current limitations:
- Star imports (`from module import *`) not fully resolved
- Import aliases (`import foo as bar`) inconsistently handled
- Relative imports (`from ..module import func`) often fail
- Dynamic imports (`__import__`, `importlib`) not tracked
- Circular import handling is incomplete
- Package-level imports not properly resolved
- `__init__.py` re-exports not tracked
- Import resolution cache may miss entries

This impacts:
- Cross-module call graph accuracy
- Dead code detection across modules
- Dependency analysis
- Refactoring safety
- Impact analysis

## Objective

Implement robust import resolution for Python that accurately tracks all import patterns, resolves symbols across modules, and builds complete cross-module call graphs with support for complex import scenarios.

## Requirements

### Functional Requirements

- Resolve star imports by analyzing exported symbols
- Track import aliases throughout resolution
- Support all relative import patterns:
  - `from . import module`
  - `from .. import module`
  - `from ...package import module`
- Handle package imports and `__init__.py` exports
- Track re-exports and import forwarding
- Support namespace packages
- Resolve dynamic imports where statically analyzable
- Build comprehensive import dependency graph
- Cache resolution results efficiently
- Handle circular imports gracefully

### Non-Functional Requirements

- Fast resolution with caching
- Memory efficient for large projects
- Clear error reporting for unresolved imports
- Deterministic resolution order
- Thread-safe for parallel analysis

## Acceptance Criteria

- [ ] Star imports correctly resolve all exported symbols
- [ ] Import aliases tracked through entire call chain
- [ ] Relative imports work across package boundaries
- [ ] Package `__init__.py` exports properly resolved
- [ ] Circular imports handled without infinite loops
- [ ] Cross-module function calls accurately detected
- [ ] Import resolution cache hit rate > 90%
- [ ] Performance improvement > 20% for multi-file projects
- [ ] All Python import patterns covered by tests

## Technical Details

### Implementation Approach

1. Create `EnhancedImportResolver` in `src/analysis/python_imports.rs`
2. Build complete import graph before call resolution
3. Implement symbol table for each module
4. Add export analysis for star imports
5. Create multi-pass resolution for complex cases
6. Integrate with cross-module context

### Architecture Changes

```rust
// src/analysis/python_imports.rs
pub struct EnhancedImportResolver {
    module_symbols: HashMap<PathBuf, ModuleSymbols>,
    import_graph: ImportGraph,
    resolution_cache: HashMap<(PathBuf, String), ResolvedSymbol>,
    alias_map: HashMap<String, String>,
}

pub struct ModuleSymbols {
    path: PathBuf,
    exports: HashSet<String>,
    implicit_exports: HashSet<String>, // __all__
    functions: HashMap<String, FunctionId>,
    classes: HashMap<String, ClassInfo>,
    re_exports: HashMap<String, PathBuf>,
}

pub struct ImportGraph {
    edges: HashMap<PathBuf, Vec<ImportEdge>>,
    cycles: Vec<Vec<PathBuf>>,
}

pub enum ImportType {
    Direct,        // import module
    From,          // from module import name
    Star,          // from module import *
    Relative,      // from . import module
    Dynamic,       // __import__()
}
```

### Data Structures

- `ImportEdge`: Represents import relationship
- `ResolvedSymbol`: Final resolution result
- `ExportList`: Module's exported symbols
- `ImportContext`: Current resolution context

### APIs and Interfaces

```rust
impl EnhancedImportResolver {
    pub fn analyze_imports(&mut self, module: &ast::Module, path: &Path);
    pub fn resolve_symbol(&self, module: &Path, name: &str) -> Option<ResolvedSymbol>;
    pub fn get_module_exports(&self, module: &Path) -> &HashSet<String>;
    pub fn resolve_star_imports(&self, module: &Path) -> Vec<ResolvedSymbol>;
    pub fn build_import_graph(&mut self, modules: &[(ast::Module, PathBuf)]);
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analysis/python_call_graph/cross_module.rs`
  - `src/analysis/python_type_tracker.rs`
  - `src/builders/call_graph.rs`
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Each import pattern type
- **Integration Tests**: Multi-module resolution
- **Edge Cases**: Circular imports, missing modules
- **Performance Tests**: Large project import graphs
- **Regression Tests**: Real-world import patterns

## Documentation Requirements

- **Code Documentation**: Import resolution algorithm
- **User Documentation**: Supported import patterns
- **Troubleshooting**: Common import resolution issues
- **Examples**: Complex import scenarios

## Implementation Notes

- Build import graph before call graph
- Use topological sort for resolution order
- Handle missing modules gracefully
- Cache aggressively but invalidate correctly
- Support Python 2 and 3 import differences
- Log import resolution for debugging
- Consider sys.path and PYTHONPATH

## Migration and Compatibility

- Backward compatible with existing resolution
- Gradual improvement as patterns added
- No configuration changes required
- Existing call graphs remain valid but improve