---
number: 193
title: Score-Based Filtering to Reduce Low-Value Recommendations
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-23
---

# Specification 193: Score-Based Filtering to Reduce Low-Value Recommendations

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently shows all debt items above tier T4 (maintenance), which can create "noise" by displaying low-severity recommendations that developers are unlikely to prioritize. Analysis tools with too many low-priority recommendations lose credibility and usefulness.

### Current Behavior

From a recent analysis:

```
TOP 5 RECOMMENDATIONS

#1 SCORE: 2.41 [LOW]
├─ CONTEXT: Example/demonstration code (pedagogical patterns accepted) (90% dampening applied)
├─ IMPACT: -8 complexity, -6.9 risk

#2 SCORE: 1.00 [LOW]
...

TOTAL DEBT SCORE: 64
DEBT DENSITY: 7.2 per 1K LOC (8852 total LOC)
```

**Problems**:
1. All top recommendations are LOW severity (scores 1.00-2.41)
2. Items with 90% context dampening are still shown
3. Total debt score (64) includes these low-value items
4. Debt density (7.2) is inflated by counting noise
5. Developers see recommendations they'll likely ignore

### Existing Filtering Mechanisms

**Tier-based filtering** (already implemented):
- `TierConfig::show_t4_in_main_report` (default: false) filters T4 maintenance items
- Used by HTML dashboard (`src/io/writers/html.rs:46`)
- Applied via `get_top_mixed_priorities_tiered()` in unified analysis queries

**What's missing**:
- No score threshold filtering within tiers (T1-T3 can still have low scores)
- Context-dampened items with low scores still pass through
- No way to filter items that are technically T2/T3 but practically not actionable
- Total debt score and density include all non-T4 items regardless of score

### HTML Dashboard vs Terminal Output

**HTML dashboard** (`src/io/writers/html.rs`):
- Lines 42-47: Uses `get_top_mixed_priorities_tiered()` with `TierConfig::default()`
- Line 50: `convert_to_unified_format()` recalculates metrics from filtered items
- Line 65: `debt_density` calculated only from items that pass tier filtering

**Terminal output** (`src/priority/formatter.rs`):
- Line 120: Uses `get_top_mixed_priorities()` which filters T4 by default
- But still shows low-score items within T1-T3
- Metrics include all non-T4 items

## Objective

Implement **configurable score-based filtering** to hide low-value recommendations and recalculate debt metrics to exclude filtered items. This will:

1. Reduce noise by hiding items below a score threshold
2. Focus developer attention on actionable, high-impact recommendations
3. Make debt metrics (total score, density) more meaningful
4. Provide configuration options for different team tolerances
5. Maintain consistency between terminal and dashboard outputs

## Requirements

### Functional Requirements

#### FR1: Score Threshold Configuration

Add `min_score_threshold` to filtering configuration:

```toml
# .debtmap.toml
[filtering]
min_score_threshold = 3.0  # Hide items with score < 3.0
# Options:
#   0.0 = show all items (no score filtering)
#   3.0 = balanced (default, hides low-severity noise)
#   5.0 = strict (only medium+ severity)
#   10.0 = very strict (only high+ severity)
```

**Threshold Guidelines**:
- `< 3.0`: LOW severity - maintenance items, context-dampened issues
- `3.0 - 5.0`: LOW-MEDIUM - minor issues worth addressing opportunistically
- `5.0 - 10.0`: MEDIUM - clear technical debt requiring attention
- `10.0+`: HIGH/CRITICAL - urgent issues blocking maintainability

#### FR2: Extend FilterConfig

Extend `src/transformers/filters.rs::FilterConfig`:

```rust
pub struct FilterConfig {
    pub min_complexity: Option<u32>,
    pub max_complexity: Option<u32>,
    pub languages: Option<Vec<Language>>,
    pub file_patterns: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
    pub min_priority: Option<Priority>,
    pub debt_types: Option<Vec<DebtType>>,

    // NEW: Score-based filtering
    pub min_score_threshold: Option<f64>,
}
```

#### FR3: Apply Score Filtering in Query Layer

Modify `src/priority/unified_analysis_queries.rs::get_top_mixed_priorities_tiered()`:

