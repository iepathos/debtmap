---
number: 101
title: Fix Score Normalization with Logarithmic Scaling
category: optimization
priority: high
status: draft
dependencies: [96]
created: 2025-09-07
---

# Specification 101: Fix Score Normalization with Logarithmic Scaling

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [96 - Remove Score Capping]

## Context

After removing the 10.0 score cap (Spec 96), scores could theoretically grow unbounded, making it difficult to compare items. The current normalization function caps scores at 10.0, which defeats the purpose of removing the cap. Instead, we need logarithmic scaling for very high scores to maintain score distinction while keeping values manageable and comparable.

## Objective

Replace the current capping-based normalization with logarithmic scaling that preserves score differentiation for high-debt items while keeping scores in a reasonable range for display and comparison.

## Requirements

### Functional Requirements
- Scores below 10 remain unchanged (linear)
- Scores 10-100 use square root scaling
- Scores above 100 use logarithmic scaling
- All scores remain distinguishable (no identical scores)
- Scaling function must be monotonic (preserves ordering)

### Non-Functional Requirements
- Smooth transitions between scaling regions
- Reversible transformation for score interpretation
- Efficient calculation (no complex math)
- Clear documentation of scaling ranges

## Acceptance Criteria

- [ ] New normalization function implemented
- [ ] No two different raw scores produce identical normalized scores
- [ ] Scores maintain relative ordering after normalization
- [ ] Display shows both normalized and raw scores
- [ ] Function is continuous at transition points
- [ ] Tests verify scaling behavior across ranges
- [ ] Documentation explains normalization clearly

## Technical Details

### Normalization Function

```rust
// src/priority/scoring/calculation.rs
pub fn normalize_final_score(raw_score: f64) -> NormalizedScore {
    let normalized = if raw_score <= 0.0 {
        0.0
    } else if raw_score <= 10.0 {
        // Linear scaling for low scores (unchanged)
        raw_score
    } else if raw_score <= 100.0 {
        // Square root scaling for medium scores
        // Maps 10-100 to 10-40 range
        10.0 + (raw_score - 10.0).sqrt() * 3.33
    } else {
        // Logarithmic scaling for high scores
        // Maps 100+ to 40+ range with slow growth
        40.0 + (raw_score / 100.0).ln() * 10.0
    };
    
    NormalizedScore {
        raw: raw_score,
        normalized,
        scaling_method: determine_scaling_method(raw_score),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedScore {
    pub raw: f64,
    pub normalized: f64,
    pub scaling_method: ScalingMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingMethod {
    Linear,      // 0-10
    SquareRoot,  // 10-100
    Logarithmic, // 100+
}

fn determine_scaling_method(score: f64) -> ScalingMethod {
    if score <= 10.0 {
        ScalingMethod::Linear
    } else if score <= 100.0 {
        ScalingMethod::SquareRoot
    } else {
        ScalingMethod::Logarithmic
    }
}
```

### Inverse Function (for interpretation)

```rust
pub fn denormalize_score(normalized: f64) -> f64 {
    if normalized <= 0.0 {
        0.0
    } else if normalized <= 10.0 {
        // Linear range
        normalized
    } else if normalized <= 40.0 {
        // Square root range (inverse)
        let adjusted = (normalized - 10.0) / 3.33;
        10.0 + adjusted.powf(2.0)
    } else {
        // Logarithmic range (inverse)
        let log_component = (normalized - 40.0) / 10.0;
        100.0 * log_component.exp()
    }
}
```

### Display Integration

```rust
// src/priority/formatter.rs
fn format_score_with_normalization(score: &NormalizedScore) -> String {
    match score.scaling_method {
        ScalingMethod::Linear => {
            format!("{:.2}", score.normalized)
        },
        ScalingMethod::SquareRoot => {
            format!("{:.1} (raw: {:.0})", score.normalized, score.raw)
        },
        ScalingMethod::Logarithmic => {
            format!("{:.1} (raw: {:.0})", score.normalized, score.raw)
        }
    }
}

fn format_score_indicator(score: &NormalizedScore) -> &str {
    match score.normalized {
        s if s >= 50.0 => "🔴",  // Critical (raw >500)
        s if s >= 40.0 => "🟠",  // High (raw >100)
        s if s >= 20.0 => "🟡",  // Medium (raw >30)
        s if s >= 10.0 => "🟢",  // Low (raw >10)
        _ => "⚪",                // Minimal
    }
}
```

