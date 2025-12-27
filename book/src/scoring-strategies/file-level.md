# File-Level Scoring

File-level scoring aggregates metrics across all functions in a file to identify architectural problems and module-level refactoring opportunities.

## Overview

While function-level scoring helps identify specific hot spots, file-level scoring reveals broader architectural issues:
- God objects with too many responsibilities
- Files with poor cohesion
- Modules that should be split
- Files with excessive function counts

File-level analysis is essential for planning major refactoring initiatives and understanding module-level technical debt.

## Formula

```
File Score = Size × Complexity × Coverage Factor × Density × GodObject × FunctionScores
```

**Note**: This is a conceptual formula showing the multiplicative relationship between factors. The actual implementation in `src/priority/file_metrics.rs:262-306` includes additional normalization steps and conditional adjustments.

Where each factor is calculated as:
- **Size** = `sqrt(total_lines / 100)`
- **Complexity** = `(avg_complexity / 5.0) × sqrt(total_complexity / 50.0)`
- **Coverage Factor** = `((1.0 - coverage_percent) × 2.0) + 1.0`
- **Density** = `1.0 + ((function_count - 50) × 0.02)` if function_count > 50, else 1.0
- **GodObject** = `1.0 + (god_object_score / 50.0)` if detected (maps score 0-100 to multiplier 1.0x-3.0x)
- **FunctionScores** = `max(sum(function_scores) / 10, 1.0)`

## Scoring Factors

### Size Factor

**Formula**: `sqrt(total_lines / 100)`

**Source**: `src/priority/file_metrics.rs:264`

The size factor reflects the impact scope of refactoring:
- A 100-line file has factor 1.0
- A 400-line file has factor 2.0
- A 1000-line file has factor 3.16

The square root dampens the effect to avoid over-penalizing large files while still accounting for their broader impact.

**Rationale**: Refactoring a 1000-line file affects more code than a 100-line file, but the impact doesn't scale linearly.

### Complexity Factor

**Formula**: `(avg_complexity / 5.0).min(3.0) × sqrt(total_complexity / 50.0)`

**Source**: `src/priority/file_metrics.rs:267-269`

Combines average and total complexity to balance per-function and aggregate complexity:
- **Average factor**: Capped at 3.0 to prevent extreme values
- **Total factor**: Square root dampening of aggregate complexity

**Rationale**: Both concentrated complexity (one very complex function) and spread-out complexity (many moderately complex functions) matter for maintainability.

### Coverage Factor

**Formula**: `((1.0 - coverage_percent) × 2.0) + 1.0`

**Source**: `src/priority/file_metrics.rs:272-273`

The coverage factor acts as a multiplicative amplifier:
- **100% coverage**: Factor = 1.0 (no amplification)
- **50% coverage**: Factor = 2.0 (doubles the score)
- **0% coverage**: Factor = 3.0 (triples the score)

**Example**: A file with 50% coverage and base score 10 becomes 20 after coverage adjustment.

**Rationale**: Untested files amplify existing complexity and risk. The multiplicative approach reflects that testing gaps compound other issues.

### Density Factor

**Formula**: `1.0 + ((function_count - 50) × 0.02)` when function_count > 50, else 1.0

**Source**: `src/priority/file_metrics.rs:276-280`

Penalizes files with excessive function counts:
- **50 or fewer functions**: Factor = 1.0 (no penalty)
- **75 functions**: Factor = 1.5 (50% increase)
- **100 functions**: Factor = 2.0 (doubles the score)

**Rationale**: Files with many functions likely violate single responsibility principle and should be split.

### God Object Multiplier

**Formula**: `1.0 + (god_object_score / 50.0)` when detected, else 1.0

**Source**: `src/priority/file_metrics.rs:282-293`

The god object multiplier applies proportional scaling based on severity:
- **Score 0 (borderline)**: Multiplier = 1.0x
- **Score 50 (moderate)**: Multiplier = 2.0x
- **Score 100 (severe)**: Multiplier = 3.0x