```rust
fn get_top_mixed_priorities_tiered(
    &self,
    n: usize,
    tier_config: &TierConfig,
) -> Vector<DebtItem> {
    use crate::priority::tiers::RecommendationTier;

    let mut all_items: Vec<DebtItem> = Vec::new();

    // Get configurable score threshold
    let min_score = crate::config::get_minimum_score_threshold();

    for item in &self.items {
        let mut item_with_tier = item.clone();
        let tier = classify_tier(item, tier_config);
        item_with_tier.tier = Some(tier);

        // Filter out Tier 4 items unless explicitly requested
        if tier == RecommendationTier::T4Maintenance && !tier_config.show_t4_in_main_report {
            continue;
        }

        // NEW: Filter out items below score threshold
        if item.score < min_score {
            continue;
        }

        all_items.push(DebtItem::Function(Box::new(item_with_tier)));
    }

    // ... rest of function
}
```

#### FR4: Recalculate Metrics After Filtering

Update `src/output/unified.rs::convert_to_unified_format()` to ensure metrics reflect only filtered items:

**Already implemented** (line 448, 468, 496-500):
- Gets filtered items via `get_top_mixed_priorities()`
- Calculates `total_debt_score` from filtered items
- Recalculates `debt_density` from filtered score

**Verify consistency**: Ensure all output formats (terminal, JSON, HTML, markdown) use the same filtering.

#### FR5: Configuration Loading

Add to `src/config/mod.rs`:

```rust
/// Get minimum score threshold for filtering recommendations
pub fn get_minimum_score_threshold() -> f64 {
    CONFIG.with(|config| {
        config
            .borrow()
            .as_ref()
            .and_then(|c| c.filtering.as_ref())
            .and_then(|f| f.min_score_threshold)
            .unwrap_or(3.0) // Default: hide items < 3.0
    })
}
```

#### FR6: Command-Line Override

Add CLI flag to override config:

```bash
debtmap analyze --min-score 5.0  # Override config, show only score >= 5.0
debtmap analyze --min-score 0    # Show all items (disable filtering)
```

Implementation in `src/main.rs`:

```rust
#[derive(Parser)]
struct AnalyzeCommand {
    // ... existing fields ...

    /// Minimum score threshold to show recommendations (0.0 = show all)
    #[arg(long, default_value = None)]
    min_score: Option<f64>,
}
```

#### FR7: Update Terminal Output

No changes needed to `src/priority/formatter.rs` - it already uses `get_top_mixed_priorities()` which will automatically apply score filtering once FR3 is implemented.

**Verify** that metrics display (lines 150-167) shows filtered totals:
- Total debt score reflects filtered items
- Debt density calculated from filtered items
- Item counts exclude filtered items

### Non-Functional Requirements

#### NFR1: Performance

- Filtering must add < 5ms overhead to analysis
- Use single-pass filtering (no multiple iterations)
- Leverage existing score calculations (already computed)

#### NFR2: Backward Compatibility

- Default threshold (3.0) should maintain current behavior for most projects
- Setting `min_score_threshold = 0.0` restores complete output
- No breaking changes to API or output formats

#### NFR3: Documentation

- Update configuration documentation with threshold guidelines
- Explain when to adjust threshold (team preferences, project maturity)
- Provide examples of impact at different threshold levels

#### NFR4: Testing

- Unit tests for score filtering in `FilterConfig`
- Integration tests verifying filtered metrics calculation
- Test cases for edge values (0.0, very high thresholds)

#### NFR5: User Experience

- Clear indication in output when items are filtered
- Suggest adjusting threshold if no recommendations shown
- Maintain clarity about what's being hidden

## Acceptance Criteria

