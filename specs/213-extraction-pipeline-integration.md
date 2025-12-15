---
number: 213
title: Extraction Pipeline Integration
category: optimization
priority: critical
status: draft
dependencies: [211, 212]
created: 2025-01-14
---

# Specification 213: Extraction Pipeline Integration

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: 211 (Types), 212 (Extractor)

## Context

With the extraction types (spec 211) and extractor (spec 212) implemented, we need to integrate unified extraction into the analysis pipeline. This spec covers:

1. Adding an extraction phase early in the pipeline
2. Passing extracted data to all downstream phases
3. Removing per-function parsing from data_flow/population.rs
4. Removing redundant parsing from call graph building

## Objective

Integrate the unified extraction architecture into the analysis pipeline so that each file is parsed exactly once, and all analysis phases consume pre-extracted data.

## Requirements

### Functional Requirements

1. **New Pipeline Phase**: Add "Unified Extraction" phase after file discovery
   - Extract all files in batches of 200
   - Reset SourceMap between batches
   - Store results in `HashMap<PathBuf, ExtractedFileData>`

2. **Modify data_flow/population.rs**: Replace per-function parsing functions
   - `populate_io_operations` → `populate_io_operations_from_extracted`
   - `populate_variable_dependencies` → `populate_variable_dependencies_from_extracted`
   - `populate_data_transformations` → `populate_data_transformations_from_extracted`
   - Delete the old per-function parsing versions

3. **Modify parallel_unified_analysis.rs**:
   - Remove `extract_purity_analysis` function (data comes from extraction)
   - Accept `&HashMap<PathBuf, ExtractedFileData>` in builder
   - Use extracted purity data in data flow graph population

4. **Modify call graph building**:
   - Add `build_from_extracted` that uses pre-extracted call sites
   - Remove direct file parsing in favor of extracted data

5. **Modify metrics extraction**:
   - Add `metrics_from_extracted` to RustAnalyzer
   - Convert `ExtractedFunctionData` → `FunctionMetrics`

### Non-Functional Requirements

- Overall analysis time should decrease by 10x+ for large codebases
- Memory usage increase should be bounded (~16MB for 2000 files)
- No change in analysis output (results must be equivalent)

## Acceptance Criteria

- [ ] Extraction phase added to pipeline after file discovery
- [ ] `data_flow/population.rs` functions replaced with extracted-data versions
- [ ] Old per-function parsing functions deleted from `data_flow/population.rs`
- [ ] `extract_purity_analysis` removed from `parallel_unified_analysis.rs`
- [ ] `ParallelUnifiedAnalysisBuilder` accepts extracted data
- [ ] Call graph building uses extracted call sites
- [ ] `FunctionMetrics` can be created from `ExtractedFunctionData`
- [ ] `debtmap analyze ../zed` completes without SourceMap overflow
- [ ] Analysis output unchanged (diff test against known baseline)
- [ ] 10x+ speedup measured on large codebase

## Technical Details

### Pipeline Modification

**File**: `src/commands/analyze/project_analysis.rs`

```rust
use crate::extraction::{ExtractedFileData, UnifiedFileExtractor};

pub fn run_analysis(config: &AnalyzeConfig) -> Result<AnalysisResults> {
    // Phase 0: File Discovery (unchanged)
    let files = discover_files(config)?;

    // Phase 1: Unified Extraction (NEW)
    let extracted_data = extract_all_files(&files, config)?;

    // Phase 2: Metrics Extraction (uses extracted data)
    let metrics = extract_metrics_from_extracted(&extracted_data)?;

    // Phase 3: Call Graph (uses extracted data)
    let call_graph = build_call_graph_from_extracted(&extracted_data)?;

    // Phase 4+: Rest of pipeline (uses extracted data)
    // ...
}

fn extract_all_files(
    files: &[PathBuf],
    config: &AnalyzeConfig,
) -> Result<HashMap<PathBuf, ExtractedFileData>> {
    const BATCH_SIZE: usize = 200;

    let mut extracted = HashMap::with_capacity(files.len());

    for batch in files.chunks(BATCH_SIZE) {
        let contents: Vec<_> = batch
            .par_iter()
            .filter_map(|p| std::fs::read_to_string(p).ok().map(|c| (p.clone(), c)))
            .collect();

        for (path, content) in contents {
            match UnifiedFileExtractor::extract(&path, &content) {
                Ok(data) => { extracted.insert(path, data); }
                Err(e) => log::warn!("Failed to extract {}: {}", path.display(), e),
            }
        }

        // Reset SourceMap after each batch
        crate::core::parsing::reset_span_locations();
    }

    Ok(extracted)
}
```

### data_flow/population.rs Changes

**Delete these functions** (they parse per-function):
- `populate_io_operations`
- `extract_variable_deps`
- `populate_variable_dependencies` (calls extract_variable_deps)
- `populate_data_transformations`

**Add these functions** (use pre-extracted data):

