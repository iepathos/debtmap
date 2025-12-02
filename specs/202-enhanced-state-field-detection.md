---
number: 202
title: Enhanced State Field Detection with Pattern-Based Heuristics
category: optimization
priority: medium
status: draft
dependencies: [201]
created: 2025-12-02
---

# Specification 202: Enhanced State Field Detection with Pattern-Based Heuristics

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 201 (Data Flow Analysis)

## Context

**Current Limitation**: State machine pattern detection relies on a **hardcoded keyword list** that misses many legitimate state-related fields:

```rust
// From state_machine_pattern_detector.rs:22-27
const STATE_FIELD_KEYWORDS: &[&str] = &[
    "state", "mode", "status", "phase", "stage",
    "desired", "current", "target", "actual",
];

const STATE_PATH_KEYWORDS: &[&str] = &[
    "state", "mode", "status", "phase"
];
```

**Problems**:

1. **Missed Naming Conventions**: Common patterns not detected
   ```rust
   // Missed: FSM-related naming
   match self.fsm_state { ... }         // "fsm" not in keywords
   match self.transition { ... }         // "transition" not in keywords
   match self.step { ... }              // "step" not in keywords

   // Missed: Context/current patterns
   match ctx.kind { ... }               // "ctx" not in keywords
   match self.current_action { ... }    // Only "current" matches, not "current_*"
   match self.next_operation { ... }    // "next" not in keywords

   // Missed: Type-based state
   match self.request_type { ... }      // "type" not in keywords
   match self.connection_kind { ... }   // "kind" not in keywords
   ```

2. **Over-reliance on Exact Matches**: Doesn't recognize semantic patterns
   ```rust
   // Missed: Enum-based state without keyword
   enum ProcessingStep { Parse, Validate, Execute }
   match self.step { ... }  // "step" not in original keywords

   // Missed: Lifecycle stages
   match self.lifecycle { ... }  // "lifecycle" not in keywords
   match self.stage_of_processing { ... }  // Only "stage" matches
   ```

3. **No Type Analysis**: Ignores that enums with multiple variants are likely state
   ```rust
   // Should be detected: Enum with 3+ variants likely represents state
   enum ConnectionState { Idle, Connecting, Connected, Disconnected }
   match self.connection { ... }  // "connection" not in keywords
   ```

4. **No Pattern Frequency Analysis**: Doesn't learn from codebase patterns
   ```rust
   // If a field is frequently matched on, it's likely state-related
   match self.flow_control { ... }  // Used 10+ times across codebase
   match self.protocol_state { ... }  // Obvious state pattern
   ```

**Impact**:
- State machine detection confidence is artificially low (false negatives)
- Developers using non-standard naming conventions get poor analysis
- Type-based state machines (common in Rust) are missed
- Reduces accuracy of complexity pattern detection

## Objective

Implement **multi-strategy state field detection** combining:
1. **Extended keyword dictionary** with common naming patterns
2. **Type-based heuristics** (enum analysis, variant counting)
3. **Semantic pattern recognition** (naming conventions, prefixes/suffixes)
4. **Usage frequency analysis** (fields frequently matched on)
5. **Confidence scoring** for state field likelihood

## Requirements

### Functional Requirements

1. **Extended Keyword Dictionary**
   - Add common state-related terms missing from current list
   - Include FSM-specific terminology
   - Add lifecycle and transition-related terms
   - Support both field and path keywords

2. **Type-Based Detection**
   - Analyze enum types to determine if they represent state
   - Count enum variants (3+ variants → likely state)
   - Detect discriminated unions and ADTs
   - Recognize Option/Result wrapping state enums

3. **Semantic Pattern Recognition**
   - Detect common prefixes: `current_*`, `next_*`, `prev_*`, `target_*`
   - Detect common suffixes: `*_state`, `*_mode`, `*_status`, `*_phase`
   - Recognize compound patterns: `flow_control`, `request_type`, `connection_kind`
   - Support snake_case, camelCase, and PascalCase variations

4. **Usage Frequency Analysis**
   - Track how often fields are used in match expressions
   - Identify fields compared frequently (e.g., `self.x == State::A`)
   - Detect fields that appear in multiple state transitions
   - Build confidence score based on usage patterns

