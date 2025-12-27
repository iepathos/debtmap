# Rebalanced Scoring

Rebalanced scoring is a multi-dimensional scoring algorithm that prioritizes actual code quality issues over pure file size concerns. It provides a more nuanced approach to technical debt prioritization by considering complexity, coverage gaps, structural problems, and code smells.

## Overview

Traditional scoring often over-emphasizes file size, causing large but simple files to rank higher than complex, untested code. The rebalanced algorithm fixes this by:

1. **De-emphasizing size**: Reduces size weight from ~1.5 to 0.3 (80% reduction)
2. **Emphasizing quality**: Increases weights for complexity (1.0) and coverage gaps (1.0)
3. **Additive bonuses**: Provides +20 bonus for complex + untested code (not multiplicative)
4. **Context-aware detection**: Automatically detects and reduces scores for generated code

**Source**: `src/priority/scoring/rebalanced.rs:1-10`

## Severity Levels

The rebalanced algorithm assigns severity levels based on normalized total scores and risk factors. Scores are normalized to a 0-200 range.

**Source**: `src/priority/scoring/rebalanced.rs:12-30` (Severity enum), `src/priority/scoring/rebalanced.rs:416-448` (determine_severity)

| Severity | Score Threshold | Additional Criteria | Description |
|----------|-----------------|---------------------|-------------|
| **CRITICAL** | > 120 | OR complexity > 60 AND coverage > 40 | Requires immediate attention |
| **HIGH** | > 80 | OR complexity > 40 AND coverage > 20 OR structural > 50 | High priority for next sprint |
| **MEDIUM** | > 40 | OR single moderate issue (complexity/coverage/structural > 30) | Plan for future sprint |
| **LOW** | Everything else | - | Minor concerns, size-only issues |

**Severity Determination Logic** (from `src/priority/scoring/rebalanced.rs:416-448`):

```rust
fn determine_severity(components: &ScoreComponents, ...) -> Severity {
    let total = components.weighted_total(&ScoreWeights::default());

    // CRITICAL: Total score > 120 OR high complexity + low coverage
    if total > 120.0 || (components.complexity_score > 60.0 && components.coverage_score > 40.0) {
        return Severity::Critical;
    }

    // HIGH: Total score > 80 OR moderate complexity + coverage gap OR severe structural issue
    if total > 80.0
        || (components.complexity_score > 40.0 && components.coverage_score > 20.0)
        || components.structural_score > 50.0 {
        return Severity::High;
    }

    // MEDIUM: Total score > 40 OR single moderate issue
    if total > 40.0
        || components.complexity_score > 30.0
        || components.coverage_score > 30.0
        || components.structural_score > 30.0 {
        return Severity::Medium;
    }

    // LOW: Everything else
    Severity::Low
}
```

## Score Components

The rebalanced algorithm computes five distinct scoring components, each contributing to the weighted total.

**Source**: `src/priority/scoring/rebalanced.rs:32-55` (ScoreComponents struct)

| Component | Weight Range | Default Weight | Description |
|-----------|--------------|----------------|-------------|
| **complexity_score** | 0-100 | 1.0 | Cyclomatic + cognitive complexity combined |
| **coverage_score** | 0-80 | 1.0 | Testing coverage deficit with complexity bonus |
| **structural_score** | 0-60 | 0.8 | God objects and architectural issues |
| **size_score** | 0-30 | 0.3 | File/function size (reduced from legacy ~1.5) |
| **smell_score** | 0-40 | 0.6 | Long functions, deep nesting, impure logic |

### ScoreComponents Struct

```rust
/// Individual scoring components with their contributions
pub struct ScoreComponents {
    pub complexity_score: f64, // Weight: 0-100
    pub coverage_score: f64,   // Weight: 0-80
    pub structural_score: f64, // Weight: 0-60
    pub size_score: f64,       // Weight: 0-30 (reduced from current)
    pub smell_score: f64,      // Weight: 0-40
}
```

**Source**: `src/priority/scoring/rebalanced.rs:32-40`

### Weighted Total Calculation

The weighted total is normalized to a 0-200 range:

```rust
pub fn weighted_total(&self, weights: &ScoreWeights) -> f64 {
    let raw_total = self.complexity_score * weights.complexity_weight
        + self.coverage_score * weights.coverage_weight
        + self.structural_score * weights.structural_weight
        + self.size_score * weights.size_weight
        + self.smell_score * weights.smell_weight;

    // Normalize to 0-200 range
    // Theoretical max: 100×1.0 + 80×1.0 + 60×0.8 + 30×0.3 + 40×0.6 = 237
    (raw_total / 237.0) * 200.0
}
```

**Source**: `src/priority/scoring/rebalanced.rs:44-54`

## Presets

Debtmap provides four weight presets for different prioritization strategies.

**Source**: `src/priority/scoring/rebalanced.rs:73-127` (ScoreWeights presets)

### Balanced (Default)

The default preset prioritizes complexity and coverage over size.

```toml
[scoring_rebalanced]
preset = "balanced"
```

**Weights**:
- complexity_weight: 1.0
- coverage_weight: 1.0
- structural_weight: 0.8
- size_weight: 0.3
- smell_weight: 0.6

**Use when**: Standard development with focus on actual code quality.

**Source**: `src/priority/scoring/rebalanced.rs:74-83`

### Quality-Focused

Maximum emphasis on code quality, minimal concern for file size.

```toml
[scoring_rebalanced]
preset = "quality-focused"
```

**Weights**:
- complexity_weight: 1.2
- coverage_weight: 1.1
- structural_weight: 0.9
- size_weight: 0.2
- smell_weight: 0.7

**Use when**: You want maximum emphasis on code quality over size.

**Source**: `src/priority/scoring/rebalanced.rs:85-94`

### Size-Focused (Legacy)

Restores legacy behavior for backward compatibility.

```toml
[scoring_rebalanced]
preset = "size-focused"
```

**Weights**:
- complexity_weight: 0.5
- coverage_weight: 0.4
- structural_weight: 0.6
- size_weight: 1.5
- smell_weight: 0.3

**Use when**: Maintaining legacy scoring behavior or when file size is the primary concern.

**Source**: `src/priority/scoring/rebalanced.rs:96-105`

### Test-Coverage-Focused

Emphasizes testing gaps above all other factors.

```toml
[scoring_rebalanced]
preset = "test-coverage"
```

**Weights**:
- complexity_weight: 0.8
- coverage_weight: 1.3
- structural_weight: 0.6
- size_weight: 0.2
- smell_weight: 0.5

**Use when**: Prioritizing test coverage improvements.

**Source**: `src/priority/scoring/rebalanced.rs:107-116`

### Preset Comparison Table

| Preset | Complexity | Coverage | Structural | Size | Smells | Best For |
|--------|------------|----------|------------|------|--------|----------|
| balanced | 1.0 | 1.0 | 0.8 | 0.3 | 0.6 | Standard development |
| quality-focused | 1.2 | 1.1 | 0.9 | 0.2 | 0.7 | Quality-first teams |
| size-focused | 0.5 | 0.4 | 0.6 | 1.5 | 0.3 | Legacy compatibility |
| test-coverage | 0.8 | 1.3 | 0.6 | 0.2 | 0.5 | Coverage campaigns |

### Preset Name Aliases

Presets support multiple naming conventions for convenience:

**Source**: `src/priority/scoring/rebalanced.rs:118-127`

| Preset | Aliases |
|--------|---------|
| balanced | `balanced` |
| quality-focused | `quality-focused`, `quality_focused`, `quality` |
| size-focused | `size-focused`, `size_focused`, `legacy` |
| test-coverage | `test-coverage`, `test_coverage`, `testing` |

## Generated Code Detection

The rebalanced scoring automatically detects and reduces scores for generated code, applying a 90% reduction to the size score.

**Source**: `src/priority/scoring/rebalanced.rs:450-467` (is_generated_file), `src/priority/scoring/rebalanced.rs:246-249` (reduction logic)

### Detection Patterns

Generated files are identified by common naming patterns:

```rust
let generated_patterns = [
    ".generated.rs",
    ".pb.rs",     // Protocol buffers
    ".g.rs",      // Grammar generated files
    "_pb.rs",     // Alternative protobuf naming
    "generated/", // Generated directory
    "/gen/",      // Gen directory
];
```

**Source**: `src/priority/scoring/rebalanced.rs:455-462`

### Score Reduction

When a file matches a generated pattern, the size score is reduced by 90%:

```rust
// Apply generated code detection and scoring reduction
if is_generated_file(&func.file) {
    // Reduce size score by 90% for generated code
    components.size_score *= 0.1;
}
```

**Source**: `src/priority/scoring/rebalanced.rs:246-249`

### Examples

| File Path | Pattern Match | Size Score Adjustment |
|-----------|---------------|----------------------|
| `src/proto/api.pb.rs` | `.pb.rs` | 90% reduction |
| `src/generated/schema.rs` | `generated/` | 90% reduction |
| `src/parser.g.rs` | `.g.rs` | 90% reduction |
| `src/main.rs` | None | No adjustment |

## Configuration

Rebalanced scoring is configured in your `.debtmap.toml` configuration file.

**Source**: `src/config/scoring.rs:495-554` (RebalancedScoringConfig)

### Enabling Rebalanced Scoring

Add the `[scoring_rebalanced]` section to activate rebalanced scoring:

```toml
# .debtmap.toml

[scoring_rebalanced]
preset = "balanced"  # Activates rebalanced scoring with balanced preset
```

### Using Presets

Select a preset to use predefined weight configurations:

```toml
[scoring_rebalanced]
preset = "quality-focused"
```

### Custom Weight Overrides

Override individual weights while using a preset as base:

```toml
[scoring_rebalanced]
preset = "balanced"
complexity_weight = 1.2      # Override complexity weight
coverage_weight = 1.0        # Keep default
# Other weights inherit from preset
```

### Full Custom Configuration

Define all weights manually without a preset:

```toml
[scoring_rebalanced]
# Custom weights (no preset)
complexity_weight = 1.0
coverage_weight = 1.0
structural_weight = 0.8
size_weight = 0.3
smell_weight = 0.6
```

### RebalancedScoringConfig Structure

```rust
pub struct RebalancedScoringConfig {
    /// Preset name (balanced, quality-focused, size-focused, test-coverage)
    pub preset: Option<String>,

    /// Custom complexity weight (overrides preset if specified)
    pub complexity_weight: Option<f64>,

    /// Custom coverage weight (overrides preset if specified)
    pub coverage_weight: Option<f64>,

    /// Custom structural weight (overrides preset if specified)
    pub structural_weight: Option<f64>,

    /// Custom size weight (overrides preset if specified)
    pub size_weight: Option<f64>,

    /// Custom smell weight (overrides preset if specified)
    pub smell_weight: Option<f64>,
}
```

**Source**: `src/config/scoring.rs:496-521`

## Scoring Rationale

Each debt item includes a detailed rationale explaining why a score was assigned. The rationale includes primary factors, bonuses, and context adjustments.

**Source**: `src/priority/scoring/rebalanced.rs:130-223` (ScoringRationale)

### ScoringRationale Struct

```rust
pub struct ScoringRationale {
    pub primary_factors: Vec<String>,
    pub bonuses: Vec<String>,
    pub context_adjustments: Vec<String>,
}
```

**Source**: `src/priority/scoring/rebalanced.rs:131-136`

### Primary Factors

Primary factors are the main contributors to the score:

- **High cyclomatic complexity** (complexity_score > 40): `"High cyclomatic complexity (+{score})"`
- **Significant coverage gap** (coverage_score > 30): `"Significant coverage gap (+{score})"`
- **Structural issues** (structural_score > 30): `"Structural issues (+{score})"`

### Bonuses

Additive enhancements to the score:

- **Complex + untested** (complexity > 40 AND coverage > 20): `"Complex + untested: +20 bonus applied"`
- **Code smells** (smell_score > 20): `"Code smells detected (+{score})"`

### Context Adjustments

Explanations for score adjustments based on context:

- **Size context-adjusted** (0 < size_score < 10): `"File size context-adjusted (reduced weight for file type)"`
- **Size de-emphasized** (size_weight < 0.5): `"Size de-emphasized (weight: {weight})"`

### Example Rationale Output

```
Debt Item: src/payment/processor.rs:142 - process_payment()
Score: 95.3 (CRITICAL)

Primary factors:
  - High cyclomatic complexity (+100.0)
  - Significant coverage gap (+57.2)

Bonuses:
  - Complex + untested: +20 bonus applied
  - Code smells detected (+25.0)

Context adjustments:
  - Size de-emphasized (weight: 0.3)
```

