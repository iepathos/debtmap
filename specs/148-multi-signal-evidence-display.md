---
number: 148
title: Multi-Signal Evidence Display in Output
category: optimization
priority: critical
status: draft
dependencies: [145]
created: 2025-10-28
updated: 2025-11-02
---

# Specification 148: Multi-Signal Evidence Display in Output

**Category**: optimization
**Priority**: critical
**Status**: draft (clarified)
**Dependencies**: Spec 145 (Multi-Signal Responsibility Aggregation)

## Clarifications Made (2025-11-02)

This spec has been updated with the following clarifications based on implementation review:

1. **Structured Evidence Types**: Changed `SignalContribution.evidence` from free-form `String` to typed `SignalEvidence` enum for consistency and type safety

2. **Alternative Classification Algorithm**: Documented how alternatives are computed from `all_scores` with explicit threshold logic

3. **Spec 145 Integration**: Added detailed integration requirements, including required changes to `MultiSignalClassifier::classify_method()` return type

4. **Functional Composition**: Refactored formatter to use pure function composition instead of mutable string building, aligning with project FP principles

5. **Configuration Precedence**: Documented CLI flags > Environment > TOML > defaults hierarchy with complete Rust and TOML examples

6. **Performance Optimization**: Added lazy evaluation as default implementation strategy with `OnceLock`, not just optimization

7. **Benchmarking Requirements**: Added concrete performance benchmarks with targets and baseline measurement requirements

8. **Implementation Plan**: Added detailed 6-phase implementation plan with time estimates and acceptance criteria

All changes follow debtmap's functional programming guidelines.

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
    pub all_scores: Vec<(ResponsibilityCategory, f64)>,  // All category scores, sorted descending
}

#[derive(Debug, Clone)]
pub struct SignalContribution {
    pub signal_type: SignalType,
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub weight: f64,
    pub contribution: f64,  // confidence * weight
    pub evidence: SignalEvidence,  // Structured evidence instead of String
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

/// Structured evidence types for type-safe formatting
#[derive(Debug, Clone)]
pub enum SignalEvidence {
    IoDetection {
        operations: Vec<IoOperation>,
        primary_type: IoType,
    },
    CallGraph {
        calls_count: usize,
        pattern: StructuralPattern,
    },
    TypeSignature {
        pattern: TypePattern,
        confidence_reason: Cow<'static, str>,
    },
    Purity {
        is_pure: bool,
        impurity_reasons: Vec<ImpurityReason>,
    },
    Framework {
        framework_type: FrameworkType,
        pattern_matched: Cow<'static, str>,
    },
    RustPatterns {
        pattern: RustPattern,
        indicators: Vec<Cow<'static, str>>,
    },
    Name {
        prefix: Option<Cow<'static, str>>,
        keywords: Vec<Cow<'static, str>>,
    },
}

impl SignalEvidence {
    /// Format evidence for standard output (concise)
    pub fn format_concise(&self) -> String {
        match self {
            Self::IoDetection { operations, primary_type } => {
                format!("{} via {:?}", primary_type, operations.first())
            }
            Self::CallGraph { calls_count, pattern } => {
                format!("{:?} pattern: calls {} functions", pattern, calls_count)
            }
            Self::TypeSignature { pattern, .. } => {
                format!("Matches {:?} pattern", pattern)
            }
            Self::Purity { is_pure, impurity_reasons } => {
                if *is_pure {
                    "Pure function".into()
                } else {
                    format!("Impure: {:?}", impurity_reasons.first())
                }
            }
            Self::Framework { framework_type, pattern_matched } => {
                format!("{:?} framework pattern: {}", framework_type, pattern_matched)
            }
            Self::RustPatterns { pattern, .. } => {
                format!("{:?} pattern detected", pattern)
            }
            Self::Name { prefix, keywords } => {
                format!("Name indicates: {:?}", prefix.or_else(|| keywords.first()))
            }
        }
    }

