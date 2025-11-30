---
number: 184
title: Add Validation Type for Config Validation
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 184: Add Validation Type for Config Validation

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Currently, configuration validation in debtmap uses a fail-fast approach with `Result` types. When validation fails, the first error is returned and processing stops. This creates a poor user experience when multiple configuration problems exist:

```rust
// Current fail-fast approach
fn validate_config(cfg: &Config) -> Result<ValidConfig> {
    validate_max_complexity(cfg.max_complexity)?;  // Fails here
    validate_output_format(cfg.format)?;           // Never reached if above fails
    validate_file_patterns(cfg.patterns)?;         // Never reached
    Ok(ValidConfig::from(cfg))
}
```

**User Experience Problem:**
1. User runs debtmap with invalid config
2. Gets error: "Invalid max_complexity: must be > 0"
3. Fixes that error, runs again
4. Gets error: "Invalid output_format: unknown format 'jsoon'"
5. Fixes that error, runs again
6. Gets error: "Invalid file pattern: regex error..."
7. Finally gets working config after 3+ iterations

The Stillwater library provides a `Validation` type that accumulates all errors instead of failing fast. This allows reporting all configuration problems in a single pass, significantly improving user experience.

According to STILLWATER_EVALUATION.md (lines 671-682), we should use Stillwater's `Validation` type to collect all validation errors at once, giving users complete feedback in one go.

## Objective

Implement comprehensive config validation using Stillwater's `Validation` type to accumulate and report all configuration errors simultaneously:

```rust
fn validate_config(cfg: &Config) -> Validation<ValidConfig, Vec<ConfigError>> {
    Validation::all((
        validate_max_complexity(cfg.max_complexity),
        validate_output_format(cfg.format),
        validate_file_patterns(cfg.patterns),
        validate_paths(cfg.paths),
        validate_thresholds(cfg),
    ))
}
```

This enables users to:
- See all configuration problems at once
- Fix multiple issues in one iteration
- Understand complete requirements upfront
- Have better CLI experience

## Requirements

### Functional Requirements

1. **Validation Type Infrastructure**
   - Import Stillwater's `Validation` type
   - Create `ConfigError` enum for all error types
   - Define `ValidConfig` newtype for validated configuration
   - Implement `From<Config>` for `ValidConfig` (after validation)

2. **Individual Validators**
   - `validate_max_complexity` - Returns `Validation<u32, ConfigError>`
   - `validate_min_lines` - Returns `Validation<usize, ConfigError>`
   - `validate_output_format` - Returns `Validation<OutputFormat, ConfigError>`
   - `validate_file_patterns` - Returns `Validation<Vec<Pattern>, ConfigError>`
   - `validate_paths` - Returns `Validation<Vec<PathBuf>, ConfigError>`
   - `validate_thresholds` - Validates threshold consistency
   - Each validator checks single concern

3. **Composite Validation**
   - `validate_config` combines all validators
   - Uses `Validation::all()` for parallel validation
   - Accumulates all errors into `Vec<ConfigError>`
   - Returns `ValidConfig` on success

4. **Error Reporting**
   - Clear error messages for each validation failure
   - Formatted output showing all errors together
   - Suggestions for fixing each error
   - Exit with appropriate error code

### Non-Functional Requirements

1. **User Experience**
   - All errors shown in single run
   - Clear, actionable error messages
   - Consistent error formatting
   - No redundant error messages

2. **Maintainability**
   - Easy to add new validators
   - Each validator independently testable
   - Clear separation of validation concerns
   - Reusable validation functions

3. **Performance**
   - Validation runs in parallel where possible
   - Negligible overhead vs fail-fast
   - Fast enough for CLI startup

## Acceptance Criteria

- [ ] `ConfigError` enum created with variants for all error types
- [ ] `ValidConfig` newtype created wrapping validated `Config`
- [ ] `validate_max_complexity` function returns `Validation` (5-10 lines)
- [ ] `validate_min_lines` function returns `Validation` (5-10 lines)
- [ ] `validate_output_format` function returns `Validation` (10-15 lines)
- [ ] `validate_file_patterns` function returns `Validation` (15-20 lines)
- [ ] `validate_paths` function returns `Validation` (15-20 lines)
- [ ] `validate_thresholds` function validates cross-field constraints (10-15 lines)
- [ ] `validate_config` combines all validators with `Validation::all()` (10-20 lines)
- [ ] Error formatting shows all errors clearly
- [ ] Integration tests verify multiple errors reported together
- [ ] Unit tests for each validator
- [ ] All existing tests pass
- [ ] Documentation updated with validation examples

