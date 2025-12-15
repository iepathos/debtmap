---
number: 214
title: Extraction Adapters
category: optimization
priority: high
status: draft
dependencies: [211, 212]
created: 2025-01-14
---

# Specification 214: Extraction Adapters

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: 211 (Types), 212 (Extractor)

## Context

The unified extraction architecture produces `ExtractedFileData` structures, but existing analysis code expects various other types like `FunctionMetrics`, `CallGraph` edges, `GodObjectAnalysis`, etc. Adapters bridge this gap by converting extracted data to the formats expected by existing code.

This separation allows:
1. Clean interfaces between extraction and analysis
2. Gradual migration without breaking existing code
3. Type-safe conversions with compile-time checks

## Objective

Implement adapter modules that convert `ExtractedFileData` to all existing data types used by the analysis pipeline.

## Requirements

### Functional Requirements

1. **Metrics Adapter**: Convert `ExtractedFunctionData` → `FunctionMetrics`
   - All complexity fields mapped correctly
   - Purity fields populated from extraction
   - Metadata fields (is_test, visibility) preserved

2. **Call Graph Adapter**: Convert extracted calls → `CallGraph`
   - Function nodes from extracted functions
   - Call edges from extracted call sites
   - Cross-file call resolution

3. **Data Flow Adapter**: Convert extracted data → `DataFlowGraph` components
   - I/O operations from extracted io_operations
   - Variable dependencies from parameter_names
   - Transformation patterns from transformation_patterns
   - Purity info from purity_analysis

4. **God Object Adapter**: Convert extracted structs/impls → `GodObjectAnalysis`
   - Struct metrics from ExtractedStructData
   - Impl block analysis from ExtractedImplData
   - Method counts aggregated from impl methods

5. **File Metrics Adapter**: Convert file-level data → `FileMetrics`
   - Aggregate function metrics
   - Total lines from extraction
   - Module scope from extraction

### Non-Functional Requirements

- Adapters must be pure functions (no I/O, no parsing)
- Conversion should be O(n) where n is number of items
- All adapters must be unit testable in isolation

## Acceptance Criteria

- [ ] `metrics_adapter::to_function_metrics(extracted) -> FunctionMetrics` implemented
- [ ] `metrics_adapter::to_file_metrics(path, extracted) -> FileMetrics` implemented
- [ ] `call_graph_adapter::build_call_graph(extracted) -> CallGraph` implemented
- [ ] `data_flow_adapter::populate_data_flow(extracted, graph)` implemented
- [ ] `god_object_adapter::analyze(path, extracted) -> Option<GodObjectAnalysis>` implemented
- [ ] All adapters produce equivalent output to direct parsing (verified by tests)
- [ ] All adapters are pure functions with no file I/O

## Technical Details

### Module Location

```
src/extraction/
├── mod.rs
├── types.rs         # Spec 211
├── extractor.rs     # Spec 212
└── adapters/
    ├── mod.rs       # Re-exports
    ├── metrics.rs   # FunctionMetrics, FileMetrics conversion
    ├── call_graph.rs # CallGraph building
    ├── data_flow.rs # DataFlowGraph population
    └── god_object.rs # GodObjectAnalysis conversion
```

### Metrics Adapter

**File**: `src/extraction/adapters/metrics.rs`

