# Multi-Pass Analysis

Multi-pass analysis performs multiple iterations over the codebase to build up comprehensive complexity attribution data. This helps understand where complexity originates and how it propagates through the call graph.

## Overview

Multi-pass analysis enables:
- Complexity attribution tracking
- Understanding complexity sources
- Identifying complexity propagation patterns
- Detailed diagnostic information

## Basic Usage

```bash
# Enable multi-pass analysis
debtmap analyze . --multi-pass

# Show attribution details
debtmap analyze . --multi-pass --show-attribution
```

## How It Works

### Pass 1: Initial Analysis
- Parse all files
- Calculate basic metrics (cyclomatic, cognitive complexity)
- Build call graph structure

### Pass 2: Attribution
- Trace complexity through call graph
- Attribute complexity to call sites
- Calculate compound complexity scores

### Pass 3: Aggregation
- Combine results across modules
- Generate attribution reports
- Identify complexity hotspots

## Attribution Display

When `--show-attribution` is enabled, output includes:
- Original complexity score
- Attributed complexity from callees
- Breakdown by call target
- Propagation paths

### Example Output

```
Function: process_data
Location: src/analyzer.rs:142
Cyclomatic Complexity: 8
Attributed Complexity: 23
  ├─ parse_input: +7
  ├─ validate: +4
  └─ transform: +4
```

## Configuration

### Enable Multi-Pass

```bash
debtmap analyze . --multi-pass
```

### Show Attribution

```bash
debtmap analyze . --multi-pass --show-attribution
```

### Combine with Other Options

```bash
# Multi-pass with coverage analysis
debtmap analyze . --multi-pass --coverage-file coverage.lcov

# Multi-pass with context providers
debtmap analyze . --multi-pass --context
```

## Performance Considerations

Multi-pass analysis requires additional processing:
- **Typical overhead**: 20-40% longer analysis time
- **Memory usage**: Higher due to attribution data storage
- **Benefits**: More accurate complexity understanding

### Optimization Tips

```bash
# Limit to specific paths for faster analysis
debtmap analyze src/core/ --multi-pass

# Use parallel processing
debtmap analyze . --multi-pass --jobs 8

# Cache results for repeated runs
debtmap analyze . --multi-pass --use-cache
```

## Use Cases

### Understanding Complex Functions

Identify which callees contribute most to perceived complexity:

```bash
debtmap analyze . --multi-pass --show-attribution \
  --filter-categories Complexity --top 10
```

### Refactoring Planning

Find functions where extracted callees would reduce complexity:

```bash
debtmap analyze . --multi-pass --show-attribution \
  --format markdown --output refactoring-targets.md
```

### Code Review

Include attribution in PR reviews to understand impact:

```bash
debtmap analyze . --multi-pass --show-attribution \
  --format json --output pr-analysis.json
```

## Best Practices

**When to use:**
- Deep analysis of complex codebases
- Understanding complexity sources
- Refactoring large functions
- Code review and architecture decisions

**When to skip:**
- Quick analysis for CI/CD
- Large codebases with time constraints
- Initial project assessment

**Combine with:**
- Call graph debugging for resolution issues
- Coverage integration for risk analysis
- Context providers for comprehensive insights

## Troubleshooting

### Slow Analysis

**Issue:** Multi-pass takes too long

**Solution:**
- Limit analysis to specific paths
- Increase parallelism with `--jobs`
- Use caching with `--use-cache`
- Skip multi-pass for CI, use for detailed analysis

### Missing Attribution

**Issue:** Attribution data incomplete

**Solution:**
- Enable call graph debugging: `--debug-call-graph`
- Check for resolution failures
- Verify all dependencies are analyzed
- Review external function calls

## See Also

- [Analysis Guide](analysis-guide.md) - Understanding complexity metrics
- [CLI Reference](cli-reference.md) - Multi-pass options
- [Performance Tips](troubleshooting.md#performance) - Optimization
