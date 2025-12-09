---
number: 201
title: File-Level Dependency Metrics in Analysis Output
category: foundation
priority: high
status: draft
dependencies: [179]
created: 2025-12-09
---

# Specification 201: File-Level Dependency Metrics in Analysis Output

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [179 - Coupling and Dependency Visualization]

## Context

Currently, debtmap's analysis output includes dependency information for **function-level items** but not for **file-level items**. This creates an incomplete picture of technical debt.

**Current Function Output** (includes dependencies):
```json
{
  "type": "Function",
  "dependencies": {
    "upstream_count": 9,
    "downstream_count": 0,
    "upstream_callers": ["file.rs:test_foo", "file.rs:from_metrics"]
  }
}
```

**Current File Output** (missing dependencies):
```json
{
  "type": "File",
  "metrics": {
    "lines": 1442,
    "functions": 39,
    "avg_complexity": 2.08
  },
  "god_object_indicators": {
    "methods_count": 39,
    "responsibilities": 8
  }
  // NO dependency information!
}
```

**The Gap**: File-level items show god object indicators and responsibilities but have **zero dependency metrics**:
- No afferent coupling (incoming dependencies - who depends on this file)
- No efferent coupling (outgoing dependencies - what this file depends on)
- No instability metric
- No import/export analysis
- No coupling score

**Real-World Impact**: When analyzing `file_metrics.rs` which shows as a god object with 8 responsibilities, developers cannot see:
- How many other files import types/functions from this file
- How many external modules this file depends on
- Whether this is a stable core module or an unstable leaf module
- The coupling-based priority for refactoring

**Root Cause Analysis**:

1. `FileDebtMetrics` struct in `src/priority/file_metrics.rs` has no dependency fields
2. `FileDebtItemOutput` struct in `src/output/unified.rs` has no `dependencies` field
3. `CouplingMetrics` exists in `src/debt/coupling.rs` but is computed at module level and disconnected from file output
4. Function-level coupling analysis in `src/risk/evidence/coupling_analyzer.rs` is not aggregated to file level

## Objective

Add file-level dependency metrics to the analysis output, providing:

1. **Afferent coupling** (Ca) - Number of external files/modules that depend on this file
2. **Efferent coupling** (Ce) - Number of external files/modules this file depends on
3. **Instability metric** - I = Ce / (Ca + Ce), range [0,1] where 0=stable, 1=unstable
4. **Import/export counts** - Number of public items exported, items imported
5. **Coupling score** - Composite metric indicating coupling-related debt

## Requirements

### Functional Requirements

1. **File-Level Dependency Collection**
   - Track `use` statements to identify efferent coupling (what this file imports)
   - Track which other files reference exports from this file (afferent coupling)
   - Distinguish between internal (same crate) and external (other crates) dependencies
   - Handle re-exports and transitive dependencies appropriately

2. **Coupling Metrics Calculation**
   - **Afferent coupling (Ca)**: Count of unique files/modules that import from this file
   - **Efferent coupling (Ce)**: Count of unique files/modules this file imports from
   - **Instability**: I = Ce / (Ca + Ce), or 0.0 if Ca + Ce = 0
   - **Total coupling**: Ca + Ce (raw coupling indicator)

3. **Aggregation from Function Level**
   - Aggregate function-level upstream/downstream counts to file level
   - Sum upstream_callers from all functions in the file
   - Sum downstream_callees from all functions in the file
   - De-duplicate callers/callees that appear multiple times

4. **Output Integration**
   - Add `dependencies` field to `FileDebtItemOutput` struct
   - Include dependency metrics in JSON output format
   - Display coupling information in text/markdown output formats
   - Integrate with TUI detail view for files

5. **Dependency Classification**
   - **Stable core**: Low instability (I < 0.3), high afferent coupling
   - **Utility module**: Medium instability, balanced coupling
   - **Leaf module**: High instability (I > 0.7), low afferent coupling
   - **Isolated module**: Very low total coupling (Ca + Ce < 3)

### Non-Functional Requirements

1. **Performance**: Dependency aggregation should add <5% to analysis time
2. **Accuracy**: Coupling counts should match manual inspection
3. **Consistency**: File-level dependencies should aggregate correctly from function-level data
4. **Backward Compatibility**: Existing JSON consumers should not break (new fields are additive)

## Acceptance Criteria

- [ ] `FileDebtItemOutput` includes a `dependencies` field with `afferent_coupling`, `efferent_coupling`, `instability`, and lists
- [ ] JSON output for File items includes dependency metrics
- [ ] Files with high coupling (Ca + Ce > 15) show coupling warning in recommendations
- [ ] Files with extreme instability (I > 0.9) or stability (I < 0.1) get appropriate context
- [ ] Dependency data aggregates correctly from function-level call graph
- [ ] TUI file detail view shows coupling information
- [ ] Text/markdown output includes coupling summary for god object files
- [ ] Unit tests verify coupling calculation accuracy
- [ ] Integration tests verify aggregation from function to file level

## Technical Details

### Implementation Approach

**Phase 1: Data Structure Updates**

Add dependency fields to `FileDebtMetrics`:
```rust
// In src/priority/file_metrics.rs
pub struct FileDebtMetrics {
    // ... existing fields ...

    /// Afferent coupling - files that depend on this file
    pub afferent_coupling: usize,
    /// Efferent coupling - files this file depends on
    pub efferent_coupling: usize,
    /// Instability metric (0.0 = stable, 1.0 = unstable)
    pub instability: f64,
    /// List of files that import from this file (top N)
    pub dependents: Vec<String>,
    /// List of files this file imports from (top N)
    pub dependencies_list: Vec<String>,
}
```

