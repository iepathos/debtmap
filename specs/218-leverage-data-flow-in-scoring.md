---
number: 218
title: Leverage Data Flow Analysis in Scoring
category: optimization
priority: medium
status: draft
dependencies: [216, 217]
created: 2025-01-09
---

# Specification 218: Leverage Data Flow Analysis in Scoring

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 216, Spec 217

## Context

Currently, the unified scoring system uses high-level signals:
- Cyclomatic/cognitive complexity
- Test coverage
- Call graph metrics (callers, callees, blast radius)
- Purity flag (is_pure: bool)
- Function role classification

However, after Specs 216 and 217, we'll have **fine-grained data flow information** that isn't being leveraged for scoring:

- **Live vs dead mutations**: Functions with many dead stores are easier to refactor
- **Escape analysis**: Non-escaping mutations are less risky to change
- **I/O concentration**: Functions with clustered I/O are good isolation candidates
- **Taint analysis**: Functions where mutations don't taint return values are "almost pure"
- **Transformation patterns**: Iterator chains vs imperative logic affects refactoring difficulty

**Opportunity**: Use data flow insights to provide more nuanced, accurate priority scoring that reflects actual refactoring difficulty and impact.

## Objective

Incorporate data flow analysis results into the unified scoring system to:

1. **Reward simplicity**: Boost scores for functions with many dead stores (easy wins)
2. **Assess purity spectrum**: Distinguish "strictly pure" vs "locally pure" vs "impure with isolated I/O"
3. **Refine risk assessment**: Use escape analysis to reduce scores for low-impact mutations
4. **Detect patterns**: Identify data transformation pipelines vs business logic
5. **Guide recommendations**: Use data flow patterns to suggest specific refactoring strategies

## Requirements

### Functional Requirements

1. **Dead Store Factor**
   - Calculate ratio: `dead_stores / total_mutations`
   - High ratio → easier to refactor → higher priority
   - Add to complexity factor or create new "refactorability" factor

2. **Purity Spectrum Scoring**
   - Strictly Pure: 0.0 multiplier (no debt)
   - Locally Pure: 0.3 multiplier (minor debt)
   - I/O Isolated: 0.6 multiplier (moderate debt, good candidate)
   - I/O Mixed: 0.9 multiplier (high debt)
   - Impure: 1.0 multiplier (full debt score)

3. **Escape Analysis Factor**
   - Calculate `escaping_mutations / total_mutations`
   - Non-escaping mutations → lower risk → reduced priority
   - Apply as dampening factor to mutation-based scores

4. **Transformation Pattern Detection**
   - Identify iterator chains, builders, serialization
   - Data flow functions → lower priority (plumbing, not logic)
   - Business logic functions → maintain priority

5. **I/O Clustering Score**
   - Concentrated I/O (few functions) → higher priority for extraction
   - Scattered I/O → lower priority (pervasive design issue)

### Non-Functional Requirements

- **Performance**: Scoring with data flow must add < 5% to analysis time
- **Transparency**: Score adjustments must be explainable and visible
- **Stability**: Small changes in code shouldn't cause dramatic score swings
- **Tuning**: Data flow factors should be configurable

## Acceptance Criteria

- [ ] Dead store factor incorporated into scoring (functions with 50%+ dead stores get +20% score)
- [ ] Purity spectrum replaces binary is_pure flag in scoring
- [ ] Escape analysis reduces priority for non-escaping mutations
- [ ] Data transformation patterns detected and scored appropriately
- [ ] I/O clustering affects extraction recommendations
- [ ] Score explanations include data flow factors
- [ ] Configuration allows tuning data flow weights
- [ ] Scoring performance overhead < 5%
- [ ] Integration tests validate score changes with/without data flow
- [ ] Documentation explains data flow scoring methodology

## Technical Details

### Implementation Approach

**Phase 1: Extend UnifiedScore**

Add data flow factors to `UnifiedScore`:

```rust
pub struct UnifiedScore {
    pub final_score: f64,
    pub complexity_factor: f64,
    pub coverage_factor: f64,
    pub dependency_factor: f64,

    // NEW: Data flow factors
    pub purity_factor: Option<f64>,        // Purity spectrum score
    pub refactorability_factor: Option<f64>, // Dead stores, escape analysis
    pub pattern_factor: Option<f64>,       // Data flow vs business logic
}
```

**Phase 2: Calculate Data Flow Factors**

Add to `src/priority/unified_scorer.rs`:

