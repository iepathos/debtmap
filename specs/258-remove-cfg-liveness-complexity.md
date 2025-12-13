---
number: 258
title: Remove CFG Liveness Analysis Complexity
category: optimization
priority: low
status: draft
dependencies: [256, 257]
created: 2025-12-12
---

# Specification 258: Remove CFG Liveness Analysis Complexity

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: Spec 256 (Remove Dead Store Analysis), Spec 257 (Simplify Mutation Analysis)

## Context

Debtmap's data flow analysis includes a sophisticated Control Flow Graph (CFG) based liveness analysis. This infrastructure was primarily built to support dead store detection.

### Current CFG Infrastructure

The `src/analysis/data_flow.rs` file contains ~1500 lines of CFG-related code:

| Component | Lines (approx) | Purpose |
|-----------|----------------|---------|
| `ControlFlowGraph` struct | 50 | CFG data structure |
| `CfgBuilder` | 400 | Builds CFG from AST |
| `LivenessInfo` | 300 | Liveness analysis |
| `ReachingDefinitions` | 200 | Def-use analysis |
| `EscapeAnalysis` | 150 | Escape detection |
| `TaintAnalysis` | 200 | Taint propagation |
| CFG tests | 500+ | Test coverage |

### After Specs 256 and 257

With dead stores removed (256) and mutation counts simplified to binary signals (257):

| Component | Still Needed? |
|-----------|---------------|
| CFG building | Partial - only for basic block structure |
| `LivenessInfo.dead_stores` | No - removed |
| `LivenessInfo.live_in/live_out` | Questionable - only used for dead stores |
| `find_dead_stores()` + helpers | No - removed |
| `ReachingDefinitions` | Partial - some uses remain |
| `EscapeAnalysis` | Yes - but could simplify |
| `TaintAnalysis` | Yes - valuable for purity |

### The Opportunity

Much of the CFG complexity exists to compute precise variable liveness. With binary signals, we can simplify:

1. **Escape detection** - Can use simpler AST-based analysis
2. **Mutation detection** - Can use simpler AST visitor
3. **I/O detection** - Already AST-based (works well)
4. **Purity classification** - Can combine simpler signals

## Objective

Simplify or remove CFG liveness analysis after specs 256 and 257 are complete:

1. **Remove unused liveness infrastructure** - `live_in`, `live_out` if not needed
2. **Simplify escape analysis** - AST-based instead of CFG-based where possible
3. **Reduce code complexity** - Target ~500 lines removal
4. **Maintain valuable analysis** - Keep taint analysis, I/O detection, purity classification

## Requirements

### Functional Requirements

1. **Audit liveness usage**
   - Identify all uses of `LivenessInfo.live_in` and `live_out`
   - Determine if they're still needed after spec 256
   - Remove if not used

2. **Simplify escape analysis**
   - Current: CFG-based backward dataflow
   - Target: AST visitor that checks return statements and field assignments
   - Keep: Closure capture tracking (valuable)

3. **Preserve valuable analyses**
   - Keep `TaintAnalysis` - tracks data dependencies
   - Keep I/O detection - AST-based, works well
   - Keep purity classification - uses taint + I/O

4. **Remove unused CFG infrastructure**
   - Remove liveness analysis if not needed
   - Simplify `CfgBuilder` if full CFG not needed
   - Remove helper functions for dead store detection

### Non-Functional Requirements

1. **Performance** - Analysis should be faster with less work
2. **Maintainability** - Less code = fewer bugs
3. **Accuracy** - Simpler analysis can be more reliable

## Acceptance Criteria

- [ ] `LivenessInfo` struct simplified or removed
- [ ] Unused CFG building code removed
- [ ] Escape analysis works correctly with simpler implementation
- [ ] I/O detection unchanged (already works well)
- [ ] Purity classification unchanged
- [ ] At least 300 lines of code removed
- [ ] All remaining tests pass
- [ ] No regression in purity/I/O detection accuracy

## Technical Details

### Implementation Approach

**Phase 1: Audit current usage**

Search for uses of liveness analysis:
```bash
grep -r "live_in\|live_out\|LivenessInfo" src/
```

Document which components actually use liveness data after specs 256/257.

**Phase 2: Simplify LivenessInfo**

```rust
// Before (after spec 256)
pub struct LivenessInfo {
    pub live_in: HashMap<BlockId, HashSet<VarId>>,
    pub live_out: HashMap<BlockId, HashSet<VarId>>,
    // dead_stores removed by spec 256
}

// After - if only escape analysis needs it
pub struct LivenessInfo {
    // May be removed entirely if escape analysis uses simpler approach
}
```

**Phase 3: Simplify escape analysis**

