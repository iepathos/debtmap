---
number: 257
title: Simplify Mutation Analysis to Binary Signals
category: optimization
priority: medium
status: draft
dependencies: [256]
created: 2025-12-12
---

# Specification 257: Simplify Mutation Analysis to Binary Signals

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 256 (Remove Dead Store Analysis)

## Context

Debtmap's mutation analysis currently attempts to count mutations precisely:

```rust
pub struct MutationInfo {
    pub live_mutations: Vec<String>,      // List of mutation names
    pub total_mutations: usize,           // Exact count
    pub dead_stores: HashSet<String>,     // Being removed in spec 256
    pub escaping_mutations: HashSet<String>,
}
```

### The Problem

The same pattern recognition issues causing dead store false positives affect mutation counts:

| Pattern | Issue |
|---------|-------|
| Struct field shorthand `Foo { x }` | `x` not recognized as used |
| `@` pattern bindings `cmd @ Commands::...` | `cmd` not tracked |
| Iterator closure captures | Variables in closures missed |
| Complex match arms | Pattern bindings not fully tracked |

**Evidence**: Analysis of `main_inner` reported only 2 live mutations (`full_args`), but the function clearly has many more variable bindings and mutations.

### Why Counts Are Hard

Accurate mutation counting requires:
1. Complete pattern matching for all Rust syntax
2. Tracking through control flow
3. Handling closures and async
4. Understanding macro expansions

The Rust compiler does this perfectly because it has full type information. Debtmap's AST-based analysis will always be incomplete.

### Why Binary Signals Are Better

