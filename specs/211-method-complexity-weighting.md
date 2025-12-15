---
number: 211
title: Method Complexity Weighting in God Object Scoring
category: optimization
priority: low
status: draft
dependencies: [207, 209]
created: 2025-12-15
---

# Specification 211: Method Complexity Weighting in God Object Scoring

**Category**: optimization
**Priority**: low (P2)
**Status**: draft
**Dependencies**: Spec 207 (LOC Calculation Fix), Spec 209 (Accessor Detection)

## Context

The current God Object scoring formula treats method complexity uniformly. A struct with 15 simple methods and a struct with 15 highly complex methods get similar scores, despite having vastly different maintenance burdens.

### Current Problem

```rust
// Struct A: 15 simple methods
impl SimpleStruct {
    pub fn get_a(&self) -> i32 { self.a }
    pub fn get_b(&self) -> i32 { self.b }
    // ... 13 more trivial getters
}

// Struct B: 15 complex methods
impl ComplexStruct {
    pub fn analyze_workspace(&mut self, files: &[File]) -> Result<Analysis> {
        // 50 lines, cyclomatic complexity 12, deep nesting
    }
    pub fn build_dependency_graph(&self) -> Graph {
        // 40 lines, cyclomatic complexity 8, recursive calls
    }
    // ... 13 more complex methods
}

// Current scoring: Both get similar method_factor because method_count = 15
// Desired: ComplexStruct should score MUCH higher due to complexity burden
```

### Current Scoring Formula

```rust
let method_factor = (method_count as f64 / thresholds.max_methods as f64).min(3.0);
let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

let base_score = method_factor * field_factor * responsibility_factor * size_factor;
```

## Objective

Incorporate method complexity into God Object scoring to:
1. Weight complex methods higher than simple methods
2. Factor in cyclomatic complexity, nesting depth, and cognitive complexity
3. Produce scores that better reflect actual maintenance burden
4. Distinguish between "many simple methods" vs "many complex methods"

## Requirements

### Functional Requirements

1. **Complexity Aggregation**: Calculate aggregate complexity metrics per struct:
   - Total cyclomatic complexity
   - Average method complexity
   - Maximum method complexity
   - Complexity variance (spread)

2. **Weighted Scoring**: Incorporate complexity into scoring formula:
   - High average complexity → higher score
   - High max complexity → bonus penalty
   - High variance → indicates mixed quality

3. **Configurable Weights**: Allow tuning of complexity contribution

### Non-Functional Requirements

- Must not significantly slow down analysis
- Should produce scores in same 0-100 range as before
- Must be deterministic

## Acceptance Criteria

- [ ] Structs with high-complexity methods score higher than structs with low-complexity methods (same method count)
- [ ] A struct with one 50-complexity method scores higher than a struct with 10 5-complexity methods
- [ ] Complexity metrics (total, avg, max) are included in GodObjectAnalysis output
- [ ] Scoring formula incorporates complexity_factor
- [ ] Thresholds are configurable
- [ ] Existing tests continue to pass (with adjusted expectations where appropriate)
- [ ] New tests validate complexity weighting behavior

## Technical Details

### Implementation Approach

#### 1. Complexity Metrics Collection

```rust
#[derive(Debug, Clone, Default)]
pub struct ComplexityMetrics {
    /// Sum of cyclomatic complexity across all methods
    pub total_cyclomatic: u32,
    /// Sum of cognitive complexity across all methods
    pub total_cognitive: u32,
    /// Highest cyclomatic complexity of any single method
    pub max_cyclomatic: u32,
    /// Highest cognitive complexity of any single method
    pub max_cognitive: u32,
    /// Average cyclomatic complexity
    pub avg_cyclomatic: f64,
    /// Average cognitive complexity
    pub avg_cognitive: f64,
    /// Standard deviation of complexities (indicates variance)
    pub complexity_variance: f64,
    /// Maximum nesting depth across all methods
    pub max_nesting: u32,
}

pub fn calculate_complexity_metrics(
    method_complexities: &[MethodComplexity],
) -> ComplexityMetrics {
    if method_complexities.is_empty() {
        return ComplexityMetrics::default();
    }

    let cyclomatic_values: Vec<u32> = method_complexities
        .iter()
        .map(|m| m.cyclomatic)
        .collect();

    let cognitive_values: Vec<u32> = method_complexities
        .iter()
        .map(|m| m.cognitive)
        .collect();

    let total_cyclomatic: u32 = cyclomatic_values.iter().sum();
    let total_cognitive: u32 = cognitive_values.iter().sum();
    let max_cyclomatic = *cyclomatic_values.iter().max().unwrap_or(&0);
    let max_cognitive = *cognitive_values.iter().max().unwrap_or(&0);

    let n = method_complexities.len() as f64;
    let avg_cyclomatic = total_cyclomatic as f64 / n;
    let avg_cognitive = total_cognitive as f64 / n;

    // Calculate variance
    let variance: f64 = cyclomatic_values.iter()
        .map(|&c| (c as f64 - avg_cyclomatic).powi(2))
        .sum::<f64>() / n;
    let complexity_variance = variance.sqrt();

    let max_nesting = method_complexities.iter()
        .map(|m| m.max_nesting)
        .max()
        .unwrap_or(0);

    ComplexityMetrics {
        total_cyclomatic,
        total_cognitive,
        max_cyclomatic,
        max_cognitive,
        avg_cyclomatic,
        avg_cognitive,
        complexity_variance,
        max_nesting,
    }
}
```

