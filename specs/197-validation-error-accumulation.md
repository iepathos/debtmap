---
number: 197
title: Validation with Error Accumulation
category: foundation
priority: critical
status: draft
dependencies: [195, 196]
created: 2025-11-24
---

# Specification 197: Validation with Error Accumulation

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 195 (Stillwater Foundation), Spec 196 (Pure Functions)

## Context

Debtmap currently uses fail-fast error handling with `anyhow::Result` throughout the codebase. When analyzing multiple files or validating configuration, execution stops at the first error. This creates a frustrating user experience:

**Current UX (fail-fast):**
```bash
$ debtmap analyze src/
Error: Failed to parse src/file1.rs: unexpected token

# User fixes file1.rs
$ debtmap analyze src/
Error: Failed to parse src/file2.rs: missing semicolon

# User fixes file2.rs
$ debtmap analyze src/
Error: Invalid config in src/file3.rs: bad threshold

# User fixes file3.rs
$ debtmap analyze src/
Success!
```
**Result**: 4 runs to see all 3 errors

**Desired UX (error accumulation):**
```bash
$ debtmap analyze src/
Found 3 errors:
  1. src/file1.rs:12 - Parse error: unexpected token 'fn'
  2. src/file2.rs:45 - Parse error: missing semicolon after statement
  3. src/file3.rs:8 - Invalid threshold: -5 (must be positive)

Tip: Fix these issues and run again. Found 142 other files OK.
```
**Result**: 1 run shows all 3 errors

This specification introduces error accumulation using stillwater's `Validation` type for:
- Configuration validation (collect ALL config errors)
- Multi-file analysis (collect ALL file errors)
- Suppression pattern validation (collect ALL pattern errors)
- Input validation (collect ALL validation errors)

## Objective

Replace fail-fast validation with error-accumulating validation using stillwater's `Validation` type, enabling users to see all errors in a single run instead of discovering them one-by-one.

## Requirements

### Functional Requirements

#### Configuration Validation
- Validate all config fields in parallel, accumulate errors
- Show ALL invalid thresholds, paths, and patterns at once
- Preserve context (which field, which file, which line)
- Support both CLI and file-based config

#### Multi-File Analysis
- Analyze all files, collect parse errors from ALL files
- Continue analysis even if some files fail
- Report all failures at end with context
- Show count of successful vs failed files

#### Suppression Pattern Validation
- Validate all suppression patterns in config
- Check all inline suppression comments
- Accumulate regex compilation errors
- Report all invalid patterns with locations

#### Input Validation
- Validate CLI arguments in parallel
- Validate file paths existence in parallel
- Validate option combinations
- Accumulate all validation errors

### Non-Functional Requirements
- No performance regression (validation is fast)
- Clear error messages with context
- Backwards compatible with existing error handling
- Support both accumulated and fail-fast modes

## Acceptance Criteria

- [ ] Config validation uses `Validation` to accumulate errors
- [ ] Multi-file analysis collects ALL parse errors before failing
- [ ] Suppression pattern validation accumulates ALL regex errors
- [ ] CLI argument validation shows ALL invalid arguments
- [ ] Error output shows numbered list of all errors
- [ ] Each error includes file path, line number, and description
- [ ] Integration tests verify error accumulation behavior
- [ ] Backwards-compatible wrappers maintain existing API
- [ ] Performance tests show no regression
- [ ] User documentation updated with examples

## Technical Details

### Implementation Approach

#### 1. Config Validation with Error Accumulation