5. **Confidence Scoring System**
   - Multi-factor scoring: keyword match + type analysis + frequency
   - Threshold-based classification (high/medium/low confidence)
   - Explainable confidence (show which factors contributed)
   - Integrate with existing confidence system (Spec 116)

### Non-Functional Requirements

1. **Performance**: Field detection overhead < 5ms per function
2. **Accuracy**: Reduce false negatives by ≥40% for state detection
3. **Maintainability**: Modular design allowing easy keyword additions
4. **Backward Compatibility**: Existing patterns still detected
5. **Configurability**: Allow users to add custom state keywords

## Acceptance Criteria

- [ ] **Extended Keyword Dictionary**
  - 30+ total keywords covering common patterns
  - Separate lists for field keywords and path keywords
  - FSM-specific terms included (fsm, transition, automaton)
  - Lifecycle terms included (lifecycle, stage, step, iteration)

- [ ] **Type-Based Detection**
  - Correctly identifies enum types with 3+ variants as state
  - Handles Option<Enum> and Result<Enum, E> wrapping
  - Detects discriminated unions via enum analysis
  - Accuracy: ≥90% for enum-based state detection

- [ ] **Semantic Pattern Recognition**
  - Detects prefix patterns: current_*, next_*, prev_*, target_*
  - Detects suffix patterns: *_state, *_mode, *_status, *_type, *_kind
  - Handles snake_case, camelCase, and PascalCase
  - Compound pattern recognition: flow_control, request_handler, etc.

- [ ] **Usage Frequency Analysis**
  - Tracks match expression frequency per field
  - Identifies high-frequency fields (≥3 matches per file)
  - Builds usage heatmap across codebase
  - Confidence boost for frequently matched fields

- [ ] **Confidence Scoring**
  - Multi-factor score: keyword (0.3) + type (0.4) + frequency (0.3)
  - Thresholds: high (≥0.75), medium (0.5-0.75), low (<0.5)
  - Explainable output showing confidence breakdown
  - Integration with Spec 116 confidence system

- [ ] **False Negative Reduction**
  - Benchmark against test corpus of 100+ state machines
  - False negative rate reduced by ≥40%
  - No increase in false positive rate
  - Coverage improvement for non-standard naming conventions

- [ ] **Configuration Support**
  - Config file allows custom keyword additions
  - Per-project customization supported
  - Default keywords can be extended, not replaced
  - Documentation for adding custom patterns

- [ ] **Performance Validation**
  - Per-function overhead ≤5ms on average
  - Total analysis time increase <10%
  - Memory overhead ≤10MB for 100k LOC codebase
  - Benchmarks demonstrating performance characteristics

- [ ] **Test Coverage**
  - Unit tests for each detection strategy
  - Integration tests with state_machine_pattern_detector
  - Regression tests for existing patterns
  - Test corpus with diverse naming conventions

- [ ] **Documentation**
  - Architecture docs explaining detection strategies
  - User guide for custom keyword configuration
  - Examples of detected vs missed patterns
  - Confidence scoring explanation

## Technical Details

### Architecture Overview

```
┌────────────────────────────────────────────────────────────┐
│         Enhanced State Field Detection Pipeline             │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  1. Keyword-Based Detection (Baseline)                      │
│     - Extended keyword dictionary (30+ terms)               │
│     - Field and path keyword matching                       │
│     - Confidence: 0.3 base score                            │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  2. Type-Based Detection (Rust-Specific)                    │
│     - Enum variant counting (3+ variants)                   │
│     - Discriminated union analysis                          │
│     - Option/Result wrapper detection                       │
│     - Confidence: +0.4 if enum detected                     │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  3. Semantic Pattern Recognition                            │
│     - Prefix detection: current_*, next_*, prev_*           │
│     - Suffix detection: *_state, *_mode, *_type             │
│     - Compound patterns: flow_control, etc.                 │
│     - Confidence: +0.2 per pattern match                    │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  4. Usage Frequency Analysis                                │
│     - Count match expressions per field                     │
│     - Track comparison frequency                            │
│     - Build usage heatmap                                   │
│     - Confidence: +0.3 if high frequency (≥3)               │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  5. Confidence Aggregation                                  │
│     - Combine all detection strategies                      │
│     - Normalize to 0.0-1.0 range                            │
│     - Apply thresholds (high/medium/low)                    │
│     - Generate explainable confidence report                │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  6. Integration with StateMachinePatternDetector            │
│     - Enhanced state field identification                   │
│     - Higher confidence for state machine signals           │
│     - Reduced false negatives                               │
└────────────────────────────────────────────────────────────┘
```

