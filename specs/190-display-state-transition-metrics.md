---
number: 190
title: Display State Transition Metrics in Output Formats
category: optimization
priority: medium
status: draft
dependencies: [179, 192]
created: 2025-11-30
---

# Specification 190: Display State Transition Metrics in Output Formats

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 179, 192 (State machine and coordinator pattern detection)

## Context

Debtmap includes sophisticated state machine and coordinator pattern detection implemented in specs 179 and 192. The `StateMachinePatternDetector` analyzes Rust code to identify:

1. **State Machine Patterns** - Functions with state transitions, match expressions on enums, and action dispatches
2. **Coordinator Patterns** - Functions that accumulate actions based on state comparisons

These patterns are detected with confidence scores and captured in detailed signal structures:

**StateMachineSignals** (`src/priority/complexity_patterns.rs:24-31`):
- `transition_count`: Number of state transitions detected
- `match_expression_count`: Number of match expressions on state
- `has_enum_match`: Boolean flag for enum variant matching
- `has_state_comparison`: Boolean flag for state field comparisons
- `action_dispatch_count`: Number of action operations
- `confidence`: Confidence score (0.0-1.0)

**CoordinatorSignals** (`src/priority/complexity_patterns.rs:33-40`):
- `actions`: Count of action accumulations
- `comparisons`: Count of state comparisons
- `has_action_accumulation`: Boolean flag
- `has_helper_calls`: Boolean flag for helper calls
- `confidence`: Confidence score (0.0-1.0)

**Current Problem**: These valuable pattern detection results are **not displayed** in any of debtmap's output formats:
- **Terminal output** (`src/io/writers/terminal.rs`) - Shows complexity metrics but omits state transition data
- **JSON output** (`src/io/writers/json.rs`, `src/output/unified.rs`) - Doesn't include signal structures
- **Markdown output** (`src/io/writers/markdown/core.rs`) - Complexity tables lack state transition columns

This creates a transparency gap where users cannot see:
- Which functions are detected as state machines or coordinators
- The confidence level of pattern detection
- Specific signal metrics (transition counts, action counts, etc.)
- How patterns influence refactoring recommendations

## Objective

Make state transition analysis **transparent and visible** across all output formats by:

1. **Terminal Output**: Display state machine/coordinator patterns in complexity hotspots with specific metrics
2. **JSON Output**: Include `state_machine_signals` and `coordinator_signals` in serialized function metrics
3. **Markdown Output**: Add columns/sections showing pattern detection results and confidence scores
4. **Unified Format**: Extend unified JSON schema to include pattern signal data

Result: Users can see state transition analysis results in all output formats, understand which functions have these patterns, and make informed refactoring decisions.

## Requirements

### Functional Requirements

1. **Terminal Output Enhancement**
   - Add pattern indicator to complexity hotspot display (e.g., "🔄 State Machine", "🎯 Coordinator")
   - Show key metrics for detected patterns:
     - State Machine: transition count, confidence
     - Coordinator: action count, comparison count, confidence
   - Include pattern-specific information in refactoring guidance
   - Use color coding for confidence levels (high: green, medium: yellow, low: red)

2. **JSON Output Enhancement**
   - Add `state_machine_signals` field to function metrics in JSON output
   - Add `coordinator_signals` field to function metrics in JSON output
   - Serialize full signal structures with all fields
   - Maintain backward compatibility (fields optional, omit if None)

3. **Markdown Output Enhancement**
   - Add "Pattern" column to complexity analysis table
   - Show pattern type (State Machine, Coordinator, or "-")
   - Add "Confidence" column showing detection confidence (0.0-1.0)
   - Create optional detailed section with pattern metrics breakdown
   - Include pattern information in recommendations

4. **Unified Format Extension**
   - Extend `UnifiedDebtItemOutput` schema to include pattern signals
   - Add `pattern_type` field: enum of "state_machine", "coordinator", or null
   - Add `pattern_confidence` field: optional f64
   - Add `pattern_details` field: object with pattern-specific metrics
   - Document schema changes in output format documentation

### Non-Functional Requirements

1. **Backward Compatibility**
   - Existing JSON/Markdown parsers should not break
   - New fields are optional and omitted when not present
   - Version unified format schema appropriately

2. **Performance**
   - Display enhancements should not slow down output generation
   - Serialization overhead should be minimal (<1% of total runtime)