```rust
// Before: CFG-based
impl EscapeAnalysis {
    pub fn analyze(cfg: &ControlFlowGraph) -> Self {
        // Complex backward dataflow analysis
        // Tracks which variables reach return statements
    }
}

// After: AST-based visitor
pub fn find_escaping_vars(block: &Block) -> HashSet<String> {
    let mut visitor = EscapeVisitor::new();
    visitor.visit_block(block);
    visitor.escaping_vars
}

struct EscapeVisitor {
    escaping_vars: HashSet<String>,
    // Track variables that:
    // 1. Appear in return statements
    // 2. Are assigned to struct fields
    // 3. Are passed to functions that store them
}
```

**Phase 4: Remove unused CFG code**

Components to potentially remove:
- `compute_use_def()` - only used for liveness
- `add_rvalue_uses()`, `add_expr_uses()` - only used for liveness
- `get_successors()` - only used for liveness
- Complex match arm processing - if CFG not needed

**Phase 5: Simplify CfgBuilder**

If full CFG is still needed for some analyses:
```rust
// Keep only what's needed
struct SimplifiedCfg {
    statements: Vec<Statement>,
    has_branches: bool,
    return_vars: Vec<VarId>,
}
```

### Files to Modify

| File | Changes |
|------|---------|
| `src/analysis/data_flow.rs` | Major simplification |
| `src/data_flow/mod.rs` | Update to use simplified analysis |
| `src/data_flow/population.rs` | Update population logic |
| `src/analyzers/purity_detector.rs` | Use simplified escape info |

### Estimated Code Reduction

| Component | Current Lines | After | Reduction |
|-----------|---------------|-------|-----------|
| `LivenessInfo` | 300 | 0-50 | 250-300 |
| `CfgBuilder` (if simplified) | 400 | 200 | 200 |
| Helper functions | 100 | 0 | 100 |
| Tests for removed code | 200 | 0 | 200 |
| **Total** | 1000 | 200-250 | **~700-800** |

### What to Keep

| Component | Reason |
|-----------|--------|
| `TaintAnalysis` | Tracks return value dependencies - valuable |
| I/O detection | AST-based, works well, unique value |
| Closure capture tracking | Helps understand hidden dependencies |
| Basic statement extraction | Needed for mutation detection |

### Risk Assessment

| Risk | Mitigation |
|------|------------|
| Escape analysis less accurate | Accept for simplicity; AST-based catches most cases |
| Losing valuable analysis | Audit before removing; keep if used |
| Breaking existing features | Comprehensive testing after changes |

## Dependencies

- **Prerequisites**:
  - Spec 256 (removes dead store usage of liveness)
  - Spec 257 (removes count-based mutation tracking)
- **Affected Components**: Data flow analysis
- **External Dependencies**: None

## Testing Strategy

### Before Implementation
- Document current behavior of escape analysis
- Create baseline tests for purity detection
- Identify edge cases that CFG handles

### Unit Tests
- Test simplified escape detection
- Test purity classification with new implementation
- Ensure I/O detection unchanged

### Integration Tests
- Analyze real codebase with simplified analysis
- Compare purity results before/after
- Verify no regression in useful signals

### Performance Tests
- Measure analysis time before/after
- Should be faster with less work

## Documentation Requirements

- **Code Documentation**: Update architecture comments
- **User Documentation**: None (internal change)
- **Architecture Updates**: Update data flow analysis docs

## Implementation Notes

### When to Implement

This spec should be implemented AFTER specs 256 and 257 are complete and stable. The sequence:

1. Spec 256: Remove dead stores → Clears primary use of liveness
2. Spec 257: Simplify mutations → Removes count-based analysis
3. Spec 258: Remove CFG complexity → Cleans up unused infrastructure

### Incremental Approach

Don't remove everything at once:

1. First: Remove definitely-unused code (dead store helpers)
2. Then: Audit remaining usage
3. Then: Simplify escape analysis
4. Finally: Remove unused CFG infrastructure

### Fallback Plan

If simplified analysis proves inadequate:
- Keep CFG infrastructure but simplify
- Use CFG for escape analysis only
- Document why complexity is necessary

## Migration and Compatibility

### Internal API Changes

Components using `LivenessInfo` will need updates:
```rust
// Before
let liveness = LivenessInfo::analyze(&cfg);
let escaping = compute_escape(&cfg, &liveness);

// After
let escaping = find_escaping_vars(&block);
```

### No External Impact

This is internal refactoring - no user-facing changes beyond those in specs 256/257.

## Future Considerations

### If Precise Analysis Needed Later

Options for more accurate analysis without reimplementing:
1. **rust-analyzer integration** - Use existing precise analysis
2. **MIR-based analysis** - Compiler's intermediate representation
3. **cargo-check diagnostics** - Parse compiler output

### Alternative Architectures

Consider separating concerns:
```
AST Analysis (simple, fast)
├── I/O Detection
├── Mutation Detection (binary)
└── Basic Escape Detection

Optional: Precise Analysis (when needed)
├── rust-analyzer queries
└── Compiler diagnostic parsing
```

This keeps debtmap simple while allowing precision when needed.
