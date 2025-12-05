---
number: 209
title: Composable Pipeline Architecture
category: foundation
priority: high
status: draft
dependencies: [207, 208]
created: 2025-12-05
---

# Specification 209: Composable Pipeline Architecture

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 207 (Stillwater Effects), Spec 208 (Pure Functions)

## Context

With Stillwater effects (Spec 207) and pure functions (Spec 208) in place, we have the building blocks for a composable pipeline architecture. However, the current `perform_unified_analysis_computation` function hard-codes a specific sequence of operations:

1. Discover files → 2. Parse metrics → 3. Build call graph → 4. Resolve traits → 5. Load coverage → 6. Analyze purity → 7. Load context → 8. Detect debt → 9. Score and prioritize

**Current Limitations**:
- **Inflexible**: Can't skip optional stages (coverage, context)
- **Not Reusable**: Can't compose alternative pipelines (e.g., just call graph + purity)
- **Progress Reporting Mixed In**: Progress tied to specific pipeline structure
- **Hard to Test**: Can't test individual pipeline compositions
- **No Parallelization Control**: Parallel execution hard-coded internally

**User Needs**:
- Skip coverage loading when not needed
- Disable context-aware analysis for faster runs
- Run partial analysis (e.g., just complexity metrics)
- Compose custom pipelines for specific use cases
- Control parallelization at pipeline level

## Objective

Create a composable pipeline architecture that:
1. **Separates Stage Definition from Execution** - Stages are composable units
2. **Supports Optional Stages** - Conditionally include/exclude stages
3. **Enables Custom Pipelines** - Users can compose their own analysis flows
4. **Provides Progress Tracking** - Automatic progress for any pipeline composition
5. **Allows Parallel Execution** - Control parallelism at pipeline or stage level
6. **Maintains Type Safety** - Compiler ensures correct data flow between stages

**Success Criteria**: A `Pipeline` builder that composes effects and pure functions into reusable, configurable analysis workflows.

## Requirements

### Functional Requirements

1. **Pipeline Stage Abstraction**
   - Define trait for pipeline stages: `Stage<Input, Output>`
   - Stages can be pure transformations or effects
   - Stages composable with `.then()` operator
   - Type-safe composition (output type matches next input type)

2. **Standard Pipeline Stages**
   - `FileDiscoveryStage`: Discover project files (effect)
   - `ParsingStage`: Parse files to metrics (effect)
   - `CallGraphStage`: Build call graph (pure)
   - `TraitResolutionStage`: Resolve trait calls (mixed: load + pure)
   - `CoverageLoadingStage`: Load coverage data (effect, optional)
   - `PurityAnalysisStage`: Analyze function purity (pure)
   - `ContextLoadingStage`: Load project context (effect, optional)
   - `DebtDetectionStage`: Detect technical debt (pure)
   - `ScoringStage`: Score and prioritize debt (pure)

3. **Pipeline Builder**
   - Fluent API: `Pipeline::new().stage(...).stage(...).build()`
   - Optional stages: `.when(condition, stage)`
   - Parallel stages: `.parallel(vec![stage1, stage2])`
   - Progress naming: `.with_progress("Phase 1/9: Discovering files")`
   - Type-safe construction (compile-time validation)

4. **Standard Pipeline Configurations**
   - `standard_pipeline()`: Full analysis with all stages
   - `fast_pipeline()`: Skip coverage and context
   - `complexity_only_pipeline()`: Just complexity analysis
   - `call_graph_pipeline()`: Call graph + purity
   - `custom_pipeline(config)`: User-defined configuration

5. **Pipeline Execution**
   - `pipeline.execute(env)`: Run all stages sequentially
   - `pipeline.execute_parallel(env, jobs)`: Parallel where possible
   - Progress reporting automatic from stage annotations
   - Error handling with context at each stage
   - Timing information for each stage

