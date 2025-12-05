---
number: 207
title: Stillwater Effects Integration for Pure Core Architecture
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-05
---

# Specification 207: Stillwater Effects Integration for Pure Core Architecture

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The current debtmap analysis pipeline violates functional programming principles by mixing I/O operations with business logic throughout the codebase. The `perform_unified_analysis_computation` function (453 lines in `src/builders/unified_analysis.rs`) interleaves:
- File system operations with data transformations
- Progress spinners with pure calculations
- Database/coverage loading with scoring algorithms
- Logging side effects with immutable computations

This makes the code:
- **Untestable** - Requires mocking file systems, progress bars, and I/O
- **Unreliable** - Side effects make functions non-deterministic
- **Unmaintainable** - Large functions with mixed concerns
- **Unreusable** - Business logic tightly coupled to infrastructure

**Stillwater Philosophy**: Separate pure business logic (the "still water" core) from effectful I/O operations (the "flowing water" shell). This creates a clear boundary where:
- **Pure Core**: Testable, deterministic transformations that never perform I/O
- **Imperative Shell**: I/O operations isolated at system boundaries
- **Effect System**: Composable abstractions for managing side effects

## Objective

Integrate the Stillwater effects library to establish a **pure core, imperative shell** architecture for debtmap. This foundation enables:
1. Clear separation between I/O and business logic
2. Composable effect chains for analysis pipeline stages
3. 100% testable pure functions without mocks
4. Type-safe effect composition with compile-time guarantees

**Success Criteria**: All I/O operations wrapped in effects, pure logic functions isolated, and example pipeline stage demonstrating the pattern.

## Requirements

### Functional Requirements

1. **Stillwater Dependency Integration**
   - Add `stillwater` crate to `Cargo.toml` with appropriate version
   - Import effect types: `Effect<T, E, Env>`, `IO`, effect combinators
   - Configure for compatibility with existing async/await code
   - Ensure zero-cost abstraction (no runtime overhead)

2. **Environment Type Definition**
   - Define `AnalysisEnv` struct containing I/O capabilities:
     - File system access
     - Progress reporting channel
     - Configuration settings
     - Optional logging sink
   - Implement required traits for Stillwater integration
   - Document environment lifecycle and initialization

3. **Effect Type Aliases**
   - Create type aliases for common effect patterns:
     - `AnalysisEffect<T> = Effect<T, AnalysisError, AnalysisEnv>`
     - `IOEffect<T> = Effect<T, AnalysisError, AnalysisEnv>`
   - Define error types compatible with `anyhow::Error`
   - Support `?` operator for effect chains

4. **Core I/O Effect Constructors**
   - File discovery: `discover_files(path, langs) -> AnalysisEffect<Vec<PathBuf>>`
   - File reading: `read_file(path) -> AnalysisEffect<String>`
   - File parsing: `parse_file(path, lang) -> AnalysisEffect<FileMetrics>`
   - Coverage loading: `load_coverage(path) -> AnalysisEffect<CoverageData>`
   - Context loading: `load_context(path) -> AnalysisEffect<ProjectContext>`
   - Progress reporting: `report_progress(msg) -> AnalysisEffect<()>`

5. **Effect Composition Helpers**
   - `with_progress`: Add progress reporting to any effect
   - `batch_parallel`: Execute effects in parallel with concurrency control
   - `chain_pure`: Inject pure transformations between I/O operations
   - `recover`: Error recovery and fallback strategies

### Non-Functional Requirements

1. **Performance**
   - Zero-cost abstractions: no runtime overhead vs direct implementation
   - Effect composition must not allocate unnecessarily
   - Support for parallel effect execution (rayon integration)
   - Lazy evaluation where appropriate

2. **Ergonomics**
   - Effects should feel natural to Rust developers
   - Support `?` operator for error propagation
   - Clear error messages for type mismatches
   - Minimal boilerplate for common patterns

3. **Compatibility**
   - Work with existing `anyhow::Result` code
   - Integrate with current error handling
   - Support gradual migration (old and new code coexist)
   - Compatible with async/await where needed

