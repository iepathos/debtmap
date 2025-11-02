---
number: 159
title: Evidence-Based Purity Confidence Scoring
category: optimization
priority: medium
status: draft
dependencies: [116, 156, 157, 158]
created: 2025-11-01
---

# Specification 159: Evidence-Based Purity Confidence Scoring

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 116 (Confidence System), 156-158 (Purity Analysis)

## Context

**Current Problem**: Confidence scores use arbitrary magic numbers (`purity_detector.rs:138-157`):
```rust
if self.accesses_external_state && !self.modifies_external_state {
    confidence *= 0.8;  // Why 0.8? No justification
}
if self.has_unsafe {
    confidence *= 0.95;  // Why 0.95? No evidence
}
```

**No Calibration**: These values are not evidence-based or validated against ground truth.

## Objective

Replace magic numbers with **Bayesian evidence model** calibrated against validation corpus.

## Requirements

1. **Evidence Collection**
   - Track positive evidence (increases confidence in purity classification)
   - Track negative evidence (decreases confidence)
   - Combine evidence using weighted model

2. **Calibration**
   - Validate against 500+ manually-tagged functions
   - Optimize weights to maximize AUC-ROC
   - Target calibration error <5%

3. **Transparency**
   - Output confidence breakdown showing evidence
   - Allow users to understand why confidence is X%

## Implementation

```rust
#[derive(Debug, Clone)]
pub struct PurityEvidence {
    // Positive evidence (+confidence)
    pub has_explicit_pure_attr: bool,       // +0.30
    pub all_params_immutable: bool,         // +0.15
    pub no_unsafe_blocks: bool,             // +0.10
    pub return_type_pure: bool,             // +0.10
    pub all_callees_known_pure: bool,       // +0.20

    // Negative evidence (-confidence)
    pub unknown_function_calls: usize,      // -0.05 each
    pub macro_calls: usize,                 // -0.03 each
    pub uses_generics: bool,                // -0.10
    pub complex_control_flow: bool,         // -0.05
    pub pointer_operations: usize,          // -0.08 each
}

impl PurityEvidence {
    pub fn calculate_confidence(&self) -> f64 {
        let mut score = 0.5; // Neutral starting point

        // Positive adjustments
        if self.has_explicit_pure_attr { score += 0.30; }
        if self.all_params_immutable { score += 0.15; }
        if self.no_unsafe_blocks { score += 0.10; }
        if self.return_type_pure { score += 0.10; }
        if self.all_callees_known_pure { score += 0.20; }

        // Negative adjustments
        score -= self.unknown_function_calls as f64 * 0.05;
        score -= self.macro_calls as f64 * 0.03;
        if self.uses_generics { score -= 0.10; }
        if self.complex_control_flow { score -= 0.05; }
        score -= self.pointer_operations as f64 * 0.08;

        score.clamp(0.1, 1.0)
    }

    pub fn explain(&self) -> String {
        format!(
            "Confidence breakdown:\n\
             + Positive: params_immutable={}, no_unsafe={}\n\
             - Negative: unknown_calls={}, macros={}",
            self.all_params_immutable,
            self.no_unsafe_blocks,
            self.unknown_function_calls,
            self.macro_calls
        )
    }
}
```

## Validation

Create ground truth corpus:
```
tests/purity_validation/
├── ground_truth.json (500 manually-tagged functions)
├── calibration.rs (optimize weights)
└── validation_report.rs (measure accuracy)
```

Target metrics:
- AUC-ROC > 0.90
- Calibration error < 5%
- Brier score < 0.15

## Testing

```rust
#[test]
fn test_high_confidence_pure() {
    let evidence = PurityEvidence {
        all_params_immutable: true,
        no_unsafe_blocks: true,
        return_type_pure: true,
        all_callees_known_pure: true,
        ..Default::default()
    };

    assert!(evidence.calculate_confidence() > 0.85);
}

#[test]
fn test_low_confidence_many_unknowns() {
    let evidence = PurityEvidence {
        unknown_function_calls: 5,
        macro_calls: 3,
        uses_generics: true,
        ..Default::default()
    };

    assert!(evidence.calculate_confidence() < 0.40);
}
```