6. **Pipeline Composition**
   - Compose pipelines: `pipeline1.then(pipeline2)`
   - Share intermediate results: `pipeline.checkpoint()`
   - Branch pipelines: `pipeline.branch(condition, alt_pipeline)`
   - Merge pipelines: `Pipeline::merge(pipeline1, pipeline2)`

### Non-Functional Requirements

1. **Type Safety**
   - Compiler ensures correct data flow between stages
   - Impossible to compose incompatible stages
   - Type inference for fluent API
   - Clear error messages for type mismatches

2. **Performance**
   - Zero-cost abstractions (no runtime overhead)
   - Efficient parallel execution (rayon integration)
   - Minimal intermediate allocations
   - Lazy evaluation where appropriate

3. **Usability**
   - Intuitive fluent API
   - Clear stage naming and organization
   - Good defaults for common use cases
   - Easy to customize for specific needs

4. **Extensibility**
   - Users can define custom stages
   - Plugins can add new pipeline configurations
   - Hook system for pre/post stage actions
   - Support for third-party integrations

## Acceptance Criteria

- [ ] `Stage<Input, Output>` trait defined in `src/pipeline/stage.rs`
- [ ] Pipeline builder implemented in `src/pipeline/builder.rs`
- [ ] All 9 standard stages implemented in `src/pipeline/stages/`
- [ ] Standard pipeline configurations in `src/pipeline/configs.rs`
- [ ] `standard_pipeline()` produces identical results to current implementation
- [ ] Optional stages (coverage, context) can be skipped via configuration
- [ ] Custom pipeline example in `examples/custom_pipeline.rs`
- [ ] Parallel execution support with configurable job count
- [ ] Progress reporting automatic for all pipeline compositions
- [ ] Type-safe composition (incompatible stages cause compile errors)
- [ ] Performance benchmarks show < 5% overhead vs direct implementation
- [ ] Documentation in `ARCHITECTURE.md` with pipeline examples
- [ ] All existing tests pass with new pipeline architecture

## Technical Details

### Implementation Approach

#### Phase 1: Core Pipeline Abstractions

```rust
// src/pipeline/stage.rs

use crate::pipeline::effects::AnalysisEffect;

/// A pipeline stage that transforms data
pub trait Stage {
    type Input;
    type Output;
    type Error;

    /// Execute this stage
    fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;

    /// Get the stage name for progress reporting
    fn name(&self) -> &str;
}

/// A pure stage (no I/O)
pub struct PureStage<F, I, O> {
    name: String,
    func: F,
    _phantom: PhantomData<(I, O)>,
}

impl<F, I, O> PureStage<F, I, O>
where
    F: Fn(I) -> O,
{
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            func,
            _phantom: PhantomData,
        }
    }
}

impl<F, I, O> Stage for PureStage<F, I, O>
where
    F: Fn(I) -> O,
{
    type Input = I;
    type Output = O;
    type Error = std::convert::Infallible;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        Ok((self.func)(input))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// An effect stage (performs I/O)
pub struct EffectStage<I, O> {
    name: String,
    effect: Box<dyn Fn(I) -> AnalysisEffect<O>>,
}

impl<I, O> EffectStage<I, O> {
    pub fn new(
        name: impl Into<String>,
        effect: impl Fn(I) -> AnalysisEffect<O> + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            effect: Box::new(effect),
        }
    }
}

impl<I, O> Stage for EffectStage<I, O> {
    type Input = I;
    type Output = O;
    type Error = AnalysisError;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        let effect = (self.effect)(input);
        // Will be executed with environment at pipeline level
        Ok(effect) // Return effect for later execution
    }

    fn name(&self) -> &str {
        &self.name
    }
}
```

#### Phase 2: Pipeline Builder

