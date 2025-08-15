---
number: 30
title: Enhanced Type Tracking for Field Access and Cross-Module Resolution
category: foundation
priority: high
status: draft
dependencies: [29]
created: 2025-08-15
---

# Specification 30: Enhanced Type Tracking for Field Access and Cross-Module Resolution

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [29 - AST-Based Type Tracking]

## Context

The current type tracking implementation (spec 29) successfully tracks local variables and resolves method calls on those variables. However, it has significant limitations that lead to false positives in dead code detection, particularly for methods called through field access chains.

A concrete example is `FrameworkPatternDetector::analyze_file()` which is called via:
```rust
self.enhanced_graph.framework_patterns.analyze_file(file_path, ast)?;
```

This pattern involves:
1. `self` reference in a method
2. Field access to `enhanced_graph` (a struct field)
3. Nested field access to `framework_patterns` 
4. Method call on the final field

The current implementation cannot track types through these field access chains, leading to approximately 10-20% false positive rate in dead code detection for methods called through struct fields.

## Objective

Extend the type tracking system to handle complex field access patterns, self references, struct field definitions, and cross-module type resolution. This will significantly reduce false positives in dead code detection and provide more accurate call graph analysis for the debtmap tool.

## Requirements

### Functional Requirements

1. **Struct Field Type Tracking**
   - Parse and store struct definitions with their field types
   - Build a registry of struct types and their fields
   - Support generic struct definitions with type parameters
   - Handle tuple structs and unit structs appropriately

2. **Self Reference Resolution**
   - Track the type of `self` in impl blocks and methods
   - Support `&self`, `&mut self`, and `self` (by value)
   - Maintain self type through method scope
   - Handle Self type alias in impl blocks

3. **Field Access Chain Resolution**
   - Resolve types through multiple field accesses (e.g., `a.b.c.d`)
   - Support method calls at any point in the chain
   - Handle field access on references and dereferenced values
   - Track types through tuple field access (e.g., `tuple.0`)

4. **Cross-Module Type Resolution**
   - Build a global type registry across all analyzed files
   - Resolve types imported via `use` statements
   - Handle module-qualified types (e.g., `module::Type`)
   - Support type aliases and re-exports

5. **Enhanced Method Resolution**
   - Resolve method calls on fields accurately
   - Distinguish between methods and associated functions
   - Handle trait methods on struct fields
   - Support method calls on nested fields

### Non-Functional Requirements

1. **Performance**
   - Type resolution overhead should remain under 30% of total analysis time
   - Memory usage should scale linearly with codebase size
   - Incremental updates should be possible for large codebases

2. **Accuracy**
   - Reduce false positive rate in dead code detection by at least 50%
   - Achieve 95%+ accuracy in resolving field access patterns
   - Handle edge cases gracefully with fallback mechanisms

3. **Maintainability**
   - Clear separation between type tracking phases
   - Well-documented type resolution algorithms
   - Extensible design for future enhancements

## Acceptance Criteria

- [ ] Struct definitions are parsed and field types are tracked
- [ ] Self references in methods resolve to the correct impl type
- [ ] Field access chains like `self.a.b.c` resolve to correct types
- [ ] Method calls on struct fields are properly resolved
- [ ] Cross-module types are correctly resolved via use statements
- [ ] The test case `FrameworkPatternDetector::analyze_file()` is no longer flagged as dead code
- [ ] False positive rate for dead code detection decreases by at least 50%
- [ ] All existing type tracking tests continue to pass
- [ ] New tests cover field access patterns, self references, and cross-module types
- [ ] Performance overhead remains under 30% for large codebases
- [ ] Memory usage scales linearly with number of type definitions

## Technical Details

### Implementation Approach

1. **Enhanced Type Registry**
```rust
pub struct GlobalTypeRegistry {
    /// Map from fully-qualified type name to type definition
    types: HashMap<String, TypeDefinition>,
    /// Map from module path to exported types
    module_exports: HashMap<Vec<String>, HashSet<String>>,
    /// Type alias mappings
    type_aliases: HashMap<String, String>,
}

pub struct TypeDefinition {
    pub name: String,
    pub kind: TypeKind,
    pub fields: Option<FieldRegistry>,
    pub methods: Vec<MethodSignature>,
    pub generic_params: Vec<String>,
}

pub struct FieldRegistry {
    /// Named fields for structs
    named_fields: HashMap<String, ResolvedType>,
    /// Positional fields for tuple structs
    tuple_fields: Vec<ResolvedType>,
}
```