```rust
use crate::core::{FunctionMetrics, FileMetrics, Language};
use crate::extraction::types::{ExtractedFileData, ExtractedFunctionData, PurityLevel};
use std::path::Path;

/// Convert extracted function data to FunctionMetrics.
///
/// This is a pure conversion with no file I/O.
pub fn to_function_metrics(
    file_path: &Path,
    extracted: &ExtractedFunctionData,
) -> FunctionMetrics {
    FunctionMetrics {
        name: extracted.name.clone(),
        file: file_path.to_path_buf(),
        line: extracted.line,
        cyclomatic: extracted.cyclomatic,
        cognitive: extracted.cognitive,
        nesting: extracted.nesting,
        length: extracted.length,

        // Purity from extraction
        is_pure: Some(extracted.purity_analysis.is_pure),
        purity_confidence: Some(extracted.purity_analysis.confidence),
        purity_level: Some(convert_purity_level(extracted.purity_analysis.purity_level)),

        // Metadata
        is_test: extracted.is_test,
        visibility: extracted.visibility.clone(),
        is_trait_method: extracted.is_trait_method,
        in_test_module: extracted.in_test_module,

        // These will be populated by other phases
        entropy_score: None,
        call_dependencies: None,
        upstream_callers: None,
        downstream_callees: None,
        detected_patterns: None,
        adjusted_complexity: None,
        composition_metrics: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        language_specific: None,
    }
}

fn convert_purity_level(level: PurityLevel) -> crate::core::PurityLevel {
    match level {
        PurityLevel::StrictlyPure => crate::core::PurityLevel::StrictlyPure,
        PurityLevel::LocallyPure => crate::core::PurityLevel::LocallyPure,
        PurityLevel::ReadOnly => crate::core::PurityLevel::ReadOnly,
        PurityLevel::Impure => crate::core::PurityLevel::Impure,
    }
}

/// Convert all functions in extracted file to FunctionMetrics.
pub fn all_function_metrics(extracted: &ExtractedFileData) -> Vec<FunctionMetrics> {
    extracted.functions
        .iter()
        .map(|f| to_function_metrics(&extracted.path, f))
        .collect()
}

/// Convert extracted file data to FileMetrics.
pub fn to_file_metrics(extracted: &ExtractedFileData) -> FileMetrics {
    let functions = all_function_metrics(extracted);

    FileMetrics {
        path: extracted.path.clone(),
        language: Language::Rust, // Could be parameterized
        complexity: crate::core::ComplexityMetrics {
            functions,
            total_cyclomatic: extracted.functions.iter().map(|f| f.cyclomatic as usize).sum(),
            total_cognitive: extracted.functions.iter().map(|f| f.cognitive as usize).sum(),
            max_nesting: extracted.functions.iter().map(|f| f.nesting).max().unwrap_or(0),
            function_count: extracted.functions.len(),
        },
        debt_items: vec![], // Populated by debt detection phase
        dependencies: vec![],
        duplications: vec![],
        total_lines: extracted.total_lines,
        module_scope: None, // Could extract from path
        classes: None,
    }
}

/// Convert all extracted files to function metrics.
pub fn all_metrics_from_extracted(
    extracted: &std::collections::HashMap<std::path::PathBuf, ExtractedFileData>,
) -> Vec<FunctionMetrics> {
    extracted
        .values()
        .flat_map(all_function_metrics)
        .collect()
}
```

### Call Graph Adapter

**File**: `src/extraction/adapters/call_graph.rs`

```rust
use crate::extraction::types::{ExtractedFileData, CallSite, CallType};
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashMap;
use std::path::PathBuf;

/// Build a CallGraph from extracted file data.
///
/// This is a pure function with no file I/O.
pub fn build_call_graph(
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> CallGraph {
    let mut graph = CallGraph::new();

    // First pass: add all function nodes
    for (path, file_data) in extracted {
        for func in &file_data.functions {
            let func_id = FunctionId::new(path.clone(), func.name.clone(), func.line);

            graph.add_function(
                func_id,
                is_entry_point(&func.name),
                func.is_test,
                func.cyclomatic,
                func.length,
            );
        }
    }

    // Second pass: add call edges
    for (path, file_data) in extracted {
        for func in &file_data.functions {
            let caller_id = FunctionId::new(path.clone(), func.name.clone(), func.line);

            for call in &func.calls {
                if let Some(callee_id) = resolve_call(path, call, extracted) {
                    graph.add_call(caller_id.clone(), callee_id);
                }
            }
        }
    }

    graph.resolve_cross_file_calls();
    graph
}

fn is_entry_point(name: &str) -> bool {
    name == "main"
        || name.starts_with("handle_")
        || name.starts_with("run_")
}

/// Resolve a call site to a FunctionId.
fn resolve_call(
    caller_file: &PathBuf,
    call: &CallSite,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> Option<FunctionId> {
    // Try same file first (most common case)
    if let Some(file_data) = extracted.get(caller_file) {
        if let Some(func) = find_function_by_name(file_data, &call.callee_name) {
            return Some(FunctionId::new(
                caller_file.clone(),
                func.name.clone(),
                func.line,
            ));
        }
    }

    // Try cross-file resolution
    // For method calls like "foo.bar()", we need to resolve the type
    match call.call_type {
        CallType::Direct | CallType::StaticMethod => {
            // Search all files for matching function
            for (path, file_data) in extracted {
                if let Some(func) = find_function_by_name(file_data, &call.callee_name) {
                    return Some(FunctionId::new(path.clone(), func.name.clone(), func.line));
                }
            }
        }
        CallType::Method | CallType::TraitMethod => {
            // Method calls are harder to resolve without type info
            // Return None and let cross_file_calls handle it
        }
        _ => {}
    }

    None
}

fn find_function_by_name<'a>(
    file_data: &'a ExtractedFileData,
    name: &str,
) -> Option<&'a crate::extraction::types::ExtractedFunctionData> {
    file_data.functions.iter().find(|f| {
        f.name == name || f.qualified_name == name || f.qualified_name.ends_with(&format!("::{}", name))
    })
}

/// Merge call graph data from extracted files into existing graph.
pub fn merge_into_call_graph(
    graph: &mut CallGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) {
    let new_graph = build_call_graph(extracted);
    graph.merge(new_graph);
}
```

