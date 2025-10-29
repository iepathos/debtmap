# Output Format Guide

This guide explains debtmap's output format, section purposes, and configuration options.

## Output Structure

Each debt item is displayed with the following sections:

### Header
```
#1 SCORE: 85.5 [CRITICAL]
```

- **Rank**: Position in priority queue (#1 = highest priority)
- **Score**: Numerical priority score (0-100)
- **Severity**: Risk level indicator - `[CRITICAL]`, `[HIGH]`, `[MEDIUM]`, or `[LOW]`

### Location
```
├─ LOCATION: src/main.rs:42 process_request()
```

Shows where the debt item is located in your codebase.

### Evidence
```
├─ COMPLEXITY: cyclomatic=25, est_branches=25, cognitive=35, nesting=4
```

Provides objective metrics that justify the priority score. Evidence includes:
- **Complexity metrics**: Cyclomatic, cognitive, nesting depth
- **Coverage data**: Test coverage percentages and uncovered lines
- **Call graph**: Callers and callees (dependencies)

### Why This Matters
```
├─ WHY THIS MATTERS: High complexity combined with poor test coverage creates maintenance risk
```

Explains the **impact** and **rationale** for addressing this debt:
- Why the evidence is concerning
- What risks it poses to your project
- How it affects maintainability

### Recommended Action
```
├─ RECOMMENDED ACTION: Extract complex logic into smaller, testable functions
   - 1. Identify distinct responsibilities
   - 2. Extract pure logic functions
   - 3. Add unit tests for each extracted function
```

Provides **actionable next steps** to resolve the issue:
- Primary action to take
- Step-by-step implementation guide
- Specific refactoring patterns

## Verbosity Levels

Debtmap supports three verbosity levels to control output detail:

### Compact (`--compact` or `-c`)
Minimal output showing only top priority items with essential metrics.
```bash
debtmap analyze . --compact
```

Shows:
- Top 3 metrics per item
- Score and severity only
- Primary action only

### Normal (default)
Balanced output with key information.
```bash
debtmap analyze .
```

Shows:
- Top 6 metrics per item
- Location, evidence, rationale, and action
- Call graph summary

### Verbose (`--verbose` or `-v`)
Detailed output with full analysis.
```bash
debtmap analyze . --verbose  # or -v, -vv, -vvv
```

Verbosity levels:
- `-v`: Show main score factors
- `-vv`: Show detailed score calculations
- `-vvv`: Show all debug information including entropy details

Shows:
- All available metrics
- Detailed score breakdown
- Full call graph with all dependencies
- Coverage gap analysis
- Entropy adjustments

## Color Configuration

### Auto-detection (default)
Debtmap automatically detects if output is to a terminal (TTY) or pipe and adjusts color usage accordingly.

### Configuration File
Control color output in `.debtmap.toml`:

```toml
[output]
use_color = true  # Force colors on
# use_color = false  # Force colors off
# use_color is optional - defaults to auto-detection
```

### NO_COLOR Environment Variable
Debtmap respects the `NO_COLOR` environment variable:

```bash
NO_COLOR=1 debtmap analyze .
```

This disables colored output regardless of other settings.

### Priority
Color configuration priority (highest to lowest):
1. `NO_COLOR` environment variable (if set, disables colors)
2. `--plain` CLI flag (disables colors and emoji)
3. `.debtmap.toml` `use_color` setting
4. Auto-detection based on TTY

## Output Formats

### Terminal (default)
Human-readable output with colors and formatting.
```bash
debtmap analyze .
```

### JSON
Machine-readable structured output.
```bash
debtmap analyze . --format json
```

### Markdown
Documentation-friendly format.
```bash
debtmap analyze . --format markdown
```

### Plain Text
ASCII-only, no colors, machine-parseable.
```bash
debtmap analyze . --plain
```

## Filtering and Display Options

### Top N Items
Show only highest priority items:
```bash
debtmap analyze . --top 10
```

### Bottom N Items
Show lowest priority items:
```bash
debtmap analyze . --tail 5
```

### Summary Mode
Compact tiered display:
```bash
debtmap analyze . --summary
```

### Minimum Priority
Filter by severity level:
```bash
debtmap analyze . --min-priority high
```

## Examples

### Quick scan with minimal output
```bash
debtmap analyze . --compact --top 5
```

### Detailed analysis
```bash
debtmap analyze . --verbose --coverage-file coverage.lcov
```

### Machine-readable output
```bash
debtmap analyze . --format json --output results.json
```

### CI/CD friendly
```bash
NO_COLOR=1 debtmap analyze . --plain --format json
```

## Understanding Sections

### Evidence Section Purpose
The Evidence section provides **objective data** that supports the priority ranking:
- Metrics are factual measurements
- Data comes from code analysis and coverage reports
- Multiple signals increase confidence in the assessment

### Why This Matters Purpose
This section connects evidence to **real-world impact**:
- Explains business/engineering consequences
- Justifies why this item has high priority
- Helps teams understand the "so what?" of the metrics

### Recommended Action Purpose
This section provides **concrete next steps**:
- Actionable guidance, not just identification
- Specific refactoring patterns
- Prioritized implementation steps

This three-part structure ensures you have:
1. **Data** (Evidence)
2. **Understanding** (Why This Matters)
3. **Action** (Recommended Action)

## Configuration Reference

### CLI Flags
- `-c, --compact`: Minimal output
- `-v, --verbose`: Detailed output (repeatable: -v, -vv, -vvv)
- `--plain`: ASCII-only, no colors
- `--format <FORMAT>`: Output format (terminal, json, markdown)
- `--top <N>`: Show top N items
- `--tail <N>`: Show bottom N items
- `--summary`: Tiered priority display

### Config File (.debtmap.toml)

```toml
[output]
# Enable or disable colored output
# default: auto-detect based on TTY
use_color = true

# Default output format
default_format = "terminal"
```

## See Also

- [Migration Guide](./migration-unified-format.md) - Upgrading from older output formats
- [Configuration Guide](../book/src/configuration.md) - Full configuration options
- [CLI Reference](../book/src/cli-reference.md) - Complete CLI documentation