### Core Data Structures

```rust
use std::collections::HashMap;
use syn::{Expr, ExprField, Type, Ident};

/// Enhanced state field detector
#[derive(Debug, Clone)]
pub struct StateFieldDetector {
    /// Extended keyword dictionary
    keywords: StateKeywordDict,

    /// Type information cache
    type_cache: HashMap<String, TypeInfo>,

    /// Usage frequency tracker
    usage_tracker: UsageTracker,

    /// Configuration
    config: StateDetectionConfig,
}

/// Extended keyword dictionary
#[derive(Debug, Clone)]
pub struct StateKeywordDict {
    /// Field keywords (for struct field access)
    pub field_keywords: Vec<String>,

    /// Path keywords (for variable/parameter names)
    pub path_keywords: Vec<String>,

    /// Prefix patterns
    pub prefix_patterns: Vec<String>,

    /// Suffix patterns
    pub suffix_patterns: Vec<String>,

    /// Compound patterns (exact match required)
    pub compound_patterns: Vec<String>,
}

impl Default for StateKeywordDict {
    fn default() -> Self {
        Self {
            field_keywords: vec![
                // Original keywords
                "state", "mode", "status", "phase", "stage",
                "desired", "current", "target", "actual",

                // NEW: FSM-specific
                "fsm", "transition", "automaton", "machine",

                // NEW: Lifecycle and flow
                "lifecycle", "step", "iteration", "round",
                "flow", "control", "sequence",

                // NEW: Type and kind
                "type", "kind", "variant", "form",

                // NEW: Connection and protocol
                "connection", "protocol", "handshake",

                // NEW: Request/response
                "request", "response", "reply",

                // NEW: Context
                "ctx", "context", "env", "environment",
            ]
            .into_iter()
            .map(String::from)
            .collect(),

            path_keywords: vec![
                // Original
                "state", "mode", "status", "phase",

                // NEW: Additional path patterns
                "fsm", "transition", "stage", "step",
                "ctx", "context", "kind", "type",
            ]
            .into_iter()
            .map(String::from)
            .collect(),

            prefix_patterns: vec![
                "current_", "next_", "prev_", "previous_",
                "target_", "desired_", "actual_", "expected_",
                "old_", "new_", "initial_", "final_",
            ]
            .into_iter()
            .map(String::from)
            .collect(),

            suffix_patterns: vec![
                "_state", "_mode", "_status", "_phase",
                "_stage", "_step", "_type", "_kind",
                "_variant", "_flag", "_control",
            ]
            .into_iter()
            .map(String::from)
            .collect(),

            compound_patterns: vec![
                "flow_control", "state_machine", "fsm_state",
                "request_type", "response_kind", "connection_state",
                "protocol_phase", "processing_stage", "lifecycle_step",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }
}

/// Type information for state detection
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// Is this an enum type?
    pub is_enum: bool,

    /// Number of enum variants (if enum)
    pub variant_count: usize,

    /// Variant names
    pub variants: Vec<String>,

    /// Is this a wrapped type? (Option<T>, Result<T, E>)
    pub is_wrapped: bool,

    /// Wrapped inner type name
    pub inner_type: Option<String>,
}

/// Usage frequency tracker
#[derive(Debug, Clone)]
pub struct UsageTracker {
    /// Count of match expressions per field
    match_counts: HashMap<String, usize>,

    /// Count of comparisons per field
    comparison_counts: HashMap<String, usize>,

    /// Total occurrences per field
    occurrence_counts: HashMap<String, usize>,
}

impl UsageTracker {
    pub fn new() -> Self {
        Self {
            match_counts: HashMap::new(),
            comparison_counts: HashMap::new(),
            occurrence_counts: HashMap::new(),
        }
    }

    pub fn record_match(&mut self, field: &str) {
        *self.match_counts.entry(field.to_string()).or_insert(0) += 1;
        *self.occurrence_counts.entry(field.to_string()).or_insert(0) += 1;
    }

    pub fn record_comparison(&mut self, field: &str) {
        *self.comparison_counts.entry(field.to_string()).or_insert(0) += 1;
        *self.occurrence_counts.entry(field.to_string()).or_insert(0) += 1;
    }

    pub fn get_frequency_score(&self, field: &str) -> f64 {
        let matches = self.match_counts.get(field).copied().unwrap_or(0);
        let comparisons = self.comparison_counts.get(field).copied().unwrap_or(0);

        // High frequency = likely state field
        let total = matches + comparisons;
        match total {
            0 => 0.0,
            1..=2 => 0.1,
            3..=5 => 0.3,
            _ => 0.4,
        }
    }
}

/// Detection result with confidence breakdown
#[derive(Debug, Clone)]
pub struct StateFieldDetection {
    /// Field name
    pub field_name: String,

    /// Overall confidence (0.0-1.0)
    pub confidence: f64,

    /// Confidence classification
    pub classification: ConfidenceClass,

    /// Breakdown of confidence sources
    pub breakdown: ConfidenceBreakdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceClass {
    High,    // ≥0.75
    Medium,  // 0.5-0.75
    Low,     // <0.5
}

#[derive(Debug, Clone)]
pub struct ConfidenceBreakdown {
    /// Keyword match contribution
    pub keyword_score: f64,

    /// Type-based contribution
    pub type_score: f64,

    /// Pattern recognition contribution
    pub pattern_score: f64,

    /// Frequency-based contribution
    pub frequency_score: f64,

    /// Explanation string
    pub explanation: String,
}

/// Configuration for state detection
#[derive(Debug, Clone)]
pub struct StateDetectionConfig {
    /// Enable type-based detection
    pub use_type_analysis: bool,

    /// Enable frequency analysis
    pub use_frequency_analysis: bool,

    /// Enable semantic pattern recognition
    pub use_pattern_recognition: bool,

    /// Minimum variant count for enum state detection
    pub min_enum_variants: usize,

    /// Custom keywords to add
    pub custom_keywords: Vec<String>,

    /// Custom patterns to add
    pub custom_patterns: Vec<String>,
}

impl Default for StateDetectionConfig {
    fn default() -> Self {
        Self {
            use_type_analysis: true,
            use_frequency_analysis: true,
            use_pattern_recognition: true,
            min_enum_variants: 3,
            custom_keywords: Vec::new(),
            custom_patterns: Vec::new(),
        }
    }
}
```

