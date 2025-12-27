# Output and Command Issues

Troubleshooting output formatting and command-specific problems.

## Output Format Selection

Debtmap supports multiple output formats for different use cases:

```bash
# Terminal format (default, human-readable with colors and emoji)
debtmap analyze .

# JSON format (unified format for programmatic processing)
debtmap analyze . --format json

# Markdown format (comprehensive analysis with LLM-optimized structure)
debtmap analyze . --format markdown

# HTML format (for web display)
debtmap analyze . --format html

# DOT format (Graphviz dependency visualization)
debtmap analyze . --format dot
```

**Source:** src/cli/args.rs:573-583 (OutputFormat enum)

## JSON Format

JSON output uses the unified format internally (spec 202 removed legacy format). All JSON output is automatically structured with a consistent schema.

```bash
# Generate JSON output
debtmap analyze . --format json

# Validate JSON with jq
debtmap analyze . --format json | jq .

# Write to file
debtmap analyze . --format json --output results.json
```

**Source:** src/output/json.rs:32 - "Always use unified format (spec 202 - removed legacy format)"

**Note:** There is no `--output-format` flag. The unified format is always used automatically.

## Plain Output Mode

For environments without color or emoji support (CI/CD, terminals without UTF-8):

```bash
# ASCII-only, no colors, no emoji
debtmap analyze . --plain

# Combine with JSON for machine-readable CI output
debtmap analyze . --format json --plain --output report.json
```

**Source:** src/cli/args.rs:176-178 (`--plain` flag definition)

**Note:** Only the `--plain` flag is supported. There is no `NO_EMOJI` environment variable.

## Compare Command Issues

The `compare` command generates diff reports between two analysis snapshots.

**Note**: The `compare` command defaults to JSON output format (unlike `analyze` which defaults to terminal).

### Basic Usage

```bash
# Save baseline results
debtmap analyze . --format json --output before.json

# Make code changes...

# Save new results
debtmap analyze . --format json --output after.json

# Compare results (outputs JSON by default)
debtmap compare --before before.json --after after.json

# Compare with terminal output
debtmap compare --before before.json --after after.json --format terminal
```

### Target Location Format

When specifying a target location, use the format `file:function:line`:

```bash
# With explicit target location
debtmap compare --before before.json --after after.json \
  --target-location "src/main.rs:process_file:42"

# Or use an implementation plan file
debtmap compare --before before.json --after after.json \
  --plan IMPLEMENTATION_PLAN.md
```

**Source:** src/cli/args.rs:483-489 (target_location format specification)

**Note:** `--plan` and `--target-location` are mutually exclusive. Using both causes an error.

### Common Compare Command Errors

**Problem**: "Cannot use --plan and --target-location together"

**Solution**: Use only one method to specify the target:
```bash
# Option 1: Use plan file
debtmap compare --before before.json --after after.json --plan IMPLEMENTATION_PLAN.md

# Option 2: Use explicit target location
debtmap compare --before before.json --after after.json --target-location "src/lib.rs:main:10"
```

**Problem**: Empty comparison results

**Solution**: Ensure both JSON files contain valid analysis output:
```bash
# Verify files have content
jq '.items | length' before.json
jq '.items | length' after.json

# Regenerate if empty
debtmap analyze . --format json --output before.json
```

**Problem**: Target location not found in comparison

**Solution**: Verify the target location format is `file:function:line`:
```bash
# Correct format
--target-location "src/parser.rs:parse_expression:45"

# Incorrect formats (missing components)
--target-location "src/parser.rs"         # Missing function and line
--target-location "parse_expression"      # Missing file and line
```

## Validate Command Issues

The `validate` command checks if a codebase meets specified quality thresholds.

### Basic Validation

```bash
# Validate codebase against thresholds
debtmap validate /path/to/project

# Set maximum acceptable debt density
debtmap validate /path/to/project --max-debt-density 10.0

# Use configuration file for thresholds
debtmap validate /path/to/project --config debtmap.toml
```

### CI/CD Integration

```bash
# In CI pipeline (fails build if validation fails)
debtmap validate . --max-debt-density 10.0 || exit 1

# With verbose output for debugging
debtmap validate . --max-debt-density 10.0 -v

# With coverage integration
debtmap validate . --coverage-file coverage.lcov --max-debt-density 10.0
```

### Exit Codes

