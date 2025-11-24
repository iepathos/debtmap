---
number: 195
title: Stillwater Foundation Setup
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-11-24
---

# Specification 195: Stillwater Foundation Setup

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently uses `anyhow::Result` pervasively (565+ uses across 100 files) with fail-fast error handling. The codebase mixes I/O operations with pure logic throughout, making testing difficult and error reporting suboptimal. When analyzing multiple files, debtmap stops at the first error, forcing users to fix issues one-by-one.

Stillwater is a Rust library for pragmatic effect composition and validation that emphasizes the "pure core, imperative shell" pattern. It provides:
- **Validation with error accumulation** - Collect ALL errors, not just the first
- **Effect composition** - Separate pure logic from I/O at type level
- **Reader pattern** - Clean dependency injection without parameter threading
- **Testing utilities** - MockEnv, assertion macros, TestEffect wrapper

This specification establishes the foundation for integrating stillwater into debtmap, setting up the core types and environment infrastructure that subsequent specs will build upon.

## Objective

Add stillwater dependency and create foundational types (environment trait, Effect type aliases, error types) that enable functional programming patterns while maintaining backwards compatibility with existing code.

## Requirements

### Functional Requirements
- Add stillwater crate dependency to Cargo.toml
- Create environment trait defining debtmap's I/O capabilities
- Create production implementation of environment trait
- Define Effect type aliases for common patterns
- Create error type compatible with both anyhow and stillwater
- Maintain backwards compatibility - all existing code continues to work

### Non-Functional Requirements
- Zero performance regression from stillwater abstractions
- Binary size increase < 100KB
- All existing tests pass without modification
- No breaking changes to public API
- Compilation time increase < 5%

## Acceptance Criteria

- [ ] `stillwater = "0.8"` added to Cargo.toml dependencies
- [ ] `src/env.rs` created with `AnalysisEnv` trait definition
- [ ] `RealEnv` struct implements `AnalysisEnv` for production use
- [ ] `src/effects.rs` created with type aliases (`AnalysisEffect<T>`, `AnalysisValidation<T>`)
- [ ] `AnalysisError` type created that can convert from/to both anyhow and stillwater errors
- [ ] All existing tests pass (no behavior changes)
- [ ] Documentation added explaining environment pattern
- [ ] Example showing how to use Effect and Validation types
- [ ] Backwards-compatible wrapper functions provided
- [ ] No breaking changes to public API

## Technical Details

### Implementation Approach

1. **Add Stillwater Dependency**
   ```toml
   # Cargo.toml
   [dependencies]
   stillwater = { version = "0.8", features = ["async"] }
   ```

2. **Create Environment Trait** (`src/env.rs`)
   ```rust
   /// Environment trait defining all I/O capabilities for analysis
   pub trait AnalysisEnv: Clone + Send + Sync {
       /// Access to file system operations
       fn file_system(&self) -> &dyn FileSystem;

       /// Access to coverage data loading
       fn coverage_loader(&self) -> &dyn CoverageLoader;

       /// Access to cache operations
       fn cache(&self) -> &dyn Cache;

       /// Access to configuration
       fn config(&self) -> &Config;
   }

   /// Production environment implementation
   #[derive(Clone)]
   pub struct RealEnv {
       file_system: Arc<dyn FileSystem>,
       coverage_loader: Arc<dyn CoverageLoader>,
       cache: Arc<dyn Cache>,
       config: Config,
   }

   impl RealEnv {
       pub fn new(config: Config) -> Self {
           Self {
               file_system: Arc::new(RealFileSystem::default()),
               coverage_loader: Arc::new(RealCoverageLoader::default()),
               cache: Arc::new(RealCache::default()),
               config,
           }
       }
   }

   impl AnalysisEnv for RealEnv {
       fn file_system(&self) -> &dyn FileSystem {
           &*self.file_system
       }

       fn coverage_loader(&self) -> &dyn CoverageLoader {
           &*self.coverage_loader
       }

       fn cache(&self) -> &dyn Cache {
           &*self.cache
       }

       fn config(&self) -> &Config {
           &self.config
       }
   }
   ```

3. **Create I/O Traits** (`src/io/traits.rs`)
   ```rust
   /// File system operations
   pub trait FileSystem: Send + Sync {
       fn read_to_string(&self, path: &Path) -> Result<String>;
       fn write(&self, path: &Path, content: &str) -> Result<()>;
       fn exists(&self, path: &Path) -> bool;
       fn is_file(&self, path: &Path) -> bool;
   }

   /// Coverage data loading
   pub trait CoverageLoader: Send + Sync {
       fn load_lcov(&self, path: &Path) -> Result<Coverage>;
       fn load_cobertura(&self, path: &Path) -> Result<Coverage>;
   }

   /// Cache operations
   pub trait Cache: Send + Sync {
       fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
       fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()>;
       fn invalidate(&self, key: &str) -> Result<()>;
   }
   ```

