---
number: 216
title: Complete Data Flow Graph Population
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-01-09
---

# Specification 216: Complete Data Flow Graph Population

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The debtmap codebase has comprehensive data flow analysis infrastructure spread across four modules:

1. **`src/analysis/data_flow.rs`** (1,325 lines): Intra-procedural CFG, liveness, escape, and taint analysis
2. **`src/data_flow.rs`** (482 lines): High-level DataFlowGraph with variable deps, I/O ops, purity info
3. **`src/organization/data_flow_analyzer.rs`** (712 lines): Pipeline stage and type flow detection
4. **`src/analyzers/rust_data_flow_analyzer.rs`** (688 lines): AST-based pattern detection

Currently, the infrastructure exists but the data pipeline is incomplete:

- ✅ `DataFlowGraph` is created from `CallGraph` (parallel_unified_analysis.rs:503)
- ✅ Purity info is populated from `FunctionMetrics` (unified_analysis_utils.rs:159-174)
- ❌ Variable dependencies are NEVER populated
- ❌ I/O operations are NEVER tracked
- ❌ Data transformations are NEVER recorded
- ❌ The detailed CFG-based analysis from purity detector is NOT integrated

The purity detector DOES use data flow analysis internally (purity_detector.rs:161-165) to filter dead mutations and includes the full `DataFlowAnalysis` in its results (line 202), but this rich information isn't propagated to the `DataFlowGraph`.

**Result**: The `DataFlowGraph` passed through scoring and output is mostly empty, containing only basic purity flags.

## Objective

Complete the data flow analysis pipeline so that `DataFlowGraph` contains comprehensive, actionable information about:
- Variable dependencies per function
- I/O operations with location and type
- Data transformations between functions
- Escape analysis results (which mutations affect outputs)
- Live vs dead mutations from CFG analysis

## Requirements

### Functional Requirements

1. **Integrate Purity Detector's CFG Analysis**
   - Extract `DataFlowAnalysis` from `PurityAnalysis` results
   - Store escape analysis (which variables affect return value)
   - Store live mutations (after dead store removal)
   - Store taint analysis (mutation propagation)

2. **Populate I/O Operations**
   - Scan function AST for I/O patterns (File::open, read, write, etc.)
   - Record operation type, variables involved, and line number
   - Use existing detection logic from `rust_data_flow_analyzer.rs:162-180`

3. **Track Variable Dependencies**
   - Extract parameter and return type information
   - Identify data dependencies between variables
   - Use existing patterns from AST analysis

4. **Record Data Transformations**
   - Detect transformation patterns (iterator chains, builders, serialization)
   - Classify transformation types (map, filter, aggregation, etc.)
   - Track input/output variable relationships

### Non-Functional Requirements

- **Performance**: Data flow population must add < 10% to total analysis time
- **Memory**: Use streaming/incremental population to avoid memory spikes
- **Accuracy**: Maintain conservative analysis (false positives ok, no false negatives)
- **Modularity**: Keep analysis concerns separated

## Acceptance Criteria

- [ ] `DataFlowGraph.variable_deps` is populated for all analyzed functions
- [ ] `DataFlowGraph.io_operations` contains detected I/O with accurate line numbers
- [ ] `DataFlowAnalysis` from purity detector is stored in `DataFlowGraph`
- [ ] Escape analysis results are accessible via `DataFlowGraph` API
- [ ] Live mutations vs total mutations are tracked separately
- [ ] Data transformations are detected for pipeline-style code
- [ ] Analysis completes within 10% overhead of current analysis time
- [ ] Unit tests validate correct population for all data types
- [ ] Integration test validates end-to-end population in `parallel_unified_analysis`

## Technical Details

### Implementation Approach

**Phase 1: Extend DataFlowGraph Schema**

Add new fields to store CFG analysis results:

```rust
pub struct DataFlowGraph {
    call_graph: CallGraph,
    variable_deps: HashMap<FunctionId, HashSet<String>>,
    data_transformations: HashMap<(FunctionId, FunctionId), DataTransformation>,
    io_operations: HashMap<FunctionId, Vec<IoOperation>>,
    purity_analysis: HashMap<FunctionId, PurityInfo>,

    // NEW: Store full CFG analysis from purity detector
    cfg_analysis: HashMap<FunctionId, DataFlowAnalysis>,

    // NEW: Store live vs total mutations
    mutation_analysis: HashMap<FunctionId, MutationInfo>,
}

pub struct MutationInfo {
    pub live_mutations: Vec<String>,
    pub total_mutations: usize,
    pub dead_stores: HashSet<String>,
}
```

**Phase 2: Create Populator Trait**

```rust
pub trait DataFlowPopulator {
    fn populate_from_purity_analysis(
        &mut self,
        func_id: FunctionId,
        purity: &PurityAnalysis
    );

    fn populate_io_operations(
        &mut self,
        func_id: FunctionId,
        syn_func: &ItemFn
    );

    fn populate_variable_dependencies(
        &mut self,
        func_id: FunctionId,
        syn_func: &ItemFn
    );
}
```

**Phase 3: Wire Up in Parallel Analysis**

In `parallel_unified_analysis.rs`, after purity analysis completes:

```rust
// Current: purity_map created
let purity_map = transformations::metrics_to_purity_map(&metrics);

// NEW: Extract full PurityAnalysis, not just bool
let purity_results: HashMap<FunctionId, PurityAnalysis> =
    extract_full_purity_analysis(&metrics);

// NEW: Populate DataFlowGraph from purity results
for (func_id, purity) in &purity_results {
    data_flow_graph.populate_from_purity_analysis(func_id, purity);
}
```

**Phase 4: Add I/O and Variable Dependency Population**

Create new module `src/data_flow/population.rs`:

```rust
pub fn populate_io_operations(
    data_flow: &mut DataFlowGraph,
    metrics: &[FunctionMetrics]
) {
    for metric in metrics {
        let func_id = FunctionId::new(/*...*/);

        // Re-parse function to detect I/O
        if let Ok(syn_func) = parse_function(&metric.file, metric.line) {
            let io_ops = detect_io_operations(&syn_func);
            for op in io_ops {
                data_flow.add_io_operation(func_id.clone(), op);
            }
        }
    }
}
```

### Architecture Changes

**New Module**: `src/data_flow/population.rs`
- `populate_from_purity_analysis()`
- `populate_io_operations()`
- `populate_variable_dependencies()`
- `populate_data_transformations()`

**Modified Files**:
- `src/data_flow.rs`: Add `cfg_analysis` and `mutation_analysis` fields
- `src/builders/parallel_unified_analysis.rs`: Call population functions
- `src/priority/unified_analysis_utils.rs`: Expose population methods

### Data Structures

```rust
// Store full CFG analysis per function
pub struct StoredCFGAnalysis {
    pub liveness: LivenessInfo,
    pub escape: EscapeAnalysis,
    pub taint: TaintAnalysis,
    pub reaching_defs: ReachingDefinitions,
}

// Store mutation details
pub struct MutationInfo {
    pub live_mutations: Vec<String>,
    pub total_mutations: usize,
    pub dead_stores: HashSet<String>,
    pub escaping_mutations: HashSet<String>,
}
```

### Integration Points

1. **Purity Detector → DataFlowGraph**
   - Extract `data_flow_info` from `PurityAnalysis`
   - Store in `DataFlowGraph.cfg_analysis`

2. **AST Analysis → DataFlowGraph**
   - Use existing patterns from `rust_data_flow_analyzer.rs`
   - Populate I/O operations and transformations

3. **Parallel Analysis Builder**
   - Call population functions after phase 2 (purity analysis)
   - Ensure population happens before scoring (phase 3)

## Dependencies

**Prerequisites**: None (all required infrastructure exists)

**Affected Components**:
- `src/data_flow.rs` - Schema extension
- `src/builders/parallel_unified_analysis.rs` - Population calls
- `src/analyzers/purity_detector.rs` - Expose full analysis
- `src/analyzers/rust_data_flow_analyzer.rs` - Reuse detection logic