```rust
// src/pipeline/builder.rs

use super::stage::Stage;
use super::effects::{AnalysisEnv, AnalysisEffect};
use std::marker::PhantomData;

/// Pipeline builder for composing analysis stages
pub struct Pipeline<T> {
    stages: Vec<Box<dyn Stage<Input = (), Output = T>>>,
    progress_enabled: bool,
    parallel_enabled: bool,
    jobs: Option<usize>,
}

impl Pipeline<()> {
    /// Create a new empty pipeline
    pub fn new() -> PipelineBuilder<()> {
        PipelineBuilder {
            stages: vec![],
            _phantom: PhantomData,
        }
    }

    /// Create standard full analysis pipeline
    pub fn standard(config: &AnalyzeConfig) -> PipelineBuilder<UnifiedAnalysis> {
        Self::new()
            .stage(FileDiscoveryStage::new(&config.path, &config.languages))
            .stage(ParsingStage::new())
            .stage(CallGraphStage::new())
            .stage(TraitResolutionStage::new(&config.path))
            .when(config.coverage_file.is_some(), |p| {
                p.stage(CoverageLoadingStage::new(config.coverage_file.as_ref().unwrap()))
            })
            .stage(PurityAnalysisStage::new())
            .when(config.enable_context, |p| {
                p.stage(ContextLoadingStage::new(&config.path))
            })
            .stage(DebtDetectionStage::new(&config.thresholds))
            .stage(ScoringStage::new())
    }

    /// Create fast pipeline (skip coverage and context)
    pub fn fast(config: &AnalyzeConfig) -> PipelineBuilder<UnifiedAnalysis> {
        Self::new()
            .stage(FileDiscoveryStage::new(&config.path, &config.languages))
            .stage(ParsingStage::new())
            .stage(CallGraphStage::new())
            .stage(PurityAnalysisStage::new())
            .stage(DebtDetectionStage::new(&config.thresholds))
            .stage(ScoringStage::new())
    }

    /// Create complexity-only pipeline
    pub fn complexity_only(config: &AnalyzeConfig) -> PipelineBuilder<ComplexityReport> {
        Self::new()
            .stage(FileDiscoveryStage::new(&config.path, &config.languages))
            .stage(ParsingStage::new())
            .stage(ComplexityAnalysisStage::new(&config.thresholds))
    }
}

/// Builder for constructing pipelines
pub struct PipelineBuilder<T> {
    stages: Vec<Box<dyn AnyStage>>,
    _phantom: PhantomData<T>,
}

impl<T> PipelineBuilder<T> {
    /// Add a stage to the pipeline
    pub fn stage<S>(mut self, stage: S) -> PipelineBuilder<S::Output>
    where
        S: Stage<Input = T> + 'static,
    {
        self.stages.push(Box::new(stage));
        PipelineBuilder {
            stages: self.stages,
            _phantom: PhantomData,
        }
    }

    /// Add a stage conditionally
    pub fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }

    /// Enable progress reporting
    pub fn with_progress(mut self) -> Self {
        self.progress_enabled = true;
        self
    }

    /// Enable parallel execution
    pub fn parallel(mut self, jobs: usize) -> Self {
        self.parallel_enabled = true;
        self.jobs = Some(jobs);
        self
    }

    /// Build the final pipeline
    pub fn build(self) -> BuiltPipeline<T> {
        BuiltPipeline {
            stages: self.stages,
            progress_enabled: self.progress_enabled,
            parallel_enabled: self.parallel_enabled,
            jobs: self.jobs,
            _phantom: PhantomData,
        }
    }

    /// Execute the pipeline immediately
    pub fn execute(self, env: &AnalysisEnv) -> Result<T, AnalysisError> {
        self.build().execute(env)
    }
}

/// Built pipeline ready for execution
pub struct BuiltPipeline<T> {
    stages: Vec<Box<dyn AnyStage>>,
    progress_enabled: bool,
    parallel_enabled: bool,
    jobs: Option<usize>,
    _phantom: PhantomData<T>,
}

impl<T> BuiltPipeline<T> {
    /// Execute the pipeline with the given environment
    pub fn execute(&self, env: &AnalysisEnv) -> Result<T, AnalysisError> {
        let mut data: Box<dyn Any> = Box::new(());

        // Report total number of stages if progress enabled
        if self.progress_enabled {
            if let Some(ref progress) = env.progress {
                progress.init_stages(self.stages.len());
            }
        }

        // Execute each stage in sequence
        for (i, stage) in self.stages.iter().enumerate() {
            if self.progress_enabled {
                if let Some(ref progress) = env.progress {
                    progress.start_stage(i, stage.name());
                }
            }

            let result = stage.execute_any(data)?;
            data = result;

            if self.progress_enabled {
                if let Some(ref progress) = env.progress {
                    progress.complete_stage(i);
                }
            }
        }

        // Downcast final result
        data.downcast::<T>()
            .map(|b| *b)
            .map_err(|_| AnalysisError::Internal("Type mismatch in pipeline".into()))
    }

    /// Get timing information for each stage
    pub fn execute_with_timing(
        &self,
        env: &AnalysisEnv,
    ) -> Result<(T, Vec<StageTiming>), AnalysisError> {
        let mut data: Box<dyn Any> = Box::new(());
        let mut timings = vec![];

        for (i, stage) in self.stages.iter().enumerate() {
            let start = std::time::Instant::now();

            if self.progress_enabled {
                if let Some(ref progress) = env.progress {
                    progress.start_stage(i, stage.name());
                }
            }

            let result = stage.execute_any(data)?;
            data = result;

            let elapsed = start.elapsed();
            timings.push(StageTiming {
                name: stage.name().to_string(),
                duration: elapsed,
            });

            if self.progress_enabled {
                if let Some(ref progress) = env.progress {
                    progress.complete_stage(i);
                }
            }
        }

        let result = data
            .downcast::<T>()
            .map(|b| *b)
            .map_err(|_| AnalysisError::Internal("Type mismatch in pipeline".into()))?;

        Ok((result, timings))
    }
}
```