4. **Create Effect Type Aliases** (`src/effects.rs`)
   ```rust
   use stillwater::{Effect, Validation};

   /// Effect type for debtmap analysis operations
   pub type AnalysisEffect<T> = Effect<T, AnalysisError, Box<dyn AnalysisEnv>>;

   /// Validation type for debtmap validations
   pub type AnalysisValidation<T> = Validation<T, Vec<AnalysisError>>;

   /// Helper to create effects from sync operations
   pub fn effect_from_sync<T, F>(f: F) -> AnalysisEffect<T>
   where
       F: FnOnce() -> Result<T, AnalysisError> + Send + 'static,
       T: Send + 'static,
   {
       Effect::new(move |_env| f())
   }

   /// Helper to create effects from async operations
   #[cfg(feature = "async")]
   pub fn effect_from_async<T, F, Fut>(f: F) -> AnalysisEffect<T>
   where
       F: FnOnce() -> Fut + Send + 'static,
       Fut: Future<Output = Result<T, AnalysisError>> + Send + 'static,
       T: Send + 'static,
   {
       Effect::new_async(move |_env| f())
   }
   ```

5. **Create Error Types** (`src/errors.rs`)
   ```rust
   use std::fmt;
   use anyhow;
   use stillwater::ContextError;

   /// Unified error type for debtmap
   #[derive(Debug, Clone)]
   pub enum AnalysisError {
       IoError(String),
       ParseError(String),
       ValidationError(String),
       ConfigError(String),
       CoverageError(String),
       Other(String),
   }

   impl fmt::Display for AnalysisError {
       fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
           match self {
               Self::IoError(msg) => write!(f, "I/O error: {}", msg),
               Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
               Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
               Self::ConfigError(msg) => write!(f, "Config error: {}", msg),
               Self::CoverageError(msg) => write!(f, "Coverage error: {}", msg),
               Self::Other(msg) => write!(f, "{}", msg),
           }
       }
   }

   impl std::error::Error for AnalysisError {}

   // Convert from anyhow for backwards compatibility
   impl From<anyhow::Error> for AnalysisError {
       fn from(err: anyhow::Error) -> Self {
           Self::Other(err.to_string())
       }
   }

   // Convert to anyhow for backwards compatibility
   impl From<AnalysisError> for anyhow::Error {
       fn from(err: AnalysisError) -> Self {
           anyhow::anyhow!("{}", err)
       }
   }

   // Support stillwater's context chaining
   impl From<ContextError<AnalysisError>> for AnalysisError {
       fn from(err: ContextError<AnalysisError>) -> Self {
           Self::Other(err.to_string())
       }
   }
   ```

6. **Create Backwards-Compatible Wrappers**
   ```rust
   /// Wrapper to run effects and convert to anyhow::Result
   pub fn run_effect<T>(
       effect: AnalysisEffect<T>,
       config: Config,
   ) -> anyhow::Result<T> {
       let env = RealEnv::new(config);
       effect.run(&env).map_err(Into::into)
   }

   /// Wrapper to run validations and convert to anyhow::Result
   pub fn run_validation<T>(
       validation: AnalysisValidation<T>,
   ) -> anyhow::Result<T> {
       validation.into_result().map_err(|errors| {
           anyhow::anyhow!(
               "Validation failed:\n{}",
               errors.iter()
                   .enumerate()
                   .map(|(i, e)| format!("  {}. {}", i + 1, e))
                   .collect::<Vec<_>>()
                   .join("\n")
           )
       })
   }
   ```

### Architecture Changes

**New Module Structure:**
```
src/
├── env.rs           # NEW: Environment trait and implementations
├── effects.rs       # NEW: Effect type aliases and helpers
├── errors.rs        # NEW: Unified error types
├── io/
│   ├── traits.rs    # NEW: I/O trait definitions
│   ├── real.rs      # NEW: Production implementations
│   └── mod.rs       # Updated to export new traits
└── lib.rs           # Updated to export new modules
```

**No Changes to Existing Modules:**
- All existing code continues to use `anyhow::Result`
- No breaking changes to public API
- Wrappers allow gradual migration

### Dependencies

**New Dependencies:**
- `stillwater = "0.8"` with `async` feature

**No Removed Dependencies:**
- Keep `anyhow` for backwards compatibility
- Both can coexist during migration

## Dependencies

- **Prerequisites**: None (foundation spec)
- **Blocked by**: None
- **Blocks**:
  - Spec 196 (Pure Function Extraction)
  - Spec 197 (Validation Accumulation)
  - Spec 198 (Effect Composition)
  - Spec 199 (Reader Pattern)
  - Spec 200 (Testing Infrastructure)