**External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_populate_from_purity_analysis() {
    let mut data_flow = DataFlowGraph::new();
    let purity = create_test_purity_analysis_with_mutations();

    data_flow.populate_from_purity_analysis(func_id, &purity);

    assert!(data_flow.cfg_analysis.contains_key(&func_id));
    assert_eq!(data_flow.mutation_analysis[&func_id].live_mutations.len(), 2);
}

#[test]
fn test_populate_io_operations() {
    let mut data_flow = DataFlowGraph::new();
    let code = r#"
        fn read_file(path: &Path) -> Result<String> {
            let mut file = File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            Ok(content)
        }
    "#;

    data_flow.populate_io_operations(func_id, &parse(code));

    let io_ops = data_flow.get_io_operations(&func_id).unwrap();
    assert_eq!(io_ops.len(), 2); // open + read_to_string
}
```

### Integration Tests

```rust
#[test]
fn test_full_data_flow_pipeline() {
    let metrics = load_test_metrics();
    let builder = ParallelUnifiedAnalysisBuilder::new();

    // Execute full analysis
    let (analysis, _) = builder.analyze_with_coverage(metrics, None);

    // Verify DataFlowGraph is populated
    assert!(!analysis.data_flow_graph.variable_deps.is_empty());
    assert!(!analysis.data_flow_graph.io_operations.is_empty());
    assert!(!analysis.data_flow_graph.cfg_analysis.is_empty());
}
```

### Performance Tests

```rust
#[test]
fn test_population_performance_overhead() {
    let metrics = generate_large_metric_set(1000);

    let start = Instant::now();
    let baseline_analysis = analyze_without_population(&metrics);
    let baseline_time = start.elapsed();

    let start = Instant::now();
    let full_analysis = analyze_with_population(&metrics);
    let full_time = start.elapsed();

    let overhead = (full_time - baseline_time).as_secs_f64() / baseline_time.as_secs_f64();
    assert!(overhead < 0.10, "Overhead should be < 10%, got {}%", overhead * 100.0);
}
```

## Documentation Requirements

### Code Documentation

- Document new `DataFlowGraph` fields with usage examples
- Add module-level docs to `src/data_flow/population.rs`
- Document population functions with before/after examples

### User Documentation

- Update `book/src/analysis-guide/advanced-features.md`
- Add section on data flow analysis results
- Provide examples of querying populated data

### Architecture Updates

Update `ARCHITECTURE.md`:
```markdown
## Data Flow Analysis

### Population Pipeline

1. **Phase 1**: Create DataFlowGraph from CallGraph
2. **Phase 2**: Populate purity info from FunctionMetrics
3. **Phase 3**: Extract CFG analysis from PurityAnalysis ← NEW
4. **Phase 4**: Detect and record I/O operations ← NEW
5. **Phase 5**: Track variable dependencies ← NEW
6. **Phase 6**: Identify data transformations ← NEW

### Data Flow Components

- **CFG Analysis**: Liveness, escape, taint analysis per function
- **I/O Operations**: File, network, console operations with locations
- **Variable Dependencies**: Data flow between variables and parameters
- **Transformations**: Iterator chains, builders, serialization patterns
```

## Implementation Notes

### Avoiding Re-parsing

The purity detector already parses functions and creates CFG. To avoid duplicate work:

1. Store `PurityAnalysis` (which contains `DataFlowAnalysis`) in a cache
2. Pass cached results to population functions
3. Only re-parse for I/O detection if CFG isn't sufficient

### Conservative Analysis

When in doubt:
- Over-report I/O operations (false positives acceptable)
- Mark mutations as live if uncertain
- Conservative escape analysis (assume variables escape if unclear)

### Performance Optimization

- Use parallel iteration for population (rayon)
- Lazy population: only populate on-demand for displayed items
- Cache parsed AST to avoid duplicate parsing

## Migration and Compatibility

### Breaking Changes

None - this is additive functionality.

### Backward Compatibility

- Old JSON output remains unchanged
- New fields are optional and not serialized if empty
- Existing scoring code continues to work

### Migration Path

No migration required. Data flow information will be automatically populated on next analysis run.
