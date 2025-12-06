---
number: 206
title: Command Handler Type System Improvements
category: optimization
priority: medium
status: draft
dependencies: [182, 204]
created: 2025-12-06
---

# Specification 206: Command Handler Type System Improvements

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 182 (handle_analyze_command), Spec 204 (build_analyze_config)

## Context

The current command handler architecture in `src/main.rs` has several type system issues that create confusion and reduce type safety:

1. **Nested Result Types**: `Result<Result<()>>` appears in command handlers
2. **Unclear Error Boundaries**: Different error types mixed without clear domains
3. **Missing Type State**: Configuration states not encoded in types
4. **Ad-hoc Validation**: Validation scattered across multiple functions
5. **Inconsistent Error Handling**: Some handlers return different error types

Example of problematic signature:
```rust
fn handle_analyze_command(command: Commands) -> Result<Result<()>> {
    // Why nested Result?
    // What's the difference between outer and inner Result?
    // When should each error type be used?
}
```

This creates several problems:
- Developers must mentally track which Result represents what
- Error handling code is unclear (which error to propagate?)
- Type system doesn't prevent invalid states
- Difficult to compose handlers functionally

## Objective

Redesign the command handler type system to:
- Eliminate nested Result types
- Encode configuration states in the type system
- Provide clear error type boundaries
- Enable functional composition of handlers
- Improve compile-time guarantees

Goals:
- Single, unambiguous Result type per handler
- Phantom types for configuration validation states
- Clear domain error types (CLI errors vs Analysis errors)
- Type-safe configuration building
- Functional handler composition

## Requirements

### Functional Requirements

1. **Clear Error Types**
   - Define domain-specific error types (CliError, ConfigError, AnalysisError)
   - Use typed errors instead of `anyhow::Error` where appropriate
   - Provide clear error context and recovery suggestions
   - Map between error types at domain boundaries

2. **Type-State Pattern for Configuration**
   - Encode validation states in types (Unvalidated, Validated)
   - Prevent using unvalidated configuration at compile time
   - Make impossible states unrepresentable
   - Clear type signatures showing validation requirements

3. **Functional Handler Composition**
   - Remove nested Result types
   - Enable handler composition with combinators
   - Support pipeline-style error handling
   - Maintain separation of concerns

4. **Backward Compatibility**
   - Preserve external CLI behavior
   - Maintain existing error messages for users
   - No changes to command-line interface
   - Keep exit codes consistent

### Non-Functional Requirements

1. **Type Safety**
   - Compile-time prevention of invalid states
   - No runtime validation of statically-known facts
   - Exhaustive error handling verification
   - Type-driven correctness

2. **Clarity**
   - Self-documenting type signatures
   - Clear error type meanings
   - Obvious validation boundaries
   - Easy to understand control flow

3. **Composability**
   - Handlers compose functionally
   - Errors map between domains cleanly
   - Reusable validation logic
   - Testable error handling

## Acceptance Criteria

- [ ] Nested `Result<Result<()>>` eliminated from all handlers
- [ ] Domain-specific error types defined (CliError, ConfigError, AnalysisError)
- [ ] Type-state pattern implemented for configuration validation
- [ ] Configuration in Unvalidated state cannot be used for analysis
- [ ] All command handlers have clear, single Result type
- [ ] Error mapping at domain boundaries is explicit
- [ ] All existing tests pass
- [ ] No clippy warnings
- [ ] Type signatures are self-documenting
- [ ] Functional composition examples in tests

## Technical Details

### Implementation Approach

#### Problem Analysis: Nested Results

Current problematic code:
```rust
fn handle_analyze_command(command: Commands) -> Result<Result<()>> {
    // Outer Result: Represents command parsing/validation errors
    // Inner Result: Represents analysis execution errors
    // But this is confusing and error-prone!

    let config = build_analyze_config(/* params */);
    Ok(debtmap::commands::analyze::handle_analyze(config))
    // ^^ Wrapping Result in Result - why?
}
```

**Root cause**: Mixing error domains without type-level separation.

#### Solution: Domain-Specific Error Types

