---
number: 3
title: Sink Effect for Report Streaming
category: optimization
priority: medium
status: draft
dependencies: [1]
created: 2025-12-20
---

# Specification 003: Sink Effect for Report Streaming

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 001 (Stillwater 0.15 Upgrade)

## Context

When analyzing large codebases, debtmap can generate substantial reports. Current implementation:

- Accumulates all report data in memory before writing
- Can cause memory pressure on very large projects
- Delays user feedback until entire analysis completes
- Blocks on final report generation

Stillwater 0.15's Sink Effect enables streaming output with O(1) memory overhead, allowing:

- Real-time report generation as analysis progresses
- Constant memory usage regardless of codebase size
- Progressive feedback to users during long analyses

## Objective

Integrate the Sink Effect pattern to stream analysis reports (JSON, text, or custom formats) directly to output files or stdout with constant memory overhead, enabling real-time output for large codebases.

## Requirements

### Functional Requirements

1. **Streaming Report Output**
   - Stream report lines as they're generated
   - Support JSON Lines (JSONL) format for streaming JSON
   - Support text report streaming
   - Enable file and stdout destinations

2. **Sink Effect Integration**
   - Create type aliases for Sink-enabled report effects
   - Implement streaming for each output format
   - Support async file I/O for non-blocking writes

3. **Memory Efficiency**
   - O(1) memory overhead for report generation
   - No accumulation of report data in memory
   - Immediate write-through to destination

4. **Testing Support**
   - `run_collecting()` for test environments
   - Ability to capture streamed output in tests
   - Mock sink implementations

### Non-Functional Requirements

- Support files up to 10GB of report output
- Maintain current report format compatibility
- Minimal latency between analysis completion and output
- Graceful handling of I/O errors during streaming

## Acceptance Criteria

- [ ] `ReportSinkEffect<T>` type alias for sink-enabled effects
- [ ] `emit_report_line()` helper for streaming single lines
- [ ] JSON Lines (JSONL) streaming format implemented
- [ ] Text report streaming implemented
- [ ] File sink implementation with async I/O
- [ ] Stdout sink implementation
- [ ] Memory usage remains constant regardless of output size
- [ ] Existing report tests pass using `run_collecting()`
- [ ] New integration tests verify streaming behavior
- [ ] CLI `--streaming` flag to enable streaming mode

## Technical Details

### Implementation Approach

```rust
// Type alias for sink-enabled report effects
pub type ReportSinkEffect<T> = impl SinkEffect<
    Output = T,
    Error = AnalysisError,
    Env = RealEnv,
    Item = ReportLine,
>;

// Report line types
#[derive(Debug, Clone)]
pub enum ReportLine {
    JsonLine(String),      // Single JSON object as line
    TextLine(String),      // Text report line
    Separator,             // Section separator
    Header(String),        // Section header
}

// Helper for emitting report lines
pub fn emit_report_line(line: ReportLine) -> impl SinkEffect<Output = (), Item = ReportLine> {
    emit(line)
}

// Streaming JSON Lines for file metrics
pub fn stream_file_metrics(metrics: &FileMetrics) -> impl SinkEffect<Output = (), Item = ReportLine> {
    let json = serde_json::to_string(metrics).unwrap_or_default();
    emit_report_line(ReportLine::JsonLine(json))
}
```

### Streaming Analysis Pipeline

```rust
// Full analysis with streaming output
pub fn analyze_and_stream(config: &AnalysisConfig) -> ReportSinkEffect<AnalysisSummary> {
    discover_files(config)
        .and_then(|files| {
            // Process each file and stream results
            files.into_iter().fold(
                pure(AnalysisSummary::default()),
                |acc, file| {
                    acc.and_then(move |summary| {
                        analyze_file(&file)
                            .and_then(|metrics| {
                                // Stream this file's results immediately
                                stream_file_metrics(&metrics)
                                    .map(move |_| summary.add(metrics))
                            })
                    })
                }
            )
        })
}

// Execute with file sink
pub async fn run_with_file_sink(
    effect: impl SinkEffect<Item = ReportLine>,
    output_path: &Path,
) -> Result<()> {
    let file = tokio::fs::File::create(output_path).await?;
    let mut writer = tokio::io::BufWriter::new(file);

    effect.run_with_sink(&env, |line| async move {
        let text = match line {
            ReportLine::JsonLine(json) => format!("{}\n", json),
            ReportLine::TextLine(text) => format!("{}\n", text),
            ReportLine::Separator => "---\n".to_string(),
            ReportLine::Header(h) => format!("\n## {}\n\n", h),
        };
        writer.write_all(text.as_bytes()).await.map_err(|e| e.into())
    }).await
}
```