**Implementation Comment** (from `src/priority/file_metrics.rs:282-284`):
> Score 0-100 maps to multiplier 1.0x-3.0x (not 2x-102x!)
> This aligns with contextual risk cap (max 3x) for consistent scoring

**Rationale**: God objects need architectural attention. The proportional scaling ensures severe god objects rank higher while keeping multipliers within a reasonable range that aligns with other contextual risk factors.

### Function Scores Factor

**Formula**: `max(sum(function_scores) / 10.0, 1.0)`

**Source**: `src/priority/file_metrics.rs:296-297`

Aggregates individual function debt scores with normalization:
- Sum of all function scores divided by 10
- Minimum value of 1.0 to prevent near-zero scores

**Rationale**: The aggregate function debt provides a baseline before other multiplicative modifiers are applied.

## FileScoreFactors Struct

For transparency in score calculations, Debtmap provides the `FileScoreFactors` struct that breaks down how each factor contributes to the final score.

**Source**: `src/priority/file_metrics.rs:217-259`

```rust
pub struct FileScoreFactors {
    pub size_factor: f64,
    pub size_basis: usize,
    pub complexity_factor: f64,
    pub avg_complexity: f64,
    pub total_complexity: u32,
    pub coverage_factor: f64,
    pub coverage_percent: f64,
    pub coverage_gap: f64,
    pub density_factor: f64,
    pub function_count: usize,
    pub god_object_multiplier: f64,
    pub god_object_score: f64,
    pub is_god_object: bool,
    pub function_factor: f64,
    pub function_score_sum: f64,
}
```

### Using get_score_factors()

You can access the score breakdown via the `get_score_factors()` method:

```rust
let factors = metrics.get_score_factors();
println!("Coverage factor: {:.2} ({:.0}% coverage)",
         factors.coverage_factor,
         factors.coverage_percent * 100.0);
```

**Source**: `src/priority/file_metrics.rs:333-394`

## File Context Adjustments

File-level scoring integrates with context-aware adjustments that reduce scores for test files, generated files, and other non-production code.

**Source**: `src/priority/scoring/file_context_scoring.rs`

### Adjustment Rules

| File Context | Confidence | Score Multiplier | Effect |
|--------------|------------|------------------|--------|
| Test file | > 0.8 | 0.2 | 80% reduction |
| Probable test | 0.5-0.8 | 0.6 | 40% reduction |
| Generated file | - | 0.1 | 90% reduction |
| Production | - | 1.0 | No adjustment |

### Example

```rust
use debtmap::priority::{FileDebtItem, FileDebtMetrics};
use debtmap::analysis::FileContext;

let metrics = FileDebtMetrics::default();
let test_context = FileContext::Test {
    confidence: 0.95,
    test_framework: Some("rust-std".to_string()),
    test_count: 10,
};

let item = FileDebtItem::from_metrics(metrics, Some(&test_context));
// item.score is now reduced by 80% due to test file context
```

**Source**: `src/priority/file_metrics.rs:675-706`

## Aggregation Methods

Debtmap supports multiple aggregation methods for combining function scores into file-level scores, configurable via CLI or configuration file.

### Weighted Sum (Default)

**Formula**: `Σ(function_score × complexity_weight × coverage_weight)`

```bash
debtmap analyze . --aggregation-method weighted_sum
```

Or via configuration:
```toml
[aggregation]
method = "weighted_sum"
```

**Characteristics**:
- Weights functions by their complexity and coverage gaps
- Emphasizes high-impact functions over trivial ones
- Best for most use cases where you want to focus on significant issues

### Simple Sum

**Formula**: `Σ(function_scores)`

```toml
[aggregation]
method = "sum"
```

**Characteristics**:
- Adds all function scores directly without weighting
- Treats all functions equally regardless of complexity
- Useful for broad overview and trend analysis

