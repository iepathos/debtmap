---
number: 2
title: Writer Effect for Analysis Telemetry
category: optimization
priority: high
status: draft
dependencies: [1]
created: 2025-12-20
---

# Specification 002: Writer Effect for Analysis Telemetry

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 001 (Stillwater 0.15 Upgrade)

## Context

Currently, debtmap threads metrics and telemetry data through function parameters or accumulates them in mutable state. This approach:

- Clutters function signatures with telemetry concerns
- Makes testing more difficult (need to mock telemetry receivers)
- Couples business logic with logging/metrics infrastructure
- Requires explicit state management for collecting analysis statistics

Stillwater 0.15's Writer Effect provides a clean solution: accumulate telemetry alongside computation without threading state through every function.

## Objective

Integrate the Writer Effect pattern to collect analysis telemetry (files processed, complexity scores, debt items found, timing metrics) without polluting function signatures or requiring explicit state threading.

## Requirements

### Functional Requirements

1. **Define Analysis Event Types**
   - Create `AnalysisEvent` enum for telemetry events
   - Events should capture: file processing, parsing, complexity calculation, debt detection
   - Include timing information where relevant

2. **Writer Effect Integration**
   - Create type aliases for Writer-enabled analysis effects
   - Implement Monoid for telemetry event collection
   - Integrate with existing `AnalysisEffect<T>` infrastructure

3. **Telemetry Emission Points**
   - Emit events at key analysis pipeline stages
   - Capture file start/complete events
   - Record complexity metrics per file
   - Track debt item detection counts

4. **Telemetry Aggregation**
   - Collect all events via `run_writer()`
   - Aggregate into summary statistics
   - Support both detailed logging and summary modes

### Non-Functional Requirements

- Zero-cost when telemetry is not consumed
- Minimal memory overhead for event accumulation
- Thread-safe for parallel analysis
- Compatible with existing effect composition patterns

## Acceptance Criteria

- [ ] `AnalysisEvent` enum defined with comprehensive event variants
- [ ] `AnalysisMetrics` type implements `Monoid` for aggregation
- [ ] `AnalysisWriterEffect<T>` type alias created for writer-enabled effects
- [ ] `tell_event()` helper function for emitting single events
- [ ] File analysis emits start/complete events with timing
- [ ] Complexity analysis emits metrics events
- [ ] Debt detection emits discovery events
- [ ] `run_writer()` successfully collects all events
- [ ] Existing tests pass without modification
- [ ] New tests verify telemetry collection
- [ ] Telemetry can be disabled without code changes (via empty consumer)

## Technical Details

### Implementation Approach

```rust
// Event types
#[derive(Debug, Clone)]
pub enum AnalysisEvent {
    FileStarted { path: PathBuf, timestamp: Instant },
    FileCompleted { path: PathBuf, duration_ms: u64 },
    ParseComplete { path: PathBuf, function_count: usize },
    ComplexityCalculated { path: PathBuf, cognitive: u32, cyclomatic: u32 },
    DebtItemDetected { path: PathBuf, severity: Severity, category: String },
    PhaseStarted { phase: AnalysisPhase },
    PhaseCompleted { phase: AnalysisPhase, duration_ms: u64 },
}

// Metrics aggregation
#[derive(Debug, Clone, Default)]
pub struct AnalysisMetrics {
    pub events: Vec<AnalysisEvent>,
}

impl Monoid for AnalysisMetrics {
    fn empty() -> Self { Self::default() }
    fn combine(self, other: Self) -> Self {
        AnalysisMetrics {
            events: self.events.into_iter().chain(other.events).collect(),
        }
    }
}

// Type alias for writer-enabled effects
pub type AnalysisWriterEffect<T> = impl WriterEffect<
    Output = T,
    Error = AnalysisError,
    Env = RealEnv,
    Writes = AnalysisMetrics,
>;

// Helper for emitting events
pub fn tell_event(event: AnalysisEvent) -> impl WriterEffect<Output = (), Writes = AnalysisMetrics> {
    tell_one(AnalysisMetrics { events: vec![event] })
}
```

### Integration Pattern

```rust
// Example: file analysis with telemetry
pub fn analyze_file_with_telemetry(path: &Path) -> AnalysisWriterEffect<FileMetrics> {
    let start = Instant::now();

    tell_event(AnalysisEvent::FileStarted {
        path: path.to_path_buf(),
        timestamp: start
    })
    .and_then(|_| parse_file(path))
    .tap_tell(|ast| AnalysisMetrics {
        events: vec![AnalysisEvent::ParseComplete {
            path: path.to_path_buf(),
            function_count: ast.functions().count(),
        }]
    })
    .map(|ast| calculate_complexity(&ast))
    .tap_tell(|metrics| AnalysisMetrics {
        events: vec![
            AnalysisEvent::ComplexityCalculated {
                path: path.to_path_buf(),
                cognitive: metrics.cognitive,
                cyclomatic: metrics.cyclomatic,
            },
            AnalysisEvent::FileCompleted {
                path: path.to_path_buf(),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        ]
    })
}
```

### Data Structures

```rust
// Summary statistics derived from events
pub struct AnalysisSummary {
    pub files_processed: usize,
    pub total_duration_ms: u64,
    pub total_functions: usize,
    pub avg_complexity: f64,
    pub debt_items_by_severity: HashMap<Severity, usize>,
}

impl From<AnalysisMetrics> for AnalysisSummary {
    fn from(metrics: AnalysisMetrics) -> Self {
        // Aggregate events into summary
    }
}
```

### Affected Files

- `src/effects/core.rs` - Add Writer Effect types and helpers
- `src/effects/telemetry.rs` - New module for telemetry types
- `src/analysis/workflow/actions.rs` - Integrate telemetry emission
- `src/analyzers/*.rs` - Add telemetry emission points

## Dependencies

- **Prerequisites**: Spec 001 (Stillwater 0.15 Upgrade)
- **Affected Components**: Effect system, analysis pipeline, workflow actions
- **External Dependencies**: stillwater 0.15 Writer Effect

## Testing Strategy

- **Unit Tests**: Verify event emission and collection
- **Integration Tests**: Full analysis with telemetry collection
- **Property Tests**: Monoid laws for AnalysisMetrics
- **Performance Tests**: Ensure minimal overhead when telemetry collected

```rust
#[test]
fn telemetry_collects_all_events() {
    let effect = analyze_file_with_telemetry(&test_file);
    let (result, metrics) = effect.run_writer(&env).await;

    assert!(result.is_ok());
    assert!(metrics.events.iter().any(|e| matches!(e, AnalysisEvent::FileStarted { .. })));
    assert!(metrics.events.iter().any(|e| matches!(e, AnalysisEvent::FileCompleted { .. })));
}
```

## Documentation Requirements

- **Code Documentation**: Document event types and their semantics
- **User Documentation**: Explain telemetry output in CLI help
- **Architecture Updates**: Update ARCHITECTURE.md with telemetry design

## Implementation Notes

- Start with core analysis pipeline, then expand to all analyzers
- Consider using `im::Vector` for O(1) append if event volume is high
- Ensure events are `Send + Sync` for parallel processing
- Consider adding event filtering for verbose vs summary modes

## Migration and Compatibility

No breaking changes to public API. Telemetry is additive and opt-in through the Writer Effect pattern. Existing code continues to work unchanged.
