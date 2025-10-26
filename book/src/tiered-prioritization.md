# Tiered Prioritization

Debtmap uses a sophisticated tiered prioritization system to surface critical architectural issues above simple testing gaps. This chapter explains the tier strategy, how to interpret tier classifications, and how to customize tier thresholds for your project.

## Overview

The tiered prioritization system organizes technical debt into four distinct tiers based on impact, urgency, and architectural significance. This prevents "walls of similar-scored items" and ensures critical issues don't get lost among minor problems.

## The Four Tiers

### Tier 1: Critical Architecture

**Description**: God Objects, God Modules, excessive complexity requiring immediate architectural attention

**Priority**: Must address before adding new features

**Weight**: 1.5x (highest priority multiplier)

**Impact**: High impact on maintainability and team velocity

**Examples**:
- Files with 15+ responsibilities
- Modules with 50+ methods
- Functions with cyclomatic complexity > 50 (extreme complexity hotspots requiring architectural redesign)
- God objects flagged by detection algorithms
- Circular dependencies affecting core modules

**When to Address**: Immediately, before sprint work begins. These issues compound over time and block progress.

```bash
# Focus on Tier 1 items
debtmap analyze . --min-priority high --top 5
```

### Tier 2: Complex Untested

**Description**: Untested code with high complexity or critical dependencies. Items qualify for Tier 2 if they meet ANY of: cyclomatic complexity ≥ 15, total dependencies ≥ 10, or are entry point functions with any coverage gap.

**Priority**: Risk of bugs in critical paths

**Weight**: 1.0x (standard multiplier)

**Action**: Should be tested before refactoring to prevent regressions

**Examples**:
- Functions with cyclomatic complexity ≥ 15 and 0% coverage
- Functions with 10+ dependencies and low test coverage
- Business logic entry points without tests
- Complex error handling without validation

**When to Address**: Within current sprint. Add tests before making changes.

```bash
# See Tier 2 testing gaps
debtmap analyze . --lcov coverage.lcov --min-priority high
```

### Tier 3: Testing Gaps

**Description**: Untested code with moderate complexity

**Priority**: Improve coverage to prevent future issues

**Weight**: 0.7x (reduced multiplier)

**Action**: Add tests opportunistically or during related changes

**Examples**:
- Functions with cyclomatic complexity 10-15 and low coverage
- Utility functions without edge case tests
- Moderate complexity with partial coverage

**When to Address**: Next sprint or when touching related code.

### Tier 4: Maintenance

**Description**: Low-complexity issues and code quality improvements

**Priority**: Address opportunistically during other work

**Weight**: 0.3x (lowest multiplier)

**Action**: Fix when convenient, low urgency

**Examples**:
- Simple functions with minor code quality issues
- TODO markers in well-tested code
- Minor duplication in test code

**When to Address**: During cleanup sprints or when refactoring nearby code.

## Configuration

### Default Tier Thresholds

```toml
[tiers]
# Tier 2 thresholds (Complex Untested)
t2_complexity_threshold = 15         # Cyclomatic complexity cutoff
t2_dependency_threshold = 10         # Dependency count cutoff

# Tier 3 thresholds (Testing Gaps)
t3_complexity_threshold = 10         # Lower complexity threshold

# Display options
show_t4_in_main_report = false      # Hide Tier 4 from main output

# Tier weights (multipliers applied to base scores)
t1_weight = 1.5    # Critical architecture
t2_weight = 1.0    # Complex untested
t3_weight = 0.7    # Testing gaps
t4_weight = 0.3    # Maintenance
```

### Customizing Tier Thresholds

Adjust thresholds to match your team's standards:

```toml
# Stricter thresholds for high-quality codebases
[tiers]
t2_complexity_threshold = 12
t3_complexity_threshold = 8

# More lenient for legacy codebases
[tiers]
t2_complexity_threshold = 20
t3_complexity_threshold = 15
```

### Tier Weight Customization

Tier weights are multipliers applied to base debt scores during prioritization. A weight of 1.5 means items in that tier will score 50% higher than equivalent items in a tier with weight 1.0, pushing them higher in priority rankings.