## Technical Details

### Implementation Approach

**Phase 1: Define Error Types**

```rust
use stillwater::validation::Validation;

/// Configuration validation errors.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    InvalidMaxComplexity { value: u32, reason: String },
    InvalidMinLines { value: usize, reason: String },
    InvalidOutputFormat { format: String, reason: String },
    InvalidFilePattern { pattern: String, error: String },
    InvalidPath { path: PathBuf, reason: String },
    ThresholdInconsistency { message: String },
    MissingRequiredField { field: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidMaxComplexity { value, reason } => {
                write!(f, "Invalid max_complexity ({}): {}", value, reason)
            }
            ConfigError::InvalidMinLines { value, reason } => {
                write!(f, "Invalid min_lines ({}): {}", value, reason)
            }
            ConfigError::InvalidOutputFormat { format, reason } => {
                write!(f, "Invalid output_format ('{}'): {}", format, reason)
            }
            ConfigError::InvalidFilePattern { pattern, error } => {
                write!(f, "Invalid file pattern '{}': {}", pattern, error)
            }
            ConfigError::InvalidPath { path, reason } => {
                write!(f, "Invalid path '{}': {}", path.display(), reason)
            }
            ConfigError::ThresholdInconsistency { message } => {
                write!(f, "Threshold inconsistency: {}", message)
            }
            ConfigError::MissingRequiredField { field } => {
                write!(f, "Missing required field: {}", field)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Validated configuration (type-level proof of validity).
#[derive(Debug, Clone)]
pub struct ValidConfig(Config);

impl ValidConfig {
    pub fn inner(&self) -> &Config {
        &self.0
    }

    pub fn into_inner(self) -> Config {
        self.0
    }
}
```

**Phase 2: Individual Validators (Pure Functions)**

