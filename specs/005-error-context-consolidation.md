---
number: 5
title: Error Context Consolidation
category: foundation
priority: medium
status: draft
dependencies: [1]
created: 2025-12-20
---

# Specification 005: Error Context Consolidation

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 001 (Stillwater 0.15 Upgrade)

## Context

Debtmap currently has three separate error hierarchies:

1. **Domain Errors** (`src/error.rs`): `CliError`, `ConfigError`, `AnalysisError`, `AppError`
2. **Unified Errors** (`src/errors/mod.rs`): `AnalysisError` enum with IoError, ParseError, ValidationError variants
3. **Core Errors** (`src/core/errors.rs`): Basic `Error` enum with filesystem focus

This fragmentation causes:
- Confusion about which error type to use
- Inconsistent error context propagation
- Difficulty tracing errors through the analysis pipeline
- Verbose error handling code with multiple conversions

Stillwater 0.15's `.context()` chaining provides a clean solution for building error context trails through effect chains, and would benefit from a consolidated error type.

## Objective

Consolidate the three error hierarchies into a single, unified error system that uses stillwater's context chaining for rich, traceable error messages throughout the analysis pipeline.

## Requirements

### Functional Requirements

1. **Unified Error Type**
   - Single `DebtmapError` enum covering all error cases
   - Clear categorization via variants (Io, Parse, Config, Analysis, Cli)
   - Source error preservation for debugging
   - Structured error codes for programmatic handling

2. **Context Chaining Integration**
   - Use stillwater's `.context()` throughout effect chains
   - Build context trails showing error provenance
   - Display context in user-facing error messages
   - Preserve context in logs

3. **Error Classification**
   - Distinguish retryable vs non-retryable errors
   - Classify user-fixable vs internal errors
   - Provide actionable error messages
   - Include error codes for documentation lookup

4. **Migration Helpers**
   - From<OldError> implementations for gradual migration
   - Deprecation warnings on old error types
   - Clear migration path documentation

### Non-Functional Requirements

- Zero performance overhead for error context
- Compatible with `anyhow` for external API boundaries
- Thread-safe error types
- Serializable for structured logging

## Acceptance Criteria

- [ ] `DebtmapError` enum consolidates all error cases
- [ ] Error codes assigned to each variant (e.g., `E001`, `E002`)
- [ ] `.context()` used in all effect chains in analysis pipeline
- [ ] Error display shows full context trail
- [ ] `is_retryable()` method for retry decisions
- [ ] `is_user_fixable()` method for error classification
- [ ] Old error types deprecated with migration warnings
- [ ] All existing tests pass with new error types
- [ ] Error documentation generated from error codes
- [ ] Structured logging integration works

## Technical Details

### Implementation Approach

```rust
// Unified error type
#[derive(Debug, Clone, thiserror::Error)]
pub enum DebtmapError {
    // I/O and filesystem errors
    #[error("[E001] I/O error: {message}")]
    Io {
        code: ErrorCode,
        message: String,
        path: Option<PathBuf>,
        #[source]
        source: Option<Arc<std::io::Error>>,
    },

    // Parsing errors
    #[error("[E010] Parse error in {path}: {message}")]
    Parse {
        code: ErrorCode,
        path: PathBuf,
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },

    // Configuration errors
    #[error("[E020] Configuration error: {message}")]
    Config {
        code: ErrorCode,
        message: String,
        field: Option<String>,
    },

    // Analysis errors
    #[error("[E030] Analysis error: {message}")]
    Analysis {
        code: ErrorCode,
        message: String,
        phase: Option<AnalysisPhase>,
    },

    // CLI errors
    #[error("[E040] CLI error: {message}")]
    Cli {
        code: ErrorCode,
        message: String,
        arg: Option<String>,
    },

    // Validation errors (can contain multiple)
    #[error("[E050] Validation failed with {count} errors")]
    Validation {
        code: ErrorCode,
        count: usize,
        errors: Vec<String>,
    },
}

// Error codes for documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode(&'static str);

impl DebtmapError {
    pub fn code(&self) -> ErrorCode { ... }

    pub fn is_retryable(&self) -> bool {
        matches!(self,
            DebtmapError::Io { .. } |
            DebtmapError::Analysis { phase: Some(AnalysisPhase::CoverageLoading), .. }
        )
    }

    pub fn is_user_fixable(&self) -> bool {
        matches!(self,
            DebtmapError::Config { .. } |
            DebtmapError::Cli { .. } |
            DebtmapError::Validation { .. }
        )
    }
}
```

