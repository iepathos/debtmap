# Context Providers

Context providers enhance debtmap's risk analysis by incorporating additional factors beyond complexity and test coverage. They analyze critical execution paths, dependency relationships, and version control history to provide a more comprehensive understanding of technical risk.

## Overview

Context providers implement the `ContextProvider` trait, which gathers risk-relevant information about functions and modules. Each provider analyzes a specific dimension of risk:

- **Critical Path Provider**: Identifies functions on critical execution paths
- **Dependency Provider**: Analyzes call graph relationships and blast radius
- **Git History Provider**: Integrates version control history for change patterns

Context providers help debtmap understand:
- Which code paths are most critical
- How functions depend on each other
- Which code changes most frequently
- Where bugs are likely to occur

This context-aware analysis improves prioritization accuracy and reduces false positives.

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

The provider automatically detects entry points based on function names and file paths. These weights determine the base criticality of execution paths:

| Entry Type | Weight | Detection Pattern | User-Facing |
|------------|--------|-------------------|-------------|
| Main | 10.0 | Function named `main` | Yes |
| API Endpoint | 8.0 | `handle_*`, `*_handler`, `get_*`, `post_*` in `api/`, `handler/`, `route/` paths | Yes |
| CLI Command | 7.0 | `cmd_*`, `command_*`, `*_command` in `cli/`, `command/` paths | Yes |
| Web Handler | 7.0 | Functions with `route`, `handler` in `web/`, `http/` paths | Yes |
| Event Handler | 5.0 | `on_*`, `*_listener`, contains `event` | No |
| Test Entry | 2.0 | `test_*`, in `test/` paths | No |

**Note on API Endpoint detection:** Detection requires BOTH conditions: (1) path contains `api/`, `handler/`, or `route/` AND (2) function starts with `handle_*`, `get_*`, `post_*`, `put_*`, `delete_*` or ends with `*_handler`. This combined matching ensures accurate classification of HTTP endpoint handlers.

**What it detects:**
- Entry points (main functions, CLI handlers, API endpoints)
- Error handling paths
- Data processing pipelines
- Resource initialization

### Path Weighting

Functions on critical paths receive contribution scores based on:

- **Path weight**: The maximum entry point weight leading to the function
- **User-facing flag**: Doubles contribution for user-facing paths

The contribution formula consists of two steps:

```rust
// Step 1: Calculate base contribution (normalized 0-1)
base_contribution = path_weight / max_weight

// Step 2: Apply user-facing multiplier
final_contribution = base_contribution × user_facing_multiplier

// Example: main entry path (weight 10.0, user-facing)
base = 10.0 / 10.0 = 1.0
final = 1.0 × 2.0 = 2.0

// Example: event handler path (weight 5.0, non-user-facing)
base = 5.0 / 10.0 = 0.5
final = 0.5 × 1.0 = 0.5
```

**Impact on scoring:**
- Functions on critical paths get higher priority
- Entry point multiplier: 1.5x
- Business logic multiplier: 1.2x

### Use Cases

- **API prioritization**: Identify critical endpoints that need careful review
- **Refactoring safety**: Avoid breaking user-facing execution paths
- **Test coverage**: Ensure critical paths have adequate test coverage

### Enable

```bash
debtmap analyze . --context-providers critical_path
```

**Configuration:**
```toml
[analysis]
context_providers = ["critical_path"]

# Note: Provider-specific TOML sections below are planned features.
# Currently, providers use hard-coded defaults. Use CLI flags for now.

[context.critical_path]
# Multiplier for entry points (default: 1.5)
entry_point_multiplier = 1.5

# Multiplier for business logic (default: 1.2)
business_logic_multiplier = 1.2
```

## Dependency Provider

The Dependency provider analyzes call graph relationships to identify functions with high architectural impact. It calculates how changes propagate through the dependency graph and determines the blast radius of modifications.

### Dependency Chain Analysis

The provider builds a dependency graph where:

- **Modules** contain functions and have intrinsic risk scores
- **Edges** represent dependencies with coupling strength (0.0-1.0)
- **Risk propagation** flows through dependencies using iterative refinement

**Convergence Parameters:** The risk propagation algorithm uses iterative convergence with a maximum of 10 iterations. Convergence is reached when the maximum risk change between iterations falls below 0.01. This ensures risk stabilizes throughout the dependency graph.

**What it detects:**
- Upstream dependencies (functions this function calls)
- Downstream dependencies (functions that call this function)
- Transitive dependencies through the call graph
- Dependency criticality

### Blast Radius Calculation