### Implementation: Multi-Strategy Detection

```rust
impl StateFieldDetector {
    pub fn new(config: StateDetectionConfig) -> Self {
        let mut keywords = StateKeywordDict::default();

        // Add custom keywords from config
        keywords.field_keywords.extend(config.custom_keywords.clone());
        keywords.compound_patterns.extend(config.custom_patterns.clone());

        Self {
            keywords,
            type_cache: HashMap::new(),
            usage_tracker: UsageTracker::new(),
            config,
        }
    }

    /// Detect if a field is likely state-related
    pub fn detect_state_field(&self, field_expr: &ExprField) -> StateFieldDetection {
        let field_name = match &field_expr.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(_) => return self.low_confidence_result("unnamed"),
        };

        let mut breakdown = ConfidenceBreakdown {
            keyword_score: 0.0,
            type_score: 0.0,
            pattern_score: 0.0,
            frequency_score: 0.0,
            explanation: String::new(),
        };

        // Strategy 1: Keyword matching (baseline)
        if self.matches_keyword(&field_name) {
            breakdown.keyword_score = 0.3;
            breakdown.explanation.push_str("keyword match; ");
        }

        // Strategy 2: Type-based detection
        if self.config.use_type_analysis {
            if let Some(type_info) = self.analyze_field_type(field_expr) {
                if type_info.is_enum && type_info.variant_count >= self.config.min_enum_variants {
                    breakdown.type_score = 0.4;
                    breakdown.explanation.push_str(&format!(
                        "enum with {} variants; ",
                        type_info.variant_count
                    ));
                }
            }
        }

        // Strategy 3: Semantic pattern recognition
        if self.config.use_pattern_recognition {
            let pattern_score = self.analyze_semantic_patterns(&field_name);
            breakdown.pattern_score = pattern_score;
            if pattern_score > 0.0 {
                breakdown.explanation.push_str("semantic pattern; ");
            }
        }

        // Strategy 4: Usage frequency
        if self.config.use_frequency_analysis {
            let freq_score = self.usage_tracker.get_frequency_score(&field_name);
            breakdown.frequency_score = freq_score;
            if freq_score > 0.0 {
                breakdown.explanation.push_str(&format!(
                    "high usage frequency (score: {:.2}); ",
                    freq_score
                ));
            }
        }

        // Aggregate confidence
        let confidence = breakdown.keyword_score
            + breakdown.type_score
            + breakdown.pattern_score
            + breakdown.frequency_score;

        let classification = match confidence {
            c if c >= 0.75 => ConfidenceClass::High,
            c if c >= 0.5 => ConfidenceClass::Medium,
            _ => ConfidenceClass::Low,
        };

        StateFieldDetection {
            field_name,
            confidence,
            classification,
            breakdown,
        }
    }

    /// Check if field name matches keyword dictionary
    fn matches_keyword(&self, field_name: &str) -> bool {
        let normalized = field_name.to_lowercase();

        // Exact compound pattern match
        if self.keywords.compound_patterns.iter().any(|p| normalized == p.to_lowercase()) {
            return true;
        }

        // Field keyword substring match
        self.keywords
            .field_keywords
            .iter()
            .any(|kw| normalized.contains(&kw.to_lowercase()))
    }

    /// Analyze semantic patterns (prefix/suffix)
    fn analyze_semantic_patterns(&self, field_name: &str) -> f64 {
        let normalized = field_name.to_lowercase();
        let mut score = 0.0;

        // Check prefix patterns
        for prefix in &self.keywords.prefix_patterns {
            if normalized.starts_with(&prefix.to_lowercase()) {
                score += 0.15;
                break;
            }
        }

        // Check suffix patterns
        for suffix in &self.keywords.suffix_patterns {
            if normalized.ends_with(&suffix.to_lowercase()) {
                score += 0.15;
                break;
            }
        }

        score.min(0.3) // Cap at 0.3
    }

    /// Analyze field type to detect enum-based state
    fn analyze_field_type(&self, field_expr: &ExprField) -> Option<TypeInfo> {
        // In real implementation, use type inference or cached type info
        // For now, simplified: check if field name suggests enum type

        // This would require integration with rustc's type system
        // or maintaining a type database from previous analysis passes

        // Placeholder: return cached type info if available
        let field_name = match &field_expr.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(_) => return None,
        };

        self.type_cache.get(&field_name).cloned()
    }

    /// Build type information database from AST
    pub fn build_type_database(&mut self, items: &[syn::Item]) {
        for item in items {
            if let syn::Item::Enum(enum_item) = item {
                let type_name = enum_item.ident.to_string();
                let variant_count = enum_item.variants.len();
                let variants: Vec<String> = enum_item
                    .variants
                    .iter()
                    .map(|v| v.ident.to_string())
                    .collect();

                self.type_cache.insert(
                    type_name.clone(),
                    TypeInfo {
                        is_enum: true,
                        variant_count,
                        variants,
                        is_wrapped: false,
                        inner_type: None,
                    },
                );
            }
        }
    }

    fn low_confidence_result(&self, field_name: &str) -> StateFieldDetection {
        StateFieldDetection {
            field_name: field_name.to_string(),
            confidence: 0.0,
            classification: ConfidenceClass::Low,
            breakdown: ConfidenceBreakdown {
                keyword_score: 0.0,
                type_score: 0.0,
                pattern_score: 0.0,
                frequency_score: 0.0,
                explanation: "no indicators".to_string(),
            },
        }
    }
}
```