```rust
// src/error.rs

/// Errors that occur during CLI argument parsing and validation
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    #[error("Invalid argument value: {0}")]
    InvalidArgument(String),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
}

/// Errors during configuration building and validation
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid threshold value: {0}")]
    InvalidThreshold(String),

    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("Invalid configuration file: {0}")]
    InvalidConfigFile(#[source] toml::de::Error),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

/// Errors during analysis execution
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("Failed to parse file: {path}")]
    ParseError {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Top-level application error
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("CLI error: {0}")]
    Cli(#[from] CliError),

    #[error("Analysis error: {0}")]
    Analysis(#[from] AnalysisError),
}
```

#### Type-State Pattern for Configuration

```rust
// src/config/state.rs

use std::marker::PhantomData;

/// Marker type for unvalidated configuration
pub struct Unvalidated;

/// Marker type for validated configuration
pub struct Validated;

/// Analysis configuration with type-state tracking
pub struct AnalyzeConfig<State = Unvalidated> {
    // Configuration fields
    pub path: PathBuf,
    pub threshold_complexity: u32,
    pub threshold_duplication: usize,
    // ... all other fields

    // Phantom data to track validation state
    _state: PhantomData<State>,
}

impl AnalyzeConfig<Unvalidated> {
    /// Create new unvalidated configuration
    pub fn new(path: PathBuf) -> Self {
        AnalyzeConfig {
            path,
            threshold_complexity: 50,
            threshold_duplication: 10,
            // ... defaults for other fields
            _state: PhantomData,
        }
    }

    /// Validate configuration, returning validated config or error
    pub fn validate(self) -> Result<AnalyzeConfig<Validated>, ConfigError> {
        // Validate path exists
        if !self.path.exists() {
            return Err(ConfigError::PathNotFound(self.path));
        }

        // Validate thresholds are reasonable
        if self.threshold_complexity == 0 {
            return Err(ConfigError::InvalidThreshold(
                "Complexity threshold must be > 0".to_string()
            ));
        }

        if self.threshold_duplication == 0 {
            return Err(ConfigError::InvalidThreshold(
                "Duplication threshold must be > 0".to_string()
            ));
        }

        // All validations passed - transition to Validated state
        Ok(AnalyzeConfig {
            path: self.path,
            threshold_complexity: self.threshold_complexity,
            threshold_duplication: self.threshold_duplication,
            // ... all other fields
            _state: PhantomData,
        })
    }
}

impl AnalyzeConfig<Validated> {
    /// Only validated configs can be executed
    pub fn execute(self) -> Result<AnalysisResults, AnalysisError> {
        // This function can ONLY be called with validated config
        // Compiler prevents calling with unvalidated config!
        debtmap::commands::analyze::run_analysis(self)
    }
}

// Builder methods work on unvalidated config
impl<State> AnalyzeConfig<State> {
    pub fn with_threshold_complexity(mut self, threshold: u32) -> Self {
        self.threshold_complexity = threshold;
        self
    }

    pub fn with_threshold_duplication(mut self, threshold: usize) -> Self {
        self.threshold_duplication = threshold;
        self
    }

    // ... other builder methods
}
```

#### Refactored Command Handlers

```rust
// src/main.rs

/// Main CLI entry point
fn main() -> Result<(), AppError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { .. } => handle_analyze_command(cli.command)?,
        Commands::Compare { .. } => handle_compare_command(cli.command)?,
        // ... other commands
    }

    Ok(())
}

/// Handle analyze command with clear error types
fn handle_analyze_command(command: Commands) -> Result<(), CliError> {
    // Extract parameters
    let params = extract_analyze_params(command)?;

    // Handle special cases early
    if params.explain_metrics {
        print_metrics_explanation();
        return Ok(());
    }

    // Build configuration groups (from spec 204)
    let path_cfg = build_path_config(&params)?;
    let threshold_cfg = build_threshold_config(&params)?;
    // ... build other configs

    // Create unvalidated configuration
    let config = AnalyzeConfig::new(params.path)
        .with_threshold_complexity(threshold_cfg.complexity)
        .with_threshold_duplication(threshold_cfg.duplication);
        // ... set other fields

    // Validate configuration (state transition)
    let validated_config = config.validate()
        .map_err(CliError::Config)?;

    // Execute analysis (only possible with validated config)
    validated_config.execute()
        .map_err(|e| CliError::Config(ConfigError::ValidationFailed(e.to_string())))?;

    Ok(())
}

/// Extract parameters from command enum
fn extract_analyze_params(command: Commands) -> Result<AnalyzeParams, CliError> {
    match command {
        Commands::Analyze { path, format, /* ... */ } => {
            Ok(AnalyzeParams {
                path,
                format,
                // ... all fields
            })
        }
        _ => Err(CliError::InvalidCommand("Expected Analyze command".to_string())),
    }
}
```