```rust
/// Populate I/O operations from pre-extracted data.
/// NO FILE PARSING - uses data already extracted.
pub fn populate_io_operations_from_extracted(
    data_flow: &mut DataFlowGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> usize {
    let mut total_ops = 0;

    for (path, file_data) in extracted {
        for func in &file_data.functions {
            let func_id = FunctionId::new(path.clone(), func.name.clone(), func.line);

            for io_op in &func.io_operations {
                data_flow.add_io_operation(func_id.clone(), io_op.clone().into());
                total_ops += 1;
            }
        }
    }

    total_ops
}

/// Populate variable dependencies from pre-extracted data.
/// NO FILE PARSING - uses parameter names already extracted.
pub fn populate_variable_dependencies_from_extracted(
    data_flow: &mut DataFlowGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> usize {
    let mut total_deps = 0;

    for (path, file_data) in extracted {
        for func in &file_data.functions {
            if !func.parameter_names.is_empty() {
                let func_id = FunctionId::new(path.clone(), func.name.clone(), func.line);
                let deps: HashSet<String> = func.parameter_names.iter().cloned().collect();
                data_flow.add_variable_dependencies(func_id, deps.clone());
                total_deps += deps.len();
            }
        }
    }

    total_deps
}

/// Populate data transformations from pre-extracted data.
/// NO FILE PARSING - uses patterns already extracted.
pub fn populate_data_transformations_from_extracted(
    data_flow: &mut DataFlowGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
    metrics: &[FunctionMetrics],
) -> usize {
    let mut count = 0;

    for (path, file_data) in extracted {
        for func in &file_data.functions {
            // Find matching metric for this function
            let metric = metrics.iter().find(|m| {
                m.file == *path && m.name == func.name && m.line == func.line
            });

            if let Some(m) = metric {
                for pattern in &func.transformation_patterns {
                    count += register_transformation(data_flow, m, pattern);
                }
            }
        }
    }

    count
}
```

### parallel_unified_analysis.rs Changes

**Delete**: `extract_purity_analysis` function entirely

**Modify** `ParallelUnifiedAnalysisBuilder`:

```rust
pub struct ParallelUnifiedAnalysisBuilder {
    call_graph: Arc<CallGraph>,
    options: ParallelUnifiedAnalysisOptions,
    timings: AnalysisPhaseTimings,
    risk_analyzer: Option<crate::risk::RiskAnalyzer>,
    project_path: PathBuf,
    line_count_index: HashMap<PathBuf, usize>,
    // NEW: Pre-extracted file data
    extracted_data: Arc<HashMap<PathBuf, ExtractedFileData>>,
}

impl ParallelUnifiedAnalysisBuilder {
    pub fn new_with_extracted_data(
        call_graph: CallGraph,
        extracted_data: HashMap<PathBuf, ExtractedFileData>,
        options: ParallelUnifiedAnalysisOptions,
    ) -> Self {
        Self {
            call_graph: Arc::new(call_graph),
            extracted_data: Arc::new(extracted_data),
            options,
            // ...
        }
    }

    fn execute_phase1_tasks(&mut self, metrics: &[FunctionMetrics], ...) {
        // Data flow graph creation now uses extracted data
        // instead of calling extract_purity_analysis

        rayon::scope(|s| {
            // Task 1: Data flow graph from extracted data
            self.spawn_data_flow_task_from_extracted(
                s,
                Arc::clone(&self.extracted_data),
                Arc::clone(&metrics_arc),
                // ...
            );
            // ... other tasks
        });
    }

    fn spawn_data_flow_task_from_extracted<'a>(
        &self,
        scope: &rayon::Scope<'a>,
        extracted: Arc<HashMap<PathBuf, ExtractedFileData>>,
        metrics: Arc<Vec<FunctionMetrics>>,
        result: Arc<Mutex<Option<DataFlowGraph>>>,
        timings: Arc<Mutex<AnalysisPhaseTimings>>,
        progress: Arc<indicatif::ProgressBar>,
    ) {
        scope.spawn(move |_| {
            let start = Instant::now();

            let mut data_flow = DataFlowGraph::new();

            // Populate from extracted purity data (NO PARSING)
            for (path, file_data) in extracted.iter() {
                for func in &file_data.functions {
                    let func_id = FunctionId::new(
                        path.clone(),
                        func.name.clone(),
                        func.line
                    );

                    // Purity info from extraction
                    let purity_info = PurityInfo {
                        is_pure: func.purity_analysis.is_pure,
                        confidence: func.purity_analysis.confidence,
                        impurity_reasons: if !func.purity_analysis.is_pure {
                            vec!["Function has side effects".to_string()]
                        } else {
                            vec![]
                        },
                    };
                    data_flow.set_purity_info(func_id.clone(), purity_info);

                    // Mutations from extraction
                    for mutation in &func.purity_analysis.local_mutations {
                        data_flow.add_mutation(func_id.clone(), mutation.clone(), true);
                    }
                    for mutation in &func.purity_analysis.upvalue_mutations {
                        data_flow.add_mutation(func_id.clone(), mutation.clone(), false);
                    }
                }
            }

            // Populate I/O, deps, transformations from extracted data
            let io_count = populate_io_operations_from_extracted(
                &mut data_flow,
                &extracted,
            );
            let dep_count = populate_variable_dependencies_from_extracted(
                &mut data_flow,
                &extracted,
            );
            let trans_count = populate_data_transformations_from_extracted(
                &mut data_flow,
                &extracted,
                &metrics,
            );

            timings.lock().data_flow_creation = start.elapsed();
            *result.lock() = Some(data_flow);
            progress.finish_with_message(format!(
                "Data flow complete: {} I/O ops, {} deps, {} transforms",
                io_count, dep_count, trans_count
            ));
        });
    }
}
```