## Complexity Scoring Details

The complexity score is computed from both cyclomatic and cognitive complexity.

**Source**: `src/priority/scoring/rebalanced.rs:265-317` (score_complexity)

### Cyclomatic Complexity Thresholds

| Cyclomatic Complexity | Base Score |
|----------------------|------------|
| > 30 | 100.0 |
| > 20 | 80.0 |
| > 15 | 60.0 |
| > 10 | 40.0 |
| > 5 | 20.0 |
| <= 5 | 0.0 |

### Cognitive Complexity Bonus

An additive bonus is applied based on cognitive complexity:

| Cognitive Complexity | Bonus |
|---------------------|-------|
| > 50 | +20.0 |
| > 30 | +15.0 |
| > 20 | +10.0 |
| > 15 | +5.0 |
| <= 15 | 0.0 |

The final complexity score is capped at 100.0:

```rust
(cyclomatic_score + cognitive_bonus).min(100.0)
```

### Example Calculations

| Function | Cyclomatic | Cognitive | Base Score | Bonus | Final |
|----------|------------|-----------|------------|-------|-------|
| Simple getter | 2 | 3 | 0.0 | 0.0 | 0.0 |
| Moderate logic | 12 | 18 | 40.0 | 5.0 | 45.0 |
| Complex parser | 25 | 45 | 80.0 | 15.0 | 95.0 |
| Very complex | 35 | 60 | 100.0 | 20.0 | 100.0 (capped) |

## Score Normalization

Weighted totals are normalized to a 0-200 range for consistent comparison across projects.

**Source**: `src/priority/scoring/rebalanced.rs:44-54`

### Normalization Formula

```
normalized_score = (raw_total / 237.0) * 200.0
```

### Theoretical Maximum

The theoretical maximum raw score is 237:
- Complexity: 100 × 1.0 = 100
- Coverage: 80 × 1.0 = 80
- Structural: 60 × 0.8 = 48
- Size: 30 × 0.3 = 9
- Smells: 40 × 0.6 = 24
- **Total**: 237

This maps to a normalized score of 200.

## CLI Usage

### Enable Rebalanced Scoring via Config

Create or modify `.debtmap.toml`:

```toml
[scoring_rebalanced]
preset = "balanced"
```

### Compare Standard vs Rebalanced Scoring

```bash
# Create test config with rebalanced scoring
cat > .debtmap-rebalanced.toml <<EOF
[scoring_rebalanced]
preset = "balanced"
EOF

# Compare results
debtmap analyze . --format terminal                            # Standard scoring
debtmap analyze . --config .debtmap-rebalanced.toml --format terminal  # Rebalanced scoring
```

### Test Different Presets

```bash
# Quality-focused preset
cat > .debtmap-quality.toml <<EOF
[scoring_rebalanced]
preset = "quality-focused"
EOF

debtmap analyze . --config .debtmap-quality.toml --top 10
```

## Best Practices

1. **Start with balanced preset** for standard development workflows
2. **Use quality-focused** when code quality is the primary concern
3. **Use test-coverage** during coverage improvement campaigns
4. **Use size-focused (legacy)** only for backward compatibility
5. **Review rationale output** to understand why items are prioritized
6. **Combine with file-level analysis** for comprehensive debt assessment
7. **Track severity distributions** over time to measure improvement

## Migration from Legacy Scoring

### Breaking Changes

- Scores will change significantly for all debt items
- Large files with low complexity will rank lower
- Complex untested code will rank higher
- Size-based prioritization reduced by 80%

### Restoring Legacy Behavior

```toml
[scoring_rebalanced]
preset = "size-focused"
```

### Gradual Migration Steps

1. **Test first**: Add `[scoring_rebalanced]` section to a test config file
2. **Compare**: Run analysis with both standard and rebalanced scoring
3. **Evaluate**: Review how priorities change
4. **Adopt**: Switch your primary config after validation
5. **Tune**: Adjust preset or custom weights based on team priorities

## See Also

- [File-Level Scoring](file-level.md) - For architectural issue identification
- [Scoring Configuration](../configuration/scoring.md) - Full scoring configuration options
- [Analysis Guide](../analysis-guide/index.md) - Interpreting analysis results