```rust
// src/config/validation.rs
use stillwater::Validation;

/// Validate entire config, accumulating ALL errors
pub fn validate_config(raw: &RawConfig) -> AnalysisValidation<Config> {
    Validation::all((
        validate_thresholds(&raw.thresholds),
        validate_paths(&raw.paths),
        validate_patterns(&raw.patterns),
        validate_exclusions(&raw.exclusions),
    ))
    .map(|(thresholds, paths, patterns, exclusions)| Config {
        thresholds,
        paths,
        patterns,
        exclusions,
    })
}

/// Validate thresholds - accumulate ALL threshold errors
fn validate_thresholds(thresholds: &Thresholds) -> Validation<Thresholds, AnalysisError> {
    let mut errors = Vec::new();

    if thresholds.complexity < 0.0 {
        errors.push(AnalysisError::ConfigError(
            format!("Complexity threshold cannot be negative: {}", thresholds.complexity)
        ));
    }

    if thresholds.complexity > 1000.0 {
        errors.push(AnalysisError::ConfigError(
            format!("Complexity threshold too high: {} (max: 1000)", thresholds.complexity)
        ));
    }

    if thresholds.coverage < 0.0 || thresholds.coverage > 100.0 {
        errors.push(AnalysisError::ConfigError(
            format!("Coverage threshold must be 0-100: {}", thresholds.coverage)
        ));
    }

    if thresholds.depth < 1 {
        errors.push(AnalysisError::ConfigError(
            format!("Nesting depth must be at least 1: {}", thresholds.depth)
        ));
    }

    if errors.is_empty() {
        Validation::Success(thresholds.clone())
    } else {
        Validation::Failure(errors)
    }
}

/// Validate paths - accumulate ALL path errors
fn validate_paths(paths: &[PathBuf]) -> Validation<Vec<PathBuf>, AnalysisError> {
    let validations: Vec<Validation<PathBuf, AnalysisError>> = paths
        .iter()
        .map(|path| {
            if path.exists() {
                Validation::Success(path.clone())
            } else {
                Validation::Failure(vec![
                    AnalysisError::ConfigError(
                        format!("Path not found: {}", path.display())
                    )
                ])
            }
        })
        .collect();

    // Collect all errors or all successes
    Validation::sequence(validations)
}

/// Validate patterns - accumulate ALL regex errors
fn validate_patterns(patterns: &[String]) -> Validation<Vec<Regex>, AnalysisError> {
    let validations: Vec<Validation<Regex, AnalysisError>> = patterns
        .iter()
        .map(|pattern| {
            Regex::new(pattern)
                .map(Validation::Success)
                .unwrap_or_else(|e| Validation::Failure(vec![
                    AnalysisError::ConfigError(
                        format!("Invalid regex pattern '{}': {}", pattern, e)
                    )
                ]))
        })
        .collect();

    Validation::sequence(validations)
}
```

#### 2. Multi-File Analysis with Error Accumulation

```rust
// src/builders/validated_analysis.rs
use stillwater::{Validation, traverse};

/// Analyze multiple files, accumulating ALL errors
pub fn analyze_files_validated(
    files: Vec<PathBuf>,
    config: &Config,
) -> AnalysisValidation<Vec<FileMetrics>> {
    // First: Validate all files can be read
    let read_validations: Vec<Validation<(PathBuf, String), AnalysisError>> = files
        .into_iter()
        .map(|path| validate_file_readable(&path))
        .collect();

    // Collect all read errors
    let validated_files = Validation::sequence(read_validations);

    // Then: Parse and analyze each file
    validated_files.and_then(|file_contents| {
        let parse_validations: Vec<Validation<FileMetrics, AnalysisError>> = file_contents
            .into_iter()
            .map(|(path, content)| {
                Validation::all((
                    parse_file_validated(&content, &path),
                    check_not_suppressed(&path, config),
                ))
                .and_then(|(ast, _)| {
                    // Pure analysis (can't fail)
                    let metrics = analyze_ast_pure(&ast, &path);
                    Validation::Success(metrics)
                })
            })
            .collect();

        Validation::sequence(parse_validations)
    })
}

fn validate_file_readable(path: &PathBuf) -> Validation<(PathBuf, String), AnalysisError> {
    match std::fs::read_to_string(path) {
        Ok(content) => Validation::Success((path.clone(), content)),
        Err(e) => Validation::Failure(vec![
            AnalysisError::IoError(
                format!("Failed to read {}: {}", path.display(), e)
            )
        ]),
    }
}

fn parse_file_validated(content: &str, path: &PathBuf) -> Validation<syn::File, AnalysisError> {
    match syn::parse_file(content) {
        Ok(ast) => Validation::Success(ast),
        Err(e) => Validation::Failure(vec![
            AnalysisError::ParseError(
                format!("{}:{}: {}", path.display(), e.span().start().line, e)
            )
        ]),
    }
}

fn check_not_suppressed(path: &PathBuf, config: &Config) -> Validation<(), AnalysisError> {
    if config.is_suppressed(path) {
        Validation::Failure(vec![
            AnalysisError::ValidationError(
                format!("File is suppressed: {}", path.display())
            )
        ])
    } else {
        Validation::Success(())
    }
}
```

#### 3. Backwards-Compatible API