3. **Clarity**
   - Pattern information should be clear and actionable
   - Confidence scores should be easy to interpret
   - Metrics should be self-explanatory or well-documented

4. **Consistency**
   - Pattern display should be consistent across all formats
   - Terminology should match detection code (state machine, coordinator)
   - Confidence thresholds should align with detection thresholds (0.6-0.7)

## Acceptance Criteria

- [ ] Terminal output shows state machine pattern indicator in complexity hotspots
- [ ] Terminal output shows coordinator pattern indicator in complexity hotspots
- [ ] Terminal output displays transition count for state machines
- [ ] Terminal output displays action/comparison counts for coordinators
- [ ] Terminal output shows confidence scores with color coding
- [ ] JSON output includes `state_machine_signals` field when present
- [ ] JSON output includes `coordinator_signals` field when present
- [ ] JSON serialization includes all signal fields (transition_count, confidence, etc.)
- [ ] Markdown output has "Pattern" column in complexity table
- [ ] Markdown output has "Confidence" column in complexity table
- [ ] Markdown shows pattern types correctly (State Machine, Coordinator, -)
- [ ] Unified JSON format includes `pattern_type` field
- [ ] Unified JSON format includes `pattern_confidence` field
- [ ] Unified JSON format includes `pattern_details` object
- [ ] Enhanced markdown writer displays pattern information
- [ ] All output formats tested with functions having state machine signals
- [ ] All output formats tested with functions having coordinator signals
- [ ] Backward compatibility verified (old JSON/Markdown still parseable)
- [ ] Documentation updated to describe new output fields
- [ ] Integration tests verify pattern display in all formats

## Technical Details

### Implementation Approach

**Phase 1: Data Access**

The signals are stored in `FunctionMetrics.language_specific`:
```rust
// src/core/mod.rs:63-92
pub struct FunctionMetrics {
    pub language_specific: Option<LanguageSpecificData>,
    // ... other fields
}

// For Rust code:
pub enum LanguageSpecificData {
    Rust(RustPatternResult),
    // ... other languages
}

// src/analysis/rust_patterns/detector.rs:133-145
pub struct RustPatternResult {
    pub state_machine_signals: Option<StateMachineSignals>,
    pub coordinator_signals: Option<CoordinatorSignals>,
    // ... other pattern fields
}
```

**Phase 2: Terminal Output** (`src/io/writers/terminal.rs`)

Enhance `print_complexity_hotspots()` method:

```rust
fn format_pattern_info(metrics: &FunctionMetrics) -> Option<String> {
    if let Some(LanguageSpecificData::Rust(rust_data)) = &metrics.language_specific {
        // Check state machine first
        if let Some(sm_signals) = &rust_data.state_machine_signals {
            if sm_signals.confidence >= 0.7 {
                return Some(format!(
                    "🔄 State Machine (transitions: {}, confidence: {:.2})",
                    sm_signals.transition_count,
                    sm_signals.confidence
                ));
            }
        }

        // Check coordinator second
        if let Some(coord_signals) = &rust_data.coordinator_signals {
            if coord_signals.confidence >= 0.7 {
                return Some(format!(
                    "🎯 Coordinator (actions: {}, comparisons: {}, confidence: {:.2})",
                    coord_signals.actions,
                    coord_signals.comparisons,
                    coord_signals.confidence
                ));
            }
        }
    }
    None
}
```

Display in hotspot output:
```
  1. src/analyzers/state_machine_pattern_detector.rs:123 detect_state_machine()
     Cyclomatic: 12, Cognitive: 18, Nesting: 3
     🔄 State Machine (transitions: 4, confidence: 0.85)
     ACTION: Consider transition table pattern for state management
```

**Phase 3: JSON Output** (`src/io/writers/json.rs`)

Ensure `FunctionMetrics` serialization includes language_specific field.
The signals should already be serializable via `#[derive(Serialize)]`, but verify:

```rust
// Verify in src/priority/complexity_patterns.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineSignals {
    pub transition_count: u32,
    pub match_expression_count: u32,
    pub has_enum_match: bool,
    pub has_state_comparison: bool,
    pub action_dispatch_count: u32,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorSignals {
    pub actions: u32,
    pub comparisons: u32,
    pub has_action_accumulation: bool,
    pub has_helper_calls: bool,
    pub confidence: f64,
}
```

