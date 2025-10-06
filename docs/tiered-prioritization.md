# Tiered Prioritization Strategy

## Overview

Debtmap uses a four-tier prioritization strategy to surface critical architectural issues above simple testing gaps. This prevents "walls of similar-scored items" and helps users focus on the most impactful work first.

## Problem Statement

Without tiered prioritization, a project with:
- 1 god object (high complexity, architectural debt)
- 50 untested functions (moderate complexity, testing debt)

Would surface a "wall" of 50 similar-scored testing items, burying the critical architectural issue that should be addressed first.

## Solution: Four-Tier Classification

### Tier 1: Critical Architecture
**Priority:** Highest
**Weight:** 1.5x (configurable)

**Criteria:**
- God Objects (15+ responsibilities)
- God Modules (files with excessive complexity)
- Complexity hotspots (cyclomatic > 30)

**Rationale:**
Must address before adding new features. High impact on maintainability. These issues create systemic problems that slow down all future development.

**Examples:**
- `UserService` with 20 responsibilities (authentication, validation, persistence, email, logging, etc.)
- 800-line file with 50 methods and 15 fields
- Single function with cyclomatic complexity of 45

### Tier 2: Complex Untested
**Priority:** High
**Weight:** 1.0x (configurable)

**Criteria:**
- Cyclomatic complexity ≥ 15 (configurable via `t2_complexity_threshold`)
- Coverage = 0%
- OR upstream dependencies ≥ 10 (configurable via `t2_dependency_threshold`)

**Rationale:**
Risk of bugs in critical paths. Should be tested before refactoring. Complex untested code represents immediate technical risk.

**Examples:**
- Payment processing function with complexity 18 and 0% test coverage
- Authentication middleware with 12 upstream callers and no tests
- Database transaction handler with nested error paths, untested

### Tier 3: Testing Gaps
**Priority:** Medium
**Weight:** 0.7x (configurable)

**Criteria:**
- Cyclomatic complexity 10-15 (configurable via `t3_complexity_threshold`)
- Coverage < 80%
- Not already T1 or T2

**Rationale:**
Improve coverage to prevent future issues. Lower priority than architectural debt. These are important but not urgent.

**Examples:**
- Validation function with complexity 12 and 40% coverage
- Data transformation with moderate branching logic
- Helper function with some edge cases untested

### Tier 4: Maintenance
**Priority:** Low
**Weight:** 0.3x (configurable)

**Criteria:**
- Everything else (low complexity, minor issues)
- Hidden by default (configurable via `show_t4_in_main_report`)

**Rationale:**
Address opportunistically. Minimal impact on system health. These can wait until other work is complete.

**Examples:**
- Simple getter with dead code warning
- Low-complexity function with minor code smell
- Trivial helper with style issue

## Implementation

### Classification Logic

Located in `src/priority/tiers.rs`:

```rust
pub fn classify_tier(item: &UnifiedDebtItem, config: &TierConfig) -> RecommendationTier {
    // T1: Architectural issues (god objects, high complexity)
    if matches!(item.debt_type, DebtType::GodObject { .. }) {
        return RecommendationTier::T1CriticalArchitecture;
    }

    // T2: Complex untested code
    if (item.cyclomatic_complexity >= config.t2_complexity_threshold
        || item.upstream_dependencies >= config.t2_dependency_threshold)
        && matches!(item.debt_type, DebtType::TestingGap { coverage, .. } if coverage < 0.1)
    {
        return RecommendationTier::T2ComplexUntested;
    }

    // T3: Testing gaps (moderate complexity)
    if item.cyclomatic_complexity >= config.t3_complexity_threshold {
        return RecommendationTier::T3TestingGaps;
    }

    // T4: Everything else
    RecommendationTier::T4Maintenance
}
```

### Score Adjustment

Tier weights multiply the base unified score:

```rust
let tier_weight = item.tier.unwrap_or(RecommendationTier::T4Maintenance).weight(config);
let adjusted_score = item.unified_score.final_score * tier_weight;
```

This ensures:
- T1 items (1.5x) always appear before T2 items (1.0x) with similar base scores
- T2 items appear before T3 items (0.7x)
- T3 items appear before T4 items (0.3x)

### Integration with Unified Scoring

The tiered prioritization works in conjunction with the unified scoring system:

1. **Base Score Calculation** - Computes complexity, coverage, and dependency factors
2. **Tier Classification** - Assigns item to T1, T2, T3, or T4
3. **Tier Weight Application** - Multiplies base score by tier weight
4. **Final Ranking** - Sorts by adjusted score

This creates a "tiered ladder" where:
- All T1 items rank above all T2 items
- All T2 items rank above all T3 items
- All T3 items rank above all T4 items