The `validate` command returns:
- `0` - Success (all thresholds passed)
- Non-zero - Failure (thresholds exceeded or errors occurred)

**Source:** src/cli/args.rs:378-471 (Validate command definition)

## Validate-Improvement Command Issues

The `validate-improvement` command validates that technical debt improvements meet quality thresholds.

### Basic Usage

```bash
# First, create a comparison file
debtmap compare --before before.json --after after.json --output comparison.json

# Then validate the improvement
debtmap validate-improvement --comparison comparison.json
```

**Source:** src/commands/validate_improvement/mod.rs:1-45

### Configuration Options

```bash
# Set custom improvement threshold (default: 75%)
debtmap validate-improvement --comparison comparison.json --threshold 80.0

# Custom output location (default: .prodigy/debtmap-validation.json)
debtmap validate-improvement --comparison comparison.json --output validation.json

# Track progress across multiple attempts
debtmap validate-improvement --comparison comparison.json \
  --previous-validation .prodigy/previous-validation.json

# Suppress console output (for automation)
debtmap validate-improvement --comparison comparison.json --quiet

# Output format (json, terminal, or markdown)
debtmap validate-improvement --comparison comparison.json --format terminal
```

**Source:** src/cli/args.rs:502-531 (ValidateImprovement command definition)

### Composite Score Calculation

The validation score combines three weighted components:

| Component | Weight | Description |
|-----------|--------|-------------|
| Target Improvement | 50% | Did the specific target item improve? |
| Project Health | 30% | Did overall project debt decrease? |
| No Regressions | 20% | Were new critical items introduced? |

**Formula:** `score = (0.5 × target) + (0.3 × health) + (0.2 × no_regressions)`

**Source:** src/commands/validate_improvement/scoring.rs (calculate_composite_score function)

### Common Validate-Improvement Errors

**Problem**: "Cannot read comparison file"

**Solution**: Ensure the comparison file exists and is valid JSON:
```bash
# Verify file exists
ls -la comparison.json

# Validate JSON format
jq . comparison.json

# Regenerate if needed
debtmap compare --before before.json --after after.json --output comparison.json
```

**Problem**: Low improvement score despite fixing issues

**Cause**: The composite score considers overall project health and regressions, not just the target fix.

**Solution**: Check for regressions introduced elsewhere:
```bash
# Use verbose output to see score breakdown
debtmap validate-improvement --comparison comparison.json --format terminal
```

**Problem**: Threshold not being met

**Solution**: Adjust the threshold or improve more areas:
```bash
# Lower threshold for partial improvements
debtmap validate-improvement --comparison comparison.json --threshold 50.0

# Or continue improving until the default 75% threshold is met
```

### Trend Tracking

Track improvement trends across multiple validation attempts:

```bash
# First attempt
debtmap validate-improvement --comparison comparison.json \
  --output .prodigy/validation-1.json

# Second attempt (after more improvements)
debtmap validate-improvement --comparison comparison-2.json \
  --previous-validation .prodigy/validation-1.json \
  --output .prodigy/validation-2.json
```

The output includes trend analysis with direction and recommendations.

**Source:** src/commands/validate_improvement/types.rs:81-88 (TrendAnalysis structure)

## Summary vs Full Output

```bash
# Summary mode (compact tiered display)
debtmap analyze . --summary
debtmap analyze . -s

# Full output (default)
debtmap analyze .

# Limit number of items
debtmap analyze . --top 10
debtmap analyze . --tail 10
```

## Filtering Output

```bash
# Minimum priority level
debtmap analyze . --min-priority high

# Filter by debt categories (comma-separated)
debtmap analyze . --filter complexity,duplication

# Combine filters
debtmap analyze . --min-priority medium --top 20 --filter complexity

# Show filter statistics
debtmap analyze . --filter complexity --show-filter-stats

# Minimum score threshold for T3/T4 items (default: 3.0)
debtmap analyze . --min-score 5.0
```

**Source:** src/cli/args.rs:153-155 (`--filter` with `value_delimiter=','`)

**Note:** Categories are comma-separated values. Use `--show-filter-stats` to see how many items were filtered.

## See Also

- [Quick Fixes](quick-fixes.md) - Common problems with immediate solutions
- [CLI Reference](../cli-reference.md) - Complete CLI flag documentation
- [Debug Mode](debug-mode.md) - Debugging analysis issues
