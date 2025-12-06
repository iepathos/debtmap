# Debtmap Configuration Guide

This guide explains how to configure debtmap for your project using configuration files, presets, and command-line options.

## Table of Contents

- [Quick Start](#quick-start)
- [Configuration File Locations](#configuration-file-locations)
- [Configuration Hierarchy](#configuration-hierarchy)
- [Configuration Presets](#configuration-presets)
- [Configuration Sections](#configuration-sections)
- [Examples](#examples)
- [Migration from CLI Flags](#migration-from-cli-flags)

## Quick Start

### Using Defaults

The simplest way to use debtmap is with no configuration:

```bash
debtmap analyze src/
```

This uses built-in balanced defaults suitable for most Rust projects.

### Using a Preset

Start with a preset configuration for common scenarios:

```bash
# Strict mode - high code quality standards
debtmap analyze src/ --threshold-preset strict

# Balanced mode - recommended defaults (default)
debtmap analyze src/ --threshold-preset balanced

# Permissive mode - legacy or complex codebases
debtmap analyze src/ --threshold-preset permissive
```

### Creating a Project Configuration

Initialize a configuration file for your project:

```bash
# Create .debtmap.toml with example configuration
debtmap init

# Or copy from examples
cp .debtmap.example.toml .debtmap.toml
```

Then customize `.debtmap.toml` for your project's needs.

## Configuration File Locations

Debtmap looks for configuration in multiple locations, in order of precedence:

### 1. Built-in Defaults (Lowest Priority)

Hard-coded defaults for all settings.

### 2. User Configuration

Global user settings at `~/.config/debtmap/config.toml` (Unix/Linux/macOS).

Example use case: Set your preferred output format and verbosity across all projects.

```toml
# ~/.config/debtmap/config.toml
[display]
format = "json"
verbosity = 2
```

### 3. Project Configuration

Project-specific settings in `.debtmap.toml` in your project root (or any parent directory).

Example use case: Define thresholds and rules specific to your codebase.

```toml
# .debtmap.toml (in project root)
[thresholds]
complexity = 50
duplication = 10

[ignore]
patterns = ["generated/**", "vendor/**"]
```

### 4. Environment Variables

Environment variables override file-based configuration.

```bash
export DEBTMAP_CONFIG=/path/to/custom/config.toml
export DEBTMAP_THRESHOLD_PRESET=strict
```

### 5. CLI Arguments (Highest Priority)

Command-line flags override all other sources.

```bash
debtmap analyze src/ --threshold-complexity 30 --format json
```

## Configuration Hierarchy

Configuration is loaded and merged in this order:

```
Built-in Defaults
  ↓ (merged with)
User Config (~/.config/debtmap/config.toml)
  ↓ (merged with)
Project Config (.debtmap.toml)
  ↓ (merged with)
Environment Variables
  ↓ (merged with)
CLI Arguments (final overrides)
```

**Important**: Later sources override earlier ones. For example, a CLI flag will override the same setting in `.debtmap.toml`.

## Configuration Presets

Debtmap provides three built-in presets for common scenarios:

### Strict Preset

**Use for**: New projects, critical systems, high-reliability software

**Characteristics**:
- Low complexity thresholds (flags issues early)
- High coverage emphasis
- Comprehensive validation checks
- Verbose output

**Example**:
```bash
debtmap analyze src/ --threshold-preset strict
```

**Config file equivalent**:
```toml
[thresholds]
complexity = 20
duplication = 5

[scoring]
coverage = 0.55
complexity = 0.30
dependency = 0.15
```

See [`examples/config-strict.toml`](../examples/config-strict.toml) for full configuration.

### Balanced Preset (Default)

**Use for**: Most projects, standard development

**Characteristics**:
- Reasonable thresholds for typical codebases
- Balanced scoring weights
- Standard validation
- Normal verbosity

**Example**:
```bash
debtmap analyze src/  # Balanced is the default
```

**Config file equivalent**:
```toml
[thresholds]
complexity = 50
duplication = 10

[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15
```

### Permissive Preset

**Use for**: Legacy codebases, complex business logic, gradual migration

**Characteristics**:
- High complexity thresholds (focuses on critical issues)
- Lower coverage requirements
- Optional validation checks
- Focus on actionable issues

**Example**:
```bash
debtmap analyze src/ --threshold-preset permissive
```

**Config file equivalent**:
```toml
[thresholds]
complexity = 100
duplication = 20

[scoring]
coverage = 0.40
complexity = 0.40
dependency = 0.20
```

See [`examples/config-permissive.toml`](../examples/config-permissive.toml) for full configuration.

### Customizing Presets

Start with a preset and override specific values:

```bash
# Use strict preset but with custom complexity threshold
debtmap analyze src/ --threshold-preset strict --threshold-complexity 30
```

Or in a config file:

```toml
# .debtmap.toml
# Start with strict preset and customize
[thresholds]
complexity = 30  # Override just this value
```

## Configuration Sections

### `[thresholds]` - Complexity Thresholds

Controls when functions are flagged for complexity issues.

```toml
[thresholds]
complexity = 50              # Cyclomatic complexity threshold
duplication = 10             # Duplication threshold (lines)
max_file_length = 1000       # Maximum file length (lines)
max_function_length = 50     # Maximum function length (lines)
max_function_params = 4      # Maximum function parameters
max_nesting_depth = 4        # Maximum nesting depth
```

### `[scoring]` - Scoring Weights

Controls how different factors contribute to debt scores.

**Important**: All active weights must sum to 1.0.

```toml
[scoring]
coverage = 0.50      # Test coverage weight
complexity = 0.35    # Complexity weight
dependency = 0.15    # Dependency weight
semantic = 0.00      # Semantic weight (inactive)
security = 0.00      # Security weight (inactive)
organization = 0.00  # Organization weight (inactive)
```

### `[display]` - Output Configuration

Controls how results are displayed.

```toml
[display]
format = "terminal"           # Output format (json|markdown|terminal|html)
verbosity = 1                 # Verbosity level (0-3)
compact = false               # Use compact output
summary = false               # Use summary format
group_by_category = false     # Group by debt category
detail_level = "standard"     # Detail level (summary|standard|comprehensive|debug)
```

### `[filters]` - Result Filtering

Controls which issues are shown in the output.

```toml
[filters]
min_score = 3.0                # Minimum score threshold
min_priority = "medium"        # Minimum priority (low|medium|high|critical)
filter_categories = ["complexity", "duplication"]  # Category filter
```

### `[performance]` - Performance Settings

Controls parallel processing and optimization.

```toml
[performance]
parallel = true      # Enable parallel processing
jobs = 0             # Thread count (0 = all cores)
multi_pass = true    # Enable multi-pass analysis
```

### `[ignore]` - Ignore Patterns

Specify files and directories to exclude from analysis.

```toml
[ignore]
patterns = [
    "tests/**/*",
    "examples/**/*",
    "benches/**/*",
    "**/target/**",
    "**/node_modules/**",
]
```

## Examples

### Example 1: Minimal Configuration

```toml
# .debtmap.toml
[thresholds]
complexity = 40

[display]
format = "json"
```

### Example 2: Comprehensive Configuration

```toml
# .debtmap.toml
version = "1.0"

[thresholds]
complexity = 50
duplication = 10
max_file_length = 800

[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15

[display]
format = "terminal"
verbosity = 2
group_by_category = true

[filters]
min_score = 3.0

[performance]
parallel = true
jobs = 4

[ignore]
patterns = [
    "tests/**/*",
    "generated/**/*",
]
```

### Example 3: CI/CD Configuration

```toml
# .debtmap.toml - For CI/CD validation
[thresholds]
complexity = 50
duplication = 10

[thresholds.validation]
max_average_complexity = 10.0
max_debt_density = 50.0
max_codebase_risk_score = 7.0

[display]
format = "json"
quiet = true

[filters]
min_priority = "high"  # Only show high and critical issues
```

### Example 4: User Global Preferences

```toml
# ~/.config/debtmap/config.toml
# Global preferences for all projects

[display]
format = "terminal"
verbosity = 2
plain = false  # Enable colors

[performance]
jobs = 8  # Use 8 threads on my machine
```

## Migration from CLI Flags

If you're currently using CLI flags, here's how to migrate to configuration files:

### Before (CLI flags):

```bash
debtmap analyze src/ \
  --threshold-complexity 50 \
  --threshold-duplication 10 \
  --format json \
  --output results.json \
  --verbose \
  --parallel \
  --jobs 4
```

### After (Configuration file):

```toml
# .debtmap.toml
[thresholds]
complexity = 50
duplication = 10

[output]
path = "results.json"

[display]
format = "json"
verbosity = 2

[performance]
parallel = true
jobs = 4
```

```bash
# Now just run:
debtmap analyze src/
```

### Benefits of Configuration Files

1. **Version Control**: Configuration is tracked with your code
2. **Team Consistency**: Everyone uses the same settings
3. **Readability**: Easier to understand than long CLI commands
4. **Maintainability**: Change settings without updating scripts
5. **Documentation**: Config file serves as self-documentation

### Gradual Migration

You can migrate gradually by mixing config files and CLI flags:

```bash
# Use config file as base, override specific values
debtmap analyze src/ --threshold-complexity 30
```

CLI flags always take precedence over config file settings.

## Advanced Topics

### Environment Variable Overrides

All settings can be overridden with environment variables:

```bash
# Override config file path
export DEBTMAP_CONFIG=/path/to/config.toml

# Override specific threshold
export DEBTMAP_THRESHOLD_COMPLEXITY=60

# Use in CI/CD
DEBTMAP_FORMAT=json debtmap analyze src/
```

### Viewing Active Configuration

To see which configuration values are active and where they came from:

```bash
debtmap analyze src/ --show-config-sources
```

This shows each setting and its source (default, user config, project config, CLI, etc.).

### Validating Configuration

Test your configuration without running analysis:

```bash
# The init command validates the config file
debtmap init --validate
```

## Best Practices

1. **Start with a preset**: Begin with `balanced`, adjust as needed
2. **Commit `.debtmap.toml`**: Keep config in version control
3. **Document overrides**: Add comments explaining custom settings
4. **Use user config for preferences**: Keep personal preferences in `~/.config/debtmap/config.toml`
5. **Test in CI**: Validate configuration in CI/CD pipelines
6. **Gradual strictness**: Start permissive, tighten over time
7. **Team review**: Discuss threshold changes with the team

## Troubleshooting

### Configuration Not Loading

```bash
# Check which config file is being used
debtmap analyze src/ --show-config-sources
```

### Weights Don't Sum to 1.0

Debtmap will warn if scoring weights don't sum to 1.0 and will normalize them automatically. Update your config:

```toml
[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15  # Total = 1.0 ✓
```

### Config File Syntax Errors

If your TOML file has syntax errors, debtmap will print a clear error message:

```
Error: Failed to parse .debtmap.toml: invalid TOML syntax at line 15
```

Use a TOML validator or editor plugin to catch syntax errors.

## See Also

- [Example Configuration Files](../examples/)
- [Preset Configurations](../examples/config-strict.toml)
- [CLI Reference](./CLI.md)
- [Scoring Guide](./SCORING.md)
