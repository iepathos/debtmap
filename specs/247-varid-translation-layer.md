---
number: 247
title: VarId to Variable Name Translation Layer
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-06
---

# Specification 247: VarId to Variable Name Translation Layer

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None (independent of specs 245-246, complements them)

## Context

The CFG-based data flow analysis (implemented in `src/analysis/data_flow.rs`) performs sophisticated analysis including:
- Liveness analysis (detecting dead stores)
- Escape analysis (tracking which variables affect return values)
- Taint analysis (tracking mutation propagation)

However, this rich analysis data is **invisible to users** because:

1. **Variables stored as VarIds**: The CFG uses numeric IDs (`VarId { name_id: u32, version: u32 }`) for efficiency
2. **Name mapping not preserved**: The `var_names: Vec<String>` mapping exists in the `ControlFlowGraph` but is **not stored** alongside the analysis results
3. **Intentionally empty fields**: Functions `extract_dead_stores` and `extract_escaping_mutations` in `src/data_flow/population.rs:59-77` return empty sets with comments explaining the data exists but can't be translated

**Current implementation**:
```rust
fn extract_dead_stores(_purity: &PurityAnalysis) -> HashSet<String> {
    // Return empty set - dead stores are available as VarIds via cfg_analysis
    HashSet::new()
}
```

**Result**: TUI data flow page shows empty lists for dead stores and escaping mutations, even though the data exists.

## Objective

Create a translation layer that preserves the CFG's `var_names` mapping alongside `DataFlowAnalysis` results, enabling conversion of `VarId` numeric IDs back to human-readable variable names for display in the TUI and reports.

## Requirements

### Functional Requirements

- **FR1**: Store var_names alongside DataFlowAnalysis
  - New wrapper struct `CfgAnalysisWithContext` containing both analysis and var_names
  - Modify `DataFlowGraph` to store this wrapper instead of raw `DataFlowAnalysis`

- **FR2**: Translate dead stores to variable names
  - `get_dead_store_names(func_id) -> Vec<String>`
  - Convert `VarId` set to variable names using stored mapping

- **FR3**: Translate escaping variables to names
  - `get_escaping_var_names(func_id) -> Vec<String>`
  - Convert escape analysis VarIds to readable names

- **FR4**: Translate return dependencies to names
  - `get_return_dependency_names(func_id) -> Vec<String>`
  - Show which variables affect the return value

- **FR5**: Translate tainted variables to names
  - `get_tainted_var_names(func_id) -> Vec<String>`
  - Show which variables are affected by mutations

- **FR6**: Handle missing or invalid VarIds gracefully
  - Return placeholder like `"unknown_42"` if VarId doesn't map
  - Log warnings for debugging
  - Never panic on invalid IDs

### Non-Functional Requirements

- **NFR1**: Memory efficiency - Adding var_names should increase memory by <10%
- **NFR2**: Serialization - CfgAnalysisWithContext should be serializable (for caching)
- **NFR3**: Performance - Translation should be O(1) lookup per VarId
- **NFR4**: Maintainability - Clear separation between storage and translation logic

## Acceptance Criteria

- [ ] Create `CfgAnalysisWithContext` struct in `src/data_flow/mod.rs`
- [ ] Modify `DataFlowGraph.cfg_analysis` field to use new wrapper
- [ ] Implement translation methods for all VarId-based data (FR2-FR5)
- [ ] Update `populate_from_purity_analysis` to store var_names
- [ ] Update TUI data flow page to call translation methods and display results
- [ ] Add unit tests for each translation method (>90% coverage)
- [ ] Add integration test showing mutation data in TUI
- [ ] Handle edge cases: missing VarIds, empty mappings, corrupted data
- [ ] Document translation strategy and limitations
- [ ] Memory benchmark shows <10% increase

## Technical Details

### Implementation Approach

#### Phase 1: Create Translation Infrastructure

