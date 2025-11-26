---
number: 201
title: Premortem Configuration Integration
category: foundation
priority: medium
status: draft
dependencies: [195, 197, 199]
created: 2025-11-25
---

# Specification 201: Premortem Configuration Integration

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 195 (Stillwater Foundation), Spec 197 (Validation Error Accumulation), Spec 199 (Reader Pattern)

## Context

Debtmap currently lacks robust configuration file support. Configuration is handled through:
- **CLI arguments** via clap (primary method)
- **Hardcoded defaults** in `DefaultConfigProvider` (HashMap with static values)
- **No config file support** (`load_from_file()` is a stub returning `Ok(())`)

The premortem crate (v0.3.0 on crates.io) provides configuration management with:
- **Multi-source configuration** (files, environment variables, defaults)
- **Error accumulation** (shows ALL config errors at once)
- **Source tracking** (knows where each value came from)
- **Derive macros** for validation (`#[derive(Validate)]`)
- **Built on stillwater 0.8** (same foundation as specs 195-200)

Integrating premortem enables:
1. **Config file support** - `.debtmap.toml` for persistent settings
2. **Environment variable overrides** - `DEBTMAP_*` for CI/CD
3. **Error accumulation** - Aligns with spec 197's goals for config validation
4. **Source transparency** - Debug which setting came from where

## Objective

Integrate the premortem crate to provide robust configuration management with multi-source loading, error accumulation, and source tracking, while maintaining backwards compatibility with existing CLI-only workflow.

## Requirements

### Functional Requirements

#### Configuration File Support
- Support `.debtmap.toml` configuration file in project root
- Support `~/.config/debtmap/config.toml` for user defaults
- Support `DEBTMAP_CONFIG` environment variable for custom config path
- Config files are optional (tool works without them)

#### Multi-Source Loading
- Load configuration from multiple sources with layered precedence:
  1. Built-in defaults (lowest priority)
  2. User config (`~/.config/debtmap/config.toml`)
  3. Project config (`.debtmap.toml`)
  4. Environment variables (`DEBTMAP_*`)
  5. CLI arguments (highest priority)
- Later sources override earlier sources
- Support partial configs (only specify what you want to change)

#### Error Accumulation
- Collect ALL config errors before reporting
- Show source location for each error (which file, which line)
- Provide actionable error messages
- Group errors by source when displaying

#### Source Tracking
- Track origin of each configuration value
- Support `--show-config-sources` flag to display value origins
- Enable debugging of unexpected config behavior
- Identify overridden vs default values

#### Validation
- Validate complexity thresholds (positive numbers)
- Validate coverage thresholds (0-100)
- Validate file paths exist (for coverage data paths)
- Validate regex patterns (exclusion patterns)
- Accumulate ALL validation errors

### Non-Functional Requirements
- No performance regression on startup
- Backwards compatible (existing workflows unchanged)
- Clear migration path for users
- Minimal binary size increase (<200KB)

## Acceptance Criteria

- [ ] `premortem = "0.3"` added to Cargo.toml with `toml` and `derive` features
- [ ] `DebtmapConfig` struct defined with serde and premortem derives
- [ ] Configuration loads from `.debtmap.toml` when present
- [ ] Environment variables override file config (`DEBTMAP_COMPLEXITY_THRESHOLD`, etc.)
- [ ] CLI arguments override all other sources
- [ ] All config errors shown in single run (error accumulation)
- [ ] `--show-config-sources` flag shows where each value came from
- [ ] Existing CLI-only workflow unchanged
- [ ] Documentation explains config file format
- [ ] Integration tests verify multi-source loading
- [ ] Migration guide for users adding config files

## Technical Details

### Implementation Approach

#### 1. Add Premortem Dependency

```toml
# Cargo.toml
[dependencies]
premortem = { version = "0.3", features = ["toml", "derive"] }
```

#### 2. Define Configuration Schema

