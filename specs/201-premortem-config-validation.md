---
number: 201
title: Configuration Validation with Premortem
category: foundation
priority: high
status: draft
dependencies: []
supersedes: [184]
created: 2025-12-10
---

# Specification 201: Configuration Validation with Premortem

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None
**Supersedes**: Spec 184 (Add Validation Type for Config Validation)

## Context

Spec 184 proposed using Stillwater's raw `Validation` type for configuration validation. While this approach accumulates all errors, it requires significant boilerplate for:

- Source location tracking (which file/line caused the error)
- Layered configuration (files + environment variables)
- Derive macros for declarative validation
- Value tracing (where each config value came from)

The **premortem** library builds on Stillwater and provides all of this out of the box. It's specifically designed for configuration validation with the motto "Know how your app will die—before it does."

**Current State (debtmap):**
```rust
// src/builders/unified_analysis.rs
pub struct UnifiedAnalysisOptions<'a> {
    pub parallel: bool,
    pub jobs: usize,
    pub aggregate_only: bool,
    pub no_aggregation: bool,  // Mutually exclusive with aggregate_only!
    pub coverage_file: Option<PathBuf>,
    pub enable_context: bool,
    // ... 15+ more fields with implicit constraints
}
```

**Problems:**
1. No validation at all - invalid combinations silently cause bugs
2. `aggregate_only` + `no_aggregation` can both be true (contradiction)
3. `jobs: 0` with `parallel: true` is undefined behavior
4. `coverage_file` path not validated to exist
5. No source location tracking - hard to debug config issues

**With premortem:**
```
$ debtmap analyze --aggregate-only --no-aggregation
Configuration errors (2):
  [cli:--aggregate-only] conflicting options: aggregate_only and no_aggregation are mutually exclusive
  [cli:--no-aggregation] conflicting options: aggregate_only and no_aggregation are mutually exclusive
```

## Objective

Replace manual/no validation with **premortem** to:

1. **Validate all configuration at once** - Accumulate ALL errors before failing
2. **Track value sources** - Know if error came from file, env, or CLI
3. **Use derive macros** - `#[derive(Validate)]` for declarative constraints
4. **Enable layered config** - Support `debtmap.toml` + env vars + CLI args
5. **Provide actionable errors** - Clear messages with source locations

**Success Metric**: Users see all configuration problems in one run, with exact source locations.

## Requirements

### Functional Requirements

1. **Add premortem Dependency**
   - Add `premortem = "0.6"` to Cargo.toml
   - Use `premortem::prelude::*` for common imports

2. **Define Validated Configuration Types**
   - `AnalysisConfig` - Main configuration struct with `#[derive(Validate)]`
   - Derive macros for common constraints (non_empty, range, etc.)
   - Custom validation for cross-field constraints

3. **Implement Source Layering**
   - Layer 1 (lowest): Built-in defaults
   - Layer 2: `debtmap.toml` file (if exists)
   - Layer 3: Environment variables (`DEBTMAP_*`)
   - Layer 4 (highest): CLI arguments

4. **Validate Mutual Exclusions**
   - `aggregate_only` XOR `no_aggregation`
   - `parallel: true` requires `jobs > 0`
   - `multi_pass` requires `enable_context`

5. **Validate Paths**
   - `coverage_file` must exist if provided
   - `project_path` must be a directory
   - Exclude patterns must be valid globs

6. **Error Formatting**
   - Show all errors with source locations
   - Include suggestions for common mistakes
   - Exit with appropriate code (1 for validation failure)

### Non-Functional Requirements

1. **Testability** - All validation via `MockEnv` for deterministic tests
2. **Performance** - Validation adds < 10ms to startup
3. **Maintainability** - New fields get validation via derive macros
4. **Documentation** - Each constraint documented in struct

## Acceptance Criteria

- [ ] `premortem` added to dependencies
- [ ] `AnalysisConfig` struct with `#[derive(Validate)]`
- [ ] `#[validate(non_empty)]` on project_path
- [ ] `#[validate(range(1..=256))]` on jobs when parallel
- [ ] Custom validator for mutual exclusions
- [ ] Source layering: defaults → file → env → CLI
- [ ] Errors show source location (e.g., `[debtmap.toml:5]`)
- [ ] All tests use `MockEnv` - no real I/O in tests
- [ ] Integration test verifies multiple errors reported together
- [ ] Migration guide for any config file format changes