#### 2. Complexity Factor Calculation

```rust
/// Calculate complexity factor for God Object scoring.
///
/// Returns a multiplier in range [0.5, 3.0]:
/// - 0.5-1.0: Low complexity methods (simple struct)
/// - 1.0-1.5: Average complexity
/// - 1.5-2.0: High complexity
/// - 2.0-3.0: Very high complexity (severe God Object)
pub fn calculate_complexity_factor(
    metrics: &ComplexityMetrics,
    thresholds: &ComplexityThresholds,
) -> f64 {
    // Average complexity contribution (40% weight)
    let avg_factor = (metrics.avg_cyclomatic / thresholds.target_avg_complexity)
        .clamp(0.5, 2.0);

    // Max complexity contribution (30% weight)
    // Penalize having any extremely complex method
    let max_factor = (metrics.max_cyclomatic as f64 / thresholds.max_method_complexity as f64)
        .clamp(0.5, 2.5);

    // Total complexity contribution (20% weight)
    let total_factor = (metrics.total_cyclomatic as f64 / thresholds.target_total_complexity as f64)
        .clamp(0.5, 2.0);

    // Variance contribution (10% weight)
    // High variance indicates inconsistent quality
    let variance_factor = (metrics.complexity_variance / 5.0)
        .clamp(0.8, 1.5);

    // Weighted combination
    let combined = avg_factor * 0.4
        + max_factor * 0.3
        + total_factor * 0.2
        + variance_factor * 0.1;

    combined.clamp(0.5, 3.0)
}

#[derive(Debug, Clone)]
pub struct ComplexityThresholds {
    /// Target average complexity (default: 5)
    pub target_avg_complexity: f64,
    /// Maximum acceptable single method complexity (default: 15)
    pub max_method_complexity: u32,
    /// Target total complexity for max_methods methods (default: 75)
    pub target_total_complexity: f64,
}

impl Default for ComplexityThresholds {
    fn default() -> Self {
        Self {
            target_avg_complexity: 5.0,
            max_method_complexity: 15,
            target_total_complexity: 75.0, // 15 methods * 5 avg
        }
    }
}
```

#### 3. Updated Scoring Formula

```rust
pub fn calculate_god_object_score_with_complexity(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_metrics: &ComplexityMetrics,
    thresholds: &GodObjectThresholds,
    complexity_thresholds: &ComplexityThresholds,
) -> f64 {
    // Existing factors
    let method_factor = (method_count as f64 / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // NEW: Complexity factor
    let complexity_factor = calculate_complexity_factor(complexity_metrics, complexity_thresholds);

    // Updated base score calculation
    // Complexity factor acts as a multiplier on the method contribution
    let adjusted_method_factor = method_factor * complexity_factor.sqrt();

    let base_score = adjusted_method_factor * field_factor * responsibility_factor * size_factor;

    // Violation-based minimum scores (unchanged)
    let violation_count = count_violations(
        method_count,
        field_count,
        responsibility_count,
        lines_of_code,
        complexity_metrics,
        thresholds,
    );

    if violation_count > 0 {
        let base_min_score = match violation_count {
            1 => 30.0,
            2 => 50.0,
            _ => 70.0,
        };
        let score = base_score * 20.0 * (violation_count as f64);
        score.max(base_min_score)
    } else {
        base_score * 10.0
    }
}

fn count_violations(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_metrics: &ComplexityMetrics,
    thresholds: &GodObjectThresholds,
) -> usize {
    let mut violations = 0;

    if method_count > thresholds.max_methods { violations += 1; }
    if field_count > thresholds.max_fields { violations += 1; }
    if responsibility_count > thresholds.max_traits { violations += 1; }
    if lines_of_code > thresholds.max_lines { violations += 1; }
    if complexity_metrics.total_cyclomatic > thresholds.max_complexity { violations += 1; }

    // NEW: Add complexity-based violations
    if complexity_metrics.max_cyclomatic > 25 { violations += 1; }
    if complexity_metrics.avg_cyclomatic > 10.0 { violations += 1; }

    violations
}
```

#### 4. Integration with Analysis

Update `analyze_single_struct` in `detector.rs`:

