---
number: 95
title: Reduce Module Dependencies
category: optimization
priority: medium
status: draft
dependencies: [91]
created: 2025-09-07
---

# Specification 95: Reduce Module Dependencies

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [91 - Refactor rust_call_graph.rs God Object]

## Context

The codebase exhibits high coupling between modules, with complex dependency chains that increase technical debt. Many modules directly import from multiple other modules, creating tight coupling and making the code harder to maintain, test, and refactor. Dependency analysis shows circular dependencies in some areas and excessive coupling in others. Reducing these dependencies through better abstraction and dependency injection would improve modularity and reduce debt score by an estimated 20-30 points.

## Objective

Reduce inter-module coupling by establishing clear module boundaries, implementing dependency injection patterns, and creating abstraction layers that minimize direct dependencies between modules.

## Requirements

### Functional Requirements
- Identify and document all module dependencies
- Eliminate circular dependencies completely
- Reduce direct cross-module imports by at least 40%
- Implement dependency injection for configurable components
- Create clear module interfaces using traits

### Non-Functional Requirements
- Maintain or improve performance
- Preserve all existing functionality
- Follow Rust idioms and best practices
- Improve testability through better isolation
- Enhance code reusability

## Acceptance Criteria

- [ ] No circular dependencies exist in the codebase
- [ ] Module coupling reduced by at least 40% as measured by dependency analysis
- [ ] All major modules have defined trait interfaces
- [ ] Dependency injection implemented for at least 5 major components
- [ ] Module dependency graph is documented
- [ ] Technical debt score reduced by at least 20 points
- [ ] All existing tests pass without modification

## Technical Details

### Current Problem Areas

1. **Analyzer Dependencies**:
   - Direct imports between analyzer modules
   - Shared state without clear ownership
   - Tight coupling to scoring modules

2. **Scoring and Priority Modules**:
   - Circular dependencies with analyzers
   - Direct access to internal structures
   - Mixed responsibilities

3. **I/O and Cache Modules**:
   - Hard-coded dependencies on specific implementations
   - No abstraction layer for different backends
   - Direct file system access scattered throughout

### Proposed Architecture

```rust
// Define clear trait boundaries
pub trait Analyzer {
    type Input;
    type Output;
    fn analyze(&self, input: Self::Input) -> Result<Self::Output>;
}

pub trait Scorer {
    type Item;
    fn score(&self, item: &Self::Item) -> f64;
}

pub trait Cache {
    type Key;
    type Value;
    fn get(&self, key: &Self::Key) -> Option<Self::Value>;
    fn set(&mut self, key: Self::Key, value: Self::Value);
}

// Use dependency injection
pub struct DebtAnalyzer<A, S, C> 
where
    A: Analyzer,
    S: Scorer,
    C: Cache,
{
    analyzer: A,
    scorer: S,
    cache: C,
}

impl<A, S, C> DebtAnalyzer<A, S, C> 
where
    A: Analyzer,
    S: Scorer<Item = A::Output>,
    C: Cache,
{
    pub fn new(analyzer: A, scorer: S, cache: C) -> Self {
        Self { analyzer, scorer, cache }
    }
}
```

### Module Structure Improvements

1. **Create Core Module**:
   ```
   src/core/
   ├── traits.rs      // All shared trait definitions
   ├── types.rs       // Common type definitions
   └── errors.rs      // Shared error types
   ```

2. **Establish Layer Boundaries**:
   ```
   Domain Layer (no external deps):
   - src/scoring/
   - src/complexity/
   - src/priority/
   
   Application Layer (domain + infrastructure traits):
   - src/analyzers/
   - src/organization/
   
   Infrastructure Layer (external I/O):
   - src/io/
   - src/cache/
   - src/config/
   ```

3. **Dependency Rules**:
   - Domain layer has no dependencies
   - Application layer depends only on domain and trait definitions
   - Infrastructure provides trait implementations
   - Cross-layer communication only through traits

### Refactoring Strategy

1. **Phase 1: Trait Extraction**
   - Extract common interfaces into traits
   - Move traits to core module
   - Update implementations to use traits

2. **Phase 2: Dependency Injection**
   - Replace direct instantiation with injection
   - Use builder pattern for complex construction
   - Implement factory traits where appropriate

3. **Phase 3: Module Reorganization**
   - Move modules to appropriate layers
   - Eliminate cross-layer direct dependencies
   - Update imports to use trait boundaries

4. **Phase 4: Circular Dependency Resolution**
   - Identify circular dependencies using cargo-deps
   - Break cycles by extracting shared interfaces
   - Move shared code to common modules

## Dependencies

- **Prerequisites**: 
  - Spec 91 (God Object refactoring) should be completed first
- **Affected Components**: 
  - All modules will need import updates
  - Test modules may need dependency injection setup
- **External Dependencies**: 
  - Consider adding `cargo-deps` for visualization
  - May use `cargo-modules` for analysis

## Testing Strategy

- **Unit Tests**: Test modules in isolation with mock dependencies
- **Integration Tests**: Verify module interactions through traits
- **Dependency Tests**: Automated checks for circular dependencies
- **Performance Tests**: Ensure no regression from abstraction

## Documentation Requirements

- **Dependency Graph**: Visual representation of module dependencies
- **Layer Documentation**: Clear documentation of architectural layers
- **Trait Documentation**: Comprehensive trait documentation with examples
- **Migration Guide**: Guide for updating existing code

## Implementation Notes

1. **Analysis Tools**:
   ```bash
   # Visualize dependencies
   cargo deps --all-deps | dot -Tpng > deps.png
   
   # Check for circular dependencies
   cargo modules dependencies --no-externs
   
   # Measure coupling
   cargo modules orphans
   ```

2. **Common Patterns**:
   - Use `Arc<dyn Trait>` for shared ownership
   - Implement `From/Into` for type conversions
   - Use newtype pattern for strong typing
   - Apply builder pattern for complex construction

3. **Testing Improvements**:
   ```rust
   // Easy testing with trait mocks
   #[cfg(test)]
   mod tests {
       use mockall::mock;
       
       mock! {
           Analyzer {}
           impl Analyzer for Analyzer {
               type Input = String;
               type Output = Report;
               fn analyze(&self, input: String) -> Result<Report>;
           }
       }
   }
   ```

## Migration and Compatibility

- Gradual migration using feature flags
- Maintain backward compatibility during transition
- Deprecate old APIs with clear migration path
- Complete migration over 2-3 release cycles