- [ ] Configuration field `min_score_threshold` added to `.debtmap.toml` schema
- [ ] `FilterConfig` extended with `min_score_threshold` field
- [ ] `get_top_mixed_priorities_tiered()` applies score filtering
- [ ] `convert_to_unified_format()` verified to recalculate metrics from filtered items
- [ ] Configuration loading function `get_minimum_score_threshold()` implemented
- [ ] CLI flag `--min-score` added and functional
- [ ] Default threshold (3.0) tested and documented
- [ ] Terminal output shows only items >= threshold
- [ ] HTML dashboard shows only items >= threshold
- [ ] JSON output shows only items >= threshold
- [ ] Total debt score reflects filtered items only
- [ ] Debt density calculated from filtered items only
- [ ] Unit tests for filtering logic (85%+ coverage)
- [ ] Integration tests for different threshold values
- [ ] Documentation updated with threshold guidelines
- [ ] CHANGELOG entry describing feature

## Technical Details

### Implementation Approach

**Phase 1: Configuration** (1-2 hours)
1. Add `min_score_threshold` to config schema
2. Implement `get_minimum_score_threshold()` accessor
3. Add CLI flag parsing

**Phase 2: Filtering Logic** (2-3 hours)
1. Extend `FilterConfig` with score field
2. Modify `get_top_mixed_priorities_tiered()` to apply score filter
3. Verify `convert_to_unified_format()` uses filtered results

**Phase 3: Testing** (2-3 hours)
1. Unit tests for filtering at various thresholds
2. Integration tests for metric recalculation
3. Verify consistency across output formats

**Phase 4: Documentation** (1 hour)
1. Update configuration guide
2. Add threshold selection guidance
3. Document CLI flag usage

### Architecture Changes

**No new modules** - extends existing filtering infrastructure:
- `src/config/mod.rs` - configuration loading
- `src/transformers/filters.rs` - filter data structures
- `src/priority/unified_analysis_queries.rs` - query filtering logic
- `src/main.rs` - CLI argument parsing

### Data Structures

```rust
// Config schema extension
#[derive(Deserialize, Serialize)]
pub struct FilteringConfig {
    pub min_complexity: Option<u32>,
    pub max_complexity: Option<u32>,
    // ... existing fields ...

    /// Minimum score threshold to show recommendations
    /// Default: 3.0 (hide LOW severity items)
    #[serde(default)]
    pub min_score_threshold: Option<f64>,
}
```

### Filtering Flow

```
1. Load config → get_minimum_score_threshold() → 3.0 (default)
2. Query items → get_top_mixed_priorities_tiered()
   ├─ Filter T4 items (existing)
   └─ Filter items < min_score_threshold (new)
3. Calculate metrics → convert_to_unified_format()
   ├─ Sum scores from filtered items
   └─ Calculate density from filtered total
4. Display → formatter/JSON/HTML
   └─ Show only filtered items
```

### Edge Cases

1. **Threshold = 0.0**: Show all items (no score filtering)
2. **Threshold > max score**: Show no items, suggest lowering threshold
3. **All items filtered**: Display message "No recommendations above score threshold X.X. Try --min-score 0 to see all."
4. **Config vs CLI conflict**: CLI flag takes precedence over config file

## Dependencies

**Prerequisites**: None - extends existing filtering system

**Affected Components**:
- `src/config/mod.rs` - configuration schema
- `src/transformers/filters.rs` - filter configuration
- `src/priority/unified_analysis_queries.rs` - query filtering
- `src/output/unified.rs` - metric calculation (verify only)
- `src/main.rs` - CLI argument parsing

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**Configuration Loading**:
```rust
#[test]
fn test_default_score_threshold() {
    let threshold = get_minimum_score_threshold();
    assert_eq!(threshold, 3.0);
}

#[test]
fn test_configured_score_threshold() {
    // Load config with min_score_threshold = 5.0
    let threshold = get_minimum_score_threshold();
    assert_eq!(threshold, 5.0);
}
```

**Filtering Logic**:
```rust
#[test]
fn test_score_filtering_excludes_low_items() {
    let analysis = create_test_analysis_with_scores(vec![
        1.0, 2.5, 3.0, 5.0, 10.0
    ]);
    let tier_config = TierConfig {
        min_score_threshold: 3.0,
        ..Default::default()
    };

    let items = analysis.get_top_mixed_priorities_tiered(10, &tier_config);

    assert_eq!(items.len(), 3); // Only 3.0, 5.0, 10.0 pass
    assert!(items.iter().all(|item| item.score() >= 3.0));
}

#[test]
fn test_score_filtering_disabled_with_zero() {
    let analysis = create_test_analysis_with_scores(vec![
        0.5, 1.0, 2.0, 5.0
    ]);
    let tier_config = TierConfig {
        min_score_threshold: 0.0,
        ..Default::default()
    };

    let items = analysis.get_top_mixed_priorities_tiered(10, &tier_config);

    assert_eq!(items.len(), 4); // All items pass
}
```