```rust
fn calculate_purity_factor(
    func_id: &FunctionId,
    data_flow: &DataFlowGraph,
) -> Option<f64> {
    let cfg_analysis = data_flow.cfg_analysis.get(func_id)?;
    let mutation_info = data_flow.mutation_analysis.get(func_id)?;
    let io_ops = data_flow.get_io_operations(func_id);

    // Classify on purity spectrum
    let purity_level = if mutation_info.live_mutations.is_empty()
        && io_ops.is_none() {
        PuritySpectrum::StrictlyPure
    } else if !mutation_info.escaping_mutations.is_empty()
        || io_ops.is_some() {
        PuritySpectrum::Impure
    } else if mutation_info.live_mutations.len() <= 2 {
        PuritySpectrum::LocallyPure
    } else {
        PuritySpectrum::MixedPure
    };

    Some(purity_level.score_multiplier())
}

fn calculate_refactorability_factor(
    func_id: &FunctionId,
    data_flow: &DataFlowGraph,
) -> Option<f64> {
    let mutation_info = data_flow.mutation_analysis.get(func_id)?;

    if mutation_info.total_mutations == 0 {
        return Some(1.0); // Pure, maximally refactorable
    }

    // High dead store ratio → easier to refactor
    let dead_store_ratio = mutation_info.dead_stores.len() as f64
        / mutation_info.total_mutations as f64;

    // Non-escaping mutations → lower risk
    let escape_ratio = mutation_info.escaping_mutations.len() as f64
        / mutation_info.live_mutations.len().max(1) as f64;

    // Combined refactorability: higher is easier
    let refactorability = (1.0 + dead_store_ratio) * (1.0 - escape_ratio * 0.5);

    Some(refactorability)
}

fn calculate_pattern_factor(
    func: &FunctionMetrics,
    data_flow: &DataFlowGraph,
) -> Option<f64> {
    // Use rust_data_flow_analyzer to classify
    let profile = analyze_data_flow(&func.syn_func?);

    if profile.transformation_ratio > 0.7 && profile.confidence > 0.7 {
        // Data flow function → reduce priority
        Some(0.7)
    } else if profile.business_logic_ratio > 0.7 {
        // Business logic → maintain priority
        Some(1.0)
    } else {
        // Mixed → neutral
        Some(0.85)
    }
}
```

**Phase 3: Integrate into Unified Scoring**

Update `calculate_unified_priority()`:

```rust
pub fn calculate_unified_priority(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    data_flow: Option<&DataFlowGraph>,
) -> UnifiedScore {
    let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);

    // Existing factors
    let complexity_factor = calculate_complexity_factor(func);
    let coverage_factor = calculate_coverage_factor(func, coverage);
    let dependency_factor = calculate_dependency_factor(&func_id, call_graph);

    // NEW: Data flow factors
    let purity_factor = data_flow
        .and_then(|df| calculate_purity_factor(&func_id, df));

    let refactorability_factor = data_flow
        .and_then(|df| calculate_refactorability_factor(&func_id, df));

    let pattern_factor = data_flow
        .and_then(|df| calculate_pattern_factor(func, df));

    // Combine factors
    let base_score = complexity_factor * coverage_factor * dependency_factor;

    let final_score = base_score
        * purity_factor.unwrap_or(1.0)
        * refactorability_factor.unwrap_or(1.0)
        * pattern_factor.unwrap_or(1.0);

    UnifiedScore {
        final_score,
        complexity_factor,
        coverage_factor,
        dependency_factor,
        purity_factor,
        refactorability_factor,
        pattern_factor,
        base_score: Some(base_score),
        pre_adjustment_score: None,
    }
}
```

**Phase 4: Configuration**

Add to `src/config/mod.rs`:

```rust
#[derive(Deserialize)]
pub struct DataFlowScoringConfig {
    /// Enable data flow scoring factors
    pub enabled: bool,

    /// Weight for purity spectrum (0.0 - 1.0)
    pub purity_weight: f64,

    /// Weight for refactorability factor (0.0 - 1.0)
    pub refactorability_weight: f64,

    /// Weight for pattern factor (0.0 - 1.0)
    pub pattern_weight: f64,

    /// Minimum dead store ratio to boost score (0.0 - 1.0)
    pub min_dead_store_ratio: f64,

    /// Boost multiplier for high dead store ratio
    pub dead_store_boost: f64,
}

impl Default for DataFlowScoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            purity_weight: 0.8,
            refactorability_weight: 1.2,
            pattern_weight: 0.9,
            min_dead_store_ratio: 0.5,
            dead_store_boost: 1.2,
        }
    }
}
```

