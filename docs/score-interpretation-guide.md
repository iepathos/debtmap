# Debtmap Score Interpretation Guide

## Overview

As of spec 96, Debtmap scores are no longer capped at 10.0. This change allows for more accurate representation of extreme technical debt cases and provides better differentiation between high-risk items.

**Update (spec 101)**: Score normalization now uses a three-stage scaling approach with smooth transitions between linear, square root, and logarithmic scaling for better score distribution across all ranges.

## Score Ranges and Interpretation

### 0.0 - 2.0: Minimal Debt
- Well-tested code with good coverage
- Low complexity
- Few dependencies
- **Action**: Regular maintenance only

### 2.0 - 4.0: Low Debt
- Reasonable test coverage
- Manageable complexity
- Normal dependency count
- **Action**: Monitor during regular reviews

### 4.0 - 6.0: Moderate Debt
- Some testing gaps
- Increasing complexity
- Growing dependencies
- **Action**: Schedule for refactoring in next sprint

### 6.0 - 8.0: High Debt
- Significant coverage gaps
- High complexity metrics
- Many dependencies
- **Action**: Priority refactoring needed

### 8.0 - 10.0: Very High Debt
- Critical coverage gaps
- Very high complexity
- Extensive dependencies
- **Action**: Immediate attention required

### 10.0+: Extreme Debt
- **New with spec 96**: Scores can now exceed 10.0
- Represents compound risk factors
- Multiple critical issues present
- **Action**: Critical - block other work to address

## Key Changes from Previous Scoring

### Before Spec 96
- All scores were capped at 10.0
- Large, complex files couldn't be differentiated beyond the cap
- Risk plateaued for extreme cases

### After Spec 96
- No artificial ceiling on scores
- Better differentiation of extreme cases
- More accurate risk representation
- Square root scaling above 10.0 for gradual increase

### After Spec 101 (Current)
- **Three-stage normalization**:
  - **Linear scaling (0-10)**: Raw scores 0-10 map directly to normalized 0-10
  - **Square root scaling (10-100)**: Raw scores 10-100 map to normalized 10-40 range
  - **Logarithmic scaling (100+)**: Raw scores 100+ map to normalized 40+ with slow growth
- **Smooth transitions**: Continuous function ensures no jumps at boundaries
- **Better distribution**: Each range optimized for typical score patterns

## Score Components

### Coverage Factor
- 0% coverage: Factor ~1.1
- 50% coverage: Factor ~0.45
- 100% coverage: Factor ~0.1

### Complexity Factor
- Low (0-5): Linear scaling (0-3)
- Medium (6-10): Moderate scaling (3-6)
- High (11+): Power scaling (6+)

### Dependency Factor
- 0 dependencies: Factor 0.1
- 1-2 dependencies: Factor 0.5-0.7
- 10+ dependencies: Factor ~1.65

### Interaction Bonus
- Applied when low coverage meets high complexity
- Multiplier of 1.5x for compound risk

## Migration Notes

### For CI/CD Pipelines
- **Update threshold checks**: Systems expecting max score of 10.0 need updating
- **Adjust quality gates**: Consider new thresholds for blocking deployments
- **Review alerting**: Update monitoring alerts for new score ranges

### For Development Teams
- **Rebaseline metrics**: Historical comparisons may need adjustment
- **Update dashboards**: Visualization scales may need extending
- **Review priorities**: Items previously at 10.0 may now show true severity

### For Analysis Tools
- **Score normalization**: Remove any hardcoded 10.0 caps
- **Percentile calculations**: Recalculate based on new distribution
- **Trend analysis**: Account for scoring change in historical data

## Practical Examples

### Example 1: Simple Function
```
Coverage: 80%
Complexity: 5
Dependencies: 2
Raw Score: ~2.3
Normalized: 2.3 (Linear range)
```

### Example 2: Complex Module
```
Coverage: 30%
Complexity: 25
Dependencies: 8
Raw Score: ~45
Normalized: ~16.7 (Square root range)
```

### Example 3: Critical Legacy Code
```
Coverage: 0%
Complexity: 50
Dependencies: 15
Raw Score: ~250
Normalized: ~50.8 (Logarithmic range)
```

### Score Normalization Examples
```
Raw Score → Normalized Score (Scaling Method)
5.0 → 5.0 (Linear)
10.0 → 10.0 (Linear/Square root boundary)
50.0 → 23.7 (Square root)
100.0 → 41.6 (Square root/Logarithmic boundary)
500.0 → 57.7 (Logarithmic)
1000.0 → 64.6 (Logarithmic)
```

## Best Practices

1. **Focus on trends**: Rising scores indicate accumulating debt
2. **Set team thresholds**: Define acceptable ranges for your codebase
3. **Prioritize by score**: Address highest scores first
4. **Monitor outliers**: Items scoring >10 need immediate attention
5. **Regular reviews**: Track score changes over time

## FAQ

### Q: Why was the cap removed?
A: To provide better differentiation between high-risk items and more accurately represent extreme technical debt cases.

### Q: How should we handle scores above 10?
A: Treat them as critical issues requiring immediate attention. These represent compound risks that significantly impact maintainability.

### Q: Will scores continue to increase infinitely?
A: While there's no hard cap, the three-stage scaling approach (spec 101) ensures scores increase gradually:
- Linear growth for low scores (0-10)
- Square root scaling for medium scores (10-100)
- Logarithmic scaling for high scores (100+)
This provides meaningful differentiation at all levels while preventing runaway scores.

### Q: How does this affect our existing tooling?
A: Review any tools that assume a maximum score of 10.0. Update thresholds, visualizations, and quality gates accordingly.