#### Error Mapping at Boundaries

```rust
// src/commands/analyze.rs

/// Run analysis with validated configuration
pub fn run_analysis(
    config: AnalyzeConfig<Validated>
) -> Result<AnalysisResults, AnalysisError> {
    // This function works in the Analysis error domain
    // All I/O and parsing errors are AnalysisError

    let files = discover_files(&config.path)?;

    let results = files
        .into_par_iter()
        .map(|file| analyze_file(&file))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(AnalysisResults::from(results))
}

fn analyze_file(path: &Path) -> Result<FileMetrics, AnalysisError> {
    let content = std::fs::read_to_string(path)
        .map_err(AnalysisError::Io)?;

    let ast = parse_file(&content, path)
        .map_err(|e| AnalysisError::ParseError {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;

    let metrics = compute_metrics(&ast);

    Ok(metrics)
}
```

### Architecture Changes

**Before:**
```
main()
  └─ handle_analyze_command() -> Result<Result<()>>
      ├─ build_analyze_config() -> AnalyzeConfig
      │   └─ (no validation)
      └─ handle_analyze(config) -> Result<()>
          └─ (validation mixed with execution)
```

**After:**
```
main() -> Result<(), AppError>
  └─ handle_analyze_command() -> Result<(), CliError>
      ├─ extract_params() -> Result<AnalyzeParams, CliError>
      ├─ build_config() -> AnalyzeConfig<Unvalidated>
      │   └─ (pure construction, no validation)
      ├─ config.validate() -> Result<AnalyzeConfig<Validated>, ConfigError>
      │   └─ (all validation happens here)
      └─ config.execute() -> Result<AnalysisResults, AnalysisError>
          └─ (can only call with Validated config)
```

### Type System Benefits

**Compile-Time Guarantees:**

```rust
// This COMPILES:
let config = AnalyzeConfig::new(path);
let validated = config.validate()?;
validated.execute()?;  // ✅ Works - config is validated

// This DOES NOT COMPILE:
let config = AnalyzeConfig::new(path);
config.execute()?;  // ❌ Error: execute requires Validated state
//     ^^^^^^^ method `execute` not found for `AnalyzeConfig<Unvalidated>`

// This COMPILES but with clear error types:
fn analyze(config: AnalyzeConfig<Validated>) -> Result<(), AnalysisError> {
    config.execute()  // ✅ Type signature shows validation requirement
}

// This DOES NOT COMPILE:
fn analyze(config: AnalyzeConfig<Unvalidated>) -> Result<(), AnalysisError> {
    config.execute()  // ❌ Cannot execute unvalidated config
}
```

### Error Recovery and User Experience

```rust
// src/error.rs

impl AppError {
    /// Get exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Cli(_) => 2,      // Invalid usage
            AppError::Analysis(_) => 1,  // Analysis failed
        }
    }

    /// Get user-facing error message with recovery suggestions
    pub fn user_message(&self) -> String {
        match self {
            AppError::Cli(CliError::Config(ConfigError::PathNotFound(path))) => {
                format!(
                    "Error: Path '{}' does not exist.\n\n\
                     Suggestion: Check the path and try again, or run:\n\
                     debtmap analyze <path>",
                    path.display()
                )
            }
            AppError::Cli(CliError::Config(ConfigError::InvalidThreshold(msg))) => {
                format!(
                    "Error: {}\n\n\
                     Suggestion: Use --threshold-complexity <n> where n > 0\n\
                     See 'debtmap analyze --help' for more information.",
                    msg
                )
            }
            AppError::Analysis(AnalysisError::ParseError { path, source }) => {
                format!(
                    "Error: Failed to parse '{}':\n  {}\n\n\
                     Suggestion: Check file syntax or exclude with --exclude",
                    path.display(),
                    source
                )
            }
            _ => self.to_string(),
        }
    }
}

// Main error handling
fn main() {
    match run() {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("{}", e.user_message());
            std::process::exit(e.exit_code());
        }
    }
}

fn run() -> Result<(), AppError> {
    let cli = Cli::parse();
    // ... rest of main logic
}
```