- **Affected Components**:
  - Build system (Cargo.toml)
  - Module structure (new modules)
  - Error handling (new types)
- **External Dependencies**:
  - stillwater crate from crates.io

## Testing Strategy

- **Unit Tests**:
  - Test `RealEnv` creation and trait implementation
  - Test error conversions (anyhow ↔ AnalysisError)
  - Test Effect type aliases compile
  - Test backwards-compatible wrappers

- **Integration Tests**:
  - All existing tests pass without modification
  - No behavior changes
  - No performance regressions

- **Compilation Tests**:
  - Verify all existing code still compiles
  - Check for no new warnings
  - Validate trait implementations

- **Example Tests**:
  - Create example showing Effect usage
  - Create example showing Validation usage
  - Create example showing environment pattern

## Documentation Requirements

- **Code Documentation**:
  - Add module docs to `env.rs` explaining environment pattern
  - Add module docs to `effects.rs` explaining Effect and Validation
  - Add inline docs for all public types and functions
  - Add examples to type documentation

- **User Documentation**:
  - Not needed (internal refactoring)

- **Architecture Updates**:
  - Add section to ARCHITECTURE.md about environment pattern
  - Document effect composition principles
  - Explain migration strategy

## Implementation Notes

### Design Decisions

1. **Environment as Trait**
   - Allows for easy mocking in tests
   - Enables dependency injection
   - Supports multiple implementations (real, mock, test)

2. **Type Aliases for Effects**
   - Reduces boilerplate in function signatures
   - Centralizes environment type
   - Easier to refactor if needed

3. **Backwards-Compatible Errors**
   - Allows gradual migration from anyhow
   - No breaking changes
   - Can coexist during transition

4. **Wrapper Functions**
   - Enable using Effects in existing anyhow code
   - Zero-cost abstraction (inlined)
   - Smooth migration path

### Potential Issues

1. **Trait Object Overhead**
   - **Risk**: Using `dyn Trait` may impact performance
   - **Mitigation**: Profile before/after, use `Arc` for cheap clones
   - **Alternative**: Can use generics if performance critical

2. **Error Conversion Loss**
   - **Risk**: Converting between error types may lose context
   - **Mitigation**: Preserve error messages, use Display impl
   - **Future**: Enhance AnalysisError with more structured data

3. **Learning Curve**
   - **Risk**: Team unfamiliar with Effect/Validation patterns
   - **Mitigation**: Provide examples, documentation, and training
   - **Approach**: Gradual adoption, coexist with anyhow initially

### Migration Guidelines

This spec is **non-breaking**:
- All existing code continues to work
- New patterns introduced alongside old ones
- Gradual adoption over multiple specs
- Can be incrementally tested and validated

### Example Usage

```rust
use debtmap::{AnalysisEffect, RealEnv, run_effect};

// New Effect-based function
fn analyze_file_effect(path: PathBuf) -> AnalysisEffect<FileMetrics> {
    Effect::new(|env| {
        let content = env.file_system().read_to_string(&path)?;
        // ... analysis logic ...
        Ok(metrics)
    })
}

// Backwards-compatible usage
fn analyze_file(path: &Path, config: Config) -> anyhow::Result<FileMetrics> {
    run_effect(analyze_file_effect(path.to_path_buf()), config)
}
```

## Migration and Compatibility

### Breaking Changes

None. This is a foundation spec that adds new capabilities without removing existing functionality.

### Compatibility Strategy

- **Phase 1** (this spec): Add foundation, maintain full backwards compatibility
- **Phase 2-5**: Gradually migrate modules to use Effects/Validation
- **Phase 6**: Eventually remove anyhow dependency (future, not this spec)

### Rollback Plan

If issues arise:
1. Remove `stillwater` dependency
2. Delete new modules (`env.rs`, `effects.rs`, `errors.rs`)
3. Revert Cargo.toml changes
4. No impact on existing functionality

## Success Metrics

- **Compilation**:
  - All code compiles with no new warnings
  - No breaking changes to public API
  - Compilation time increase < 5%

- **Performance**:
  - All tests pass at same speed
  - Binary size increase < 100KB
  - No runtime overhead measurable

- **Completeness**:
  - Environment trait fully implemented
  - Effect type aliases available
  - Error conversions bidirectional
  - Examples and documentation complete

## Future Considerations

This foundation enables:
- **Spec 196**: Pure function extraction
- **Spec 197**: Validation accumulation
- **Spec 198**: Effect composition for I/O
- **Spec 199**: Reader pattern for config
- **Spec 200**: Testing with MockEnv

Once foundation is stable, subsequent specs can build functional patterns incrementally.
