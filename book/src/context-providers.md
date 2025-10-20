# Context Providers

Context providers enhance debtmap's analysis by incorporating additional context beyond static code analysis. This chapter covers critical path detection, dependency analysis, and git history integration.

## Overview

Context providers help debtmap understand:
- Which code paths are most critical
- How functions depend on each other
- Which code changes most frequently
- Where bugs are likely to occur

This context-aware analysis improves prioritization accuracy and reduces false positives.

## Available Providers

### Critical Path Detection

Identifies functions on critical execution paths that directly impact user-facing functionality or system stability.

**What it detects:**
- Entry points (main functions, CLI handlers, API endpoints)
- Error handling paths
- Data processing pipelines
- Resource initialization

**Impact on scoring:**
- Functions on critical paths get higher priority
- Entry point multiplier: 1.5x
- Business logic multiplier: 1.2x

**Enable:**
```bash
debtmap analyze . --context-providers critical_path
```

**Configuration:**
```toml
[analysis]
context_providers = ["critical_path"]
```

### Dependency Analysis

Analyzes function call graphs to identify high-impact functions called by many other functions.

**What it detects:**
- Upstream dependencies (functions this function calls)
- Downstream dependencies (functions that call this function)
- Transitive dependencies through the call graph
- Dependency criticality

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

**Enable:**
```bash
debtmap analyze . --context-providers dependency
```

**Configuration:**
```toml
[analysis]
context_providers = ["dependency"]
```

### Git History Integration

Uses version control history to identify frequently changing code, which is more likely to contain bugs.

**What it analyzes:**
- Commit frequency per file/function
- Bug fix patterns (commits with "fix" in message)
- Code churn (lines added/removed)
- Recent activity

**Impact on scoring:**
- High-churn functions get higher priority
- Recently fixed bugs indicate risk areas
- Stable code (no recent changes) gets lower priority

**Enable:**
```bash
debtmap analyze . --context-providers git_history
```

**Configuration:**
```toml
[analysis]
context_providers = ["git_history"]
```

## Enabling Multiple Providers

Combine providers for comprehensive analysis:

```bash
debtmap analyze . --context-providers critical_path,dependency,git_history
```

Or via config:
```toml
[analysis]
context_providers = ["critical_path", "dependency", "git_history"]
```

## Disabling Providers

Disable specific providers:

```bash
debtmap analyze . --disable-context critical_path
```

Disable all context-aware analysis:

```bash
debtmap analyze . --no-context-aware
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

## Examples

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

## Configuration

Configure context providers in `.debtmap.toml`:

```toml
[analysis]
# Enable context-aware analysis (default: true)
enable_context = true

# Specify which providers to use
context_providers = ["critical_path", "dependency", "git_history"]

# Disable specific providers
# disable_context = ["git_history"]

[context.git_history]
# Commits to analyze (default: 100)
max_commits = 100

# Time range in days (default: 90)
time_range_days = 90

# Minimum commits to consider "high churn" (default: 10)
high_churn_threshold = 10

[context.critical_path]
# Multiplier for entry points (default: 1.5)
entry_point_multiplier = 1.5

# Multiplier for business logic (default: 1.2)
business_logic_multiplier = 1.2

[context.dependency]
# Include transitive dependencies (default: true)
include_transitive = true

# Maximum depth for transitive analysis (default: 5)
max_depth = 5
```

## Performance Considerations

Context providers add overhead to analysis:

**Impact on analysis time:**
- Critical path: +10-15%
- Dependency: +20-30%
- Git history: +30-50%

**Combined overhead:** ~60-80% increase in analysis time

**For large projects:**
```bash
# Disable git history for faster analysis
debtmap analyze . --disable-context git_history

# Or disable all context
debtmap analyze . --no-context-aware
```

**For CI/CD:**
```bash
# Full analysis with context (run nightly)
debtmap analyze . --context-providers critical_path,dependency,git_history

# Fast analysis without context (run on every commit)
debtmap analyze . --no-context-aware
```

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

## Best Practices

1. **Use all providers for comprehensive analysis** - Especially for production code
2. **Disable git history in CI** - Use shallow clones or disable for speed
3. **Verify role classifications** - Use `-vv` to see how functions are classified
4. **Tune multipliers for your project** - Adjust in config based on architecture
5. **Combine with coverage data** - Context providers enhance coverage-based risk analysis

## See Also

- [Analysis Guide](analysis-guide.md) - Core analysis concepts
- [Risk Assessment](analysis-guide.md#risk-assessment) - Risk scoring methodology
- [Configuration](configuration.md) - Complete configuration reference
- [Parallel Processing](parallel-processing.md) - Performance optimization
