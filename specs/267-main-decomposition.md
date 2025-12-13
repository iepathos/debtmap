---
number: 267
title: main.rs Decomposition
category: maintainability
priority: medium
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 267: main.rs Decomposition

**Category**: maintainability
**Priority**: medium
**Status**: draft
**Dependencies**: None (can proceed independently)

## Context

`main.rs` has grown to 2,079 lines, mixing multiple concerns:

1. **CLI argument parsing** - Clap definitions and handling
2. **Command handlers** - `handle_analyze_command()`, `handle_compare_command()`, etc.
3. **DI implementations** - `DefaultConfigProvider`, `DefaultResourceResolver`
4. **Configuration building** - Converting CLI args to config structs
5. **Setup logic** - Thread pool initialization, logging setup

**Current Problems:**

```rust
// main.rs mixes all these concerns in one file:

// 1. CLI definitions (lines 1-200)
#[derive(Parser)]
struct Cli { ... }

// 2. DI implementations (lines 200-400)
struct DefaultConfigProvider { ... }
impl ConfigProvider for DefaultConfigProvider { ... }

// 3. Command handlers (lines 400-1500)
fn handle_analyze_command(...) -> Result<()> {
    // 500+ lines mixing config, analysis, output
}

// 4. Entry point (lines 1500-2079)
fn main() -> Result<()> {
    // Setup, routing, error handling
}
```

**Stillwater Philosophy:**

> "Composition Over Complexity" - Each module should have a single, clear purpose.

## Objective

Decompose `main.rs` into focused modules:

1. **CLI module** - Argument parsing and command routing
2. **Command handlers** - One file per command
3. **DI module** - Dependency injection implementations
4. **Config builders** - CLI-to-config conversion helpers

Result: `main.rs` becomes a thin entry point (~200 lines).

## Requirements

### Functional Requirements

1. **CLI Module Extraction**
   - `src/cli/mod.rs` - Clap structs and enums
   - `src/cli/args.rs` - Argument definitions
   - `src/cli/setup.rs` - Thread pool, logging initialization

2. **Command Handler Extraction**
   - `src/cli/commands/analyze.rs` - Analysis command
   - `src/cli/commands/validate.rs` - Validation command
   - `src/cli/commands/compare.rs` - Comparison command
   - `src/cli/commands/mod.rs` - Command dispatcher

3. **DI Module Extraction**
   - `src/di/mod.rs` - Trait re-exports
   - `src/di/default_implementations.rs` - Production DI implementations

4. **Config Builder Extraction**
   - `src/cli/config_builder.rs` - CLI args → config conversion

5. **Thin Entry Point**
   - `main.rs` - Only entry point, panic handler, basic routing

### Non-Functional Requirements

1. **File Size Limits**
   - Each file under 400 lines
   - main.rs under 200 lines

2. **Clear Separation**
   - Each module has single responsibility
   - No circular dependencies

3. **Testability**
   - Command handlers testable in isolation
   - Config builders unit testable

## Acceptance Criteria

- [ ] main.rs under 200 lines
- [ ] Each command handler in separate file
- [ ] DI implementations isolated in `src/di/`
- [ ] Config builders in `src/cli/config_builder.rs`
- [ ] No circular dependencies
- [ ] All existing tests pass
- [ ] CLI behavior unchanged
- [ ] No clippy warnings

## Technical Details

### Target Module Structure

```
src/
├── main.rs                       (~150 lines)  - Entry point only
├── cli/
│   ├── mod.rs                    (~50 lines)   - Re-exports
│   ├── args.rs                   (~300 lines)  - Clap definitions
│   ├── setup.rs                  (~150 lines)  - Thread pool, logging
│   ├── config_builder.rs         (~200 lines)  - CLI → Config conversion
│   └── commands/
│       ├── mod.rs                (~50 lines)   - Command dispatcher
│       ├── analyze.rs            (~400 lines)  - Analyze command
│       ├── validate.rs           (~200 lines)  - Validate command
│       └── compare.rs            (~200 lines)  - Compare command
└── di/
    ├── mod.rs                    (~30 lines)   - Re-exports
    └── default_implementations.rs (~200 lines) - DI implementations
```

### Implementation Approach

**Phase 1: Extract CLI Args**

```rust
// src/cli/args.rs

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Debtmap - Code complexity and technical debt analyzer
#[derive(Parser)]
#[command(name = "debtmap")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress progress output
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze a codebase for technical debt
    Analyze(AnalyzeArgs),

    /// Validate debtmap configuration
    Validate(ValidateArgs),

    /// Compare two analysis results
    Compare(CompareArgs),
}

#[derive(Parser)]
pub struct AnalyzeArgs {
    /// Path to analyze
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    // ... other arguments
}

#[derive(ValueEnum, Clone, Copy)]
pub enum OutputFormat {
    Text,
    Json,
    Sarif,
    Html,
}
```

**Phase 2: Extract DI Implementations**