```rust
/// Validates maximum complexity threshold.
///
/// Pure function - no side effects, deterministic.
///
/// # Validation Rules
///
/// - Must be greater than 0
/// - Must be less than 10,000 (sanity check)
/// - Recommended range: 10-200
fn validate_max_complexity(
    max: Option<u32>
) -> Validation<Option<u32>, ConfigError> {
    match max {
        None => Validation::success(None),
        Some(0) => Validation::fail(ConfigError::InvalidMaxComplexity {
            value: 0,
            reason: "must be greater than 0".to_string(),
        }),
        Some(v) if v > 10000 => Validation::fail(ConfigError::InvalidMaxComplexity {
            value: v,
            reason: "unreasonably high (max: 10000)".to_string(),
        }),
        Some(v) => Validation::success(Some(v)),
    }
}

/// Validates minimum lines threshold.
fn validate_min_lines(
    min: Option<usize>
) -> Validation<Option<usize>, ConfigError> {
    match min {
        None => Validation::success(None),
        Some(v) if v > 100000 => Validation::fail(ConfigError::InvalidMinLines {
            value: v,
            reason: "unreasonably high (max: 100000)".to_string(),
        }),
        Some(v) => Validation::success(Some(v)),
    }
}

/// Validates output format.
fn validate_output_format(
    format: Option<String>
) -> Validation<Option<OutputFormat>, ConfigError> {
    match format {
        None => Validation::success(None),
        Some(ref s) => match s.to_lowercase().as_str() {
            "json" => Validation::success(Some(OutputFormat::Json)),
            "yaml" => Validation::success(Some(OutputFormat::Yaml)),
            "text" => Validation::success(Some(OutputFormat::Text)),
            "markdown" => Validation::success(Some(OutputFormat::Markdown)),
            _ => Validation::fail(ConfigError::InvalidOutputFormat {
                format: s.clone(),
                reason: format!(
                    "unknown format (valid: json, yaml, text, markdown)"
                ),
            }),
        },
    }
}

/// Validates file patterns (glob or regex).
fn validate_file_patterns(
    patterns: &[String]
) -> Validation<Vec<Pattern>, ConfigError> {
    let validations: Vec<_> = patterns
        .iter()
        .map(|pattern| {
            Pattern::new(pattern).map_err(|e| ConfigError::InvalidFilePattern {
                pattern: pattern.clone(),
                error: e.to_string(),
            })
        })
        .map(|result| match result {
            Ok(pattern) => Validation::success(pattern),
            Err(err) => Validation::fail(err),
        })
        .collect();

    Validation::sequence(validations)
}

/// Validates paths exist and are accessible.
fn validate_paths(
    paths: &[PathBuf]
) -> Validation<Vec<PathBuf>, ConfigError> {
    if paths.is_empty() {
        return Validation::fail(ConfigError::MissingRequiredField {
            field: "paths".to_string(),
        });
    }

    let validations: Vec<_> = paths
        .iter()
        .map(|path| {
            if path.exists() {
                Validation::success(path.clone())
            } else {
                Validation::fail(ConfigError::InvalidPath {
                    path: path.clone(),
                    reason: "path does not exist".to_string(),
                })
            }
        })
        .collect();

    Validation::sequence(validations)
}

/// Validates threshold consistency (cross-field validation).
fn validate_thresholds(
    cfg: &Config
) -> Validation<(), ConfigError> {
    let mut errors = Vec::new();

    // min_complexity should be less than max_complexity
    if let (Some(min), Some(max)) = (cfg.min_complexity, cfg.max_complexity) {
        if min >= max {
            errors.push(ConfigError::ThresholdInconsistency {
                message: format!(
                    "min_complexity ({}) must be less than max_complexity ({})",
                    min, max
                ),
            });
        }
    }

    // min_lines should be reasonable relative to max_lines
    if let (Some(min), Some(max)) = (cfg.min_lines, cfg.max_lines) {
        if min >= max {
            errors.push(ConfigError::ThresholdInconsistency {
                message: format!(
                    "min_lines ({}) must be less than max_lines ({})",
                    min, max
                ),
            });
        }
    }

    if errors.is_empty() {
        Validation::success(())
    } else {
        Validation::fail_many(errors)
    }
}
```

**Phase 3: Composite Validation**

```rust
/// Validates complete configuration.
///
/// Uses Stillwater's Validation to accumulate all errors.
/// Returns all validation errors in a single pass.
///
/// # Returns
///
/// - `Validation::Success(ValidConfig)` - All validation passed
/// - `Validation::Failure(errors)` - One or more validation errors
///
/// # Examples
///
/// ```
/// let config = Config { /* ... */ };
///
/// match validate_config(&config) {
///     Validation::Success(valid_config) => {
///         // Use valid_config safely
///         run_analysis(valid_config.inner())
///     }
///     Validation::Failure(errors) => {
///         // Show all errors to user
///         eprintln!("Configuration validation failed:");
///         for error in errors {
///             eprintln!("  - {}", error);
///         }
///         std::process::exit(1);
///     }
/// }
/// ```
pub fn validate_config(cfg: &Config) -> Validation<ValidConfig, Vec<ConfigError>> {
    Validation::all((
        validate_max_complexity(cfg.max_complexity),
        validate_min_lines(cfg.min_lines),
        validate_output_format(cfg.format.clone()),
        validate_file_patterns(&cfg.exclude_patterns),
        validate_paths(&cfg.paths),
        validate_thresholds(cfg),
    ))
    .map(|_| ValidConfig(cfg.clone()))
}
```

**Phase 4: Error Formatting and Reporting**

```rust
/// Formats validation errors for user display.
pub fn format_validation_errors(errors: &[ConfigError]) -> String {
    let mut output = String::from("Configuration validation failed:\n\n");

    for (i, error) in errors.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, error));

        // Add suggestions where applicable
        if let Some(suggestion) = get_suggestion(error) {
            output.push_str(&format!("   Suggestion: {}\n", suggestion));
        }
        output.push('\n');
    }

    output.push_str(&format!("Found {} configuration error(s).\n", errors.len()));
    output.push_str("Please fix all errors and try again.\n");

    output
}

