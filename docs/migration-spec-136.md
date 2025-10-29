# Migration Guide: Spec 136 - Rebalanced Scoring Algorithm

## Overview

Spec 136 introduces a rebalanced debt scoring algorithm that fundamentally changes how debtmap prioritizes technical debt. This guide helps you understand the changes and migrate smoothly.

## What's Changing

### Before (Legacy Scoring)
- **Heavy size emphasis**: Large files dominated priority lists regardless of complexity
- **Multiplicative coverage**: Coverage gaps multiplied all other factors
- **Limited customization**: Few options to adjust prioritization strategy
- **Size weight**: ~1.5 (high impact on scores)

### After (Rebalanced Scoring)
- **Quality emphasis**: Complexity + coverage gaps prioritized over size
- **Additive bonuses**: Complex + untested gets +20 bonus (not multiplicative)
- **Preset strategies**: Four built-in presets for different priorities
- **Size weight**: 0.3 (80% reduction from legacy)

## Impact on Your Workflow

### Scores Will Change Significantly

**Large Simple Files**: Will rank LOWER
```
Before: src/config/flags.rs
  2000 lines, complexity: 3
  Score: 125.0 (HIGH priority)

After: src/config/flags.rs
  2000 lines, complexity: 3
  Score: 15.2 (LOW priority)

Reason: Size no longer dominates; low complexity = low priority
```

**Complex Untested Code**: Will rank HIGHER
```
Before: src/payment/processor.rs:142
  150 lines, complexity: 42, coverage: 38%
  Score: 45.8 (MEDIUM priority)

After: src/payment/processor.rs:142
  150 lines, complexity: 42, coverage: 38%
  Score: 95.3 (CRITICAL priority)

Reason: Complexity + coverage gap now prioritized
```

### Priority List Changes

**Before** (legacy scoring top 5):
```
1. config/flags.rs - Score: 125.0 (2000 lines, simple)
2. utils/helpers.rs - Score: 118.5 (1800 lines, simple)
3. payment/processor.rs:142 - Score: 45.8 (complex, untested)
4. models/schema.rs - Score: 42.1 (large schema file)
5. templates/renderer.rs - Score: 38.9 (large template)
```

**After** (rebalanced scoring top 5):
```
1. payment/processor.rs:142 - Score: 95.3 (complex, untested)
2. auth/validator.rs:89 - Score: 87.2 (complex, low coverage)
3. api/handler.rs:256 - Score: 76.5 (moderate complexity, no tests)
4. utils/parser.rs:45 - Score: 62.1 (complex logic, partial coverage)
5. config/flags.rs - Score: 15.2 (large but simple)
```

**Result**: Your team will focus on actual code quality issues instead of file size.

## Migration Strategies

### Strategy 1: Immediate Switch (Recommended)

Best for teams ready to prioritize code quality over file size.

**Steps**:
1. Run analysis with rebalanced scoring:
   ```bash
   debtmap analyze . --scoring-strategy rebalanced
   ```

2. Review new top priorities:
   ```bash
   debtmap analyze . --scoring-strategy rebalanced --top 20
   ```

3. Update your `.debtmap.toml`:
   ```toml
   [scoring_rebalanced]
   preset = "balanced"
   ```

4. Communicate changes to your team:
   - "We're now prioritizing complex untested code over large simple files"
   - "Focus on improving test coverage for complex functions"
   - "File splitting is lower priority unless it has structural issues"

**Pros**:
- Immediate quality benefits
- Team focuses on high-impact issues
- Better test coverage prioritization

**Cons**:
- Requires team adjustment
- Existing workflows may need updates
- Metrics dashboards may show different trends

### Strategy 2: Gradual Migration

Best for teams with established workflows or concerns about disruption.

**Steps**:

1. **Week 1: Analysis and Comparison**
   ```bash
   # Compare legacy vs rebalanced
   debtmap analyze . --scoring-strategy legacy -o legacy.json
   debtmap analyze . --scoring-strategy rebalanced -o rebalanced.json
   debtmap compare --before legacy.json --after rebalanced.json
   ```

