---
number: 148
title: Multi-Signal Evidence Display in Output
category: optimization
priority: critical
status: draft
dependencies: [145]
created: 2025-10-28
---

# Specification 148: Multi-Signal Evidence Display in Output

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 145 (Multi-Signal Responsibility Aggregation)

## Context

Spec 145 (Multi-Signal Responsibility Aggregation) has been implemented and is working internally, combining I/O detection, call graph analysis, purity analysis, framework patterns, type signatures, and name heuristics to produce high-accuracy responsibility classifications.

However, the **explainability layer is missing** from the user-facing output. Users see recommendations like:

```
RECOMMENDED SPLITS (3 modules):
- [M] python_type_tracker_parsing_&_input.rs - Parsing & Input (6 methods)
```

But they **cannot see**:
- Why was this classified as "Parsing & Input"?
- Which signals contributed to this classification?
- How confident is the classification?
- What evidence supports this recommendation?

This lack of transparency reduces user trust and makes it difficult to understand why certain recommendations were made. When users see "Utilities" groupings, they can't tell if it's a genuine utility category or a catch-all for uncertain classifications.

## Objective

Add comprehensive multi-signal evidence display to debtmap's output, showing users:
1. Which signals contributed to each classification
2. Individual signal confidences and weights
3. Combined confidence score
4. Specific evidence from each signal
5. Alternative classifications (if close)

This will increase user trust, enable debugging of classification quality, and provide actionable insights for refactoring.

## Requirements

### Functional Requirements

**Signal Evidence Display**:
- Show all contributing signals for each classification
- Display individual signal confidence scores (0.0-1.0)
- Show signal weights used in aggregation
- Calculate and display weighted contribution (confidence × weight)
- Show specific evidence from each signal (e.g., "Calls 5 functions, orchestrator pattern")

**Confidence Visualization**:
- Display overall classification confidence (0.0-1.0)
- Use visual indicators (✓, •, ✗) for signal availability
- Show confidence bands: HIGH (>0.80), MEDIUM (0.60-0.80), LOW (<0.60)
- Highlight low-confidence classifications for user review

**Alternative Classifications**:
- Show top 2-3 alternative classifications if scores are close
- Display score difference between primary and alternatives
- Indicate when classification is ambiguous (alternatives within 0.10 of primary)

**Evidence Formatting**:
- Concise, user-friendly evidence descriptions
- Technical details available in verbose mode
- Links to relevant code patterns or structures
- Examples from analyzed code