#### Phase 3: Standard Pipeline Stages

```rust
// src/pipeline/stages/file_discovery.rs

use crate::pipeline::stage::{Stage, EffectStage};
use crate::pipeline::effects::*;

/// Stage for discovering project files
pub struct FileDiscoveryStage {
    path: PathBuf,
    languages: Vec<Language>,
}

impl FileDiscoveryStage {
    pub fn new(path: &Path, languages: &[Language]) -> Self {
        Self {
            path: path.to_path_buf(),
            languages: languages.to_vec(),
        }
    }
}

impl Stage for FileDiscoveryStage {
    type Input = ();
    type Output = Vec<PathBuf>;
    type Error = AnalysisError;

    fn execute(&self, _input: Self::Input) -> AnalysisEffect<Self::Output> {
        discover_files(&self.path, &self.languages)
    }

    fn name(&self) -> &str {
        "File Discovery"
    }
}
```

```rust
// src/pipeline/stages/call_graph.rs

use crate::pipeline::stage::{Stage, PureStage};

/// Stage for building call graph from metrics
pub struct CallGraphStage;

impl CallGraphStage {
    pub fn new() -> Self {
        Self
    }
}

impl Stage for CallGraphStage {
    type Input = Vec<FunctionMetrics>;
    type Output = (Vec<FunctionMetrics>, CallGraph);
    type Error = std::convert::Infallible;

    fn execute(&self, metrics: Self::Input) -> Result<Self::Output, Self::Error> {
        let graph = crate::pipeline::stages::build_call_graph(&metrics);
        Ok((metrics, graph))
    }

    fn name(&self) -> &str {
        "Call Graph Construction"
    }
}
```

```rust
// src/pipeline/stages/debt_detection.rs

use crate::pipeline::stage::Stage;

/// Stage for detecting technical debt
pub struct DebtDetectionStage {
    thresholds: Thresholds,
}

impl DebtDetectionStage {
    pub fn new(thresholds: &Thresholds) -> Self {
        Self {
            thresholds: thresholds.clone(),
        }
    }
}

impl Stage for DebtDetectionStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = std::convert::Infallible;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        let debt_items: Vec<DebtItem> = data
            .metrics
            .iter()
            .flat_map(|m| {
                crate::pipeline::stages::debt::detect_all_debt(m, &self.thresholds)
            })
            .collect();

        data.debt_items = debt_items;
        Ok(data)
    }

    fn name(&self) -> &str {
        "Debt Detection"
    }
}
```

