---
number: 003
title: Validation with Field Context and Error Accumulation
category: foundation
priority: medium
status: draft
dependencies: [001]
created: 2025-12-20
---

# Specification 003: Validation with Field Context and Error Accumulation

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 001 (Refined Types for Domain Invariants)

## Context

Debtmap's current validation approach has several limitations:

1. **Limited error accumulation** - Config validation uses `Validation`, but file processing short-circuits on first error
2. **Missing field context** - Errors don't always indicate which field/file caused the problem
3. **Inconsistent patterns** - Different modules use different validation approaches
4. **Poor user experience** - Users must fix errors one at a time, re-running between each fix

Stillwater 1.0 enhances the validation system with:

- `FieldError<E>` type for field-specific error context
- `with_field()` method to attach field names to errors
- `validate_field()` for direct field context attachment
- `ValidationFieldExt` trait for ergonomic field validation
- Improved `Validation::all()` for tuple accumulation up to 12 elements

## Objective

Extend debtmap's validation patterns to provide comprehensive field context, accumulate all errors across multi-field validation, and improve error messages for better user experience.

## Requirements

### Functional Requirements

1. **Configuration validation enhancement**
   - Attach field names to all config validation errors
   - Accumulate all config errors before reporting
   - Provide clear paths for nested configuration (e.g., `thresholds.cyclomatic`)

