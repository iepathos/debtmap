# Display and Output Configuration

This chapter covers configuration options that control how debtmap displays analysis results and formats output. These settings affect terminal rendering, verbosity levels, color modes, and evidence display for multi-signal classification.

## Quick Reference

Key display and output configuration options (from `src/config/display.rs`):

| Configuration | Default | Purpose |
|---------------|---------|---------|
| **Display Settings** | | |
| `tiered` | `true` | Use tiered priority display |
| `items_per_tier` | `5` | Items shown per priority tier |
| `verbosity` | `detailed` | Output verbosity level |
| **Output Settings** | | |
| `evidence_verbosity` | `minimal` | Multi-signal evidence detail |
| `min_confidence_warning` | `0.80` | Minimum confidence for warnings |
| **Color Mode** | | |
| `color` | `auto` | Color output control |
| **Formatting** | | |
| `show_splits` | `false` | Show god object split recommendations |
| `max_callers` | `5` | Maximum callers to display |
| `max_callees` | `5` | Maximum callees to display |

## Display Settings

The `[display]` section controls how analysis results are organized and presented. These settings are defined in the `DisplayConfig` struct (`src/config/display.rs:56-68`).

### Basic Display Configuration

```toml
[display]
# Use tiered priority display for organizing results
tiered = true

# Maximum items to show per tier
items_per_tier = 5

# Verbosity level (summary, detailed, comprehensive)
verbosity = "detailed"
```

### Verbosity Levels

The `VerbosityLevel` enum (`src/config/display.rs:7-15`) controls how much detail appears in output:

| Level | Description | Use Case |
|-------|-------------|----------|
| `summary` | Essential information only | Quick health checks |
| `detailed` | Includes module structure details | Normal development use |
| `comprehensive` | All available analysis data | Debugging and deep analysis |

**Example:**

```toml
[display]
# Summary mode - minimal output
verbosity = "summary"
```

```toml
[display]
# Comprehensive mode - all details
verbosity = "comprehensive"
```

The default is `detailed`, which provides a balance between information density and readability.

### Tiered Priority Display

When `tiered = true`, debtmap organizes debt items by priority tier (Critical, High, Medium, Low) and limits output to `items_per_tier` items per tier:

```toml
[display]
tiered = true
items_per_tier = 10  # Show up to 10 items per tier
```

This prevents output from being overwhelming when analyzing large codebases with many debt items.

## Output Format Configuration

The `format` field in `[display]` sets the default output format:

```toml
[display]
# Default output format (terminal, json, markdown, html)
format = "terminal"
```

Available formats (from `src/cli/args.rs:574-583`):

| Format | Description |
|--------|-------------|
| `terminal` | Interactive colored output for terminals |
| `json` | Machine-readable structured data |
| `markdown` | Documentation-friendly reports |
| `html` | Interactive web dashboard |
| `dot` | Graphviz DOT format for dependency visualization |

See [Output Formats](../output-formats.md) for detailed format documentation.

## Color and Terminal Options

### Color Mode

The `ColorMode` enum (`src/formatting/mod.rs:6-11`) controls color output:

| Mode | Behavior |
|------|----------|
| `auto` | Detect terminal color support automatically |
| `always` | Force colors on (even when piping) |
| `never` | Disable colors entirely |

```toml
[display]
# Color configuration is typically handled via CLI or environment
# but can be set in config
plain = false  # false = colors enabled (when supported)
```

### Environment Variable Controls

Debtmap respects standard environment variables for color control (`src/formatting/mod.rs:67-90`):

