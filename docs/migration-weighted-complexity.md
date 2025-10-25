# Migration Guide: Weighted Complexity Scoring

This guide helps you migrate to the new weighted complexity scoring system introduced in spec 121.

## Overview

Debtmap now uses a weighted complexity scoring system that combines cyclomatic and cognitive complexity with configurable weights. By default, cognitive complexity is weighted more heavily (70%) than cyclomatic complexity (30%) because research shows cognitive complexity correlates better with bug density and maintenance difficulty.

## What's Changed

### Before (Raw Metrics)
```
Complexity Hotspot: Cyclomatic: 15, Cognitive: 3
```

### After (Weighted Scoring)
```
Complexity Hotspot: cyclomatic=15, cognitive=3 → weighted=11.1 (cognitive-driven)
```

The new format shows:
1. Raw cyclomatic complexity value
2. Raw cognitive complexity value
3. Normalized weighted score (0-100 scale)
4. Which metric is driving the score (cognitive-driven or cyclomatic-driven)

## Impact on Prioritization

### Expected Score Changes

**Functions with high cyclomatic but low cognitive complexity will score lower:**
- Example: Mapping functions with many simple cases
- Before: High priority due to high cyclomatic complexity
- After: Medium priority due to low cognitive complexity

**Functions with high cognitive complexity will score higher:**
- Example: Deeply nested conditional logic
- Before: May have been under-prioritized
- After: Higher priority due to cognitive weight

### Threshold Adjustments

If you have custom complexity thresholds in `.debtmap.toml`, you may need to adjust them:

**Old thresholds (raw cyclomatic):**
```toml
[thresholds]
complexity = 15  # Based on cyclomatic complexity
```

**New approach (weighted score):**
```toml
[thresholds]
complexity = 15  # Now interpreted as weighted score

[complexity_weights]
cyclomatic = 0.3
cognitive = 0.7
max_cyclomatic = 50.0
max_cognitive = 100.0
```

## Configuration Options

### Default Configuration (Recommended)

The default weights favor cognitive complexity:

```toml
[complexity_weights]
cyclomatic = 0.3
cognitive = 0.7
max_cyclomatic = 50.0
max_cognitive = 100.0
```

### Custom Weights

You can customize weights based on your team's priorities:

**Balance both metrics equally:**
```toml
[complexity_weights]
cyclomatic = 0.5
cognitive = 0.5
```

**Heavily favor cognitive (for complex business logic):**
```toml
[complexity_weights]
cyclomatic = 0.2
cognitive = 0.8
```

**Favor cyclomatic (for highly branching code):**
```toml
[complexity_weights]
cyclomatic = 0.6
cognitive = 0.4
```

**Important:** Weights must sum to exactly 1.0.

### Adjusting Normalization

Customize the maximum values used for normalization based on your codebase:

```toml
[complexity_weights]
# ... weight configuration ...

# If your codebase has higher complexity values
max_cyclomatic = 75.0   # Increase if you have functions >50 cyclomatic
max_cognitive = 150.0   # Increase if you have functions >100 cognitive
```

To find appropriate max values for your codebase:
1. Run `debtmap analyze . --format json`
2. Look at the highest complexity values in your output
3. Set max values to 20% above your highest actual values

## Opting Out (Not Recommended)

If you need to temporarily revert to raw cyclomatic complexity scoring, you can configure weights to favor cyclomatic:

```toml
[complexity_weights]
cyclomatic = 1.0
cognitive = 0.0
max_cyclomatic = 50.0
max_cognitive = 100.0
```

**Note:** This defeats the purpose of the improved scoring and is not recommended for long-term use.

## Role-Based Weighting

The system also supports automatic weight adjustments based on function role (available in code, not yet exposed in config):

- **Pure functions**: 50% cyclomatic, 50% cognitive (balanced)
- **Orchestrators**: 25% cyclomatic, 75% cognitive (favor understanding flow)
- **Entry points**: 25% cyclomatic, 75% cognitive (favor understanding flow)
- **I/O wrappers**: Default weights (30/70)

This feature helps prioritize functions based on their architectural role.

## Migration Checklist

- [ ] Review your current complexity thresholds
- [ ] Test the new scoring on your codebase
- [ ] Adjust weights if needed for your team's priorities
- [ ] Update any CI/CD scripts that parse debtmap output
- [ ] Communicate changes to your team
- [ ] Update internal documentation with new scoring format

## Examples

### Example 1: Mapping Function (Lower Priority Now)

**Before:**
```
#5 SCORE: 7.2 [HIGH]
├─ COMPLEXITY: src/converter.rs:42 status_to_string()
├─ Cyclomatic: 15, Cognitive: 3
```

**After:**
```
#8 SCORE: 5.4 [MEDIUM]
├─ COMPLEXITY: src/converter.rs:42 status_to_string()
├─ cyclomatic=15, cognitive=3 → weighted=11.1 (cognitive-driven)
```

**Why:** High cyclomatic from many simple cases, but low cognitive complexity means it's actually easy to understand.

### Example 2: Nested Logic (Higher Priority Now)

**Before:**
```
#12 SCORE: 5.8 [MEDIUM]
├─ COMPLEXITY: src/validator.rs:89 validate_config()
├─ Cyclomatic: 8, Cognitive: 25
```

**After:**
```
#3 SCORE: 8.1 [HIGH]
├─ COMPLEXITY: src/validator.rs:89 validate_config()
├─ cyclomatic=8, cognitive=25 → weighted=19.9 (cognitive-driven)
```

**Why:** Deeply nested validation logic is hard to understand, now correctly prioritized higher.

## Benefits

1. **Fewer false positives**: Simple repetitive patterns score lower
2. **Better prioritization**: Truly complex code surfaces first
3. **Research-backed**: Cognitive complexity correlates with bug density
4. **Transparent**: See both raw metrics and the weighted result
5. **Configurable**: Adjust for your team's specific needs

## Getting Help

If you encounter issues or have questions:

1. Check the [Configuration Guide](https://iepathos.github.io/debtmap/configuration.html)
2. Review the [Analysis Guide](https://iepathos.github.io/debtmap/analysis-guide.html)
3. Open an issue: https://github.com/iepathos/debtmap/issues

## Related Documentation

- [Configuration Guide](https://iepathos.github.io/debtmap/configuration.html)
- [Analysis Guide](https://iepathos.github.io/debtmap/analysis-guide.html)
- [CLI Reference](https://iepathos.github.io/debtmap/cli-reference.html)
- Spec 121: Cognitive Complexity Weighted Scoring