Example JSON output:
```json
{
  "function": "detect_state_machine",
  "cyclomatic_complexity": 12,
  "cognitive_complexity": 18,
  "language_specific": {
    "Rust": {
      "state_machine_signals": {
        "transition_count": 4,
        "match_expression_count": 2,
        "has_enum_match": true,
        "has_state_comparison": true,
        "action_dispatch_count": 8,
        "confidence": 0.85
      }
    }
  }
}
```

**Phase 4: Markdown Output** (`src/io/writers/markdown/core.rs`)

Modify `write_complexity_analysis()` to add columns:

```rust
fn write_complexity_table(&mut self, items: &[ComplexityItem]) -> anyhow::Result<()> {
    writeln!(self.writer, "| Location | Function | Cyclomatic | Cognitive | Pattern | Confidence | Recommendation |")?;
    writeln!(self.writer, "|----------|----------|------------|-----------|---------|------------|----------------|")?;

    for item in items {
        let pattern_type = extract_pattern_type(&item.metrics);
        let confidence = extract_pattern_confidence(&item.metrics);

        writeln!(
            self.writer,
            "| {}:{} | {} | {} | {} | {} | {} | {} |",
            item.file,
            item.line,
            item.function,
            item.metrics.cyclomatic,
            item.metrics.cognitive,
            pattern_type.unwrap_or("-"),
            confidence.map(|c| format!("{:.2}", c)).unwrap_or_else(|| "-".to_string()),
            item.recommendation
        )?;
    }
    Ok(())
}

fn extract_pattern_type(metrics: &FunctionMetrics) -> Option<&str> {
    if let Some(LanguageSpecificData::Rust(rust_data)) = &metrics.language_specific {
        if rust_data.state_machine_signals.as_ref()
            .map(|s| s.confidence >= 0.7).unwrap_or(false) {
            return Some("State Machine");
        }
        if rust_data.coordinator_signals.as_ref()
            .map(|s| s.confidence >= 0.7).unwrap_or(false) {
            return Some("Coordinator");
        }
    }
    None
}

fn extract_pattern_confidence(metrics: &FunctionMetrics) -> Option<f64> {
    if let Some(LanguageSpecificData::Rust(rust_data)) = &metrics.language_specific {
        if let Some(signals) = &rust_data.state_machine_signals {
            if signals.confidence >= 0.7 {
                return Some(signals.confidence);
            }
        }
        if let Some(signals) = &rust_data.coordinator_signals {
            if signals.confidence >= 0.7 {
                return Some(signals.confidence);
            }
        }
    }
    None
}
```

Example markdown output:
```markdown
| Location | Function | Cyclomatic | Cognitive | Pattern | Confidence | Recommendation |
|----------|----------|------------|-----------|---------|------------|----------------|
| detector.rs:123 | detect_state_machine | 12 | 18 | State Machine | 0.85 | Consider transition table |
| analyzer.rs:45 | process_events | 8 | 14 | Coordinator | 0.78 | Extract action dispatch |
| parser.rs:200 | parse_expression | 6 | 9 | - | - | Acceptable complexity |
```

**Phase 5: Unified Format** (`src/output/unified.rs`)

Extend `UnifiedDebtItemOutput` schema:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedDebtItemOutput {
    // ... existing fields ...

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_type: Option<String>,  // "state_machine" | "coordinator"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_confidence: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_details: Option<serde_json::Value>,  // Pattern-specific metrics
}