2. **Week 2-3: Pilot with Quality Preset**
   ```toml
   [scoring_rebalanced]
   preset = "quality-focused"  # More aggressive than balanced
   ```

   Run on a subset of your codebase to validate results.

3. **Week 4: Team Training**
   - Present comparison results to team
   - Explain new prioritization philosophy
   - Show examples of what will change

4. **Week 5: Full Rollout**
   ```toml
   [scoring_rebalanced]
   preset = "balanced"  # Switch to balanced for production
   ```

**Pros**:
- Smooth transition
- Team buy-in before changes
- Opportunity to validate results

**Cons**:
- Slower to realize benefits
- More coordination overhead

### Strategy 3: Hybrid Approach

Best for teams managing large legacy codebases.

Use different presets for different parts of your codebase:

**New Features** (quality-focused):
```toml
# .debtmap.toml in src/features/
[scoring_rebalanced]
preset = "quality-focused"
```

**Legacy Code** (size-focused):
```toml
# .debtmap.toml in src/legacy/
[scoring_rebalanced]
preset = "size-focused"
```

**Test Gap Focus** (test-coverage preset):
```toml
# .debtmap.toml in src/core/
[scoring_rebalanced]
preset = "test-coverage"
```

## Restoring Legacy Behavior

If you need to maintain legacy scoring behavior:

```toml
[scoring_rebalanced]
preset = "size-focused"

# This gives you:
# - Size weight: 1.5 (old high value)
# - Complexity weight: 0.5 (reduced)
# - Coverage weight: 0.4 (reduced)
# - Structural weight: 0.6
# - Smell weight: 0.3
```

**When to use**:
- Managing legacy codebases where file size is the primary concern
- Existing workflows depend on size-based prioritization
- Team's priority is file splitting over quality improvements

## Configuration Examples

### Balanced (Default) - Recommended for Most Teams
```toml
[scoring_rebalanced]
preset = "balanced"
```

**Characteristics**:
- Equal weight to complexity and coverage (1.0 each)
- Moderate structural emphasis (0.8)
- Low size weight (0.3)
- Moderate smell detection (0.6)

**Best for**: Standard development prioritizing code quality

### Quality-Focused - For Quality-First Teams
```toml
[scoring_rebalanced]
preset = "quality-focused"
```

**Characteristics**:
- Higher complexity weight (1.2)
- Higher coverage weight (1.1)
- Even lower size weight (0.2)
- Higher smell detection (0.7)

**Best for**: Teams with strong quality culture, new projects

### Test-Coverage - For Coverage Improvement Sprints
```toml
[scoring_rebalanced]
preset = "test-coverage"
```

**Characteristics**:
- Maximum coverage weight (1.3)
- Reduced complexity weight (0.8)
- Minimal size weight (0.2)

**Best for**: Sprints focused on improving test coverage

### Custom Weights - For Advanced Tuning
```toml
[scoring_rebalanced]
complexity_weight = 1.1
coverage_weight = 1.2
structural_weight = 0.7
size_weight = 0.25
smell_weight = 0.65
```

**Best for**: Teams with specific prioritization needs

## Breaking Changes and Compatibility

### Breaking Changes
1. **Score values will change** - All debt items will have different scores
2. **Priority rankings will shift** - Large simple files will rank lower
3. **Severity levels may change** - Some items may move between CRITICAL/HIGH/MEDIUM/LOW

### Backwards Compatibility
- Legacy scoring available via `preset = "size-focused"`
- Configuration files remain compatible
- Output formats unchanged
- API contracts maintained

### CI/CD Integration Changes

**Update validation thresholds**:

Before:
```yaml
# CI validation with legacy scoring
- debtmap validate --max-debt-score 10000
```

After:
```yaml
# CI validation with rebalanced scoring
# Scores are in 0-200 range, adjust thresholds accordingly
- debtmap validate --max-debt-score 5000 --scoring-strategy rebalanced
```

**Update regression checks**:
```yaml
# Don't fail CI on score changes during migration
- debtmap compare --before baseline.json --after current.json --allow-score-changes
```

