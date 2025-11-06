# Tiered Prioritization

Debtmap uses a sophisticated tiered prioritization system to surface critical architectural issues above simple testing gaps. This chapter explains the tier strategy, how to interpret tier classifications, and how to customize tier thresholds for your project.

## Overview

The tiered prioritization system organizes technical debt into four distinct tiers based on impact, urgency, and architectural significance. This prevents "walls of similar-scored items" and ensures critical issues don't get lost among minor problems.

**Two Tier Systems**: Debtmap uses two complementary tier systems:
1. **RecommendationTier** (T1-T4): Used internally to classify items based on architectural significance and testing needs
2. **Display Tier** (Critical/High/Moderate/Low): Score-based tiers shown in terminal output, derived from final calculated scores

The configuration examples below control the RecommendationTier classification logic, which influences scoring through tier weights. The final display uses score-based tiers for consistency across all output formats.

## The Four Tiers

### Tier 1: Critical Architecture

**Description**: God Objects, God Modules, excessive complexity requiring immediate architectural attention

**Priority**: Must address before adding new features

**Weight**: 1.5x (highest priority multiplier)

**Impact**: High impact on maintainability and team velocity

**Examples**:
- Files with 15+ responsibilities
- Modules with 50+ methods
- ComplexityHotspot debt items with cyclomatic complexity > 50 (extreme complexity requiring architectural redesign)
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

Tier configuration is optional in `.debtmap.toml`. If not specified, Debtmap uses the balanced defaults shown below.

### Default Tier Thresholds

```toml
[tiers]
# Tier 2 thresholds (Complex Untested)
t2_complexity_threshold = 15         # Cyclomatic complexity cutoff
t2_dependency_threshold = 10         # Dependency count cutoff

# Tier 3 thresholds (Testing Gaps)
t3_complexity_threshold = 10         # Lower complexity threshold

# Display options
show_t4_in_main_report = false      # Hide Tier 4 from main output (not yet implemented)

# Tier weights (multipliers applied to base scores)
t1_weight = 1.5    # Critical architecture
t2_weight = 1.0    # Complex untested
t3_weight = 0.7    # Testing gaps
t4_weight = 0.3    # Maintenance
```

To use tier-based prioritization with custom settings, add the `[tiers]` section to your `.debtmap.toml` configuration file:

```bash
# Analyze with custom tier configuration
debtmap analyze . --config .debtmap.toml
```

### Tier Preset Configurations

Debtmap provides three built-in tier presets for different project needs:

**Balanced (Default)**
```toml
[tiers]
t2_complexity_threshold = 15
t2_dependency_threshold = 10
t3_complexity_threshold = 10
```
Suitable for most projects. Balances detection sensitivity with manageable issue counts.

**Strict**
```toml
[tiers]
t2_complexity_threshold = 10
t2_dependency_threshold = 7
t3_complexity_threshold = 7
```
For high-quality codebases or teams with strict quality standards. Flags more items as requiring attention.

**Lenient**
```toml
[tiers]
t2_complexity_threshold = 20
t2_dependency_threshold = 15
t3_complexity_threshold = 15
```
For legacy codebases or gradual technical debt reduction. Focuses on the most critical issues first.

**Programmatic Access**: These presets are also available as methods when using Debtmap as a library:
- `TierConfig::balanced()` - Equivalent to the balanced preset above
- `TierConfig::strict()` - Equivalent to the strict preset above
- `TierConfig::lenient()` - Equivalent to the lenient preset above

These methods can be used in Rust code to configure tier settings programmatically without manual TOML configuration.

### Customizing Tier Thresholds

You can also create custom threshold configurations tailored to your project:

```toml
# Custom thresholds for specific project needs
[tiers]
t2_complexity_threshold = 12
t2_dependency_threshold = 8
t3_complexity_threshold = 8
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

Terminal output displays items grouped by **score-based tiers**:

```
TECHNICAL DEBT ANALYSIS - PRIORITY TIERS

Critical (score >= 90)
  src/services.rs - God Object (score: 127.5)
  src/core/engine.rs - Circular dependency (score: 95.2)

High (score 70-89.9)
  src/processing/transform.rs:145 - UntestableComplexity (score: 85.0)
  src/api/handlers.rs - God Module (score: 78.3)
  ...

Moderate (score 50-69.9)
  src/utils/parser.rs:220 - TestingGap (score: 62.1)
  ...

Low (score < 50)
  [Items with score < 50 appear here]
```

**Note**: The scores shown reflect tier weight multipliers applied during classification. Items classified as Tier 1 (Critical Architecture) receive a 1.5x weight boost, which often elevates them into the Critical or High score ranges.

### JSON Output

JSON output uses the same **score-based priority** levels as terminal output:

```json
{
  "summary": {
    "score_distribution": {
      "critical": 2,
      "high": 5,
      "medium": 12,
      "low": 45
    }
  },
  "items": [
    {
      "type": "File",
      "score": 127.5,
      "priority": "critical",
      "location": {
        "file": "src/services.rs"
      },
      "debt_type": "GodObject"
    },
    {
      "type": "Function",
      "score": 85.0,
      "priority": "high",
      "location": {
        "file": "src/processing/transform.rs",
        "line": 145,
        "function": "process_data"
      },
      "debt_type": "UntestableComplexity"
    }
  ]
}
```

The `priority` field is derived from the `score` field using these thresholds:
- `critical`: score >= 100.0
- `high`: score >= 50.0
- `medium`: score >= 20.0
- `low`: score < 20.0

**Note**: While RecommendationTier (T1-T4) classifications exist internally for applying tier weights, they are not included in JSON output. The output shows final calculated scores and their corresponding priority levels.

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