4. **Documentation**
   - Clear examples of effect usage
   - Migration guide from imperative to effect-based code
   - Best practices for effect composition
   - Common patterns and anti-patterns

## Acceptance Criteria

- [ ] Stillwater crate added to `Cargo.toml` and compiles successfully
- [ ] `AnalysisEnv` struct defined with all required I/O capabilities
- [ ] Effect type aliases created in `src/pipeline/effects/types.rs`
- [ ] Core I/O effect constructors implemented in `src/pipeline/effects/io.rs`
- [ ] Effect composition helpers implemented in `src/pipeline/effects/combinators.rs`
- [ ] At least one existing I/O operation refactored to use effects (e.g., file discovery)
- [ ] Unit tests demonstrate pure functions separated from I/O
- [ ] Integration test shows effect composition and execution
- [ ] Documentation added to `ARCHITECTURE.md` explaining effect system
- [ ] Example code in `examples/effect_pipeline.rs` demonstrating usage
- [ ] All existing tests still pass (backward compatibility)

## Technical Details

### Implementation Approach

#### Phase 1: Setup and Configuration

1. **Add Stillwater Dependency**
   ```toml
   [dependencies]
   stillwater = "0.1"  # Adjust version as needed
   ```

2. **Create Module Structure**
   ```
   src/pipeline/
   ├── mod.rs
   ├── effects/
   │   ├── mod.rs
   │   ├── types.rs         # AnalysisEnv, effect type aliases
   │   ├── io.rs            # I/O effect constructors
   │   ├── combinators.rs   # Effect composition helpers
   │   └── progress.rs      # Progress reporting effects
   └── stages/
       └── mod.rs           # Pure transformation stages (next spec)
   ```

3. **Define Core Types**
   ```rust
   // src/pipeline/effects/types.rs
   use stillwater::prelude::*;
   use std::path::PathBuf;
   use crate::progress::ProgressReporter;

   /// Environment providing I/O capabilities for analysis pipeline
   pub struct AnalysisEnv {
       pub project_path: PathBuf,
       pub progress: Option<ProgressReporter>,
       pub config: AnalysisConfig,
   }

   /// Effect type for analysis operations
   pub type AnalysisEffect<T> = Effect<T, AnalysisError, AnalysisEnv>;

   /// Error type for analysis effects
   #[derive(Debug, thiserror::Error)]
   pub enum AnalysisError {
       #[error("I/O error: {0}")]
       Io(#[from] std::io::Error),

       #[error("Parse error: {0}")]
       Parse(String),

       #[error("Analysis error: {0}")]
       Analysis(String),
   }

   impl From<anyhow::Error> for AnalysisError {
       fn from(err: anyhow::Error) -> Self {
           AnalysisError::Analysis(err.to_string())
       }
   }
   ```

#### Phase 2: I/O Effect Constructors

```rust
// src/pipeline/effects/io.rs
use super::types::*;
use stillwater::prelude::*;
use std::path::{Path, PathBuf};

/// Discover project files (I/O effect)
pub fn discover_files(
    path: &Path,
    languages: &[Language],
) -> AnalysisEffect<Vec<PathBuf>> {
    let path = path.to_path_buf();
    let languages = languages.to_vec();

    IO::execute(move |env: &AnalysisEnv| {
        // Report progress if available
        if let Some(ref progress) = env.progress {
            progress.report("Discovering files...");
        }

        // Perform I/O operation
        crate::io::walker::find_project_files_with_config(
            &path,
            languages,
            env.config.clone(),
        )
        .map_err(|e| AnalysisError::Io(e.into()))
    })
}

/// Load coverage data from LCOV file (I/O effect)
pub fn load_coverage(lcov_path: &Path) -> AnalysisEffect<CoverageData> {
    let path = lcov_path.to_path_buf();

    IO::execute(move |env: &AnalysisEnv| {
        if let Some(ref progress) = env.progress {
            progress.report("Loading coverage data...");
        }

        crate::risk::lcov::parse_lcov_file(&path)
            .map_err(Into::into)
    })
}

/// Parse a single file to metrics (I/O effect)
pub fn parse_file(
    file_path: &Path,
    language: Language,
) -> AnalysisEffect<FileMetrics> {
    let path = file_path.to_path_buf();

    IO::execute(move |_env: &AnalysisEnv| {
        // Read file content
        let content = std::fs::read_to_string(&path)?;

        // Parse AST and extract metrics (pure logic would be separate)
        crate::analyzers::parse_and_analyze(&path, &content, language)
            .map_err(Into::into)
    })
}
```

