---
number: 256
title: Remove Dead Store Analysis
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-12
---

# Specification 256: Remove Dead Store Analysis

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently implements custom dead store detection as part of its data flow analysis. Dead stores are variable assignments whose values are never read before being overwritten or going out of scope.

### The Problem

The Rust compiler already provides accurate dead store detection through its built-in warnings:

```
warning: unused variable: `x`
warning: value assigned to `x` is never read
```

Debtmap's custom implementation:
1. **Produces false positives** - Fails to recognize struct field shorthand (`Foo { x }`), `@` pattern bindings, and iterator closure uses
2. **Is less accurate** - Lacks full type information and understanding of all Rust syntax patterns
3. **Requires ongoing maintenance** - Each new Rust pattern requires explicit handling
4. **Duplicates functionality** - The Rust compiler already does this better

### Recent Evidence

Analysis of `main_inner` reported 30 dead stores, but all were false positives from struct field shorthand:

```rust
Commands::Validate { path, config, ... } => {
    let validate_config = ValidateConfig {
        path,      // Reported as dead store - FALSE POSITIVE
        config,    // Reported as dead store - FALSE POSITIVE
        ...
    };
}
```

Analysis of `discover_files` reported `files` and `extensions` as dead stores despite both being clearly used.

### Affected Components

The dead store analysis touches these files:

| File | Usage |
|------|-------|
| `src/analysis/data_flow.rs` | Core dead store detection in `LivenessInfo` |
| `src/data_flow/mod.rs` | `MutationInfo.dead_stores` field, `get_dead_store_names()` |
| `src/data_flow/population.rs` | `extract_dead_stores()` function |
| `src/priority/unified_scorer.rs` | Dead store ratio in refactorability scoring |
| `src/config/scoring.rs` | `min_dead_store_ratio`, `dead_store_boost` config |
| `src/tui/results/detail_pages/data_flow.rs` | TUI display of dead stores |
| `src/tui/results/actions.rs` | Copy action includes dead stores |
| `src/io/writers/markdown/enhanced.rs` | Markdown output of dead stores |
| `src/analyzers/purity_detector.rs` | Checks if mutations are dead stores |
| `src/priority/scoring/recommendation_extended.rs` | Recommendations mention dead stores |

## Objective

Remove dead store analysis from debtmap to:

1. **Eliminate false positives** - Stop reporting incorrect dead stores that undermine user trust
2. **Reduce maintenance burden** - Remove code that requires ongoing updates for new Rust patterns
3. **Simplify the codebase** - Remove ~200 lines of complex liveness analysis code
4. **Focus on unique value** - Concentrate on metrics the Rust compiler doesn't provide (complexity, coupling, coverage)

## Requirements

### Functional Requirements

1. **Remove dead store detection logic**
   - Remove `find_dead_stores()`, `has_use_after()`, `rvalue_uses()`, `expr_kind_uses()`, `terminator_var_uses()` from `LivenessInfo`
   - Remove `dead_stores` field from `LivenessInfo` struct
   - Keep other liveness analysis (live_in, live_out) if used elsewhere

2. **Remove dead store data structures**
   - Remove `dead_stores: HashSet<String>` from `MutationInfo`
   - Remove `get_dead_store_names()` from `DataFlowGraph`
   - Remove `extract_dead_stores()` from population module

3. **Remove dead store configuration**
   - Remove `min_dead_store_ratio` config option
   - Remove `dead_store_boost` config option
   - Remove related default functions

4. **Update scoring logic**
   - Remove dead store ratio from refactorability factor calculation
   - Adjust scoring to not penalize/boost based on dead stores

5. **Update UI/output**
   - Remove dead store display from TUI data flow page
   - Remove dead store section from markdown output
   - Remove dead store from copy actions
   - Update mutation analysis display to only show live mutations and totals

6. **Update purity detection**
   - Remove dead store checking from purity analysis
   - A mutation is impure regardless of whether it's "dead"

7. **Remove ExprKind::Uses variant**
   - Remove the recently added `ExprKind::Uses(Vec<VarId>)` variant
   - This was added to fix dead store false positives; no longer needed

