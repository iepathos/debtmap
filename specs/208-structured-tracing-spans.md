---
number: 208
title: Structured Tracing with Spans
category: foundation
priority: high
status: draft
dependencies: [207]
created: 2025-12-14
---

# Specification 208: Structured Tracing with Spans

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 207 (Panic Hook with Crash Reports)

## Context

Debtmap currently uses 688 occurrences of `eprintln!` scattered across 137 files for logging. This approach has several problems:

1. **No structure**: Can't filter, query, or aggregate logs
2. **No context propagation**: Logs don't show call hierarchy
3. **Mixed concerns**: Logging side effects mixed with pure business logic
4. **No log levels**: Everything is printed, no way to control verbosity
5. **Hard to debug**: When something fails, no trail showing what led to it

Following Stillwater's **"Pure Core, Imperative Shell"** principle, logging should happen at effect boundaries, not inside pure computation functions.

## Objective

Replace ad-hoc `eprintln!` logging with structured tracing using the `tracing` crate:

1. **Add spans** to major analysis phases for context hierarchy
2. **Replace eprintln!** with appropriate `tracing::` macros at effect boundaries
3. **Keep pure functions pure** - no logging inside pure computation
4. **Enable filtering** via RUST_LOG environment variable
5. **Integrate with panic hook** (spec 207) for span context in crash reports

## Requirements

### Functional Requirements

1. **Phase-Level Spans**
   - Each major analysis phase wrapped in a span
   - Spans include relevant metadata (file count, config, etc.)
   - Nested spans show call hierarchy

2. **Log Level Mapping**
   - `error!` - Actual errors that affect results
   - `warn!` - Recoverable issues, degraded functionality
   - `info!` - Major milestones (phase start/complete, file counts)
   - `debug!` - Detailed progress for debugging
   - `trace!` - Very verbose, per-item logging

3. **Effect Boundary Logging**
   - Log at I/O boundaries (file read, output write)
   - Log at external calls (coverage file loading)
   - NO logging in pure computation functions

4. **Subscriber Configuration**
   - Default: minimal output (errors and warnings only)
   - `RUST_LOG=info`: Phase-level progress
   - `RUST_LOG=debug`: Detailed debugging
   - `RUST_LOG=debtmap=debug`: Module-specific filtering

### Non-Functional Requirements

1. **Zero-cost when disabled**: Unused log levels compile away
2. **Backward compatibility**: Existing behavior preserved by default
3. **Thread-safe**: Works correctly with rayon parallel iterators
4. **Minimal dependencies**: Only `tracing` and `tracing-subscriber`

## Acceptance Criteria

- [ ] `tracing` and `tracing-subscriber` added to dependencies
- [ ] Subscriber initialized at application startup
- [ ] Major analysis phases have info-level spans
- [ ] File analysis has debug-level spans
- [ ] Spans include relevant metadata (file paths, counts)
- [ ] `eprintln!` calls at effect boundaries replaced with tracing macros
- [ ] Pure functions contain no logging calls
- [ ] RUST_LOG environment variable controls verbosity
- [ ] Default output matches current behavior (minimal)
- [ ] Debug mode shows phase progression
- [ ] Span hierarchy visible in trace output
- [ ] Performance not regressed with default log level

## Technical Details

### Implementation Approach

**Phase 1: Add Dependencies**

```toml
# Cargo.toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

**Phase 2: Initialize Subscriber**

```rust
// src/observability/tracing.rs
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

/// Initialize tracing subscriber with environment filter
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(filter)
        .init();
}

// src/main.rs
fn main() -> Result<()> {
    install_panic_hook();
    init_tracing(); // Initialize tracing early

    // ... rest of main
}
```

**Phase 3: Add Phase Spans**

```rust
// src/builders/unified_analysis.rs
use tracing::{info_span, debug_span, info, debug};

pub fn perform_unified_analysis_with_options(
    project_path: &Path,
    results: &AnalysisResults,
    config: &Config,
) -> Result<UnifiedAnalysis> {
    let span = info_span!(
        "unified_analysis",
        project = %project_path.display(),
        file_count = results.file_metrics.len(),
    );
    let _guard = span.enter();

    info!("Starting unified analysis");

    // Call graph building phase
    let call_graph = {
        let _span = info_span!("call_graph_building").entered();
        info!("Building call graph");
        build_call_graph_with_progress(results, config)?
    };

    // Purity analysis phase
    let purity_results = {
        let _span = info_span!("purity_analysis").entered();
        info!("Analyzing function purity");
        analyze_purity(results, &call_graph)?
    };

    // Debt scoring phase
    let debt_items = {
        let _span = info_span!("debt_scoring").entered();
        info!("Scoring technical debt items");
        score_debt_items(results, &call_graph, &purity_results)?
    };

    info!(item_count = debt_items.len(), "Analysis complete");

    Ok(UnifiedAnalysis { call_graph, purity_results, debt_items })
}
```

**Phase 4: File-Level Spans**

```rust
// src/analyzers/rust_analyzer.rs
use tracing::{debug_span, debug, warn};