#### Phase 3: Effect Composition Helpers

```rust
// src/pipeline/effects/combinators.rs
use super::types::*;
use stillwater::prelude::*;

/// Add progress reporting to any effect
pub fn with_progress<T>(
    effect: AnalysisEffect<T>,
    message: &str,
) -> AnalysisEffect<T> {
    let msg = message.to_string();

    IO::execute(move |env: &AnalysisEnv| {
        if let Some(ref progress) = env.progress {
            progress.report(&msg);
        }
        Ok(())
    })
    .and_then(|_| effect)
}

/// Execute effects in parallel with concurrency control
pub fn batch_parallel<T: Send + 'static>(
    effects: Vec<AnalysisEffect<T>>,
    batch_size: usize,
) -> AnalysisEffect<Vec<T>> {
    IO::execute(move |env: &AnalysisEnv| {
        use rayon::prelude::*;

        effects
            .into_par_iter()
            .with_max_len(batch_size)
            .map(|eff| eff.run(env))
            .collect::<Result<Vec<_>, _>>()
    })
}

/// Chain a pure transformation between I/O operations
pub fn chain_pure<T, U, F>(
    effect: AnalysisEffect<T>,
    f: F,
) -> AnalysisEffect<U>
where
    F: FnOnce(T) -> U + Send + 'static,
    T: Send + 'static,
    U: Send + 'static,
{
    effect.map(f)
}
```

#### Phase 4: Example Usage

```rust
// examples/effect_pipeline.rs
use debtmap::pipeline::effects::*;
use stillwater::prelude::*;

fn main() -> anyhow::Result<()> {
    // Create environment
    let env = AnalysisEnv {
        project_path: PathBuf::from("."),
        progress: Some(ProgressReporter::new()),
        config: AnalysisConfig::default(),
    };

    // Compose effects
    let pipeline = discover_files(&env.project_path, &[Language::Rust])
        .and_then(|files| {
            // Pure transformation injected between I/O
            let limited = files.into_iter().take(10).collect();
            Effect::pure(limited)
        })
        .and_then(|files| {
            // Parse files in parallel
            let parse_effects: Vec<_> = files
                .iter()
                .map(|f| parse_file(f, Language::Rust))
                .collect();

            batch_parallel(parse_effects, 4)
        })
        .with_progress("Analyzing files...");

    // Execute effect
    let metrics = pipeline.run(&env)?;

    println!("Analyzed {} files", metrics.len());

    Ok(())
}
```

### Architecture Changes

1. **Module Hierarchy**
   - New `pipeline` module at `src/pipeline/`
   - Effects sub-module for I/O abstractions
   - Stages sub-module for pure transformations (next spec)

2. **Separation of Concerns**
   - **Before**: I/O mixed with logic in 453-line function
   - **After**: I/O isolated in effect constructors, logic in pure functions

3. **Data Flow**
   ```
   Old:
   fn analyze() -> Result<Analysis> {
       let files = read_files()?;        // I/O
       let metrics = parse(files)?;      // I/O
       let graph = build_graph(metrics); // Pure
       let scored = score(graph);        // Pure
       save_results(scored)?;            // I/O
   }

   New:
   fn analyze() -> AnalysisEffect<Analysis> {
       discover_files(path, langs)       // Effect
           .and_then(parse_files)        // Effect
           .map(build_graph)             // Pure (injected)
           .map(score)                   // Pure (injected)
           .and_then(save_results)       // Effect
   }
   ```