```rust
// In src/data_flow/mod.rs

/// CFG-based data flow analysis with variable name context for translation
#[derive(Debug, Clone)]
pub struct CfgAnalysisWithContext {
    /// The full data flow analysis results
    pub analysis: DataFlowAnalysis,
    /// Variable name mapping from CFG (VarId.name_id -> variable name)
    pub var_names: Vec<String>,
}

impl CfgAnalysisWithContext {
    /// Create from a ControlFlowGraph and its analysis
    pub fn from_cfg(cfg: &ControlFlowGraph, analysis: DataFlowAnalysis) -> Self {
        Self {
            analysis,
            var_names: cfg.var_names.clone(),
        }
    }

    /// Translate a VarId to a variable name
    pub fn var_name(&self, var_id: VarId) -> String {
        self.var_names
            .get(var_id.name_id as usize)
            .cloned()
            .unwrap_or_else(|| format!("unknown_{}", var_id.name_id))
    }

    /// Translate multiple VarIds to names
    pub fn var_names_for(&self, var_ids: impl Iterator<Item = VarId>) -> Vec<String> {
        var_ids.map(|id| self.var_name(id)).collect()
    }
}
```

#### Phase 2: Update DataFlowGraph

```rust
// In src/data_flow/mod.rs

pub struct DataFlowGraph {
    // ... existing fields ...

    /// Full CFG-based data flow analysis with variable name context
    /// Note: Serialized separately since it contains full CFG context
    cfg_analysis_with_context: HashMap<FunctionId, CfgAnalysisWithContext>,
}

impl DataFlowGraph {
    /// Get CFG analysis with translation context
    pub fn get_cfg_analysis_with_context(
        &self,
        func_id: &FunctionId,
    ) -> Option<&CfgAnalysisWithContext> {
        self.cfg_analysis_with_context.get(func_id)
    }

    /// Set CFG analysis with context
    pub fn set_cfg_analysis_with_context(
        &mut self,
        func_id: FunctionId,
        context: CfgAnalysisWithContext,
    ) {
        self.cfg_analysis_with_context.insert(func_id, context);
    }

    /// Get dead store variable names
    pub fn get_dead_store_names(&self, func_id: &FunctionId) -> Vec<String> {
        if let Some(ctx) = self.get_cfg_analysis_with_context(func_id) {
            ctx.var_names_for(ctx.analysis.liveness.dead_stores.iter().copied())
        } else {
            vec![]
        }
    }

    /// Get escaping variable names
    pub fn get_escaping_var_names(&self, func_id: &FunctionId) -> Vec<String> {
        if let Some(ctx) = self.get_cfg_analysis_with_context(func_id) {
            ctx.var_names_for(ctx.analysis.escape_info.escaping_vars.iter().copied())
        } else {
            vec![]
        }
    }

    /// Get return dependency variable names
    pub fn get_return_dependency_names(&self, func_id: &FunctionId) -> Vec<String> {
        if let Some(ctx) = self.get_cfg_analysis_with_context(func_id) {
            ctx.var_names_for(ctx.analysis.escape_info.return_dependencies.iter().copied())
        } else {
            vec![]
        }
    }

    /// Get tainted variable names
    pub fn get_tainted_var_names(&self, func_id: &FunctionId) -> Vec<String> {
        if let Some(ctx) = self.get_cfg_analysis_with_context(func_id) {
            ctx.var_names_for(ctx.analysis.taint_info.tainted_vars.iter().copied())
        } else {
            vec![]
        }
    }
}
```

#### Phase 3: Update Population Logic

```rust
// In src/data_flow/population.rs

pub fn populate_from_purity_analysis(
    data_flow: &mut DataFlowGraph,
    func_id: &FunctionId,
    purity: &PurityAnalysis,
) {
    // Store full CFG analysis WITH context if available
    if let Some(cfg_analysis) = &purity.data_flow_info {
        // Need to reconstruct CFG to get var_names
        // Option 1: Store CFG in PurityAnalysis (memory overhead)
        // Option 2: Reconstruct CFG from function (performance overhead)
        // Option 3: Store var_names separately in PurityAnalysis

        // For now, use Option 3 (add var_names to PurityAnalysis)
        // See "Architecture Changes" section below
    }

    // ... rest of existing logic ...
}
```

#### Phase 4: Update TUI Display

