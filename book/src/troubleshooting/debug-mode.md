# Debug Mode

Use verbosity flags to diagnose issues and understand analysis behavior.

## Verbosity Levels

```bash
# Level 1: Show main score factors
debtmap analyze . -v

# Level 2: Show detailed calculations
debtmap analyze . -vv

# Level 3: Show all debug information
debtmap analyze . -vvv
```

**What each level shows**:
- `-v`: Score breakdowns, main contributing factors
- `-vv`: Detailed metric calculations, file processing
- `-vvv`: Full debug output, context provider details

## Diagnostic Options

```bash
# Show macro parsing warnings (Rust)
debtmap analyze . --verbose-macro-warnings

# Show macro expansion statistics (Rust)
debtmap analyze . --show-macro-stats

# Disable semantic analysis (fallback mode)
debtmap analyze . --semantic-off

# Validate LOC consistency
debtmap analyze . --validate-loc
```

**Note**: The `--explain-score` flag has been deprecated in favor of granular verbosity levels (`-v`, `-vv`, `-vvv`).

## Debugging Score Calculations

```bash
# Use verbosity levels to see score breakdown
debtmap analyze . -v    # Shows score factors

# See how coverage affects scores
debtmap analyze . --coverage-file coverage.info -v

# See how context affects scores
debtmap analyze . --context --context-providers critical_path -v
```

## Example Debug Session

```bash
# Step 1: Run with verbosity to see what's happening
debtmap analyze . -vv

# Step 2: Try without semantic analysis
debtmap analyze . --semantic-off -v

# Step 3: Check specific file
debtmap analyze path/to/file.rs -vvv

# Step 4: Validate results
debtmap analyze . --validate-loc
```

## See Also

- [Quick Fixes](quick-fixes.md) - Common problems with immediate solutions
- [Error Messages Reference](error-messages.md) - Detailed error message explanations