**Integration Points**:
- Module split recommendations (#1-#9 in latest output)
- Individual function classifications (item #10)
- Summary statistics (overall classification accuracy)

### Non-Functional Requirements

- **Performance**: Evidence formatting adds <2% to output generation time
- **Readability**: Evidence fits within 80-column terminal output
- **Verbosity Levels**: Support minimal, standard, and verbose output modes
- **Consistency**: Same evidence format across all recommendation types

## Acceptance Criteria

- [ ] Module split recommendations show signal evidence for each proposed module
- [ ] Individual function classifications (#10) show contributing signals
- [ ] Confidence scores are displayed with visual indicators
- [ ] Signal weights are shown (configurable, default to Spec 145 weights)
- [ ] Specific evidence is shown for each signal (not just category names)
- [ ] Alternative classifications shown when confidence < 0.80
- [ ] "Utilities" classifications show why they couldn't be more specific
- [ ] Verbose mode provides technical details (AST patterns, metrics)
- [ ] Performance overhead <2% for evidence formatting
- [ ] Test suite includes output formatting tests

## Technical Details

### Implementation Approach

**Phase 1: Evidence Data Structure**

```rust
#[derive(Debug, Clone)]
pub struct ClassificationEvidence {
    pub primary: ResponsibilityCategory,
    pub confidence: f64,
    pub signals: Vec<SignalContribution>,
    pub alternatives: Vec<AlternativeClassification>,
}

#[derive(Debug, Clone)]
pub struct SignalContribution {
    pub signal_type: SignalType,
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub weight: f64,
    pub contribution: f64,  // confidence * weight
    pub evidence: String,
    pub is_available: bool,
}

#[derive(Debug, Clone)]
pub enum SignalType {
    IoDetection,
    CallGraph,
    TypeSignatures,
    Purity,
    Framework,
    RustPatterns,
    Name,
}

#[derive(Debug, Clone)]
pub struct AlternativeClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub difference: f64,  // Difference from primary
}

impl ClassificationEvidence {
    pub fn confidence_band(&self) -> ConfidenceBand {
        match self.confidence {
            c if c >= 0.80 => ConfidenceBand::High,
            c if c >= 0.60 => ConfidenceBand::Medium,
            _ => ConfidenceBand::Low,
        }
    }

    pub fn is_ambiguous(&self) -> bool {
        self.alternatives.first()
            .map(|alt| alt.difference < 0.10)
            .unwrap_or(false)
    }
}
```

**Phase 2: Evidence Formatter**

```rust
pub struct EvidenceFormatter {
    verbosity: VerbosityLevel,
    color_enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum VerbosityLevel {
    Minimal,   // Only show confidence and primary category
    Standard,  // Show signal summary (default)
    Verbose,   // Show all details including technical evidence
}

impl EvidenceFormatter {
    pub fn format_evidence(&self, evidence: &ClassificationEvidence) -> String {
        let mut output = String::new();

        match self.verbosity {
            VerbosityLevel::Minimal => {
                output.push_str(&format!(
                    "Classification: {} [{:.0}% confidence]\n",
                    evidence.primary,
                    evidence.confidence * 100.0
                ));
            }
            VerbosityLevel::Standard => {
                self.format_standard_evidence(&mut output, evidence);
            }
            VerbosityLevel::Verbose => {
                self.format_verbose_evidence(&mut output, evidence);
            }
        }

        output
    }

    fn format_standard_evidence(&self, output: &mut String, evidence: &ClassificationEvidence) {
        // Header
        output.push_str(&format!(
            "\nCLASSIFICATION ANALYSIS:\n\
             Primary: {} [Confidence: {:.2}, {}]\n\n",
            evidence.primary,
            evidence.confidence,
            self.format_confidence_band(evidence.confidence_band())
        ));

        // Contributing signals
        output.push_str("Contributing Signals:\n");
        for signal in &evidence.signals {
            if !signal.is_available {
                output.push_str(&format!(
                    "  • {:?}: N/A\n",
                    signal.signal_type
                ));
                continue;
            }

            let indicator = if signal.contribution > 0.15 {
                "✓"
            } else if signal.contribution > 0.05 {
                "•"
            } else {
                "-"
            };

            output.push_str(&format!(
                "  {} {:?} ({:.0}% conf, {:.0}% weight) = {:.3} contribution\n\
                     Evidence: {}\n",
                indicator,
                signal.signal_type,
                signal.confidence * 100.0,
                signal.weight * 100.0,
                signal.contribution,
                signal.evidence
            ));
        }

        // Alternatives if ambiguous
        if evidence.is_ambiguous() {
            output.push_str("\nAlternative Classifications:\n");
            for alt in &evidence.alternatives {
                output.push_str(&format!(
                    "  - {} ({:.2} confidence, Δ{:.2})\n",
                    alt.category,
                    alt.confidence,
                    alt.difference
                ));
            }
            output.push_str("  ⚠ Classification is ambiguous - consider manual review\n");
        }

        // Total score
        output.push_str(&format!(
            "\nWeighted Score: {:.3}\n",
            evidence.confidence
        ));
    }

    fn format_verbose_evidence(&self, output: &mut String, evidence: &ClassificationEvidence) {
        self.format_standard_evidence(output, evidence);

        output.push_str("\nDETAILED BREAKDOWN:\n");

        for signal in &evidence.signals {
            if !signal.is_available {
                continue;
            }

            output.push_str(&format!(
                "\n{:?} Signal:\n\
                   Category: {}\n\
                   Confidence: {:.4}\n\
                   Weight: {:.4}\n\
                   Contribution: {:.4}\n\
                   Evidence: {}\n",
                signal.signal_type,
                signal.category,
                signal.confidence,
                signal.weight,
                signal.contribution,
                signal.evidence
            ));
        }
    }

    fn format_confidence_band(&self, band: ConfidenceBand) -> &str {
        match band {
            ConfidenceBand::High => "HIGH",
            ConfidenceBand::Medium => "MEDIUM",
            ConfidenceBand::Low => "LOW",
        }
    }
}
```

**Phase 3: Integration with Recommendation Output**

```rust
// In src/priority/formatter.rs (or new evidence module)

impl RecommendationFormatter {
    pub fn format_module_split_recommendation(
        &self,
        recommendation: &ModuleSplitRecommendation,
        evidence_map: &HashMap<String, ClassificationEvidence>,
    ) -> String {
        let mut output = String::new();

        // Existing recommendation format
        output.push_str(&format!("{}\n", recommendation.summary));

        // NEW: Add evidence for each proposed module
        output.push_str("\n  - CLASSIFICATION EVIDENCE:\n");

        for split in &recommendation.proposed_modules {
            if let Some(evidence) = evidence_map.get(&split.name) {
                output.push_str(&format!(
                    "  - Module: {} (Confidence: {:.2})\n",
                    split.name,
                    evidence.confidence
                ));

                // Show top signals
                let top_signals: Vec<_> = evidence.signals.iter()
                    .filter(|s| s.is_available && s.contribution > 0.05)
                    .take(3)
                    .collect();

                for signal in top_signals {
                    output.push_str(&format!(
                        "    • {:?}: {} ({:.0}%)\n",
                        signal.signal_type,
                        signal.evidence.split('\n').next().unwrap_or(""),
                        signal.confidence * 100.0
                    ));
                }

                if evidence.is_ambiguous() {
                    output.push_str(&format!(
                        "    ⚠ Alternative: {} ({:.2})\n",
                        evidence.alternatives[0].category,
                        evidence.alternatives[0].confidence
                    ));
                }
            }
        }

        output
    }
}
```

**Phase 4: "Utilities" Catch-All Explanation**

When classification confidence is low or "Utilities" is assigned:

```rust
impl EvidenceFormatter {
    pub fn explain_utilities_classification(&self, evidence: &ClassificationEvidence) -> String {
        let mut reasons = Vec::new();

        // Analyze why we couldn't be more specific
        if evidence.signals.iter().all(|s| !s.is_available || s.confidence < 0.50) {
            reasons.push("No strong signals detected - needs manual review");
        }

        let io_signal = evidence.signals.iter()
            .find(|s| matches!(s.signal_type, SignalType::IoDetection));

        if let Some(io) = io_signal {
            if io.confidence < 0.60 {
                reasons.push("I/O pattern unclear - mixed operations");
            }
        }

        let call_graph = evidence.signals.iter()
            .find(|s| matches!(s.signal_type, SignalType::CallGraph));

        if let Some(cg) = call_graph {
            if cg.confidence < 0.60 {
                reasons.push("No clear structural pattern - neither orchestrator nor leaf");
            }
        }

        if evidence.alternatives.len() >= 2 &&
           evidence.alternatives[0].difference < 0.15 {
            reasons.push("Multiple responsibilities detected - may need further splitting");
        }

        format!(
            "  ⚠ Classified as 'Utilities' because:\n{}",
            reasons.iter()
                .map(|r| format!("    - {}", r))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}
```

### Architecture Changes

**New Module**: `src/output/evidence_formatter.rs`
- Evidence display formatting
- Verbosity level handling
- Confidence band visualization
- Alternative classification display

**Modified Module**: `src/priority/formatter.rs`
- Integrate evidence display into recommendations
- Add verbosity configuration option
- Format signal contributions

**New Configuration**: Add to `DebtmapConfig`:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct OutputConfig {
    pub show_evidence: bool,          // Default: true
    pub evidence_verbosity: VerbosityLevel,  // Default: Standard
    pub show_alternatives: bool,      // Default: true when ambiguous
    pub min_confidence_warning: f64,  // Default: 0.60
}
```

### Data Structures

See Phase 1 above for main data structures.

## Dependencies

- **Prerequisites**: Spec 145 (Multi-Signal Aggregation) - provides ClassificationEvidence
- **Affected Components**:
  - `src/priority/formatter.rs` - output formatting
  - `src/organization/god_object_detector.rs` - recommendation generation
  - `src/config.rs` - configuration for evidence display
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_standard_evidence() {
        let evidence = ClassificationEvidence {
            primary: ResponsibilityCategory::Parsing,
            confidence: 0.85,
            signals: vec![
                SignalContribution {
                    signal_type: SignalType::IoDetection,
                    category: ResponsibilityCategory::FileIO,
                    confidence: 0.90,
                    weight: 0.40,
                    contribution: 0.36,
                    evidence: "Reads file content".into(),
                    is_available: true,
                },
                SignalContribution {
                    signal_type: SignalType::TypeSignatures,
                    category: ResponsibilityCategory::Parsing,
                    confidence: 0.85,
                    weight: 0.15,
                    contribution: 0.1275,
                    evidence: "Matches parser pattern: &str → Result<T, E>".into(),
                    is_available: true,
                },
            ],
            alternatives: vec![],
        };

        let formatter = EvidenceFormatter::new(VerbosityLevel::Standard);
        let output = formatter.format_evidence(&evidence);

        assert!(output.contains("Primary: Parsing"));
        assert!(output.contains("Confidence: 0.85"));
        assert!(output.contains("IoDetection"));
        assert!(output.contains("Reads file content"));
    }

    #[test]
    fn show_alternatives_when_ambiguous() {
        let evidence = ClassificationEvidence {
            primary: ResponsibilityCategory::Utilities,
            confidence: 0.65,
            signals: vec![],
            alternatives: vec![
                AlternativeClassification {
                    category: ResponsibilityCategory::Validation,
                    confidence: 0.62,
                    difference: 0.03,
                },
                AlternativeClassification {
                    category: ResponsibilityCategory::Transformation,
                    confidence: 0.58,
                    difference: 0.07,
                },
            ],
        };

        let formatter = EvidenceFormatter::new(VerbosityLevel::Standard);
        let output = formatter.format_evidence(&evidence);

        assert!(output.contains("Alternative Classifications"));
        assert!(output.contains("Validation"));
        assert!(output.contains("ambiguous"));
    }

    #[test]
    fn explain_utilities_low_confidence() {
        let evidence = ClassificationEvidence {
            primary: ResponsibilityCategory::Utilities,
            confidence: 0.45,
            signals: vec![
                SignalContribution {
                    signal_type: SignalType::IoDetection,
                    confidence: 0.40,
                    is_available: true,
                    ..Default::default()
                },
            ],
            alternatives: vec![],
        };

        let formatter = EvidenceFormatter::new(VerbosityLevel::Standard);
        let explanation = formatter.explain_utilities_classification(&evidence);

        assert!(explanation.contains("I/O pattern unclear"));
    }
}
```

### Integration Tests

```rust
#[test]
fn evidence_display_in_full_output() {
    let analysis_results = analyze_project("tests/fixtures/test_project");
    let formatted_output = format_full_output(&analysis_results);

    // Should contain evidence sections
    assert!(formatted_output.contains("CLASSIFICATION ANALYSIS"));
    assert!(formatted_output.contains("Contributing Signals"));

    // Should show confidence scores
    assert!(formatted_output.contains("confidence"));

    // Should show at least one signal type
    assert!(
        formatted_output.contains("IoDetection") ||
        formatted_output.contains("CallGraph") ||
        formatted_output.contains("TypeSignatures")
    );
}
```

## Documentation Requirements

### User Documentation

Update README.md:
```markdown
## Understanding Classifications

Debtmap uses multi-signal analysis to classify code responsibilities. Each
recommendation includes:

**Signal Evidence**:
- Which signals contributed to the classification
- Individual confidence scores and weights
- Specific evidence from your code

**Confidence Levels**:
- HIGH (>80%): Strong classification, high trust
- MEDIUM (60-80%): Good classification, reasonable confidence
- LOW (<60%): Uncertain classification, review recommended

**Alternative Classifications**:
- Shown when classification is ambiguous
- Indicates functions/modules with mixed responsibilities

**Verbosity Modes**:
```bash
debtmap                           # Standard evidence (default)
debtmap --output-verbosity minimal  # Minimal (confidence only)
debtmap --output-verbosity verbose  # Verbose (all technical details)
```
```

### Example Output

```
#1 SCORE: 155 [CRITICAL - FILE - GOD OBJECT]
└─ ./src/analysis/python_type_tracker.rs (3093 lines, 102 functions)
└─ ACTION: Split by data flow...

  - RECOMMENDED SPLITS (3 modules):

  - [M] python_type_tracker_parsing_&_input.rs - Parsing & Input (6 methods)

    CLASSIFICATION ANALYSIS:
    Primary: Parsing & Input [Confidence: 0.82, HIGH]

    Contributing Signals:
      ✓ IoDetection (85% conf, 40% weight) = 0.340
        Evidence: Reads file content via std::fs::read_to_string

      ✓ TypeSignatures (80% conf, 15% weight) = 0.120
        Evidence: Parser pattern: &str → Result<T, ParseError>

      ✓ CallGraph (65% conf, 30% weight) = 0.195
        Evidence: Orchestrates 3 parsing functions

      • Purity (40% conf, 10% weight) = 0.040
        Evidence: Impure (I/O operations detected)

      - Name (45% conf, 5% weight) = 0.023
        Evidence: Contains "extract" prefix

    Weighted Score: 0.718 → 0.82 (framework override applied)

  - [M] python_type_tracker_utilities.rs - Utilities (20 methods)

    CLASSIFICATION ANALYSIS:
    Primary: Utilities [Confidence: 0.48, LOW]

    ⚠ Classified as 'Utilities' because:
      - No strong signals detected - needs manual review
      - No clear structural pattern - neither orchestrator nor leaf
      - Multiple responsibilities detected - may need further splitting

    Alternative Classifications:
      - Validation (0.45 confidence, Δ0.03)
      - Transformation (0.42 confidence, Δ0.06)

    ⚠ Classification is ambiguous - consider manual review
```

## Implementation Notes

### Performance Optimization

Evidence formatting can be expensive for large projects. Optimize:

```rust
// Lazy evaluation - only format when displaying
pub struct LazyEvidence {
    evidence: OnceCell<String>,
    data: Arc<ClassificationEvidence>,
}

impl LazyEvidence {
    pub fn format(&self, verbosity: VerbosityLevel) -> &str {
        self.evidence.get_or_init(|| {
            EvidenceFormatter::new(verbosity).format_evidence(&self.data)
        })
    }
}
```

### Configuration Example

`debtmap.toml`:
```toml
[output.evidence]
show_evidence = true
verbosity = "standard"  # minimal | standard | verbose
show_alternatives = true
min_confidence_warning = 0.60

[output.evidence.signals]
show_io_detection = true
show_call_graph = true
show_type_signatures = true
show_purity = true
show_framework = true
show_name_heuristics = false  # Hide low-weight fallback
```

## Migration and Compatibility

### Backward Compatibility

- **Default behavior**: Show evidence in standard mode (new default)
- **Legacy flag**: `--no-evidence` to disable and get old output format
- **Gradual rollout**: Evidence shown but optional in v0.3.2, mandatory in v0.4.0

### Migration Path

1. **Phase 1**: Add evidence as optional output section (this spec)
2. **Phase 2**: Make evidence default, add `--no-evidence` flag
3. **Phase 3**: Deprecate `--no-evidence`, evidence always shown

## Expected Impact

### User Benefits

- **Transparency**: Users understand why classifications were made
- **Trust**: Confidence scores indicate reliability
- **Debugging**: Can identify misclassifications and report issues
- **Learning**: Understand code patterns and responsibility separation

### Quality Improvements

- **Identify weak classifications**: Low confidence exposes poor categorization
- **Tune weights**: See which signals are most/least effective
- **Improve accuracy**: User feedback on evidence quality
- **Reduce "Utilities" catch-all**: Make ambiguity explicit

### Example Impact

**Before (current output)**:
```
- [M] python_type_tracker_utilities.rs - Utilities (20 methods)
```
User thinks: "Why utilities? That's not helpful."

**After (with evidence)**:
```
- [M] python_type_tracker_utilities.rs - Utilities (20 methods)

  ⚠ Classified as 'Utilities' because:
    - No strong signals detected - needs manual review
    - Multiple responsibilities: Validation (0.45), Transformation (0.42)

  Recommendation: Split further into validation.rs and transformation.rs
```
User thinks: "Ah, makes sense - the module is too mixed for clear classification."

## Success Metrics

- [ ] 100% of recommendations include classification evidence
- [ ] User feedback: >80% find evidence helpful
- [ ] Low confidence classifications (<0.60) are flagged in output
- [ ] "Utilities" groupings include explanation of why they're unclear
- [ ] Performance overhead <2% for evidence formatting
- [ ] Zero user confusion about why classifications were made (via surveys/issues)