```rust
// In src/tui/results/detail_pages/data_flow.rs

pub fn render(
    frame: &mut Frame,
    _app: &ResultsApp,
    item: &UnifiedDebtItem,
    data_flow: &DataFlowGraph,
    area: Rect,
    theme: &Theme,
) {
    // ... existing code ...

    // Dead stores section - NOW WITH ACTUAL NAMES!
    if let Some(mutation_info) = data_flow.get_mutation_info(&func_id) {
        let dead_store_names = data_flow.get_dead_store_names(&func_id);

        if !dead_store_names.is_empty() {
            add_section_header(&mut lines, "dead stores", theme);
            for var_name in &dead_store_names {
                lines.push(Line::from(vec![
                    Span::raw("                        "),
                    Span::styled(
                        format!("{} (never read after assignment)", var_name),
                        Style::default().fg(theme.muted),
                    ),
                ]));
            }
            add_blank_line(&mut lines);
        }
    }

    // Escaping variables section
    if let Some(cfg_ctx) = data_flow.get_cfg_analysis_with_context(&func_id) {
        let escaping_names = data_flow.get_escaping_var_names(&func_id);

        if !escaping_names.is_empty() {
            add_section_header(&mut lines, "escaping variables", theme);
            for var_name in &escaping_names {
                lines.push(Line::from(vec![
                    Span::raw("                        "),
                    Span::styled(
                        var_name.clone(),
                        Style::default().fg(theme.primary),
                    ),
                ]));
            }
            add_blank_line(&mut lines);
        }
    }

    // ... rest of existing code ...
}
```

### Architecture Changes

1. **Modified**: `src/data_flow/mod.rs`
   - Add `CfgAnalysisWithContext` struct
   - Update `DataFlowGraph.cfg_analysis` → `cfg_analysis_with_context`
   - Add translation methods

2. **Modified**: `src/analyzers/purity_detector.rs`
   - Add `var_names: Vec<String>` field to `PurityAnalysis` struct
   - Populate from CFG during analysis

3. **Modified**: `src/data_flow/population.rs`
   - Update `populate_from_purity_analysis` to use var_names
   - Remove placeholder functions that return empty sets

4. **Modified**: `src/tui/results/detail_pages/data_flow.rs`
   - Call new translation methods
   - Display dead stores and escaping variables with actual names

### Data Structures

```rust
// New struct in src/data_flow/mod.rs
#[derive(Debug, Clone)]
pub struct CfgAnalysisWithContext {
    pub analysis: DataFlowAnalysis,
    pub var_names: Vec<String>,
}

// Modified in src/analyzers/purity_detector.rs
pub struct PurityAnalysis {
    pub is_pure: bool,
    pub purity_level: PurityLevel,
    pub reasons: Vec<String>,
    pub confidence: f32,
    pub data_flow_info: Option<DataFlowAnalysis>,
    pub live_mutations: Vec<LocalMutation>,
    pub total_mutations: usize,

    // NEW: Variable name mapping for translating VarIds
    pub var_names: Vec<String>,
}
```

### Memory Overhead Analysis

**Current state**:
- `DataFlowAnalysis`: ~200 bytes per function (HashMaps of VarIds)
- No var_names stored

**With translation layer**:
- Add `var_names: Vec<String>`: ~8 bytes per variable
- Average function has 5-10 variables → 40-80 bytes
- Overhead: 20-40% increase for DataFlowAnalysis
- Overall impact: <10% of total memory (most memory is in ASTs and metrics)

**Optimization**: Could use string interning if memory becomes an issue.

## Dependencies

- **Prerequisites**: None (independent enhancement)
- **Affected Components**:
  - `src/data_flow/mod.rs` (add translation layer)
  - `src/analyzers/purity_detector.rs` (store var_names)
  - `src/data_flow/population.rs` (use var_names)
  - `src/tui/results/detail_pages/data_flow.rs` (display translated names)
- **External Dependencies**: None (uses existing syn/dashmap)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_varid_translation() {
    let var_names = vec!["x".to_string(), "y".to_string(), "buffer".to_string()];
    let analysis = create_test_analysis(); // Contains VarIds
    let ctx = CfgAnalysisWithContext { analysis, var_names };

    let var_id = VarId { name_id: 0, version: 0 };
    assert_eq!(ctx.var_name(var_id), "x");

    let var_id = VarId { name_id: 2, version: 1 };
    assert_eq!(ctx.var_name(var_id), "buffer");
}

#[test]
fn test_translation_with_missing_id() {
    let var_names = vec!["x".to_string()];
    let analysis = create_test_analysis();
    let ctx = CfgAnalysisWithContext { analysis, var_names };

    let invalid_id = VarId { name_id: 999, version: 0 };
    assert_eq!(ctx.var_name(invalid_id), "unknown_999");
}

