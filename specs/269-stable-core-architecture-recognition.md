---
number: 269
title: Stable Core Architecture Recognition
category: optimization
priority: medium
status: draft
dependencies: [267, 268]
created: 2025-01-10
---

# Specification 269: Stable Core Architecture Recognition

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [267 - Test Caller Filtering, 268 - File-Scope Analysis Improvements]

## Context

Debtmap currently treats all high-dependency code as risky without considering whether that dependency pattern is architecturally intentional. Stable, foundational modules that many other modules depend on are flagged as "critical blast radius" even when this coupling is by design.

### Problem Analysis

A real-world example from `cargo-cargofmt/src/formatting/overflow.rs`:
- **Instability: 0.26** (low - depends on few, depended upon by many)
- **Blast Radius: 121** (marked as "critical")
- **Coupling Classification: Stable Core**
- **Test Caller Ratio: >90%** (most callers are tests)

The low instability score indicates this is an intentional stable foundation. In the Stable Dependencies Principle (SDP) from Clean Architecture, stable modules **should** be depended upon by less stable modules. Flagging this as debt is a false positive.

### Architectural Context

Robert Martin's **Stable Dependencies Principle**:
> "Dependencies should be in the direction of stability."

A module with low instability (I = Ce/(Ca+Ce) < 0.3) that is highly depended upon is architecturally correct - it's a **stable core component**. The concern should be if unstable modules have high coupling, not stable ones.

### Root Cause

1. **No Architecture Recognition**: Debtmap treats all coupling as potential debt
2. **Instability Not Used in Scoring**: The calculated instability metric doesn't influence debt prioritization
3. **Test Coverage Not Recognized as Positive**: High test callers penalize instead of reward
4. **Missing "Intentional Core" Classification**: No way to mark modules as architectural foundations

## Objective

Add architectural pattern recognition to:
1. Identify "stable core" modules that should have high incoming dependencies
2. Recognize high test coverage as a positive architectural signal
3. Reduce false positives for well-designed foundational code
4. Provide architectural context in debt reports

## Requirements

### Functional Requirements

#### FR-1: Coupling Classification Enhancement
Enhance existing `CouplingClassification` with architectural context:
- `WellTestedCore`: Low instability + high test caller ratio (>80%)
- `StableFoundation`: Low instability + high production callers
- `ArchitecturalHub`: Central connector with balanced instability
- `UnstableHighCoupling`: High instability + high callers (actual debt)
- `LeafModule`: Low callers, high callees (normal dependency)

#### FR-2: Instability-Aware Scoring
Incorporate instability into debt scoring:
- Low instability (< 0.3) + high callers → reduced debt score
- High instability (> 0.7) + high callers → increased debt score
- Use instability as a multiplier on dependency factor

#### FR-3: Test Coverage Quality Signal
When test caller ratio exceeds threshold (default: 70%):
- Apply positive adjustment to debt score
- Note "well-tested" status in output
- Combine with instability for "stable core" detection

#### FR-4: Architectural Report Section
Add new output section for architectural analysis:
```markdown
## Architectural Analysis

### Stable Core Components
These modules have low instability and high incoming dependencies - this is architecturally correct:
- src/formatting/overflow.rs (I=0.26, 5 production callers, 85 test callers)
- src/parsing/tokens.rs (I=0.18, 12 production callers, 45 test callers)

### Architectural Concerns
These modules have high instability but many dependents - potential architectural debt:
- src/commands/format.rs (I=0.72, 8 production callers)
```

#### FR-5: Low Confidence Threshold
Items with `completeness_confidence < 0.5` should:
- Be flagged as "uncertain" in output
- Be excluded from top priority lists by default
- Include note explaining low confidence

### Non-Functional Requirements

#### NFR-1: No False Negatives
- Unstable high-coupling code should still be flagged
- Architectural recognition only reduces scores for genuinely stable code

#### NFR-2: Transparency
- Clear explanation of why a module is classified as "stable core"
- Show all inputs to classification (instability, test ratio, etc.)