2. **File processing validation**
   - Accumulate parse errors across multiple files
   - Track file path in each error
   - Support partial success (analyze files that parse, report those that don't)

3. **Analysis validation**
   - Validate analysis results before aggregation
   - Accumulate metric calculation errors
   - Provide function/file context in errors

4. **Error formatting**
   - Structured error output with field paths
   - JSON-serializable error format for tooling integration
   - Human-readable error messages with context

### Non-Functional Requirements

- **Accumulation** - All independent errors collected, not just first
- **Context preservation** - Full path from root to failing field
- **Serialization** - Errors serializable for IDE integration
- **Backward compatibility** - Existing error types still work

## Acceptance Criteria

- [ ] Config validation uses `FieldError<AnalysisError>` for all fields
- [ ] At least 5 validation points use `with_field()` for context
- [ ] File processing accumulates errors from multiple files
- [ ] Error output includes field path (e.g., `config.thresholds.cyclomatic`)
- [ ] Unit tests verify error accumulation (multiple errors collected)
- [ ] Integration test shows multi-file error accumulation
- [ ] Error format is JSON-serializable

## Technical Details

### Implementation Approach

1. **Phase 1: Error Types**
   - Define `ValidationError` with field context
   - Implement `FieldPath` for nested field tracking
   - Add serialization support

2. **Phase 2: Config Validation**
   - Update `validate_config()` to use field context
   - Use `Validation::all()` for tuple accumulation
   - Attach nested paths for sub-configs

3. **Phase 3: File Validation**
   - Create `ValidatedFileSet` for partial success
   - Accumulate parse errors across files
   - Report all errors with file paths

### Architecture Changes

```rust
// src/validation/errors.rs
use stillwater::refined::{FieldError, ValidationFieldExt};
use stillwater::Validation;

/// Nested field path for error context
#[derive(Clone, Debug, Serialize)]
pub struct FieldPath(Vec<String>);

impl FieldPath {
    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub fn push(&self, field: &str) -> Self {
        let mut path = self.0.clone();
        path.push(field.to_string());
        Self(path)
    }

    pub fn to_string(&self) -> String {
        self.0.join(".")
    }
}

/// Validation error with full context
#[derive(Clone, Debug, Serialize)]
pub struct ValidationError {
    pub field: FieldPath,
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl ValidationError {
    pub fn at_field(field: &str, message: impl Into<String>) -> Self {
        Self {
            field: FieldPath::root().push(field),
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    pub fn with_context(mut self, expected: &str, actual: &str) -> Self {
        self.expected = Some(expected.to_string());
        self.actual = Some(actual.to_string());
        self
    }
}
```

### Data Structures

```rust
/// Result of validating multiple files
pub struct ValidatedFileSet {
    /// Successfully parsed files
    pub valid: Vec<ParsedFile>,
    /// Files that failed to parse with errors
    pub errors: Vec<FileError>,
}

impl ValidatedFileSet {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn is_partial_success(&self) -> bool {
        !self.valid.is_empty() && self.has_errors()
    }
}

/// Error for a specific file
#[derive(Clone, Debug, Serialize)]
pub struct FileError {
    pub path: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub message: String,
    pub error_code: Option<String>,
}
```

### APIs and Interfaces

```rust
// Enhanced config validation
fn validate_thresholds(
    config: &ThresholdsConfigRaw,
    path: &FieldPath,
) -> Validation<ThresholdsConfig, Vec<ValidationError>> {
    let cyclomatic = ComplexityThreshold::validate(config.cyclomatic)
        .map_err(|e| vec![ValidationError::at_field(
            &path.push("cyclomatic").to_string(),
            e
        )]);

    let cognitive = CognitiveThreshold::validate(config.cognitive)
        .map_err(|e| vec![ValidationError::at_field(
            &path.push("cognitive").to_string(),
            e
        )]);

    let nesting = NestingThreshold::validate(config.nesting)
        .map_err(|e| vec![ValidationError::at_field(
            &path.push("nesting").to_string(),
            e
        )]);

    Validation::all((cyclomatic, cognitive, nesting))
        .map(|(c, cog, n)| ThresholdsConfig {
            cyclomatic: c,
            cognitive: cog,
            nesting: n,
        })
}

// File processing with accumulation
fn parse_all_files(
    files: Vec<PathBuf>,
) -> Validation<ValidatedFileSet, Vec<FileError>> {
    let results: Vec<_> = files
        .into_iter()
        .map(|path| parse_file(&path).map_err(|e| FileError::from_parse(path, e)))
        .collect();

    let (valid, errors): (Vec<_>, Vec<_>) = results
        .into_iter()
        .partition(|r| r.is_ok());

    if errors.is_empty() {
        Validation::success(ValidatedFileSet {
            valid: valid.into_iter().map(|r| r.unwrap()).collect(),
            errors: vec![],
        })
    } else if valid.is_empty() {
        Validation::failure(errors.into_iter().map(|r| r.unwrap_err()).collect())
    } else {
        // Partial success - report errors but continue
        Validation::success(ValidatedFileSet {
            valid: valid.into_iter().map(|r| r.unwrap()).collect(),
            errors: errors.into_iter().map(|r| r.unwrap_err()).collect(),
        })
    }
}
```

### Usage with ensure combinator

```rust
// Using ensure with field context
fn validate_scoring(config: &ScoringConfigRaw) -> Validation<ScoringConfig, Vec<ValidationError>> {
    Validation::success(config)
        .ensure_fn(
            |c| c.complexity_weight + c.coverage_weight + c.churn_weight == 1.0,
            ValidationError::at_field("scoring.weights", "weights must sum to 1.0")
        )
        .and_then(|c| validate_individual_weights(c))
}
```

## Dependencies

- **Prerequisites**: Spec 001 (Refined Types)
- **Affected Components**:
  - `src/config/validation.rs` - Configuration validation
  - `src/io/effects/file.rs` - File processing
  - `src/error.rs` - Error types
  - `src/effects/validation.rs` - Validation effects
- **External Dependencies**: stillwater 1.0 validation module, serde

## Testing Strategy

- **Unit Tests**:
  - Test field path construction and formatting
  - Test error accumulation with multiple fields
  - Test partial success scenarios

- **Integration Tests**:
  - Test config validation with multiple invalid fields
  - Test file processing with some invalid files
  - Test error serialization

- **User Experience Tests**:
  - Verify error messages are actionable
  - Test JSON output format for tooling

## Documentation Requirements

- **Code Documentation**:
  - Rustdoc for error types with examples
  - Document validation patterns

- **User Documentation**:
  - Document error format for IDE integration
  - Provide troubleshooting guide based on error codes

- **Architecture Updates**:
  - Add validation flow to architecture docs

## Implementation Notes

1. Use `Vec<ValidationError>` for accumulation (implements `Semigroup`)
2. Field paths should be relative to validation root
3. Consider adding error codes for programmatic handling
4. JSON format should be stable for tooling integration
5. Support `--fail-fast` flag to opt-out of accumulation

## Migration and Compatibility

- **Breaking Changes**: Error type changes in internal APIs
- **Migration Path**:
  1. Add new error types alongside existing
  2. Update validators to use field context
  3. Update error formatting
  4. Remove old error handling
- **Compatibility**: CLI output format enhanced but backward-compatible