### Data Flow Adapter

**File**: `src/extraction/adapters/data_flow.rs`

```rust
use crate::data_flow::{DataFlowGraph, PurityInfo, IOOperation as DFIOOperation};
use crate::extraction::types::{ExtractedFileData, IoOperation, IoType, TransformationPattern, PatternType};
use crate::priority::call_graph::FunctionId;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Populate a DataFlowGraph from extracted file data.
///
/// This is a pure function with no file I/O.
pub fn populate_data_flow(
    graph: &mut DataFlowGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> PopulationStats {
    let mut stats = PopulationStats::default();

    for (path, file_data) in extracted {
        for func in &file_data.functions {
            let func_id = FunctionId::new(path.clone(), func.name.clone(), func.line);

            // Purity info
            stats.purity_entries += populate_purity(graph, &func_id, &func.purity_analysis);

            // I/O operations
            stats.io_operations += populate_io(graph, &func_id, &func.io_operations);

            // Variable dependencies (from parameters)
            stats.variable_deps += populate_deps(graph, &func_id, &func.parameter_names);

            // Transformation patterns
            stats.transformations += populate_transformations(graph, &func_id, &func.transformation_patterns);
        }
    }

    stats
}

#[derive(Default)]
pub struct PopulationStats {
    pub purity_entries: usize,
    pub io_operations: usize,
    pub variable_deps: usize,
    pub transformations: usize,
}

fn populate_purity(
    graph: &mut DataFlowGraph,
    func_id: &FunctionId,
    purity: &crate::extraction::types::PurityAnalysisData,
) -> usize {
    let purity_info = PurityInfo {
        is_pure: purity.is_pure,
        confidence: purity.confidence,
        impurity_reasons: if !purity.is_pure {
            let mut reasons = Vec::new();
            if purity.has_mutations {
                reasons.push("Has mutations".to_string());
            }
            if purity.has_io_operations {
                reasons.push("Has I/O operations".to_string());
            }
            if purity.has_unsafe {
                reasons.push("Contains unsafe code".to_string());
            }
            reasons
        } else {
            vec![]
        },
    };

    graph.set_purity_info(func_id.clone(), purity_info);

    // Add mutations
    for mutation in &purity.local_mutations {
        graph.add_mutation(func_id.clone(), mutation.clone(), true);
    }
    for mutation in &purity.upvalue_mutations {
        graph.add_mutation(func_id.clone(), mutation.clone(), false);
    }

    1 + purity.local_mutations.len() + purity.upvalue_mutations.len()
}

fn populate_io(
    graph: &mut DataFlowGraph,
    func_id: &FunctionId,
    io_ops: &[IoOperation],
) -> usize {
    for op in io_ops {
        let df_op = convert_io_operation(op);
        graph.add_io_operation(func_id.clone(), df_op);
    }
    io_ops.len()
}

fn convert_io_operation(op: &IoOperation) -> DFIOOperation {
    match op.io_type {
        IoType::File => DFIOOperation::File(op.description.clone()),
        IoType::Console => DFIOOperation::Console(op.description.clone()),
        IoType::Network => DFIOOperation::Network(op.description.clone()),
        IoType::Database => DFIOOperation::Database(op.description.clone()),
        IoType::AsyncIO => DFIOOperation::AsyncIO(op.description.clone()),
        IoType::Environment => DFIOOperation::Environment(op.description.clone()),
        IoType::System => DFIOOperation::System(op.description.clone()),
    }
}

fn populate_deps(
    graph: &mut DataFlowGraph,
    func_id: &FunctionId,
    params: &[String],
) -> usize {
    if !params.is_empty() {
        let deps: HashSet<String> = params.iter().cloned().collect();
        graph.add_variable_dependencies(func_id.clone(), deps);
        params.len()
    } else {
        0
    }
}

fn populate_transformations(
    graph: &mut DataFlowGraph,
    func_id: &FunctionId,
    patterns: &[TransformationPattern],
) -> usize {
    for pattern in patterns {
        let pattern_name = match pattern.pattern_type {
            PatternType::Map => "map",
            PatternType::Filter => "filter",
            PatternType::Fold => "fold",
            PatternType::FlatMap => "flat_map",
            PatternType::Collect => "collect",
            PatternType::ForEach => "for_each",
            PatternType::Find => "find",
            PatternType::Any => "any",
            PatternType::All => "all",
            PatternType::Reduce => "reduce",
        };
        graph.add_transformation(func_id.clone(), pattern_name.to_string());
    }
    patterns.len()
}
```