```rust
// src/config/loader.rs

/// Load and validate config - new API with error accumulation
pub fn load_config_validated(path: &Path) -> AnalysisValidation<Config> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            match toml::from_str::<RawConfig>(&content) {
                Ok(raw) => validate_config(&raw),
                Err(e) => Validation::Failure(vec![
                    AnalysisError::ConfigError(
                        format!("TOML parse error: {}", e)
                    )
                ]),
            }
        }
        Err(e) => Validation::Failure(vec![
            AnalysisError::IoError(
                format!("Failed to read config file: {}", e)
            )
        ]),
    }
}

/// Load and validate config - backwards-compatible API
pub fn load_config(path: &Path) -> anyhow::Result<Config> {
    load_config_validated(path)
        .into_result()
        .map_err(|errors| {
            anyhow::anyhow!(
                "Configuration validation failed:\n{}",
                errors.iter()
                    .enumerate()
                    .map(|(i, e)| format!("  {}. {}", i + 1, e))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
}
```

#### 4. CLI Error Reporting

```rust
// src/commands/analyze.rs

pub fn run_analyze_command(args: &AnalyzeArgs) -> anyhow::Result<()> {
    // Validate config first
    let config_validation = validate_config_from_args(args);

    match config_validation {
        Validation::Success(config) => {
            // Proceed with analysis
            let analysis = analyze_files_validated(
                discover_files(&config)?,
                &config,
            );

            match analysis {
                Validation::Success(results) => {
                    print_results(&results);
                    Ok(())
                }
                Validation::Failure(errors) => {
                    print_error_report(errors);
                    std::process::exit(1);
                }
            }
        }
        Validation::Failure(errors) => {
            print_error_report(errors);
            std::process::exit(1);
        }
    }
}

fn print_error_report(errors: Vec<AnalysisError>) {
    eprintln!("\n{} {} found:\n",
        "Error:".red().bold(),
        if errors.len() == 1 { "1 issue" } else { &format!("{} issues", errors.len()) }
    );

    for (i, error) in errors.iter().enumerate() {
        eprintln!("  {}. {}", (i + 1).to_string().yellow(), error);
    }

    eprintln!(
        "\n{} Fix the issues above and run again.",
        "Tip:".cyan().bold()
    );
}
```

#### 5. Enhanced Error Types with Context