2. **Multi-Phase Type Analysis**
```rust
impl TypeTracker {
    /// Phase 1: Collect all type definitions
    pub fn collect_type_definitions(&mut self, ast: &syn::File) {
        // Visit all struct, enum, and type alias definitions
        // Build the global type registry
    }
    
    /// Phase 2: Resolve field types
    pub fn resolve_field_types(&mut self) {
        // Resolve types referenced in field definitions
        // Handle generic parameters and constraints
    }
    
    /// Phase 3: Track variable and field access
    pub fn track_usage(&mut self, ast: &syn::File) {
        // Track variable declarations and assignments
        // Resolve field access chains
        // Track method calls on resolved types
    }
}
```

3. **Self Type Tracking**
```rust
impl<'ast> Visit<'ast> for TypeTracker {
    fn visit_impl_item_fn(&mut self, impl_fn: &'ast ImplItemFn) {
        // Determine self type from impl block
        let self_type = self.current_impl_type.clone();
        
        // Track self parameter type
        if let Some(self_param) = extract_self_param(&impl_fn.sig) {
            self.record_variable("self", ResolvedType {
                type_name: self_type,
                is_reference: self_param.is_reference,
                is_mutable: self_param.is_mutable,
                ..
            });
        }
        
        // Continue visiting
        syn::visit::visit_impl_item_fn(self, impl_fn);
    }
}
```

4. **Field Access Resolution**
```rust
pub fn resolve_field_access(&self, base_expr: &Expr, field_name: &str) -> Option<ResolvedType> {
    // Get the type of the base expression
    let base_type = self.resolve_expr_type(base_expr)?;
    
    // Look up the field in the type definition
    let type_def = self.type_registry.get(&base_type.type_name)?;
    
    // Get the field type
    type_def.fields?.get_field_type(field_name)
}
```

### Architecture Changes

1. **New Components**
   - `src/analyzers/type_registry.rs`: Global type registry implementation
   - `src/analyzers/field_resolver.rs`: Field access chain resolution
   - `src/analyzers/import_resolver.rs`: Use statement and module resolution

2. **Modified Components**
   - `src/analyzers/type_tracker.rs`: Extended with new resolution capabilities
   - `src/analyzers/rust_call_graph.rs`: Updated to use enhanced type tracking
   - `src/analysis/call_graph/mod.rs`: Integration with global type registry

### Data Structures

1. **Type Definition Storage**
   - Efficient HashMap-based lookups for type definitions
   - Lazy loading of type information from other modules
   - Cache for resolved type chains

2. **Scope Management**
   - Enhanced scope tracking with impl context
   - Module path tracking for qualified names
   - Import scope for use statements

### APIs and Interfaces

