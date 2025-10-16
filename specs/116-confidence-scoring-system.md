---
number: 116
title: Confidence Scoring System
category: foundation
priority: high
status: draft
dependencies: [112, 113, 114, 115]
created: 2025-10-16
---

# Specification 116: Confidence Scoring System

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Specs 112, 113, 114, 115

## Context

Debtmap v0.2.8 presents all findings with equal weight, providing no indication of confidence or likelihood that a finding is accurate. This makes it difficult for users to prioritize which recommendations to trust and act upon.

**Real-World Impact from Bug Report**:
- **50% false positive rate** in dead code detection
- No way to distinguish high-confidence from low-confidence findings
- Users must manually verify every recommendation
- Critical false positives (#10) presented with same confidence as valid findings (#1-4)
- Report recommends: "Separate 'high confidence dead code' from 'low confidence'"

**Current Behavior**:
All findings shown with same priority:
```
#6 create_bots_from_list [SCORE: 4.50] [MEDIUM]
  Recommendation: Private function has no callers and can be safely removed
  (Actually: Public API function - FALSE POSITIVE)

#10 ConversationManager.add_message [SCORE: 1.44] [LOW]
  Recommendation: Private function has no callers and can be safely removed
  (Actually: Used across files - CRITICAL FALSE POSITIVE)
```

**What's Missing**:
- No confidence score (0.0-1.0) indicating finding reliability
- No distinction between "definitely dead" vs "might be dead"
- No integration of multiple signals (cross-file, public API, patterns, static analysis)
- No clear guidance on which findings to act on first

**Why This is Critical**:
- Users need to prioritize verification efforts
- High-confidence findings can be acted on with less risk
- Low-confidence findings require manual investigation
- Confidence enables automatic filtering (e.g., only show >0.8 confidence)
- Reduces time wasted on false positives

## Objective

Implement a comprehensive confidence scoring system that integrates signals from cross-file analysis, public API detection, pattern recognition, and static analysis to provide users with reliability estimates for each finding, enabling better prioritization and reducing false positive impact.

## Requirements

### Functional Requirements

1. **Multi-Signal Confidence Calculation**
   - Integrate cross-file usage data (Spec 109)
   - Incorporate public API heuristics (Spec 110)
   - Include pattern recognition results (Spec 111)
   - Factor in static analysis warnings (Spec 112)
   - Combine complexity metrics
   - Weight signals appropriately

2. **Confidence Score Components**
   - **Direct evidence** (0.0-1.0): Confirmed callers or usage
   - **Indirect evidence** (0.0-1.0): Pattern usage, public API indicators
   - **Negative evidence** (0.0-1.0): No usage found, no public indicators
   - **Risk factors** (0.0-1.0): Static analysis errors, broken code
   - **Uncertainty** (0.0-1.0): Wildcard imports, dynamic code

3. **Confidence Levels**
   - **Very High (0.9-1.0)**: Definite finding, safe to act
   - **High (0.7-0.89)**: Strong confidence, verify before acting
   - **Medium (0.5-0.69)**: Moderate confidence, requires investigation
   - **Low (0.3-0.49)**: Weak confidence, manual verification essential
   - **Very Low (0.0-0.29)**: High uncertainty, likely false positive

4. **Scoring Algorithm**
   - Bayesian combination of independent signals
   - Weight signals by reliability
   - Handle missing signals gracefully
   - Penalize uncertainty factors
   - Boost confidence for multiple confirming signals

5. **Output and Visualization**
   - Show confidence score in all output formats
   - Add confidence badge to terminal output
   - Include confidence breakdown in verbose mode
   - Sort findings by confidence
   - Filter findings by minimum confidence threshold

6. **Confidence Explanations**
   - Explain why confidence is high/low
   - List contributing factors
   - Show which signals were used
   - Provide actionable verification steps

### Non-Functional Requirements

1. **Accuracy**
   - High-confidence findings (>0.8) have <5% false positive rate
   - Low-confidence findings (<0.5) have >40% false positive rate
   - Confidence calibration matches actual accuracy

2. **Transparency**
   - Users understand why confidence is assigned
   - Confidence breakdown available in verbose mode
   - Clear documentation of scoring algorithm

3. **Configurability**
   - Users can adjust signal weights
   - Configurable confidence thresholds
   - Option to filter by minimum confidence

4. **Performance**
   - Confidence calculation adds < 5% overhead
   - Cached signals reused across findings

## Acceptance Criteria

- [ ] Confidence score (0.0-1.0) calculated for all findings
- [ ] Cross-file usage signals integrated (from Spec 109)
- [ ] Public API heuristics integrated (from Spec 110)
- [ ] Pattern recognition signals integrated (from Spec 111)
- [ ] Static analysis signals integrated (from Spec 112)
- [ ] Confidence levels (Very High/High/Medium/Low/Very Low) assigned
- [ ] High-confidence findings (>0.8) have <5% false positive rate
- [ ] Low-confidence findings (<0.5) flagged as "requires verification"
- [ ] Confidence badge shown in terminal output
- [ ] Confidence breakdown available with `-v` verbose flag
- [ ] Findings sortable by confidence
- [ ] Filter by minimum confidence threshold (`--min-confidence`)
- [ ] Confidence explanation shows contributing factors
- [ ] Configuration for adjusting signal weights
- [ ] Documentation explains confidence scoring
- [ ] False positive #10 has low confidence (<0.4)
- [ ] Valid findings #1-4 have high confidence (>0.7)

## Technical Details

### Implementation Approach

**Phase 1: Confidence Framework**
1. Define confidence score structure
2. Implement signal aggregation algorithm
3. Create confidence level classification

**Phase 2: Signal Integration**
1. Integrate cross-file analysis signals (Spec 109)
2. Integrate public API detection signals (Spec 110)
3. Integrate pattern recognition signals (Spec 111)
4. Integrate static analysis signals (Spec 112)

**Phase 3: Scoring Algorithm**
1. Implement Bayesian signal combination
2. Add signal weighting
3. Handle uncertainty factors
4. Calibrate thresholds

**Phase 4: Output and Configuration**
1. Add confidence to output formats
2. Implement confidence filtering
3. Add verbose confidence breakdown
4. Create configuration options

### Architecture Changes

```rust
// src/confidence/mod.rs
pub mod signals;
pub mod scoring;
pub mod calibration;

pub struct ConfidenceScorer {
    config: ConfidenceConfig,
    weights: SignalWeights,
}

#[derive(Debug, Clone)]
pub struct ConfidenceConfig {
    pub use_cross_file: bool,
    pub use_public_api: bool,
    pub use_patterns: bool,
    pub use_static_analysis: bool,
    pub weights: SignalWeights,
    pub thresholds: ConfidenceThresholds,
}

#[derive(Debug, Clone)]
pub struct SignalWeights {
    pub cross_file_usage: f32,      // 0.35
    pub public_api_score: f32,      // 0.25
    pub pattern_usage: f32,         // 0.20
    pub static_analysis: f32,       // 0.15
    pub complexity_metrics: f32,    // 0.05
}

impl Default for SignalWeights {
    fn default() -> Self {
        Self {
            cross_file_usage: 0.35,
            public_api_score: 0.25,
            pattern_usage: 0.20,
            static_analysis: 0.15,
            complexity_metrics: 0.05,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfidenceThresholds {
    pub very_high: f32,  // 0.9
    pub high: f32,       // 0.7
    pub medium: f32,     // 0.5
    pub low: f32,        // 0.3
}

impl Default for ConfidenceThresholds {
    fn default() -> Self {
        Self {
            very_high: 0.9,
            high: 0.7,
            medium: 0.5,
            low: 0.3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfidenceScore {
    pub score: f32,
    pub level: ConfidenceLevel,
    pub signals: SignalContributions,
    pub explanation: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfidenceLevel {
    VeryHigh,  // 0.9-1.0
    High,      // 0.7-0.89
    Medium,    // 0.5-0.69
    Low,       // 0.3-0.49
    VeryLow,   // 0.0-0.29
}

impl ConfidenceLevel {
    pub fn from_score(score: f32, thresholds: &ConfidenceThresholds) -> Self {
        if score >= thresholds.very_high {
            ConfidenceLevel::VeryHigh
        } else if score >= thresholds.high {
            ConfidenceLevel::High
        } else if score >= thresholds.medium {
            ConfidenceLevel::Medium
        } else if score >= thresholds.low {
            ConfidenceLevel::Low
        } else {
            ConfidenceLevel::VeryLow
        }
    }

    pub fn badge(&self) -> &str {
        match self {
            ConfidenceLevel::VeryHigh => "[🟢 VERY HIGH]",
            ConfidenceLevel::High => "[🟢 HIGH]",
            ConfidenceLevel::Medium => "[🟡 MEDIUM]",
            ConfidenceLevel::Low => "[🟠 LOW]",
            ConfidenceLevel::VeryLow => "[🔴 VERY LOW]",
        }
    }

    pub fn should_act(&self) -> bool {
        matches!(self, ConfidenceLevel::VeryHigh | ConfidenceLevel::High)
    }
}

#[derive(Debug, Clone)]
pub struct SignalContributions {
    pub cross_file: Option<SignalScore>,
    pub public_api: Option<SignalScore>,
    pub pattern: Option<SignalScore>,
    pub static_analysis: Option<SignalScore>,
    pub complexity: Option<SignalScore>,
}

#[derive(Debug, Clone)]
pub struct SignalScore {
    pub value: f32,
    pub weight: f32,
    pub contribution: f32, // value * weight
    pub reasoning: String,
}

impl ConfidenceScorer {
    pub fn new(config: ConfidenceConfig) -> Self;

    pub fn calculate_confidence(
        &self,
        finding: &DebtFinding,
        context: &AnalysisContext,
    ) -> ConfidenceScore;

    pub fn validate_weights(&self) -> Result<()> {
        let total = self.weights.cross_file_usage
            + self.weights.public_api_score
            + self.weights.pattern_usage
            + self.weights.static_analysis
            + self.weights.complexity_metrics;

        if (total - 1.0).abs() > 0.001 {
            return Err(anyhow!("Signal weights must sum to 1.0, got {}", total));
        }

        Ok(())
    }
}

// src/confidence/signals.rs
pub trait ConfidenceSignal {
    fn name(&self) -> &str;
    fn evaluate(&self, finding: &DebtFinding, context: &AnalysisContext) -> Option<f32>;
    fn explain(&self, finding: &DebtFinding, context: &AnalysisContext) -> String;
}

pub struct CrossFileSignal;
pub struct PublicApiSignal;
pub struct PatternSignal;
pub struct StaticAnalysisSignal;
pub struct ComplexitySignal;

impl ConfidenceSignal for CrossFileSignal {
    fn evaluate(&self, finding: &DebtFinding, context: &AnalysisContext) -> Option<f32> {
        if let Some(cross_file_graph) = &context.cross_file_graph {
            let usage_count = cross_file_graph.get_usage_count(&finding.function);

            // Strong negative signal: definitely used across files
            if usage_count > 0 {
                return Some(0.0); // Not dead code
            }

            // Check for wildcard imports (uncertainty)
            if context.has_wildcard_imports(&finding.function.file) {
                return Some(0.3); // Low confidence - might be imported via wildcard
            }

            // Strong positive signal: no cross-file usage found
            Some(1.0)
        } else {
            None // Cross-file analysis not available
        }
    }

    fn explain(&self, finding: &DebtFinding, context: &AnalysisContext) -> String {
        if let Some(graph) = &context.cross_file_graph {
            let usage_count = graph.get_usage_count(&finding.function);

            if usage_count > 0 {
                format!("Function is called {} times across files", usage_count)
            } else if context.has_wildcard_imports(&finding.function.file) {
                "No direct cross-file usage, but wildcard imports present (uncertain)".to_string()
            } else {
                "No cross-file usage detected in project-wide analysis".to_string()
            }
        } else {
            "Cross-file analysis not performed".to_string()
        }
    }
}

impl ConfidenceSignal for PublicApiSignal {
    fn evaluate(&self, finding: &DebtFinding, context: &AnalysisContext) -> Option<f32> {
        if let Some(public_api_detector) = &context.public_api_detector {
            let api_score = public_api_detector.is_public_api(&finding.function, &context.file_context)?;

            if api_score.is_public {
                // Public API → likely false positive
                return Some(1.0 - api_score.confidence); // Invert: high public API score = low dead code confidence
            }

            // Not public API → higher confidence it's dead code
            Some(api_score.confidence)
        } else {
            None
        }
    }

    fn explain(&self, finding: &DebtFinding, context: &AnalysisContext) -> String {
        if let Some(detector) = &context.public_api_detector {
            let api_score = detector.is_public_api(&finding.function, &context.file_context).unwrap();

            if api_score.is_public {
                format!("Detected as public API (confidence: {:.2}): {}",
                    api_score.confidence,
                    api_score.reasoning.join("; "))
            } else {
                "Not detected as public API - likely internal function".to_string()
            }
        } else {
            "Public API detection not performed".to_string()
        }
    }
}

impl ConfidenceSignal for PatternSignal {
    fn evaluate(&self, finding: &DebtFinding, context: &AnalysisContext) -> Option<f32> {
        if let Some(pattern_detector) = &context.pattern_detector {
            if let Some(pattern) = pattern_detector.is_function_used_by_pattern(&finding.function, context) {
                // Used via pattern → definitely not dead code
                return Some(0.0);
            }

            // Not used via pattern → higher confidence it's dead
            Some(1.0)
        } else {
            None
        }
    }

    fn explain(&self, finding: &DebtFinding, context: &AnalysisContext) -> String {
        if let Some(detector) = &context.pattern_detector {
            if let Some(pattern) = detector.is_function_used_by_pattern(&finding.function, context) {
                format!("Used via {:?} pattern", pattern.pattern_type)
            } else {
                "No design pattern usage detected".to_string()
            }
        } else {
            "Pattern recognition not performed".to_string()
        }
    }
}

impl ConfidenceSignal for StaticAnalysisSignal {
    fn evaluate(&self, finding: &DebtFinding, context: &AnalysisContext) -> Option<f32> {
        if let Some(static_results) = &context.static_analysis {
            let warnings = static_results.get(&finding.function.file)?;
            let relevant_warnings = warnings.iter()
                .filter(|w| w.function.as_ref() == Some(&finding.function.name))
                .collect::<Vec<_>>();

            if relevant_warnings.is_empty() {
                return Some(0.7); // No errors, moderate confidence
            }

            // Has errors → likely broken code, very high confidence
            let has_errors = relevant_warnings.iter().any(|w| w.severity == Severity::Error);
            if has_errors {
                return Some(0.95); // Broken code is definitely problematic
            }

            // Has warnings → moderate confidence
            Some(0.8)
        } else {
            None
        }
    }

    fn explain(&self, finding: &DebtFinding, context: &AnalysisContext) -> String {
        if let Some(results) = &context.static_analysis {
            if let Some(warnings) = results.get(&finding.function.file) {
                let relevant = warnings.iter()
                    .filter(|w| w.function.as_ref() == Some(&finding.function.name))
                    .collect::<Vec<_>>();

                if !relevant.is_empty() {
                    let error_count = relevant.iter().filter(|w| w.severity == Severity::Error).count();
                    let warning_count = relevant.len() - error_count;

                    return format!("Static analysis found {} errors, {} warnings",
                        error_count, warning_count);
                }
            }

            "No static analysis warnings found".to_string()
        } else {
            "Static analysis not performed".to_string()
        }
    }
}

// src/confidence/scoring.rs
impl ConfidenceScorer {
    pub fn calculate_confidence(
        &self,
        finding: &DebtFinding,
        context: &AnalysisContext,
    ) -> ConfidenceScore {
        let mut contributions = SignalContributions::default();
        let mut signals = Vec::new();
        let mut explanation = Vec::new();

        // Evaluate cross-file signal
        if self.config.use_cross_file {
            let signal = CrossFileSignal;
            if let Some(value) = signal.evaluate(finding, context) {
                let contribution = value * self.weights.cross_file_usage;
                contributions.cross_file = Some(SignalScore {
                    value,
                    weight: self.weights.cross_file_usage,
                    contribution,
                    reasoning: signal.explain(finding, context),
                });
                signals.push(contribution);
                explanation.push(format!("Cross-file analysis: {}", contributions.cross_file.as_ref().unwrap().reasoning));
            }
        }

        // Evaluate public API signal
        if self.config.use_public_api {
            let signal = PublicApiSignal;
            if let Some(value) = signal.evaluate(finding, context) {
                let contribution = value * self.weights.public_api_score;
                contributions.public_api = Some(SignalScore {
                    value,
                    weight: self.weights.public_api_score,
                    contribution,
                    reasoning: signal.explain(finding, context),
                });
                signals.push(contribution);
                explanation.push(format!("Public API detection: {}", contributions.public_api.as_ref().unwrap().reasoning));
            }
        }

        // Evaluate pattern signal
        if self.config.use_patterns {
            let signal = PatternSignal;
            if let Some(value) = signal.evaluate(finding, context) {
                let contribution = value * self.weights.pattern_usage;
                contributions.pattern = Some(SignalScore {
                    value,
                    weight: self.weights.pattern_usage,
                    contribution,
                    reasoning: signal.explain(finding, context),
                });
                signals.push(contribution);
                explanation.push(format!("Pattern recognition: {}", contributions.pattern.as_ref().unwrap().reasoning));
            }
        }

        // Evaluate static analysis signal
        if self.config.use_static_analysis {
            let signal = StaticAnalysisSignal;
            if let Some(value) = signal.evaluate(finding, context) {
                let contribution = value * self.weights.static_analysis;
                contributions.static_analysis = Some(SignalScore {
                    value,
                    weight: self.weights.static_analysis,
                    contribution,
                    reasoning: signal.explain(finding, context),
                });
                signals.push(contribution);
                explanation.push(format!("Static analysis: {}", contributions.static_analysis.as_ref().unwrap().reasoning));
            }
        }

        // Calculate final score
        let score = if signals.is_empty() {
            0.5 // Default medium confidence if no signals available
        } else {
            signals.iter().sum::<f32>() / signals.len() as f32
        };

        let level = ConfidenceLevel::from_score(score, &self.config.thresholds);

        ConfidenceScore {
            score,
            level,
            signals: contributions,
            explanation,
        }
    }
}
```

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct AnalysisContext {
    pub cross_file_graph: Option<CrossFileCallGraph>,
    pub public_api_detector: Option<PublicApiDetector>,
    pub pattern_detector: Option<PatternDetector>,
    pub static_analysis: Option<HashMap<PathBuf, Vec<StaticAnalysisResult>>>,
    pub file_context: FileContext,
}

// Update DebtFinding to include confidence
#[derive(Debug, Clone)]
pub struct DebtFinding {
    pub function: FunctionDef,
    pub finding_type: FindingType,
    pub severity: Severity,
    pub confidence: ConfidenceScore, // NEW
    pub reason: String,
    pub recommendation: String,
}
```

### APIs and Interfaces

```rust
// Configuration in .debtmap.toml
[confidence]
enabled = true

[confidence.weights]
cross_file_usage = 0.35
public_api_score = 0.25
pattern_usage = 0.20
static_analysis = 0.15
complexity_metrics = 0.05

[confidence.thresholds]
very_high = 0.9
high = 0.7
medium = 0.5
low = 0.3

// CLI options
Commands::Analyze {
    /// Minimum confidence threshold (0.0-1.0)
    #[arg(long = "min-confidence")]
    min_confidence: Option<f32>,

    /// Show only high-confidence findings
    #[arg(long = "high-confidence-only")]
    high_confidence_only: bool,

    /// Sort findings by confidence (descending)
    #[arg(long = "sort-by-confidence")]
    sort_by_confidence: bool,
}

// Output format
{
  "function": "ConversationManager.add_message",
  "file": "conversation_manager.py",
  "line": 121,
  "type": "dead_code",
  "confidence": {
    "score": 0.35,
    "level": "Low",
    "signals": {
      "cross_file": {
        "value": 0.0,
        "weight": 0.35,
        "contribution": 0.0,
        "reasoning": "Function is called 2 times across files"
      },
      "public_api": {
        "value": 0.3,
        "weight": 0.25,
        "contribution": 0.075,
        "reasoning": "Not detected as public API - likely internal function"
      }
    },
    "explanation": [
      "Cross-file analysis: Function is called 2 times across files",
      "Public API detection: Not detected as public API - likely internal function"
    ]
  },
  "recommendation": "[LOW CONFIDENCE] Requires manual verification - likely false positive"
}
```

## Dependencies

- **Prerequisites**:
  - Spec 115: Cross-File Dependency Analysis
  - Spec 113: Public API Detection Heuristics
  - Spec 114: Design Pattern Recognition
  - Spec 115: Static Analysis Integration
- **Affected Components**:
  - `src/confidence/` - New module
  - `src/debt/` - Add confidence to findings
  - `src/io/output/` - Display confidence scores
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_level_classification() {
        let thresholds = ConfidenceThresholds::default();

        assert_eq!(ConfidenceLevel::from_score(0.95, &thresholds), ConfidenceLevel::VeryHigh);
        assert_eq!(ConfidenceLevel::from_score(0.8, &thresholds), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(0.6, &thresholds), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.4, &thresholds), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(0.2, &thresholds), ConfidenceLevel::VeryLow);
    }

    #[test]
    fn test_signal_weights_validation() {
        let mut weights = SignalWeights::default();
        let scorer = ConfidenceScorer::new(ConfidenceConfig {
            weights: weights.clone(),
            ..Default::default()
        });

        assert!(scorer.validate_weights().is_ok());

        // Invalid weights
        weights.cross_file_usage = 0.5;
        let scorer = ConfidenceScorer::new(ConfidenceConfig {
            weights,
            ..Default::default()
        });

        assert!(scorer.validate_weights().is_err());
    }

    #[test]
    fn test_cross_file_signal_with_usage() {
        let finding = create_test_finding("test_function");
        let mut context = AnalysisContext::default();

        // Function has cross-file usage
        let mut graph = CrossFileCallGraph::new();
        graph.add_call(
            CallSite::new("other.py", "caller", "test.py", "test_function", 10)
        );
        context.cross_file_graph = Some(graph);

        let signal = CrossFileSignal;
        let value = signal.evaluate(&finding, &context);

        assert_eq!(value, Some(0.0)); // Used → not dead code
    }

    #[test]
    fn test_public_api_signal() {
        let finding = create_test_finding("create_bots_from_list");
        let mut context = AnalysisContext::default();

        let mut detector = PublicApiDetector::new(PublicApiConfig::default());
        context.public_api_detector = Some(detector);

        let signal = PublicApiSignal;
        let value = signal.evaluate(&finding, &context);

        assert!(value.is_some());
        assert!(value.unwrap() < 0.5); // Public API → low dead code confidence
    }

    #[test]
    fn test_confidence_calculation_integration() {
        let finding = create_test_finding("unused_function");
        let context = create_comprehensive_context();

        let scorer = ConfidenceScorer::new(ConfidenceConfig::default());
        let confidence = scorer.calculate_confidence(&finding, &context);

        assert!(confidence.score >= 0.0 && confidence.score <= 1.0);
        assert!(!confidence.explanation.is_empty());
        assert!(confidence.signals.cross_file.is_some());
    }
}
```

### Integration Tests

Test confidence scores on bug report examples:

```rust
#[test]
fn test_bug_report_example_10_low_confidence() {
    // ConversationManager.add_message - FALSE POSITIVE
    // Should have LOW confidence (used cross-file)
    let finding = load_finding_from_bug_report(10);
    let context = analyze_promptconstruct_project();

    let scorer = ConfidenceScorer::new(ConfidenceConfig::default());
    let confidence = scorer.calculate_confidence(&finding, &context);

    assert!(confidence.score < 0.4, "False positive should have low confidence");
    assert!(matches!(confidence.level, ConfidenceLevel::Low | ConfidenceLevel::VeryLow));
}

#[test]
fn test_bug_report_example_1_high_confidence() {
    // ConversationPanel.on_paint - VALID COMPLEXITY ISSUE
    // Should have HIGH confidence
    let finding = load_finding_from_bug_report(1);
    let context = analyze_promptconstruct_project();

    let scorer = ConfidenceScorer::new(ConfidenceConfig::default());
    let confidence = scorer.calculate_confidence(&finding, &context);

    assert!(confidence.score > 0.7, "Valid finding should have high confidence");
    assert!(matches!(confidence.level, ConfidenceLevel::High | ConfidenceLevel::VeryHigh));
}
```

## Documentation Requirements

### User Documentation

```markdown
## Confidence Scoring

Debtmap assigns confidence scores to all findings:

### Confidence Levels

- **🟢 Very High (0.9-1.0)**: Safe to act on, < 5% false positive rate
- **🟢 High (0.7-0.89)**: Verify before acting, < 10% false positive rate
- **🟡 Medium (0.5-0.69)**: Investigate, ~20% false positive rate
- **🟠 Low (0.3-0.49)**: Manual verification essential, ~40% false positive rate
- **🔴 Very Low (0.0-0.29)**: Likely false positive, > 50% false positive rate

### How Confidence is Calculated

Multiple signals are combined:

1. **Cross-file usage** (35%): Used across files → low confidence
2. **Public API detection** (25%): Public API indicators → low confidence
3. **Pattern recognition** (20%): Used via patterns → low confidence
4. **Static analysis** (15%): Has errors → high confidence (broken code)
5. **Complexity metrics** (5%): Supporting evidence

### Filtering by Confidence

```bash
# Show only high-confidence findings
debtmap analyze src --min-confidence 0.7

# Or use flag
debtmap analyze src --high-confidence-only

# Sort by confidence
debtmap analyze src --sort-by-confidence
```

### Configuration

```toml
[confidence.weights]
cross_file_usage = 0.35
public_api_score = 0.25
pattern_usage = 0.20
static_analysis = 0.15
complexity_metrics = 0.05
```

### Example Output

```
#10 ConversationManager.add_message [🔴 VERY LOW CONFIDENCE: 0.35]
  Location: conversation_manager.py:121
  Reason: No callers detected

  Confidence Breakdown:
    [✓] Cross-file usage: Function is called 2 times across files (0.0 * 0.35 = 0.00)
    [✓] Public API: Not public API (0.3 * 0.25 = 0.075)
    [✓] Pattern: No pattern usage (1.0 * 0.20 = 0.20)
    [✓] Static analysis: No warnings (0.7 * 0.15 = 0.105)

  Overall Score: 0.35 → LOW CONFIDENCE
  ⚠️  Requires manual verification - likely false positive
```
```

## Implementation Notes

### Signal Weight Rationale

- **Cross-file usage (35%)**: Strongest signal - direct evidence of usage
- **Public API (25%)**: Strong signal - intentional exposure
- **Pattern usage (20%)**: Strong signal - framework/design pattern requirement
- **Static analysis (15%)**: Moderate signal - indicates code health
- **Complexity (5%)**: Weak signal - doesn't indicate dead code

### Calibration

Validate confidence calibration:
```rust
// For findings with confidence > 0.8, < 5% should be false positives
let high_confidence_findings = findings.iter().filter(|f| f.confidence.score > 0.8);
let false_positive_rate = validate_findings(high_confidence_findings);
assert!(false_positive_rate < 0.05);
```

## Success Metrics

- **Calibration accuracy**: 90% of scores match actual false positive rates
- **High-confidence precision**: < 5% false positives for >0.8 confidence
- **User adoption**: 60% of users filter by confidence
- **Time savings**: 50% reduction in false positive investigation time
- **Trust improvement**: Positive user feedback on confidence accuracy

## Related Specifications

All prerequisite specs contribute signals to confidence scoring.
