# Coverage Integration

Debtmap integrates with test coverage tools to identify truly risky code: complex functions without adequate test coverage. This chapter covers coverage setup, tool integration, and troubleshooting.

## Overview

Coverage-risk correlation is one of Debtmap's unique features. By combining complexity metrics with test coverage data, Debtmap identifies:

- **Critical gaps:** High complexity + low/no coverage
- **Managed risk:** High complexity + good coverage
- **Low priority:** Low complexity regardless of coverage

## LCOV Format Support

Debtmap uses the LCOV format, a standard coverage format supported by most tools across languages.

### LCOV File Structure

```
TN:test_name
SF:/path/to/source.rs
FN:10,function_name
FNDA:5,function_name
FNF:1
FNH:1
DA:10,5
DA:11,5
DA:12,0
LF:3
LH:2
end_of_record
```

**Key fields:**
- `SF:` - Source file path
- `FN:` - Function at line number
- `FNDA:` - Function execution count
- `DA:` - Line execution count
- `LH/LF` - Lines hit / lines found

## Generating Coverage Data

### Rust (cargo-tarpaulin)

**Install:**
```bash
cargo install cargo-tarpaulin
```

**Generate coverage:**
```bash
cargo tarpaulin --out lcov --output-dir target/coverage
```

**Analyze with coverage:**
```bash
debtmap analyze . --lcov target/coverage/lcov.info
```

**Common options:**
```bash
# Exclude tests from coverage
cargo tarpaulin --out lcov --exclude-tests

# Include all files (even untested)
cargo tarpaulin --out lcov --follow-exec --post-test-delay 1

# Faster coverage (skip doc tests)
cargo tarpaulin --out lcov --no-default-features
```

### Python (pytest + pytest-cov)

**Install:**
```bash
pip install pytest pytest-cov
```

**Generate coverage:**
```bash
pytest --cov=src --cov-report=lcov:coverage/lcov.info
```

**Analyze with coverage:**
```bash
debtmap analyze . --lcov coverage/lcov.info --languages python
```

### JavaScript/TypeScript (Jest)

**Configure Jest** (`jest.config.js`):
```javascript
module.exports = {
  collectCoverage: true,
  coverageReporters: ['lcov'],
  coverageDirectory: 'coverage',
};
```

**Generate coverage:**
```bash
npm test -- --coverage
```

**Analyze with coverage:**
```bash
debtmap analyze . --lcov coverage/lcov.info --languages javascript,typescript
```

### Go (go test + gocover-cobertura)

**Generate coverage:**
```bash
go test -coverprofile=coverage.out ./...
gocover-cobertura < coverage.out > coverage.xml
```

**Convert to LCOV:**
```bash
# Use a converter like gocov-xml or gcov2lcov
gcov2lcov -i coverage.out -o coverage/lcov.info
```

**Analyze with coverage:**
```bash
debtmap analyze . --lcov coverage/lcov.info --languages go
```

## Coverage Index Performance

Debtmap builds an efficient coverage index for fast lookups:

**Index characteristics:**
- **Build time:** O(n), ~20-30ms for 5000 functions
- **Lookup time:** O(1) for exact matches (~0.5μs), O(log n) for fallback (~5-8μs)
- **Memory usage:** ~200 bytes per record (~2MB for 5000 functions)
- **Thread safety:** Arc<CoverageIndex> for lock-free parallel access

**Analysis overhead with coverage:**
- Baseline (no coverage): 100%
- With coverage: ~250% (2.5x)
- Target: ≤3x

## Transitive Coverage Propagation

Coverage impact flows through the call graph:

```
Transitive Coverage = Direct Coverage + Σ(Caller Coverage × Weight)
```

**Example:**
```
Function A: 0% direct coverage
  ├─ Called by: main() [integration tested, 100% coverage]
  └─ Called by: process_batch() [0% coverage]

Transitive Coverage for A: ~60% (weighted by caller importance)
Priority: MEDIUM (reduced from HIGH due to integration test coverage)
```

**Benefits:**
- Functions called by well-tested code have reduced urgency
- Entry points tested via integration tests don't need unit tests
- Risk propagates through untested call chains

## Using Coverage Data

### Basic Analysis

```bash
debtmap analyze . --lcov target/coverage/lcov.info
```

