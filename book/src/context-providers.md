# Context Providers

Context providers enhance debtmap's risk analysis by incorporating additional factors beyond complexity and test coverage. They analyze critical execution paths, dependency relationships, and version control history to provide a more comprehensive understanding of technical risk.

## Overview

Context providers implement the `ContextProvider` trait, which gathers risk-relevant information about functions and modules. Each provider analyzes a specific dimension of risk:

- **Critical Path Provider**: Identifies functions on critical execution paths
- **Dependency Provider**: Analyzes call graph relationships and blast radius
- **Git History Provider**: Integrates version control history for change patterns

The `ContextAggregator` combines context from multiple enabled providers and adjusts risk scores using the formula:

```
contextual_risk = base_risk × (1.0 + context_contribution)
```

Where `context_contribution` is the weighted sum of all provider contributions:

```
context_contribution = Σ(provider.contribution × provider.weight)
```

## Critical Path Provider

The Critical Path provider identifies functions that lie on critical execution paths through your application. Functions on these paths have elevated risk because failures directly impact user-facing functionality.

### Entry Point Detection

The provider automatically detects entry points based on function names and file paths:

| Entry Type | Weight | Detection Pattern | User-Facing |
|------------|--------|-------------------|-------------|
| Main | 10.0 | Function named `main` | Yes |
| API Endpoint | 8.0 | `handle_*`, `*_handler`, `get_*`, `post_*` in `api/`, `handler/`, `route/` paths | Yes |
| CLI Command | 7.0 | `cmd_*`, `command_*`, `*_command` in `cli/`, `command/` paths | Yes |
| Web Handler | 7.0 | Functions with `route`, `handler` in `web/`, `http/` paths | Yes |
| Event Handler | 5.0 | `on_*`, `*_listener`, contains `event` | No |
| Test Entry | 2.0 | `test_*`, in `test/` paths | No |

### Path Weighting

Functions on critical paths receive contribution scores based on:

- **Path weight**: The maximum entry point weight leading to the function
- **User-facing flag**: Doubles contribution for user-facing paths

```rust
// Example: Function on main entry path (weight 10.0, user-facing)
contribution = (10.0 / 10.0) × 2.0 = 2.0

// Example: Function on event handler path (weight 5.0, non-user-facing)
contribution = (5.0 / 10.0) × 1.0 = 0.5
```

### Use Cases

- **API prioritization**: Identify critical endpoints that need careful review
- **Refactoring safety**: Avoid breaking user-facing execution paths
- **Test coverage**: Ensure critical paths have adequate test coverage

## Dependency Provider

The Dependency provider analyzes call graph relationships to identify functions with high architectural impact. It calculates how changes propagate through the dependency graph and determines the blast radius of modifications.

### Dependency Chain Analysis

The provider builds a dependency graph where:

- **Modules** contain functions and have intrinsic risk scores
- **Edges** represent dependencies with coupling strength (0.0-1.0)
- **Risk propagation** flows through dependencies using iterative refinement

### Blast Radius Calculation

The blast radius represents how many modules would be affected by changes to a function:

| Blast Radius | Contribution | Impact Level |
|--------------|--------------|--------------|
| > 10 modules | 1.5 | Critical dependency affecting many modules |
| 6-10 modules | 1.0 | Important dependency with moderate impact |
| 3-5 modules | 0.5 | Limited dependency impact |
| ≤ 2 modules | 0.2 | Minimal or isolated component |

### Risk Propagation Formula

```rust
propagated_risk = base_risk × criticality_factor + dependency_risk

where:
  criticality_factor = 1.0 + min(0.5, dependents.len() × 0.1)
  dependency_risk = Σ(dependency.risk × coupling_strength × 0.3)
```

### Use Cases

- **Architectural refactoring**: Identify high-impact modules to refactor carefully
- **Change impact analysis**: Understand downstream effects of modifications
- **Module decoupling**: Find tightly coupled modules with high blast radius

## Git History Provider

The Git History provider integrates version control data to detect change-prone code and bug patterns. Files with frequent changes and bug fixes indicate higher maintenance risk.

### Metrics Collected

The provider analyzes Git history to calculate:

- **Change frequency**: Commits per month (recent activity indicator)
- **Bug density**: Ratio of bug fix commits to total commits
- **Age**: Days since first commit (maturity indicator)
- **Author count**: Number of unique contributors (complexity indicator)

### Risk Classification

