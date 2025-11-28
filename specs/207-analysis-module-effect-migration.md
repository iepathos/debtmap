---
number: 207
title: Analysis Module Effect Migration
category: foundation
priority: medium
status: draft
dependencies: [195, 196, 197, 198, 203]
created: 2025-11-27
---

# Specification 207: Analysis Module Effect Migration

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 195-198 (stillwater foundation), 203 (traverse pattern)

## Context

The `src/analysis/` directory contains core analysis algorithms that currently use `anyhow::Result` directly. This creates inconsistency with the stillwater-based code in other modules and prevents:

1. **Testability** - Analysis functions can't use `DebtmapTestEnv`
2. **Error Accumulation** - Can't collect all analysis errors
3. **Reader Pattern** - Config must be threaded as parameters
4. **Composability** - Can't chain with other effects naturally

Key modules needing migration:
- `multi_pass.rs` - Multi-pass analysis orchestration
- `call_graph/*.rs` - Call graph construction
- `python_call_graph/*.rs` - Python-specific call analysis
- `purity_propagation/*.rs` - Purity analysis
- `diagnostics/*.rs` - Diagnostic generation

## Objective

Migrate core analysis modules to use stillwater's Effect pattern, enabling consistent testability, error handling, and composition across the codebase.

## Requirements

### Functional Requirements

1. **Effect-Based Analysis Functions**
   - Convert analysis functions to return `AnalysisEffect<T>`
   - Use Reader pattern for configuration access
   - Support both sync and async analysis operations

2. **Error Accumulation for Validation**
   - Use `AnalysisValidation` for multi-file validation
   - Collect all analysis warnings and errors
   - Provide comprehensive analysis reports

3. **Backwards Compatibility**
   - Provide `_result` wrappers for existing callers
   - Maintain existing public API signatures
   - Support gradual migration

### Non-Functional Requirements

1. **Performance**
   - No significant overhead from Effect wrapping
   - Support parallel analysis where applicable
   - Maintain memory efficiency

2. **Testability**
   - All migrated functions testable with `DebtmapTestEnv`
   - No file system access required in unit tests
   - Easy to mock dependencies

## Acceptance Criteria

- [ ] Migrate `multi_pass.rs` to effect-based pattern
- [ ] Migrate `call_graph/mod.rs` core functions
- [ ] Migrate `diagnostics/mod.rs` to effect-based pattern
- [ ] Add Reader pattern for configuration access in analysis
- [ ] Create `analyze_project_effect` orchestrating function
- [ ] Update tests to use `DebtmapTestEnv`
- [ ] Backwards-compatible wrappers for all public functions
- [ ] Documentation with migration examples

## Technical Details

### Implementation Approach

#### 1. Multi-Pass Analysis as Effects

```rust
// In src/analysis/multi_pass.rs
use stillwater::effect::prelude::*;
use crate::effects::{AnalysisEffect, asks_config, asks_thresholds};

/// Multi-pass analysis result containing all passes.
#[derive(Debug, Clone)]
pub struct MultiPassResult {
    pub complexity_pass: ComplexityPassResult,
    pub call_graph_pass: CallGraphPassResult,
    pub debt_detection_pass: DebtDetectionResult,
}

/// Run multi-pass analysis as an Effect.
pub fn analyze_multi_pass_effect(
    files: Vec<ParsedFile>,
) -> AnalysisEffect<MultiPassResult> {
    // Use Reader pattern to access config
    asks_config(move |config| {
        let files = files.clone();
        let config = config.clone();

        // Chain passes using and_then
        run_complexity_pass_effect(files.clone())
            .and_then(move |complexity| {
                run_call_graph_pass_effect(files.clone(), &complexity)
                    .and_then(move |call_graph| {
                        run_debt_detection_effect(files, &complexity, &call_graph, &config)
                            .map(move |debt| MultiPassResult {
                                complexity_pass: complexity,
                                call_graph_pass: call_graph,
                                debt_detection_pass: debt,
                            })
                    })
            })
    }).and_then(|effect| effect).boxed()
}

/// Run complexity analysis pass.
fn run_complexity_pass_effect(
    files: Vec<ParsedFile>,
) -> AnalysisEffect<ComplexityPassResult> {
    from_fn(move |env: &RealEnv| {
        let thresholds = env.config().thresholds.as_ref();

        let results: Vec<FileComplexity> = files
            .par_iter()
            .map(|file| analyze_file_complexity(file, thresholds))
            .collect();

        Ok(ComplexityPassResult { files: results })
    }).boxed()
}
```

#### 2. Call Graph Analysis as Effects

