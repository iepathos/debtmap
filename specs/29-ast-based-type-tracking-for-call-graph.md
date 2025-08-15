---
number: 29
title: AST-Based Type Tracking for Accurate Method Call Resolution
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-08-15
---

# Specification 29: AST-Based Type Tracking for Accurate Method Call Resolution

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current call graph analysis in debtmap incorrectly resolves method calls, leading to false positives in dead code detection. When encountering method calls like `dep_graph.calculate_coupling_metrics()` or `calc.calculate()`, the analyzer cannot determine the correct type of the receiver object and therefore cannot resolve these calls to the appropriate method implementations (e.g., `DependencyGraph::calculate_coupling_metrics` or `Calculator::calculate`).

This results in:
- Methods being incorrectly marked as dead code when they are actually called
- Inaccurate dependency analysis
- Misleading technical debt metrics
- False positives that reduce user trust in the tool

Current workarounds using naming patterns or heuristics are inadequate and don't generalize to real-world codebases.

## Objective

Implement proper AST-based type tracking with scope management to accurately resolve method calls by maintaining a symbol table that tracks variable types throughout the code analysis. This will eliminate false positives in dead code detection and provide accurate call graph analysis.

## Requirements

### Functional Requirements

1. **Variable Type Tracking**
   - Track variable declarations with explicit type annotations
   - Track variable assignments from constructors and method calls
   - Track function parameter types
   - Track struct field types
   - Support type aliases and imports

2. **Scope Management**
   - Maintain nested scope hierarchy (function, block, module)
   - Support variable shadowing within inner scopes
   - Clean up scope information when exiting blocks
   - Handle closure captures and their types

3. **Type Resolution**
   - Resolve method calls to their correct implementations
   - Support method calls on self, variables, and field accesses
   - Handle chained method calls (e.g., `obj.method1().method2()`)
   - Resolve calls through type aliases

4. **Integration with Call Graph**
   - Update call graph extractor to use type information
   - Maintain backward compatibility with existing analysis
   - Preserve performance for large codebases

### Non-Functional Requirements

- Performance impact should be minimal (< 20% overhead)
- Memory usage should scale linearly with codebase size
- Must handle incomplete type information gracefully
- Should work incrementally without full project compilation

## Acceptance Criteria

- [ ] Variable type tracking correctly identifies types from explicit annotations
- [ ] Method calls like `dep_graph.calculate_coupling_metrics()` resolve to `DependencyGraph::calculate_coupling_metrics`
- [ ] The test `test_rust_method_with_same_name_as_function_not_false_positive` passes
- [ ] The test `test_rust_function_vs_method_distinction` passes
- [ ] No regression in existing call graph analysis tests
- [ ] Performance overhead is less than 20% on large codebases
- [ ] Memory usage remains reasonable for projects with 100k+ LOC
- [ ] False positive rate for dead code detection decreases by at least 50%

## Technical Details

### Implementation Approach

1. **Create Type Tracking Infrastructure**
```rust
pub struct TypeTracker {
    /// Stack of scopes, innermost last
    scopes: Vec<Scope>,
    /// Current module path for resolving imports
    module_path: Vec<String>,
    /// Type definitions found in the file
    type_definitions: HashMap<String, TypeInfo>,
}

pub struct Scope {
    /// Variable name to type mapping
    variables: HashMap<String, ResolvedType>,
    /// Scope kind (function, block, impl, module)
    kind: ScopeKind,
    /// Parent type for impl blocks
    impl_type: Option<String>,
}

pub struct ResolvedType {
    /// Fully qualified type name
    type_name: String,
    /// Source location where type was determined
    source: TypeSource,
    /// Generic parameters if any
    generics: Vec<String>,
}

pub enum TypeSource {
    /// Explicit type annotation
    Annotation(Span),
    /// Constructor call (e.g., Type::new())
    Constructor(Span),
    /// Struct literal
    StructLiteral(Span),
    /// Function return type
    FunctionReturn(Span),
    /// Field access
    FieldAccess(Span),
}
```

2. **Enhance Call Graph Extractor**
```rust
impl CallGraphExtractor {
    /// Track type when visiting variable declarations
    fn visit_local(&mut self, local: &Local) {
        if let Some(ty) = extract_type_from_pattern(&local.pat, &local.init) {
            self.type_tracker.record_variable(var_name, ty);
        }
    }
    
    /// Use type information when resolving method calls
    fn visit_expr_method_call(&mut self, method_call: &ExprMethodCall) {
        let receiver_type = self.type_tracker.resolve_expr_type(&method_call.receiver);
        let qualified_name = if let Some(ty) = receiver_type {
            format!("{}::{}", ty, method_call.method)
        } else {
            // Fallback to unqualified name
            method_call.method.to_string()
        };
        self.add_unresolved_call(qualified_name, ...);
    }
}
```

