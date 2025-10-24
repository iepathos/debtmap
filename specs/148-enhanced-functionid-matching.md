---
number: 148
title: Enhanced FunctionId Matching Strategy
category: optimization
priority: high
status: draft
dependencies: [146]
created: 2025-10-24
---

# Specification 148: Enhanced FunctionId Matching Strategy

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 146 (Cross-Module Call Resolution Enhancement)

## Context

The current `FunctionId` structure uses a strict equality check across all fields (file path, function name, line number, and module_path). This works well for exact matches but causes issues when:

1. **Line number mismatches**: Function defined at line 160, but lookup uses estimated line
2. **File path variations**: Absolute vs relative paths, canonicalized vs raw paths
3. **Module path inconsistencies**: Same function, different module path representations
4. **Generic instantiations**: `foo<T>` vs `foo<String>` are different FunctionIds

This strict matching causes legitimate function calls to fail resolution, leading to inaccurate caller/callee counts.

**Current Implementation:**
```rust
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
    pub module_path: String,
}
```

All four fields must match exactly for equality, which is too strict for fuzzy resolution scenarios.

## Objective

Implement a flexible FunctionId matching strategy that supports both strict equality (for disambiguation) and fuzzy matching (for resolution), improving call graph accuracy while maintaining correctness.

## Requirements

### Functional Requirements

1. **Multiple Matching Strategies**
   - Exact match: All fields must match (current behavior)
   - Fuzzy match: Matches on name + file, ignoring line/module_path
   - Semantic match: Matches considering generics, paths, and context
   - Configurable strategy per use case

2. **FunctionId Normalization**
   - Canonicalize file paths for consistent comparison
   - Normalize generic type parameters
   - Strip module path redundancies
   - Handle path separators consistently (Unix vs Windows)

3. **Lookup Strategies**
   - Primary index: Exact FunctionId lookup (fast)
   - Secondary index: Name + file lookup (fuzzy)
   - Tertiary index: Name only lookup (cross-file)
   - Strategy selection based on context and confidence

4. **Generic Function Support**
   - Store generic functions with normalized names
   - Match generic calls to base definitions
   - Handle turbofish syntax (`::<Type>`)
   - Support trait bounds and where clauses

### Non-Functional Requirements

1. **Performance**: Fuzzy matching overhead < 5ms per lookup
2. **Correctness**: No false positive matches between distinct functions
3. **Maintainability**: Clear separation between matching strategies
4. **Backward Compatibility**: Existing code continues to work

## Acceptance Criteria

- [ ] FunctionId supports both exact and fuzzy matching modes
- [ ] CallGraph maintains three indexes (exact, fuzzy, name-only)
- [ ] Lookups try exact first, then fuzzy, then name-only
- [ ] Generic functions match correctly regardless of type parameters
- [ ] File path variations (relative/absolute) resolve to same function
- [ ] Line number differences don't prevent fuzzy matches
- [ ] No false positives when distinct functions have same simple name
- [ ] Performance overhead < 5ms per fuzzy lookup
- [ ] All existing tests pass with enhanced matching
- [ ] New tests cover fuzzy matching edge cases

## Technical Details

### Implementation Approach

1. **Phase 1: FunctionId Extensions**
   - Add `FunctionIdKey` enum for different match levels
   - Implement `Hash` and `Eq` for each key type
   - Add normalization methods to FunctionId
   - Maintain backward compatible equality

2. **Phase 2: Multi-Index CallGraph**
   - Add fuzzy_index: `HashMap<FuzzyKey, Vec<FunctionId>>`
   - Add name_index: `HashMap<String, Vec<FunctionId>>`
   - Update `add_function` to populate all indexes
   - Update `get_callers`/`get_callees` to try all strategies

3. **Phase 3: Lookup Strategy Chain**
   - Implement `FunctionLookup` trait with multiple strategies
   - Add `ExactLookup`, `FuzzyLookup`, `NameLookup` implementations
   - Chain strategies with short-circuit on success
   - Add confidence scores to disambiguation