**Metric Recalculation**:
```rust
#[test]
fn test_debt_metrics_reflect_filtered_items() {
    let analysis = create_test_analysis_with_scores(vec![
        1.0, 2.0, 5.0, 10.0 // Total raw = 18.0
    ]);

    let output = convert_to_unified_format_with_min_score(&analysis, 3.0);

    assert_eq!(output.summary.total_items, 2); // Only 5.0, 10.0
    assert_eq!(output.summary.total_debt_score, 15.0); // 5.0 + 10.0
    // debt_density should use filtered total
}
```

### Integration Tests

**End-to-End Filtering**:
```rust
#[test]
fn test_terminal_output_respects_score_threshold() {
    let config = create_config_with_min_score(5.0);
    let analysis = analyze_with_config("./fixtures/example_project", &config);

    let output = format_priorities(&analysis, OutputFormat::Default);

    // Verify no items with score < 5.0 in output
    assert!(!output.contains("SCORE: 1."));
    assert!(!output.contains("SCORE: 2."));
    assert!(!output.contains("SCORE: 3."));
    assert!(!output.contains("SCORE: 4."));
}
```

**CLI Flag Override**:
```bash
# Test CLI flag overrides config
./target/debug/debtmap analyze examples/ --min-score 10.0

# Verify output shows only items >= 10.0
```

### Performance Tests

```rust
#[bench]
fn bench_score_filtering(b: &mut Bencher) {
    let analysis = create_large_analysis(10000); // 10k items
    let tier_config = TierConfig {
        min_score_threshold: 3.0,
        ..Default::default()
    };

    b.iter(|| {
        black_box(analysis.get_top_mixed_priorities_tiered(100, &tier_config))
    });
}
```

**Target**: < 5ms overhead for 10,000 items

## Documentation Requirements

### Configuration Guide

Add to `book/src/configuration.md`:

```markdown
## Filtering Configuration

### Score-Based Filtering

Control which recommendations appear in output by setting a minimum score threshold:

```toml
[filtering]
min_score_threshold = 3.0  # Hide items with score < 3.0
```

**Threshold Guidelines**:

| Threshold | Effect | Use Case |
|-----------|--------|----------|
| `0.0` | Show all items (no filtering) | Comprehensive audits, research |
| `3.0` | Hide LOW severity (default) | Balanced view, actionable recommendations |
| `5.0` | Show only MEDIUM+ severity | Focus on clear technical debt |
| `10.0` | Show only HIGH+ severity | Critical issues only |

**Choosing a Threshold**:

- **New projects** or **high standards**: Use 5.0 or higher to focus on significant issues
- **Mature projects**: Use 3.0 (default) for balanced recommendations
- **Comprehensive review**: Use 0.0 to see all detected issues
- **CI/CD gates**: Use 10.0 to fail only on critical problems

**Example**:

```toml
[filtering]
min_score_threshold = 5.0  # Only show medium+ severity
```

**CLI Override**:

```bash
# Temporary override for current run
debtmap analyze --min-score 10.0  # Show only critical items
debtmap analyze --min-score 0     # Show everything
```
```

### Troubleshooting Guide

Add to `book/src/troubleshooting.md`:

```markdown
## No Recommendations Shown

**Problem**: Running `debtmap analyze` shows no recommendations.

**Cause**: All detected issues are below the configured score threshold.

**Solution**:

1. Check your threshold setting:
   ```bash
   grep min_score_threshold .debtmap.toml
   ```

2. Try lowering the threshold:
   ```bash
   debtmap analyze --min-score 0  # Show all items
   ```

3. If items appear with `--min-score 0`, your threshold is too strict:
   ```toml
   [filtering]
   min_score_threshold = 3.0  # Lower from 5.0 or 10.0
   ```
```