### Architecture Changes

**Modified Files**:
- `src/priority/unified_scorer.rs` - Add data flow factor calculations
- `src/priority/mod.rs` - Extend UnifiedScore struct
- `src/config/mod.rs` - Add DataFlowScoringConfig
- `src/priority/scoring/construction.rs` - Pass data_flow to scoring

**New Types**:
```rust
pub enum PuritySpectrum {
    StrictlyPure,     // No mutations, no I/O
    LocallyPure,      // Only non-escaping mutations
    IOIsolated,       // I/O in dedicated section
    IOMixed,          // I/O scattered throughout
    Impure,           // Mutations escape or pervasive I/O
}

impl PuritySpectrum {
    pub fn score_multiplier(&self) -> f64 {
        match self {
            Self::StrictlyPure => 0.0,
            Self::LocallyPure => 0.3,
            Self::IOIsolated => 0.6,
            Self::IOMixed => 0.9,
            Self::Impure => 1.0,
        }
    }
}
```

### Scoring Formula Changes

**Current**:
```
final_score = complexity_factor × coverage_factor × dependency_factor
```

**New**:
```
base_score = complexity_factor × coverage_factor × dependency_factor

final_score = base_score
    × purity_factor        // 0.0-1.0 based on purity spectrum
    × refactorability      // 1.0-1.5 based on dead stores & escape
    × pattern_factor       // 0.7-1.0 based on data flow patterns
```

**Example Impact**:

Function with high complexity (80), low coverage (0.5), high dependencies (1.2):
- Base: 80 × 0.5 × 1.2 = 48.0
- With data flow:
  - Purity: LocallyPure (0.3) → 48 × 0.3 = 14.4
  - Refactorability: 60% dead stores → 14.4 × 1.2 = 17.3
  - Pattern: Data flow (0.7) → 17.3 × 0.7 = 12.1

**Result**: Score reduced from 48.0 → 12.1 because function is mostly pure with easy refactoring opportunities.

## Dependencies

**Prerequisites**:
- Spec 216: Complete Data Flow Graph Population
- Spec 217: Surface Data Flow Insights in Output

**Affected Components**:
- Unified scoring system
- Priority ranking
- Recommendation generation
- Score explanation/transparency

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_purity_factor_calculation() {
    let data_flow = create_data_flow_with_local_mutations();
    let factor = calculate_purity_factor(&func_id, &data_flow);

    assert_eq!(factor, Some(0.3)); // LocallyPure
}

#[test]
fn test_refactorability_boost_for_dead_stores() {
    let mutation_info = MutationInfo {
        total_mutations: 10,
        live_mutations: vec!["result".to_string()],
        dead_stores: ["temp1", "temp2", "temp3"].iter()
            .map(|s| s.to_string()).collect(),
        escaping_mutations: HashSet::new(),
    };

    let factor = calculate_refactorability_from_info(&mutation_info);

    assert!(factor > 1.0); // Should boost due to high dead store ratio
}

#[test]
fn test_pattern_factor_reduces_score_for_data_flow() {
    let func = create_iterator_chain_function();
    let data_flow = create_empty_data_flow();

    let factor = calculate_pattern_factor(&func, &data_flow);

    assert_eq!(factor, Some(0.7)); // Data flow pattern detected
}
```

### Integration Tests

```rust
#[test]
fn test_scoring_with_vs_without_data_flow() {
    let func = create_complex_function_with_dead_stores();
    let call_graph = create_test_call_graph();
    let data_flow = create_populated_data_flow();

    let score_without = calculate_unified_priority(&func, &call_graph, None, None);
    let score_with = calculate_unified_priority(&func, &call_graph, None, Some(&data_flow));

    // Score should be higher with data flow (easier to refactor)
    assert!(score_with.final_score > score_without.final_score * 1.1);

    // Refactorability factor should be present
    assert!(score_with.refactorability_factor.unwrap() > 1.0);
}