### CLI Integration

```rust
// CLI flag for streaming mode
#[derive(Parser)]
struct AnalyzeArgs {
    /// Enable streaming output mode for large codebases
    #[arg(long, default_value = "false")]
    streaming: bool,

    /// Output file for streaming mode
    #[arg(long, requires = "streaming")]
    stream_to: Option<PathBuf>,
}

// Command handler
pub async fn handle_analyze(args: AnalyzeArgs) -> Result<()> {
    let effect = build_analysis_pipeline(&args);

    if args.streaming {
        let output = args.stream_to.unwrap_or_else(|| PathBuf::from("-"));
        if output == Path::new("-") {
            run_with_stdout_sink(effect).await
        } else {
            run_with_file_sink(effect, &output).await
        }
    } else {
        // Traditional batch mode
        let results = effect.run(&env).await?;
        write_report(&results)
    }
}
```

### Data Structures

```rust
// Sink configuration
pub struct SinkConfig {
    pub format: ReportFormat,
    pub destination: SinkDestination,
    pub buffer_size: usize,
    pub flush_interval: Duration,
}

pub enum SinkDestination {
    Stdout,
    File(PathBuf),
    Callback(Box<dyn Fn(ReportLine) -> Result<()> + Send + Sync>),
}

pub enum ReportFormat {
    JsonLines,    // One JSON object per line
    Text,         // Human-readable text
    Markdown,     // Markdown-formatted report
}
```

### Affected Files

- `src/effects/core.rs` - Add Sink Effect types
- `src/effects/sink.rs` - New module for sink implementations
- `src/io/report.rs` - Refactor for streaming support
- `src/cli/commands/analyze.rs` - Add streaming flag
- `src/output/formats/*.rs` - Streaming format implementations

## Dependencies

- **Prerequisites**: Spec 001 (Stillwater 0.15 Upgrade)
- **Affected Components**: Effect system, I/O layer, CLI, output formatters
- **External Dependencies**: stillwater 0.15 Sink Effect, tokio async I/O

## Testing Strategy

- **Unit Tests**: Verify sink emission and collection
- **Integration Tests**: End-to-end streaming to temp files
- **Memory Tests**: Verify O(1) memory for large outputs
- **Performance Tests**: Compare streaming vs batch for large codebases

```rust
#[tokio::test]
async fn streaming_uses_constant_memory() {
    let large_codebase = generate_large_test_codebase(10_000); // 10k files

    let initial_memory = get_memory_usage();

    let effect = analyze_and_stream(&large_codebase);
    let tempfile = tempfile::NamedTempFile::new()?;
    run_with_file_sink(effect, tempfile.path()).await?;

    let peak_memory = get_peak_memory_usage();

    // Memory should not grow linearly with output size
    assert!(peak_memory - initial_memory < 100_000_000); // < 100MB
}

#[tokio::test]
async fn run_collecting_captures_all_lines() {
    let effect = analyze_and_stream(&test_config);
    let (result, lines) = effect.run_collecting(&env).await;

    assert!(result.is_ok());
    assert!(!lines.is_empty());
    assert!(lines.iter().any(|l| matches!(l, ReportLine::JsonLine(_))));
}
```

## Documentation Requirements

- **Code Documentation**: Document sink types and streaming patterns
- **User Documentation**: Update CLI help with streaming options
- **Architecture Updates**: Document streaming architecture

## Implementation Notes

- Use buffered writers for better I/O performance
- Consider periodic flush for real-time visibility
- Handle broken pipe gracefully for stdout streaming
- Ensure proper cleanup on error (close file handles)
- Consider progress indicator integration with streaming

## Migration and Compatibility

No breaking changes. Streaming is opt-in via `--streaming` flag. Default behavior remains unchanged (batch mode). Report format compatibility maintained through format-specific streaming implementations.