```rust
// src/di/default_implementations.rs

use crate::config::DebtmapConfig;
use crate::di::{ConfigProvider, ResourceResolver};
use std::collections::HashMap;
use std::sync::RwLock;

/// Default configuration provider using RwLock for thread safety
pub struct DefaultConfigProvider {
    config: RwLock<HashMap<String, String>>,
}

impl DefaultConfigProvider {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_config(initial: HashMap<String, String>) -> Self {
        Self {
            config: RwLock::new(initial),
        }
    }
}

impl ConfigProvider for DefaultConfigProvider {
    fn get(&self, key: &str) -> Option<String> {
        // Use .ok() to handle poisoned lock gracefully
        self.config
            .read()
            .ok()
            .and_then(|config| config.get(key).cloned())
    }

    fn set(&self, key: &str, value: String) {
        if let Ok(mut config) = self.config.write() {
            config.insert(key.to_string(), value);
        }
    }
}

/// Default resource resolver for file system access
pub struct DefaultResourceResolver {
    root: PathBuf,
}

impl DefaultResourceResolver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl ResourceResolver for DefaultResourceResolver {
    fn resolve(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    fn exists(&self, path: &str) -> bool {
        self.resolve(path).exists()
    }
}
```

**Phase 3: Extract Command Handlers**

```rust
// src/cli/commands/analyze.rs

use crate::cli::args::AnalyzeArgs;
use crate::cli::config_builder::build_analysis_config;
use crate::builders::unified_analysis::perform_unified_analysis;
use crate::env::RealEnv;
use crate::io::formatters::{format_output, OutputFormat};
use anyhow::Result;

/// Handle the analyze command
pub fn handle_analyze(args: AnalyzeArgs, verbose: bool) -> Result<()> {
    // Build configuration from CLI args
    let config = build_analysis_config(&args)?;

    // Create environment
    let env = RealEnv::new(config.clone());

    // Perform analysis
    let result = perform_unified_analysis(&env, &config)?;

    // Format and output results
    let output = format_output(&result, args.format)?;

    match args.output {
        Some(path) => std::fs::write(path, output)?,
        None => println!("{}", output),
    }

    Ok(())
}
```

```rust
// src/cli/commands/mod.rs

mod analyze;
mod compare;
mod validate;

pub use analyze::handle_analyze;
pub use compare::handle_compare;
pub use validate::handle_validate;

use crate::cli::args::Commands;
use anyhow::Result;

/// Dispatch to appropriate command handler
pub fn dispatch(command: Commands, verbose: bool) -> Result<()> {
    match command {
        Commands::Analyze(args) => handle_analyze(args, verbose),
        Commands::Validate(args) => handle_validate(args, verbose),
        Commands::Compare(args) => handle_compare(args, verbose),
    }
}
```

**Phase 4: Extract Config Builder**

```rust
// src/cli/config_builder.rs

use crate::cli::args::AnalyzeArgs;
use crate::config::{DebtmapConfig, AnalysisConfig, ThresholdConfig};
use anyhow::{Context, Result};

/// Build analysis configuration from CLI arguments
pub fn build_analysis_config(args: &AnalyzeArgs) -> Result<DebtmapConfig> {
    let mut config = load_config_file(&args.path)?;

    // Override with CLI args
    if let Some(threshold) = args.complexity_threshold {
        config.thresholds.complexity = threshold;
    }

    if let Some(ref ignore) = args.ignore {
        config.ignore.patterns.extend(ignore.clone());
    }

    // Apply environment variable overrides
    apply_env_overrides(&mut config);

    Ok(config)
}

/// Load configuration from file or use defaults
fn load_config_file(path: &Path) -> Result<DebtmapConfig> {
    let config_path = path.join("debtmap.toml");

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .context("Failed to read debtmap.toml")?;
        toml::from_str(&content)
            .context("Failed to parse debtmap.toml")
    } else {
        Ok(DebtmapConfig::default())
    }
}

/// Apply environment variable overrides
fn apply_env_overrides(config: &mut DebtmapConfig) {
    if let Ok(val) = std::env::var("DEBTMAP_COMPLEXITY_THRESHOLD") {
        if let Ok(threshold) = val.parse() {
            config.thresholds.complexity = threshold;
        }
    }

    if let Ok(val) = std::env::var("DEBTMAP_PARALLEL") {
        config.parallel = val != "0" && val.to_lowercase() != "false";
    }

    // ... other overrides
}
```

**Phase 5: Extract Setup Logic**

```rust
// src/cli/setup.rs

use anyhow::Result;
use std::num::NonZeroUsize;

/// Initialize thread pool with appropriate size
pub fn init_thread_pool(parallel: bool) -> Result<()> {
    if parallel {
        let threads = std::thread::available_parallelism()
            .unwrap_or(NonZeroUsize::new(4).unwrap())
            .get();

        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()?;
    } else {
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build_global()?;
    }

    Ok(())
}

/// Initialize logging based on verbosity
pub fn init_logging(verbose: bool, quiet: bool) {
    let level = if quiet {
        log::LevelFilter::Error
    } else if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::new()
        .filter_level(level)
        .init();
}

/// Display configuration summary (when verbose)
pub fn display_config_summary(config: &DebtmapConfig) {
    log::debug!("Analysis configuration:");
    log::debug!("  Complexity threshold: {}", config.thresholds.complexity);
    log::debug!("  Parallel: {}", config.parallel);
    // ... other config
}
```

