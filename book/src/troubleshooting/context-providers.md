# Context Provider Issues

Diagnose and fix issues with context providers (critical_path, dependency, git_history).

## Enable Context Analysis

```bash
# Enable with default providers
debtmap analyze . --context

# Specify specific providers
debtmap analyze . --context --context-providers critical_path,dependency,git_history
```

## Disable Specific Providers

```bash
# Disable git_history only
debtmap analyze . --context --disable-context git_history

# Disable multiple providers
debtmap analyze . --context --disable-context git_history,dependency

# Disable context-aware filtering entirely
debtmap analyze . --no-context-aware
```

## Git History Provider Issues

**Problem**: "Git history error" when running analysis

**Causes**:
- Not in a git repository
- No git history for files
- Git not installed or accessible

**Solutions**:
```bash
# Verify in git repository
git status

# Disable git_history provider
debtmap analyze . --context --disable-context git_history

# Initialize git repo if needed
git init
```

## Dependency Provider Issues

**Problem**: "Dependency error" or incomplete dependency graph

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

## Critical Path Provider Issues

**Problem**: Critical path analysis fails or produces unexpected results

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

# Debug with verbosity
debtmap analyze . --context --context-providers critical_path -vvv
```

## Context Impact on Scoring

Context providers add additional risk factors to scoring:

```bash
# See context contribution to scores
debtmap analyze . --context -v

# Compare with and without context
debtmap analyze . --format json --output baseline.json
debtmap analyze . --context --format json --output with_context.json
debtmap compare --before baseline.json --after with_context.json
```

## Debug Context Providers

```bash
# See detailed context provider output
debtmap analyze . --context -vvv

# Check which providers are active
debtmap analyze . --context -v 2>&1 | grep "context provider"
```

## See Also

- [Quick Fixes](quick-fixes.md) - Common problems with immediate solutions
- [Advanced Analysis Issues](advanced-analysis.md) - Call graph and pattern detection issues