### APIs and Interfaces

#### Public API

```rust
pub mod pipeline {
    pub mod effects {
        // Core types
        pub use types::{AnalysisEnv, AnalysisEffect, AnalysisError};

        // I/O effects
        pub use io::{discover_files, load_coverage, parse_file};

        // Combinators
        pub use combinators::{with_progress, batch_parallel, chain_pure};
    }
}
```

#### Integration with Existing Code

```rust
// Migration helper for gradual adoption
pub fn effect_to_result<T>(
    effect: AnalysisEffect<T>,
    env: &AnalysisEnv,
) -> Result<T, AnalysisError> {
    effect.run(env)
}

// Existing code can call effects like this:
let env = create_default_env();
let files = effect_to_result(discover_files(&path, &langs), &env)?;
```

## Dependencies

- **Prerequisites**: None (foundational change)
- **Affected Components**:
  - `src/builders/unified_analysis.rs` - Will be refactored in Spec 208
  - `src/commands/analyze.rs` - Will integrate effects in Spec 209
  - All I/O operations throughout codebase
- **External Dependencies**:
  - Stillwater crate (to be added)
  - Compatibility with existing `anyhow`, `thiserror`

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_composition() {
        let effect = Effect::pure(42)
            .map(|x| x * 2)
            .map(|x| x + 1);

        let env = test_env();
        let result = effect.run(&env).unwrap();

        assert_eq!(result, 85);
    }

    #[test]
    fn test_io_effect_execution() {
        let temp_dir = tempfile::tempdir().unwrap();
        let env = AnalysisEnv {
            project_path: temp_dir.path().to_path_buf(),
            progress: None,
            config: AnalysisConfig::default(),
        };

        let effect = discover_files(temp_dir.path(), &[Language::Rust]);
        let files = effect.run(&env).unwrap();

        assert!(files.is_empty()); // No Rust files in temp dir
    }

    #[test]
    fn test_pure_function_in_effect() {
        // Pure function - easily testable
        fn double(x: i32) -> i32 { x * 2 }

        let effect = Effect::pure(21).map(double);
        let result = effect.run(&test_env()).unwrap();

        assert_eq!(result, 42);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_full_effect_pipeline() {
    let env = test_env_with_fixtures();

    let pipeline = discover_files(&env.project_path, &[Language::Rust])
        .and_then(|files| {
            let parse_effects: Vec<_> = files
                .iter()
                .take(5)
                .map(|f| parse_file(f, Language::Rust))
                .collect();
            batch_parallel(parse_effects, 2)
        })
        .map(|metrics| {
            // Pure transformation
            metrics.len()
        });

    let count = pipeline.run(&env).unwrap();
    assert_eq!(count, 5);
}
```

### Performance Tests

```rust
#[bench]
fn bench_effect_overhead(b: &mut Bencher) {
    let env = test_env();

    b.iter(|| {
        let effect = Effect::pure(42).map(|x| x * 2);
        effect.run(&env)
    });
}

#[bench]
fn bench_parallel_effects(b: &mut Bencher) {
    let env = test_env();
    let effects: Vec<_> = (0..100)
        .map(|i| Effect::pure(i))
        .collect();

    b.iter(|| {
        batch_parallel(effects.clone(), 10).run(&env)
    });
}
```

## Documentation Requirements

### Code Documentation

1. **Module-level documentation** for `src/pipeline/effects/`
   - Explanation of effect system philosophy
   - Examples of effect composition
   - Common patterns and idioms

2. **Function-level documentation** for all public APIs
   - Purpose and usage
   - Example code
   - Error conditions

3. **Type documentation** for `AnalysisEnv` and effect types
   - What capabilities they provide
   - How to construct them
   - Lifetime and ownership considerations

### User Documentation

Update `ARCHITECTURE.md`:
```markdown
## Effect System

Debtmap uses the Stillwater effects library to separate pure business
logic from I/O operations. This enables:

- **Testability**: Pure functions need no mocks
- **Composability**: Build complex pipelines from simple effects
- **Reliability**: Type-safe effect composition prevents errors

### Core Concepts

- **Pure Core**: Business logic functions that never perform I/O
- **Imperative Shell**: I/O operations wrapped in effects
- **Effect Composition**: Chain effects with `.and_then()`, `.map()`

### Example

```rust
let pipeline = discover_files(&path, &langs)
    .and_then(parse_files)
    .map(build_call_graph)
    .map(score_complexity);

let result = pipeline.run(&env)?;
```
```

## Implementation Notes

### Best Practices

1. **Effect Naming Conventions**
   - Use verb phrases: `discover_files`, `load_coverage`
   - Suffix with `_effect` if ambiguous: `parse_effect`
   - Pure functions use nouns: `build_graph`, `score_items`

2. **Error Handling**
   - Use `?` operator in effect chains
   - Convert errors at boundaries with `Into`
   - Provide context with `.context()` from anyhow

3. **Performance**
   - Avoid unnecessary `.clone()` in effect closures
   - Use `move` closures to take ownership
   - Prefer `batch_parallel` for I/O-bound operations

4. **Testing**
   - Test pure functions separately (no env needed)
   - Test effects with mock environment
   - Integration tests use real filesystem in temp directory

### Common Pitfalls

1. **Leaking I/O into Pure Functions**
   ```rust
   // Bad: I/O in pure function
   fn build_graph(files: &[PathBuf]) -> CallGraph {
       let content = std::fs::read_to_string(&files[0]).unwrap(); // NO!
       parse(content)
   }

   // Good: I/O in effect, pure transformation separate
   fn parse_files_effect(files: &[PathBuf]) -> AnalysisEffect<Vec<String>> {
       // I/O wrapped in effect
   }

   fn build_graph_pure(contents: &[String]) -> CallGraph {
       // Pure transformation
   }
   ```

2. **Premature Execution**
   ```rust
   // Bad: Executing effect too early
   let files = discover_files(&path, &langs).run(&env)?;
   let metrics = parse_files(&files).run(&env)?;

   // Good: Compose first, execute once
   let pipeline = discover_files(&path, &langs)
       .and_then(parse_files);
   let metrics = pipeline.run(&env)?;
   ```

3. **Blocking the Main Thread**
   ```rust
   // Bad: Sequential I/O
   for file in files {
       let content = read_file(&file).run(&env)?;
       // Process...
   }

   // Good: Parallel I/O
   let effects: Vec<_> = files.iter().map(read_file).collect();
   let contents = batch_parallel(effects, 10).run(&env)?;
   ```

## Migration and Compatibility

### Gradual Migration Strategy

1. **Phase 1**: Add effects module, no breaking changes
2. **Phase 2**: Refactor one pipeline stage to use effects
3. **Phase 3**: Add more stages incrementally
4. **Phase 4**: Deprecate old imperative code

### Backward Compatibility

- Old code continues to work unchanged
- New effects can be called from old code via `effect_to_result`
- No breaking changes to public API

### Migration Helper

```rust
/// Convert effect to Result for compatibility with existing code
pub fn effect_to_result<T>(
    effect: AnalysisEffect<T>,
) -> anyhow::Result<T> {
    let env = AnalysisEnv::default();
    effect.run(&env).map_err(Into::into)
}
```

## Success Metrics

- [ ] Zero performance regression (benchmarks validate)
- [ ] 100% of unit tests pass
- [ ] Example pipeline demonstrates effect composition
- [ ] Documentation clear and complete
- [ ] Team members can write new effects after reading docs
- [ ] At least one real I/O operation migrated to effects

## References

- [Stillwater Philosophy](../stillwater/PHILOSOPHY.md)
- [Functional Core, Imperative Shell](https://www.destroyallsoftware.com/screencasts/catalog/functional-core-imperative-shell)
- [Railway Oriented Programming](https://fsharpforfunandprofit.com/rop/)