/// Provides helpful suggestions for common errors.
fn get_suggestion(error: &ConfigError) -> Option<String> {
    match error {
        ConfigError::InvalidMaxComplexity { value: 0, .. } => {
            Some("Set max_complexity to a positive number (e.g., 50)".to_string())
        }
        ConfigError::InvalidOutputFormat { .. } => {
            Some("Use one of: json, yaml, text, markdown".to_string())
        }
        ConfigError::InvalidPath { path, .. } => {
            Some(format!("Check that '{}' exists and is accessible", path.display()))
        }
        ConfigError::MissingRequiredField { field } if field == "paths" => {
            Some("Provide at least one path to analyze".to_string())
        }
        _ => None,
    }
}
```

**Phase 5: Integration with CLI**

```rust
// In main.rs or config.rs
pub fn load_and_validate_config(cli: Cli) -> Result<ValidConfig> {
    let config = Config::from_cli(cli)?;

    match validate_config(&config) {
        Validation::Success(valid_config) => Ok(valid_config),
        Validation::Failure(errors) => {
            eprintln!("{}", format_validation_errors(&errors));
            std::process::exit(1);
        }
    }
}
```

### Architecture Changes

**Before (Fail-Fast):**
```
validate_config
  ├─ validate_max_complexity? (fails immediately)
  ├─ validate_output_format? (never reached if above fails)
  └─ validate_file_patterns? (never reached if any above fail)
```

**After (Accumulating):**
```
validate_config
  ├─ validate_max_complexity     → Success/Failure
  ├─ validate_min_lines          → Success/Failure
  ├─ validate_output_format      → Success/Failure
  ├─ validate_file_patterns      → Success/Failure
  ├─ validate_paths              → Success/Failure
  └─ validate_thresholds         → Success/Failure
       ↓
  Validation::all combines all results
       ↓
  Returns all errors together (or success)
```

### Data Structures

**ConfigError (Comprehensive Error Type):**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    // Threshold validation
    InvalidMaxComplexity { value: u32, reason: String },
    InvalidMinComplexity { value: u32, reason: String },
    InvalidMinLines { value: usize, reason: String },
    InvalidMaxLines { value: usize, reason: String },

    // Format validation
    InvalidOutputFormat { format: String, reason: String },

    // Pattern validation
    InvalidFilePattern { pattern: String, error: String },
    InvalidExcludePattern { pattern: String, error: String },

    // Path validation
    InvalidPath { path: PathBuf, reason: String },
    PathNotFound { path: PathBuf },
    PathNotReadable { path: PathBuf },

    // Cross-field validation
    ThresholdInconsistency { message: String },

    // Required fields
    MissingRequiredField { field: String },

    // Feature conflicts
    ConflictingOptions { option1: String, option2: String, reason: String },
}
```

### APIs and Interfaces

**Public API:**

```rust
// Validation functions (all pure)
pub fn validate_config(cfg: &Config) -> Validation<ValidConfig, Vec<ConfigError>>;
pub fn validate_max_complexity(max: Option<u32>) -> Validation<Option<u32>, ConfigError>;
pub fn validate_min_lines(min: Option<usize>) -> Validation<Option<usize>, ConfigError>;
pub fn validate_output_format(format: Option<String>) -> Validation<Option<OutputFormat>, ConfigError>;
pub fn validate_file_patterns(patterns: &[String]) -> Validation<Vec<Pattern>, ConfigError>;
pub fn validate_paths(paths: &[PathBuf]) -> Validation<Vec<PathBuf>, ConfigError>;

// Error handling
pub fn format_validation_errors(errors: &[ConfigError]) -> String;

// Types
pub struct ValidConfig(Config);
pub enum ConfigError { /* ... */ }
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/config.rs` - Add validation functions
  - `src/main.rs` - Use validation before analysis
  - `src/commands/analyze.rs` - Accept ValidConfig instead of Config
- **External Dependencies**:
  - `stillwater` (already in use) - Provides `Validation` type

## Testing Strategy