The blast radius represents how many modules would be affected by changes to a function. It counts unique modules reachable through transitive dependencies by traversing the dependency graph edges.

| Blast Radius | Contribution | Impact Level |
|--------------|--------------|--------------|
| > 10 modules | 1.5 | Critical dependency affecting many modules |
| > 5 modules | 1.0 | Important dependency with moderate impact |
| > 2 modules | 0.5 | Medium impact |
| ≤ 2 modules | 0.2 | Minimal or isolated component |

### Risk Propagation Formula

Risk propagation uses an iterative convergence algorithm to stabilize risk scores throughout the dependency graph:

```rust
propagated_risk = base_risk × criticality_factor + Σ(caller.risk × 0.3)

where:
  criticality_factor = 1.0 + min(0.5, dependents.len() × 0.1)
  The 0.3 factor dampens risk propagation from callers
```

**Iterative Convergence:** The algorithm runs with a maximum of 10 iterations and converges when the maximum risk change between iterations falls below 0.01. This ensures risk stabilizes throughout the dependency graph without requiring manual tuning.

**Note**: The constants (0.5, 0.1, 0.3) are currently hard-coded based on empirical analysis. Future versions may make these configurable.

**Impact on scoring:**
```
dependency_factor = normalized_to_0_10(upstream + downstream)

Ranges:
- Entry points: 8-10 (critical path)
- Business logic: 6-8 (core functionality)
- Data access: 5-7 (important but stable)
- Utilities: 3-5 (lower priority)
- Test helpers: 1-3 (lowest priority)
```

### Use Cases

- **Architectural refactoring**: Identify high-impact modules to refactor carefully
- **Change impact analysis**: Understand downstream effects of modifications
- **Module decoupling**: Find tightly coupled modules with high blast radius

### Enable

```bash
debtmap analyze . --context-providers dependency
```

**Configuration:**
```toml
[analysis]
context_providers = ["dependency"]

# Note: Provider-specific TOML sections below are planned features.
# Currently, providers use hard-coded defaults. Use CLI flags for now.

[context.dependency]
# Include transitive dependencies (default: true)
include_transitive = true

# Maximum depth for transitive analysis (default: 5)
max_depth = 5
```

## Git History Provider

The Git History provider integrates version control data to detect change-prone code and bug patterns. Files with frequent changes and bug fixes indicate higher maintenance risk.

### Metrics Collected

The provider analyzes Git history to calculate:

- **Change frequency**: Commits per month (recent activity indicator)
- **Bug density**: Ratio of bug fix commits to total commits
- **Age**: Days since first commit (maturity indicator)
- **Author count**: Number of unique contributors (complexity indicator)
- **Total commits**: Total number of commits to the file
- **Last modified**: Timestamp of the most recent commit
- **Stability score**: Weighted combination of churn, bug fixes, and age (0.0-1.0)

**What it analyzes:**
- Commit frequency per file/function
- Bug fix patterns (commits with "fix" in message)
- Code churn (lines added/removed)
- Recent activity

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

### Stability Status Classifications

The provider internally classifies files into stability statuses based on the calculated metrics:

| Status | Criteria | Explanation |
|--------|----------|-------------|
| HighlyUnstable | freq > 5.0 AND bug_density > 0.3 | Extremely high churn combined with many bug fixes |
| FrequentlyChanged | freq > 2.0 | High change rate regardless of bug density |
| BugProne | bug_density > 0.2 | High proportion of bug fix commits |
| MatureStable | age > 365 days | Code older than one year (unless unstable) |
| RelativelyStable | (default) | Moderate activity, typical stability |

These classifications are used internally for contribution calculations and appear in verbose output.

**Impact on scoring:**
- High-churn functions get higher priority
- Recently fixed bugs indicate risk areas
- Stable code (no recent changes) gets lower priority

### Use Cases

- **Find change-prone code**: Identify files that change frequently and need attention
- **Detect bug hotspots**: Locate areas with high bug fix rates
- **Prioritize refactoring**: Target unstable code for improvement
- **Team collaboration patterns**: Files touched by many authors may need better documentation

### Enable

```bash
debtmap analyze . --context-providers git_history
```

**Configuration:**
```toml
[analysis]
context_providers = ["git_history"]

# Note: Provider-specific TOML sections below are planned features.
# Currently, providers use hard-coded defaults. Use CLI flags for now.

[context.git_history]
# Commits to analyze (default: 100)
max_commits = 100

# Time range in days (default: 90)
time_range_days = 90

# Minimum commits to consider "high churn" (default: 10)
high_churn_threshold = 10
```

### Troubleshooting