#### Phase 4: Pipeline Data Structure

```rust
// src/pipeline/data.rs

/// Data flowing through the analysis pipeline
#[derive(Clone)]
pub struct PipelineData {
    pub files: Vec<PathBuf>,
    pub metrics: Vec<FunctionMetrics>,
    pub call_graph: Option<CallGraph>,
    pub coverage: Option<CoverageData>,
    pub purity: Option<PurityAnalysis>,
    pub context: Option<ProjectContext>,
    pub debt_items: Vec<DebtItem>,
    pub scored_items: Vec<PrioritizedDebt>,
}

impl PipelineData {
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self {
            files,
            metrics: vec![],
            call_graph: None,
            coverage: None,
            purity: None,
            context: None,
            debt_items: vec![],
            scored_items: vec![],
        }
    }

    pub fn with_metrics(mut self, metrics: Vec<FunctionMetrics>) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn with_call_graph(mut self, graph: CallGraph) -> Self {
        self.call_graph = Some(graph);
        self
    }

    pub fn with_coverage(mut self, coverage: CoverageData) -> Self {
        self.coverage = Some(coverage);
        self
    }

    // ... other builder methods
}
```

#### Phase 5: Integration with Existing Code

```rust
// src/commands/analyze.rs

use crate::pipeline::{Pipeline, AnalysisEnv};

pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    // Create environment
    let env = AnalysisEnv {
        project_path: config.path.clone(),
        progress: Some(ProgressReporter::new()),
        config: config.clone(),
    };

    // Build and execute pipeline
    let pipeline = Pipeline::standard(&config)
        .with_progress()
        .when(config.parallel, |p| p.parallel(config.jobs));

    let (analysis, timings) = pipeline
        .build()
        .execute_with_timing(&env)?;

    // Print timing information in verbose mode
    if config.verbosity > 0 {
        for timing in timings {
            println!("{}: {:?}", timing.name, timing.duration);
        }
    }

    // Output results
    output::write_analysis(&analysis, &config)?;

    Ok(())
}
```

### Architecture Changes

**Before**:
```
analyze.rs
  └─> perform_unified_analysis_computation()
       └─> Hard-coded sequence of operations
           Mixed I/O + logic
           453 lines
```

**After**:
```
analyze.rs
  └─> Pipeline::standard(config)
       .with_progress()
       .parallel(jobs)
       .execute(env)

Pipeline composes:
  FileDiscoveryStage (effect)
    → ParsingStage (effect)
    → CallGraphStage (pure)
    → TraitResolutionStage (mixed)
    → [CoverageLoadingStage] (optional effect)
    → PurityAnalysisStage (pure)
    → [ContextLoadingStage] (optional effect)
    → DebtDetectionStage (pure)
    → ScoringStage (pure)
```

### APIs and Interfaces

#### Public Pipeline API

```rust
pub mod pipeline {
    // Core types
    pub use builder::{Pipeline, PipelineBuilder, BuiltPipeline};
    pub use stage::Stage;
    pub use data::PipelineData;

    // Standard configurations
    pub use configs::{
        standard_pipeline,
        fast_pipeline,
        complexity_only_pipeline,
        call_graph_pipeline,
    };

    // Stages
    pub use stages::{
        FileDiscoveryStage,
        ParsingStage,
        CallGraphStage,
        TraitResolutionStage,
        CoverageLoadingStage,
        PurityAnalysisStage,
        ContextLoadingStage,
        DebtDetectionStage,
        ScoringStage,
    };
}
```

#### Custom Pipeline Example