### Unit Tests (Pure Validators)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_max_complexity_valid() {
        let result = validate_max_complexity(Some(50));
        assert!(result.is_success());
    }

    #[test]
    fn test_validate_max_complexity_zero() {
        let result = validate_max_complexity(Some(0));
        assert!(result.is_failure());

        if let Validation::Failure(errors) = result {
            assert_eq!(errors.len(), 1);
            assert!(matches!(
                errors[0],
                ConfigError::InvalidMaxComplexity { value: 0, .. }
            ));
        }
    }

    #[test]
    fn test_validate_max_complexity_too_high() {
        let result = validate_max_complexity(Some(20000));
        assert!(result.is_failure());
    }

    #[test]
    fn test_validate_output_format_valid() {
        let result = validate_output_format(Some("json".to_string()));
        assert!(result.is_success());
    }

    #[test]
    fn test_validate_output_format_invalid() {
        let result = validate_output_format(Some("jsoon".to_string()));
        assert!(result.is_failure());
    }

    #[test]
    fn test_validate_thresholds_consistent() {
        let config = Config {
            min_complexity: Some(10),
            max_complexity: Some(50),
            ..Default::default()
        };

        let result = validate_thresholds(&config);
        assert!(result.is_success());
    }

    #[test]
    fn test_validate_thresholds_inconsistent() {
        let config = Config {
            min_complexity: Some(50),
            max_complexity: Some(10),
            ..Default::default()
        };

        let result = validate_thresholds(&config);
        assert!(result.is_failure());
    }
}
```

### Integration Tests (Multiple Errors)

```rust
#[test]
fn test_validate_config_multiple_errors() {
    let config = Config {
        paths: vec![],  // Missing paths
        max_complexity: Some(0),  // Invalid
        format: Some("jsoon".to_string()),  // Invalid
        exclude_patterns: vec!["[invalid".to_string()],  // Invalid regex
        ..Default::default()
    };

    let result = validate_config(&config);

    match result {
        Validation::Failure(errors) => {
            // Should have multiple errors
            assert!(errors.len() >= 3);

            // Check we got expected errors
            assert!(errors.iter().any(|e| matches!(
                e,
                ConfigError::MissingRequiredField { .. }
            )));
            assert!(errors.iter().any(|e| matches!(
                e,
                ConfigError::InvalidMaxComplexity { .. }
            )));
            assert!(errors.iter().any(|e| matches!(
                e,
                ConfigError::InvalidOutputFormat { .. }
            )));
        }
        Validation::Success(_) => panic!("Expected validation to fail"),
    }
}