Adjust weights based on your priorities:

```toml
# Emphasize testing over architecture
[tiers]
t1_weight = 1.2    # Reduce architecture weight
t2_weight = 1.3    # Increase testing weight
t3_weight = 0.8
t4_weight = 0.3

# Focus on architecture first
[tiers]
t1_weight = 2.0    # Maximize architecture weight
t2_weight = 1.0
t3_weight = 0.5
t4_weight = 0.2
```

## Use Cases

### Sprint Planning

Use tiered prioritization to allocate work:

```bash
# See Tier 1 items for architectural planning
debtmap analyze . --min-priority high --top 5

# See Tier 2/3 for testing sprint work
debtmap analyze . --lcov coverage.lcov --min-priority medium
```

### Code Review Focus

Prioritize review attention based on tiers:
- **Tier 1**: Architectural review required, senior dev attention
- **Tier 2**: Test coverage validation critical
- **Tier 3**: Standard review process
- **Tier 4**: Quick review or automated checks

### Refactoring Strategy

```bash
# Phase 1: Address Tier 1 architectural issues
debtmap analyze . --min-priority high

# Phase 2: Add tests for Tier 2 complex code
debtmap analyze . --lcov coverage.lcov --min-priority high

# Phase 3: Improve Tier 3 coverage
debtmap analyze . --lcov coverage.lcov --min-priority medium
```

## Best Practices

1. **Always address Tier 1 before feature work** - Architectural issues compound
2. **Test Tier 2 items before refactoring** - Avoid regressions
3. **Batch Tier 3 items** - Address multiple in one sprint
4. **Defer Tier 4 items** - Only fix during cleanup or when convenient
5. **Track tier distribution over time** - Aim to reduce Tier 1/2 counts

## Interpreting Tier Output

### Terminal Output

```
════════════════════════════════════════════
    TIERED TECHNICAL DEBT REPORT
════════════════════════════════════════════

🔴 TIER 1: CRITICAL ARCHITECTURE (3 items)
  1. src/services.rs - God Object (85% god score, 52 methods)
  2. src/core/engine.rs - Circular dependency with parsers module
  3. src/api/handlers.rs - God Module (15 responsibilities)

🟠 TIER 2: COMPLEX UNTESTED (12 items)
  1. src/processing/transform.rs:145 - Complexity 18, Coverage 0%
  ...

🟡 TIER 3: TESTING GAPS (45 items)
  ...

⚪ TIER 4: MAINTENANCE (120 items) [hidden]
  To show Tier 4 items, add show_t4_in_main_report = true under [tiers] in .debtmap.toml
```

### JSON Output

Tier values use PascalCase enum variants without underscores: `T1CriticalArchitecture`, `T2ComplexUntested`, `T3TestingGaps`, `T4Maintenance`.

```json
{
  "tier_distribution": {
    "t1_count": 3,
    "t2_count": 12,
    "t3_count": 45,
    "t4_count": 120
  },
  "items": [
    {
      "tier": "T1CriticalArchitecture",
      "priority_weight": 1.5,
      "base_score": 8.5,
      "final_score": 12.75
    }
  ]
}
```

## Troubleshooting

**Issue**: Too many Tier 1 items

**Solution**: Lower tier weights or increase thresholds temporarily:
```toml
[tiers]
t1_weight = 1.2    # Reduce from 1.5
```

**Issue**: Not enough items in Tier 1

**Solution**: Check if god object detection is enabled:
```toml
[god_object_detection]
enabled = true
```

**Issue**: All items in Tier 4

**Solution**: Lower minimum thresholds:
```toml
[thresholds]
minimum_debt_score = 1.0
minimum_cyclomatic_complexity = 2
```

## See Also

- [Scoring Strategies](./scoring-strategies.md) - Understanding file-level vs function-level scoring
- [Configuration](./configuration.md) - Complete configuration reference
- [Analysis Guide](./analysis-guide.md) - Detailed metric explanations