```rust
fn analyze_single_struct(...) -> Option<GodObjectAnalysis> {
    // ... existing code ...

    // Calculate complexity metrics for this struct's methods
    let method_complexities: Vec<MethodComplexity> = visitor
        .function_complexity
        .iter()
        .filter(|fc| method_names.contains(&fc.name))
        .map(|fc| MethodComplexity {
            cyclomatic: fc.cyclomatic_complexity,
            cognitive: fc.cognitive_complexity,
            max_nesting: fc.max_nesting,
        })
        .collect();

    let complexity_metrics = calculate_complexity_metrics(&method_complexities);

    // Use new scoring with complexity
    let god_object_score = calculate_god_object_score_with_complexity(
        method_count,
        field_count,
        responsibility_count,
        lines_of_code,
        &complexity_metrics,
        thresholds,
        &ComplexityThresholds::default(),
    );

    // Include metrics in analysis
    Some(GodObjectAnalysis {
        // ... existing fields ...
        complexity_metrics: Some(complexity_metrics),
        // ...
    })
}
```

### Scoring Examples

| Scenario | Methods | Avg CC | Max CC | Old Score | New Score | Change |
|----------|---------|--------|--------|-----------|-----------|--------|
| Simple getters | 15 | 1 | 2 | 70 | 55 | -15 |
| Mixed simple | 15 | 5 | 10 | 70 | 70 | 0 |
| High complexity | 15 | 12 | 25 | 70 | 95 | +25 |
| Few complex | 5 | 15 | 30 | 40 | 65 | +25 |
| Many trivial | 20 | 2 | 3 | 80 | 65 | -15 |

## Dependencies

- **Prerequisites**:
  - Spec 207: LOC Calculation Fix (accurate size metrics)
  - Spec 209: Accessor Detection (identifies trivial methods)
- **Affected Components**:
  - `scoring.rs`: New scoring function with complexity
  - `detector.rs`: Pass complexity metrics to scoring
  - `types.rs`: Add ComplexityMetrics to GodObjectAnalysis

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_complexity_factor_low() {
    let metrics = ComplexityMetrics {
        avg_cyclomatic: 2.0,
        max_cyclomatic: 5,
        total_cyclomatic: 20,
        complexity_variance: 1.0,
        ..Default::default()
    };
    let thresholds = ComplexityThresholds::default();

    let factor = calculate_complexity_factor(&metrics, &thresholds);
    assert!(factor < 1.0, "Low complexity should produce factor < 1.0");
}

#[test]
fn test_complexity_factor_high() {
    let metrics = ComplexityMetrics {
        avg_cyclomatic: 15.0,
        max_cyclomatic: 30,
        total_cyclomatic: 200,
        complexity_variance: 10.0,
        ..Default::default()
    };
    let thresholds = ComplexityThresholds::default();

    let factor = calculate_complexity_factor(&metrics, &thresholds);
    assert!(factor > 2.0, "High complexity should produce factor > 2.0");
}

#[test]
fn test_score_with_complexity_simple_struct() {
    let metrics = ComplexityMetrics {
        avg_cyclomatic: 1.5,
        max_cyclomatic: 3,
        total_cyclomatic: 15,
        ..Default::default()
    };

    let score = calculate_god_object_score_with_complexity(
        15, 5, 3, 200, &metrics,
        &GodObjectThresholds::default(),
        &ComplexityThresholds::default(),
    );

    // Simple methods should reduce score
    assert!(score < 70.0, "Simple methods should result in lower score");
}

#[test]
fn test_score_with_complexity_complex_struct() {
    let metrics = ComplexityMetrics {
        avg_cyclomatic: 12.0,
        max_cyclomatic: 25,
        total_cyclomatic: 180,
        ..Default::default()
    };

    let score = calculate_god_object_score_with_complexity(
        15, 5, 3, 200, &metrics,
        &GodObjectThresholds::default(),
        &ComplexityThresholds::default(),
    );

    // Complex methods should increase score
    assert!(score > 70.0, "Complex methods should result in higher score");
}
```

### Integration Tests

```rust
#[test]
fn test_simple_struct_scores_lower() {
    let simple_content = /* struct with 15 trivial getters */;
    let complex_content = /* struct with 15 complex methods */;

    let simple_score = analyze_content(simple_content).first().unwrap().god_object_score;
    let complex_score = analyze_content(complex_content).first().unwrap().god_object_score;

    assert!(
        complex_score > simple_score,
        "Complex struct ({}) should score higher than simple struct ({})",
        complex_score, simple_score
    );
}
```

## Documentation Requirements

- **Code Documentation**: Document complexity factor calculation in scoring.rs
- **User Documentation**: Explain how complexity affects God Object detection

## Implementation Notes

1. **Backwards Compatibility**: Provide `calculate_god_object_score` without complexity for legacy callers
2. **Default Thresholds**: Tuned to match existing behavior for average-complexity code
3. **Capping**: All factors capped to prevent extreme scores
4. **Square Root**: Using sqrt on complexity factor to moderate its impact

## Migration and Compatibility

- Existing scores will change (some up, some down based on complexity)
- Consider gradual rollout with complexity weight configurable
- Add `--no-complexity-weight` flag for backwards compatibility

## Estimated Effort

- Implementation: ~4 hours
- Testing: ~2 hours
- Documentation: ~0.5 hours
- Total: ~6.5 hours