### Functional Composition

```rust
// src/handlers.rs

/// Type alias for handler functions
pub type Handler<T, E> = fn(T) -> Result<(), E>;

/// Compose handlers sequentially
pub fn compose_handlers<T, E>(
    handlers: Vec<Handler<T, E>>
) -> impl Fn(T) -> Result<(), E>
where
    T: Clone,
{
    move |input: T| {
        for handler in &handlers {
            handler(input.clone())?;
        }
        Ok(())
    }
}

/// Map errors between domains
pub trait MapErr<T, E, F> {
    fn map_err_to<G>(self, f: F) -> Result<T, G>
    where
        F: FnOnce(E) -> G;
}

impl<T, E, F> MapErr<T, E, F> for Result<T, E> {
    fn map_err_to<G>(self, f: F) -> Result<T, G>
    where
        F: FnOnce(E) -> G,
    {
        self.map_err(f)
    }
}

// Usage:
fn handle_command(cmd: Commands) -> Result<(), CliError> {
    extract_params(cmd)
        .and_then(build_config)
        .and_then(|cfg| cfg.validate().map_err(CliError::Config))
        .and_then(|cfg| cfg.execute().map_err_to(|e| {
            CliError::Config(ConfigError::ValidationFailed(e.to_string()))
        }))
}
```

## Dependencies

- **Prerequisites**:
  - Spec 182: Provides refactored handle_analyze_command
  - Spec 204: Provides configuration group structures
- **Affected Components**:
  - `src/main.rs` - Command handler signatures
  - `src/error.rs` - New error type definitions
  - `src/config/state.rs` - New type-state implementation
  - `src/commands/analyze.rs` - Updated to use typed errors
- **External Dependencies**:
  - `thiserror` crate for error derivation

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unvalidated_config_cannot_execute() {
        // This test ensures the type system prevents execution
        // of unvalidated config (would not compile if attempted)
        let config = AnalyzeConfig::new(PathBuf::from("src"));

        // Can build but not execute
        let config = config.with_threshold_complexity(50);

        // Must validate first
        let validated = config.validate().unwrap();

        // Now can execute
        let _ = validated.execute();
    }

    #[test]
    fn validation_catches_invalid_path() {
        let config = AnalyzeConfig::new(PathBuf::from("/nonexistent"));

        let result = config.validate();

        assert!(matches!(
            result,
            Err(ConfigError::PathNotFound(_))
        ));
    }

    #[test]
    fn validation_catches_zero_threshold() {
        let config = AnalyzeConfig::new(PathBuf::from("src"))
            .with_threshold_complexity(0);

        let result = config.validate();

        assert!(matches!(
            result,
            Err(ConfigError::InvalidThreshold(_))
        ));
    }

    #[test]
    fn error_mapping_preserves_context() {
        let cli_err = CliError::Config(
            ConfigError::InvalidThreshold("test".to_string())
        );

        let app_err: AppError = cli_err.into();

        assert!(matches!(app_err, AppError::Cli(_)));
    }
}
```

### Type System Tests

```rust
// These tests ensure compile-time guarantees
// They should NOT compile (tested via trybuild crate)

#[test]
fn ui_test_unvalidated_cannot_execute() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/unvalidated_execute.rs");
}

// tests/ui/unvalidated_execute.rs:
/*
fn main() {
    let config = AnalyzeConfig::new(PathBuf::from("src"));
    config.execute();  // Should not compile
}
*/
```

### Integration Tests

```rust
#[test]
fn full_command_handler_flow() {
    let command = Commands::Analyze {
        path: PathBuf::from("tests/fixtures"),
        threshold_complexity: 50,
        // ... other params
    };

    let result = handle_analyze_command(command);

    assert!(result.is_ok());
}