### Integration with StateMachinePatternDetector

```rust
// In src/analyzers/state_machine_pattern_detector.rs

use crate::analyzers::state_field_detector::{StateFieldDetector, StateDetectionConfig};

impl StateMachinePatternDetector {
    pub fn new_with_enhanced_detection() -> Self {
        Self {
            state_detector: StateFieldDetector::new(StateDetectionConfig::default()),
            // ... other fields
        }
    }

    pub fn detect_state_machine(&self, block: &Block) -> Option<StateMachineSignals> {
        let mut visitor = StateMachineVisitor::new();
        visitor.visit_block(block);

        // OLD: Simple keyword check
        // if !visitor.has_enum_match && visitor.state_comparison_count == 0 {
        //     return None;
        // }

        // NEW: Enhanced state field detection with confidence
        let state_fields: Vec<_> = visitor
            .field_accesses
            .iter()
            .map(|field| self.state_detector.detect_state_field(field))
            .filter(|detection| detection.classification != ConfidenceClass::Low)
            .collect();

        if state_fields.is_empty() && !visitor.has_enum_match {
            return None;
        }

        // Calculate enhanced confidence using state field detections
        let field_confidence: f64 = state_fields
            .iter()
            .map(|d| d.confidence)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let confidence = calculate_enhanced_state_machine_confidence(
            visitor.enum_match_count,
            visitor.tuple_match_count,
            state_fields.len(),
            visitor.action_dispatch_count,
            field_confidence,
        );

        if confidence < 0.5 {  // Lowered threshold from 0.6
            return None;
        }

        Some(StateMachineSignals {
            transition_count: visitor.enum_match_count + visitor.tuple_match_count,
            has_enum_match: visitor.has_enum_match,
            has_state_comparison: !state_fields.is_empty(),
            action_dispatch_count: visitor.action_dispatch_count,
            confidence,
            state_field_detections: Some(state_fields),
        })
    }
}

/// Enhanced confidence calculation with state field detection
fn calculate_enhanced_state_machine_confidence(
    enum_match_count: usize,
    tuple_match_count: usize,
    state_field_count: usize,
    action_dispatch_count: usize,
    max_field_confidence: f64,
) -> f64 {
    let mut confidence = 0.0;

    // Enum/tuple matching (original logic)
    if enum_match_count > 0 || tuple_match_count >= 2 {
        confidence += 0.5;
    }

    // NEW: State field detection confidence
    if state_field_count > 0 {
        confidence += max_field_confidence * 0.4;  // Weight: 40% of field confidence
    }

    // Action dispatch (original logic)
    if action_dispatch_count >= 2 {
        confidence += 0.2;
    }

    confidence.min(1.0)
}
```