### Context Chain Integration

```rust
// Effect chain with context
pub fn analyze_project(config: &Config) -> AnalysisEffect<ProjectResults> {
    discover_files(config)
        .context("Discovering source files")
        .and_then(|files| parse_files(files))
        .context("Parsing source files")
        .and_then(|asts| analyze_complexity(asts))
        .context("Calculating complexity metrics")
        .and_then(|metrics| detect_debt(metrics))
        .context("Detecting technical debt")
}

// Error output with context trail:
// Error: [E010] Parse error in src/main.rs: unexpected token
//   -> Parsing source files
//   -> Analyzing project at /path/to/project
```

### Context Error Display

```rust
// Custom display for context-wrapped errors
impl std::fmt::Display for ContextError<DebtmapError> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Error: {}", self.inner())?;
        for context in self.context_chain() {
            writeln!(f, "  -> {}", context)?;
        }
        Ok(())
    }
}
```

### Migration From Old Types

```rust
// Migration helpers
impl From<crate::error::AnalysisError> for DebtmapError {
    fn from(e: crate::error::AnalysisError) -> Self {
        // Map old error to new unified type
        DebtmapError::Analysis {
            code: ErrorCode("E030"),
            message: e.to_string(),
            phase: None,
        }
    }
}

// Deprecation on old types
#[deprecated(since = "0.12.0", note = "Use DebtmapError instead")]
pub type AnalysisError = crate::errors::AnalysisError;
```

### Structured Logging Integration

```rust
// Serialize for structured logging
impl serde::Serialize for DebtmapError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("DebtmapError", 4)?;
        state.serialize_field("code", &self.code().as_str())?;
        state.serialize_field("category", &self.category())?;
        state.serialize_field("message", &self.to_string())?;
        state.serialize_field("retryable", &self.is_retryable())?;
        state.end()
    }
}
```

### Affected Files

- `src/error.rs` - Deprecate, add From impls
- `src/errors/mod.rs` - Deprecate, add From impls
- `src/core/errors.rs` - Deprecate, add From impls
- `src/debtmap_error.rs` - New unified error module
- `src/effects/core.rs` - Update AnalysisEffect error type
- All effect chains - Add .context() calls

## Dependencies

- **Prerequisites**: Spec 001 (Stillwater 0.15 Upgrade)
- **Affected Components**: All modules using error types
- **External Dependencies**: thiserror, stillwater context

## Testing Strategy

- **Unit Tests**: Verify error construction and display
- **Integration Tests**: Full pipeline with context trails
- **Migration Tests**: Verify From implementations work
- **Display Tests**: Verify user-facing error format

```rust
#[test]
fn context_chain_displays_correctly() {
    let error = DebtmapError::parse("unexpected token", "src/main.rs");
    let with_context = error
        .wrap_context("Parsing source files")
        .wrap_context("Analyzing project");

    let display = format!("{}", with_context);
    assert!(display.contains("unexpected token"));
    assert!(display.contains("Parsing source files"));
    assert!(display.contains("Analyzing project"));
}

#[test]
fn retryable_errors_identified() {
    let io_error = DebtmapError::io("file not found");
    let config_error = DebtmapError::config("invalid threshold");

    assert!(io_error.is_retryable());
    assert!(!config_error.is_retryable());
}
```

## Documentation Requirements

- **Code Documentation**: Document error codes and meanings
- **User Documentation**: Error code reference page
- **Architecture Updates**: Document error handling strategy

## Implementation Notes

- Migrate one module at a time, starting with effects/core.rs
- Keep From<OldType> impls until all usages migrated
- Add tracing integration for automatic context capture
- Consider generating error code documentation from code
- Ensure error messages are actionable and user-friendly

## Migration and Compatibility

**Breaking Change**: Error types change, but migration is gradual:

1. Phase 1: Add `DebtmapError`, keep old types
2. Phase 2: Add `From` implementations, deprecate old types
3. Phase 3: Update all internal usages to new type
4. Phase 4: Remove deprecated types in next major version

External API boundaries continue to use `anyhow::Error` for compatibility.