Within each tier, items are sorted by their base unified score.

## Configuration

Users can customize tier thresholds and weights in `.debtmap.toml`:

```toml
[tiers]
# Complexity threshold for Tier 2 (complex untested code)
t2_complexity_threshold = 15

# Dependency threshold for Tier 2
t2_dependency_threshold = 10

# Complexity threshold for Tier 3 (testing gaps)
t3_complexity_threshold = 10

# Show Tier 4 items in main report (default: false)
show_t4_in_main_report = false

# Tier weights for score adjustment
t1_weight = 1.5  # Boost architectural issues
t2_weight = 1.0  # Standard weight for complex untested
t3_weight = 0.7  # Lower priority for testing gaps
t4_weight = 0.3  # Minimal priority for maintenance
```

## Category Filtering

Users can filter by debt category to focus on specific types:

```bash
# Show only architectural issues (T1 focus)
debtmap analyze . --filter Architecture

# Show only testing issues (T2, T3 focus)
debtmap analyze . --filter Testing

# Show both
debtmap analyze . --filter Architecture,Testing
```

Available categories:
- **Architecture** - God objects, complexity hotspots, dead code
- **Testing** - Testing gaps, low coverage areas
- **Performance** - Resource leaks, inefficient patterns
- **CodeQuality** - Code smells, maintainability issues

Implementation in `src/priority/mod.rs`:

```rust
pub fn filter_by_categories(&self, categories: &[DebtCategory]) -> Self {
    let filtered_items: Vector<UnifiedDebtItem> = self
        .items
        .iter()
        .filter(|item| {
            let item_category = DebtCategory::from_debt_type(&item.debt_type);
            categories.contains(&item_category)
        })
        .cloned()
        .collect();
    // ...
}
```

## Output Formatting

Tier labels appear in markdown output:

```markdown
### #1 [T1] Score: 8.5 [HIGH]
**Type:** God Object | **Location:** `src/user_service.rs:42 handle_user()`

### #2 [T2] Score: 7.2 [MEDIUM]
**Type:** Testing Gap | **Location:** `src/payment.rs:100 process_payment()`

### #3 [T3] Score: 5.8 [MEDIUM]
**Type:** Testing Gap | **Location:** `src/validator.rs:30 validate_input()`
```

The tier labels help users quickly identify:
- What type of issue they're looking at
- Why it was prioritized this way
- What the remediation strategy should be

## Architectural Decisions

### Why Four Tiers?

- **T1 (Architecture)**: Clear top priority - systemic issues
- **T2 (Complex Untested)**: High-risk code that needs immediate attention
- **T3 (Testing)**: Important but lower urgency
- **T4 (Maintenance)**: Can be deferred

Fewer tiers would not provide enough granularity. More tiers would create confusion.

### Why Multiply Base Scores?

Alternatives considered:
1. **Fixed score ranges** (T1: 8-10, T2: 5-8, etc.)
   - Problem: Items would cluster at tier boundaries
   - Problem: Doesn't preserve relative differences within tiers

2. **Add tier bonus** (T1: +3, T2: +1, etc.)
   - Problem: Small bonuses might not overcome large base score differences
   - Problem: Could still allow T2 to overtake T1

3. **Multiply base scores** (chosen)
   - Preserves relative ordering within tiers
   - Guarantees tier separation (T1 always > T2)
   - Configurable via weights

### Why Default Weights (1.5, 1.0, 0.7, 0.3)?

Chosen to create clear separation:
- T1 @ 1.5x ensures architectural issues always surface first
- T2 @ 1.0x baseline for complex untested code
- T3 @ 0.7x reduces testing gap priority (still important)
- T4 @ 0.3x significantly de-prioritizes maintenance items

Users can tune these weights based on their project's needs.

## Testing Strategy

Integration tests verify:
1. Tier classification logic (`tests/test_category_filtering.rs`)
2. Category filtering (`--filter Architecture,Testing`)
3. Tier label display in output
4. Configuration file parsing for `[tiers]` section

Unit tests verify:
1. Tier weight calculation
2. Score adjustment logic
3. Tier boundary conditions

## Future Enhancements

Potential improvements:
1. **Dynamic tier thresholds** based on project statistics
2. **Tier-specific recommendations** (different advice for T1 vs T3)
3. **Tier progress tracking** (show reduction in T1/T2 over time)
4. **Custom tier definitions** (user-defined tier rules)

## References

- Spec 125: Improve Coverage-Driven Prioritization
- `src/priority/tiers.rs` - Tier classification logic
- `src/priority/mod.rs` - Unified analysis and filtering
- `src/priority/formatter_markdown.rs` - Tier label display
- `src/config.rs` - Tier configuration