#[test]
fn test_pure_function_gets_low_score() {
    let func = create_pure_function();
    let data_flow = create_data_flow_for_pure();

    let score = calculate_unified_priority(&func, &call_graph, None, Some(&data_flow));

    // Pure function should have very low score (no debt)
    assert!(score.purity_factor.unwrap() < 0.1);
    assert!(score.final_score < 10.0);
}
```

### Performance Tests

```rust
#[test]
fn test_data_flow_scoring_performance() {
    let metrics = generate_large_metric_set(1000);
    let data_flow = create_populated_data_flow_for_metrics(&metrics);

    let start = Instant::now();
    for metric in &metrics {
        calculate_unified_priority(metric, &call_graph, None, Some(&data_flow));
    }
    let duration = start.elapsed();

    let per_function = duration.as_micros() / metrics.len() as u128;

    // Should add < 50µs per function
    assert!(per_function < 50, "Data flow scoring took {}µs per function", per_function);
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Calculate purity factor based on mutation and I/O analysis
///
/// # Purity Spectrum
///
/// - **Strictly Pure** (0.0): No mutations, no I/O
/// - **Locally Pure** (0.3): Only non-escaping local mutations
/// - **I/O Isolated** (0.6): I/O operations in dedicated section
/// - **I/O Mixed** (0.9): I/O scattered with business logic
/// - **Impure** (1.0): Mutations escape or pervasive side effects
///
/// # Example
///
/// ```
/// let factor = calculate_purity_factor(&func_id, &data_flow);
/// // For locally pure function with 2 non-escaping mutations:
/// assert_eq!(factor, Some(0.3));
/// ```
pub fn calculate_purity_factor(/*...*/) -> Option<f64>
```

### User Documentation

Update `book/src/scoring-guide.md`:

```markdown
## Data Flow Factors

Data flow analysis enhances scoring with three additional factors:

### Purity Factor (0.0 - 1.0)

Classifies functions on a purity spectrum:
- **0.0**: Strictly pure (no debt)
- **0.3**: Locally pure (minor refactoring opportunity)
- **0.6**: I/O isolated (good extraction candidate)
- **0.9**: I/O mixed with logic (higher priority)
- **1.0**: Impure (full complexity score applies)

### Refactorability Factor (1.0 - 1.5)

Boosts scores for easier refactoring:
- High dead store ratio → easier to clean up
- Non-escaping mutations → lower risk changes
- Functions with 50%+ dead stores get 20% score boost

### Pattern Factor (0.7 - 1.0)

Adjusts for code patterns:
- **0.7**: Data transformation pipeline (lower priority)
- **0.85**: Mixed pattern
- **1.0**: Business logic (maintain priority)

### Configuration

Tune data flow scoring in `debtmap.toml`:

```toml
[scoring.data_flow]
enabled = true
purity_weight = 0.8
refactorability_weight = 1.2
pattern_weight = 0.9
min_dead_store_ratio = 0.5
dead_store_boost = 1.2
```
```

### Architecture Updates

Update `ARCHITECTURE.md`:

```markdown
## Unified Scoring

### Factor Composition

final_score = base_score × data_flow_multiplier

Where:
- base_score = complexity × coverage × dependencies
- data_flow_multiplier = purity × refactorability × pattern

### Data Flow Factors

1. **Purity Factor**: Spectrum from strictly pure (0.0) to impure (1.0)
2. **Refactorability**: Dead store ratio and escape analysis (1.0-1.5)
3. **Pattern Factor**: Data flow vs business logic detection (0.7-1.0)

See `src/priority/unified_scorer.rs` for implementation.
```

## Implementation Notes

### Factor Weights

Start conservative, tune based on user feedback:
- Purity: 0.8 (strong signal)
- Refactorability: 1.2 (moderate boost)
- Pattern: 0.9 (gentle adjustment)

### Edge Cases

- **No data flow data**: Fall back to base scoring (graceful degradation)
- **Zero mutations**: Purity factor = 0.0, refactorability = 1.0
- **All dead stores**: Maximum refactorability boost
- **Pattern unclear**: Use neutral factor (0.85)

### Performance Optimization

- Cache factor calculations per function
- Lazy computation: only calculate if data flow present
- Parallel scoring with rayon

## Migration and Compatibility

### Breaking Changes

- Score values will change for all functions with data flow info
- Ranking may shift significantly

### Migration Strategy

1. **Phase 1**: Add factors, default disabled
2. **Phase 2**: Enable for new analyses, keep old scores for comparison
3. **Phase 3**: Default enable after validation period
4. **Phase 4**: Remove feature flag

### Backward Compatibility

- Config option to disable: `scoring.data_flow.enabled = false`
- Score explanations show old vs new for comparison
- JSON output includes both base and adjusted scores