pub fn analyze_file(path: &Path, content: &str) -> Result<FileMetrics> {
    let span = debug_span!(
        "analyze_file",
        path = %path.display(),
    );
    let _guard = span.enter();

    debug!("Parsing file");

    let ast = match parse_file(content) {
        Ok(ast) => ast,
        Err(e) => {
            warn!(error = %e, "Failed to parse file, skipping");
            return Ok(FileMetrics::empty(path));
        }
    };

    debug!(functions = ast.functions().count(), "Extracted AST");

    // Pure computation - NO LOGGING HERE
    let metrics = compute_metrics(&ast);

    debug!(complexity = metrics.total_complexity, "Analysis complete");

    Ok(metrics)
}
```

**Phase 5: Replace eprintln! at Effect Boundaries**

```rust
// BEFORE - eprintln! mixed with logic
fn load_coverage(path: &Path) -> Result<CoverageData> {
    eprintln!("[COVERAGE] Loading coverage from {}", path.display());
    let content = std::fs::read_to_string(path)?;
    eprintln!("[COVERAGE] Parsing {} bytes", content.len());
    let data = parse_lcov(&content)?;
    eprintln!("[COVERAGE] Found {} records", data.records.len());
    Ok(data)
}

// AFTER - tracing at I/O boundary, pure parsing separate
fn load_coverage(path: &Path) -> Result<CoverageData> {
    let span = debug_span!("load_coverage", path = %path.display());
    let _guard = span.enter();

    debug!("Reading coverage file");
    let content = std::fs::read_to_string(path)?;

    debug!(bytes = content.len(), "Parsing coverage data");
    let data = parse_lcov(&content)?; // Pure function - no logging inside

    info!(records = data.records.len(), "Coverage data loaded");
    Ok(data)
}

// parse_lcov is PURE - no logging
fn parse_lcov(content: &str) -> Result<CoverageData> {
    // Pure parsing logic only
    // NO eprintln! or tracing calls here
}
```

**Phase 6: Integration with Panic Hook**

```rust
// src/observability/panic_hook.rs
use tracing::Span;