## Acceptance Criteria

- [ ] `CouplingClassification` enum expanded with `WellTestedCore` and `UnstableHighCoupling` variants
- [ ] Instability metric used in dependency factor calculation
- [ ] Test caller ratio factors into classification (requires spec 267)
- [ ] Stable core modules (I < 0.3, test_ratio > 0.7) get reduced scores
- [ ] Unstable high-coupling modules (I > 0.7, callers > 10) get increased scores
- [ ] LLM output includes "Architectural Analysis" section
- [ ] Items with confidence < 0.5 marked as "uncertain"
- [ ] Unit tests verify classification logic for all coupling types
- [ ] Integration test shows `overflow.rs` classified as `WellTestedCore`

## Technical Details

### Implementation Approach

#### Phase 1: Enhanced Coupling Classification

Update `src/priority/scoring/classification.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CouplingClassification {
    // Existing
    StableCore,
    ArchitecturalHub,
    LeafModule,

    // NEW: Architecture-aware classifications
    WellTestedCore,       // Low instability + high test coverage
    StableFoundation,     // Low instability + high production callers
    UnstableHighCoupling, // High instability + many callers (debt!)
    Isolated,             // Few callers and callees
}

impl CouplingClassification {
    pub fn is_architectural_concern(&self) -> bool {
        matches!(self, Self::UnstableHighCoupling | Self::ArchitecturalHub)
    }

    pub fn is_stable_by_design(&self) -> bool {
        matches!(self, Self::WellTestedCore | Self::StableFoundation | Self::StableCore)
    }

    pub fn score_multiplier(&self) -> f64 {
        match self {
            Self::WellTestedCore => 0.3,       // 70% reduction
            Self::StableFoundation => 0.5,     // 50% reduction
            Self::StableCore => 0.6,           // 40% reduction
            Self::LeafModule => 0.8,           // 20% reduction
            Self::Isolated => 0.9,             // 10% reduction
            Self::ArchitecturalHub => 1.0,     // No change
            Self::UnstableHighCoupling => 1.5, // 50% increase
        }
    }
}
```

#### Phase 2: Classification Logic

Create `src/priority/architecture_recognition.rs`:
```rust
/// Classify coupling pattern based on architectural metrics
pub fn classify_coupling_pattern(
    instability: f64,
    production_caller_count: usize,
    test_caller_count: usize,
    callee_count: usize,
) -> CouplingClassification {
    let total_callers = production_caller_count + test_caller_count;
    let test_ratio = if total_callers > 0 {
        test_caller_count as f64 / total_callers as f64
    } else {
        0.0
    };

    // Classification decision tree
    match (instability, total_callers, test_ratio) {
        // Well-tested core: stable + mostly test callers
        (i, c, t) if i < 0.3 && c > 5 && t > 0.7 => CouplingClassification::WellTestedCore,

        // Stable foundation: stable + many production callers
        (i, _, _) if i < 0.3 && production_caller_count > 10 => {
            CouplingClassification::StableFoundation
        }

        // Stable core: stable + moderate callers
        (i, c, _) if i < 0.3 && c > 5 => CouplingClassification::StableCore,

        // Unstable high coupling: unstable + many callers (DEBT)
        (i, _, _) if i > 0.7 && production_caller_count > 5 => {
            CouplingClassification::UnstableHighCoupling
        }

        // Architectural hub: balanced instability + high coupling
        (i, c, _) if i > 0.3 && i < 0.7 && c > 10 => CouplingClassification::ArchitecturalHub,

        // Leaf module: depends on many, few depend on it
        (_, c, _) if c < 3 && callee_count > 5 => CouplingClassification::LeafModule,

        // Isolated: minimal coupling
        (_, c, _) if c < 3 && callee_count < 3 => CouplingClassification::Isolated,

        // Default to leaf module
        _ => CouplingClassification::LeafModule,
    }
}

/// Calculate instability metric: I = Ce / (Ca + Ce)
/// Where Ce = efferent coupling (outgoing), Ca = afferent coupling (incoming)
pub fn calculate_instability(incoming_count: usize, outgoing_count: usize) -> f64 {
    let total = incoming_count + outgoing_count;
    if total == 0 {
        return 0.5; // Neutral instability for isolated components
    }
    outgoing_count as f64 / total as f64
}
```