```rust
// User-defined custom pipeline
let pipeline = Pipeline::new()
    .stage(FileDiscoveryStage::new(&path, &[Language::Rust]))
    .stage(ParsingStage::new())
    .stage(CallGraphStage::new())
    .stage(PurityAnalysisStage::new())
    .stage(CustomAnalysisStage::new()) // User-defined!
    .with_progress()
    .build();

let result = pipeline.execute(&env)?;
```

## Dependencies

- **Prerequisites**:
  - Spec 207 (Stillwater Effects Integration)
  - Spec 208 (Pure Function Extraction)
- **Affected Components**:
  - `src/commands/analyze.rs` - Use new pipeline API
  - `src/builders/unified_analysis.rs` - Replaced by pipeline
  - All analysis stages become pipeline stages
- **External Dependencies**: None (builds on Specs 207 & 208)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_builder_type_safety() {
        // This should compile
        let _pipeline = Pipeline::new()
            .stage(FileDiscoveryStage::new(&path, &langs))
            .stage(ParsingStage::new())
            .build();

        // This should NOT compile (type mismatch)
        // let _pipeline = Pipeline::new()
        //     .stage(ParsingStage::new())  // Expects Vec<PathBuf>, gets ()
        //     .build();
    }

    #[test]
    fn test_pipeline_conditional_stages() {
        let with_coverage = Pipeline::new()
            .stage(FileDiscoveryStage::new(&path, &langs))
            .when(true, |p| p.stage(CoverageLoadingStage::new(&lcov_path)))
            .build();

        let without_coverage = Pipeline::new()
            .stage(FileDiscoveryStage::new(&path, &langs))
            .when(false, |p| p.stage(CoverageLoadingStage::new(&lcov_path)))
            .build();

        // Both should compile and execute
    }

    #[test]
    fn test_pipeline_execution() {
        let env = test_env();
        let pipeline = Pipeline::complexity_only(&test_config())
            .build();

        let result = pipeline.execute(&env).unwrap();

        assert!(result.metrics.len() > 0);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_standard_pipeline_equals_old_implementation() {
    let config = test_config();
    let env = test_env();

    // New pipeline
    let new_result = Pipeline::standard(&config)
        .build()
        .execute(&env)
        .unwrap();

    // Old implementation
    let old_result = perform_unified_analysis_computation(&config).unwrap();

    // Results should be identical
    assert_eq!(new_result.items.len(), old_result.items.len());
    assert_eq!(new_result.total_impact, old_result.total_impact);
}

#[test]
fn test_fast_pipeline_skips_stages() {
    let config = test_config_with_coverage();
    let env = test_env();

    let (result, timings) = Pipeline::fast(&config)
        .build()
        .execute_with_timing(&env)
        .unwrap();

    // Should not have coverage data
    assert!(result.coverage.is_none());

    // Should have fewer stages
    assert!(timings.len() < 9);
}
```

### Performance Tests

```rust
#[bench]
fn bench_pipeline_overhead(b: &mut Bencher) {
    let config = test_config();
    let env = test_env();

    b.iter(|| {
        Pipeline::complexity_only(&config)
            .build()
            .execute(&env)
    });
}

#[bench]
fn bench_parallel_pipeline(b: &mut Bencher) {
    let config = test_config();
    let env = test_env();

    b.iter(|| {
        Pipeline::standard(&config)
            .parallel(8)
            .build()
            .execute(&env)
    });
}
```

## Documentation Requirements

### User Documentation

Update `README.md`:
```markdown
## Custom Analysis Pipelines

Debtmap provides a flexible pipeline system for customizing analysis:

### Standard Pipeline
```rust
// Full analysis with all features
debtmap analyze . --lcov coverage.info --context
```

### Fast Pipeline
```rust
// Skip coverage and context for faster analysis
debtmap analyze . --fast
```

### Custom Pipeline (API)
```rust
use debtmap::pipeline::*;

let pipeline = Pipeline::new()
    .stage(FileDiscoveryStage::new(&path, &[Language::Rust]))
    .stage(ParsingStage::new())
    .stage(CallGraphStage::new())
    .stage(CustomStage::new())  // Your custom analysis
    .with_progress()
    .build();

let result = pipeline.execute(&env)?;
```
```

### Architecture Documentation

Update `ARCHITECTURE.md`:
```markdown
## Pipeline Architecture

Debtmap uses a composable pipeline architecture that separates:

1. **Stage Definition**: Pure functions and effects as reusable stages
2. **Pipeline Composition**: Fluent API for building analysis workflows
3. **Execution**: Type-safe execution with progress tracking

### Pipeline Stages

Each stage transforms data:
- **Input Type**: What data the stage expects
- **Output Type**: What data the stage produces
- **Pure or Effect**: Whether stage performs I/O

### Standard Pipelines

- `standard_pipeline`: Full analysis (9 stages)
- `fast_pipeline`: Quick analysis (6 stages)
- `complexity_only`: Just complexity metrics (3 stages)
- `call_graph_only`: Call graph + purity (4 stages)

### Custom Pipelines

Users can compose custom pipelines:

```rust
Pipeline::new()
    .stage(stage1)
    .when(condition, |p| p.stage(optional_stage))
    .stage(stage2)
    .parallel(8)
    .with_progress()
    .build()
    .execute(&env)
```
```

## Implementation Notes

### Best Practices

1. **Type-Safe Composition**
   - Let compiler validate pipeline construction
   - Use type inference for fluent API
   - Avoid `Box<dyn Any>` except internally

2. **Progress Reporting**
   - Automatic from stage names
   - No manual progress tracking in stages
   - Pipeline handles progress lifecycle

3. **Error Context**
   - Each stage adds context on error
   - Pipeline reports which stage failed
   - Full error chain preserved

### Common Pitfalls

1. **Type Mismatches**
   ```rust
   // Bad: Types don't match
   Pipeline::new()
       .stage(returns_files)     // Output: Vec<PathBuf>
       .stage(expects_metrics)   // Input: Vec<FunctionMetrics>
       // Won't compile!

   // Good: Add parsing stage
   Pipeline::new()
       .stage(returns_files)
       .stage(parsing_stage)     // Vec<PathBuf> → Vec<FunctionMetrics>
       .stage(expects_metrics)
       // Compiles!
   ```

2. **Premature Execution**
   ```rust
   // Bad: Can't reuse
   let result = Pipeline::new()
       .stage(stage1)
       .execute(&env)?;  // Consumed!

   // Good: Build then execute multiple times
   let pipeline = Pipeline::new()
       .stage(stage1)
       .build();

   let result1 = pipeline.execute(&env1)?;
   let result2 = pipeline.execute(&env2)?;
   ```

## Migration and Compatibility

### Migration Strategy

1. **Phase 1**: Implement pipeline system
2. **Phase 2**: Create adapters for old API
3. **Phase 3**: Migrate `handle_analyze` to use pipeline
4. **Phase 4**: Deprecate old functions
5. **Phase 5**: Remove old implementation

### Backward Compatibility

Old API delegates to pipeline:
```rust
// Old function (kept for compatibility)
pub fn perform_unified_analysis(...) -> Result<UnifiedAnalysis> {
    let pipeline = Pipeline::standard(&config).build();
    let env = AnalysisEnv::from_config(&config);
    pipeline.execute(&env)
}
```

## Success Metrics

- [ ] Pipeline builder type-safe (incompatible stages cause compile errors)
- [ ] Standard pipeline produces identical results to old implementation
- [ ] Fast pipeline runs in < 50% time of standard pipeline
- [ ] Custom pipeline example works and is documented
- [ ] Performance overhead < 5% vs direct implementation
- [ ] All existing tests pass
- [ ] New pipeline tests achieve > 90% coverage

## References

- [Spec 207: Stillwater Effects Integration](./207-stillwater-effects-integration.md)
- [Spec 208: Pure Function Extraction](./208-pure-function-extraction.md)
- [Stillwater Philosophy](../stillwater/PHILOSOPHY.md)
- [Railway Oriented Programming](https://fsharpforfunandprofit.com/rop/)