fn extract_pattern_data(metrics: &FunctionMetrics) -> (Option<String>, Option<f64>, Option<serde_json::Value>) {
    if let Some(LanguageSpecificData::Rust(rust_data)) = &metrics.language_specific {
        if let Some(sm_signals) = &rust_data.state_machine_signals {
            if sm_signals.confidence >= 0.7 {
                let details = serde_json::json!({
                    "transition_count": sm_signals.transition_count,
                    "match_expression_count": sm_signals.match_expression_count,
                    "action_dispatch_count": sm_signals.action_dispatch_count,
                });
                return (
                    Some("state_machine".to_string()),
                    Some(sm_signals.confidence),
                    Some(details),
                );
            }
        }

        if let Some(coord_signals) = &rust_data.coordinator_signals {
            if coord_signals.confidence >= 0.7 {
                let details = serde_json::json!({
                    "actions": coord_signals.actions,
                    "comparisons": coord_signals.comparisons,
                });
                return (
                    Some("coordinator".to_string()),
                    Some(coord_signals.confidence),
                    Some(details),
                );
            }
        }
    }
    (None, None, None)
}
```

### Architecture Changes

**No breaking changes** - This is purely additive enhancement to output formatting.

Modified files:
- `src/io/writers/terminal.rs` - Add pattern display to complexity hotspots
- `src/io/writers/markdown/core.rs` - Add pattern columns to complexity table
- `src/io/writers/markdown/formatters.rs` - Pure formatting functions for patterns
- `src/output/unified.rs` - Extend UnifiedDebtItemOutput schema
- `src/io/writers/enhanced_markdown/mod.rs` - Add pattern sections (if applicable)

### Data Structures

Pattern information already exists in:
- `StateMachineSignals` (src/priority/complexity_patterns.rs:24-31)
- `CoordinatorSignals` (src/priority/complexity_patterns.rs:33-40)
- `RustPatternResult` (src/analysis/rust_patterns/detector.rs:133-145)
- `FunctionMetrics.language_specific` (src/core/mod.rs:63-92)

New helper structures:
```rust
// In src/io/writers/terminal.rs
struct PatternDisplay {
    icon: &'static str,
    name: &'static str,
    metrics: Vec<(String, String)>,  // (label, value) pairs
    confidence: f64,
}

impl PatternDisplay {
    fn from_state_machine(signals: &StateMachineSignals) -> Self { ... }
    fn from_coordinator(signals: &CoordinatorSignals) -> Self { ... }
    fn format(&self) -> String { ... }
}
```

### APIs and Interfaces

**Public API additions** (pure functions in formatters):

```rust
// src/io/writers/markdown/formatters.rs
pub fn format_pattern_type(metrics: &FunctionMetrics) -> String;
pub fn format_pattern_confidence(metrics: &FunctionMetrics) -> String;
pub fn format_pattern_details(metrics: &FunctionMetrics) -> String;

// src/io/writers/terminal.rs (or shared module)
pub fn extract_pattern_info(metrics: &FunctionMetrics) -> Option<PatternInfo>;

pub struct PatternInfo {
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub display_metrics: HashMap<String, String>,
}

pub enum PatternType {
    StateMachine,
    Coordinator,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 179: State machine and coordinator pattern detection implementation
  - Spec 192: Enhanced coordinator detection with state-awareness

- **Affected Components**:
  - `src/io/writers/terminal.rs` - Terminal output enhancement
  - `src/io/writers/markdown/core.rs` - Markdown table enhancement
  - `src/io/writers/markdown/formatters.rs` - Pure formatting functions
  - `src/io/writers/enhanced_markdown/mod.rs` - Enhanced markdown patterns
  - `src/output/unified.rs` - Unified format schema extension
  - `src/priority/complexity_patterns.rs` - Ensure Serialize/Deserialize traits
  - Documentation files describing output formats

- **External Dependencies**: None (uses existing serde serialization)

## Testing Strategy

### Unit Tests

**Terminal Output Tests** (`src/io/writers/terminal.rs`):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_format_state_machine_pattern() {
        let signals = StateMachineSignals {
            transition_count: 4,
            match_expression_count: 2,
            has_enum_match: true,
            has_state_comparison: true,
            action_dispatch_count: 8,
            confidence: 0.85,
        };

        let formatted = format_pattern_info_from_signals(&signals);
        assert!(formatted.contains("State Machine"));
        assert!(formatted.contains("transitions: 4"));
        assert!(formatted.contains("0.85"));
    }

    #[test]
    fn test_format_coordinator_pattern() {
        let signals = CoordinatorSignals {
            actions: 5,
            comparisons: 3,
            has_action_accumulation: true,
            has_helper_calls: true,
            confidence: 0.78,
        };

        let formatted = format_pattern_info_from_signals(&signals);
        assert!(formatted.contains("Coordinator"));
        assert!(formatted.contains("actions: 5"));
        assert!(formatted.contains("comparisons: 3"));
    }

