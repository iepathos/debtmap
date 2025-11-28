---
number: 204
title: Output Writers as Effects
category: foundation
priority: medium
status: draft
dependencies: [195, 198, 200]
created: 2025-11-27
---

# Specification 204: Output Writers as Effects

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 195, 198, 200 (stillwater foundation, effect composition, testing)

## Context

Debtmap's output writers currently perform direct I/O operations, making them:

1. **Hard to test** - Require file system setup and cleanup
2. **Not composable** - Can't be chained with other effects
3. **Inconsistent** - Different error handling patterns across writers
4. **Inflexible** - Can't easily mock output destinations

Current output writers in `src/io/writers/`:
- `terminal.rs` - Console output
- `markdown/*.rs` - Markdown file generation
- `json.rs` - JSON output
- `html.rs` - HTML report generation

Migrating these to the Effect pattern enables:
- **Testability** - Use `DebtmapTestEnv` for unit tests
- **Composability** - Chain multiple outputs in a single pipeline
- **Consistency** - Unified error handling across all writers
- **Flexibility** - Easy to add new output formats

## Objective

Convert all output writers to use stillwater's Effect pattern, enabling testable, composable output operations while maintaining backwards compatibility.

## Requirements

### Functional Requirements

1. **Effect-Based Writing**
   - All output operations return `AnalysisEffect<()>`
   - Support both file and stream destinations
   - Enable chaining multiple output formats

2. **Multiple Output Destinations**
   - File system (existing)
   - In-memory buffers (for testing)
   - Stdout/stderr streams
   - Custom writers via trait

3. **Composable Output Pipelines**
   - Write to multiple formats in one pass
   - Transform output before writing
   - Support conditional output based on config

### Non-Functional Requirements

1. **Performance**
   - No significant overhead vs direct I/O
   - Support buffered writing for large outputs
   - Enable parallel writes to different files

2. **Testability**
   - All writers testable without file system
   - Easy to verify output content in tests
   - No temp file cleanup required

## Acceptance Criteria

- [ ] Create `OutputEffect<T>` type alias for output operations
- [ ] Migrate `TerminalWriter` to effect-based pattern
- [ ] Migrate `MarkdownWriter` to effect-based pattern
- [ ] Migrate `JsonWriter` to effect-based pattern
- [ ] Migrate `HtmlWriter` to effect-based pattern
- [ ] Add `OutputDestination` trait for pluggable destinations
- [ ] Create `write_analysis_report_effect` for full report generation
- [ ] Add tests using `DebtmapTestEnv` for all writers
- [ ] Update existing code to use new effect-based writers
- [ ] Documentation with examples

## Technical Details

### Implementation Approach

#### 1. Output Destination Trait

```rust
// In src/io/traits.rs
/// Trait for output destinations that can receive analysis output.
pub trait OutputDestination: Send + Sync {
    /// Write string content to the destination.
    fn write_str(&self, content: &str) -> Result<(), AnalysisError>;

    /// Flush any buffered content.
    fn flush(&self) -> Result<(), AnalysisError>;

    /// Get a description of the destination for error messages.
    fn description(&self) -> String;
}

/// File system output destination.
pub struct FileDestination {
    path: PathBuf,
}

/// In-memory output destination for testing.
pub struct MemoryDestination {
    buffer: Arc<RwLock<String>>,
}

/// Standard output destination.
pub struct StdoutDestination;
```

#### 2. Effect-Based Writers

```rust
// In src/io/writers/effects.rs
use stillwater::effect::prelude::*;

/// Write analysis results to markdown format.
pub fn write_markdown_effect(
    results: AnalysisResults,
    path: PathBuf,
) -> AnalysisEffect<()> {
    from_fn(move |env: &RealEnv| {
        let content = render_markdown(&results, env.config())?;
        env.file_system()
            .write(&path, &content)
            .map_err(|e| AnalysisError::io_with_path(
                format!("Failed to write markdown: {}", e.message()),
                &path,
            ))
    }).boxed()
}

/// Write analysis results to JSON format.
pub fn write_json_effect(
    results: AnalysisResults,
    path: PathBuf,
) -> AnalysisEffect<()> {
    from_fn(move |env: &RealEnv| {
        let json = serde_json::to_string_pretty(&results)
            .map_err(|e| AnalysisError::serialization(e.to_string()))?;
        env.file_system()
            .write(&path, &json)
            .map_err(|e| AnalysisError::io_with_path(
                format!("Failed to write JSON: {}", e.message()),
                &path,
            ))
    }).boxed()
}

/// Write analysis results to HTML report.
pub fn write_html_effect(
    results: AnalysisResults,
    path: PathBuf,
) -> AnalysisEffect<()> {
    from_fn(move |env: &RealEnv| {
        let html = render_html(&results, env.config())?;
        env.file_system()
            .write(&path, &html)
            .map_err(|e| AnalysisError::io_with_path(
                format!("Failed to write HTML: {}", e.message()),
                &path,
            ))
    }).boxed()
}

/// Write to terminal with formatting.
pub fn write_terminal_effect(
    results: AnalysisResults,
) -> AnalysisEffect<()> {
    from_fn(move |env: &RealEnv| {
        let output = format_terminal(&results, env.config())?;
        print!("{}", output);
        Ok(())
    }).boxed()
}
```

#### 3. Composable Output Pipeline

