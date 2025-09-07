# Migration Notes: Spec 96 - Uncapped Scoring System

## Overview

Spec 96 removes the 10.0 score cap from Debtmap's scoring system. This document provides migration guidance for teams upgrading to this version.

## Breaking Changes

### 1. Score Range Changes
- **Before**: Scores ranged from 0.0 to 10.0 (hard cap)
- **After**: Scores start at 0.0 with no upper limit
- **Impact**: Any code expecting maximum score of 10.0 needs updating

### 2. Test Assertions
- Tests checking for `score <= 10.0` will fail for high-risk code
- Tests expecting exact score of 10.0 for extreme cases need adjustment

### 3. API Response Changes
- JSON responses may now include scores > 10.0
- Clients parsing scores need to handle larger values

## Migration Steps

### Step 1: Update Score Handling Code

#### Before:
```rust
fn validate_score(score: f64) -> bool {
    score >= 0.0 && score <= 10.0
}
```

#### After:
```rust
fn validate_score(score: f64) -> bool {
    score >= 0.0  // No upper bound check
}
```

### Step 2: Update Threshold Configurations

#### Before:
```yaml
quality_gates:
  fail_threshold: 8.0    # 80% of max
  warn_threshold: 6.0    # 60% of max
```

#### After:
```yaml
quality_gates:
  fail_threshold: 8.0    # Absolute value (not percentage)
  warn_threshold: 6.0    # Consider adjusting based on new distributions
  critical_threshold: 10.0  # New: anything above old max
```

### Step 3: Update Visualizations

#### Before:
```javascript
const scoreScale = d3.scaleLinear()
  .domain([0, 10])
  .range([0, 100]);
```

#### After:
```javascript
const scoreScale = d3.scaleLinear()
  .domain([0, Math.max(10, maxObservedScore)])
  .range([0, 100]);
```

### Step 4: Update CI/CD Pipelines

#### Before:
```bash
if [ $(echo "$SCORE > 8.0" | bc) -eq 1 ]; then
  echo "Score too high (max 10.0)"
  exit 1
fi
```

#### After:
```bash
if [ $(echo "$SCORE > 8.0" | bc) -eq 1 ]; then
  if [ $(echo "$SCORE > 10.0" | bc) -eq 1 ]; then
    echo "CRITICAL: Score exceeds old maximum!"
  fi
  echo "Score too high: $SCORE"
  exit 1
fi
```

### Step 5: Update Monitoring and Alerts

#### Before:
```yaml
alerts:
  - name: high_debt_score
    condition: score > 9.0
    severity: warning
```

#### After:
```yaml
alerts:
  - name: high_debt_score
    condition: score > 8.0 AND score <= 10.0
    severity: warning
  - name: extreme_debt_score
    condition: score > 10.0
    severity: critical
```

## Testing Your Migration

### 1. Run Test Suite
```bash
cargo test
```
Expect some test failures related to score caps - these are intentional and should be updated.

### 2. Verify Score Calculations
```bash
# Analyze a known complex file
debtmap analyze src/complex_module.rs --verbose

# Check for scores > 10.0
debtmap analyze . --format json | jq '.files[] | select(.score > 10)'
```

### 3. Validate CI/CD Integration
- Run a build with intentionally complex code
- Verify proper handling of scores > 10.0
- Ensure alerts fire correctly

## Rollback Plan

If you need to temporarily restore the 10.0 cap:

### Option 1: Use Compatibility Mode (if available)
```bash
DEBTMAP_SCORE_CAP=10.0 debtmap analyze .
```

### Option 2: Post-process Scores
```python
def cap_scores(results):
    for file in results['files']:
        file['score'] = min(file['score'], 10.0)
    return results
```

### Option 3: Revert to Previous Version
```bash
cargo install debtmap --version 0.95.0  # Last version with cap
```

## Common Issues and Solutions

### Issue 1: Dashboard Shows Broken Scale
**Solution**: Update visualization libraries to handle dynamic ranges

### Issue 2: Historical Comparisons Look Wrong
**Solution**: Either recalculate historical data or add a note about the scoring change

### Issue 3: Quality Gates Blocking Everything
**Solution**: Temporarily increase thresholds while establishing new baselines

### Issue 4: Database Schema Constraints
**Solution**: Update score column constraints to remove max value check
```sql
ALTER TABLE debt_scores 
DROP CONSTRAINT scores_max_check;
```

## Support and Resources

- **Documentation**: See [Score Interpretation Guide](./score-interpretation-guide.md)
- **Examples**: Check `examples/` directory for updated configurations
- **Issues**: Report problems to the Debtmap issue tracker

## Timeline Recommendations

1. **Week 1**: Update test suites and development environments
2. **Week 2**: Update CI/CD pipelines and monitoring
3. **Week 3**: Deploy to staging and validate
4. **Week 4**: Production deployment with monitoring

## Verification Checklist

- [ ] All tests pass with updated assertions
- [ ] CI/CD pipelines handle scores > 10.0
- [ ] Monitoring alerts configured for new ranges
- [ ] Dashboards display scores correctly
- [ ] Team briefed on interpretation changes
- [ ] Documentation updated with new thresholds
- [ ] Historical data migration plan in place