### Architecture Changes

**File**: `src/priority/call_graph/types.rs`
- Add `FunctionIdKey` enum
- Add `FuzzyFunctionId` and `SimpleFunctionId` structs
- Implement multiple `Hash`/`Eq` implementations
- Add normalization methods

**File**: `src/priority/call_graph/graph_operations.rs`
- Add `fuzzy_index` and `name_index` fields to CallGraph
- Update `add_function` to populate all indexes
- Add `get_function_fuzzy` and `get_function_by_name` methods
- Implement disambiguation logic for multiple matches

**File**: `src/priority/call_graph/mod.rs`
- Add `FunctionLookup` trait
- Implement concrete lookup strategies
- Add `LookupChain` that tries strategies in order

### Data Structures

```rust
/// Different matching strategies for FunctionId
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStrategy {
    /// All fields must match exactly
    Exact,
    /// Name and normalized file must match (ignores line/module_path)
    Fuzzy,
    /// Only function name must match (returns multiple candidates)
    NameOnly,
}

/// Key for exact lookups (current behavior)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ExactFunctionKey {
    file: PathBuf,
    name: String,
    line: usize,
    module_path: String,
}

/// Key for fuzzy lookups (name + file, ignores line/module)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FuzzyFunctionKey {
    canonical_file: PathBuf,  // Canonicalized path
    normalized_name: String,   // Name without generic parameters
}

/// Key for name-only lookups
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SimpleFunctionKey {
    normalized_name: String,
}

impl FunctionId {
    /// Get exact key (all fields)
    pub fn exact_key(&self) -> ExactFunctionKey;

    /// Get fuzzy key (name + file only)
    pub fn fuzzy_key(&self) -> FuzzyFunctionKey;

    /// Get simple key (name only)
    pub fn simple_key(&self) -> SimpleFunctionKey;

    /// Normalize function name (strip generics, whitespace)
    pub fn normalize_name(name: &str) -> String;

    /// Canonicalize file path for consistent matching
    pub fn canonicalize_path(path: &Path) -> PathBuf;
}
```

### Enhanced CallGraph Structure

```rust
pub struct CallGraph {
    // Existing exact index
    nodes: HashMap<FunctionId, FunctionNode>,

    // New: Fuzzy matching index (name + file)
    fuzzy_index: HashMap<FuzzyFunctionKey, Vec<FunctionId>>,

    // New: Name-only index (cross-file matching)
    name_index: HashMap<String, Vec<FunctionId>>,

    // Existing fields...
    edges: Vector<FunctionCall>,
    caller_index: HashMap<FunctionId, HashSet<FunctionId>>,
    callee_index: HashMap<FunctionId, HashSet<FunctionId>>,
}

impl CallGraph {
    /// Lookup function with fallback strategies
    pub fn find_function(&self, query: &FunctionId) -> Option<FunctionId> {
        // 1. Try exact match
        if self.nodes.contains_key(query) {
            return Some(query.clone());
        }

        // 2. Try fuzzy match (name + file)
        if let Some(candidates) = self.fuzzy_index.get(&query.fuzzy_key()) {
            if candidates.len() == 1 {
                return Some(candidates[0].clone());
            }
            // Multiple candidates: try to disambiguate
            return self.disambiguate_fuzzy(candidates, query);
        }

        // 3. Try name-only match (cross-file)
        if let Some(candidates) = self.name_index.get(&query.name) {
            return self.disambiguate_name_only(candidates, query);
        }

        None
    }

    /// Disambiguate between multiple fuzzy matches
    fn disambiguate_fuzzy(
        &self,
        candidates: &[FunctionId],
        query: &FunctionId,
    ) -> Option<FunctionId>;

    /// Disambiguate between multiple name-only matches
    fn disambiguate_name_only(
        &self,
        candidates: &[FunctionId],
        query: &FunctionId,
    ) -> Option<FunctionId>;
}
```

