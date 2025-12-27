# Output and Command Issues

Troubleshooting output formatting and command-specific problems.

## Output Format Selection

```bash
# Terminal format (default, human-readable)
debtmap analyze .

# JSON format
debtmap analyze . --format json

# Markdown format
debtmap analyze . --format markdown
```

## JSON Format Options

```bash
# Legacy format (default): {File: {...}}
debtmap analyze . --format json --output-format legacy

# Unified format: consistent structure with 'type' field
debtmap analyze . --format json --output-format unified

# Validate JSON
debtmap analyze . --format json | jq .

# Write to file
debtmap analyze . --format json --output results.json
```

## Plain Output Mode

For environments without color/emoji support:

```bash
# ASCII-only, no colors, no emoji
debtmap analyze . --plain

# Or set environment variable
export NO_EMOJI=1
debtmap analyze .
```

## Compare Command Issues

The `compare` command helps track changes in technical debt over time.

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

### Incompatible Format Errors

**Problem**: "Incompatible formats" error when comparing files

**Solutions**:
```bash
# Ensure both files use same output format
debtmap analyze . --format json --output-format unified --output before.json
debtmap analyze . --format json --output-format unified --output after.json
debtmap compare --before before.json --after after.json
```

## Validate Command Issues

The `validate` command checks if a codebase meets specified quality thresholds.

### Basic Validation

```bash
# Validate codebase passes default thresholds
debtmap validate /path/to/project

# Set maximum acceptable debt density
debtmap validate /path/to/project --max-debt-density 10.0
```

### CI/CD Integration

```bash
# In CI pipeline (fails build if validation fails)
debtmap validate . --max-debt-density 10.0 || exit 1

# With verbose output for debugging
debtmap validate . --max-debt-density 10.0 -v
```

## Summary vs Full Output

```bash
# Summary mode (compact)
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
debtmap analyze . --min-priority 5

# Category filters
debtmap analyze . --filter "complexity,debt"

# Combine filters
debtmap analyze . --min-priority 3 --top 20 --filter complexity
```

## See Also

- [Quick Fixes](quick-fixes.md) - Common problems with immediate solutions
- [CLI Reference](../cli-reference.md) - Complete CLI flag documentation