```rust
// src/config/schema.rs
use premortem::Validate;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Debtmap configuration loaded from multiple sources
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(default)]
pub struct DebtmapConfig {
    /// Analysis settings
    pub analysis: AnalysisConfig,

    /// Output settings
    pub output: OutputConfig,

    /// Threshold settings
    pub thresholds: ThresholdsConfig,

    /// Exclusion patterns
    pub exclusions: ExclusionsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(default)]
pub struct AnalysisConfig {
    /// Include test files in analysis
    pub include_tests: bool,

    /// Maximum file size to analyze (bytes)
    #[validate(range(1..=100_000_000))]
    pub max_file_size: usize,

    /// Enable parallel processing
    pub parallel: bool,

    /// Number of threads (0 = auto)
    pub threads: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(default)]
pub struct ThresholdsConfig {
    /// Cyclomatic complexity threshold
    #[validate(range(1..=1000))]
    pub complexity: u32,

    /// Cognitive complexity threshold
    #[validate(range(1..=500))]
    pub cognitive_complexity: u32,

    /// Minimum coverage percentage
    #[validate(range(0.0..=100.0))]
    pub coverage: f64,

    /// Maximum nesting depth
    #[validate(range(1..=20))]
    pub nesting_depth: u32,

    /// Maximum function length (lines)
    #[validate(range(1..=1000))]
    pub function_length: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(default)]
pub struct OutputConfig {
    /// Default output format
    pub format: String,

    /// JSON output structure (legacy/unified)
    pub json_format: String,

    /// Enable colored output
    pub color: bool,

    /// Verbosity level (0-3)
    #[validate(range(0..=3))]
    pub verbosity: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ExclusionsConfig {
    /// File patterns to exclude
    pub patterns: Vec<String>,

    /// Directories to exclude
    pub directories: Vec<String>,
}

impl Default for DebtmapConfig {
    fn default() -> Self {
        Self {
            analysis: AnalysisConfig::default(),
            output: OutputConfig::default(),
            thresholds: ThresholdsConfig::default(),
            exclusions: ExclusionsConfig::default(),
        }
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            include_tests: false,
            max_file_size: 1_000_000,
            parallel: true,
            threads: 0,
        }
    }
}

impl Default for ThresholdsConfig {
    fn default() -> Self {
        Self {
            complexity: 10,
            cognitive_complexity: 15,
            coverage: 80.0,
            nesting_depth: 4,
            function_length: 50,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: "terminal".to_string(),
            json_format: "legacy".to_string(),
            color: true,
            verbosity: 1,
        }
    }
}
```

#### 3. Configuration Loader

```rust
// src/config/loader.rs
use premortem::{Config, ConfigBuilder, ConfigErrors, Toml, Env, Defaults};
use std::path::PathBuf;

/// Load configuration from all sources
pub fn load_config() -> Result<Config<DebtmapConfig>, ConfigErrors> {
    let mut builder = Config::<DebtmapConfig>::builder()
        // 1. Built-in defaults (lowest priority)
        .source(Defaults::from(DebtmapConfig::default()));

    // 2. User config (~/.config/debtmap/config.toml)
    if let Some(user_config) = user_config_path() {
        builder = builder.source(Toml::file(user_config).optional());
    }

    // 3. Project config (.debtmap.toml)
    builder = builder.source(Toml::file(".debtmap.toml").optional());

    // 4. Custom config from DEBTMAP_CONFIG env var
    if let Ok(custom_path) = std::env::var("DEBTMAP_CONFIG") {
        builder = builder.source(Toml::file(custom_path));
    }

    // 5. Environment variables (DEBTMAP_*)
    builder = builder.source(Env::prefix("DEBTMAP_").separator("__"));

    builder.build()
}

/// Load config with traced source information
pub fn load_config_traced() -> Result<TracedConfig<DebtmapConfig>, ConfigErrors> {
    let mut builder = Config::<DebtmapConfig>::builder()
        .source(Defaults::from(DebtmapConfig::default()));

    if let Some(user_config) = user_config_path() {
        builder = builder.source(Toml::file(user_config).optional());
    }

    builder = builder
        .source(Toml::file(".debtmap.toml").optional())
        .source(Env::prefix("DEBTMAP_").separator("__"));

    builder.build_traced()
}

fn user_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("debtmap/config.toml"))
}
```

#### 4. CLI Integration

```rust
// src/cli.rs modifications
use crate::config::{load_config, load_config_traced, DebtmapConfig};

#[derive(Parser, Debug)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Show where each config value came from
    #[arg(long, global = true)]
    pub show_config_sources: bool,

    /// Config file path (overrides default locations)
    #[arg(long, global = true, env = "DEBTMAP_CONFIG")]
    pub config: Option<PathBuf>,
}

impl Cli {
    /// Load configuration with CLI overrides
    pub fn load_config(&self) -> Result<DebtmapConfig, ConfigErrors> {
        let mut config = load_config()?.into_inner();

        // Apply CLI overrides (handled per-command)
        // CLI args take highest priority

        Ok(config)
    }

    /// Show config sources if requested
    pub fn maybe_show_sources(&self) -> Result<(), ConfigErrors> {
        if self.show_config_sources {
            let traced = load_config_traced()?;

            println!("Configuration sources:");
            println!();

            for path in traced.all_paths() {
                if let Some(trace) = traced.trace(path) {
                    let source = &trace.final_value.source;
                    let value = &trace.final_value.value;
                    println!("  {} = {:?}", path, value);
                    println!("    from: {}", source);

                    if trace.was_overridden() {
                        println!("    (overrode {} other source(s))",
                            trace.previous_values.len());
                    }
                    println!();
                }
            }
        }
        Ok(())
    }
}
```