**Phase 6: Thin main.rs**

```rust
// src/main.rs

mod cli;
mod di;

use clap::Parser;
use cli::args::Cli;
use cli::commands::dispatch;
use cli::setup::{init_logging, init_thread_pool};
use anyhow::Result;

fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, cli.quiet);

    // Initialize thread pool
    init_thread_pool(true)?;

    // Set up panic handler
    setup_panic_handler();

    // Dispatch to command handler
    dispatch(cli.command, cli.verbose)
}

fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().map(|l| format!("{}:{}", l.file(), l.line()));
        let message = info.payload()
            .downcast_ref::<&str>()
            .map(|s| *s)
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("Unknown panic");

        eprintln!("Internal error: {}", message);
        if let Some(loc) = location {
            eprintln!("Location: {}", loc);
        }
        eprintln!("\nThis is a bug. Please report it at: https://github.com/debtmap/debtmap/issues");
    }));
}
```

### Migration Strategy

1. **Create module structure** - Empty files with stubs
2. **Extract CLI args** - Move Clap definitions to `src/cli/args.rs`
3. **Extract DI** - Move implementations to `src/di/`
4. **Extract one command at a time**:
   - `handle_analyze_command` → `src/cli/commands/analyze.rs`
   - `handle_validate_command` → `src/cli/commands/validate.rs`
   - `handle_compare_command` → `src/cli/commands/compare.rs`
5. **Extract config builder** - Move to `src/cli/config_builder.rs`
6. **Extract setup** - Move to `src/cli/setup.rs`
7. **Update main.rs** - Import and delegate
8. **Verify** - Run all tests, check CLI behavior

### Files to Create/Modify

1. **Create** `src/cli/mod.rs`
2. **Create** `src/cli/args.rs`
3. **Create** `src/cli/setup.rs`
4. **Create** `src/cli/config_builder.rs`
5. **Create** `src/cli/commands/mod.rs`
6. **Create** `src/cli/commands/analyze.rs`
7. **Create** `src/cli/commands/validate.rs`
8. **Create** `src/cli/commands/compare.rs`
9. **Create** `src/di/mod.rs`
10. **Create** `src/di/default_implementations.rs`
11. **Modify** `src/main.rs` - Reduce to entry point
12. **Modify** `src/lib.rs` - Export new modules

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/main.rs`
  - Integration tests that invoke CLI
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
// src/cli/config_builder.rs tests
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_build_config_defaults() {
        let args = AnalyzeArgs::default();
        let config = build_analysis_config(&args).unwrap();
        assert!(config.thresholds.complexity > 0);
    }

    #[test]
    fn test_cli_args_override_file() {
        let temp = TempDir::new().unwrap();
        std::fs::write(
            temp.path().join("debtmap.toml"),
            r#"[thresholds]
            complexity = 10"#
        ).unwrap();

        let mut args = AnalyzeArgs::default();
        args.path = temp.path().to_path_buf();
        args.complexity_threshold = Some(20);

        let config = build_analysis_config(&args).unwrap();
        assert_eq!(config.thresholds.complexity, 20);
    }
}
```

### Integration Tests

```rust
// tests/cli_integration.rs
use assert_cmd::Command;

#[test]
fn test_analyze_command_basic() {
    let mut cmd = Command::cargo_bin("debtmap").unwrap();
    cmd.arg("analyze")
        .arg("tests/fixtures/simple_project")
        .assert()
        .success();
}

#[test]
fn test_validate_command() {
    let mut cmd = Command::cargo_bin("debtmap").unwrap();
    cmd.arg("validate")
        .arg("tests/fixtures/debtmap.toml")
        .assert()
        .success();
}

#[test]
fn test_help_output() {
    let mut cmd = Command::cargo_bin("debtmap").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("analyze"));
}
```

## Documentation Requirements

### Code Documentation

- Module-level docs explaining purpose of each module
- Public function docs with usage examples
- CLI args documented via Clap attributes

### User Documentation

Ensure `--help` output remains unchanged:

```bash
$ debtmap --help
# Should show same commands and options
```

## Implementation Notes

### Preserving CLI Behavior

- All argument names and defaults unchanged
- Help text preserved via Clap attributes
- Exit codes unchanged

### Error Handling

Each command handler should return `anyhow::Result`:

```rust
fn handle_analyze(args: AnalyzeArgs, verbose: bool) -> Result<()> {
    // Errors propagate to main() which handles display
}
```

### Pitfalls to Avoid

1. **Circular imports** - CLI should not import command handlers that import CLI
2. **Breaking help text** - Keep Clap attributes during extraction
3. **Missing re-exports** - Ensure public API accessible from `src/lib.rs`

## Migration and Compatibility

### Breaking Changes

None - CLI interface unchanged.

### Backward Compatibility

- All CLI commands work identically
- Configuration file format unchanged
- Environment variable support unchanged

## Success Metrics

- main.rs under 200 lines
- Each module under 400 lines
- All CLI tests pass
- No change to user-visible behavior
- Clear module boundaries with single responsibilities