#### Phase 3: Scoring Integration

Update `src/priority/scoring/calculation.rs`:
```rust
/// Calculate dependency factor with architectural awareness
pub fn calculate_architectural_dependency_factor(
    production_upstream_count: usize,
    test_upstream_count: usize,
    downstream_count: usize,
) -> (f64, CouplingClassification) {
    let instability = calculate_instability(
        production_upstream_count + test_upstream_count,
        downstream_count,
    );

    let classification = classify_coupling_pattern(
        instability,
        production_upstream_count,
        test_upstream_count,
        downstream_count,
    );

    // Base factor from production callers only (spec 267)
    let base_factor = calculate_dependency_factor(production_upstream_count);

    // Apply architectural multiplier
    let adjusted_factor = base_factor * classification.score_multiplier();

    (adjusted_factor, classification)
}
```

#### Phase 4: Confidence Threshold

Update `src/priority/filtering.rs`:
```rust
const LOW_CONFIDENCE_THRESHOLD: f64 = 0.5;

/// Filter out low-confidence items from top priority list
pub fn filter_uncertain_items(items: &[UnifiedDebtItem]) -> Vec<&UnifiedDebtItem> {
    items
        .iter()
        .filter(|item| item.completeness_confidence >= LOW_CONFIDENCE_THRESHOLD)
        .collect()
}

/// Mark items with low confidence
pub fn annotate_confidence(item: &mut UnifiedDebtItem) {
    if item.completeness_confidence < LOW_CONFIDENCE_THRESHOLD {
        item.confidence_note = Some(format!(
            "Low confidence ({:.0}%) - metrics may be incomplete",
            item.completeness_confidence * 100.0
        ));
    }
}
```

#### Phase 5: Architectural Report Section

Update `src/io/writers/llm_markdown.rs`:
```rust
fn write_architectural_analysis(
    items: &[UnifiedDebtItem],
    out: &mut String,
) {
    writeln!(out, "## Architectural Analysis\n");

    // Group by classification
    let stable_cores: Vec<_> = items
        .iter()
        .filter(|i| i.coupling_classification.is_stable_by_design())
        .collect();

    let concerns: Vec<_> = items
        .iter()
        .filter(|i| i.coupling_classification.is_architectural_concern())
        .collect();

    if !stable_cores.is_empty() {
        writeln!(out, "### Stable Core Components");
        writeln!(out, "These modules have low instability and high incoming dependencies - architecturally correct:\n");
        for item in stable_cores.iter().take(10) {
            writeln!(out, "- {} (I={:.2}, {} production, {} test callers)",
                item.location.file.display(),
                item.instability,
                item.production_upstream_count,
                item.test_upstream_count
            );
        }
    }

    if !concerns.is_empty() {
        writeln!(out, "\n### Architectural Concerns");
        writeln!(out, "These modules may have problematic coupling patterns:\n");
        for item in concerns.iter().take(10) {
            writeln!(out, "- {} ({:?}, I={:.2}, {} callers)",
                item.location.file.display(),
                item.coupling_classification,
                item.instability,
                item.production_upstream_count
            );
        }
    }
}
```

### Architecture Changes

- New module: `src/priority/architecture_recognition.rs`
- Enhanced `CouplingClassification` enum
- Modified scoring pipeline to incorporate instability
- New LLM output section for architectural analysis

### Data Structures