#### 5. Error Reporting

```rust
// src/config/errors.rs
use premortem::ConfigErrors;
use colored::Colorize;

/// Pretty-print configuration errors
pub fn print_config_errors(errors: &ConfigErrors) {
    eprintln!("\n{} Configuration errors:\n", "Error:".red().bold());

    // Group by source
    let grouped = errors.group_by_source();

    for (source, source_errors) in grouped {
        eprintln!("  {}:", source.cyan());

        for (i, error) in source_errors.iter().enumerate() {
            eprintln!("    {}. {}", (i + 1).to_string().yellow(), error);

            if let Some(location) = error.location() {
                eprintln!("       at line {}, column {}",
                    location.line, location.column);
            }
        }
        eprintln!();
    }

    eprintln!(
        "{} Fix the configuration errors above and run again.",
        "Tip:".cyan().bold()
    );
}
```

#### 6. Example Config File

```toml
# .debtmap.toml - Debtmap configuration file
# All settings are optional - only specify what you want to change

[analysis]
include_tests = false
max_file_size = 1000000
parallel = true
threads = 0  # 0 = auto-detect

[thresholds]
complexity = 10
cognitive_complexity = 15
coverage = 80.0
nesting_depth = 4
function_length = 50

[output]
format = "terminal"  # terminal, json, yaml, markdown
json_format = "unified"
color = true
verbosity = 1  # 0=quiet, 1=normal, 2=verbose, 3=debug

[exclusions]
patterns = [
    "**/node_modules/**",
    "**/target/**",
    "**/*.generated.rs",
]
directories = [
    ".git",
    "vendor",
    "build",
]
```

### Integration with Environment Trait (Spec 195/199)

```rust
// src/env.rs - Updated to use premortem config
use premortem::Config;
use crate::config::DebtmapConfig;

pub trait AnalysisEnv: Clone + Send + Sync {
    fn config(&self) -> &DebtmapConfig;
    fn file_system(&self) -> &dyn FileSystem;
    fn coverage_loader(&self) -> &dyn CoverageLoader;
}

#[derive(Clone)]
pub struct RealEnv {
    config: Config<DebtmapConfig>,
    file_system: Arc<dyn FileSystem>,
    coverage_loader: Arc<dyn CoverageLoader>,
}

impl RealEnv {
    pub fn new(config: Config<DebtmapConfig>) -> Self {
        Self {
            config,
            file_system: Arc::new(RealFileSystem::default()),
            coverage_loader: Arc::new(RealCoverageLoader::default()),
        }
    }
}

impl AnalysisEnv for RealEnv {
    fn config(&self) -> &DebtmapConfig {
        &self.config
    }

    fn file_system(&self) -> &dyn FileSystem {
        &*self.file_system
    }

    fn coverage_loader(&self) -> &dyn CoverageLoader {
        &*self.coverage_loader
    }
}
```

### Testing with MockEnv (Spec 200)

```rust
// Tests can use premortem's MockEnv for config testing
#[cfg(test)]
mod tests {
    use premortem::testing::MockEnv;

    #[test]
    fn test_config_from_file() {
        let env = MockEnv::new()
            .with_file(".debtmap.toml", r#"
                [thresholds]
                complexity = 20
            "#);

        let config = Config::<DebtmapConfig>::builder()
            .source(Defaults::from(DebtmapConfig::default()))
            .source(Toml::file(".debtmap.toml"))
            .build_with_env(&env)
            .unwrap();

        assert_eq!(config.thresholds.complexity, 20);
    }

    #[test]
    fn test_env_overrides_file() {
        let env = MockEnv::new()
            .with_file(".debtmap.toml", r#"
                [thresholds]
                complexity = 20
            "#)
            .with_env("DEBTMAP_THRESHOLDS__COMPLEXITY", "30");

        let config = Config::<DebtmapConfig>::builder()
            .source(Defaults::from(DebtmapConfig::default()))
            .source(Toml::file(".debtmap.toml"))
            .source(Env::prefix("DEBTMAP_").separator("__"))
            .build_with_env(&env)
            .unwrap();

        assert_eq!(config.thresholds.complexity, 30);
    }

    #[test]
    fn test_validation_accumulates_errors() {
        let env = MockEnv::new()
            .with_file(".debtmap.toml", r#"
                [thresholds]
                complexity = -5
                coverage = 150.0
                nesting_depth = 0
            "#);

        let result = Config::<DebtmapConfig>::builder()
            .source(Toml::file(".debtmap.toml"))
            .build_with_env(&env);

        match result {
            Err(errors) => {
                // All 3 errors collected, not just first
                assert_eq!(errors.len(), 3);
            }
            Ok(_) => panic!("Expected validation failure"),
        }
    }
}
```