### Non-Functional Requirements

1. **Backward compatibility** - Config files with dead store options should not cause errors (ignore unknown fields or deprecate gracefully)
2. **Test updates** - Remove or update tests that verify dead store detection
3. **Documentation** - Update any documentation referencing dead store analysis

## Acceptance Criteria

- [ ] `LivenessInfo` struct no longer contains `dead_stores` field
- [ ] `MutationInfo` struct no longer contains `dead_stores` field
- [ ] No references to "dead store" in scoring calculations
- [ ] TUI data flow page shows mutation analysis without dead stores section
- [ ] Markdown output shows mutation analysis without dead stores section
- [ ] Config file with `min_dead_store_ratio` or `dead_store_boost` does not cause parse errors
- [ ] All existing tests pass (after updating dead store related tests)
- [ ] `cargo clippy` passes with no new warnings
- [ ] No "dead store" text appears in analysis output

## Technical Details

### Implementation Approach

**Phase 1: Remove from data structures**

```rust
// Before (data_flow/mod.rs)
pub struct MutationInfo {
    pub live_mutations: Vec<String>,
    pub total_mutations: usize,
    pub dead_stores: HashSet<String>,        // REMOVE
    pub escaping_mutations: HashSet<String>,
}

// After
pub struct MutationInfo {
    pub live_mutations: Vec<String>,
    pub total_mutations: usize,
    pub escaping_mutations: HashSet<String>,
}
```

**Phase 2: Remove from LivenessInfo**

```rust
// Before (analysis/data_flow.rs)
pub struct LivenessInfo {
    pub live_in: HashMap<BlockId, HashSet<VarId>>,
    pub live_out: HashMap<BlockId, HashSet<VarId>>,
    pub dead_stores: HashSet<VarId>,  // REMOVE
}

// After
pub struct LivenessInfo {
    pub live_in: HashMap<BlockId, HashSet<VarId>>,
    pub live_out: HashMap<BlockId, HashSet<VarId>>,
}
```

Remove these functions from `impl LivenessInfo`:
- `find_dead_stores()`
- `has_use_after()`
- `rvalue_uses()`
- `expr_kind_uses()`
- `terminator_var_uses()`

**Phase 3: Remove ExprKind::Uses**

```rust
// Before
pub enum ExprKind {
    MethodCall { ... },
    MacroCall { ... },
    Closure { ... },
    Uses(Vec<VarId>),  // REMOVE
    Other,
}

// After
pub enum ExprKind {
    MethodCall { ... },
    MacroCall { ... },
    Closure { ... },
    Other,
}
```

Revert `process_expr` fallback to not extract uses:
```rust
_ => {
    self.process_closures_in_expr(expr);
    self.current_block.push(Statement::Expr {
        expr: ExprKind::Other,
        line: None,
    });
}
```

**Phase 4: Remove from config**

```rust
// Before (config/scoring.rs)
pub struct RefactorabilityConfig {
    pub min_dead_store_ratio: f64,  // REMOVE
    pub dead_store_boost: f64,      // REMOVE
}

// After
pub struct RefactorabilityConfig {
    // Other fields remain
}
```

**Phase 5: Update scoring**

```rust
// Before (priority/unified_scorer.rs)
fn calculate_refactorability_factor(...) -> f64 {
    if let Some(info) = mutation_info {
        if info.total_mutations > 0 {
            let dead_store_ratio = info.dead_stores.len() as f64 / info.total_mutations as f64;
            if dead_store_ratio >= config.min_dead_store_ratio {
                return 1.0 + (dead_store_ratio * config.dead_store_boost);
            }
        }
    }
    1.0
}

// After - remove dead store factor entirely or return constant
fn calculate_refactorability_factor(...) -> f64 {
    1.0  // Or remove this function if not used for other purposes
}
```

**Phase 6: Update UI**

In `tui/results/detail_pages/data_flow.rs`:
- Remove the "dead stores" section rendering
- Keep "live mutations" and "total" displays