    /// Format evidence for verbose output (detailed)
    pub fn format_verbose(&self) -> String {
        match self {
            Self::IoDetection { operations, primary_type } => {
                format!(
                    "I/O Type: {:?}\nOperations: {}",
                    primary_type,
                    operations.iter()
                        .map(|op| format!("{:?}", op))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Self::CallGraph { calls_count, pattern } => {
                format!(
                    "Structural Pattern: {:?}\nCalls: {} function(s)\nIndicates orchestration/coordination behavior",
                    pattern, calls_count
                )
            }
            Self::TypeSignature { pattern, confidence_reason } => {
                format!("Type Pattern: {:?}\nReason: {}", pattern, confidence_reason)
            }
            Self::Purity { is_pure, impurity_reasons } => {
                if *is_pure {
                    "Pure function: no side effects detected".into()
                } else {
                    format!(
                        "Impure function\nReasons:\n{}",
                        impurity_reasons.iter()
                            .map(|r| format!("  - {:?}", r))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                }
            }
            Self::Framework { framework_type, pattern_matched } => {
                format!(
                    "Framework: {:?}\nPattern: {}\nIndicates framework-specific responsibility",
                    framework_type, pattern_matched
                )
            }
            Self::RustPatterns { pattern, indicators } => {
                format!(
                    "Rust Pattern: {:?}\nIndicators:\n{}",
                    pattern,
                    indicators.iter()
                        .map(|i| format!("  - {}", i))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
            Self::Name { prefix, keywords } => {
                format!(
                    "Name Analysis:\nPrefix: {:?}\nKeywords: {}",
                    prefix,
                    keywords.join(", ")
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlternativeClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub difference: f64,  // Difference from primary
}

impl ClassificationEvidence {
    /// Create evidence from multi-signal classification results
    pub fn from_classification(
        primary: ResponsibilityCategory,
        confidence: f64,
        signals: Vec<SignalContribution>,
        all_scores: Vec<(ResponsibilityCategory, f64)>,
    ) -> Self {
        Self {
            primary,
            confidence,
            signals,
            all_scores,
        }
    }

    pub fn confidence_band(&self) -> ConfidenceBand {
        match self.confidence {
            c if c >= 0.80 => ConfidenceBand::High,
            c if c >= 0.60 => ConfidenceBand::Medium,
            _ => ConfidenceBand::Low,
        }
    }

    pub fn is_ambiguous(&self) -> bool {
        self.alternatives(0.10).first().is_some()
    }

    /// Compute alternative classifications from all_scores
    /// Returns alternatives within `threshold` of primary score
    pub fn alternatives(&self, threshold: f64) -> Vec<AlternativeClassification> {
        self.all_scores.iter()
            .skip(1)  // Skip primary (first element)
            .take(3)  // Top 3 alternatives max
            .filter(|(_, score)| self.confidence - score < threshold)
            .map(|(category, score)| AlternativeClassification {
                category: *category,
                confidence: *score,
                difference: self.confidence - score,
            })
            .collect()
    }
}
```

**Phase 2: Evidence Formatter**

```rust
pub struct EvidenceFormatter {
    verbosity: VerbosityLevel,
    color_enabled: bool,
    show_all_signals: bool,  // True at -vvv to override signal filters
    config: OutputConfig,
}

#[derive(Debug, Clone, Copy)]
pub enum VerbosityLevel {
    Minimal,   // Only show confidence and primary category
    Standard,  // Show signal summary (default)
    Verbose,   // Show all details including technical evidence
}

impl EvidenceFormatter {
    /// Create formatter from verbosity count
    /// - 0: Minimal (category + confidence only)
    /// - 1: Standard (signal summary)
    /// - 2+: Verbose (detailed breakdown)
    pub fn new(verbose_count: u8, config: &OutputConfig) -> Self {
        let verbosity = match verbose_count {
            0 => VerbosityLevel::Minimal,
            1 => VerbosityLevel::Standard,
            _ => VerbosityLevel::Verbose,
        };

        // At -vvv (3+), override signal filters to show all signals
        let show_all_signals = verbose_count >= 3;

        Self {
            verbosity,
            color_enabled: atty::is(atty::Stream::Stdout),
            show_all_signals,
            config: config.clone(),
        }
    }

    pub fn format_evidence(&self, evidence: &ClassificationEvidence) -> String {
        match self.verbosity {
            VerbosityLevel::Minimal => self.format_minimal(evidence),
            VerbosityLevel::Standard => self.format_standard_evidence(evidence),
            VerbosityLevel::Verbose => self.format_verbose_evidence(evidence),
        }
    }

    fn format_minimal(&self, evidence: &ClassificationEvidence) -> String {
        format!(
            "Classification: {} [{:.0}% confidence]\n",
            evidence.primary,
            evidence.confidence * 100.0
        )
    }

    fn format_standard_evidence(&self, evidence: &ClassificationEvidence) -> String {
        [
            self.format_header(evidence),
            self.format_signals(&evidence.signals),
            self.format_alternatives_if_ambiguous(evidence),
            self.format_score(evidence.confidence),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
    }

    fn format_header(&self, evidence: &ClassificationEvidence) -> String {
        format!(
            "\nCLASSIFICATION ANALYSIS:\n\
             Primary: {} [Confidence: {:.2}, {}]\n",
            evidence.primary,
            evidence.confidence,
            self.format_confidence_band(evidence.confidence_band())
        )
    }

    fn format_signals(&self, signals: &[SignalContribution]) -> String {
        let formatted_signals: Vec<String> = signals.iter()
            .filter(|s| self.should_show_signal(s))
            .map(|signal| self.format_signal(signal))
            .collect();

        format!("Contributing Signals:\n{}", formatted_signals.join("\n"))
    }

    fn should_show_signal(&self, signal: &SignalContribution) -> bool {
        // -vvv shows all signals, overriding config
        if self.show_all_signals {
            return true;
        }

        // Otherwise respect signal filters
        match signal.signal_type {
            SignalType::IoDetection => self.config.signal_filters.show_io_detection,
            SignalType::CallGraph => self.config.signal_filters.show_call_graph,
            SignalType::TypeSignatures => self.config.signal_filters.show_type_signatures,
            SignalType::Purity => self.config.signal_filters.show_purity,
            SignalType::Framework => self.config.signal_filters.show_framework,
            SignalType::RustPatterns => true,  // Always show
            SignalType::Name => self.config.signal_filters.show_name_heuristics,
        }
    }

    fn format_signal(&self, signal: &SignalContribution) -> String {
        if !signal.is_available {
            return format!("  • {:?}: N/A", signal.signal_type);
        }

        let indicator = match signal.contribution {
            c if c > 0.15 => "✓",
            c if c > 0.05 => "•",
            _ => "-",
        };

        format!(
            "  {} {:?} ({:.0}% conf, {:.0}% weight) = {:.3} contribution\n\
                 Evidence: {}",
            indicator,
            signal.signal_type,
            signal.confidence * 100.0,
            signal.weight * 100.0,
            signal.contribution,
            signal.evidence.format_concise()  // Use structured evidence formatting
        )
    }

    fn format_alternatives_if_ambiguous(&self, evidence: &ClassificationEvidence) -> String {
        if !evidence.is_ambiguous() {
            return String::new();
        }

        let alternatives = evidence.alternatives(0.10);
        let formatted_alts: Vec<String> = alternatives.iter()
            .map(|alt| format!(
                "  - {} ({:.2} confidence, Δ{:.2})",
                alt.category,
                alt.confidence,
                alt.difference
            ))
            .collect();

        format!(
            "\nAlternative Classifications:\n{}\n  ⚠ Classification is ambiguous - consider manual review",
            formatted_alts.join("\n")
        )
    }

    fn format_score(&self, confidence: f64) -> String {
        format!("\nWeighted Score: {:.3}", confidence)
    }

    fn format_verbose_evidence(&self, evidence: &ClassificationEvidence) -> String {
        [
            self.format_standard_evidence(evidence),
            self.format_detailed_breakdown(&evidence.signals),
        ]
        .join("\n")
    }

    fn format_detailed_breakdown(&self, signals: &[SignalContribution]) -> String {
        let details: Vec<String> = signals.iter()
            .filter(|s| s.is_available)
            .map(|signal| format!(
                "\n{:?} Signal:\n\
                   Category: {}\n\
                   Confidence: {:.4}\n\
                   Weight: {:.4}\n\
                   Contribution: {:.4}\n\
                   Evidence:\n{}",
                signal.signal_type,
                signal.category,
                signal.confidence,
                signal.weight,
                signal.contribution,
                signal.evidence.format_verbose()  // Use detailed structured evidence
            ))
            .collect();

        format!("\nDETAILED BREAKDOWN:{}", details.join(""))
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

**Prerequisites: Verify/Modify Spec 145 Integration**

Before implementing this spec, verify that `MultiSignalClassifier` from Spec 145 returns evidence data:

```rust
// In src/organization/multi_signal_classifier.rs
// CURRENT (Spec 145 - verify this exists):
impl MultiSignalClassifier {
    pub fn classify_method(&self, method: &MethodInfo) -> ResponsibilityCategory {
        // ... classification logic
    }
}

// REQUIRED for Spec 148:
pub struct ClassificationResult {
    pub category: ResponsibilityCategory,
    pub evidence: ClassificationEvidence,
}

impl MultiSignalClassifier {
    pub fn classify_method(&self, method: &MethodInfo) -> ClassificationResult {
        // Track all signal contributions during classification
        let mut signals = Vec::new();
        let mut category_scores: HashMap<ResponsibilityCategory, f64> = HashMap::new();

        // I/O Detection signal
        if let Some(io_result) = self.io_detector.classify(method) {
            signals.push(SignalContribution {
                signal_type: SignalType::IoDetection,
                category: io_result.category,
                confidence: io_result.confidence,
                weight: self.weights.io_detection,
                contribution: io_result.confidence * self.weights.io_detection,
                evidence: io_result.evidence,  // Structured evidence
                is_available: true,
            });
            *category_scores.entry(io_result.category).or_default() +=
                io_result.confidence * self.weights.io_detection;
        }

        // CallGraph signal
        if let Some(cg_result) = self.call_graph_analyzer.classify(method) {
            signals.push(SignalContribution {
                signal_type: SignalType::CallGraph,
                category: cg_result.category,
                confidence: cg_result.confidence,
                weight: self.weights.call_graph,
                contribution: cg_result.confidence * self.weights.call_graph,
                evidence: cg_result.evidence,
                is_available: true,
            });
            *category_scores.entry(cg_result.category).or_default() +=
                cg_result.confidence * self.weights.call_graph;
        }

        // ... repeat for other signals

        // Sort scores to find primary and alternatives
        let mut all_scores: Vec<_> = category_scores.into_iter().collect();
        all_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let (primary_category, primary_confidence) = all_scores[0];

        ClassificationResult {
            category: primary_category,
            evidence: ClassificationEvidence::from_classification(
                primary_category,
                primary_confidence,
                signals,
                all_scores,
            ),
        }
    }
}
```

**Action Items for Spec 145 Integration**:
1. Verify Spec 145 implementation tracks individual signal results
2. If not, modify to store signal contributions during classification
3. Update return type from `ResponsibilityCategory` to `ClassificationResult`
4. Update all call sites to use `.category` to access the category

**New Module**: `src/output/evidence_formatter.rs`
- Evidence display formatting
- Verbosity level handling
- Confidence band visualization
- Alternative classification display
- Lazy evaluation wrapper for performance

**Modified Module**: `src/organization/multi_signal_classifier.rs`
- Return `ClassificationResult` instead of just `ResponsibilityCategory`
- Track signal contributions during classification
- Build `ClassificationEvidence` from aggregated results

**Modified Module**: `src/priority/formatter.rs`
- Integrate evidence display into recommendations
- Add verbosity configuration option
- Format signal contributions using `EvidenceFormatter`

**New Configuration**: Add to `DebtmapConfig`:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct OutputConfig {
    pub show_alternatives: bool,      // Default: true
    pub min_confidence_warning: f64,  // Default: 0.60
    pub signal_filters: SignalFilters,  // Control which signals to display
}

// Verbosity level determined by CLI args.verbose count (0, 1, 2, 3+)
```

### Data Structures

See Phase 1 above for main data structures.

## Dependencies

- **Prerequisites**: Spec 145 (Multi-Signal Aggregation) - must be extended to return ClassificationEvidence
  - **Required Changes to Spec 145**: Modify `MultiSignalClassifier::classify_method()` to return `ClassificationResult` instead of just `ResponsibilityCategory`
  - **Verification Step**: Confirm Spec 145 implementation includes signal tracking and scoring data
- **Affected Components**:
  - `src/organization/multi_signal_classifier.rs` - extend return types to include evidence
  - `src/output/evidence_formatter.rs` - new module for evidence display (create)
  - `src/priority/formatter.rs` - integrate evidence into output formatting
  - `src/organization/god_object_detector.rs` - pass evidence through recommendations
  - `src/config.rs` - add OutputConfig for evidence display settings
- **External Dependencies**: None
- **Configuration Precedence**: CLI flags > Environment variables > debtmap.toml > defaults

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
debtmap           # Minimal: category + confidence only
debtmap -v        # Standard: signal summary with top contributors
debtmap -vv       # Verbose: detailed signal breakdown
debtmap -vvv      # Very verbose: all signals including low-weight ones
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

Evidence formatting can be expensive for large projects. Optimize by using lazy evaluation from the start:

```rust
use std::sync::OnceLock;
use std::sync::Arc;

/// Lazy evidence formatting - only formats when actually displayed
pub struct LazyEvidence {
    data: Arc<ClassificationEvidence>,
    formatted: OnceLock<FormattedEvidence>,
}

#[derive(Debug, Clone)]
struct FormattedEvidence {
    minimal: String,
    standard: String,
    verbose: String,
}

impl LazyEvidence {
    pub fn new(evidence: ClassificationEvidence) -> Self {
        Self {
            data: Arc::new(evidence),
            formatted: OnceLock::new(),
        }
    }

    /// Get formatted evidence for given verbosity level
    pub fn format(&self, verbosity: VerbosityLevel) -> &str {
        let formatted = self.formatted.get_or_init(|| {
            let formatter = EvidenceFormatter::new(VerbosityLevel::Standard);
            FormattedEvidence {
                minimal: formatter.format_minimal(&self.data),
                standard: formatter.format_standard_evidence(&self.data),
                verbose: formatter.format_verbose_evidence(&self.data),
            }
        });

        match verbosity {
            VerbosityLevel::Minimal => &formatted.minimal,
            VerbosityLevel::Standard => &formatted.standard,
            VerbosityLevel::Verbose => &formatted.verbose,
        }
    }

    /// Get raw evidence data without formatting
    pub fn raw(&self) -> &ClassificationEvidence {
        &self.data
    }
}
```

### Memory Optimization

Use `Cow<'static, str>` for common evidence strings to avoid allocations:

```rust
// In SignalEvidence variants
pub enum SignalEvidence {
    TypeSignature {
        pattern: TypePattern,
        confidence_reason: Cow<'static, str>,  // Avoids allocation for common reasons
    },
    // ...
}

// Usage in classification code
let evidence = SignalEvidence::TypeSignature {
    pattern: TypePattern::Parser,
    confidence_reason: Cow::Borrowed("Matches &str → Result<T, E> pattern"),  // No allocation!
};
```

### Performance Benchmarks

Add benchmarks before implementation to establish baseline:

```rust
// benches/evidence_formatting.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn create_test_evidence(signal_count: usize) -> ClassificationEvidence {
    let signals = (0..signal_count)
        .map(|i| SignalContribution {
            signal_type: match i % 7 {
                0 => SignalType::IoDetection,
                1 => SignalType::CallGraph,
                2 => SignalType::TypeSignatures,
                3 => SignalType::Purity,
                4 => SignalType::Framework,
                5 => SignalType::RustPatterns,
                _ => SignalType::Name,
            },
            category: ResponsibilityCategory::Parsing,
            confidence: 0.8,
            weight: 0.15,
            contribution: 0.12,
            evidence: create_test_signal_evidence(i),
            is_available: true,
        })
        .collect();

    ClassificationEvidence::from_classification(
        ResponsibilityCategory::Parsing,
        0.82,
        signals,
        vec![(ResponsibilityCategory::Parsing, 0.82)],
    )
}

fn bench_evidence_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("evidence_formatting");

    for signal_count in [1, 3, 7].iter() {
        group.bench_with_input(
            BenchmarkId::new("standard", signal_count),
            signal_count,
            |b, &count| {
                let evidence = create_test_evidence(count);
                let formatter = EvidenceFormatter::new(VerbosityLevel::Standard);
                b.iter(|| {
                    formatter.format_evidence(black_box(&evidence))
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("lazy_first_access", signal_count),
            signal_count,
            |b, &count| {
                let evidence = create_test_evidence(count);
                b.iter(|| {
                    let lazy = LazyEvidence::new(evidence.clone());
                    lazy.format(VerbosityLevel::Standard)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("lazy_cached_access", signal_count),
            signal_count,
            |b, &count| {
                let evidence = create_test_evidence(count);
                let lazy = LazyEvidence::new(evidence);
                let _ = lazy.format(VerbosityLevel::Standard); // Prime cache
                b.iter(|| {
                    lazy.format(VerbosityLevel::Standard)
                });
            },
        );
    }

    group.finish();
}

fn bench_full_analysis_overhead(c: &mut Criterion) {
    // Benchmark full analysis with and without evidence formatting
    let mut group = c.benchmark_group("full_analysis_overhead");

    group.bench_function("analysis_without_evidence", |b| {
        b.iter(|| {
            // Run full analysis without formatting evidence
            analyze_test_project_no_evidence()
        });
    });

    group.bench_function("analysis_with_evidence_standard", |b| {
        b.iter(|| {
            // Run full analysis with standard evidence formatting
            analyze_test_project_with_evidence(VerbosityLevel::Standard)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_evidence_formatting, bench_full_analysis_overhead);
criterion_main!(benches);
```

### Performance Targets

- **Individual evidence formatting**: <1ms per classification
- **Lazy evaluation cache hit**: <10μs (near-zero overhead)
- **Full project analysis overhead**: <2% total runtime increase
- **Memory overhead**: <5% increase for evidence storage

Validate these targets with benchmarks before and after implementation.

### Configuration Integration

**Configuration precedence**: CLI flags > Environment variables > `debtmap.toml` > defaults

**Rust configuration structure**:
```rust
// In src/config.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebtmapConfig {
    // ... existing fields
    #[serde(default)]
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutputConfig {
    #[serde(default = "default_show_alternatives")]
    pub show_alternatives: bool,

    #[serde(default = "default_min_confidence_warning")]
    pub min_confidence_warning: f64,

    #[serde(default)]
    pub signal_filters: SignalFilters,
}

// Note: Verbosity level comes from CLI args.verbose, not config

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VerbosityLevel {
    Minimal,
    #[serde(alias = "default")]
    Standard,
    Verbose,
}

impl Default for VerbosityLevel {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SignalFilters {
    #[serde(default = "default_true")]
    pub show_io_detection: bool,
    #[serde(default = "default_true")]
    pub show_call_graph: bool,
    #[serde(default = "default_true")]
    pub show_type_signatures: bool,
    #[serde(default = "default_true")]
    pub show_purity: bool,
    #[serde(default = "default_true")]
    pub show_framework: bool,
    #[serde(default = "default_false")]
    pub show_name_heuristics: bool,  // Default off: low-weight fallback signal
}

impl Default for SignalFilters {
    fn default() -> Self {
        Self {
            show_io_detection: true,
            show_call_graph: true,
            show_type_signatures: true,
            show_purity: true,
            show_framework: true,
            show_name_heuristics: false,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            show_alternatives: true,
            min_confidence_warning: 0.60,
            signal_filters: SignalFilters::default(),
        }
    }
}

fn default_show_alternatives() -> bool { true }
fn default_min_confidence_warning() -> f64 { 0.60 }
fn default_true() -> bool { true }
fn default_false() -> bool { false }
```

**TOML configuration file** (`debtmap.toml`):
```toml
[output]
show_alternatives = true
min_confidence_warning = 0.60

[output.signal_filters]
show_io_detection = true
show_call_graph = true
show_type_signatures = true
show_purity = true
show_framework = true
show_name_heuristics = false  # Hide low-weight fallback signal
```

Note: Verbosity is controlled via CLI flags (`-v`, `-vv`, `-vvv`) only, not config file.

**CLI flags** (add to `clap` argument parser):
```rust
#[derive(Parser)]
pub struct Args {
    // ... existing args

    /// Increase verbosity (-v for standard evidence, -vv for detailed, -vvv for all signals)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Minimum confidence threshold for low-confidence warnings
    #[arg(long, value_name = "THRESHOLD", default_value = "0.60")]
    pub min_confidence_warning: f64,
}
```

**Verbosity mapping**:
- No flags (default): Minimal evidence (category + confidence only)
- `-v`: Standard evidence (signal summary with top contributors)
- `-vv`: Verbose evidence (detailed signal breakdown)
- `-vvv`: Very verbose (all signals including low-weight ones)

Evidence is always shown - verbosity level controls how much detail.

## Migration Path

Evidence display will be **always enabled** once implemented. No backward compatibility flags or gradual rollout needed - this is a pure improvement to output quality and user understanding.

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

## Implementation Plan (Revised with Clarifications)

### Pre-Implementation: Spec 145 Verification (0.5 day)

**Tasks**:
1. Read Spec 145 implementation in `src/organization/multi_signal_classifier.rs`
2. Verify if `classify_method()` currently returns evidence data
3. If not, plan modification to track signal contributions
4. Document required changes to Spec 145

**Acceptance**:
- Clear understanding of Spec 145's current state
- Plan for extending return type to include evidence
- List of all call sites that need updating

### Phase 1: Data Structures + Spec 145 Integration (1.5-2 days)

**Tasks**:
1. Define `SignalEvidence` enum with structured variants (IoDetection, CallGraph, etc.)
2. Define `ClassificationEvidence` struct with `all_scores` field
3. Define `ClassificationResult` wrapper struct
4. Implement `SignalEvidence::format_concise()` and `format_verbose()` methods
5. Implement `ClassificationEvidence::alternatives()` method
6. Modify `MultiSignalClassifier::classify_method()` to return `ClassificationResult`
7. Update all call sites to use `.category` field
8. Write unit tests for evidence data structures

**Acceptance**:
- All data structures compile with proper trait bounds
- Spec 145 returns evidence data alongside classification
- All existing tests pass with updated return type
- Evidence alternatives are correctly computed from `all_scores`

### Phase 2: Evidence Formatter (2 days)

**Tasks**:
1. Create `src/output/evidence_formatter.rs` module
2. Implement `EvidenceFormatter` with functional composition (no mutable strings)
3. Implement all three verbosity levels (Minimal, Standard, Verbose)
4. Implement `LazyEvidence` wrapper with `OnceLock` caching
5. Write unit tests for each formatting method
6. Add performance benchmarks (baseline before integration)

**Acceptance**:
- Formatting produces expected output for all verbosity levels
- Functional composition avoids mutable string building
- Lazy evaluation caches formatted strings correctly
- Benchmarks show <1ms formatting time per evidence
- All unit tests pass

### Phase 3: Configuration Support (0.5 day)

**Tasks**:
1. Add `OutputConfig` to `src/config.rs`
2. Implement `SignalFilters` for hiding specific signals
3. Add `-v` verbosity flag counting (existing or new)
4. Add `--min-confidence-warning` CLI flag
5. Update TOML deserialization for `[output]` section
6. Wire verbosity count through to `EvidenceFormatter::new(verbose_count)`

**Acceptance**:
- Configuration loads from TOML correctly
- CLI flags override config file settings
- Verbosity count (0, 1, 2, 3+) correctly maps to evidence levels
- Default configuration matches spec requirements

### Phase 4: Output Integration (1.5 days)

**Tasks**:
1. Modify `src/priority/formatter.rs` to accept evidence data
2. Integrate evidence display into module split recommendations
3. Integrate evidence into function classification output (#10)
4. Implement "Utilities" explanation logic (`explain_utilities_classification()`)
5. Apply `SignalFilters` to hide unwanted signals
6. Add integration tests for full output

**Acceptance**:
- Module recommendations show evidence for each proposed module
- Function classifications show signal contributions
- Low confidence classifications display warnings
- Utilities groupings explain why they're ambiguous
- Signal filtering works correctly
- Integration tests verify full output format

### Phase 5: Performance Validation & Documentation (1 day)

**Tasks**:
1. Run full project analysis benchmarks with evidence enabled
2. Verify <2% performance overhead target
3. Optimize if needed (string interning, caching, etc.)
4. Update README.md with evidence explanation
5. Add example output to documentation
6. Update CHANGELOG.md

**Acceptance**:
- Performance benchmarks show <2% overhead
- Memory overhead <5% for evidence storage
- User documentation explains evidence display clearly
- Example output demonstrates all features
- CHANGELOG accurately describes changes

### Phase 6: Testing & Polish (0.5 day)

**Tasks**:
1. Run full test suite (`cargo test --all-features`)
2. Run clippy (`cargo clippy --all-targets -- -D warnings`)
3. Format code (`cargo fmt --all`)
4. Test with real codebases (debtmap itself, other Rust projects)
5. Verify verbosity levels work correctly

**Acceptance**:
- All tests pass
- No clippy warnings
- Code formatted consistently
- Real-world testing shows useful evidence at all verbosity levels
- Evidence display improves user understanding

**Total Estimated Time**: 6-8 days

**Critical Path**:
1. Spec 145 verification (blocking)
2. Data structures (blocking for formatter)
3. Formatter implementation (blocking for integration)
4. Output integration (user-facing)
5. Performance validation (quality gate)