    #[test]
    fn test_low_confidence_pattern_not_displayed() {
        let signals = StateMachineSignals {
            confidence: 0.5,  // Below threshold
            ..Default::default()
        };

        let formatted = format_pattern_info(&create_metrics_with_signals(signals));
        assert!(formatted.is_none());
    }
}
```

**Markdown Tests** (`src/io/writers/markdown/formatters.rs`):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_extract_pattern_type_state_machine() {
        let metrics = create_metrics_with_state_machine(0.85);
        assert_eq!(extract_pattern_type(&metrics), Some("State Machine"));
    }

    #[test]
    fn test_extract_pattern_type_coordinator() {
        let metrics = create_metrics_with_coordinator(0.78);
        assert_eq!(extract_pattern_type(&metrics), Some("Coordinator"));
    }

    #[test]
    fn test_extract_pattern_type_none() {
        let metrics = create_metrics_without_patterns();
        assert_eq!(extract_pattern_type(&metrics), None);
    }

    #[test]
    fn test_format_confidence() {
        let metrics = create_metrics_with_state_machine(0.8567);
        assert_eq!(format_pattern_confidence(&metrics), "0.86");
    }
}
```

**JSON Serialization Tests** (`src/output/unified.rs`):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_serialize_state_machine_pattern() {
        let item = create_unified_item_with_state_machine();
        let json = serde_json::to_string(&item).unwrap();

        assert!(json.contains("\"pattern_type\":\"state_machine\""));
        assert!(json.contains("\"pattern_confidence\":0.85"));
        assert!(json.contains("\"pattern_details\""));
        assert!(json.contains("\"transition_count\":4"));
    }

    #[test]
    fn test_serialize_no_pattern_omits_fields() {
        let item = create_unified_item_without_pattern();
        let json = serde_json::to_string(&item).unwrap();

        assert!(!json.contains("pattern_type"));
        assert!(!json.contains("pattern_confidence"));
        assert!(!json.contains("pattern_details"));
    }

    #[test]
    fn test_deserialize_backward_compatibility() {
        // Old JSON without pattern fields should still parse
        let old_json = r#"{"score":50,"category":"complexity"}"#;
        let item: UnifiedDebtItemOutput = serde_json::from_str(old_json).unwrap();

        assert!(item.pattern_type.is_none());
        assert!(item.pattern_confidence.is_none());
    }
}
```

### Integration Tests

**End-to-End Output Tests** (`tests/output_format_integration_test.rs`):
```rust
#[test]
fn test_terminal_output_displays_state_machine() {
    // Analyze test file with known state machine pattern
    let test_file = "tests/fixtures/state_machine_example.rs";
    let results = analyze_file(test_file).unwrap();

    // Generate terminal output
    let mut buffer = Vec::new();
    let mut writer = TerminalWriter::new(&mut buffer);
    writer.write_results(&results).unwrap();

    let output = String::from_utf8(buffer).unwrap();

    // Verify state machine pattern is displayed
    assert!(output.contains("🔄 State Machine"));
    assert!(output.contains("transitions:"));
    assert!(output.contains("confidence:"));
}

#[test]
fn test_json_output_includes_pattern_signals() {
    let test_file = "tests/fixtures/coordinator_example.rs";
    let results = analyze_file(test_file).unwrap();

    let mut buffer = Vec::new();
    let mut writer = JsonWriter::new(&mut buffer);
    writer.write_results(&results).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&buffer).unwrap();

    // Find function with coordinator pattern
    let functions = json["files"][0]["functions"].as_array().unwrap();
    let coordinator_func = functions.iter()
        .find(|f| f["language_specific"]["Rust"]["coordinator_signals"].is_object())
        .expect("Should find coordinator pattern");

    let signals = &coordinator_func["language_specific"]["Rust"]["coordinator_signals"];
    assert!(signals["actions"].as_u64().unwrap() > 0);
    assert!(signals["comparisons"].as_u64().unwrap() > 0);
    assert!(signals["confidence"].as_f64().unwrap() >= 0.7);
}

#[test]
fn test_markdown_table_has_pattern_columns() {
    let test_file = "tests/fixtures/state_machine_example.rs";
    let results = analyze_file(test_file).unwrap();

    let mut buffer = Vec::new();
    let mut writer = MarkdownWriter::new(&mut buffer);
    writer.write_results(&results).unwrap();

    let markdown = String::from_utf8(buffer).unwrap();

    // Verify pattern columns exist
    assert!(markdown.contains("| Pattern |"));
    assert!(markdown.contains("| Confidence |"));
    assert!(markdown.contains("| State Machine |"));
}
```

### Performance Tests

```rust
#[test]
fn test_output_generation_performance() {
    let large_results = generate_large_analysis_results(1000); // 1000 functions

    // Baseline: output without pattern display
    let baseline = measure_output_time(&large_results, false);

    // Enhanced: output with pattern display
    let enhanced = measure_output_time(&large_results, true);

    // Overhead should be < 1% of total time
    let overhead_ratio = (enhanced - baseline) / baseline;
    assert!(overhead_ratio < 0.01, "Output overhead too high: {:.2}%", overhead_ratio * 100.0);
}
```

### Test Fixtures

Create test files demonstrating patterns:

**tests/fixtures/state_machine_example.rs**:
```rust
enum State { Active, Standby, Error }
enum Action { Activate, Deactivate, Reset }