| Category | Conditions | Contribution | Explanation |
|----------|------------|--------------|-------------|
| Very unstable | freq > 5.0 AND bug_density > 0.3 | 2.0 | High churn with many bug fixes |
| Moderately unstable | freq > 2.0 OR bug_density > 0.2 | 1.0 | Frequent changes or bug-prone |
| Slightly unstable | freq > 1.0 OR bug_density > 0.1 | 0.5 | Some instability |
| Stable | freq ≤ 1.0 AND bug_density ≤ 0.1 | 0.1 | Low change rate, few bugs |

### Bug Fix Detection

The provider identifies bug fixes by searching commit messages for patterns:

```bash
git log --grep=fix --grep=bug --grep=Fix --grep=Bug -- <file>
```

### Stability Score

Stability is calculated using weighted factors:

```rust
stability = (churn_factor × 0.4) + (bug_factor × 0.4) + (age_factor × 0.2)

where:
  churn_factor = 1.0 / (1.0 + monthly_churn)
  bug_factor = 1.0 - (bug_fixes / total_commits)
  age_factor = min(1.0, age_days / 365.0)
```

### Use Cases

- **Find change-prone code**: Identify files that change frequently and need attention
- **Detect bug hotspots**: Locate areas with high bug fix rates
- **Prioritize refactoring**: Target unstable code for improvement
- **Team collaboration patterns**: Files touched by many authors may need better documentation

### Troubleshooting

**Git repository not found**: The provider requires a Git repository. If analysis fails:

```bash
# Verify you're in a git repository
git rev-parse --git-dir

# If not a git repo, initialize one or disable git_history provider
debtmap analyze --disable-context git_history
```

**Performance issues**: Git history analysis can be slow for large repositories:

```bash
# Use only lightweight providers
debtmap analyze --context-providers critical_path,dependency
```

## Enabling Context Providers

Context-aware analysis is disabled by default. Enable it using CLI flags:

### Enable All Providers

```bash
# Enable all available context providers
debtmap analyze --context
# or
debtmap analyze --enable-context
```

### Enable Specific Providers

```bash
# Enable only critical_path and dependency
debtmap analyze --context-providers critical_path,dependency

# Enable only git_history
debtmap analyze --context-providers git_history

# Enable all three explicitly
debtmap analyze --context-providers critical_path,dependency,git_history
```

### Disable Specific Providers

```bash
# Enable context but disable git_history (useful for non-git repos)
debtmap analyze --context --disable-context git_history

# Enable context but disable dependency analysis
debtmap analyze --context --disable-context dependency
```

## Provider Weights

Each provider has a weight that determines its influence on the final risk score:

| Provider | Weight | Rationale |
|----------|--------|-----------|
| critical_path | 1.5 | Critical paths have high impact on users |
| dependency_risk | 1.2 | Architectural dependencies affect many modules |
| git_history | 1.0 | Historical patterns indicate future risk |

The total context contribution is calculated as:

```rust
total_contribution = Σ(contribution_i × weight_i)

Example with all providers:
  critical_path: 2.0 × 1.5 = 3.0
  dependency:    1.0 × 1.2 = 1.2
  git_history:   0.5 × 1.0 = 0.5
  ────────────────────────────
  total_contribution = 4.7

contextual_risk = base_risk × (1.0 + 4.7) = base_risk × 5.7
```

## Context Details Structure

When using `--format json`, context information is included in the output. The `ContextDetails` enum contains provider-specific data:

### CriticalPath

```json
{
  "provider": "critical_path",
  "weight": 1.5,
  "contribution": 2.0,
  "details": {
    "CriticalPath": {
      "entry_points": ["main (Main)", "handle_request (ApiEndpoint)"],
      "path_weight": 10.0,
      "is_user_facing": true
    }
  }
}
```

### DependencyChain

```json
{
  "provider": "dependency_risk",
  "weight": 1.2,
  "contribution": 1.5,
  "details": {
    "DependencyChain": {
      "depth": 3,
      "propagated_risk": 8.5,
      "dependents": ["module_a", "module_b", "module_c"],
      "blast_radius": 12
    }
  }
}
```

### Historical

```json
{
  "provider": "git_history",
  "weight": 1.0,
  "contribution": 1.0,
  "details": {
    "Historical": {
      "change_frequency": 3.5,
      "bug_density": 0.25,
      "age_days": 180,
      "author_count": 5
    }
  }
}
```

## Practical Examples

### Example 1: API Endpoint Prioritization

Analyze a web service to identify critical API endpoints:

```bash
debtmap analyze --context-providers critical_path --format json
```

Functions on API endpoint paths will receive elevated risk scores. Use this to prioritize code review and testing for user-facing functionality.

### Example 2: Finding Change-Prone Code

Identify files with high change frequency and bug fixes:

```bash
debtmap analyze --context-providers git_history --top 20
```

This highlights unstable areas of the codebase that may benefit from refactoring or increased test coverage.