### Filter Untested Functions

```bash
debtmap analyze . --lcov target/coverage/lcov.info --min-priority high
```

### Show Coverage Details

```bash
debtmap analyze . --lcov target/coverage/lcov.info -vv
```

Output includes:
- Direct coverage percentage
- Transitive coverage percentage
- Coverage lookup details (exact vs fallback match)

### Coverage Dampening

Coverage data dampens debt scores:

```
Final Score = Base Score × (1.0 - coverage_percentage)
```

**Examples:**
- 100% coverage → Score multiplier = 0.0 (near-zero debt score)
- 50% coverage → Score multiplier = 0.5 (half the base score)
- 0% coverage → Score multiplier = 1.0 (full base score)

**Invariant:**
```
Total debt score with coverage ≤ Total debt score without coverage
```

## Troubleshooting

### Coverage File Not Found

**Error:**
```
Error: Failed to read coverage file: No such file or directory
```

**Solution:**
```bash
# Verify file exists
ls -l target/coverage/lcov.info

# Use absolute path
debtmap analyze . --lcov $(pwd)/target/coverage/lcov.info
```

### Coverage Not Correlating with Functions

**Symptoms:**
- All functions show 0% coverage
- Coverage percentages seem incorrect

**Possible causes:**
1. Path mismatch between LCOV and source files
2. LCOV file is empty or malformed
3. Function name normalization issues

**Solutions:**

**Check LCOV file contents:**
```bash
head -50 target/coverage/lcov.info
```

Verify `SF:` paths match source file locations.

**Check for path prefix differences:**
```bash
# LCOV may use absolute paths
SF:/Users/you/project/src/main.rs

# While debtmap expects relative paths
src/main.rs
```

**Use -vv to see coverage lookup details:**
```bash
debtmap analyze . --lcov target/coverage/lcov.info -vv
```

Look for:
```
Coverage lookup: src/main.rs:process_data:45
  ├─ Exact match: YES
  └─ Coverage: 85.5%
```

or:
```
Coverage lookup: src/main.rs:process_data:45
  ├─ Exact match: NO
  ├─ Fallback match: src/main.rs:process_data (no line number)
  └─ Coverage: 85.5%
```

### Low Coverage Percentages

**Symptoms:**
- Expected high coverage, but debtmap shows low

**Possible cause:** Coverage tool excludes certain code (tests, macros)

**Solution:**
```bash
# Include all code in coverage
cargo tarpaulin --out lcov --follow-exec --include-tests
```

### Performance Issues with Large Coverage Files

**Symptoms:**
- Analysis is very slow with coverage
- Memory usage is high

**Solutions:**

**1. Filter coverage file:**
```bash
# Only include source files (not tests)
grep -v '/tests/' target/coverage/lcov.info > target/coverage/lcov-filtered.info
debtmap analyze . --lcov target/coverage/lcov-filtered.info
```

**2. Reduce thread count:**
```bash
debtmap analyze . --lcov target/coverage/lcov.info --jobs 2
```

## Best Practices

1. **Generate coverage before analysis** - Run tests first, then analyze
2. **Include all source files** - Even untested files should appear in LCOV
3. **Use relative paths** - Ensure LCOV paths match source tree structure
4. **Exclude test code** - Focus coverage on production code
5. **Verify coverage quality** - Check that important functions are covered
6. **Use -vv for debugging** - Verbose output shows coverage lookup details

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Code Quality

on: [push, pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install tools
        run: |
          cargo install cargo-tarpaulin debtmap

      - name: Generate coverage
        run: cargo tarpaulin --out lcov --output-dir coverage

      - name: Analyze with debtmap
        run: |
          debtmap analyze . \
            --lcov coverage/lcov.info \
            --format json \
            --output debtmap-report.json

      - name: Upload results
        uses: actions/upload-artifact@v2
        with:
          name: debtmap-report
          path: debtmap-report.json
```

## See Also

- [Risk Assessment](analysis-guide.md#risk-assessment) - How risk scoring works
- [Unified Scoring](scoring-strategies.md) - Multi-factor scoring system
- [Configuration](configuration.md) - Scoring weight configuration
- [Troubleshooting](troubleshooting.md) - General troubleshooting guide
