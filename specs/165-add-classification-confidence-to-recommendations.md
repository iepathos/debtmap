---
number: 165
title: Add Classification Confidence to Recommendations
category: optimization
priority: medium
status: draft
dependencies: [145, 150]
created: 2025-11-02
---

# Specification 165: Add Classification Confidence to Recommendations

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 145 (Multi-Signal Responsibility Aggregation), 150 (Weight Tuning)

## Context

Debtmap performs responsibility classification using multiple signals (I/O detection, call graph, type signatures, purity analysis, etc.) per Spec 145. However, **classification confidence is not visible in the output**.

**Current Output**:
```
- RECOMMENDED SPLITS (3 modules):
-  [M] rust_utilities.rs - Utilities (15 methods, ~300 lines) [Medium]
-  [M] rust_construction.rs - Construction (7 methods, ~140 lines) [Medium]
-  [M] rust_computation.rs - Computation (6 methods, ~120 lines) [Medium]
```

**Issues**:
1. No indication of classification certainty - is "Utilities" 99% confident or 51%?
2. Users cannot assess trustworthiness of recommendations
3. Low-confidence classifications ("Utilities" with 52% confidence) appear equally authoritative as high-confidence ones
4. Missing context for human review - which splits need manual verification?

**Existing Infrastructure**:

From Spec 145, responsibility classification already produces confidence scores:
```rust
// src/organization/module_function_classifier.rs
pub struct ClassificationEvidence {
    pub confidence: f64,           // 0.0 to 1.0
    pub responsibility: Responsibility,
    pub reasoning: Vec<String>,
    pub alternatives: Vec<(Responsibility, f64)>,
}
```

From `src/organization/god_object_analysis.rs:191-196`:
```rust
pub enum GodObjectConfidence {
    Definite,     // Exceeds all thresholds
    Probable,     // Exceeds most thresholds
    Possible,     // Exceeds some thresholds
    NotGodObject, // Within acceptable limits
}
```

**Problem**: This confidence data is **computed but not displayed** to users.

## Objective

Make classification confidence visible in debtmap output to help users:
1. **Assess trustworthiness** of split recommendations
2. **Prioritize manual review** for low-confidence classifications
3. **Understand uncertainty** in responsibility assignments
4. **Validate** that weight tuning (Spec 150) is working effectively

## Requirements

### Functional Requirements

**1. Display Confidence in Split Recommendations**

Add confidence indicators to each `ModuleSplit` in the output:

```
- RECOMMENDED SPLITS (3 modules):
-  [M] rust_utilities.rs - Utilities (15 methods, ~300 lines) [Medium] [Confidence: 52%] ⚠
-  [H] rust_construction.rs - Construction (7 methods, ~140 lines) [High] [Confidence: 91%]
-  [H] rust_computation.rs - Computation (6 methods, ~120 lines) [High] [Confidence: 87%]
```

**Visual Indicators**:
- **≥90% confidence**: No indicator (high trust)
- **70-89% confidence**: No indicator (moderate trust)
- **50-69% confidence**: `⚠` warning symbol (review recommended)
- **<50% confidence**: `⚠⚠` double warning (manual classification needed)

**2. Confidence-Aware Priority Adjustment**

Downgrade priority for low-confidence splits:
- High priority + Low confidence (< 70%) → Medium priority
- Medium priority + Low confidence (< 50%) → Low priority
- Add warning message for splits downgraded due to low confidence

**3. Classification Evidence Display (Verbosity Mode)**

With `--verbose` flag, show detailed classification reasoning:

```
-  [M] rust_utilities.rs - Utilities (15 methods, ~300 lines) [Medium] [Confidence: 52%] ⚠
     Classification Evidence:
       • I/O patterns: 0.30 (weak)
       • Call graph analysis: 0.45 (medium)
       • Type signatures: 0.60 (medium)
       • Purity analysis: 0.70 (strong)
       • Aggregate confidence: 0.52
     Alternative classifications considered:
       • Data Access: 0.48
       • Transformation: 0.45
     ⚠ Low confidence - consider manual review
```

**4. Summary Statistics**

Add confidence statistics to overall analysis summary:

```
TOTAL DEBT SCORE: 1793
DEBT DENSITY: 14.8 per 1K LOC (120898 total LOC)
OVERALL COVERAGE: 81.14%

CLASSIFICATION CONFIDENCE:
  High confidence (≥90%): 45 splits (45%)
  Moderate confidence (70-89%): 35 splits (35%)
  Low confidence (50-69%): 15 splits (15%) ⚠
  Very low confidence (<50%): 5 splits (5%) ⚠⚠
```

### Non-Functional Requirements

- **Performance**: Negligible impact (confidence already computed, just needs display)
- **Backward Compatibility**: Default output minimally changed (add confidence only for low values)
- **JSON Output**: Include confidence in JSON for programmatic analysis
- **Color Support**: Use colors to highlight warnings (yellow for ⚠, red for ⚠⚠)

## Acceptance Criteria

- [ ] Each `ModuleSplit` in output displays confidence percentage when < 90%
- [ ] Low confidence splits (<70%) show `⚠` warning indicator
- [ ] Very low confidence splits (<50%) show `⚠⚠` double warning
- [ ] Priority automatically downgraded for low-confidence classifications
- [ ] `--verbose` mode shows full classification evidence including reasoning and alternatives
- [ ] Summary section shows confidence distribution across all recommendations
- [ ] JSON output includes `confidence` field in each `ModuleSplit`
- [ ] Color-coded warnings (yellow/red) when terminal supports colors
- [ ] Documentation explains how to interpret confidence scores
- [ ] Integration tests verify confidence display logic

## Technical Details

### Implementation Approach

**Phase 1: Data Structure Updates**

Update `ModuleSplit` to store confidence:
```rust
// src/organization/god_object_analysis.rs
pub struct ModuleSplit {
    pub suggested_name: String,
    pub methods_to_move: Vec<String>,
    pub structs_to_move: Vec<String>,
    pub responsibility: String,
    pub estimated_lines: usize,
    pub method_count: usize,
    pub warning: Option<String>,
    pub priority: Priority,
    pub cohesion_score: Option<f64>,

    // NEW: Classification confidence (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,

    // NEW: Detailed classification evidence (for verbose mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_evidence: Option<ClassificationEvidence>,
}
```

**Phase 2: Confidence Propagation**

Ensure confidence flows through the analysis pipeline:

1. `module_function_classifier.rs` already produces `ClassificationEvidence` with confidence
2. Store confidence when creating `ModuleSplit`:
   ```rust
   let split = ModuleSplit {
       suggested_name: format!("{}_module", responsibility),
       responsibility: responsibility.to_string(),
       methods_to_move: functions.iter().map(|f| f.name.clone()).collect(),
       confidence: Some(evidence.confidence),
       classification_evidence: Some(evidence),
       ...
   };
   ```

**Phase 3: Priority Adjustment**

Add confidence-aware priority adjustment:
```rust
fn adjust_priority_for_confidence(
    priority: Priority,
    confidence: f64
) -> Priority {
    match (priority, confidence) {
        (Priority::High, c) if c < 0.70 => Priority::Medium,
        (Priority::Medium, c) if c < 0.50 => Priority::Low,
        _ => priority,
    }
}
```

**Phase 4: Formatter Updates**

Update `src/priority/formatter.rs` to display confidence:

```rust
fn format_split_with_confidence(
    split: &ModuleSplit,
    verbosity: u8,
) -> String {
    let mut output = String::new();

    // Base split info
    write!(output, "  - [{}] {} - {} ({} methods, ~{} lines)",
        priority_indicator(split.priority),
        split.suggested_name,
        split.responsibility,
        split.method_count,
        split.estimated_lines
    );

    // Add confidence if available and below threshold
    if let Some(confidence) = split.confidence {
        if confidence < 0.90 {
            let confidence_pct = (confidence * 100.0) as u8;
            write!(output, " [Confidence: {}%]", confidence_pct);

            // Add warning indicators
            if confidence < 0.50 {
                write!(output, " ⚠⚠");
            } else if confidence < 0.70 {
                write!(output, " ⚠");
            }
        }
    }

    writeln!(output);

    // Verbose mode: show detailed evidence
    if verbosity > 0 {
        if let Some(ref evidence) = split.classification_evidence {
            output.push_str(&format_classification_evidence(evidence));
        }
    }

    output
}
```

**Phase 5: Summary Statistics**