## Implementation Notes

### Score vs Tier Filtering

Both filtering mechanisms work together:

1. **Tier filtering** (existing): Filters T4 maintenance items
2. **Score filtering** (new): Filters low-score items within T1-T3

**Combined effect**:
- Item must pass BOTH filters to appear in output
- T1 item with score 2.0 would be hidden (score < 3.0)
- T2 item with score 5.0 would be shown (passes both filters)

### Context Dampening Interaction

Context dampening (spec 192) reduces scores for special cases (examples, generated code). Score filtering naturally handles these:

- Example code with 90% dampening: score 10.0 → 1.0 (filtered out)
- Generated code with 80% dampening: score 15.0 → 3.0 (passes default threshold)

**This is desired behavior** - heavily dampened items shouldn't clutter output.

### Migration Path

**Default behavior change**: None
- Default threshold 3.0 approximates current behavior (T4 already filtered)
- Most T1-T3 items have scores >= 3.0

**Gradual adoption**:
1. Initially: Use default 3.0 (minimal change)
2. Tune based on team: Adjust to 5.0 or higher if too noisy
3. CI/CD: Use stricter thresholds (10.0+) for quality gates

## Migration and Compatibility

**Breaking Changes**: None

**Configuration Migration**:
- Missing `min_score_threshold` → use default 3.0
- Existing configs continue working unchanged

**API Compatibility**:
- `get_top_mixed_priorities()` behavior unchanged (uses default threshold)
- `get_top_mixed_priorities_tiered()` signature unchanged (threshold read from config)

**Output Compatibility**:
- JSON schema unchanged (fewer items, same structure)
- HTML dashboard unchanged (fewer items, same layout)
- Terminal output unchanged (fewer items, same format)

## Alternatives Considered

### Alternative 1: Severity-Based Filtering

Filter by Priority enum (Critical, High, Medium, Low) instead of score.

**Rejected because**:
- Less granular (only 4 levels vs continuous score)
- Ignores context dampening effects
- Score is more meaningful (represents actual impact)

### Alternative 2: Percentile-Based Filtering

Show top N% of items by score.

**Rejected because**:
- Inconsistent across projects (small projects might show noise)
- Harder to configure (what percentile is reasonable?)
- Absolute threshold more intuitive

### Alternative 3: Category-Specific Thresholds

Different thresholds for different debt types (complexity, testing, architecture).

**Rejected for initial implementation**:
- More complex to configure and understand
- Can be added later if needed
- Single threshold sufficient for most teams

## Future Enhancements

**Possible follow-ups** (not in this spec):

1. **Per-category thresholds** (spec 194?):
   ```toml
   [filtering.thresholds]
   architecture = 5.0
   testing = 3.0
   complexity = 10.0
   ```

2. **Adaptive threshold** (spec 195?):
   Automatically adjust based on project size and score distribution

3. **Threshold presets** (spec 196?):
   ```toml
   [filtering]
   preset = "strict"  # Equivalent to min_score_threshold = 10.0
   ```

4. **Filter summary in output**:
   ```
   TOP 10 RECOMMENDATIONS (15 items filtered, score < 3.0)
   ```

## References

- Existing tier filtering: `src/priority/tiers.rs`
- HTML dashboard filtering: `src/io/writers/html.rs:42-67`
- Metric calculation: `src/output/unified.rs:468-500`
- Context dampening: Spec 192 (examples, generated code)
- Filter infrastructure: `src/transformers/filters.rs`

## Success Metrics

**Quantitative**:
- Reduce terminal output noise by 30-50% for typical projects
- Debt density more accurately reflects actionable debt
- < 5ms filtering overhead

**Qualitative**:
- Developers report recommendations are more actionable
- Fewer recommendations ignored or dismissed
- Increased confidence in debt analysis

**Validation**:
Run on `examples/` directory:
```bash
# Before (current behavior)
debtmap analyze examples/ | grep "^#" | wc -l
# After (with default threshold)
debtmap analyze examples/ | grep "^#" | wc -l
# Should show fewer items, all with score >= 3.0
```