3. **Type Extraction Patterns**
```rust
/// Extract type from various AST patterns
fn extract_type_from_pattern(pat: &Pat, init: &Option<Box<Expr>>) -> Option<ResolvedType> {
    match pat {
        Pat::Type(pat_type) => {
            // Explicit type annotation: let x: Type = ...
            Some(extract_type_from_type(&pat_type.ty))
        }
        Pat::Ident(pat_ident) if init.is_some() => {
            // Type inference from initializer
            extract_type_from_expr(init.as_ref().unwrap())
        }
        _ => None
    }
}

fn extract_type_from_expr(expr: &Expr) -> Option<ResolvedType> {
    match expr {
        Expr::Call(call) => {
            // Constructor call: Type::new()
            if let Expr::Path(path) = &*call.func {
                extract_type_from_constructor_path(&path.path)
            } else {
                None
            }
        }
        Expr::Struct(struct_expr) => {
            // Struct literal: Type { field: value }
            Some(extract_type_from_path(&struct_expr.path))
        }
        Expr::Path(path) => {
            // Variable reference - look up in type tracker
            None // Requires type tracker context
        }
        _ => None
    }
}
```

### Architecture Changes

1. Add `TypeTracker` component to `CallGraphExtractor`
2. Enhance AST visitor pattern to track type information
3. Modify call resolution to use type information
4. Update test infrastructure to verify type tracking

### Data Structures

- `TypeTracker`: Main type tracking component
- `Scope`: Represents a lexical scope with variable bindings
- `ResolvedType`: Represents a resolved type with metadata
- `TypeSource`: Tracks how a type was determined
- `ScopeKind`: Enum for different scope types

### APIs and Interfaces

No public API changes. Internal changes to `CallGraphExtractor`:
- New method: `track_variable_type(name: String, ty: ResolvedType)`
- New method: `resolve_variable_type(name: &str) -> Option<ResolvedType>`
- Enhanced: `resolve_function` to consider type information

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/analyzers/rust_call_graph.rs`
  - `src/priority/call_graph.rs`
  - Test files for call graph analysis
- **External Dependencies**: None (uses existing syn crate)

## Testing Strategy

- **Unit Tests**: 
  - Test type extraction from various patterns
  - Test scope entry/exit
  - Test variable shadowing
  - Test type resolution in nested scopes

- **Integration Tests**:
  - Test real-world code patterns
  - Test with debtmap's own codebase
  - Test with popular Rust projects

- **Performance Tests**:
  - Benchmark analysis time before/after
  - Memory usage profiling
  - Test with large codebases (100k+ LOC)

- **Regression Tests**:
  - Ensure all existing tests pass
  - Add tests for previously failing cases

## Documentation Requirements

- **Code Documentation**: 
  - Document TypeTracker API
  - Add examples of type tracking patterns
  - Document limitations and edge cases

- **User Documentation**:
  - Update README with improved accuracy claims
  - Add section on how type tracking works
  - Document any new CLI options

- **Architecture Updates**:
  - Update ARCHITECTURE.md with type tracking component
  - Document data flow for type resolution
  - Add sequence diagrams for method call resolution

## Implementation Notes

### Phase 1: Basic Type Tracking
- Implement TypeTracker struct
- Add scope management
- Track explicit type annotations
- Track simple constructor patterns

### Phase 2: Enhanced Resolution
- Track struct literals
- Handle method chaining
- Support type aliases
- Track imports and use statements

### Phase 3: Advanced Features
- Handle generic types (basic)
- Track closure types
- Support trait methods
- Handle type inference for common patterns

### Limitations to Accept
- Won't handle full type inference (e.g., `let x = vec![]`)
- Won't resolve complex generic constraints
- Won't handle dynamic dispatch (trait objects)
- Won't track types across file boundaries initially

### Fallback Strategy
When type information is unavailable:
1. Try to match qualified names first
2. Use existing resolution logic
3. Log unresolved types for debugging

## Migration and Compatibility

- No breaking changes to external API
- Existing analysis results remain valid
- Can be enabled/disabled via feature flag initially
- Gradual rollout with monitoring

### Rollout Plan
1. Implement behind feature flag
2. Test with internal projects
3. Beta test with selected users
4. Enable by default
5. Remove feature flag after stability confirmed

### Metrics to Track
- False positive rate before/after
- Performance impact
- Memory usage increase
- User-reported issues