### APIs and Interfaces

```rust
/// Trait for function lookup strategies
pub trait FunctionLookup {
    /// Attempt to find a function matching the query
    fn find(&self, call_graph: &CallGraph, query: &FunctionId) -> Option<FunctionId>;

    /// Get confidence score for this lookup strategy (0.0 - 1.0)
    fn confidence(&self) -> f32;
}

/// Exact match lookup
pub struct ExactLookup;

impl FunctionLookup for ExactLookup {
    fn find(&self, call_graph: &CallGraph, query: &FunctionId) -> Option<FunctionId> {
        call_graph.nodes.get(query).map(|n| n.id.clone())
    }

    fn confidence(&self) -> f32 { 1.0 }
}

/// Fuzzy match lookup (name + file)
pub struct FuzzyLookup {
    prefer_same_module: bool,
}

impl FunctionLookup for FuzzyLookup {
    fn find(&self, call_graph: &CallGraph, query: &FunctionId) -> Option<FunctionId>;
    fn confidence(&self) -> f32 { 0.8 }
}

/// Name-only lookup (cross-file)
pub struct NameOnlyLookup {
    prefer_same_crate: bool,
}

impl FunctionLookup for NameOnlyLookup {
    fn find(&self, call_graph: &CallGraph, query: &FunctionId) -> Option<FunctionId>;
    fn confidence(&self) -> f32 { 0.5 }
}

/// Chain of lookup strategies
pub struct LookupChain {
    strategies: Vec<Box<dyn FunctionLookup>>,
}

impl LookupChain {
    pub fn new() -> Self {
        Self {
            strategies: vec![
                Box::new(ExactLookup),
                Box::new(FuzzyLookup { prefer_same_module: true }),
                Box::new(NameOnlyLookup { prefer_same_crate: true }),
            ],
        }
    }

    pub fn find(&self, call_graph: &CallGraph, query: &FunctionId) -> Option<(FunctionId, f32)>;
}
```

## Dependencies

- **Prerequisites**: Spec 146 (Cross-Module Call Resolution) - Provides import context for disambiguation
- **Affected Components**:
  - `src/priority/call_graph/types.rs` (FunctionId structure)
  - `src/priority/call_graph/graph_operations.rs` (lookup methods)
  - `src/analyzers/call_graph/call_resolution.rs` (uses new lookup methods)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Normalization Tests** (`src/priority/call_graph/types.rs`)
   ```rust
   #[test]
   fn test_normalize_generic_name() {
       assert_eq!(FunctionId::normalize_name("foo<T>"), "foo");
       assert_eq!(FunctionId::normalize_name("bar<A, B>"), "bar");
       assert_eq!(FunctionId::normalize_name("baz"), "baz");
   }

   #[test]
   fn test_canonicalize_path() {
       // Test relative vs absolute paths
       // Test with symlinks
       // Test Windows vs Unix separators
   }

   #[test]
   fn test_fuzzy_key_equality() {
       // Same name + file, different lines → equal keys
       // Different files → different keys
   }
   ```

2. **Lookup Strategy Tests** (`src/priority/call_graph/graph_operations.rs`)
   ```rust
   #[test]
   fn test_exact_lookup() {
       // Exact match succeeds
       // Near match fails
   }

   #[test]
   fn test_fuzzy_lookup() {
       // Same name + file, different line → succeeds
       // Multiple candidates → returns most confident
   }

   #[test]
   fn test_name_only_lookup() {
       // Cross-file lookup succeeds
       // Ambiguous case → returns None or most likely
   }

   #[test]
   fn test_lookup_chain() {
       // Tries exact, then fuzzy, then name-only
       // Short-circuits on first success
   }
   ```

3. **Disambiguation Tests**
   ```rust
   #[test]
   fn test_disambiguate_same_module_preferred() {
       // Multiple matches, same-module wins
   }

   #[test]
   fn test_disambiguate_line_proximity() {
       // Multiple matches, closest line number wins
   }
   ```

