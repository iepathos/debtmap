# Data Flow Scoring

Data flow scoring enhances Debtmap's technical debt analysis by evaluating function purity, refactorability, and code patterns. This subsection explains how data flow analysis affects debt prioritization through three key factors: purity, refactorability, and pattern recognition.

## Overview

Data flow scoring is an optional scoring layer that adjusts debt priorities based on functional programming principles. Functions that are pure, easily refactorable, or follow recognized patterns receive reduced priority scores, reflecting their lower maintenance burden.

**Key principle**: Pure functions and data transformation pipelines represent less technical debt than impure functions with side effects, because they're easier to test, reason about, and refactor.

**Source**: `src/priority/unified_scorer.rs:995-1020` (`calculate_unified_priority_with_data_flow`)

## How Data Flow Scoring Works

Data flow scoring applies three weighted factors to the base debt score:

```
adjusted_score = base_score * combined_adjustment
combined_adjustment = (purity_factor * purity_weight
                     + refactorability_factor * refactorability_weight
                     + pattern_factor * pattern_weight)
                    / total_weight
```

Each factor ranges from 0.0 to 1.0, where lower values reduce the final priority score.

**Source**: `src/priority/unified_scorer.rs:1058-1075`

## Purity Spectrum

The purity spectrum classifies functions into five levels based on their side effects and mutation behavior. Pure functions receive the lowest priority multipliers since they represent minimal technical debt.

### Classification Levels

| Level | Multiplier | Description |
|-------|------------|-------------|
| `StrictlyPure` | 0.0 | No mutations, no I/O, referentially transparent |
| `LocallyPure` | 0.3 | Pure interface but uses local mutations internally |
| `IOIsolated` | 0.6 | I/O operations clearly separated from logic |
| `IOMixed` | 0.9 | I/O mixed with business logic |
| `Impure` | 1.0 | Mutable state, side effects throughout |

**Source**: `src/priority/unified_scorer.rs:64-94` (`PuritySpectrum` enum)

### Classification Algorithm

The purity factor is calculated by analyzing three sources of information from the data flow graph:

1. **Purity Analysis Results**: High-confidence purity (>80%) indicates strict or local purity
2. **Mutation Analysis**: Tracks whether a function has local mutations
3. **I/O Operations**: Identifies I/O patterns for non-pure functions

```rust
// Classification logic (simplified)
if purity.is_pure && purity.confidence > 0.8 {
    if mutations.has_mutations {
        PuritySpectrum::LocallyPure  // 0.3 multiplier
    } else {
        PuritySpectrum::StrictlyPure // 0.0 multiplier
    }
} else if purity.is_pure {
    PuritySpectrum::LocallyPure      // 0.3 multiplier
} else {
    classify_io_isolation(io_ops)    // 0.6-1.0 multiplier
}
```

**Source**: `src/priority/unified_scorer.rs:878-918` (`calculate_purity_factor`)

### I/O Isolation Classification

For impure functions, the system evaluates I/O isolation based on concentration:

- **IOIsolated (0.6)**: At most 2 unique I/O operation types and 3 total operations
- **IOMixed (0.9)**: More than 2 unique types or more than 3 operations
- **Impure (1.0)**: No I/O information available

**Source**: `src/priority/unified_scorer.rs:921-935` (`classify_io_isolation`)

## Purity Level vs Purity Spectrum

Debtmap uses two related but distinct purity classifications:

| Aspect | PurityLevel | PuritySpectrum |
|--------|-------------|----------------|
| **Purpose** | Analysis classification | Scoring multiplier |
| **Levels** | 4 (StrictlyPure, LocallyPure, ReadOnly, Impure) | 5 (adds IOIsolated, IOMixed) |
| **Usage** | `src/analysis/purity_analysis.rs` | `src/priority/unified_scorer.rs` |
| **Focus** | Categorizing purity type | Assigning debt priority |

**PurityLevel** (from purity analysis) describes *what kind* of function this is. **PuritySpectrum** (for scoring) determines *how much* this affects debt priority, with finer granularity for I/O patterns.

**Source**: `src/analysis/purity_analysis.rs:32-43` (`PurityLevel`), `src/priority/unified_scorer.rs:64-94` (`PuritySpectrum`)

## Pattern Factor

The pattern factor distinguishes data flow pipelines from business logic. Pure data transformation chains (map/filter/reduce patterns) receive reduced priority.

### Calculation

```rust
// Pattern factor ranges from 0.7 to 1.0
let transform_ratio = transform_count / dependency_count;

if transform_ratio > 0.5 {
    0.7   // Data flow pipeline - lowest priority
} else if transform_ratio > 0.3 {
    0.85  // Mixed - moderate reduction
} else {
    1.0   // Business logic - no reduction
}
```

**Rationale**: Functions with high transformation-to-dependency ratios are likely data flow pipelines, which are easier to test and maintain than complex business logic.

**Source**: `src/priority/unified_scorer.rs:949-978` (`calculate_pattern_factor`)

### Data Transformation Detection

The system counts data transformations by examining the data flow graph for:
- Outgoing function calls with associated data transformations
- Variable dependencies passed between functions
- Transformation types (map, filter, reduce, etc.)