Add to `FileDebtItemOutput`:
```rust
// In src/output/unified.rs
pub struct FileDebtItemOutput {
    // ... existing fields ...

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<FileDependencies>,
}

pub struct FileDependencies {
    pub afferent_coupling: usize,
    pub efferent_coupling: usize,
    pub instability: f64,
    pub total_coupling: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_dependents: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_dependencies: Vec<String>,
    pub coupling_classification: String,
}
```

**Phase 2: Coupling Analysis Integration**

Connect existing `CouplingMetrics` from `src/debt/coupling.rs`:
```rust
// Build file-level coupling from module dependencies
fn aggregate_file_coupling(
    file_path: &Path,
    module_deps: &[ModuleDependency],
    function_items: &[UnifiedDebtItem],
) -> FileDependencies {
    // 1. Get module-level coupling metrics
    let coupling_metrics = calculate_coupling_metrics(module_deps);

    // 2. Aggregate function-level callers/callees
    let (upstream, downstream) = aggregate_function_dependencies(file_path, function_items);

    // 3. Combine and classify
    FileDependencies {
        afferent_coupling: upstream.len(),
        efferent_coupling: downstream.len(),
        instability: calculate_instability(upstream.len(), downstream.len()),
        total_coupling: upstream.len() + downstream.len(),
        top_dependents: upstream.into_iter().take(5).collect(),
        top_dependencies: downstream.into_iter().take(5).collect(),
        coupling_classification: classify_coupling(upstream.len(), downstream.len()),
    }
}
```

**Phase 3: Output Integration**

Update `FileDebtItemOutput::from_file_item` in `src/output/unified.rs`:
```rust
fn from_file_item(item: &FileDebtItem, include_scoring_details: bool) -> Self {
    FileDebtItemOutput {
        // ... existing fields ...
        dependencies: Some(FileDependencies {
            afferent_coupling: item.metrics.afferent_coupling,
            efferent_coupling: item.metrics.efferent_coupling,
            instability: item.metrics.instability,
            total_coupling: item.metrics.afferent_coupling + item.metrics.efferent_coupling,
            top_dependents: item.metrics.dependents.iter().take(5).cloned().collect(),
            top_dependencies: item.metrics.dependencies_list.iter().take(5).cloned().collect(),
            coupling_classification: classify_coupling_level(
                item.metrics.afferent_coupling,
                item.metrics.efferent_coupling,
            ),
        }),
    }
}
```

### Architecture Changes

1. **New data flow**: Call graph → Function dependencies → File aggregation → Output
2. **Connect existing**: Link `CouplingMetrics` calculation to file output pipeline
3. **Aggregation layer**: New function to roll up function-level data to file-level

### Data Structures

```rust
/// File-level dependency classification
pub enum CouplingClassification {
    StableCore,    // Low instability, high Ca
    UtilityModule, // Balanced coupling
    LeafModule,    // High instability, low Ca
    Isolated,      // Very low total coupling
    HighlyCoupled, // Ca + Ce > threshold (problematic)
}
```

### APIs and Interfaces

**JSON Output Changes**:
```json
{
  "type": "File",
  "score": 89.8,
  "metrics": { /* existing */ },
  "dependencies": {
    "afferent_coupling": 12,
    "efferent_coupling": 8,
    "instability": 0.4,
    "total_coupling": 20,
    "top_dependents": ["main.rs", "lib.rs", "commands/analyze.rs"],
    "top_dependencies": ["std::collections", "serde", "crate::core"],
    "coupling_classification": "UtilityModule"
  }
}
```

## Dependencies

- **Prerequisites**: Spec 179 (Coupling and Dependency Visualization) provides method-level coupling
- **Affected Components**:
  - `src/priority/file_metrics.rs` - Add dependency fields
  - `src/output/unified.rs` - Add FileDependencies to output
  - `src/debt/coupling.rs` - Reuse CouplingMetrics calculations
  - `src/builders/unified_analysis.rs` - Aggregate during analysis
  - `src/tui/results/detail_pages/dependencies.rs` - Display in TUI
- **External Dependencies**: None new

## Testing Strategy

- **Unit Tests**:
  - Test instability calculation (edge cases: 0/0, high Ca, high Ce)
  - Test coupling classification thresholds
  - Test dependency list truncation to top N

- **Integration Tests**:
  - Analyze a known codebase, verify coupling counts match expectations
  - Verify function-to-file aggregation produces correct totals
  - Test JSON output parsing and validation

- **Performance Tests**:
  - Measure analysis time impact on large codebases
  - Verify <5% overhead for dependency aggregation

## Documentation Requirements

- **Code Documentation**: Document `FileDependencies` struct and classification enum
- **User Documentation**: Add section on interpreting coupling metrics in output
- **Architecture Updates**: Document data flow from call graph to file-level output

## Implementation Notes

1. **Start with aggregation**: The easiest path is aggregating existing function-level data
2. **Leverage existing code**: `CouplingMetrics` in `debt/coupling.rs` already calculates what we need
3. **Top N lists**: Limit dependent/dependency lists to 5-10 items to avoid output bloat
4. **Internal vs external**: Consider distinguishing crate-internal from external dependencies
5. **TUI integration**: The TUI already has a dependencies detail page that can be enhanced

## Migration and Compatibility

- **Additive change**: New fields added to JSON output, no breaking changes
- **Default values**: Files analyzed before this change will have `dependencies: null`
- **Backward compatible**: Existing JSON consumers will ignore new fields unless they opt in