```rust
// In src/analysis/call_graph/mod.rs
use stillwater::effect::prelude::*;

/// Build project-wide call graph as an Effect.
pub fn build_call_graph_effect(
    files: Vec<ParsedFile>,
) -> AnalysisEffect<ProjectCallGraph> {
    from_fn(move |env: &RealEnv| {
        let mut graph = ProjectCallGraph::new();

        // Build per-file graphs
        for file in &files {
            let file_graph = build_file_call_graph(file)?;
            graph.merge(file_graph);
        }

        // Resolve cross-file references
        resolve_cross_module_calls(&mut graph, &files)?;

        Ok(graph)
    }).boxed()
}

/// Build call graph for a single file.
fn build_file_call_graph(file: &ParsedFile) -> Result<FileCallGraph, AnalysisError> {
    // Pure function - no Effect needed
    let mut graph = FileCallGraph::new(file.path.clone());

    for function in &file.functions {
        let calls = extract_function_calls(function)?;
        graph.add_function(function.name.clone(), calls);
    }

    Ok(graph)
}
```

#### 3. Diagnostics Generation as Effects

```rust
// In src/analysis/diagnostics/mod.rs
use stillwater::effect::prelude::*;

/// Generate diagnostics from analysis results.
pub fn generate_diagnostics_effect(
    analysis: &AnalysisResults,
) -> AnalysisEffect<Vec<Diagnostic>> {
    asks_config(move |config| {
        let analysis = analysis.clone();
        let config = config.clone();

        from_fn(move |_env: &RealEnv| {
            let mut diagnostics = Vec::new();

            // Generate complexity diagnostics
            diagnostics.extend(generate_complexity_diagnostics(&analysis, &config));

            // Generate coverage diagnostics
            diagnostics.extend(generate_coverage_diagnostics(&analysis, &config));

            // Generate debt diagnostics
            diagnostics.extend(generate_debt_diagnostics(&analysis, &config));

            Ok(diagnostics)
        })
    }).and_then(|effect| effect).boxed()
}

/// Pure function to generate complexity diagnostics.
fn generate_complexity_diagnostics(
    analysis: &AnalysisResults,
    config: &DebtmapConfig,
) -> Vec<Diagnostic> {
    let threshold = config.thresholds
        .as_ref()
        .and_then(|t| t.complexity)
        .unwrap_or(10);

    analysis.files
        .iter()
        .flat_map(|file| {
            file.functions
                .iter()
                .filter(|f| f.complexity > threshold)
                .map(|f| Diagnostic {
                    level: DiagnosticLevel::Warning,
                    message: format!(
                        "Function '{}' has complexity {} (threshold: {})",
                        f.name, f.complexity, threshold
                    ),
                    location: f.location.clone(),
                    category: DiagnosticCategory::Complexity,
                })
        })
        .collect()
}
```

#### 4. Project Analysis Orchestration

```rust
// In src/analyzers/mod.rs
use stillwater::effect::prelude::*;
use stillwater::traverse::traverse_effect;

/// Orchestrate full project analysis as an Effect.
pub fn analyze_project_effect(
    project_root: PathBuf,
) -> AnalysisEffect<ProjectAnalysis> {
    // Discover files
    walk_dir_with_config_effect(project_root.clone(), supported_languages())
        // Parse all files
        .and_then(|files| parse_files_effect(files))
        // Run multi-pass analysis
        .and_then(|parsed| analyze_multi_pass_effect(parsed))
        // Generate diagnostics
        .and_then(|analysis| {
            generate_diagnostics_effect(&analysis.into())
                .map(move |diagnostics| ProjectAnalysis {
                    results: analysis,
                    diagnostics,
                })
        })
        .boxed()
}

/// Parse multiple files using traverse_effect.
fn parse_files_effect(paths: Vec<PathBuf>) -> AnalysisEffect<Vec<ParsedFile>> {
    traverse_effect(paths, |path| parse_file_effect(path))
}

/// Parse a single file as an Effect.
fn parse_file_effect(path: PathBuf) -> AnalysisEffect<ParsedFile> {
    read_file_effect(path.clone())
        .and_then(move |content| {
            from_fn(move |_env: &RealEnv| {
                let language = Language::from_path(&path)
                    .ok_or_else(|| AnalysisError::parse(
                        format!("Unsupported file type: {}", path.display())
                    ))?;

                let parser = get_parser_for_language(language);
                parser.parse(&content, &path)
                    .map(|ast| ParsedFile { path, content, ast, language })
            })
        })
        .boxed()
}
```

### Architecture Changes

1. **Modified Module**: `src/analysis/multi_pass.rs`
   - Effect-based pass orchestration
   - Reader pattern for config

2. **Modified Module**: `src/analysis/call_graph/mod.rs`
   - Effect wrappers for graph building
   - Pure core functions

