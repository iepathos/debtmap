# Context Provider Issues

This section covers troubleshooting for debtmap's context providers: `critical_path`, `dependency`, and `git_history`. Context providers gather additional risk-relevant information to enhance the base complexity analysis.

## Overview

Context providers implement the `ContextProvider` trait (defined in `src/risk/context/mod.rs:13-25`) and gather context for analysis targets. Each provider:

- Has a name identifying the provider
- Gathers context for a given analysis target
- Has a weight for its contribution to overall risk
- Can explain its context contribution

The three available providers are:

| Provider | Purpose | Source |
|----------|---------|--------|
| `critical_path` | Analyzes call graph paths from entry points | `src/risk/context/critical_path.rs` |
| `dependency` | Calculates risk propagation through dependencies | `src/risk/context/dependency.rs` |
| `git_history` | Provides change frequency and bug density metrics | `src/risk/context/git_history.rs` |

## Enabling Context Analysis

```bash
# Enable with default providers
debtmap analyze . --context

# Or use the explicit flag
debtmap analyze . --enable-context

# Specify specific providers
debtmap analyze . --context --context-providers critical_path,dependency,git_history
```

**Source**: CLI arguments defined in `src/cli/args.rs:111-125`

## Disabling Specific Providers

```bash
# Disable git_history only
debtmap analyze . --context --disable-context git_history

# Disable multiple providers
debtmap analyze . --context --disable-context git_history,dependency

# Disable context-aware filtering (keeps providers but disables filtering)
debtmap analyze . --no-context-aware
```

**Source**: CLI arguments defined in `src/cli/args.rs:123-125` and `src/cli/args.rs:202-204`

## Git History Provider Issues

The `GitHistoryProvider` (defined in `src/risk/context/git_history.rs:27-33`) analyzes git commit history to provide:

- Change frequency
- Bug fix count
- Author count
- File age in days
- Stability score

### Problem: "Git history error" or "Not a git repository"

**Causes**:
- Not in a git repository
- No git history for files
- Git not installed or accessible

**Solutions**:

```bash
# Verify you're in a git repository
git status

# If not a git repository, disable git_history provider
debtmap analyze . --context --disable-context git_history

# Initialize git repo if needed
git init
```

**Technical detail**: The provider verifies the git repository by running `git rev-parse --git-dir` (see `src/risk/context/git_history.rs:38-46`). If this fails, the provider returns an error.

### Problem: Git History Analysis is Slow

The git history provider fetches all git data upfront using batched operations.

**Solutions**:

Reduce commit history depth in your CI pipeline:
```bash
# Use shallow clone in CI
git clone --depth 50 repo.git
debtmap analyze . --context-providers critical_path,dependency
```

Disable git_history for faster analysis:
```bash
debtmap analyze . --context --disable-context git_history
```

**Technical detail**: The provider uses `batched::BatchedGitHistory` (see `src/risk/context/git_history.rs:49-62`) to load git data in bulk rather than per-file queries.

## Dependency Provider Issues

The `DependencyRiskProvider` (defined in `src/risk/context/dependency.rs:181-191`) calculates:

- Propagated risk through the dependency graph
- Blast radius (how many modules would be affected by changes)
- Coupling strength between modules

### Problem: "Dependency error" or incomplete dependency graph

**Causes**:
- Complex import structures
- Circular dependencies
- Unsupported dependency patterns

**Solutions**:

```bash
# Disable dependency provider
debtmap analyze . --context --disable-context dependency

# Try with verbosity to see details
debtmap analyze . --context -vvv

# Use without context
debtmap analyze .
```

### Problem: Dependency Analysis Errors

The dependency calculator uses iterative risk propagation with a maximum of 10 iterations and a convergence threshold of 0.01 (see `src/risk/context/dependency.rs:93-118`).

If analysis errors occur:

```bash
# Check for circular dependencies in your codebase
debtmap analyze . --context -vvv 2>&1 | grep -i "dependency\|circular"

# Disable dependency provider if issues persist
debtmap analyze . --context --disable-context dependency
```

## Critical Path Provider Issues

The `CriticalPathProvider` (defined in `src/risk/context/critical_path.rs`) analyzes:

- Entry points (main, API endpoints, CLI commands, event handlers)
- Call graph paths
- User-facing code paths

### Problem: Critical path analysis fails or produces unexpected results