## Technical Details

### Implementation Approach

**Phase 1: Add premortem and Define Types**

```rust
// src/config/analysis.rs
use premortem::prelude::*;
use serde::Deserialize;
use std::path::PathBuf;

/// Analysis configuration with declarative validation.
///
/// Uses premortem for error accumulation and source tracking.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct AnalysisConfig {
    /// Project root directory to analyze.
    #[validate(non_empty)]
    pub project_path: PathBuf,

    /// Enable parallel analysis.
    #[serde(default)]
    pub parallel: bool,

    /// Number of parallel jobs (1-256).
    #[validate(range(1..=256))]
    #[serde(default = "default_jobs")]
    pub jobs: usize,

    /// Output only aggregate results.
    #[serde(default)]
    pub aggregate_only: bool,

    /// Disable aggregation entirely.
    #[serde(default)]
    pub no_aggregation: bool,

    /// LCOV coverage file path.
    pub coverage_file: Option<PathBuf>,

    /// Enable context-aware analysis.
    #[serde(default)]
    pub enable_context: bool,

    /// Enable multi-pass analysis.
    #[serde(default)]
    pub multi_pass: bool,

    /// Complexity threshold for recommendations.
    #[validate(range(1..=1000))]
    #[serde(default = "default_complexity_threshold")]
    pub complexity_threshold: u32,

    /// File patterns to exclude.
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
}

fn default_complexity_threshold() -> u32 { 50 }
```

**Phase 2: Custom Cross-Field Validation**

```rust
use premortem::{ConfigValidation, ConfigError, Validation, validate_field};

impl Validate for AnalysisConfig {
    fn validate(&self) -> ConfigValidation<()> {
        Validation::all((
            // Individual field validation from derive macro runs first
            self.validate_derived(),

            // Cross-field validations
            self.validate_mutual_exclusions(),
            self.validate_parallel_jobs(),
            self.validate_context_dependencies(),
            self.validate_paths(),
        ))
        .map(|_| ())
    }
}

impl AnalysisConfig {
    /// Validates mutually exclusive options.
    fn validate_mutual_exclusions(&self) -> ConfigValidation<()> {
        if self.aggregate_only && self.no_aggregation {
            Validation::fail_with(ConfigError::ValidationError {
                path: "aggregate_only".to_string(),
                source_location: current_source_location("aggregate_only"),
                value: Some("true".to_string()),
                message: "aggregate_only and no_aggregation are mutually exclusive".to_string(),
            })
        } else {
            Validation::Success(())
        }
    }

    /// Validates parallel job configuration.
    fn validate_parallel_jobs(&self) -> ConfigValidation<()> {
        if self.parallel && self.jobs == 0 {
            Validation::fail_with(ConfigError::ValidationError {
                path: "jobs".to_string(),
                source_location: current_source_location("jobs"),
                value: Some("0".to_string()),
                message: "jobs must be > 0 when parallel is enabled".to_string(),
            })
        } else {
            Validation::Success(())
        }
    }

    /// Validates context-dependent options.
    fn validate_context_dependencies(&self) -> ConfigValidation<()> {
        if self.multi_pass && !self.enable_context {
            Validation::fail_with(ConfigError::ValidationError {
                path: "multi_pass".to_string(),
                source_location: current_source_location("multi_pass"),
                value: Some("true".to_string()),
                message: "multi_pass requires enable_context to be true".to_string(),
            })
        } else {
            Validation::Success(())
        }
    }

    /// Validates file paths exist.
    fn validate_paths(&self) -> ConfigValidation<()> {
        let mut validations = vec![
            validate_path_is_dir(&self.project_path, "project_path"),
        ];

        if let Some(ref coverage) = self.coverage_file {
            validations.push(validate_path_exists(coverage, "coverage_file"));
        }

        Validation::all_vec(validations).map(|_| ())
    }
}

fn validate_path_exists(path: &PathBuf, field: &str) -> ConfigValidation<()> {
    if path.exists() {
        Validation::Success(())
    } else {
        Validation::fail_with(ConfigError::ValidationError {
            path: field.to_string(),
            source_location: current_source_location(field),
            value: Some(path.display().to_string()),
            message: format!("file does not exist: {}", path.display()),
        })
    }
}

fn validate_path_is_dir(path: &PathBuf, field: &str) -> ConfigValidation<()> {
    if path.is_dir() {
        Validation::Success(())
    } else if !path.exists() {
        Validation::fail_with(ConfigError::ValidationError {
            path: field.to_string(),
            source_location: current_source_location(field),
            value: Some(path.display().to_string()),
            message: format!("directory does not exist: {}", path.display()),
        })
    } else {
        Validation::fail_with(ConfigError::ValidationError {
            path: field.to_string(),
            source_location: current_source_location(field),
            value: Some(path.display().to_string()),
            message: format!("path is not a directory: {}", path.display()),
        })
    }
}
```

