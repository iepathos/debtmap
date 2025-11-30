---
number: 185
title: Remove/Gate Debug Print Statements
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 185: Remove/Gate Debug Print Statements

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

An audit of the debtmap codebase revealed **464 debug print statements** (using `println!`, `eprintln!`, `dbg!`, etc.) scattered throughout the code. These statements create several problems:

1. **Performance**: Unnecessary I/O in hot paths slows down analysis
2. **Output pollution**: Debug output mixed with production output
3. **Professionalism**: Production tools should have clean, controlled output
4. **Testability**: Print statements make output testing fragile
5. **Maintainability**: Hard to distinguish intentional output from debug leftovers

**Current State:**
```rust
// src/analyzers/rust.rs
pub fn analyze_function(&self, func: &syn::ItemFn) -> FunctionMetrics {
    println!("DEBUG: Analyzing function {}", func.sig.ident);  // Debug output
    eprintln!("Function complexity: {}", complexity);           // More debug
    dbg!(&func.sig);                                           // Even more debug

    // ... actual analysis ...
}
```

According to STILLWATER_EVALUATION.md (lines 684-687), we should:
1. **Audit all 464 print statements** to understand their purpose
2. **Remove unnecessary ones** from core analysis code
3. **Gate remaining ones** behind `#[cfg(debug_assertions)]` or feature flags
4. **Use proper logging** for production diagnostics

This will result in:
- Cleaner production output
- Better performance (no wasted I/O)
- Professional tool behavior
- Easier testing and maintenance

## Objective

Systematically audit and clean up all 464 debug print statements in the codebase:

1. **Categorize** all print statements by purpose
2. **Remove** debug statements from core analysis code
3. **Gate** development-only output behind `#[cfg(debug_assertions)]`
4. **Convert** important diagnostics to proper logging (using `log` crate)
5. **Keep** only intentional user-facing output

Result: Clean, professional output with optional debug information when needed.

## Requirements

### Functional Requirements

1. **Audit All Print Statements**
   - Find all instances of `println!`, `eprintln!`, `dbg!`, `print!`, `eprint!`
   - Categorize each by purpose:
     - Debug/development only
     - User-facing output (keep)
     - Error reporting (convert to proper error handling)
     - Progress/status (keep, ensure consistency)
     - Leftover debugging (remove)
   - Document findings in audit report