Add confidence summary calculation:
```rust
fn calculate_confidence_summary(splits: &[ModuleSplit]) -> ConfidenceSummary {
    let mut high = 0;
    let mut moderate = 0;
    let mut low = 0;
    let mut very_low = 0;

    for split in splits {
        if let Some(conf) = split.confidence {
            match conf {
                c if c >= 0.90 => high += 1,
                c if c >= 0.70 => moderate += 1,
                c if c >= 0.50 => low += 1,
                _ => very_low += 1,
            }
        }
    }

    ConfidenceSummary { high, moderate, low, very_low }
}
```

### Architecture Changes

**New Types**:
```rust
pub struct ConfidenceSummary {
    pub high_confidence: usize,      // ≥90%
    pub moderate_confidence: usize,  // 70-89%
    pub low_confidence: usize,       // 50-69%
    pub very_low_confidence: usize,  // <50%
}

pub enum ConfidenceLevel {
    High,      // ≥90%
    Moderate,  // 70-89%
    Low,       // 50-69%
    VeryLow,   // <50%
}
```

### Data Format Changes

**JSON Output** (with confidence):
```json
{
  "recommended_splits": [
    {
      "suggested_name": "rust_utilities",
      "responsibility": "Utilities",
      "method_count": 15,
      "estimated_lines": 300,
      "priority": "Medium",
      "confidence": 0.52,
      "classification_evidence": {
        "confidence": 0.52,
        "responsibility": "Utilities",
        "reasoning": [
          "I/O patterns: weak (0.30)",
          "Call graph: medium (0.45)",
          "Type signatures: medium (0.60)",
          "Purity: strong (0.70)"
        ],
        "alternatives": [
          { "responsibility": "DataAccess", "confidence": 0.48 },
          { "responsibility": "Transformation", "confidence": 0.45 }
        ]
      }
    }
  ]
}
```

## Dependencies

- **Prerequisites**:
  - Spec 145: Multi-Signal Responsibility Aggregation (provides `ClassificationEvidence`)
  - Spec 150: Weight Tuning (improves confidence scores)
- **Affected Components**:
  - `src/organization/god_object_analysis.rs` (ModuleSplit struct)
  - `src/organization/module_function_classifier.rs` (confidence propagation)
  - `src/priority/formatter.rs` (display logic)
  - `src/output/json.rs` (JSON serialization)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_confidence_display_thresholds() {
    assert_eq!(format_confidence(0.95), "");  // High, no display
    assert_eq!(format_confidence(0.85), "[Confidence: 85%]");
    assert_eq!(format_confidence(0.65), "[Confidence: 65%] ⚠");
    assert_eq!(format_confidence(0.45), "[Confidence: 45%] ⚠⚠");
}

#[test]
fn test_priority_adjustment() {
    assert_eq!(adjust_priority(Priority::High, 0.95), Priority::High);
    assert_eq!(adjust_priority(Priority::High, 0.65), Priority::Medium);
    assert_eq!(adjust_priority(Priority::Medium, 0.45), Priority::Low);
}

#[test]
fn test_confidence_summary_calculation() {
    let splits = vec![
        make_split(0.95),  // High
        make_split(0.85),  // Moderate
        make_split(0.65),  // Low
        make_split(0.45),  // Very low
    ];

    let summary = calculate_confidence_summary(&splits);
    assert_eq!(summary.high_confidence, 1);
    assert_eq!(summary.moderate_confidence, 1);
    assert_eq!(summary.low_confidence, 1);
    assert_eq!(summary.very_low_confidence, 1);
}
```

### Integration Tests

```rust
#[test]
fn test_confidence_in_formatted_output() {
    let analysis = analyze_test_file("tests/fixtures/mixed_responsibilities.rs");
    let output = format_priorities(&analysis, OutputFormat::Default);

    // High confidence splits should not show confidence
    assert!(!output.contains("[Confidence: 95%]"));

    // Low confidence splits should show warning
    assert!(output.contains("[Confidence: 65%] ⚠"));
    assert!(output.contains("[Confidence: 45%] ⚠⚠"));
}

#[test]
fn test_verbose_evidence_display() {
    let analysis = analyze_test_file("tests/fixtures/utilities.rs");
    let output = format_priorities_with_verbosity(&analysis, OutputFormat::Default, 1);

    // Verbose mode should show evidence
    assert!(output.contains("Classification Evidence:"));
    assert!(output.contains("I/O patterns:"));
    assert!(output.contains("Alternative classifications considered:"));
}