fn state_transition(current: State, desired: State) -> Vec<Action> {
    let mut actions = Vec::new();
    match (current, desired) {
        (State::Standby, State::Active) => actions.push(Action::Activate),
        (State::Active, State::Standby) => actions.push(Action::Deactivate),
        (State::Error, _) => actions.push(Action::Reset),
        _ => {}
    }
    actions
}
```

**tests/fixtures/coordinator_example.rs**:
```rust
fn coordinate_actions(items: &[Item], target: &Config) -> Vec<Action> {
    let mut actions = Vec::new();
    for item in items {
        if item.state != target.desired_state {
            actions.push(Action::Transition(item.id));
        }
        if item.config != target.config {
            actions.push(Action::Reconfigure(item.id));
        }
    }
    actions
}
```

## Documentation Requirements

### Code Documentation

1. **Add docstrings to new formatting functions**:
   ```rust
   /// Extracts pattern type from function metrics.
   ///
   /// Returns pattern name if confidence >= 0.7, otherwise None.
   /// Prioritizes state machine over coordinator when both detected.
   pub fn extract_pattern_type(metrics: &FunctionMetrics) -> Option<&str>
   ```

2. **Document PatternInfo structure**:
   ```rust
   /// Information about a detected complexity pattern.
   ///
   /// Used for consistent pattern display across output formats.
   pub struct PatternInfo {
       /// The type of pattern detected
       pub pattern_type: PatternType,
       /// Detection confidence (0.0-1.0, >= 0.7 required)
       pub confidence: f64,
       /// Pattern-specific metrics for display
       pub display_metrics: HashMap<String, String>,
   }
   ```

### User Documentation

**Update README.md** or create **docs/output-formats.md**:

```markdown
## Output Formats

### Pattern Detection Display

Debtmap detects state machine and coordinator patterns in Rust code and displays
them in all output formats.

#### Terminal Output

Complexity hotspots include pattern indicators:

```
  1. detector.rs:123 detect_state_machine()
     Cyclomatic: 12, Cognitive: 18
     🔄 State Machine (transitions: 4, confidence: 0.85)
     ACTION: Consider transition table pattern
```

Pattern types:
- 🔄 **State Machine**: Functions with state transitions and match expressions
- 🎯 **Coordinator**: Functions that accumulate actions based on state comparisons

Confidence levels:
- **High** (0.8-1.0): Strong pattern detection
- **Medium** (0.7-0.8): Moderate confidence
- Patterns with confidence < 0.7 are not displayed

#### JSON Output

Function metrics include pattern signals when detected:

```json
{
  "language_specific": {
    "Rust": {
      "state_machine_signals": {
        "transition_count": 4,
        "match_expression_count": 2,
        "has_enum_match": true,
        "has_state_comparison": true,
        "action_dispatch_count": 8,
        "confidence": 0.85
      }
    }
  }
}
```

#### Markdown Output

Complexity analysis table includes pattern information:

| Location | Function | Cyclomatic | Cognitive | Pattern | Confidence | Recommendation |
|----------|----------|------------|-----------|---------|------------|----------------|
| detector.rs:123 | detect_state_machine | 12 | 18 | State Machine | 0.85 | ... |

#### Unified JSON Format

Includes top-level pattern fields for easy parsing:

```json
{
  "pattern_type": "state_machine",
  "pattern_confidence": 0.85,
  "pattern_details": {
    "transition_count": 4,
    "match_expression_count": 2,
    "action_dispatch_count": 8
  }
}
```
```

### ARCHITECTURE.md Updates

Add section on output formatting patterns:

```markdown
## Output Layer Patterns

### Pattern Display

State machine and coordinator patterns are displayed consistently across all formats:

1. **Terminal**: Visual indicators with key metrics
2. **JSON**: Full signal structures in language_specific data
3. **Markdown**: Pattern columns in complexity tables
4. **Unified**: Top-level pattern fields for tools