```rust
/// Extended debt item with architectural context
pub struct UnifiedDebtItem {
    // Existing fields...

    // NEW: Architectural metrics
    pub instability: f64,
    pub coupling_classification: CouplingClassification,
    pub production_upstream_count: usize,  // From spec 267
    pub test_upstream_count: usize,        // From spec 267
    pub completeness_confidence: f64,
    pub confidence_note: Option<String>,
}
```

### APIs and Interfaces

New public functions:
- `classify_coupling_pattern(instability, prod_callers, test_callers, callees) -> CouplingClassification`
- `calculate_instability(incoming, outgoing) -> f64`
- `calculate_architectural_dependency_factor(prod, test, downstream) -> (f64, CouplingClassification)`
- `filter_uncertain_items(items) -> Vec<&UnifiedDebtItem>`

## Dependencies

- **Prerequisites**:
  - Spec 267 (Test Caller Filtering) - for separate test/production caller counts
  - Spec 268 (File-Scope Analysis) - for distribution metrics
- **Affected Components**:
  - `src/priority/scoring/classification.rs`
  - `src/priority/scoring/calculation.rs`
  - `src/io/writers/llm_markdown.rs`
  - `src/output/unified/types.rs`
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_well_tested_core_classification() {
    let classification = classify_coupling_pattern(
        0.2,  // Low instability
        5,    // Few production callers
        85,   // Many test callers
        10,   // Some callees
    );
    assert_eq!(classification, CouplingClassification::WellTestedCore);
}

#[test]
fn test_unstable_high_coupling_classification() {
    let classification = classify_coupling_pattern(
        0.8,  // High instability
        15,   // Many production callers
        5,    // Few test callers
        20,   // Many callees
    );
    assert_eq!(classification, CouplingClassification::UnstableHighCoupling);
}

#[test]
fn test_score_multipliers() {
    assert!(CouplingClassification::WellTestedCore.score_multiplier() < 0.5);
    assert!(CouplingClassification::UnstableHighCoupling.score_multiplier() > 1.0);
}

#[test]
fn test_instability_calculation() {
    // Stable: many incoming, few outgoing
    assert!(calculate_instability(100, 10) < 0.2);

    // Unstable: few incoming, many outgoing
    assert!(calculate_instability(10, 100) > 0.8);

    // Balanced
    assert!((calculate_instability(50, 50) - 0.5).abs() < 0.01);
}
```

### Integration Tests

```rust
#[test]
fn test_overflow_rs_classified_as_well_tested_core() {
    let analysis = analyze_file("fixtures/overflow.rs");

    assert_eq!(
        analysis.coupling_classification,
        CouplingClassification::WellTestedCore
    );
    assert!(analysis.final_score < 30.0);  // Not high priority
}

#[test]
fn test_god_function_with_unstable_coupling_flagged() {
    let analysis = analyze_file("fixtures/unstable_god_object.rs");

    assert_eq!(
        analysis.coupling_classification,
        CouplingClassification::UnstableHighCoupling
    );
    assert!(analysis.final_score > 50.0);  // High priority
}
```

## Documentation Requirements

- **Code Documentation**: Document classification criteria and thresholds
- **User Documentation**: Explain architectural analysis section and classifications
- **Architecture Updates**: Add section on Stable Dependencies Principle integration

## Implementation Notes

### Threshold Rationale

- **Instability < 0.3**: Based on Clean Architecture guidance for "stable" modules
- **Instability > 0.7**: Indicates highly volatile module
- **Test ratio > 0.7**: Strong indication of intentional core component
- **Confidence < 0.5**: Analysis results are unreliable

### Clean Architecture Alignment

This implementation aligns with Robert Martin's SOLID principles:
- **Stable Dependencies Principle**: Dependencies flow toward stability
- **Stable Abstractions Principle**: Stable modules should be abstract

By recognizing stable cores as intentional architecture, we avoid flagging well-designed systems as debt.

## Migration and Compatibility

- No breaking changes
- New classification variants are additive
- Existing `CouplingClassification` users see enriched data
- Output formats enhanced with architectural sections
- Score adjustments are opt-in via configuration flag initially
