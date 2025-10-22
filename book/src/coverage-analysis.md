# Coverage Gap Analysis

Debtmap provides precise line-level coverage gap reporting to help you understand exactly which lines of code lack test coverage, rather than relying on misleading function-level percentages.

## Understanding Coverage Gaps

A **coverage gap** represents the portion of a function that is not executed during tests. Traditional tools report this as a simple percentage (e.g., "50% covered"), but this can be misleading:

- A 100-line function with 1 uncovered line shows "99% covered" - sounds great!
- A 10-line function with 1 uncovered line shows "90% covered" - sounds worse, but is actually better

Debtmap improves on this by:
1. Reporting the actual number of uncovered lines
2. Showing which specific lines are uncovered
3. Calculating the gap as a percentage of instrumented lines (not total lines)
4. Providing visual severity indicators based on gap size

## Precise vs Estimated Gaps

Debtmap uses different precision levels depending on available coverage data:

### Precise Gaps (Line-Level Data Available)

When LCOV coverage data is available, debtmap provides exact line-level reporting:

```
Business logic - 1 line uncovered (11% gap) - line 52
Complex calculation - 4 lines uncovered (20% gap) - lines 10-12, 15
```

**Benefits:**
- Exact line numbers for uncovered code
- Accurate gap percentage based on instrumented lines
- Compact line range formatting (e.g., "10-12, 15, 20-21")
- Distinguishes between code that can't be instrumented vs uncovered code

**How it works:**
- Debtmap reads LCOV coverage data from your test runs
- Matches functions by file path and name
- Extracts precise uncovered line numbers
- Calculates percentage as: `(uncovered_lines / instrumented_lines) * 100`

### Estimated Gaps (Function-Level Data Only)

When only function-level coverage percentages are available:

```
Data processing - ~50% gap (estimated, ~25 lines)
Helper function - ~100% gap (estimated, 15 lines)
Utility - ~3% gap (mostly covered)
```

**Characteristics:**
- Estimates uncovered line count from percentage
- Uses tilde (~) prefix to indicate estimation
- Special cases:
  - ≥99% gap → "~100% gap"
  - <5% gap → "mostly covered"
  - Otherwise → "~X% gap (estimated, ~Y lines)"

**How it works:**
- Falls back when LCOV data unavailable or function not found
- Calculates: `estimated_uncovered = total_lines * (gap_percentage / 100)`
- Useful for quick overview but less actionable than precise gaps

### Unknown Coverage

When no coverage data is available:

```
Untested module - Coverage data unavailable (42 lines)
```

This typically occurs when:
- No coverage collection has been run
- File not included in coverage report
- Coverage data file path mismatch

## Gap Severity Indicators

Debtmap uses visual indicators to quickly identify the severity of coverage gaps:

| Indicator | Range | Severity | Meaning |
|-----------|-------|----------|---------|
| 🟡 | 1-25% | LOW | Minor gaps, mostly covered |
| 🟠 | 26-50% | MODERATE | Significant gaps, needs attention |
| 🔴 | 51-75% | HIGH | Major gaps, high priority |
| 🔴🔴 | 76-100% | CRITICAL | Severe gaps, critical priority |

These indicators appear in debtmap's priority output to help you quickly identify which functions need testing most urgently.

### Severity Calculation

Gap severity is based on the percentage of uncovered code:

```rust
fn get_severity(gap_percentage: f64) -> &'static str {
    match gap_percentage {
        p if p <= 25.0 => "🟡 LOW",
        p if p <= 50.0 => "🟠 MODERATE",
        p if p <= 75.0 => "🔴 HIGH",
        _ => "🔴🔴 CRITICAL"
    }
}
```

This works for both precise and estimated gaps, ensuring consistent severity classification across your codebase.

## Example Output

### High Verbosity Mode