### Integration Tests

1. **Generic Function Resolution** (`tests/call_graph_generic_matching_test.rs`)
   - Define generic function: `fn foo<T>()`
   - Call with type parameter: `foo::<String>()`
   - Verify call is correctly attributed to base function

2. **Cross-Module Fuzzy Matching** (`tests/call_graph_fuzzy_cross_module_test.rs`)
   - Define function in module A
   - Call from module B with slightly different context
   - Verify fuzzy matching resolves correctly

3. **Path Variation Test** (`tests/call_graph_path_variations_test.rs`)
   - Test with relative paths
   - Test with absolute paths
   - Test with canonicalized paths
   - Verify all resolve to same function

### Performance Tests

1. **Benchmark Lookup Time** (`benches/call_graph_bench.rs`)
   - Measure exact lookup time (baseline)
   - Measure fuzzy lookup time (should be < 5ms overhead)
   - Measure name-only lookup time
   - Profile with large call graphs (1000+ functions)

## Documentation Requirements

### Code Documentation

- Document each matching strategy and when to use it
- Explain normalization rules (generics, paths)
- Provide examples of disambiguation logic
- Document confidence scoring system

### User Documentation

No user-facing documentation needed - this is an internal optimization.

### Architecture Updates

**ARCHITECTURE.md sections to update:**
- Call Graph → FunctionId Matching Strategies
- Performance → Multi-Index Lookup Architecture
- Data Structures → FunctionId Keys and Indexes

## Implementation Notes

### Normalization Rules

**Function Names:**
- Strip all generic parameters: `foo<T, U>` → `foo`
- Remove whitespace: `foo < T >` → `foo`
- Preserve namespacing: `mod::foo` → `mod::foo`
- Normalize method syntax: `Type::method` stays as-is

**File Paths:**
- Canonicalize to absolute paths
- Resolve symlinks
- Normalize separators (Unix/Windows)
- Strip workspace prefix for relative comparison

### Disambiguation Strategy

When multiple candidates match, prefer in order:
1. **Exact match** on all fields (highest confidence)
2. **Same file** as caller (very high confidence)
3. **Same module** as caller (high confidence)
4. **Closest line number** to expected location (medium confidence)
5. **Most common** (if function is called from many places) (low confidence)

### Edge Cases

- **Recursive calls**: FunctionId matches itself exactly
- **Overloaded functions**: Rust doesn't allow, but handle gracefully
- **Trait methods**: May have multiple implementations, disambiguate by receiver type
- **Macro-generated code**: Line numbers may be synthetic

### Performance Optimization

- Cache normalization results (canonicalized paths, stripped names)
- Use lazy evaluation for expensive canonicalization
- Short-circuit on exact matches (most common case)
- Limit name-only candidates to reasonable number (e.g., < 10)

## Migration and Compatibility

### Breaking Changes

None - existing FunctionId equality unchanged. New lookup methods are additive.

### Backward Compatibility

- Existing exact matches work exactly as before
- New fuzzy matching only used when exact match fails
- CallGraph serialization format unchanged (indexes rebuilt on load)

### Migration Path

1. Add new index fields to CallGraph (with defaults)
2. Populate indexes during existing add_function calls
3. Update resolution code to use new find_function method
4. Test extensively with existing codebases
5. Deploy incrementally with feature flag

## Success Metrics

- **Primary**: Functions with known callers resolve correctly even with line mismatches
- **Secondary**: Fuzzy matching adds < 5ms lookup overhead
- **Tertiary**: No false positive matches in test suite
- **User-Facing**: Improved caller/callee accuracy in real-world codebases

## Related Work

- Spec 146: Cross-Module Call Resolution (uses these matching strategies)
- Spec 147: Caller/Callee Output (displays more accurate data)
- Spec 149: Call Graph Debug Tools (helps diagnose matching issues)