### Continuity Verification

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normalization_continuity() {
        // Test continuity at transition points
        let eps = 0.001;
        
        // At 10.0 transition
        let below_10 = normalize_final_score(10.0 - eps);
        let at_10 = normalize_final_score(10.0);
        let above_10 = normalize_final_score(10.0 + eps);
        assert!((at_10.normalized - below_10.normalized).abs() < 0.1);
        assert!((above_10.normalized - at_10.normalized).abs() < 0.1);
        
        // At 100.0 transition
        let below_100 = normalize_final_score(100.0 - eps);
        let at_100 = normalize_final_score(100.0);
        let above_100 = normalize_final_score(100.0 + eps);
        assert!((at_100.normalized - below_100.normalized).abs() < 0.1);
        assert!((above_100.normalized - at_100.normalized).abs() < 0.1);
    }
    
    #[test]
    fn test_normalization_monotonic() {
        // Verify ordering is preserved
        let scores = vec![1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0];
        let normalized: Vec<_> = scores.iter()
            .map(|&s| normalize_final_score(s).normalized)
            .collect();
        
        for i in 1..normalized.len() {
            assert!(normalized[i] > normalized[i-1]);
        }
    }
    
    #[test]
    fn test_inverse_function() {
        let test_scores = vec![5.0, 15.0, 50.0, 150.0, 500.0];
        
        for score in test_scores {
            let normalized = normalize_final_score(score);
            let denormalized = denormalize_score(normalized.normalized);
            assert!((denormalized - score).abs() < 0.1);
        }
    }
}
```

### Expected Normalized Ranges

| Raw Score Range | Normalized Range | Scaling Method | Use Case |
|----------------|------------------|----------------|----------|
| 0-10 | 0-10 | Linear | Minor issues |
| 10-30 | 10-20 | Square Root | Medium issues |
| 30-100 | 20-40 | Square Root | Major issues |
| 100-500 | 40-50 | Logarithmic | Critical issues |
| 500+ | 50+ | Logarithmic | God objects |

### Configuration

```rust
// src/config.rs
#[derive(Debug, Clone, Deserialize)]
pub struct NormalizationConfig {
    pub linear_threshold: f64,      // Default: 10.0
    pub logarithmic_threshold: f64, // Default: 100.0
    pub sqrt_multiplier: f64,       // Default: 3.33
    pub log_multiplier: f64,        // Default: 10.0
    pub show_raw_scores: bool,      // Default: true
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 96 (Remove Score Capping) - must be completed first
- **Affected Components**: 
  - Score calculation modules
  - Display/formatting modules
  - Sorting and ranking logic
  - Test suites
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test normalization function directly
- **Continuity Tests**: Verify smooth transitions
- **Monotonicity Tests**: Ensure ordering preserved
- **Inverse Tests**: Verify denormalization accuracy
- **Display Tests**: Check formatting at all ranges
- **Integration Tests**: End-to-end scoring with normalization

## Documentation Requirements

- **Normalization Guide**: Explain the scaling regions
- **Score Interpretation**: How to read normalized vs raw
- **Mathematical Documentation**: Provide formulas
- **Examples**: Show real examples at each range

## Implementation Notes

1. **Performance Optimization**:
   ```rust
   // Pre-compute common values
   const SQRT_MULTIPLIER: f64 = 3.33;
   const LOG_MULTIPLIER: f64 = 10.0;
   const LINEAR_THRESHOLD: f64 = 10.0;
   const LOG_THRESHOLD: f64 = 100.0;
   ```

2. **Visualization Support**:
   ```rust
   // Generate normalization curve for documentation
   pub fn generate_normalization_curve() -> Vec<(f64, f64)> {
       (0..1000).map(|i| {
           let raw = i as f64;
           let normalized = normalize_final_score(raw).normalized;
           (raw, normalized)
       }).collect()
   }
   ```

3. **Smooth Transition Alternative**:
   ```rust
   // Alternative: Use smooth interpolation at boundaries
   pub fn smooth_normalize(raw: f64) -> f64 {
       // Sigmoid-based smooth transition
       // Provides perfectly smooth curve
   }
   ```

## Migration and Compatibility

- Significant change in score display
- Provide mapping table for score interpretation
- Update all documentation with new ranges
- Consider showing both old and new scores during transition