**Phase 3: Source Layering**

```rust
// src/config/loader.rs
use premortem::{Config, Toml, Env, Defaults};

/// Loads configuration with layered sources.
///
/// Priority (highest to lowest):
/// 1. CLI arguments (passed separately)
/// 2. Environment variables (DEBTMAP_*)
/// 3. Config file (debtmap.toml)
/// 4. Built-in defaults
pub fn load_config() -> Result<AnalysisConfig, ConfigErrors> {
    Config::<AnalysisConfig>::builder()
        .source(Defaults::from(AnalysisConfig::default()))
        .source(Toml::file("debtmap.toml").optional())
        .source(Env::prefix("DEBTMAP"))
        .build()
}

/// Loads with CLI overrides applied.
pub fn load_config_with_cli(cli: &Cli) -> Result<AnalysisConfig, ConfigErrors> {
    let mut config = load_config()?;

    // Apply CLI overrides
    if let Some(path) = &cli.path {
        config.project_path = path.clone();
    }
    if cli.parallel {
        config.parallel = true;
    }
    if let Some(jobs) = cli.jobs {
        config.jobs = jobs;
    }
    // ... more CLI overrides

    // Re-validate after CLI overrides
    config.validate().map(|()| config).map_err(|e| e.into())
}
```

**Phase 4: Integration with Main**

```rust
// src/main.rs
fn main() {
    let cli = Cli::parse();

    let config = match load_config_with_cli(&cli) {
        Ok(config) => config,
        Err(errors) => {
            // premortem formats errors with source locations
            eprintln!("{}", errors.pretty_print(PrettyPrintOptions::default()));
            std::process::exit(1);
        }
    };

    // config is now validated - safe to use
    run_analysis(&config)
}
```

### Example Output

**Multiple Errors with Source Locations:**
```
Configuration errors (3):

  [debtmap.toml:8] aggregate_only: aggregate_only and no_aggregation are mutually exclusive
    value: true
    suggestion: Remove one of these options

  [env:DEBTMAP_JOBS] jobs: value 0 is not in range 1..=256
    value: 0
    suggestion: Set DEBTMAP_JOBS to a value between 1 and 256

  [cli:--coverage-file] coverage_file: file does not exist: ./coverage.lcov
    value: ./coverage.lcov
    suggestion: Check that the file path is correct

Found 3 configuration errors. Fix all errors and try again.
```

### File Structure

```
src/config/
├── mod.rs           # Re-exports
├── analysis.rs      # AnalysisConfig struct with validation
├── loader.rs        # Source layering and loading
├── validators.rs    # Custom validators (paths, patterns)
└── tests.rs         # Unit tests with MockEnv
```

### Architecture Changes

**Before:**
```
CLI args → parse → Config struct → use (no validation)
```

**After:**
```
Defaults → debtmap.toml → DEBTMAP_* env → CLI args
                    ↓
              Merged config
                    ↓
              Validation (all errors at once)
                    ↓
              AnalysisConfig (validated)
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/config/` - New module for configuration
  - `src/main.rs` - Use new config loading
  - `src/builders/unified_analysis.rs` - Accept validated config
  - `src/commands/*.rs` - Use validated config
- **External Dependencies**:
  - `premortem = "0.6"` - Configuration library with validation
  - `stillwater` - Already present (premortem uses it internally)

## Testing Strategy