## Team Communication Template

Use this template to communicate changes to your team:

```markdown
## Technical Debt Scoring Update

We're updating how debtmap prioritizes technical debt to better focus on code quality.

### What's Changing
- **More focus** on complex untested code
- **Less focus** on large but simple files
- **New presets** for different prioritization strategies

### Why This Matters
- Complex untested code carries the highest risk
- Large simple files are lower priority unless they have structural issues
- Better alignment with code quality best practices

### Action Items
1. Review the new top priority items: [link to analysis]
2. Focus testing efforts on complex functions
3. Use `preset = "balanced"` in `.debtmap.toml`

### Questions?
See the migration guide: docs/migration-spec-136.md
```

## Troubleshooting

### Issue: Too many high-priority items after migration

**Solution**: Use quality-focused preset to create more differentiation:
```toml
[scoring_rebalanced]
preset = "quality-focused"
```

### Issue: Scores seem too low compared to legacy

**Solution**: Remember that rebalanced scoring uses 0-200 range. Adjust your mental model:
- Legacy score 100+ → Rebalanced 80+
- Legacy score 50-100 → Rebalanced 40-80
- Legacy score <50 → Rebalanced <40

### Issue: Some important files ranking too low

**Solution**: Use custom weights to emphasize specific factors:
```toml
[scoring_rebalanced]
complexity_weight = 1.3  # Increase if complex files ranking low
structural_weight = 1.0  # Increase if god objects ranking low
```

### Issue: Team disagrees with new priorities

**Solution**: Run side-by-side analysis and discuss:
```bash
debtmap analyze . --scoring-strategy legacy -o legacy.json
debtmap analyze . --scoring-strategy rebalanced -o rebalanced.json
```

Present both results and explain the philosophy behind rebalanced scoring.

## Performance Considerations

The rebalanced scoring algorithm has minimal performance impact:
- **Same O(n) complexity** as legacy scoring
- **5% overhead** for rationale generation
- **No additional I/O** required
- **Parallel processing compatible**

## Success Metrics

Track these metrics to measure migration success:

1. **Test Coverage Improvement**
   - Measure coverage increase on high-complexity functions
   - Target: 10-20% coverage increase in 2-3 months

2. **Complexity Reduction**
   - Track average cyclomatic complexity of top priority items
   - Target: Reduce average complexity by 15-25%

3. **Code Quality Incidents**
   - Monitor bugs in complex untested code
   - Target: 30-50% reduction in quality incidents

4. **Team Velocity**
   - Measure time spent on high-impact vs low-impact refactoring
   - Target: 70% of refactoring time on true quality issues

## Timeline Recommendations

### Week 1: Preparation
- Read this migration guide
- Run comparison analysis
- Present to team

### Week 2-3: Pilot
- Use rebalanced scoring on subset of codebase
- Gather team feedback
- Adjust configuration as needed

### Week 4: Rollout
- Update `.debtmap.toml` to use rebalanced scoring
- Update CI/CD pipelines
- Communicate changes to all team members

### Month 2-3: Optimization
- Monitor success metrics
- Fine-tune weights based on team feedback
- Iterate on configuration

## Additional Resources

- [Scoring Strategies Documentation](../book/src/scoring-strategies.md)
- [Rebalanced Scoring Implementation](../src/priority/scoring/rebalanced.rs)
- [Configuration Reference](../book/src/configuration.md)
- [Spec 136 Full Specification](./spec-136-rebalanced-scoring.md)

## Support

If you encounter issues or have questions:
1. Check this migration guide
2. Review the troubleshooting section
3. Consult the scoring strategies documentation
4. File an issue on GitHub with your configuration and results

## Summary

The rebalanced scoring algorithm represents a significant improvement in how debtmap prioritizes technical debt. By emphasizing code quality over file size, your team can focus on the issues that matter most.

Choose the migration strategy that fits your team's needs, communicate changes clearly, and monitor the success metrics to ensure you're getting the benefits of quality-focused prioritization.