### Example 3: Architectural Impact Analysis

Find high-impact modules with large blast radius:

```bash
debtmap analyze --context-providers dependency --format json | \
  jq '.[] | select(.blast_radius > 10)'
```

Use this to identify architectural choke points that require careful change management.

### Example 4: Comprehensive Risk Assessment

Combine all providers for holistic risk analysis:

```bash
debtmap analyze --context -v
```

The verbose output shows how each provider contributes to the final risk score:

```
function: process_payment
  base_risk: 5.0
  critical_path: +3.0 (on main path, user-facing)
  dependency: +1.2 (12 dependent modules)
  git_history: +1.0 (3.5 changes/month, 0.25 bug density)
  ──────────────────
  contextual_risk: 26.0
```

## Performance Considerations

Context providers add computational overhead to analysis:

- **critical_path**: Fast (call graph traversal)
- **dependency**: Moderate (iterative risk propagation)
- **git_history**: Slow for large repos (multiple git commands per file)

### Optimization Tips

1. **Start minimal**: Use `--context-providers critical_path,dependency` initially
2. **Add git_history selectively**: Enable for critical modules only
3. **Use caching**: The `ContextAggregator` caches results by `file:function` key
4. **Profile with verbose flags**: Use `-vvv` to see provider execution times

### When to Use Each Provider

| Scenario | Recommended Providers |
|----------|----------------------|
| API service refactoring | `critical_path` |
| Legacy codebase analysis | `git_history` |
| Microservice boundaries | `dependency` |
| Pre-release risk review | All providers (`--context`) |
| CI/CD integration | `critical_path,dependency` (faster) |

## Future Enhancements

### Business Context Provider (Planned)

A Business context provider is defined but not yet implemented. It will support:

```rust
Business {
    priority: Priority,      // Critical, High, Medium, Low
    impact: Impact,          // Revenue, UserExperience, Security, Compliance
    annotations: Vec<String> // Custom business metadata
}
```

This will allow manual prioritization based on business requirements through code annotations or configuration files.

## Troubleshooting

### Common Issues

**Issue**: Context providers not affecting scores

**Solution**: Ensure providers are enabled with `--context` or `--context-providers`

```bash
# Wrong: context flag missing
debtmap analyze

# Correct: context enabled
debtmap analyze --context
```

---

**Issue**: Git history provider fails with "Not a git repository"

**Solution**: Disable git_history if not using version control

```bash
debtmap analyze --context --disable-context git_history
```

---

**Issue**: Dependency analysis errors

**Solution**: Check for circular dependencies or disable dependency provider

```bash
debtmap analyze --context --disable-context dependency
```

---

**Issue**: Slow analysis with all providers

**Solution**: Use selective providers or increase verbosity to identify bottlenecks

```bash
# Faster: skip git_history
debtmap analyze --context-providers critical_path,dependency

# Debug: see provider execution times
debtmap analyze --context -vvv
```

---

For more troubleshooting guidance, see the [Troubleshooting](troubleshooting.md) chapter.

## Advanced Usage

### Interpreting Context Contribution

Enable verbose output to see detailed context contributions:

```bash
debtmap analyze --context -v
```

Each function shows:
- Base risk score from complexity/coverage
- Individual provider contributions
- Total contextual risk score
- Provider-specific explanations

### Architecture Exploration

The `ContextAggregator` caches context by `file:function` key. This enables efficient re-analysis:

```rust
let mut aggregator = ContextAggregator::new()
    .with_provider(Box::new(CriticalPathProvider::new(analyzer)))
    .with_provider(Box::new(DependencyRiskProvider::new(graph)))
    .with_provider(Box::new(GitHistoryProvider::new(repo_root)?));

let context = aggregator.analyze(&target);
let contribution = context.total_contribution();
```

### Custom Provider Implementation

Advanced users can implement custom context providers by implementing the `ContextProvider` trait:

```rust
pub trait ContextProvider: Send + Sync {
    fn name(&self) -> &str;
    fn gather(&self, target: &AnalysisTarget) -> Result<Context>;
    fn weight(&self) -> f64;
    fn explain(&self, context: &Context) -> String;
}
```

See [src/risk/context/mod.rs](https://github.com/your-repo/debtmap/blob/main/src/risk/context/mod.rs) for implementation examples.

## Summary

Context providers transform debtmap from a static complexity analyzer into a comprehensive risk assessment tool. By combining:

- **Critical path analysis** for user impact
- **Dependency analysis** for architectural risk
- **Git history analysis** for maintenance patterns

You gain actionable insights for prioritizing technical debt and refactoring efforts. Start with `--context` to enable all providers, then refine based on your project's needs.