### Call Graph Building Changes

**File**: `src/builders/parallel_call_graph.rs`

Add function to build from extracted data:

```rust
/// Build call graph from pre-extracted data.
/// NO FILE PARSING - uses call sites already extracted.
pub fn build_call_graph_from_extracted(
    extracted: &HashMap<PathBuf, ExtractedFileData>,
    base_graph: CallGraph,
) -> CallGraph {
    let mut graph = base_graph;

    for (path, file_data) in extracted {
        // Add all functions as nodes
        for func in &file_data.functions {
            let func_id = FunctionId::new(path.clone(), func.name.clone(), func.line);

            graph.add_function(
                func_id.clone(),
                is_entry_point(&func.name),
                func.is_test,
                func.cyclomatic,
                func.length,
            );

            // Add call edges from extracted call sites
            for call in &func.calls {
                // Resolve callee to FunctionId
                if let Some(callee_id) = resolve_call_target(path, &call, extracted) {
                    graph.add_call(func_id.clone(), callee_id);
                }
            }
        }
    }

    graph.resolve_cross_file_calls();
    graph
}

fn resolve_call_target(
    caller_file: &Path,
    call: &CallSite,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> Option<FunctionId> {
    // First try same file
    if let Some(file_data) = extracted.get(caller_file) {
        if let Some(func) = file_data.functions.iter()
            .find(|f| f.name == call.callee_name || f.qualified_name == call.callee_name)
        {
            return Some(FunctionId::new(
                caller_file.to_path_buf(),
                func.name.clone(),
                func.line,
            ));
        }
    }

    // Try other files (cross-file calls)
    for (path, file_data) in extracted {
        if let Some(func) = file_data.functions.iter()
            .find(|f| f.name == call.callee_name)
        {
            return Some(FunctionId::new(path.clone(), func.name.clone(), func.line));
        }
    }

    None
}
```

### Metrics Conversion

**File**: `src/analyzers/rust.rs` or new `src/extraction/adapters/metrics.rs`

```rust
impl From<&ExtractedFunctionData> for FunctionMetrics {
    fn from(extracted: &ExtractedFunctionData) -> Self {
        FunctionMetrics {
            name: extracted.name.clone(),
            file: PathBuf::new(), // Set by caller
            line: extracted.line,
            cyclomatic: extracted.cyclomatic,
            cognitive: extracted.cognitive,
            nesting: extracted.nesting,
            length: extracted.length,
            is_test: extracted.is_test,
            visibility: extracted.visibility.clone(),
            is_trait_method: extracted.is_trait_method,
            in_test_module: extracted.in_test_module,
            is_pure: Some(extracted.purity_analysis.is_pure),
            purity_confidence: Some(extracted.purity_analysis.confidence),
            purity_level: Some(extracted.purity_analysis.purity_level.into()),
            // ... other fields
        }
    }
}

/// Create FunctionMetrics from extracted file data
pub fn metrics_from_extracted(
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> Vec<FunctionMetrics> {
    extracted
        .iter()
        .flat_map(|(path, file_data)| {
            file_data.functions.iter().map(|func| {
                let mut metrics = FunctionMetrics::from(func);
                metrics.file = path.clone();
                metrics
            })
        })
        .collect()
}
```

## Dependencies

- **Prerequisites**: Specs 211, 212 must be implemented first
- **Affected Components**:
  - `src/commands/analyze/project_analysis.rs`
  - `src/data_flow/population.rs`
  - `src/builders/parallel_unified_analysis.rs`
  - `src/builders/parallel_call_graph.rs`
  - `src/analyzers/rust.rs`

## Testing Strategy

- **Unit Tests**: Each new `*_from_extracted` function produces same output as old function
- **Integration Tests**: Full pipeline produces identical results
- **Regression Tests**: Run on known codebases, diff output
- **Performance Tests**: Measure speedup on zed codebase
- **Overflow Tests**: Verify zed analysis completes without panic

## Documentation Requirements

- **Code Documentation**: Update function docs to indicate no parsing
- **Architecture Updates**: Update ARCHITECTURE.md with new extraction phase

## Implementation Notes

- Delete old functions completely (no deprecation needed pre-1.0)
- Keep the old `parallel_call_graph.rs` batched approach as fallback initially
- Ensure all progress reporting still works with new pipeline

## Migration and Compatibility

No user-facing changes. Internal refactoring only. Output format unchanged.