**Git repository not found**: The provider requires a Git repository. If analysis fails:

```bash
# Verify you're in a git repository
git rev-parse --git-dir

# If not a git repo, initialize one or disable git_history provider
# Option 1: Enable context but exclude git_history
debtmap analyze --context --disable-context git_history

# Option 2: Use only specific providers
debtmap analyze --context-providers critical_path,dependency
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

### Enabling Multiple Providers

Combine providers for comprehensive analysis:

```bash
debtmap analyze . --context-providers critical_path,dependency,git_history
```

Or via config:
```toml
[analysis]
context_providers = ["critical_path", "dependency", "git_history"]
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

## How Context Affects Scoring

### Base Scoring (No Context)

```
Base Score = (Complexity × 0.40) + (Coverage × 0.40) + (Dependency × 0.20)
```

### With Context Providers

```
Context-Adjusted Score = Base Score × Role Multiplier × Churn Multiplier

Role Multiplier (from critical path & dependency analysis):
- Entry points: 1.5x
- Business logic: 1.2x
- Data access: 1.0x
- Infrastructure: 0.8x
- Utilities: 0.5x
- Test code: 0.1x

Churn Multiplier (from git history):
- High churn (10+ commits/month): 1.3x
- Medium churn (5-10 commits/month): 1.1x
- Low churn (1-5 commits/month): 1.0x
- Stable (0 commits/6 months): 0.8x
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

### Example 1: Entry Point vs Utility

**Without context providers:**
```
Function: main() - Entry point
Complexity: 8
Coverage: 50%
Score: 6.0 [MEDIUM]

Function: format_string() - Utility
Complexity: 8
Coverage: 50%
Score: 6.0 [MEDIUM]
```

Both functions have the same score.

**With context providers:**
```
Function: main() - Entry point
Complexity: 8
Coverage: 50%
Base Score: 6.0
Role Multiplier: 1.5x (entry point)
Final Score: 9.0 [CRITICAL]

Function: format_string() - Utility
Complexity: 8
Coverage: 50%
Base Score: 6.0
Role Multiplier: 0.5x (utility)
Final Score: 3.0 [LOW]
```

Entry point is prioritized over utility.

### Example 2: High-Churn Function

**Without git history:**
```
Function: process_payment()
Complexity: 12
Coverage: 60%
Score: 7.5 [HIGH]
```

**With git history:**
```
Function: process_payment()
Complexity: 12
Coverage: 60%
Base Score: 7.5
Churn: 15 commits in last month (bug fixes)
Churn Multiplier: 1.3x
Final Score: 9.75 [CRITICAL]
```

High-churn function is elevated to critical.

### Example 3: Stable Well-Tested Code

**Without context:**
```
Function: legacy_parser()
Complexity: 15
Coverage: 95%
Score: 3.5 [LOW]
```

**With context:**
```
Function: legacy_parser()
Complexity: 15
Coverage: 95%
Base Score: 3.5
Churn: 0 commits in last 2 years
Churn Multiplier: 0.8x
Role: Data access (stable)
Role Multiplier: 1.0x
Final Score: 2.8 [LOW]
```

Stable, well-tested code gets even lower priority.

### Example 4: API Endpoint Prioritization

Analyze a web service to identify critical API endpoints:

```bash
debtmap analyze --context-providers critical_path --format json
```

Functions on API endpoint paths will receive elevated risk scores. Use this to prioritize code review and testing for user-facing functionality.

### Example 5: Finding Change-Prone Code

Identify files with high change frequency and bug fixes:

```bash
debtmap analyze --context-providers git_history --top 20
```

This highlights unstable areas of the codebase that may benefit from refactoring or increased test coverage.

### Example 6: Architectural Impact Analysis

Find high-impact modules with large blast radius:

```bash
debtmap analyze --context-providers dependency --format json | \
  jq '.[] | select(.blast_radius > 10)'
```

Use this to identify architectural choke points that require careful change management.

### Example 7: Comprehensive Risk Assessment

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

## Configuration

> ⚠️ **Configuration Limitation**: Provider-specific TOML configuration sections shown below are planned features not yet implemented. Currently, all provider settings use hard-coded defaults from the implementation. Use CLI flags (`--context`, `--context-providers`, `--disable-context`) to control providers. See the CLI examples throughout this chapter for working configurations.

Configure context providers in `.debtmap.toml`:

```toml
[analysis]
# Enable context-aware analysis (default: false)
enable_context = true

# Specify which providers to use
context_providers = ["critical_path", "dependency", "git_history"]