fn print_crash_report(info: &PanicInfo<'_>) {
    // ... header ...

    // Get current span chain
    let current_span = Span::current();
    if let Some(metadata) = current_span.metadata() {
        eprintln!("║  SPAN: {:<68} ║", metadata.name());
    }

    // ... rest of report ...
}
```

### Span Hierarchy Design

```
unified_analysis [project=/path, file_count=4231]
├── call_graph_building
│   └── process_file [path=src/main.rs]
│   └── process_file [path=src/lib.rs]
├── purity_analysis
│   └── analyze_function [name=foo]
│   └── analyze_function [name=bar]
├── debt_scoring
│   └── score_file [path=src/main.rs]
│   └── score_file [path=src/lib.rs]
└── output_generation
```

### Module-Level Guidelines

| Module | Span Level | Notes |
|--------|------------|-------|
| `main.rs` | info | Top-level spans |
| `builders/` | info | Phase spans |
| `analyzers/` | debug | File-level spans |
| `priority/` | debug | Scoring spans |
| `io/` | debug | I/O operation spans |
| `complexity/` | NONE | Pure computation |
| `debt/` | NONE | Pure computation |

### Architecture Changes

New/modified files:
- `src/observability/tracing.rs` (new) - Subscriber initialization
- `src/observability/mod.rs` - Export tracing module
- `src/main.rs` - Initialize tracing
- `src/builders/*.rs` - Add phase spans
- `src/analyzers/*.rs` - Add file spans
- `src/io/*.rs` - Replace eprintln! with tracing

Files that should NOT have tracing:
- `src/complexity/*.rs` - Pure computation
- `src/debt/*.rs` - Pure computation
- `src/risk/*.rs` - Pure scoring logic

### APIs and Interfaces

```rust
// src/observability/tracing.rs

/// Initialize tracing with environment-based filter
pub fn init_tracing();

/// Initialize tracing with custom filter
pub fn init_tracing_with_filter(filter: &str);

/// Check if debug logging is enabled (for expensive debug formatting)
pub fn is_debug_enabled() -> bool {
    tracing::enabled!(tracing::Level::DEBUG)
}
```

## Dependencies

- **Prerequisites**: Spec 207 (Panic Hook)
- **Affected Components**:
  - All effect boundary modules
  - Main entry point
  - Builder modules
- **External Dependencies**:
  - `tracing = "0.1"`
  - `tracing-subscriber = "0.3"` with `env-filter` feature

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_phase_span_created() {
        perform_unified_analysis_with_options(&path, &results, &config).unwrap();

        assert!(logs_contain("unified_analysis"));
        assert!(logs_contain("call_graph_building"));
        assert!(logs_contain("debt_scoring"));
    }

    #[traced_test]
    #[test]
    fn test_file_span_includes_path() {
        analyze_file(Path::new("test.rs"), "fn foo() {}").unwrap();

        assert!(logs_contain("analyze_file"));
        assert!(logs_contain("test.rs"));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_rust_log_filtering() {
    // With RUST_LOG=warn, only warnings should appear
    std::env::set_var("RUST_LOG", "warn");
    init_tracing();

    // Run analysis
    let output = capture_stderr(|| {
        analyze_project(&path).unwrap();
    });

    // Should not contain debug/info messages
    assert!(!output.contains("Building call graph"));
    assert!(!output.contains("analyze_file"));
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Initialize the tracing subscriber for debtmap.
///
/// This sets up structured logging with the following log levels:
/// - `error!` - Actual errors affecting results
/// - `warn!` - Recoverable issues
/// - `info!` - Phase-level progress
/// - `debug!` - Detailed per-file progress
/// - `trace!` - Very verbose output
///
/// Control verbosity with RUST_LOG:
/// ```bash
/// RUST_LOG=info debtmap analyze .    # Phase progress
/// RUST_LOG=debug debtmap analyze .   # Detailed progress
/// RUST_LOG=debtmap=debug debtmap .   # Debug only debtmap crate
/// ```
pub fn init_tracing() { ... }
```

### User Documentation

Update CLI documentation:
```markdown
## Logging and Debugging

Debtmap uses structured logging controlled by the `RUST_LOG` environment variable:

```bash
# Default: warnings and errors only
debtmap analyze .

# Show phase-level progress
RUST_LOG=info debtmap analyze .

# Detailed debugging output
RUST_LOG=debug debtmap analyze .

# Very verbose tracing
RUST_LOG=trace debtmap analyze .

# Debug specific modules
RUST_LOG=debtmap::builders=debug debtmap analyze .
```

### Log Levels

- **error**: Unrecoverable errors
- **warn**: Issues that don't stop analysis
- **info**: Major milestones (phase start/complete)
- **debug**: Per-file progress and details
- **trace**: Very verbose, per-function output
```

## Implementation Notes

### Pure Functions Stay Pure

The key principle: **pure computation functions should have NO logging**.

```rust
// WRONG - logging in pure function
fn calculate_complexity(ast: &Ast) -> u32 {
    debug!("Calculating complexity"); // NO!
    ast.functions().map(|f| f.weight()).sum()
}

// RIGHT - pure function, logging at boundary
fn calculate_complexity(ast: &Ast) -> u32 {
    // Pure computation only
    ast.functions().map(|f| f.weight()).sum()
}

// Logging happens at effect boundary
fn analyze_file(path: &Path) -> Result<Metrics> {
    let ast = parse(path)?;
    debug!("Calculating complexity");
    let complexity = calculate_complexity(&ast); // Pure call
    debug!(complexity, "Complexity calculated");
    Ok(Metrics { complexity })
}
```

### Span vs Event

- **Spans**: Wrap operations with duration (enter/exit)
- **Events**: Point-in-time log messages

```rust
// Span for operation duration
let _span = info_span!("load_config").entered();
// Events for milestones within span
info!("Reading config file");
let config = load_config()?;
info!(keys = config.len(), "Config loaded");
```

### Performance Considerations

- Spans have minimal overhead when not captured
- Use `debug!` or `trace!` for high-frequency operations
- Check `is_debug_enabled()` before expensive formatting:

```rust
if is_debug_enabled() {
    debug!(data = ?expensive_debug_format(&item), "Processing item");
}
```

## Migration and Compatibility

### Breaking Changes

None. Default behavior unchanged (warnings only).

### Migration Path

1. Add tracing dependencies
2. Initialize subscriber in main
3. Replace eprintln! gradually, module by module
4. Remove eprintln! calls from pure functions entirely
5. Add spans to major phases

### Backward Compatibility

- Default log level shows same output as before
- RUST_LOG provides opt-in verbosity
- No changes to command-line interface