```rust
/// Write analysis results to multiple formats.
pub fn write_multi_format_effect(
    results: AnalysisResults,
    config: &OutputConfig,
) -> AnalysisEffect<()> {
    let mut effects: Vec<AnalysisEffect<()>> = Vec::new();

    if let Some(ref md_path) = config.markdown_path {
        effects.push(write_markdown_effect(results.clone(), md_path.clone()));
    }

    if let Some(ref json_path) = config.json_path {
        effects.push(write_json_effect(results.clone(), json_path.clone()));
    }

    if let Some(ref html_path) = config.html_path {
        effects.push(write_html_effect(results.clone(), html_path.clone()));
    }

    if config.terminal_output {
        effects.push(write_terminal_effect(results));
    }

    // Execute all writes (potentially in parallel)
    sequence_effect(effects)
        .map(|_| ())
        .boxed()
}
```

#### 4. Pure Rendering Functions

```rust
// In src/io/writers/markdown/render.rs

/// Pure function to render analysis results to markdown string.
/// No I/O - just data transformation.
pub fn render_markdown(
    results: &AnalysisResults,
    config: &DebtmapConfig,
) -> Result<String, AnalysisError> {
    let mut output = String::new();

    render_header(&mut output, results)?;
    render_summary(&mut output, results)?;
    render_debt_items(&mut output, results, config)?;
    render_metrics(&mut output, results)?;

    Ok(output)
}

/// Pure function to render terminal output.
pub fn format_terminal(
    results: &AnalysisResults,
    config: &DebtmapConfig,
) -> Result<String, AnalysisError> {
    let color_mode = config.formatting
        .as_ref()
        .map(|f| f.color_mode)
        .unwrap_or(ColorMode::Auto);

    let mut output = String::new();
    // ... formatting logic
    Ok(output)
}
```

### Architecture Changes

1. **New Module**: `src/io/writers/effects.rs`
   - Effect-based writer functions
   - Output pipeline composition

2. **New Module**: `src/io/destinations.rs`
   - `OutputDestination` trait
   - Built-in destination implementations

3. **Modified Modules**: `src/io/writers/*.rs`
   - Extract pure rendering functions
   - Add effect wrappers

4. **Modified Module**: `src/io/output.rs`
   - Integrate effect-based writers
   - Provide backwards-compatible API

### Data Structures

```rust
/// Configuration for output generation.
#[derive(Debug, Clone)]
pub struct OutputConfig {
    /// Path for markdown output (if any).
    pub markdown_path: Option<PathBuf>,

    /// Path for JSON output (if any).
    pub json_path: Option<PathBuf>,

    /// Path for HTML output (if any).
    pub html_path: Option<PathBuf>,

    /// Whether to output to terminal.
    pub terminal_output: bool,

    /// Formatting options.
    pub formatting: FormattingConfig,
}

/// Result of an output operation with metadata.
#[derive(Debug, Clone)]
pub struct OutputResult {
    /// Destination description.
    pub destination: String,

    /// Number of bytes written.
    pub bytes_written: usize,

    /// Time taken to write.
    pub duration: Duration,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 195 (stillwater foundation)
  - Spec 198 (effect composition)
  - Spec 200 (testing infrastructure)

- **Affected Components**:
  - `src/io/writers/terminal.rs`
  - `src/io/writers/markdown/*.rs`
  - `src/io/writers/json.rs`
  - `src/io/writers/html.rs`
  - `src/io/output.rs`

- **External Dependencies**:
  - stillwater 0.11.0+ (already integrated)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_write_markdown_effect() {
    let env = DebtmapTestEnv::new();
    let results = create_test_results();

    let effect = write_markdown_effect(results, PathBuf::from("output.md"));
    let result = run_effect_with_env(effect, &env);

    assert!(result.is_ok());

    // Verify content was written
    let content = env.file_system()
        .read_to_string(Path::new("output.md"))
        .unwrap();
    assert!(content.contains("# Technical Debt Report"));
}

#[test]
fn test_multi_format_output() {
    let env = DebtmapTestEnv::new();
    let results = create_test_results();

    let config = OutputConfig {
        markdown_path: Some("report.md".into()),
        json_path: Some("report.json".into()),
        ..Default::default()
    };

    let effect = write_multi_format_effect(results, &config);
    let result = run_effect_with_env(effect, &env);

    assert!(result.is_ok());
    assert!(env.file_system().exists(Path::new("report.md")));
    assert!(env.file_system().exists(Path::new("report.json")));
}
```

### Integration Tests
- Test full report generation workflow
- Test output with various configurations
- Test error handling for write failures

### Performance Tests
- Benchmark effect-based vs direct I/O
- Test parallel writes to multiple files

## Documentation Requirements

- **Code Documentation**: Rustdoc for all writer effects
- **User Documentation**: Update CLI help with output options
- **Architecture Updates**: Document output pipeline in DESIGN.md

## Implementation Notes

1. **Pure Core**: Keep rendering logic pure (string manipulation only). Effects only for actual I/O.

2. **Buffering**: Use buffered writers for large outputs to improve performance.

3. **Error Context**: Include destination path in all error messages.

4. **Atomic Writes**: Consider writing to temp file then renaming for atomicity.

## Migration and Compatibility

- **Backwards Compatible**: Existing output functions remain available
- **Gradual Migration**: New code uses effect-based writers
- **No CLI Changes**: User-facing behavior unchanged

```rust
// Backwards-compatible wrapper
pub fn write_markdown(
    results: AnalysisResults,
    path: PathBuf,
    config: DebtmapConfig,
) -> anyhow::Result<()> {
    run_effect(write_markdown_effect(results, path), config)
}
```