In `io/writers/markdown/enhanced.rs`:
- Remove dead store count and listing from markdown output

### Files to Modify

| File | Changes |
|------|---------|
| `src/analysis/data_flow.rs` | Remove dead store functions, ExprKind::Uses, revert process_expr |
| `src/data_flow/mod.rs` | Remove dead_stores from MutationInfo, remove get_dead_store_names |
| `src/data_flow/population.rs` | Remove extract_dead_stores function |
| `src/priority/unified_scorer.rs` | Remove dead store ratio calculation |
| `src/priority/unified_scorer/tests.rs` | Remove dead store test cases |
| `src/config/scoring.rs` | Remove dead store config options |
| `src/config/mod.rs` | Remove dead store default imports |
| `src/tui/results/detail_pages/data_flow.rs` | Remove dead store display |
| `src/tui/results/actions.rs` | Remove dead store from copy action |
| `src/io/writers/markdown/enhanced.rs` | Remove dead store from markdown |
| `src/analyzers/purity_detector.rs` | Remove dead store check |
| `src/priority/scoring/recommendation_extended.rs` | Remove dead store mentions |

### Estimated Lines Removed

| Component | Approximate Lines |
|-----------|-------------------|
| Dead store detection logic | ~100 |
| ExprKind::Uses and handling | ~30 |
| Config options | ~20 |
| Scoring logic | ~15 |
| UI display | ~30 |
| Tests | ~50 |
| **Total** | **~245 lines** |

## Dependencies

- **Prerequisites**: None
- **Affected Components**: Data flow analysis, scoring, TUI, markdown output
- **External Dependencies**: None

## Testing Strategy

### Unit Tests
- Remove `test_dead_store_translation`
- Remove `test_extract_dead_stores`
- Remove `test_calculate_refactorability_factor_high_dead_stores`
- Remove `test_match_pattern_binding_field_access_not_dead_store` (recently added)
- Update any tests that reference `dead_stores` field

### Integration Tests
- Verify analysis runs without errors after removal
- Verify TUI displays correctly without dead store section
- Verify markdown output is valid without dead store section

### Regression Tests
- Ensure mutation analysis still works (total, live mutations, escaping)
- Ensure purity detection still works without dead store checks

## Documentation Requirements

- **Code Documentation**: Remove doc comments referencing dead stores
- **User Documentation**: None (dead stores were not prominently documented)
- **Architecture Updates**: None needed

## Implementation Notes

### Order of Changes

1. Start with data structures (MutationInfo, LivenessInfo)
2. Then remove functions that use those structures
3. Then remove config options
4. Then update UI/output
5. Finally update tests

### Potential Issues

1. **Serde deserialization** - Old config files may have dead store options. Use `#[serde(default)]` or ignore unknown fields to handle gracefully.

2. **Mutation count semantics** - After removal, `total_mutations` may need clarification. It currently represents all detected mutations; without dead store filtering, the meaning is clearer.

3. **Live mutations list** - Currently filtered by dead stores. After removal, `live_mutations` should be renamed to just `mutations` or keep the name but update semantics.

## Migration and Compatibility

### Config File Compatibility

Add `#[serde(skip)]` or use `#[serde(default)]` to gracefully handle old config files:

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields = false)]  // Allow old dead_store fields
pub struct RefactorabilityConfig {
    // ...
}
```

Or explicitly mark as deprecated:
```rust
#[serde(default, skip_serializing)]
#[deprecated(note = "Dead store analysis removed in v0.10")]
pub dead_store_boost: f64,
```

### Breaking Changes

- Config files explicitly setting `min_dead_store_ratio` or `dead_store_boost` will have those values ignored
- Output format changes (no dead store section)
- API changes if `MutationInfo` is part of public API

## Future Considerations

If dead store analysis is needed in the future:

1. **Integrate with rustc** - Parse `cargo check --message-format=json` output for compiler warnings
2. **Language-specific** - Only implement for languages without compiler warnings (Python, JS)
3. **Rely on LSP** - Use language server diagnostics instead of custom analysis