#[test]
fn test_validate_config_all_valid() {
    let config = Config {
        paths: vec![PathBuf::from("src")],
        max_complexity: Some(50),
        min_lines: Some(10),
        format: Some("json".to_string()),
        exclude_patterns: vec!["*.test.rs".to_string()],
        ..Default::default()
    };

    let result = validate_config(&config);
    assert!(result.is_success());
}
```

### CLI Integration Tests

```rust
#[test]
fn test_cli_shows_all_errors() {
    let output = Command::new("debtmap")
        .args(&[
            "analyze",
            "--max-complexity", "0",
            "--format", "invalid",
            "--exclude", "[invalid-regex",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should show all errors, not just first one
    assert!(stderr.contains("max_complexity"));
    assert!(stderr.contains("format"));
    assert!(stderr.contains("invalid-regex"));
    assert!(stderr.contains("Found 3 configuration error(s)"));
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Validates configuration using accumulating validation.
///
/// This function uses Stillwater's `Validation` type to collect all
/// configuration errors instead of failing fast. This provides better
/// user experience by showing all problems at once.
///
/// # Pure Function
///
/// This is a pure function with no side effects. It only examines the
/// configuration and returns validation results.
///
/// # Returns
///
/// - `Validation::Success(ValidConfig)` - Configuration is valid
/// - `Validation::Failure(Vec<ConfigError>)` - One or more errors found
///
/// # Examples
///
/// ```
/// let config = Config::from_cli(cli)?;
///
/// match validate_config(&config) {
///     Validation::Success(valid_config) => {
///         // Safe to use - validation passed
///         analyze(valid_config.inner())
///     }
///     Validation::Failure(errors) => {
///         // Show all errors to user
///         for error in errors {
///             eprintln!("Error: {}", error);
///         }
///     }
/// }
/// ```
pub fn validate_config(cfg: &Config) -> Validation<ValidConfig, Vec<ConfigError>> {
    // ...
}
```

### User Documentation

Update CLI help and documentation:

```markdown
## Configuration Validation

Debtmap validates your configuration before starting analysis. If there are
validation errors, all errors will be shown at once so you can fix them
in a single pass.

Example error output:

```
Configuration validation failed:

1. Invalid max_complexity (0): must be greater than 0
   Suggestion: Set max_complexity to a positive number (e.g., 50)

2. Invalid output_format ('jsoon'): unknown format (valid: json, yaml, text, markdown)
   Suggestion: Use one of: json, yaml, text, markdown

3. Invalid file pattern '[invalid': regex parse error: unclosed character class

Found 3 configuration error(s).
Please fix all errors and try again.
```
```

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Configuration Validation

Debtmap uses Stillwater's `Validation` type for configuration validation.
Unlike traditional `Result` types that fail fast, `Validation` accumulates
all errors:

**Fail-Fast (Old Approach):**
- First error stops validation
- User sees one error at a time
- Multiple fix-and-retry cycles

**Accumulating (Current Approach):**
- All validations run
- All errors collected
- User sees everything at once
- Single fix-and-retry cycle

This follows the Stillwater philosophy of composable effects and provides
significantly better user experience.

### Example

```rust
// Each validator is a pure function
fn validate_max_complexity(max: Option<u32>) -> Validation<Option<u32>, ConfigError>
fn validate_output_format(format: Option<String>) -> Validation<Option<OutputFormat>, ConfigError>

// Composed with Validation::all
fn validate_config(cfg: &Config) -> Validation<ValidConfig, Vec<ConfigError>> {
    Validation::all((
        validate_max_complexity(cfg.max_complexity),
        validate_output_format(cfg.format),
        // ... more validators
    ))
}
```

All validators run in parallel, and results are combined into either
`Success(ValidConfig)` or `Failure(Vec<ConfigError>)`.
```

## Implementation Notes

### Refactoring Steps

1. **Create error types** (`ConfigError` enum)
2. **Create validated type** (`ValidConfig` newtype)
3. **Implement individual validators** (one at a time, with tests)
4. **Implement composite validator** (using `Validation::all`)
5. **Add error formatting** (user-friendly output)
6. **Integrate with CLI** (use in main.rs)
7. **Update tests** (verify multiple errors shown)
8. **Update documentation**

### Common Pitfalls

1. **Forgetting validators** - Ensure all config fields validated
2. **Poor error messages** - Make errors actionable and clear
3. **Over-validation** - Don't validate things that can't fail
4. **Missing suggestions** - Help users fix problems

### Pure Function Checklist

For each validator:
- [ ] Takes config data as input
- [ ] Returns `Validation<T, ConfigError>`
- [ ] No I/O operations
- [ ] No side effects
- [ ] Deterministic
- [ ] Easily unit tested

## Migration and Compatibility

### Breaking Changes

**None** - This adds validation without changing behavior for valid configs.

### Migration Steps

1. **Phase 1**: Add validation, make it optional (warn only)
2. **Phase 2**: Make validation required, fail on errors
3. **Phase 3**: Remove old validation code

### Compatibility Considerations

- Valid configs work exactly as before
- Invalid configs now show all errors (improvement)
- Exit codes remain same (1 for validation failure)

## Success Metrics

- ✅ All validators implemented and tested
- ✅ Multiple errors shown in single run
- ✅ Clear, actionable error messages
- ✅ Helpful suggestions for common errors
- ✅ All validation is pure (no I/O)
- ✅ Easy to add new validators
- ✅ Improved user experience vs fail-fast

## Follow-up Work

After this implementation:
- Apply same pattern to other validation (file validation, pattern validation)
- Consider using Validation for analysis error accumulation
- Document Validation patterns for contributors

## References

- **STILLWATER_EVALUATION.md** - Lines 671-682 (Validation recommendation)
- **Stillwater Library** - Validation type documentation
- **CLAUDE.md** - Pure function guidelines