```rust
pub trait TypeResolver {
    /// Resolve a fully-qualified type name
    fn resolve_type(&self, name: &str) -> Option<TypeDefinition>;
    
    /// Resolve a field access on a type
    fn resolve_field(&self, type_name: &str, field: &str) -> Option<ResolvedType>;
    
    /// Resolve a method on a type
    fn resolve_method(&self, type_name: &str, method: &str) -> Option<MethodSignature>;
}

impl TypeTracker {
    /// Create with a shared type registry
    pub fn with_registry(registry: Arc<GlobalTypeRegistry>) -> Self;
    
    /// Analyze a file and populate the registry
    pub fn analyze_file(&mut self, path: &Path, ast: &syn::File) -> Result<()>;
    
    /// Resolve complex field access chains
    pub fn resolve_access_chain(&self, chain: &[AccessElement]) -> Option<ResolvedType>;
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 29: AST-Based Type Tracking (must be completed first)
  - Existing syn-based AST parsing infrastructure
  
- **Affected Components**:
  - Call graph extraction (`rust_call_graph.rs`)
  - Dead code detection (`unified_scorer.rs`)
  - Type tracking module (`type_tracker.rs`)
  
- **External Dependencies**: 
  - No new external crates required
  - Uses existing syn crate for AST parsing

## Testing Strategy

### Unit Tests

1. **Type Registry Tests**
   - Test struct definition parsing and storage
   - Test field type resolution
   - Test generic type handling
   - Test type alias resolution

2. **Field Access Tests**
   - Test single field access resolution
   - Test chained field access (2-5 levels deep)
   - Test field access on references
   - Test tuple field access

3. **Self Reference Tests**
   - Test self type in regular methods
   - Test self type in associated functions
   - Test &self vs &mut self resolution
   - Test Self type alias usage

4. **Cross-Module Tests**
   - Test type resolution across module boundaries
   - Test use statement resolution
   - Test module-qualified type names
   - Test re-exported types

### Integration Tests

1. **Real-World Patterns**
   - Test with actual debtmap codebase patterns
   - Test `FrameworkPatternDetector::analyze_file()` case
   - Test builder pattern with field access chains
   - Test complex nested struct access

2. **Performance Tests**
   - Benchmark type resolution on large codebases
   - Measure memory usage with many type definitions
   - Test incremental updates performance

3. **Accuracy Tests**
   - Measure false positive reduction in dead code detection
   - Validate correct method resolution percentages
   - Test edge cases and error recovery

### Regression Tests

- Ensure all existing type tracking tests pass
- Verify no performance degradation in simple cases
- Validate backward compatibility with existing analysis

## Documentation Requirements

### Code Documentation

- Document type resolution algorithm in detail
- Add examples of supported patterns to module docs
- Document limitations and unsupported patterns
- Include performance characteristics in docs

### User Documentation

- Update README with improved accuracy metrics
- Document new capabilities in user guide
- Add troubleshooting section for type resolution
- Include examples of newly supported patterns

### Architecture Updates

- Update ARCHITECTURE.md with new type tracking components
- Document the multi-phase type analysis approach
- Add sequence diagrams for field access resolution
- Update data flow diagrams with type registry

## Implementation Notes

### Phase 1: Core Infrastructure (Week 1)
- Implement GlobalTypeRegistry
- Add struct definition parsing
- Create basic field registry
- Set up type definition storage

### Phase 2: Self and Field Resolution (Week 2)
- Implement self type tracking
- Add single field access resolution
- Support chained field access
- Handle reference and dereference

### Phase 3: Cross-Module Support (Week 3)
- Implement use statement tracking
- Add module-qualified name resolution
- Support type aliases and re-exports
- Handle visibility modifiers

### Phase 4: Integration and Testing (Week 4)
- Integrate with existing call graph
- Update dead code detection
- Comprehensive testing
- Performance optimization

### Limitations to Accept

1. **Type Inference Boundaries**
   - Won't handle full Rust type inference
   - Limited support for complex generic constraints
   - No support for associated types initially
   - Trait objects handled as opaque types

2. **Dynamic Patterns**
   - No support for dynamic dispatch resolution
   - Limited macro expansion support
   - No async/await transformation tracking

3. **External Crates**
   - Limited support for types from external crates
   - No support for proc-macro generated code
   - Binary-only dependencies remain opaque

### Fallback Strategies

When type information cannot be resolved:
1. Fall back to existing name-based matching
2. Use heuristics for common patterns
3. Mark as "unknown type" rather than failing
4. Log unresolved types for debugging

## Migration and Compatibility

### Breaking Changes
- None expected - this is an enhancement to existing functionality

### Migration Path
1. Existing type tracking continues to work
2. New features are additive
3. Gradual rollout with feature flag if needed
4. Full backward compatibility maintained

### Compatibility Considerations
- Must work with all existing Rust syntax
- Should handle different Rust editions (2015, 2018, 2021)
- Graceful degradation for unsupported patterns
- No changes to public API or CLI interface

### Rollout Plan
1. Deploy behind feature flag initially
2. Test on internal projects first
3. Gradual rollout to beta users
4. Full release after validation
5. Remove feature flag after stability confirmed

## Success Metrics

1. **Accuracy Improvements**
   - 50% reduction in false positives for dead code detection
   - 95% accuracy in field access resolution
   - Zero regressions in existing test cases

2. **Performance Targets**
   - Less than 30% overhead for type resolution
   - Linear memory scaling with codebase size
   - Sub-second analysis for files under 1000 lines

3. **User Impact**
   - Reduced user reports of false positives
   - Improved confidence in tool recommendations
   - Better overall tool reliability

## Risk Assessment

### Technical Risks
- **Complexity**: Type resolution is inherently complex
  - Mitigation: Incremental implementation with thorough testing
- **Performance**: Could slow down analysis significantly
  - Mitigation: Careful optimization and caching strategies
- **Compatibility**: May not handle all Rust patterns
  - Mitigation: Graceful fallbacks and clear documentation

### Project Risks
- **Scope Creep**: Could expand to full type inference
  - Mitigation: Clear boundaries on supported patterns
- **Timeline**: Complex implementation may take longer
  - Mitigation: Phased approach with incremental delivery