### Logarithmic Sum

**Formula**: `log(1 + Σ(function_scores))`

```toml
[aggregation]
method = "logarithmic_sum"
```

**Characteristics**:
- Dampens impact of many small issues to prevent score explosion
- Prevents files with hundreds of minor issues from dominating
- Creates more balanced comparisons across files of different sizes

**Best for**: Legacy codebases with many small issues where you want to avoid extreme scores.

### Max Plus Average

**Formula**: `max_score × 0.6 + avg_score × 0.4`

```toml
[aggregation]
method = "max_plus_average"
```

**Characteristics**:
- Considers worst function (60%) plus average of all functions (40%)
- Balances worst-case and typical-case scenarios
- Highlights files with both a critical hot spot and general issues

### Choosing an Aggregation Method

| Codebase Type | Recommended Method | Rationale |
|---------------|-------------------|-----------|
| New/Modern | `weighted_sum` | Proportional emphasis on real issues |
| Legacy with many small issues | `logarithmic_sum` | Prevents score explosion |
| Mixed quality | `max_plus_average` | Balances hot spots with overall quality |
| Trend analysis | `sum` | Simple, consistent metric over time |

**Performance Note**: All aggregation methods have O(n) complexity where n = number of functions. Performance differences are negligible for typical codebases (<100k functions).

## CLI Usage

### Show File-Level Results Only

```bash
# Show top 10 files needing architectural refactoring
debtmap analyze . --aggregate-only --top 10
```

**Note**: The `--aggregate-only` flag changes output to show only file-level metrics instead of function-level details.

### Focus on Architectural Problems

```bash
debtmap analyze . --aggregate-only --filter Architecture
```

### Find Files with Excessive Function Counts

```bash
debtmap analyze . --aggregate-only --min-problematic 50
```

### Generate File-Level Report

```bash
debtmap analyze . --aggregate-only --format markdown -o report.md
```

## Configuration

> **IMPORTANT**: The configuration file must be named **`.debtmap.toml`** (not `debtmap.yml` or other variants) and placed in your project root directory.

```toml
[aggregation]
method = "weighted_sum"
min_problematic = 3              # Need 3+ problematic functions for file-level score

[god_object_detection]
enabled = true
max_methods = 20
max_fields = 15
max_responsibilities = 5
```

## Use Cases

### Planning Major Refactoring Initiatives

When planning sprint or quarterly refactoring work, file-level scoring helps identify which modules need the most attention:

```bash
debtmap analyze . --aggregate-only --top 10
```

Use when:
- Planning sprint or quarterly refactoring work
- Deciding which modules to split
- Prioritizing architectural improvements
- Allocating team resources

### Identifying God Objects

File-level scoring excels at finding files with architectural issues:

```bash
debtmap analyze . --aggregate-only --filter Architecture
```

Look for files with:
- High god object scores
- Many functions (50+)
- Low coverage combined with high complexity

### Breaking Up Monolithic Modules

```bash
# Find files with excessive function counts
debtmap analyze . --aggregate-only --min-problematic 50
```

### Evaluating Overall Codebase Health

```bash
# Generate file-level report for executive summary
debtmap analyze . --aggregate-only --format markdown -o report.md
```

## Best Practices

1. **Start with file-level analysis** for strategic planning before drilling into function-level details
2. **Use logarithmic aggregation** for legacy codebases to prevent score explosion
3. **Track file-level trends** over time to measure architectural improvement
4. **Combine with god object detection** to identify structural issues beyond simple size metrics
5. **Consider context adjustments** - test and generated files should not dominate your priority list

## See Also

- [Function-Level Scoring](function-level.md) - For targeted hot spot identification
- [Rebalanced Scoring](rebalanced.md) - Advanced scoring algorithm that de-emphasizes size
- [God Object Detection](../god-object-detection.md) - Detailed god object analysis
- [Configuration](../configuration/scoring.md) - Full scoring configuration options