| Metric Type | Accuracy | Actionability |
|-------------|----------|---------------|
| "5 mutations" | Low (may be wrong) | Low (what's the threshold?) |
| "has mutations" | High (easy to detect) | High (pure vs impure) |
| "mutations escape" | Medium | High (side effects) |

Binary signals are:
- **Harder to get wrong** - Detecting "any mutation" is easier than counting all
- **More actionable** - "This function is pure" vs "This function has 3 mutations"
- **Less misleading** - Wrong counts erode user trust

## Objective

Simplify mutation analysis to use binary signals instead of counts:

1. **Replace counts with booleans** - `has_mutations`, `has_escaping_mutations`
2. **Keep best-effort context** - Still list detected mutations for display
3. **Remove accuracy expectations** - Don't claim precise counts
4. **Focus on actionable classification** - Pure / Has Side Effects / Has I/O

## Requirements

### Functional Requirements

1. **Simplify MutationInfo struct**
   ```rust
   // Before
   pub struct MutationInfo {
       pub live_mutations: Vec<String>,
       pub total_mutations: usize,
       pub dead_stores: HashSet<String>,  // Removed by spec 256
       pub escaping_mutations: HashSet<String>,
   }

   // After
   pub struct MutationInfo {
       pub has_mutations: bool,
       pub has_escaping_mutations: bool,
       pub detected_mutations: Vec<String>,  // Best-effort list for context
       pub escaping_vars: Vec<String>,       // Best-effort list for context
   }
   ```

2. **Update mutation detection**
   - Set `has_mutations = true` if ANY mutation detected
   - Set `has_escaping_mutations = true` if ANY mutation escapes
   - Populate lists on best-effort basis (not claiming completeness)

3. **Update purity classification**
   - Use `has_mutations` and `has_escaping_mutations` for classification
   - Don't rely on counts for scoring

4. **Update UI/output**
   - Display "Has mutations: yes/no" instead of counts
   - Show detected mutations as context, not exhaustive list
   - Update TUI data flow page
   - Update markdown output

5. **Update scoring**
   - Remove any scoring based on mutation counts
   - Use binary signals for refactorability assessment

### Non-Functional Requirements

1. **Backward compatibility** - API consumers should handle struct changes
2. **Performance** - Should be faster (less precise tracking needed)
3. **Maintainability** - Simpler code, fewer edge cases

## Acceptance Criteria

- [ ] `MutationInfo` uses boolean fields instead of counts
- [ ] Purity classification uses binary signals
- [ ] TUI shows "Has mutations: yes/no" format
- [ ] Markdown output shows binary signals
- [ ] No scoring calculations use mutation counts
- [ ] All tests pass after updates
- [ ] No false claims of "X mutations detected"

## Technical Details

### Implementation Approach

**Phase 1: Update MutationInfo struct**

```rust
// src/data_flow/mod.rs

/// Mutation analysis information for a function.
/// Uses binary signals for reliability - precise counts are not guaranteed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationInfo {
    /// Whether any mutations were detected in the function
    pub has_mutations: bool,

    /// Whether any mutations escape the function (affect return or external state)
    pub has_escaping_mutations: bool,

    /// Best-effort list of detected mutations (may be incomplete)
    pub detected_mutations: Vec<String>,

    /// Best-effort list of escaping variables (may be incomplete)
    pub escaping_vars: Vec<String>,
}

impl MutationInfo {
    pub fn none() -> Self {
        Self {
            has_mutations: false,
            has_escaping_mutations: false,
            detected_mutations: Vec::new(),
            escaping_vars: Vec::new(),
        }
    }

    pub fn is_pure(&self) -> bool {
        !self.has_mutations && !self.has_escaping_mutations
    }
}
```

**Phase 2: Update population logic**

```rust
// src/data_flow/population.rs

pub fn populate_mutation_info(...) {
    let detected = extract_detected_mutations(...);  // Best effort
    let escaping = extract_escaping_vars(...);       // Best effort

    let mutation_info = MutationInfo {
        has_mutations: !detected.is_empty() || purity.has_mutations,
        has_escaping_mutations: !escaping.is_empty() || purity.has_escaping,
        detected_mutations: detected,
        escaping_vars: escaping,
    };

    data_flow.set_mutation_info(func_id, mutation_info);
}
```

**Phase 3: Update TUI display**

```rust
// src/tui/results/detail_pages/data_flow.rs

// Before
add_label_value(&mut lines, "total", mutation_info.total_mutations.to_string(), ...);
add_label_value(&mut lines, "live", mutation_info.live_mutations.len().to_string(), ...);

// After
add_label_value(&mut lines, "has mutations",
    if mutation_info.has_mutations { "yes" } else { "no" }, ...);
add_label_value(&mut lines, "escaping",
    if mutation_info.has_escaping_mutations { "yes" } else { "no" }, ...);

// Show detected mutations as context (not as count)
if !mutation_info.detected_mutations.is_empty() {
    add_section_header(&mut lines, "detected mutations (partial)", theme);
    for mutation in &mutation_info.detected_mutations {
        // Display each
    }
}
```

**Phase 4: Update scoring**

```rust
// src/priority/unified_scorer.rs

// Remove count-based calculations
fn calculate_refactorability_factor(...) -> f64 {
    if let Some(info) = mutation_info {
        // Binary check instead of ratio
        if info.has_escaping_mutations {
            return 0.9;  // Slightly harder to refactor
        }
        if info.has_mutations {
            return 1.0;  // Neutral
        }
        return 1.1;  // Pure functions slightly easier
    }
    1.0
}
```

### Files to Modify

| File | Changes |
|------|---------|
| `src/data_flow/mod.rs` | Update `MutationInfo` struct |
| `src/data_flow/population.rs` | Update population logic |
| `src/tui/results/detail_pages/data_flow.rs` | Update display |
| `src/tui/results/actions.rs` | Update copy action |
| `src/io/writers/markdown/enhanced.rs` | Update markdown output |
| `src/priority/unified_scorer.rs` | Update scoring logic |
| `src/analyzers/purity_detector.rs` | Use binary signals |
| Tests | Update test expectations |

### Display Format Changes

**Before (TUI)**:
```
mutation analysis
  total                     5
  live                      3
  dead stores               2    ← Removed by spec 256

live mutations
  x
  y
  z
```

**After (TUI)**:
```
mutation analysis
  has mutations             yes
  escaping                  yes

detected mutations (best-effort)
  x
  y
  z
```

**Before (Markdown)**:
```markdown
**Mutations**: 5 total, 3 live, 2 dead stores
```

**After (Markdown)**:
```markdown
**Mutations**: detected (some may escape)
```

## Dependencies

- **Prerequisites**: Spec 256 (Remove Dead Store Analysis) - removes `dead_stores` field
- **Affected Components**: Data flow analysis, TUI, markdown output, scoring
- **External Dependencies**: None

## Testing Strategy

### Unit Tests
- Test `MutationInfo::is_pure()` method
- Test population with various function types
- Test display formatting

### Integration Tests
- Verify TUI displays correctly with new format
- Verify markdown output is valid
- Verify scoring uses binary signals correctly

### Regression Tests
- Ensure purity detection still works
- Ensure functions previously marked pure remain pure

## Documentation Requirements

- **Code Documentation**: Update doc comments on `MutationInfo`
- **User Documentation**: Update any docs explaining mutation analysis
- **Architecture Updates**: None needed

## Implementation Notes

### Why "Best-Effort" Lists

The `detected_mutations` and `escaping_vars` lists are labeled "best-effort" because:
1. Pattern recognition is incomplete
2. Macro expansions are not fully analyzed
3. Complex control flow may hide mutations

By being explicit about limitations, we:
- Set correct user expectations
- Avoid misleading precision
- Allow incomplete lists without shame

### Purity Classification Priority

With binary signals, purity classification becomes cleaner:

| has_mutations | has_escaping | has_io | Classification |
|---------------|--------------|--------|----------------|
| false | false | false | Pure |
| true | false | false | Local mutations only |
| true | true | false | Has side effects |
| * | * | true | Has I/O |

### Future Improvements

If precise counts are needed later:
1. Integrate with rust-analyzer for accurate analysis
2. Parse compiler diagnostics for mutation warnings
3. Use MIR-level analysis (much more accurate than AST)

## Migration and Compatibility

### API Changes

```rust
// Old API
let count = mutation_info.total_mutations;
let live = mutation_info.live_mutations.len();

// New API
let has_any = mutation_info.has_mutations;
let escapes = mutation_info.has_escaping_mutations;
let is_pure = mutation_info.is_pure();
```

### Serialization

Update serde for new struct shape. Old serialized data won't deserialize correctly - this is acceptable as mutation info is computed, not persisted.