#[test]
fn test_dead_store_translation() {
    let mut data_flow = DataFlowGraph::new();
    let func_id = create_test_function_id("test");

    // Create analysis with dead store
    let mut dead_stores = HashSet::new();
    dead_stores.insert(VarId { name_id: 0, version: 0 });

    let analysis = DataFlowAnalysis {
        liveness: LivenessInfo {
            dead_stores,
            ..Default::default()
        },
        ..Default::default()
    };

    let ctx = CfgAnalysisWithContext {
        analysis,
        var_names: vec!["temp".to_string()],
    };

    data_flow.set_cfg_analysis_with_context(func_id.clone(), ctx);

    let names = data_flow.get_dead_store_names(&func_id);
    assert_eq!(names, vec!["temp"]);
}

#[test]
fn test_escaping_var_translation() {
    let mut data_flow = DataFlowGraph::new();
    let func_id = create_test_function_id("test");

    let mut escaping_vars = HashSet::new();
    escaping_vars.insert(VarId { name_id: 0, version: 0 });

    let analysis = DataFlowAnalysis {
        escape_info: EscapeAnalysis {
            escaping_vars,
            ..Default::default()
        },
        ..Default::default()
    };

    let ctx = CfgAnalysisWithContext {
        analysis,
        var_names: vec!["result".to_string()],
    };

    data_flow.set_cfg_analysis_with_context(func_id.clone(), ctx);

    let names = data_flow.get_escaping_var_names(&func_id);
    assert_eq!(names, vec!["result"]);
}
```

### Integration Tests

```rust
#[test]
fn test_purity_analysis_stores_var_names() {
    let code = parse_quote! {
        fn test() {
            let mut x = 1;
            x = 2; // Dead store if x not used
            let y = 3;
            y // Return y
        }
    };

    let purity = analyze_purity(&code);
    assert!(!purity.var_names.is_empty());
    assert!(purity.var_names.contains(&"x".to_string()));
    assert!(purity.var_names.contains(&"y".to_string()));
}

#[test]
fn test_tui_shows_mutation_data() {
    // End-to-end test
    let analysis_result = analyze_project(".");
    let data_flow = analysis_result.data_flow_graph;

    // Find a function with mutations
    let func_id = find_function_with_mutations(&data_flow);

    // Verify we can get variable names
    let dead_stores = data_flow.get_dead_store_names(&func_id);
    assert!(!dead_stores.is_empty(), "Should have detected dead stores with names");

    // Verify names are readable (not "unknown_*")
    for name in &dead_stores {
        assert!(!name.starts_with("unknown_"), "Variable name should be readable: {}", name);
    }
}
```

### Performance Tests

```rust
#[bench]
fn bench_varid_translation(b: &mut Bencher) {
    let ctx = create_large_cfg_context(100); // 100 variables
    let var_id = VarId { name_id: 50, version: 0 };

    b.iter(|| {
        ctx.var_name(var_id);
    });
}

#[bench]
fn bench_batch_translation(b: &mut Bencher) {
    let ctx = create_large_cfg_context(100);
    let var_ids: Vec<VarId> = (0..50).map(|i| VarId { name_id: i, version: 0 }).collect();

    b.iter(|| {
        ctx.var_names_for(var_ids.iter().copied());
    });
}
```

Target: O(1) per VarId translation (simple vector lookup)

## Documentation Requirements

### Code Documentation

```rust
/// CFG-based data flow analysis with variable name context.
///
/// This wrapper combines `DataFlowAnalysis` (which uses numeric `VarId`s)
/// with the variable name mapping from the CFG, enabling translation of
/// VarIds back to human-readable variable names.
///
/// # Why This Exists
///
/// The CFG uses `VarId { name_id: u32, version: u32 }` for efficiency during
/// analysis. To display results to users, we need to translate these IDs back
/// to variable names like "buffer", "x", "result", etc.
///
/// # Example
///
/// ```rust
/// let cfg = ControlFlowGraph::from_block(&block);
/// let analysis = DataFlowAnalysis::analyze(&cfg);
/// let ctx = CfgAnalysisWithContext::from_cfg(&cfg, analysis);
///
/// // Translate a VarId to a name
/// let var_id = VarId { name_id: 0, version: 0 };
/// let name = ctx.var_name(var_id); // "x", "buffer", etc.
/// ```
pub struct CfgAnalysisWithContext { /* ... */ }
```

### User Documentation

Update book section:

```markdown
## Data Flow Page (TUI)