### God Object Adapter

**File**: `src/extraction/adapters/god_object.rs`

```rust
use crate::extraction::types::{ExtractedFileData, ExtractedStructData, ExtractedImplData};
use crate::organization::GodObjectAnalysis;
use crate::priority::score_types::Score0To100;
use std::collections::HashMap;
use std::path::Path;

/// Analyze for god objects from extracted data.
///
/// This is a pure function with no file I/O.
pub fn analyze_god_object(
    path: &Path,
    extracted: &ExtractedFileData,
) -> Option<GodObjectAnalysis> {
    // Check if this file qualifies as a potential god object
    let total_methods: usize = extracted.impls.iter().map(|i| i.methods.len()).sum();
    let total_fields: usize = extracted.structs.iter().map(|s| s.fields.len()).sum();

    // Thresholds for god object detection
    if total_methods < 20 && extracted.total_lines < 500 {
        return None;
    }

    // Calculate god object score
    let method_score = (total_methods as f64 / 50.0 * 50.0).min(50.0);
    let loc_score = (extracted.total_lines as f64 / 2000.0 * 50.0).min(50.0);
    let god_score = method_score + loc_score;

    // Estimate responsibilities based on impl blocks
    let responsibilities: Vec<String> = extracted.impls
        .iter()
        .map(|impl_block| {
            impl_block.trait_name
                .clone()
                .unwrap_or_else(|| impl_block.type_name.clone())
        })
        .collect();

    let responsibility_method_counts: HashMap<String, usize> = extracted.impls
        .iter()
        .map(|impl_block| {
            let name = impl_block.trait_name
                .clone()
                .unwrap_or_else(|| impl_block.type_name.clone());
            (name, impl_block.methods.len())
        })
        .collect();

    let is_god_object = god_score > 50.0 || total_methods > 50 || extracted.total_lines > 2000;

    if !is_god_object {
        return None;
    }

    Some(GodObjectAnalysis {
        is_god_object: true,
        method_count: total_methods,
        field_count: total_fields,
        responsibility_count: responsibilities.len(),
        lines_of_code: extracted.total_lines,
        complexity_sum: extracted.functions.iter().map(|f| f.cyclomatic as usize).sum(),
        god_object_score: Score0To100::new(god_score),
        recommended_splits: vec![], // Could generate recommendations
        confidence: crate::organization::GodObjectConfidence::Probable,
        responsibilities,
        responsibility_method_counts,
        purity_distribution: None,
        module_structure: None,
        detection_type: crate::organization::DetectionType::GodFile,
        struct_name: extracted.structs.first().map(|s| s.name.clone()),
        struct_line: extracted.structs.first().map(|s| s.line),
        struct_location: None,
        visibility_breakdown: None,
        domain_count: 0,
        domain_diversity: 0.0,
        struct_ratio: 0.0,
        analysis_method: Default::default(),
        cross_domain_severity: None,
        domain_diversity_metrics: None,
        aggregated_entropy: None,
        aggregated_error_swallowing_count: None,
        aggregated_error_swallowing_patterns: None,
        layering_impact: None,
        anti_pattern_report: None,
    })
}
```

## Dependencies

- **Prerequisites**: Specs 211, 212
- **Affected Components**: Each adapter creates types from existing modules

## Testing Strategy

- **Unit Tests**: Each adapter function tested with synthetic ExtractedFileData
- **Equivalence Tests**: Compare adapter output with direct parsing output
- **Property Tests**: Verify adapters handle edge cases (empty files, no functions)

## Documentation Requirements

- **Code Documentation**: Rustdoc on all adapter functions
- **Examples**: Example conversions in module docs

## Implementation Notes

- All adapters are pure functions - no file I/O or parsing
- Use `impl From<T>` traits where appropriate for cleaner conversions
- Consider using `Cow<str>` for string fields if cloning becomes expensive

## Migration and Compatibility

No migration needed. Adapters are additive and used by spec 213 integration.