```
Priority 1: Authentication Logic (CRITICAL)
  File: src/auth/login.rs:45
  Coverage Gap: 2 lines uncovered (89% gap) 🔴🔴 - lines 67, 89
  Complexity: Cyclomatic 8, Cognitive 12
  Impact: High-risk business logic with critical coverage gaps

Priority 2: Data Validation (HIGH)
  File: src/validation/rules.rs:120
  Coverage Gap: 15 lines uncovered (65% gap) 🔴 - lines 145-152, 167-173
  Complexity: Cyclomatic 5, Cognitive 8
  Impact: Complex validation logic needs comprehensive testing

Priority 3: Helper Function (MODERATE)
  File: src/utils/helpers.rs:30
  Coverage Gap: ~45% gap (estimated, ~12 lines) 🟠
  Complexity: Cyclomatic 3, Cognitive 4
  Impact: Moderate complexity with estimated coverage gaps
```

### Standard Mode

```
1. Authentication Logic (src/auth/login.rs:45)
   Gap: 2 lines uncovered (89%) 🔴🔴 [lines 67, 89]

2. Data Validation (src/validation/rules.rs:120)
   Gap: 15 lines uncovered (65%) 🔴 [lines 145-152, 167-173]

3. Helper Function (src/utils/helpers.rs:30)
   Gap: ~45% (estimated) 🟠
```

## Integration with Coverage Tools

### Generating LCOV Data

For precise gap reporting, generate LCOV coverage data with your test framework:

**Rust (using cargo-tarpaulin):**
```bash
cargo tarpaulin --out Lcov --output-dir ./coverage
```

**Python (using pytest-cov):**
```bash
pytest --cov=mypackage --cov-report=lcov:coverage/lcov.info
```

**JavaScript (using Jest):**
```bash
jest --coverage --coverageReporters=lcov
```

### Configuring Debtmap

Point debtmap to your coverage data:

```bash
debtmap analyze --coverage-path ./coverage/lcov.info
```

Or in `.debtmap.toml`:

```toml
[coverage]
lcov_path = "./coverage/lcov.info"
```

## Best Practices

### 1. Use Precise Gaps When Possible

Always generate LCOV data for actionable coverage insights:
- Precise line numbers help you quickly locate untested code
- Accurate percentages prevent over/under-estimating gaps
- Line ranges show if gaps are concentrated or scattered

### 2. Focus on High Severity Gaps First

Prioritize based on severity indicators:
1. 🔴🔴 CRITICAL (76-100%) - Address immediately
2. 🔴 HIGH (51-75%) - Schedule for next sprint
3. 🟠 MODERATE (26-50%) - Address when convenient
4. 🟡 LOW (1-25%) - Acceptable for some code

### 3. Consider Context

Gap severity should be weighted by:
- **Function role**: Business logic vs utilities
- **Complexity**: High complexity + high gap = top priority
- **Change frequency**: Frequently changed code needs better coverage
- **Risk**: Security, data integrity, financial calculations

### 4. Track Progress Over Time

Run debtmap regularly to track coverage improvements:
```bash
# Weekly coverage check
debtmap analyze --coverage-path ./coverage/lcov.info > weekly-gaps.txt
```

Compare reports to see gap reduction progress.

## Troubleshooting

### "Coverage data unavailable" for all functions

**Cause**: Debtmap can't find or parse LCOV file

**Solutions**:
- Verify `--coverage-path` points to valid LCOV file
- Ensure LCOV file was generated recently
- Check file permissions (readable by debtmap)
- Validate LCOV format: `head -20 ./coverage/lcov.info`

### Line numbers don't match source code

**Cause**: Source code changed since coverage was generated

**Solutions**:
- Re-run tests with coverage collection
- Ensure clean build before coverage run
- Commit code before running coverage

### Estimated gaps for functions with LCOV data

**Cause**: Function name or path mismatch

**Solutions**:
- Check function names match exactly (case-sensitive)
- Verify file paths are consistent (relative vs absolute)
- Enable debug logging: `debtmap analyze --log-level debug`

### Missing functions in coverage report

**Cause**: Functions not instrumented or filtered out

**Solutions**:
- Check coverage tool configuration
- Ensure test execution reaches those functions
- Verify functions aren't in excluded paths

## Related Topics

- [Coverage Integration](coverage-integration.md) - Detailed coverage tool setup
- [Tiered Prioritization](tiered-prioritization.md) - How coverage gaps affect priority
- [Scoring Strategies](scoring-strategies.md) - Coverage weight in debt scoring
- [Metrics Reference](metrics-reference.md) - All coverage-related metrics
