# Scoring Strategies

Debtmap uses sophisticated scoring strategies to prioritize technical debt based on multiple factors including complexity, test coverage, and functional role. This section explains the different scoring approaches available.

## Overview

Scoring strategies determine how debtmap calculates priority scores for functions and files. The goal is to identify the most impactful technical debt that provides the best return on investment for refactoring efforts.

## Strategy Types

### File-Level vs Function-Level

Debtmap operates at two granularity levels:

- **[Function-Level Scoring](function-level.md)** - Identifies specific functions that need attention, considering complexity, coverage, and role
- **[File-Level Scoring](file-level.md)** - Aggregates function metrics to identify module-level architectural issues

### Role-Based Adjustments

Not all code deserves equal scrutiny. A complex orchestrator function has different testing requirements than pure business logic:

- **[Role-Based Adjustments](role-based.md)** - Automatic multipliers based on detected function roles (pure logic, I/O wrappers, entry points, etc.)

### Scoring Algorithms

Different algorithms for calculating final scores:

- **[Rebalanced Scoring](rebalanced.md)** - The default balanced approach that weighs coverage, complexity, and dependencies
- **[Exponential Scaling](exponential-scaling.md)** - Aggressive scaling for codebases where highest-priority items need strong emphasis
- **[Data Flow Scoring](data-flow.md)** - Scoring based on how data flows through functions (sources, sinks, transforms)

## Choosing a Strategy

| Strategy | Best For | Characteristics |
|----------|----------|-----------------|
| Rebalanced (default) | Most projects | Balanced, fair prioritization |
| Exponential | Large legacy codebases | Emphasizes worst offenders |
| Data Flow | Pipeline-heavy code | Prioritizes data transformation logic |

## Configuration

Scoring can be tuned via the `[scoring]` section in `.debtmap.toml`:

```toml
[scoring]
coverage = 0.50    # Weight for test coverage
complexity = 0.35  # Weight for complexity metrics
dependency = 0.15  # Weight for dependency analysis
```

See [Scoring Configuration](../configuration/scoring.md) for full details on available options.