**Source**: `src/priority/unified_scorer.rs:980-993` (`count_data_transformations`)

## Refactorability Factor

The refactorability factor was designed to identify dead stores and unused mutations. However, this analysis produced too many false positives and has been simplified.

**Current behavior**: Returns a neutral factor of 1.0 (no adjustment).

```rust
fn calculate_refactorability_factor(
    _func_id: &FunctionId,
    _data_flow: &DataFlowGraph,
    _config: &DataFlowScoringConfig,
) -> f64 {
    // Dead store analysis has been removed as it produced
    // too many false positives.
    1.0
}
```

**Future plans**: More sophisticated dead store analysis may be reintroduced with improved heuristics.

**Source**: `src/priority/unified_scorer.rs:937-947`

## Data Flow Graph

The `DataFlowGraph` struct provides the underlying data for all data flow scoring calculations:

```rust
pub struct DataFlowGraph {
    call_graph: CallGraph,
    variable_deps: HashMap<FunctionId, HashSet<String>>,
    data_transformations: HashMap<(FunctionId, FunctionId), DataTransformation>,
    io_operations: HashMap<FunctionId, Vec<IoOperation>>,
    purity_analysis: HashMap<FunctionId, PurityInfo>,
    mutation_analysis: HashMap<FunctionId, MutationInfo>,
    // ... (CFG analysis fields omitted for brevity)
}
```

**Key data used for scoring**:
- `purity_analysis`: Results from purity detection
- `mutation_analysis`: Tracks live vs dead mutations
- `io_operations`: I/O operation locations and types
- `variable_deps`: Variable dependencies for pattern detection
- `data_transformations`: Transformation relationships between functions

**Source**: `src/data_flow/mod.rs:113-140`

## Configuration

Configure data flow scoring in your `.debtmap.toml`:

```toml
[data_flow_scoring]
enabled = true              # Enable/disable data flow scoring (default: true)
purity_weight = 0.4         # Weight for purity factor (default: 0.4)
refactorability_weight = 0.3 # Weight for refactorability factor (default: 0.3)
pattern_weight = 0.3        # Weight for pattern factor (default: 0.3)
```

**Weight guidelines**:
- All weights should be between 0.0 and 1.0
- Weights are normalized internally (don't need to sum to 1.0)
- Higher `purity_weight` emphasizes functional programming style
- Higher `pattern_weight` rewards data transformation pipelines

**Source**: `src/config/scoring.rs:678-723` (`DataFlowScoringConfig`)

### Disabling Data Flow Scoring

To disable data flow scoring entirely:

```toml
[data_flow_scoring]
enabled = false
```

When disabled, `calculate_unified_priority_with_data_flow` returns the base score without any data flow adjustments.

## Practical Examples

### Example 1: Strictly Pure Function

```rust
fn calculate_total(prices: &[f64]) -> f64 {
    prices.iter().sum()
}
```

**Analysis**:
- No mutations: `has_mutations = false`
- No I/O operations: `io_ops = []`
- High purity confidence: `confidence > 0.8`

**Result**: `PuritySpectrum::StrictlyPure` (multiplier: 0.0)

This function's debt score is reduced by the purity factor, deprioritizing it for refactoring.

### Example 2: I/O Isolated Function

```rust
fn save_report(report: &Report) -> std::io::Result<()> {
    let json = serde_json::to_string(report)?;
    std::fs::write("report.json", json)?;
    Ok(())
}
```

**Analysis**:
- I/O operations: `[file_write]` (1 unique type, 1 operation)
- Concentrated I/O: `unique_types.len() <= 2 && ops.len() <= 3`

**Result**: `PuritySpectrum::IOIsolated` (multiplier: 0.6)

### Example 3: Data Flow Pipeline

```rust
fn process_transactions(transactions: Vec<Transaction>) -> Vec<Summary> {
    transactions
        .into_iter()
        .filter(|t| t.amount > 0.0)
        .map(|t| Summary::from(t))
        .collect()
}
```

**Analysis**:
- High transformation ratio (filter + map chains)
- `transform_ratio > 0.5`

**Result**: Pattern factor = 0.7, reducing debt priority for this data pipeline.

## Integration with Unified Scoring

Data flow scoring integrates with the broader [unified scoring system](rebalanced.md). The entry point is:

```rust
pub fn calculate_unified_priority_with_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    data_flow: &DataFlowGraph,
    coverage: Option<&LcovData>,
    _organization_issues: Option<f64>,
    debt_aggregator: Option<&DebtAggregator>,
    config: &DataFlowScoringConfig,
) -> UnifiedScore
```

The function:
1. Calculates base score using role-aware unified scoring
2. If data flow scoring is enabled, calculates the three factors
3. Applies weighted combination to adjust final score
4. Records factors in `UnifiedScore` for debugging

**Source**: `src/priority/unified_scorer.rs:995-1080`

## Related Documentation

- [Rebalanced Scoring](rebalanced.md) - How data flow factors combine with other scoring weights
- [Function-Level Scoring](function-level.md) - Base scoring for individual functions
- [File-Level Scoring](file-level.md) - Aggregated scoring at file level
- [Scoring Configuration](../configuration/scoring.md#data-flow-scoring) - Configuration reference