2. **Remove Unnecessary Debug Output**
   - Remove all debugging print statements from:
     - Core analysis functions (analyzers/*)
     - Metric calculation functions (complexity/*)
     - Pure business logic functions
     - Hot paths (called per-file or per-function)
   - Keep intentional user-facing output

3. **Gate Development Output**
   - Wrap development/debugging output in `#[cfg(debug_assertions)]`
   - Create `debug-output` feature flag for verbose mode
   - Example:
     ```rust
     #[cfg(debug_assertions)]
     eprintln!("[DEBUG] Processing file: {:?}", path);
     ```

4. **Convert to Proper Logging**
   - Use `log` crate for production diagnostics
   - Levels: `error!`, `warn!`, `info!`, `debug!`, `trace!`
   - Replace important prints with appropriate log levels
   - Example:
     ```rust
     log::debug!("Processing file: {:?}", path);
     log::warn!("Complexity threshold exceeded: {}", complexity);
     ```

5. **Preserve User-Facing Output**
   - Keep intentional output:
     - Progress bars (using indicatif)
     - Final results
     - User warnings
     - Help text
   - Ensure consistency in formatting

### Non-Functional Requirements

1. **Performance**
   - No print statements in hot paths
   - Debug output only when explicitly enabled
   - Negligible overhead in release builds

2. **User Experience**
   - Clean, professional output by default
   - Clear, consistent formatting
   - Optional verbose mode for debugging

3. **Maintainability**
   - Clear distinction between debug and production output
   - Easy to add new debug output (use proper logging)
   - No leftover debug statements in commits

## Acceptance Criteria

- [ ] Audit report created documenting all 464 print statements
- [ ] All debug print statements removed from core analysis code
- [ ] Remaining debug output gated behind `#[cfg(debug_assertions)]`
- [ ] `log` crate integrated for production logging
- [ ] All modules use consistent logging levels
- [ ] User-facing output clearly distinguished and documented
- [ ] `--verbose` flag added to CLI for debug output
- [ ] `debug-output` feature flag added to Cargo.toml
- [ ] Tests verify no debug output in release mode
- [ ] Documentation updated with logging guidelines
- [ ] All existing tests pass
- [ ] No clippy warnings

## Technical Details

### Implementation Approach

**Phase 1: Audit (Analysis)**

```bash
# Find all print statements
rg 'println!|eprintln!|dbg!|print!|eprint!' --type rust > audit/print_statements.txt

# Categorize by location
rg 'println!|eprintln!|dbg!' src/analyzers/ --type rust > audit/analyzers_prints.txt
rg 'println!|eprintln!|dbg!' src/complexity/ --type rust > audit/complexity_prints.txt
rg 'println!|eprintln!|dbg!' src/commands/ --type rust > audit/commands_prints.txt
```

Create audit report categorizing each statement:

```markdown
# Print Statement Audit Report

## Summary
- Total statements: 464
- Debug/development: 312 (remove)
- User-facing: 52 (keep)
- Error reporting: 45 (convert to proper errors)
- Progress/status: 35 (keep)
- Leftover debugging: 20 (remove immediately)

## By Module

### src/analyzers/ (158 statements)
- `println!("DEBUG: ...")` - 89 occurrences - REMOVE
- `eprintln!("Processing ...")` - 34 occurrences - GATE
- `dbg!(...)` - 35 occurrences - REMOVE

### src/complexity/ (92 statements)
...
```

**Phase 2: Remove Debug Output**

```rust
// BEFORE: Debug output in hot path
pub fn calculate_complexity(ast: &Ast) -> u32 {
    println!("DEBUG: Calculating complexity for {} nodes", ast.nodes.len());

    let complexity = ast.functions()
        .map(|func| {
            let c = func.complexity();
            eprintln!("Function {} has complexity {}", func.name, c);
            c
        })
        .sum();

    dbg!(complexity);
    complexity
}

// AFTER: Clean, no debug output
pub fn calculate_complexity(ast: &Ast) -> u32 {
    ast.functions()
        .map(|func| func.complexity())
        .sum()
}
```

**Phase 3: Gate Development Output**

```rust
// Development-only output (compile-time gating)
#[cfg(debug_assertions)]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        eprintln!("[DEBUG] {}", format!($($arg)*))
    };
}

#[cfg(not(debug_assertions))]
macro_rules! debug_print {
    ($($arg:tt)*) => {};
}

// Usage
pub fn analyze_file(path: &Path) -> Result<FileMetrics> {
    debug_print!("Analyzing file: {:?}", path);
    // ... analysis ...
}
```

**Phase 4: Proper Logging Integration**

Add `log` and `env_logger` to Cargo.toml:

```toml
[dependencies]
log = "0.4"
env_logger = "0.11"

[features]
debug-output = []
```

Initialize logging in main.rs:

```rust
use log::{debug, error, info, trace, warn};

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info")  // info level by default
    ).init();

    info!("Debtmap starting");

    // ... rest of main
}
```

Convert important prints to logging:

```rust
// BEFORE
eprintln!("[WARN] Processing limited to {} files", max_files);

// AFTER
warn!("Processing limited to {} files (set DEBTMAP_MAX_FILES=0 for all)", max_files);
```

**Phase 5: Feature-Gated Verbose Output**

```rust
// In analysis code
#[cfg(feature = "debug-output")]
macro_rules! verbose {
    ($($arg:tt)*) => {
        eprintln!("[VERBOSE] {}", format!($($arg)*))
    };
}

#[cfg(not(feature = "debug-output"))]
macro_rules! verbose {
    ($($arg:tt)*) => {};
}

// Usage
pub fn analyze_complexity(ast: &Ast) -> u32 {
    verbose!("Analyzing {} functions", ast.functions().count());

    let complexity = ast.functions()
        .map(|f| {
            let c = f.complexity();
            verbose!("Function {}: complexity = {}", f.name, c);
            c
        })
        .sum();

    verbose!("Total complexity: {}", complexity);
    complexity
}
```

**Phase 6: CLI Integration**

```rust
// In main.rs CLI definition
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose debug output
    #[arg(long, short = 'v', global = true)]
    verbose: bool,
}

// Initialize logging based on verbosity
fn setup_logging(verbose: bool) {
    let level = if verbose {
        "debug"
    } else {
        "info"
    };

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(level)
    ).init();
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    // ... rest of main
}
```

### Categorization Guidelines

**Remove (Debug/Development):**
```rust
println!("DEBUG: ...");
eprintln!("HERE");
dbg!(variable);
println!("Function called: {}", name);
eprintln!("Value: {:?}", value);
```

**Gate with #[cfg(debug_assertions)]:**
```rust
eprintln!("Processing file: {:?}", path);  // Development info
println!("Complexity: {}", c);             // Internal diagnostic
```

**Convert to Logging:**
```rust
eprintln!("[WARN] ...");      → warn!("...");
eprintln!("[ERROR] ...");     → error!("...");
eprintln!("[INFO] ...");      → info!("...");
println!("Status: ...");      → info!("Status: ...");
```

**Keep (User-Facing):**
```rust
println!("{}", formatted_results);              // Final output
eprintln!("Error: {}", error);                  // Error reporting
println!("Analyzing {} files...", count);       // Status message
// Progress bars (using indicatif)
```

### Architecture Changes

**Before:**
```
Core analysis code
  ├─ println!("DEBUG: ...") everywhere
  ├─ eprintln!("Processing ...") in hot paths
  ├─ dbg!(...) for debugging
  └─ Mixed output (debug + production)
```

**After:**
```
Core analysis code (clean)
  ├─ No print statements in hot paths
  ├─ log::debug!() for diagnostics (disabled by default)
  ├─ #[cfg(debug_assertions)] for dev-only output
  └─ Clear separation: production vs debug

Output layer
  ├─ Intentional user-facing output (CLI)
  ├─ Progress tracking (indicatif)
  ├─ Results formatting (formatters)
  └─ Error reporting (proper error types)
```

### Logging Guidelines

```rust
// For contributors - when to use each level

// error! - Fatal problems that prevent analysis
log::error!("Failed to parse file {}: {}", path.display(), err);

// warn! - Non-fatal issues that user should know about
log::warn!("Skipping unreadable file: {}", path.display());
log::warn!("Complexity threshold exceeded: {} > {}", actual, threshold);

// info! - High-level progress and status
log::info!("Analyzing {} files", count);
log::info!("Analysis complete: found {} issues", issues.len());

// debug! - Detailed diagnostic information (not shown by default)
log::debug!("Processing function: {}", func_name);
log::debug!("Calculated complexity: {}", complexity);

// trace! - Very detailed information (for deep debugging)
log::trace!("Visiting AST node: {:?}", node);
log::trace!("Intermediate result: {:?}", result);
```

### Data Structures

No new data structures needed. Configuration for logging:

```rust
pub struct LogConfig {
    pub level: log::LevelFilter,
    pub format: LogFormat,
    pub output: LogOutput,
}

pub enum LogFormat {
    Plain,      // Simple text
    Json,       // Structured JSON
    Pretty,     // Colored, formatted
}

pub enum LogOutput {
    Stderr,
    File(PathBuf),
    Both,
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - All source files with print statements (audit to identify)
  - `src/main.rs` - Initialize logging
  - `Cargo.toml` - Add logging dependencies
  - Tests - May need to capture output differently
- **External Dependencies**:
  - `log` - Logging facade
  - `env_logger` - Simple logger implementation

## Testing Strategy

### Audit Tests

```rust
#[test]
fn test_no_println_in_core_modules() {
    // Ensure core modules don't have print statements
    let files = ["src/analyzers/", "src/complexity/", "src/analysis/"];

    for dir in files {
        let output = Command::new("rg")
            .args(&["println!|eprintln!|dbg!", dir, "--type", "rust"])
            .output()
            .expect("Failed to run grep");

        if !output.stdout.is_empty() {
            let violations = String::from_utf8_lossy(&output.stdout);
            panic!(
                "Found print statements in {}: \n{}",
                dir, violations
            );
        }
    }
}
```

### Output Tests

```rust
#[test]
fn test_clean_output_in_release() {
    // Test that analysis produces no debug output
    let result = analyze_file("tests/fixtures/simple.rs");

    // Should succeed without printing
    assert!(result.is_ok());

    // If we captured stderr, it should be empty in release mode
    #[cfg(not(debug_assertions))]
    {
        // No debug output in release
    }
}

#[test]
fn test_logging_levels() {
    // Test that logging respects levels
    testing_logger::setup();

    log::debug!("Debug message");
    log::info!("Info message");
    log::warn!("Warning message");

    // Verify appropriate messages logged at each level
    testing_logger::validate(|captured_logs| {
        assert!(captured_logs
            .iter()
            .any(|log| log.level == log::Level::Info));
    });
}
```

### Feature Gate Tests

```rust
#[test]
#[cfg(feature = "debug-output")]
fn test_verbose_output_enabled() {
    // When debug-output feature enabled, should see verbose output
    // Test implementation depends on how verbose output is captured
}

#[test]
#[cfg(not(feature = "debug-output"))]
fn test_verbose_output_disabled() {
    // Without debug-output feature, should be no verbose output
}
```

## Documentation Requirements

### Code Documentation

Create `LOGGING.md` guide:

```markdown
# Logging Guidelines

## Overview

Debtmap uses the `log` crate for production logging. Debug output is gated
behind compile-time flags.

## Usage

### For Users

Control log level with `RUST_LOG` environment variable:

```bash
# Info level (default)
debtmap analyze src/

# Debug level (verbose)
RUST_LOG=debug debtmap analyze src/

# Trace level (very verbose)
RUST_LOG=trace debtmap analyze src/

# Quiet (errors only)
RUST_LOG=error debtmap analyze src/
```

### For Developers

Use appropriate log levels:

```rust
use log::{debug, error, info, trace, warn};

// Fatal errors
error!("Cannot proceed: {}", reason);

// Warnings
warn!("Skipping invalid file: {}", path);

// Progress
info!("Processing {} files", count);

// Diagnostics
debug!("Calculated complexity: {}", complexity);

// Deep debugging
trace!("AST node: {:?}", node);
```

### Debug Assertions

Use `debug_print!` macro for development-only output:

```rust
debug_print!("This only appears in debug builds");
```

### Feature Flags

Build with `debug-output` feature for verbose mode:

```bash
cargo build --features debug-output
```

## Testing

Never use print statements in production code. For testing:

```rust
#[test]
fn test_something() {
    // Use testing_logger crate for log testing
    testing_logger::setup();

    my_function();

    testing_logger::validate(|logs| {
        assert!(logs.iter().any(|log| log.body.contains("expected message")));
    });
}
```
```

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Output and Logging

Debtmap maintains clean separation between production output and debug information:

### Production Output
- **User-facing results**: Formatted analysis output (JSON, YAML, text)
- **Progress tracking**: Using `indicatif` progress bars
- **Errors**: Proper error types with context
- **Warnings**: Important user-actionable warnings

### Debug Output
- **Logging**: Using `log` crate with configurable levels
- **Debug assertions**: Development-only output in debug builds
- **Feature flags**: Optional verbose mode with `debug-output` feature

### Guidelines
- ❌ Never use `println!`/`eprintln!` in core analysis code
- ✅ Use `log::debug!()` for diagnostics
- ✅ Use `debug_print!()` macro for development-only output
- ✅ Use proper error types instead of printing errors
- ✅ Use progress bars for user feedback
```

## Implementation Notes

### Refactoring Steps

1. **Run audit**
   - Find all print statements
   - Categorize each occurrence
   - Document in audit report

2. **Remove obvious debug output**
   - Remove `println!("DEBUG: ...")`
   - Remove `dbg!(...)`
   - Remove leftover debugging prints

3. **Add logging infrastructure**
   - Add `log` and `env_logger` dependencies
   - Initialize logging in main.rs
   - Create logging macros

4. **Convert important prints**
   - User warnings → `warn!()`
   - Status messages → `info!()`
   - Diagnostics → `debug!()`

5. **Gate development output**
   - Add `#[cfg(debug_assertions)]`
   - Create `debug_print!` macro
   - Add `debug-output` feature

6. **Test and verify**
   - Run tests
   - Check output in release mode
   - Verify no print statements remain

7. **Document**
   - Create logging guidelines
   - Update architecture docs
   - Add examples

### Common Pitfalls

1. **Missing prints** - Use grep thoroughly
2. **Accidental removal** - Don't remove user-facing output
3. **Performance** - Ensure logging has minimal overhead
4. **Testing** - Update tests that expect output

### Search Patterns

```bash
# Find all variations
rg 'println!' --type rust
rg 'eprintln!' --type rust
rg 'print!' --type rust
rg 'eprint!' --type rust
rg 'dbg!' --type rust
rg 'std::io::stdout' --type rust
rg 'std::io::stderr' --type rust
```

## Migration and Compatibility

### Breaking Changes

**None** - This is an internal cleanup. User-facing output remains the same.

### Migration Steps

For developers:
1. Replace debug prints with `log::debug!()`
2. Use `debug_print!()` macro for development-only output
3. Use `--verbose` flag for debug output

### Compatibility Considerations

- All user-facing output preserved
- Same error messages
- Progress bars unchanged
- New: Optional verbose mode

## Success Metrics

- ✅ Audit report completed (all 464 statements categorized)
- ✅ Zero print statements in core analysis code
- ✅ All debug output gated appropriately
- ✅ Logging system integrated
- ✅ Clean output in release builds
- ✅ Verbose mode available for debugging
- ✅ Documentation complete
- ✅ All tests pass

## Follow-up Work

After this implementation:
- Add structured logging (JSON output)
- Create log analysis tools
- Add performance logging for hot paths
- Consider tracing integration for advanced profiling

## References

- **STILLWATER_EVALUATION.md** - Lines 684-687 (Debug statement recommendation)
- **log crate** - https://docs.rs/log/
- **env_logger crate** - https://docs.rs/env_logger/
- **CLAUDE.md** - Pure function guidelines (no side effects)