**Causes**:
- Invalid call graph
- Missing function definitions
- Complex control flow

**Solutions**:

```bash
# Disable critical_path provider
debtmap analyze . --context --disable-context critical_path

# Try with semantic analysis disabled
debtmap analyze . --context --semantic-off

# Debug with verbosity to see entry point detection
debtmap analyze . --context --context-providers critical_path -vvv
```

### Problem: Incorrect Entry Point Classification

Entry points are classified by function name and file path patterns (see `src/risk/context/critical_path.rs:82-98`). The following entry types are detected:

| Entry Type | Detection Pattern |
|------------|-------------------|
| `Main` | Functions named `main` |
| `CliCommand` | Functions in CLI-related paths |
| `ApiEndpoint` | Functions matching API patterns |
| `WebHandler` | Functions in web handler paths |
| `EventHandler` | Functions matching event handler patterns |
| `TestEntry` | Functions in test files |

**Solutions**:

```bash
# Check with verbose output to see classification
debtmap analyze . -vv | grep "entry point\|Entry"

# Verify call graph is being built
debtmap analyze . --show-call-graph
```

## Context Impact on Scoring

Context providers add additional risk factors to the base complexity score. The contribution is calculated in `ContextualRisk::new()` (see `src/risk/context/mod.rs:215-238`):

- Context contribution is capped at 2.0 to prevent excessive score amplification
- This means a maximum 3x multiplier on base risk
- Formula: `contextual_risk = base_risk * (1.0 + context_contribution)`

```bash
# See context contribution to scores
debtmap analyze . --context -v

# Compare with and without context
debtmap analyze . --format json --output baseline.json
debtmap analyze . --context --format json --output with_context.json
debtmap compare --before baseline.json --after with_context.json
```

### Problem: Context Providers Not Affecting Scores

**Solution**: Ensure providers are enabled with `--context` or `--context-providers`:

```bash
# Wrong: context flag missing
debtmap analyze .

# Correct: context enabled
debtmap analyze . --context
```

## Performance Considerations

Context analysis adds processing overhead. The `ContextAggregator` (defined in `src/risk/context/mod.rs:98-155`) uses:

- Lock-free `DashMap` for caching (thread-safe concurrent access)
- `Arc` for cheap cloning across threads
- Cache key: `{file_path}:{function_name}`

**Performance comparison**:

```bash
# Fastest: no context
debtmap analyze .

# Slowest: all context providers
debtmap analyze . --context --context-providers critical_path,dependency,git_history

# Medium: selective providers (skip git_history)
debtmap analyze . --context --context-providers critical_path,dependency
```

## Debugging Context Providers

```bash
# See detailed context provider output
debtmap analyze . --context -vvv

# Check which providers are active
debtmap analyze . --context -v 2>&1 | grep -i "context provider\|provider"

# See provider execution times
debtmap analyze . --context -vvv 2>&1 | grep -i "time\|duration\|elapsed"
```

### Understanding Provider Output

When running with verbose mode, you'll see context details including:

**CriticalPath context** (from `src/risk/context/mod.rs:48-52`):
- `entry_points`: List of reachable entry points
- `path_weight`: Weight of the critical path
- `is_user_facing`: Whether the function is user-facing

**Dependency context** (from `src/risk/context/mod.rs:53-58`):
- `depth`: Dependency chain depth
- `propagated_risk`: Risk propagated from dependencies
- `dependents`: List of dependent modules
- `blast_radius`: Number of affected modules

**Historical context** (from `src/risk/context/mod.rs:59-64`):
- `change_frequency`: How often the file changes
- `bug_density`: Bug fix density
- `age_days`: File age in days
- `author_count`: Number of contributors

## Common Issues Summary

| Issue | Quick Fix |
|-------|-----------|
| Git history error | `--disable-context git_history` |
| Dependency analysis errors | `--disable-context dependency` |
| Critical path fails | `--disable-context critical_path` |
| Slow analysis | `--disable-context git_history` |
| Providers not affecting scores | Add `--context` flag |
| Need version 0.2.0+ | `cargo install debtmap --force` |

## See Also

- [Quick Fixes](quick-fixes.md) - Common problems with immediate solutions
- [Debug Mode](debug-mode.md) - General debugging techniques
- [Advanced Analysis Issues](advanced-analysis.md) - Multi-pass and semantic analysis
- [Context Providers](../context-providers.md) - Full context providers guide
