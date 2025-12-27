# Troubleshooting

Common issues and solutions for using debtmap effectively.

## Getting Started with Troubleshooting

When you encounter an issue with debtmap, start with these steps:

1. **Try a quick fix** - See [Quick Fixes](quick-fixes.md) for common problems and immediate solutions
2. **Enable debug mode** - Use `-v`, `-vv`, or `-vvv` for increasing levels of detail
3. **Check error messages** - See [Error Messages Reference](error-messages.md) for explanations
4. **Review your configuration** - Check `.debtmap/config.toml` for any settings that might cause issues

## Subsections

This chapter is organized into focused troubleshooting topics:

- [Quick Fixes](quick-fixes.md) - Common problems with immediate solutions
- [Debug Mode](debug-mode.md) - Verbosity levels and diagnostic options
- [Context Provider Issues](context-providers.md) - Troubleshooting critical_path, dependency, and git_history providers
- [Advanced Analysis Issues](advanced-analysis.md) - Call graph, pattern detection, functional analysis issues
- [Error Messages Reference](error-messages.md) - Detailed error message explanations
- [Output and Command Issues](output-issues.md) - Output formatting and command-specific problems
- [FAQ](faq.md) - Frequently asked questions

## Common Problem Categories

### Analysis Problems

- **Slow analysis**: See [Quick Fixes](quick-fixes.md#slow-analysis) or [Debug Mode](debug-mode.md)
- **Parse errors**: See [Quick Fixes](quick-fixes.md#parse-errors) or [Error Messages Reference](error-messages.md#parse-errors)
- **No output**: See [Quick Fixes](quick-fixes.md#no-output)
- **Inconsistent results**: See [Quick Fixes](quick-fixes.md#inconsistent-results)

### Coverage and Context Problems

- **Coverage not applied**: See [Quick Fixes](quick-fixes.md#coverage-data-not-matching-functions)
- **Context provider errors**: See [Context Provider Issues](context-providers.md)

### Output Problems

- **JSON parsing issues**: See [Quick Fixes](quick-fixes.md#json-format-parsing-errors) or [Output and Command Issues](output-issues.md)
- **Compare command errors**: See [Output and Command Issues](output-issues.md#compare-command-issues)

### Detection Problems

- **God object false positives**: See [Quick Fixes](quick-fixes.md#god-object-false-positives) or [Advanced Analysis Issues](advanced-analysis.md#god-object-detection)
- **Pattern detection issues**: See [Advanced Analysis Issues](advanced-analysis.md#pattern-detection-issues)
- **Call graph problems**: See [Quick Fixes](quick-fixes.md#call-graph-resolution-failures) or [Advanced Analysis Issues](advanced-analysis.md#call-graph-debugging)

## Diagnostic Commands

### Basic Diagnostics

```bash
# Check version
debtmap --version

# Run with basic verbosity
debtmap analyze . -v

# Run with detailed output
debtmap analyze . -vv

# Run with full debug output
debtmap analyze . -vvv
```

### Coverage Diagnostics

```bash
# Debug coverage matching for a function
debtmap explain-coverage . \
  --coverage-file coverage.lcov \
  --function "function_name" \
  -v
```

### Performance Diagnostics

```bash
# Time the analysis
time debtmap analyze .

# Profile with verbosity
debtmap analyze . -vv 2>&1 | grep "processed in"
```

## When to File Bug Reports

File a bug report when:

- Parse errors on valid syntax
- Crashes or panics
- Incorrect complexity calculations
- Concurrency errors
- Incorrect error messages

**Before filing**:
1. Check this troubleshooting guide
2. Try `--semantic-off` fallback mode
3. Update to the latest version
4. Search existing issues on GitHub

See [FAQ](faq.md#when-to-file-bug-reports) for detailed guidance.

## Related Documentation

- [Configuration](../configuration/index.md) - Configure debtmap behavior
- [CLI Reference](../cli-reference.md) - Complete CLI flag documentation
- [Analysis Guide](../analysis-guide/index.md) - Understanding analysis results
- [Examples](../examples.md) - Practical usage examples