The data flow page shows detailed mutation and escape analysis:

### Mutation Analysis

Shows which variables are mutated and whether those mutations are "live" (affect output):

```
mutation analysis
  total        5
  live         2
  dead stores  1

dead stores
  temp (never read after assignment)
```

### Escape Analysis

Shows which variables "escape" the function (affect return value or are passed to external code):

```
escape analysis
  escaping     3

variables affecting return value
  result
  buffer
  status
```

This helps identify:
- Dead stores that can be removed
- Which mutations actually matter
- What data flows out of the function
```

### Architecture Updates

Update ARCHITECTURE.md:

```markdown
## Data Flow Analysis

### VarId Translation (Spec 247)

The CFG-based data flow analysis uses numeric `VarId`s internally for efficiency.
To display results to users, we store the CFG's `var_names` mapping alongside
the analysis in `CfgAnalysisWithContext`.

Translation is O(1) per VarId (simple vector lookup) with <10% memory overhead.

This enables displaying:
- Dead store variable names
- Escaping variable names
- Return dependency variable names
- Tainted variable names

All mutation and escape analysis data is now actionable in the TUI and reports.
```

## Implementation Notes

### Gotchas

1. **Serialization complexity**: `CfgAnalysisWithContext` contains full analysis + var_names
   - Mitigation: Implement custom serialization if needed
   - Alternative: Store var_names separately in a side table

2. **Version numbers**: VarIds have versions for SSA form
   - Decision: Ignore versions for name display (just use name_id)
   - Rationale: Users don't need to see "x_v0", "x_v1" - just "x"

3. **Memory overhead**: Storing var_names for every function
   - Mitigation: Typical function has <10 variables → 80 bytes overhead
   - Can optimize with string interning if needed

4. **Missing mappings**: VarId might not have corresponding name
   - Mitigation: Return "unknown_N" placeholder
   - Log warning for debugging

### Best Practices

- Always provide fallback for missing VarIds (never panic)
- Use consistent naming: "unknown_N" for missing IDs
- Document memory/performance trade-offs clearly
- Keep translation logic simple (direct vector lookup)

### Performance Considerations

- **VarId translation**: O(1) vector lookup
- **Batch translation**: O(n) for n VarIds
- **Memory overhead**: ~80 bytes per function (5-10 variables * 8 bytes each)
- **No impact on analysis time**: Translation only happens at display time

## Migration and Compatibility

### Breaking Changes

**API changes**:
- `DataFlowGraph.get_cfg_analysis()` deprecated
- New: `DataFlowGraph.get_cfg_analysis_with_context()`

**Migration**:
```rust
// Old
let cfg_analysis = data_flow.get_cfg_analysis(&func_id);

// New
let cfg_ctx = data_flow.get_cfg_analysis_with_context(&func_id);
let cfg_analysis = &cfg_ctx.analysis; // If you need raw analysis
```

### Backward Compatibility

- Old code using `get_cfg_analysis()` can be updated to use `.analysis` field
- Can provide deprecated wrapper for compatibility

### Migration Path

1. Add new `cfg_analysis_with_context` field alongside existing `cfg_analysis`
2. Populate both during transition period
3. Update all call sites to use new field
4. Remove old field once migration complete

### Rollback Plan

Keep old `cfg_analysis` field populated during rollout. Can revert TUI to use old field if issues arise.

## Success Metrics

- **Data availability**: 70%+ of functions with CFG analysis show mutation data in TUI
- **Accuracy**: <1% "unknown_N" placeholders (VarIds should always map)
- **User feedback**: TUI mutation analysis is actionable and useful
- **Memory overhead**: <10% increase in total memory usage

## Future Enhancements (Out of Scope)

- **String interning**: Reduce memory for duplicate variable names
- **Smart name abbreviation**: Show "x" instead of "x_v0" for SSA versions
- **Type annotations**: Show "buffer: Vec<u8>" instead of just "buffer"
- **Source locations**: Show line numbers where variables are defined/mutated
- **Interactive drill-down**: Click variable in TUI to see all uses/mutations