# Disable specific providers (use CLI flag --disable-context instead)
# disable_context = ["git_history"]  # Not yet implemented in config

[context.git_history]
# Commits to analyze (default: 100) - PLANNED
max_commits = 100

# Time range in days (default: 90) - PLANNED
time_range_days = 90

# Minimum commits to consider "high churn" (default: 10) - PLANNED
high_churn_threshold = 10

[context.critical_path]
# Multiplier for entry points (default: 1.5) - PLANNED
entry_point_multiplier = 1.5

# Multiplier for business logic (default: 1.2) - PLANNED
business_logic_multiplier = 1.2

[context.dependency]
# Include transitive dependencies (default: true) - PLANNED
include_transitive = true

# Maximum depth for transitive analysis (default: 5) - PLANNED
max_depth = 5
```

## Performance Considerations

Context providers add computational overhead to analysis:

**Impact on analysis time:**
- Critical path: +10-15% (fast - call graph traversal)
- Dependency: +20-30% (moderate - iterative risk propagation)
- Git history: +30-50% (slow for large repos - multiple git commands per file)

**Combined overhead:** ~60-80% increase in analysis time

### Optimization Tips

1. **Start minimal**: Use `--context-providers critical_path,dependency` initially
2. **Add git_history selectively**: Enable for critical modules only
3. **Use caching**: The `ContextAggregator` caches results by `file:function` key
4. **Profile with verbose flags**: Use `-vvv` to see provider execution times

### For Large Projects

```bash
# Disable git history for faster analysis
debtmap analyze . --disable-context git_history

# Or disable all context
debtmap analyze . --no-context-aware
```

### For CI/CD

```bash
# Full analysis with context (run nightly)
debtmap analyze . --context-providers critical_path,dependency,git_history

# Fast analysis without context (run on every commit)
debtmap analyze . --no-context-aware
```

### When to Use Each Provider

| Scenario | Recommended Providers |
|----------|----------------------|
| API service refactoring | `critical_path` |
| Legacy codebase analysis | `git_history` |
| Microservice boundaries | `dependency` |
| Pre-release risk review | All providers (`--context`) |
| CI/CD integration | `critical_path,dependency` (faster) |

## Troubleshooting

### Git History Analysis Slow

**Issue:** Analysis takes much longer with git history enabled

**Solutions:**

**Reduce commit history:**
```toml
[context.git_history]
max_commits = 50
time_range_days = 30
```

**Use shallow clone in CI:**
```bash
git clone --depth 50 repo.git
debtmap analyze . --context-providers critical_path,dependency
```

### Incorrect Role Classification

**Issue:** Function classified as wrong role (e.g., utility instead of business logic)

**Possible causes:**
1. Function naming doesn't match patterns
2. Call graph analysis incomplete
3. Function is misplaced in codebase

**Solutions:**

**Check with verbose output:**
```bash
debtmap analyze . -vv | grep "Role classification"
```

**Manually verify call graph:**
```bash
debtmap analyze . --show-call-graph
```

### Context Providers Not Available

**Issue:** `--context-providers` flag not recognized

**Solution:** Ensure you're using a recent version:
```bash
debtmap --version
# Should be 0.2.0 or later
```

Update debtmap:
```bash
cargo install debtmap --force
```

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

The `ContextAggregator` caches context by `file:function` key to avoid redundant analysis during a single run.

**Cache Lifetime:** The cache is in-memory per `ContextAggregator` instance and is cleared when a new instance is created or when analyzing a different codebase. This enables efficient re-analysis within the same run without requiring external cache management:

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

## Best Practices

1. **Use all providers for comprehensive analysis** - Especially for production code
2. **Disable git history in CI** - Use shallow clones or disable for speed
3. **Verify role classifications** - Use `-vv` to see how functions are classified
4. **Tune multipliers for your project** - Adjust in config based on architecture
5. **Combine with coverage data** - Context providers enhance coverage-based risk analysis

## Summary

Context providers transform debtmap from a static complexity analyzer into a comprehensive risk assessment tool. By combining:

- **Critical path analysis** for user impact
- **Dependency analysis** for architectural risk
- **Git history analysis** for maintenance patterns

You gain actionable insights for prioritizing technical debt and refactoring efforts. Start with `--context` to enable all providers, then refine based on your project's needs.

## See Also

- [Analysis Guide](analysis-guide.md) - Core analysis concepts
- [Risk Assessment](analysis-guide.md#risk-assessment) - Risk scoring methodology
- [Configuration](configuration.md) - Complete configuration reference
- [Parallel Processing](parallel-processing.md) - Performance optimization
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