```rust
// src/errors.rs

#[derive(Debug, Clone)]
pub enum AnalysisError {
    IoError(String),
    ParseError(String),
    ValidationError(String),
    ConfigError(String),
    CoverageError(String),
    Other(String),
}

impl AnalysisError {
    /// Add context to error
    pub fn with_context(self, context: impl AsRef<str>) -> Self {
        match self {
            Self::IoError(msg) => Self::IoError(format!("{}: {}", context.as_ref(), msg)),
            Self::ParseError(msg) => Self::ParseError(format!("{}: {}", context.as_ref(), msg)),
            Self::ValidationError(msg) => Self::ValidationError(format!("{}: {}", context.as_ref(), msg)),
            Self::ConfigError(msg) => Self::ConfigError(format!("{}: {}", context.as_ref(), msg)),
            Self::CoverageError(msg) => Self::CoverageError(format!("{}: {}", context.as_ref(), msg)),
            Self::Other(msg) => Self::Other(format!("{}: {}", context.as_ref(), msg)),
        }
    }

    /// Extract location from error if present
    pub fn location(&self) -> Option<(&str, u32)> {
        let msg = match self {
            Self::IoError(m) | Self::ParseError(m) | Self::ValidationError(m)
            | Self::ConfigError(m) | Self::CoverageError(m) | Self::Other(m) => m,
        };

        // Parse format: "path/to/file.rs:123: error message"
        if let Some((location, _)) = msg.split_once(": ") {
            if let Some((path, line)) = location.rsplit_once(':') {
                if let Ok(line_num) = line.parse::<u32>() {
                    return Some((path, line_num));
                }
            }
        }

        None
    }
}
```

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_accumulates_all_errors() {
        let config = RawConfig {
            thresholds: Thresholds {
                complexity: -5.0,    // Error 1
                coverage: 150.0,     // Error 2
                depth: 0,            // Error 3
            },
            paths: vec![
                PathBuf::from("/nonexistent1"),  // Error 4
                PathBuf::from("/nonexistent2"),  // Error 5
            ],
            patterns: vec![
                "[unclosed".to_string(),  // Error 6
                "(?P<".to_string(),       // Error 7
            ],
            exclusions: vec![],
        };

        let result = validate_config(&config);

        match result {
            Validation::Failure(errors) => {
                assert_eq!(errors.len(), 7);  // ALL errors collected
            }
            _ => panic!("Expected validation failure"),
        }
    }

    #[test]
    fn test_multi_file_accumulates_parse_errors() {
        let files = vec![
            write_temp_file("bad1.rs", "invalid rust {{{"),
            write_temp_file("good.rs", "fn main() {}"),
            write_temp_file("bad2.rs", "fn incomplete("),
        ];

        let result = analyze_files_validated(files, &Config::default());

        match result {
            Validation::Failure(errors) => {
                assert_eq!(errors.len(), 2);  // Both parse errors
                assert!(errors[0].to_string().contains("bad1.rs"));
                assert!(errors[1].to_string().contains("bad2.rs"));
            }
            _ => panic!("Expected validation failure"),
        }
    }

    #[test]
    fn test_valid_config_succeeds() {
        let config = RawConfig {
            thresholds: Thresholds {
                complexity: 10.0,
                coverage: 80.0,
                depth: 5,
            },
            paths: vec![PathBuf::from(".")],
            patterns: vec![".*\\.rs$".to_string()],
            exclusions: vec![],
        };

        let result = validate_config(&config);

        assert!(matches!(result, Validation::Success(_)));
    }

    #[test]
    fn test_error_context_preserved() {
        let error = AnalysisError::ParseError("unexpected token".to_string())
            .with_context("src/main.rs:42");

        assert_eq!(error.to_string(), "src/main.rs:42: unexpected token");
        assert_eq!(error.location(), Some(("src/main.rs", 42)));
    }

    // Integration test
    #[test]
    fn test_cli_error_output() {
        let errors = vec![
            AnalysisError::ConfigError("Invalid threshold: -5".to_string()),
            AnalysisError::ParseError("src/bad.rs:12: unexpected token".to_string()),
            AnalysisError::IoError("Permission denied: /etc/secret".to_string()),
        ];

        let output = capture_error_report(errors);

        assert!(output.contains("3 issues found"));
        assert!(output.contains("1. Invalid threshold"));
        assert!(output.contains("2. src/bad.rs:12"));
        assert!(output.contains("3. Permission denied"));
    }
}
```

## Dependencies

- **Prerequisites**:
  - Spec 195 (Stillwater Foundation) - Provides Validation type
  - Spec 196 (Pure Functions) - Used in analysis pipeline
- **Blocked by**: None
- **Blocks**:
  - Improves UX for all analysis commands
  - Enables better CI/CD integration (see all errors at once)
- **Affected Components**:
  - `src/config/` - Config validation
  - `src/builders/` - Multi-file analysis
  - `src/commands/` - CLI error reporting
  - `tests/` - Integration tests
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Config validation accumulates all errors
  - Path validation accumulates all errors
  - Pattern validation accumulates all errors
  - Multi-file analysis accumulates all errors

- **Integration Tests**:
  - CLI shows all errors in one run
  - Error messages include context
  - Location parsing works correctly
  - Backwards compatibility maintained

- **User Experience Tests**:
  - Create project with multiple errors
  - Verify all shown in single run
  - Verify clear, actionable messages

## Documentation Requirements

- **Code Documentation**:
  - Document validation patterns
  - Explain error accumulation benefits
  - Show examples of Validation usage

- **User Documentation**:
  - Update user guide with examples
  - Show how error accumulation improves workflow
  - Document error message format

- **Architecture Updates**:
  - Document validation strategy
  - Explain fail-fast vs accumulation tradeoffs

## Implementation Notes

### User Experience Improvements

**Before (fail-fast):**
- 3 errors = 3 runs
- User frustration
- Slow feedback loop

**After (accumulation):**
- 3 errors = 1 run
- All issues visible
- Fast feedback loop

### Files to Create
- `src/config/validation.rs`
- `src/builders/validated_analysis.rs`

### Files to Modify
- `src/config/loader.rs`
- `src/commands/analyze.rs`
- `src/errors.rs`

### Estimated Effort
- Config validation: 4-6 hours
- Multi-file analysis: 6-8 hours
- CLI integration: 3-4 hours
- Tests: 4-6 hours
- Documentation: 2-3 hours
- **Total: 19-27 hours**

## Migration and Compatibility

### Breaking Changes

None. Validation is additive:
- New validated APIs added
- Old APIs still work (fail-fast)
- Can mix both approaches

### Migration Path

1. Start with config validation
2. Add multi-file analysis
3. Update CLI to use validation
4. Gradually migrate other modules

## Success Metrics

- **User Experience**: Show ALL errors in 1 run instead of N runs
- **Test Coverage**: 95%+ on validation logic
- **Performance**: No regression (validation is fast)
- **Compatibility**: All existing tests pass

## Future Considerations

After this spec, error accumulation can be applied to:
- Suppression pattern validation
- Coverage data validation
- Output format validation
- Any multi-item validation scenario