Implementation follows Pure Core / Imperative Shell:
- **Pure**: `extract_pattern_type()`, `format_pattern_info()` - deterministic formatters
- **Impure**: Writing to output streams, I/O operations
```

## Implementation Notes

### Implementation Order

1. **Add helper functions** for pattern extraction (pure functions)
2. **Enhance terminal output** with pattern display
3. **Extend JSON output** (verify serialization works)
4. **Update markdown tables** with pattern columns
5. **Extend unified format** schema
6. **Add tests** for all formats
7. **Update documentation**

### Code Organization

Create shared formatting module for pattern display:

```
src/io/writers/
├── pattern_display.rs       # Shared pattern formatting logic
│   ├── extract_pattern_type()
│   ├── extract_pattern_confidence()
│   ├── format_pattern_metrics()
│   └── PatternInfo struct
├── terminal.rs
├── markdown/
│   ├── core.rs
│   └── formatters.rs
└── mod.rs
```

### Refactoring Opportunities

**Extract pure formatting functions** following spec 187 principles:

```rust
// PURE: Extract pattern information
fn extract_pattern_info(metrics: &FunctionMetrics) -> Option<PatternInfo> { ... }

// PURE: Format pattern for display
fn format_pattern_display(info: &PatternInfo) -> String { ... }

// IMPURE: Write to terminal (I/O wrapper)
fn print_pattern_info(&mut self, info: &PatternInfo) -> Result<()> {
    writeln!(self.output, "{}", format_pattern_display(info))
}
```

### Edge Cases

1. **Multiple patterns detected**: Prioritize state machine over coordinator (matches detection logic)
2. **Confidence at threshold boundary**: Use >= 0.7 consistently
3. **Non-Rust languages**: Pattern fields should be None/omitted
4. **Empty function metrics**: Handle gracefully, no pattern displayed
5. **Very high confidence (> 0.95)**: Consider highlighting as high-confidence detection

## Migration and Compatibility

### Breaking Changes

**None** - This is a backward-compatible enhancement:
- New JSON fields are optional and omitted when not present
- Old markdown/JSON parsers will ignore new fields
- Terminal output is additive (more information, same structure)

### Deprecations

**None**

### Migration Path

No migration required. Users will immediately see pattern information in outputs after upgrading.

### Version Compatibility

- **Unified format version**: Increment from current to next version (document schema change)
- **JSON output**: Backward compatible (old tools ignore new fields)
- **Markdown output**: Backward compatible (new columns added to right of table)

## Success Metrics

- ✅ Terminal output displays state machine patterns with metrics
- ✅ Terminal output displays coordinator patterns with metrics
- ✅ JSON output includes full signal structures
- ✅ Markdown tables have pattern and confidence columns
- ✅ Unified format includes pattern_type, pattern_confidence, pattern_details
- ✅ All output formats tested with real state machine/coordinator examples
- ✅ Backward compatibility verified (old JSON/Markdown still parse)
- ✅ Performance overhead < 1% of total output time
- ✅ Documentation updated with pattern display examples
- ✅ Integration tests verify pattern visibility in all formats
- ✅ User can identify state machine/coordinator functions from any output format
- ✅ Confidence scores displayed consistently across formats

## Follow-up Work

After implementing this specification:

1. **Enhanced filtering**: Add CLI flags to filter/highlight patterns (e.g., `--show-only state-machine`)
2. **Pattern-specific recommendations**: Expand refactoring guidance based on pattern details
3. **Pattern evolution tracking**: Compare patterns across commits/versions
4. **Visualization**: Create visual diagrams of state transitions from pattern data
5. **Pattern library**: Build collection of common state machine/coordinator patterns
6. **IDE integration**: Export pattern information for IDE plugins/extensions
7. **Pattern complexity metrics**: Derive additional metrics from state transition graphs

## References

- **Spec 179**: State machine and coordinator pattern detection implementation
- **Spec 192**: Enhanced coordinator detection with state-awareness
- **src/analyzers/state_machine_pattern_detector.rs**: Pattern detection implementation
- **src/priority/complexity_patterns.rs**: Signal data structures (lines 24-40)
- **src/analysis/rust_patterns/detector.rs**: RustPatternResult structure
- **src/io/writers/terminal.rs**: Terminal output implementation
- **src/io/writers/markdown/core.rs**: Markdown output implementation
- **src/output/unified.rs**: Unified JSON format
- **tests/state_machine_pattern_detection_test.rs**: Integration test examples