### Unit Tests with MockEnv

```rust
#[cfg(test)]
mod tests {
    use premortem::MockEnv;
    use super::*;

    #[test]
    fn test_valid_config_from_file() {
        let env = MockEnv::new()
            .with_file("debtmap.toml", r#"
                project_path = "src"
                parallel = true
                jobs = 4
            "#);

        let config = Config::<AnalysisConfig>::builder()
            .source(Toml::file("debtmap.toml"))
            .build_with_env(&env);

        assert!(config.is_ok());
    }

    #[test]
    fn test_mutual_exclusion_error() {
        let env = MockEnv::new()
            .with_file("debtmap.toml", r#"
                project_path = "src"
                aggregate_only = true
                no_aggregation = true
            "#);

        let result = Config::<AnalysisConfig>::builder()
            .source(Toml::file("debtmap.toml"))
            .build_with_env(&env);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e|
            e.to_string().contains("mutually exclusive")
        ));
    }

    #[test]
    fn test_multiple_errors_accumulated() {
        let env = MockEnv::new()
            .with_file("debtmap.toml", r#"
                project_path = ""
                jobs = 0
                aggregate_only = true
                no_aggregation = true
            "#);

        let result = Config::<AnalysisConfig>::builder()
            .source(Toml::file("debtmap.toml"))
            .build_with_env(&env);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.len() >= 3, "Expected at least 3 errors");
    }

    #[test]
    fn test_env_overrides_file() {
        let env = MockEnv::new()
            .with_file("debtmap.toml", r#"
                project_path = "src"
                jobs = 2
            "#)
            .with_env("DEBTMAP_JOBS", "8");

        let config = Config::<AnalysisConfig>::builder()
            .source(Toml::file("debtmap.toml"))
            .source(Env::prefix("DEBTMAP"))
            .build_with_env(&env)
            .unwrap();

        assert_eq!(config.jobs, 8); // Env wins
    }
}
```

### Integration Tests

```rust
#[test]
fn test_cli_shows_all_errors_with_locations() {
    let output = Command::new("cargo")
        .args(&["run", "--", "analyze",
            "--aggregate-only", "--no-aggregation",
            "--coverage-file", "nonexistent.lcov"])
        .output()
        .expect("Failed to run");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should show multiple errors
    assert!(stderr.contains("Configuration errors"));
    assert!(stderr.contains("mutually exclusive"));
    assert!(stderr.contains("file does not exist"));

    // Should show source locations
    assert!(stderr.contains("[cli:"));
}
```

## Documentation Requirements

- **Code Documentation**: All config fields documented with constraints
- **User Documentation**: Add `docs/configuration.md` explaining config file format
- **Architecture Updates**: Update `ARCHITECTURE.md` with config flow diagram

## Migration and Compatibility

### Breaking Changes

1. **New config file format**: `debtmap.toml` (optional)
2. **Stricter validation**: Previously silent issues now fail

### Migration Steps

1. **Phase 1**: Add premortem, warn on validation failures (don't exit)
2. **Phase 2**: Exit on validation failures after 1 release
3. **Phase 3**: Remove deprecation warnings

### Backwards Compatibility

- CLI arguments work exactly as before
- Config file is optional - not required
- Environment variables are additive - don't break existing setups

## Implementation Notes

### Why premortem over raw Stillwater?

| Feature | Raw Stillwater | premortem |
|---------|---------------|-----------|
| Error accumulation | ✅ Manual | ✅ Automatic |
| Source locations | ❌ Manual | ✅ Automatic |
| Derive macros | ❌ None | ✅ `#[derive(Validate)]` |
| File loading | ❌ Manual | ✅ Built-in (TOML, JSON, YAML) |
| Env vars | ❌ Manual | ✅ Built-in prefix mapping |
| Source layering | ❌ Manual | ✅ Built-in |
| Mock testing | ❌ Manual | ✅ `MockEnv` |
| Value tracing | ❌ None | ✅ Track where values came from |

premortem is built on Stillwater but adds the configuration-specific features we need.

## References

- **premortem documentation**: https://docs.rs/premortem
- **Stillwater Validation**: Foundation for error accumulation
- **Spec 184**: Original config validation proposal (superseded by this spec)