### Configuration File Support

```toml
# debtmap.toml

[state_detection]
# Enable type-based detection
use_type_analysis = true

# Enable frequency analysis
use_frequency_analysis = true

# Enable semantic pattern recognition
use_pattern_recognition = true

# Minimum enum variants to consider as state
min_enum_variants = 3

# Custom keywords specific to your domain
custom_keywords = [
    "workflow",
    "scenario",
    "operation",
    "command",
]

# Custom compound patterns
custom_patterns = [
    "workflow_state",
    "scenario_type",
    "operation_mode",
]
```

## Dependencies

### Spec 201: Data Flow Analysis

Data flow analysis can enhance state field detection by:
- Tracking how state fields are used across control flow
- Identifying fields that affect control flow decisions
- Building confidence based on data flow patterns

Integration:
```rust
// Use data flow to boost confidence for frequently-used state fields
if let Some(data_flow) = analysis.data_flow_info {
    if data_flow.affects_control_flow(&field_name) {
        confidence += 0.15;
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_keyword_detection_original() {
        let detector = StateFieldDetector::new(StateDetectionConfig::default());

        // Original keywords should still work
        assert!(detector.matches_keyword("state"));
        assert!(detector.matches_keyword("mode"));
        assert!(detector.matches_keyword("status"));
    }

    #[test]
    fn test_keyword_detection_new() {
        let detector = StateFieldDetector::new(StateDetectionConfig::default());

        // New keywords should be detected
        assert!(detector.matches_keyword("fsm"));
        assert!(detector.matches_keyword("transition"));
        assert!(detector.matches_keyword("lifecycle"));
        assert!(detector.matches_keyword("ctx"));
    }

    #[test]
    fn test_prefix_pattern() {
        let detector = StateFieldDetector::new(StateDetectionConfig::default());

        let score = detector.analyze_semantic_patterns("current_action");
        assert!(score > 0.0);

        let score = detector.analyze_semantic_patterns("next_step");
        assert!(score > 0.0);
    }

    #[test]
    fn test_suffix_pattern() {
        let detector = StateFieldDetector::new(StateDetectionConfig::default());

        let score = detector.analyze_semantic_patterns("connection_state");
        assert!(score > 0.0);

        let score = detector.analyze_semantic_patterns("request_type");
        assert!(score > 0.0);
    }

    #[test]
    fn test_compound_pattern() {
        let detector = StateFieldDetector::new(StateDetectionConfig::default());

        assert!(detector.matches_keyword("flow_control"));
        assert!(detector.matches_keyword("state_machine"));
    }

    #[test]
    fn test_confidence_aggregation() {
        let detector = StateFieldDetector::new(StateDetectionConfig::default());

        let field: ExprField = parse_quote! { self.fsm_state };
        let detection = detector.detect_state_field(&field);

        // Should have high confidence:
        // - keyword match ("fsm", "state"): 0.3
        // - compound pattern ("fsm_state"): bonus
        assert!(detection.confidence >= 0.3);
    }

    #[test]
    fn test_enum_type_detection() {
        let mut detector = StateFieldDetector::new(StateDetectionConfig::default());

        // Build type database with enum
        let items: Vec<syn::Item> = vec![parse_quote! {
            enum ConnectionState {
                Idle,
                Connecting,
                Connected,
                Disconnected,
            }
        }];

        detector.build_type_database(&items);

        // Check type info
        let type_info = detector.type_cache.get("ConnectionState").unwrap();
        assert!(type_info.is_enum);
        assert_eq!(type_info.variant_count, 4);
    }

    #[test]
    fn test_usage_frequency() {
        let mut tracker = UsageTracker::new();

        // Record multiple matches
        tracker.record_match("flow_control");
        tracker.record_match("flow_control");
        tracker.record_match("flow_control");

        let score = tracker.get_frequency_score("flow_control");
        assert!(score >= 0.3); // High frequency
    }

    #[test]
    fn test_custom_keywords() {
        let config = StateDetectionConfig {
            custom_keywords: vec!["workflow".to_string(), "scenario".to_string()],
            ..Default::default()
        };

        let detector = StateFieldDetector::new(config);
        assert!(detector.matches_keyword("workflow"));
        assert!(detector.matches_keyword("scenario"));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_state_machine_detection_with_enhanced_fields() {
    let code = r#"
        fn process(&self, request: &Request) -> Response {
            match self.fsm_state {  // "fsm_state" not in original keywords
                FsmState::Idle => self.handle_idle(request),
                FsmState::Processing => self.handle_processing(request),
                FsmState::Complete => self.handle_complete(request),
            }
        }
    "#;

    let detector = StateMachinePatternDetector::new_with_enhanced_detection();
    let result = analyze_with_detector(code, &detector);

    // Should detect state machine with high confidence
    assert!(result.is_some());
    let signals = result.unwrap();
    assert!(signals.confidence >= 0.6);
}

#[test]
fn test_prefix_suffix_pattern_detection() {
    let code = r#"
        fn update(&mut self) {
            match self.current_operation {  // "current_" prefix + compound
                Operation::Read => { /* ... */ }
                Operation::Write => { /* ... */ }
                Operation::Delete => { /* ... */ }
            }
        }
    "#;

    let detector = StateMachinePatternDetector::new_with_enhanced_detection();
    let result = analyze_with_detector(code, &detector);

    assert!(result.is_some());
}

#[test]
fn test_false_negative_reduction() {
    // Load test corpus of 100+ state machines with diverse naming
    let test_cases = load_state_machine_corpus();
    let detector = StateMachinePatternDetector::new_with_enhanced_detection();

    let mut detected = 0;
    for (code, expected_state_machine) in test_cases {
        if expected_state_machine {
            let result = analyze_with_detector(&code, &detector);
            if result.is_some() {
                detected += 1;
            }
        }
    }

    let detection_rate = detected as f64 / test_cases.len() as f64;

    // Should detect ≥60% more than original keyword-only approach
    assert!(detection_rate >= 0.85);
}
```