#[test]
fn test_confidence_in_json_output() {
    let analysis = analyze_test_file("tests/fixtures/utilities.rs");
    let json = to_json(&analysis);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let split = &parsed["recommended_splits"][0];
    assert!(split["confidence"].is_number());
    assert!(split["confidence"].as_f64().unwrap() >= 0.0);
    assert!(split["confidence"].as_f64().unwrap() <= 1.0);
}
```

### Manual Testing

```bash
# Test default output
cargo run -- analyze src/ --format default

# Test verbose output
cargo run -- analyze src/ --format default --verbose

# Test JSON output includes confidence
cargo run -- analyze src/ --format json | jq '.recommended_splits[].confidence'

# Test with known low-confidence case
cargo run -- analyze tests/fixtures/utilities.rs
```

## Documentation Requirements

### Code Documentation

Add doc comments explaining confidence:
```rust
/// Classification confidence score (0.0 to 1.0) indicating certainty of responsibility assignment.
///
/// Thresholds:
/// - ≥0.90: High confidence (no indicator shown)
/// - 0.70-0.89: Moderate confidence (shown but no warning)
/// - 0.50-0.69: Low confidence (⚠ warning, manual review recommended)
/// - <0.50: Very low confidence (⚠⚠ warning, manual classification needed)
///
/// Low confidence may indicate:
/// - Mixed responsibilities (function does multiple things)
/// - Weak or conflicting classification signals
/// - Edge case not well-handled by classifier
/// - Need for weight tuning (see Spec 150)
pub confidence: Option<f64>,
```

### User Documentation

Add section to user guide:

**Understanding Classification Confidence**

Debtmap assigns a confidence score to each responsibility classification:

- **High (≥90%)**: Strong signal agreement, trust the classification
- **Moderate (70-89%)**: Reasonable confidence, generally reliable
- **Low (50-69%)**: Weak signals, marked with ⚠, consider manual review
- **Very Low (<50%)**: Conflicting signals, marked with ⚠⚠, requires manual classification

Use `--verbose` to see detailed classification evidence and alternative classifications considered.

## Implementation Notes

### Gotchas

1. **Not all splits have confidence** - Struct-based domain splits (Spec 140) may not have confidence scores
   - Solution: Make `confidence` field optional
2. **Color support detection** - Terminal color support varies
   - Use `colored` crate's automatic detection
3. **JSON consumers** - External tools may need updates if they parse JSON
   - Add confidence as optional field for backward compatibility

### Best Practices

- Only show confidence when it adds value (< 90%)
- Make warnings visually distinct but not alarming
- Provide actionable guidance ("manual review recommended")
- In verbose mode, explain *why* confidence is low

## Migration and Compatibility

### Breaking Changes

**Minor JSON format change**: Adds optional `confidence` and `classification_evidence` fields.

**Impact**: Low - new fields are optional and backward compatible.

### Migration Strategy

No migration needed. New fields default to `None` for backwards compatibility.

## Implementation Order

1. **Add `confidence` field to `ModuleSplit`** with optional serialization
2. **Propagate confidence** from `ClassificationEvidence` to `ModuleSplit` creation
3. **Implement priority adjustment** based on confidence
4. **Update formatter** to display confidence with thresholds and warnings
5. **Add verbose evidence display** with detailed reasoning
6. **Implement confidence summary** for overall statistics
7. **Update tests** to verify confidence display logic
8. **Add user documentation** explaining confidence interpretation

## Related Specifications

- **Spec 145**: Multi-Signal Responsibility Aggregation (provides confidence infrastructure)
- **Spec 150**: Weight Tuning for Utilities Reduction (improves confidence scores through optimization)
- **Spec 151**: Purity and Framework Pattern Indicators (another form of evidence to display)
- **Spec 164**: Fix Duplicate Extensions (formatting improvement, similar goal)

## Success Metrics

- Users can quickly identify low-confidence classifications requiring review
- Confidence information helps validate weight tuning effectiveness (Spec 150)
- Verbose mode provides enough detail to understand classification reasoning
- JSON consumers can programmatically filter by confidence threshold
- Reduced false confidence in ambiguous classifications
- Improved trust in high-confidence recommendations
