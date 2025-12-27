# Troubleshooting FAQ

Frequently asked questions about troubleshooting debtmap issues.

## General Questions

**Q: Why is my analysis slow?**

A: Check several factors:
```bash
# Use all CPU cores
debtmap analyze . --jobs 0

# Disable multi-pass for faster analysis
debtmap analyze . --no-multi-pass

# Try faster fallback mode
debtmap analyze . --semantic-off
```

**Q: What does 'Parse error' mean?**

A: File contains syntax debtmap cannot parse. Try:
- `--semantic-off` for fallback mode
- `--verbose-macro-warnings` for Rust macros
- Exclude problematic files in `.debtmap/config.toml`

**Q: Why do scores differ between runs?**

A: Several factors affect scores:
- Coverage file changed
- Context providers enabled/disabled
- Code changes (intended behavior)
- Different threshold settings

## Coverage Questions

**Q: How does coverage affect scores?**

A: Coverage affects scores through multiplicative factors (from `src/risk/strategy.rs:189-204`):
- **< 20% coverage**: 3.0x penalty
- **20-40% coverage**: 2.0x penalty
- **40-60% coverage**: 1.5x penalty
- **60-80% coverage**: 1.2x penalty
- **≥ 80% coverage**: 0.8x reduction (bonus for high coverage)

**Q: Why isn't my coverage data being applied?**

A: Use the explain-coverage command:
```bash
debtmap explain-coverage . \
  --coverage-file coverage.lcov \
  --function "function_name" \
  -v
```

## Output Questions

**Q: Why no output?**

A: Check verbosity and filtering:
```bash
# Increase verbosity
debtmap analyze . -v

# Lower priority threshold
debtmap analyze . --min-priority 0

# Use lenient threshold
debtmap analyze . --threshold-preset lenient
```

**Q: What's the difference between legacy and unified JSON?**

A:
- **Legacy**: `{File: {...}}` - nested structure
- **Unified**: `{type: "File", ...}` - consistent structure

## Scoring Questions

**Q: What's the difference between cyclomatic and cognitive complexity?**

A: Cyclomatic complexity counts decision points (branches, conditions), while cognitive complexity measures human comprehension difficulty (nested structures, breaks in linear flow). Both are measured metrics computed directly from the AST.

**Q: How does coverage dampening work?**

A: Well-tested code gets lower debt scores through a dampening multiplier. Functions with high coverage (≥80%) receive a 0.8x reduction, surfacing untested complex functions as higher priority targets for improvement.

**Q: When should I use god object detection vs boilerplate detection?**

A: Use **god object detection** for large, complex classes that have too many responsibilities and need to be split into modules. Use **boilerplate detection** for repetitive, low-complexity code patterns that could benefit from macros or code generation.

**Q: What are measured vs estimated metrics?**

A: **Measured metrics** (cyclomatic, cognitive complexity, nesting depth) are precise values computed from AST analysis. **Estimated metrics** (branches) are heuristic approximations used for test planning and coverage predictions.

## When to File Bug Reports

File a bug report when:
- Parse errors on valid syntax
- Crashes or panics
- Incorrect complexity calculations
- Concurrency errors

**Before filing**:
1. Check this troubleshooting guide
2. Try `--semantic-off` fallback mode
3. Update to the latest version
4. Search existing GitHub issues

## See Also

- [Quick Fixes](quick-fixes.md) - Common problems with immediate solutions
- [Debug Mode](debug-mode.md) - Verbosity and diagnostics
- [Error Messages Reference](error-messages.md) - Error explanations