| Variable | Effect |
|----------|--------|
| `NO_COLOR` | If set, disables colors (per [no-color.org](https://no-color.org)) |
| `CLICOLOR=0` | Disables colors |
| `CLICOLOR_FORCE=1` | Forces colors even when not a TTY |
| `TERM=dumb` | Disables colors for dumb terminals |

**Precedence** (highest to lowest):
1. `CLICOLOR_FORCE=1` - Forces colors on
2. `NO_COLOR` or `CLICOLOR=0` - Disables colors
3. Terminal detection - Auto-detect based on TTY status

### Plain Mode

For environments without color support or when piping output:

```toml
[display]
plain = true  # ASCII-only, no colors, no emoji
```

Or via CLI: `debtmap analyze . --plain`

## Evidence Display Configuration

Multi-signal classification produces evidence that can be displayed at varying levels of detail. The `OutputConfig` struct (`src/config/display.rs:122-145`) controls evidence output.

### Evidence Verbosity

The `EvidenceVerbosity` enum (`src/config/display.rs:18-30`) maps to `-v` flag counts:

| Level | `-v` Count | Description |
|-------|-----------|-------------|
| `minimal` | 0 | Category and confidence only |
| `standard` | 1 | Signal summary |
| `verbose` | 2 | Detailed breakdown |
| `very_verbose` | 3 | All signals including low-weight ones |

```toml
[output]
# Set evidence verbosity in config
evidence_verbosity = "standard"

# Minimum confidence for showing warnings
min_confidence_warning = 0.80
```

### Signal Filters

The `SignalFilterConfig` (`src/config/display.rs:152-190`) controls which classification signals appear in output:

```toml
[output.signal_filters]
# Show I/O detection signal
show_io_detection = true

# Show call graph signal
show_call_graph = true

# Show type signatures signal
show_type_signatures = true

# Show purity signal
show_purity = true

# Show framework signal
show_framework = true

# Show name heuristics signal (low weight, hidden by default)
show_name_heuristics = false
```

**Signal Filter Defaults:**

| Filter | Default | Purpose |
|--------|---------|---------|
| `show_io_detection` | `true` | I/O operation detection signals |
| `show_call_graph` | `true` | Call graph analysis signals |
| `show_type_signatures` | `true` | Type signature analysis |
| `show_purity` | `true` | Function purity classification |
| `show_framework` | `true` | Framework pattern detection |
| `show_name_heuristics` | `false` | Low-weight naming heuristics |

Name heuristics are hidden by default because they are a low-weight fallback signal.

## Formatting Configuration

The `FormattingConfig` struct (`src/formatting/mod.rs:32-48`) controls advanced formatting options.

### Caller/Callee Display

Configure how call graph relationships are displayed:

```toml
[display.formatting]
# Maximum number of callers to display per function
max_callers = 5

# Maximum number of callees to display per function
max_callees = 5

# Show calls to external crates
show_external = false

# Show standard library calls
show_std_lib = false
```

These settings are defined in `CallerCalleeConfig` (`src/config/classification.rs:7-23`).

### Show Splits

Enable detailed god object module split recommendations:

```toml
[display.formatting]
# Show detailed module split recommendations
show_splits = false
```

When enabled, debtmap provides specific recommendations for breaking down god objects into smaller modules. This is defined in `FormattingConfig` (`src/formatting/mod.rs:36-37`).

## Complete Configuration Example

Here's a complete `[display]` configuration section showing all options:

```toml
# Display and output configuration
[display]
# Output format (terminal, json, markdown, html)
format = "terminal"

# Verbosity level (0-3 or named: summary, detailed, comprehensive)
verbosity = "detailed"

# Use compact output format
compact = false

# Use summary format with tiered priority display
summary = false

# Enable tiered priority display
tiered = true

# Items per priority tier
items_per_tier = 5

# Group output by debt category
group_by_category = false

# Show complexity attribution details
show_attribution = false

# Detail level (summary, standard, comprehensive, debug)
detail_level = "standard"

# Disable TUI progress visualization
no_tui = false

# Plain output mode (ASCII-only, no colors, no emoji)
plain = false

# Formatting options
[display.formatting]
# Show dependency information (callers/callees)
show_dependencies = false

# Maximum callers to display
max_callers = 5

# Maximum callees to display
max_callees = 5

# Show external crate calls
show_external = false

# Show standard library calls
show_std_lib = false

# Show god object split recommendations
show_splits = false

# Evidence and output configuration
[output]
# Evidence verbosity for multi-signal classification
evidence_verbosity = "minimal"

# Minimum confidence for showing warnings (0.0-1.0)
min_confidence_warning = 0.80

# Signal filters for evidence display
[output.signal_filters]
show_io_detection = true
show_call_graph = true
show_type_signatures = true
show_purity = true
show_framework = true
show_name_heuristics = false
```

## CLI Flag Overrides

Many display settings can be overridden via CLI flags:

| Config Option | CLI Flag |
|---------------|----------|
| `format` | `-f, --format` |
| `plain` | `--plain` |
| `verbosity` | `-v` (repeatable) |
| `summary` | `--summary` |
| `compact` | `--compact` |
| `show_dependencies` | `--show-dependencies` |
| `show_splits` | `--show-splits` |
| `tiered` | `--tiered` |

**Precedence:** CLI flags override config file settings.

## See Also

- [Output Formats](../output-formats.md) - Detailed format documentation
- [Scoring Configuration](scoring.md) - Configure scoring weights
- [CLI Reference](../cli-reference.md) - Command-line options