### Performance Benchmarks

```rust
#[bench]
fn bench_keyword_matching(b: &mut Bencher) {
    let detector = StateFieldDetector::new(StateDetectionConfig::default());
    b.iter(|| {
        detector.matches_keyword("fsm_state")
    });
}

#[bench]
fn bench_semantic_pattern(b: &mut Bencher) {
    let detector = StateFieldDetector::new(StateDetectionConfig::default());
    b.iter(|| {
        detector.analyze_semantic_patterns("current_operation_mode")
    });
}

#[bench]
fn bench_full_detection(b: &mut Bencher) {
    let detector = StateFieldDetector::new(StateDetectionConfig::default());
    let field: ExprField = parse_quote! { self.current_state };

    b.iter(|| {
        detector.detect_state_field(&field)
    });
}
```

## Documentation Requirements

### User Documentation

```markdown
## State Field Detection

Debtmap uses multiple strategies to detect state-related fields:

1. **Keyword Matching**: Recognizes common state-related terms
2. **Type Analysis**: Identifies enum types with multiple variants as state
3. **Pattern Recognition**: Detects prefix/suffix patterns like `current_*` and `*_state`
4. **Usage Frequency**: Fields frequently matched on are likely state

### Detected Patterns

**Keywords**: state, mode, status, phase, stage, fsm, transition, lifecycle, step, ctx, type, kind, connection, protocol, request, response

**Prefix Patterns**: current_*, next_*, prev_*, target_*, desired_*

**Suffix Patterns**: *_state, *_mode, *_status, *_type, *_kind, *_control

**Compound Patterns**: flow_control, state_machine, fsm_state, request_type, connection_state

### Custom Configuration

Add custom keywords in `debtmap.toml`:

```toml
[state_detection]
custom_keywords = ["workflow", "scenario", "operation"]
custom_patterns = ["workflow_state", "scenario_type"]
```

### Confidence Levels

- **High (≥0.75)**: Strong indicators present (keyword + type + frequency)
- **Medium (0.5-0.75)**: Multiple indicators present
- **Low (<0.5)**: Weak or no indicators
```