#[test]
fn invalid_path_returns_cli_error() {
    let command = Commands::Analyze {
        path: PathBuf::from("/nonexistent"),
        // ... other params
    };

    let result = handle_analyze_command(command);

    assert!(matches!(
        result,
        Err(CliError::Config(ConfigError::PathNotFound(_)))
    ));
}
```

## Documentation Requirements

### Type Documentation

```rust
/// Analysis configuration with compile-time validation tracking.
///
/// This type uses phantom types to track validation state, ensuring
/// that only validated configurations can be executed.
///
/// # Type States
///
/// - `AnalyzeConfig<Unvalidated>`: Configuration that has not been validated
/// - `AnalyzeConfig<Validated>`: Configuration that has passed all validations
///
/// # Examples
///
/// ```
/// use debtmap::config::{AnalyzeConfig, Unvalidated, Validated};
///
/// // Create unvalidated configuration
/// let config = AnalyzeConfig::new(PathBuf::from("src"))
///     .with_threshold_complexity(50);
///
/// // Validate (transitions to Validated state)
/// let validated = config.validate()?;
///
/// // Execute (only possible with Validated config)
/// validated.execute()?;
/// ```
///
/// # Type Safety
///
/// The following code will NOT compile:
///
/// ```compile_fail
/// let config = AnalyzeConfig::new(PathBuf::from("src"));
/// config.execute();  // Error: execute requires Validated state
/// ```
pub struct AnalyzeConfig<State = Unvalidated> { /* ... */ }
```

### Error Handling Guide

Add `docs/ERROR_HANDLING.md`:

```markdown
# Error Handling Guide

## Error Type Hierarchy

```
AppError
├── CliError (CLI parsing and command validation)
│   ├── InvalidCommand
│   ├── MissingArgument
│   └── Config (ConfigError)
│       ├── InvalidThreshold
│       ├── PathNotFound
│       └── ValidationFailed
└── AnalysisError (Analysis execution)
    ├── ParseError
    ├── AnalysisFailed
    └── Io
```

## Error Boundaries

- **CLI Layer**: Use `CliError` for argument parsing and validation
- **Config Layer**: Use `ConfigError` for configuration building
- **Analysis Layer**: Use `AnalysisError` for execution errors

## Best Practices

1. Use `?` operator for error propagation within same domain
2. Use `.map_err()` when crossing error boundaries
3. Provide context with error messages
4. Include recovery suggestions in user-facing errors
```

## Implementation Notes

### Migration Steps

1. **Add thiserror dependency** to `Cargo.toml`
2. **Create `src/error.rs`** with domain error types
3. **Create `src/config/state.rs`** with type-state pattern
4. **Update command handlers** to use typed errors
5. **Remove nested Result types** throughout codebase
6. **Add validation to config.validate()**
7. **Update tests** for new error types
8. **Update documentation** with type-state examples

### Common Pitfalls

1. **Forgetting error mapping** at boundaries
2. **Validation in wrong place** (should be in validate(), not new())
3. **Over-using anyhow** (prefer typed errors at boundaries)
4. **Inconsistent error contexts** (always add helpful messages)

## Migration and Compatibility

### Breaking Changes

**Internal API**: Yes (command handler signatures change)
**External CLI**: No (user-facing behavior unchanged)

### Migration

For internal code that calls handlers:

```rust
// Before:
let result: Result<Result<()>> = handle_analyze_command(cmd);

// After:
let result: Result<(), CliError> = handle_analyze_command(cmd);
```

## Success Metrics

- ✅ Zero nested `Result<Result<T>>` types
- ✅ All handlers use typed errors
- ✅ Type-state pattern prevents unvalidated config execution
- ✅ Compiler enforces validation requirements
- ✅ All tests pass
- ✅ Error messages improved with recovery suggestions
- ✅ User-facing behavior unchanged

## Follow-up Work

- Apply type-state pattern to other command handlers
- Add property-based tests for error handling
- Create error recovery guide for users
- Implement error telemetry and reporting

## References

- **Spec 182**: handle_analyze_command refactoring
- **Spec 204**: build_analyze_config refactoring
- **thiserror**: Error derivation library
- **Rust API Guidelines**: Error handling best practices
- **Type-State Pattern**: Encoding states in types