## Dependencies

- **Prerequisites**:
  - Spec 195 (Stillwater Foundation) - Provides environment trait structure
  - Spec 197 (Validation Error Accumulation) - Aligns with error accumulation approach
  - Spec 199 (Reader Pattern) - Config accessible via `env.config()`
- **Blocked by**: None (can implement after 195, 197, 199)
- **Blocks**: None
- **Affected Components**:
  - `src/main.rs` - Replace DefaultConfigProvider
  - `src/cli.rs` - Add config loading and source display
  - `src/config/` - New module for config schema and loading
  - `src/env.rs` - Use DebtmapConfig in AnalysisEnv
- **External Dependencies**:
  - premortem crate v0.3.0 from crates.io

## Testing Strategy

- **Unit Tests**:
  - Config schema validation
  - Default value correctness
  - Validation error accumulation
  - Source precedence ordering

- **Integration Tests**:
  - Load from `.debtmap.toml`
  - Load from environment variables
  - Combined file + env loading
  - CLI override behavior
  - Error reporting format

- **User Experience Tests**:
  - Run without config file (should work)
  - Run with invalid config (all errors shown)
  - Run with `--show-config-sources`

## Documentation Requirements

- **Code Documentation**:
  - Document DebtmapConfig fields
  - Explain source precedence
  - Show validation rules

- **User Documentation**:
  - Example `.debtmap.toml` file
  - Environment variable reference
  - Migration guide from CLI-only usage
  - Troubleshooting config issues

- **Architecture Updates**:
  - Update ARCHITECTURE.md with config loading flow
  - Document integration with AnalysisEnv

## Implementation Notes

### Design Decisions

1. **Why premortem vs manual config loading?**
   - Error accumulation matches spec 197's goals
   - Source tracking aids debugging
   - Validation derives reduce boilerplate
   - Shared stillwater foundation with specs 195-200

2. **Why optional config files?**
   - Backwards compatibility with CLI-only workflow
   - Gradual adoption for existing users
   - CI/CD environments may prefer env vars only

3. **Why nested config structure?**
   - Logical grouping of related settings
   - Matches TOML sections naturally
   - Easier to extend in future

### Potential Issues

1. **Config File Discovery**
   - Risk: Finding config in wrong directory
   - Mitigation: Clear precedence rules, `--show-config-sources` debugging

2. **Environment Variable Naming**
   - Risk: Complex nested paths (`DEBTMAP_THRESHOLDS__COMPLEXITY`)
   - Mitigation: Document clearly, provide examples

3. **Validation Strictness**
   - Risk: Breaking existing workflows with stricter validation
   - Mitigation: Match current CLI validation, warn before error

### Files to Create/Modify

**New Files:**
- `src/config/mod.rs`
- `src/config/schema.rs`
- `src/config/loader.rs`
- `src/config/errors.rs`

**Modified Files:**
- `Cargo.toml` - Add premortem dependency
- `src/main.rs` - Replace DefaultConfigProvider
- `src/cli.rs` - Add config loading
- `src/lib.rs` - Export config module
- `src/env.rs` - Use DebtmapConfig

## Migration and Compatibility

### Non-Breaking Changes

- Config file is optional
- CLI arguments work exactly as before
- No changes to output format
- No changes to analysis behavior

### Migration Path

1. **No action required** - Existing workflows continue to work
2. **Optional adoption** - Create `.debtmap.toml` to persist settings
3. **Full adoption** - Use config file + env vars for CI/CD

### Example Migration

**Before (CLI only):**
```bash
debtmap analyze src/ --threshold-complexity 15 --format json
```

**After (config file):**
```toml
# .debtmap.toml
[thresholds]
complexity = 15

[output]
format = "json"
```

```bash
debtmap analyze src/  # Uses config file
```

## Success Metrics

- **Functionality**: All config sources load correctly
- **Error Handling**: All config errors shown in single run
- **Backwards Compat**: All existing tests pass
- **User Experience**: Clear error messages with source locations
- **Performance**: <10ms startup overhead from config loading

## Future Considerations

After this spec, configuration features can be extended:

- **Hot reload** - Watch config file for changes (premortem supports this)
- **Config generation** - `debtmap init` to create `.debtmap.toml`
- **Config validation** - `debtmap config check` to validate without running
- **Schema export** - Generate JSON schema for IDE support