### Architecture Documentation

Add to `ARCHITECTURE.md`:

```markdown
## State Field Detection (Spec 202)

Enhanced multi-strategy approach for identifying state-related fields:

### Detection Strategies

1. **Keyword-Based** (baseline): Extended dictionary with 30+ terms
2. **Type-Based**: Enum analysis with variant counting
3. **Pattern-Based**: Prefix/suffix recognition
4. **Frequency-Based**: Usage heatmap tracking

### Confidence Scoring

- Keyword match: +0.3
- Enum with 3+ variants: +0.4
- Semantic pattern: +0.2
- High frequency (≥3 uses): +0.3

### Integration

- **StateMachinePatternDetector**: Enhanced state field identification
- **Data Flow Analysis (Spec 201)**: Control flow impact tracking
- **Confidence System (Spec 116)**: Multi-factor scoring
```

## Implementation Notes

### Phased Implementation

**Phase 1: Extended Keyword Dictionary** (Week 1)
- Add 20+ new keywords covering common patterns
- Update STATE_FIELD_KEYWORDS and STATE_PATH_KEYWORDS
- Backward compatibility testing

**Phase 2: Semantic Pattern Recognition** (Week 2)
- Implement prefix/suffix detection
- Add compound pattern matching
- Case-insensitive matching

**Phase 3: Type-Based Detection** (Week 3)
- Build type database from enum declarations
- Implement variant counting
- Handle Option/Result wrappers

**Phase 4: Usage Frequency Tracking** (Week 4)
- Implement UsageTracker
- Record match/comparison frequency
- Build confidence scoring

**Phase 5: Integration and Testing** (Week 5)
- Integrate with StateMachinePatternDetector
- Run false negative reduction tests
- Performance optimization

### Quick Wins

The extended keyword dictionary (Phase 1) can be implemented immediately with minimal risk and provides instant value.

## Migration and Compatibility

### Backward Compatibility

- All original keywords remain
- Existing state machines still detected
- Confidence thresholds adjusted (lowered from 0.6 to 0.5)
- No breaking changes to public APIs

### Gradual Rollout

1. **Alpha**: Extended keywords only (low risk)
2. **Beta**: Add pattern recognition
3. **Stable**: Full multi-strategy detection

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| False positive increase | Low | Medium | Conservative confidence thresholds |
| Performance overhead | Low | Low | Simple string matching, caching |
| Type database memory | Medium | Low | Lazy loading, per-file cache |
| Custom config complexity | Low | Low | Sensible defaults, clear documentation |