3. **Modified Module**: `src/analysis/diagnostics/mod.rs`
   - Effect-based diagnostic generation
   - Pure diagnostic functions

4. **Modified Module**: `src/analyzers/mod.rs`
   - Effect-based project analysis
   - Integration with traverse pattern

5. **New Module**: `src/analysis/effects.rs`
   - Shared effect utilities for analysis
   - Common error handling patterns

### Pure vs Effect Boundary

**Keep as Pure Functions** (no I/O):
- `analyze_file_complexity` - AST analysis
- `build_file_call_graph` - Graph construction
- `generate_complexity_diagnostics` - Diagnostic generation
- `calculate_debt_score` - Score calculation

**Convert to Effects** (need I/O or config):
- `analyze_project_effect` - File discovery, reading
- `build_call_graph_effect` - May need file reads
- `generate_diagnostics_effect` - Needs config access

### Data Structures

```rust
/// Parsed file ready for analysis.
#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub path: PathBuf,
    pub content: String,
    pub ast: Ast,
    pub language: Language,
}

/// Complete project analysis results.
#[derive(Debug, Clone)]
pub struct ProjectAnalysis {
    pub results: MultiPassResult,
    pub diagnostics: Vec<Diagnostic>,
}

/// Diagnostic from analysis.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub location: SourceLocation,
    pub category: DiagnosticCategory,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 195 (stillwater foundation)
  - Spec 196 (pure function extraction)
  - Spec 197 (validation)
  - Spec 198 (effect composition)
  - Spec 203 (traverse pattern)

- **Affected Components**:
  - `src/analysis/multi_pass.rs`
  - `src/analysis/call_graph/*.rs`
  - `src/analysis/diagnostics/*.rs`
  - `src/analyzers/mod.rs`

## Testing Strategy

### Unit Tests

```rust
#[tokio::test]
async fn test_multi_pass_analysis() {
    let env = DebtmapTestEnv::new()
        .with_file("main.rs", "fn complex() { if true { if true { } } }")
        .with_config(ConfigBuilder::new().complexity_threshold(5));

    let files = vec![ParsedFile {
        path: "main.rs".into(),
        content: "fn complex() { if true { if true { } } }".into(),
        ast: parse_test_code("fn complex() { if true { if true { } } }"),
        language: Language::Rust,
    }];

    let effect = analyze_multi_pass_effect(files);
    let result = run_effect_with_env(effect, &env).await;

    assert!(result.is_ok());
    let analysis = result.unwrap();
    assert!(!analysis.complexity_pass.files.is_empty());
}

#[tokio::test]
async fn test_diagnostics_generation() {
    let env = DebtmapTestEnv::new()
        .with_config(ConfigBuilder::new().complexity_threshold(5));

    let analysis = create_test_analysis_with_high_complexity();
    let effect = generate_diagnostics_effect(&analysis);
    let result = run_effect_with_env(effect, &env).await;

    assert!(result.is_ok());
    let diagnostics = result.unwrap();
    assert!(diagnostics.iter().any(|d| d.category == DiagnosticCategory::Complexity));
}
```

### Integration Tests
- Test full project analysis workflow
- Test with various project structures
- Test error handling and accumulation

### Performance Tests
- Benchmark effect vs direct function calls
- Measure memory usage
- Test with large codebases

## Documentation Requirements

- **Code Documentation**: Document all effect-based analysis functions
- **User Documentation**: Update architecture documentation
- **Architecture Updates**: Update DESIGN.md with effect patterns

## Implementation Notes

1. **Pure Core Pattern**: Keep AST manipulation and calculation logic pure. Only wrap I/O and config access in Effects.

2. **Incremental Migration**: Migrate one module at a time, maintaining backwards compatibility.

3. **Performance**: Use `from_fn` for sync operations to avoid async overhead when not needed.

4. **Error Context**: Include file paths and locations in all analysis errors.

## Migration and Compatibility

- **Backwards Compatible**: All public functions get `_result` wrappers
- **Gradual Migration**: Internal code can migrate incrementally
- **No API Changes**: External API remains stable

```rust
// Backwards-compatible wrapper
pub fn analyze_project(
    project_root: PathBuf,
    config: DebtmapConfig,
) -> anyhow::Result<ProjectAnalysis> {
    run_effect(analyze_project_effect(project_root), config)
}
```

## Migration Order

1. **Phase 1**: `src/analysis/diagnostics/` - Least dependencies
2. **Phase 2**: `src/analysis/call_graph/` - Build foundation
3. **Phase 3**: `src/analysis/multi_pass.rs` - Orchestration layer
4. **Phase 4**: `src/analyzers/` integration - Full pipeline
