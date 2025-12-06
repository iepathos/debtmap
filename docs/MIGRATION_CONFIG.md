# Configuration Migration Guide

This guide helps you transition from CLI-only configuration to using `.debtmap.toml` configuration files.

## Overview

Debtmap now supports hierarchical configuration through config files, reducing the need for long command-line arguments. The configuration hierarchy is:

1. **CLI arguments** (highest priority) - Override everything
2. **Project config** (`./.debtmap.toml`) - Project-specific settings
3. **User config** (`~/.config/debtmap/config.toml`) - Your personal defaults
4. **Built-in defaults** (lowest priority) - Sensible defaults

## Why Use Config Files?

### Before (CLI flags only):
```bash
debtmap analyze src/ \
  --threshold-complexity 20 \
  --threshold-duplication 10 \
  --min-score 3.0 \
  --coverage-file coverage.lcov \
  --enable-context \
  --context-providers critical_path,dependency \
  --no-god-object \
  --ast-functional-analysis \
  --functional-analysis-profile strict \
  --format json \
  --output analysis.json
```

### After (with config file):
```bash
# Create .debtmap.toml once
debtmap init

# Then just run:
debtmap analyze src/
```

## Migration Steps

### Step 1: Initialize Configuration

Create a `.debtmap.toml` file in your project root:

```bash
debtmap init
```

This creates a commented template with all available options.

### Step 2: Convert Your Common Flags

Take your most frequently used CLI flags and add them to `.debtmap.toml`:

#### Threshold Settings

**Before:**
```bash
--threshold-complexity 20 --threshold-duplication 10
```

**After (in .debtmap.toml):**
```toml
[thresholds]
complexity = 20
duplication = 10
min_score_threshold = 3.0
```

#### Or Use Presets

Instead of setting individual thresholds, use a preset:

**CLI:**
```bash
--threshold-preset strict
```

**Config:**
```toml
# No config needed - use CLI flag when you need different presets
# Or set a default preset in config (coming in future version)
```

Available presets:
- `strict`: Low complexity thresholds (complexity=20, min_score=2.0)
- `balanced`: Default recommended settings (complexity=50, min_score=3.0)
- `lenient`: High thresholds for legacy code (complexity=100, min_score=5.0)

#### Coverage and Context

**Before:**
```bash
--coverage-file coverage.lcov --enable-context --context-providers critical_path,dependency
```

**After:**
```toml
[coverage]
file = "coverage.lcov"

[context]
enabled = true
providers = ["critical_path", "dependency"]
```

#### Analysis Features

**Before:**
```bash
--ast-functional-analysis --functional-analysis-profile strict --no-god-object
```

**After:**
```toml
[functional_analysis]
enabled = true
profile = "strict"

[god_object_detection]
enabled = false
```

#### Output Settings

**Before:**
```bash
--format json --output analysis.json
```

**After:**
```toml
[output]
format = "json"
path = "analysis.json"
```

### Step 3: Keep CLI for Overrides

CLI flags still work and override config file values. Use them for:
- One-off analyses with different settings
- CI/CD pipeline variations
- Quick experiments

**Example:**
```bash
# Use config defaults but override output format
debtmap analyze src/ --format markdown
```

## Complete Configuration Example

Here's a production-ready `.debtmap.toml`:

```toml
# Project-wide code quality standards
[thresholds]
complexity = 30
duplication = 15
min_score_threshold = 3.0
max_file_length = 800
max_function_length = 60

# Coverage-based prioritization
[coverage]
file = "coverage.lcov"

# Context-aware analysis
[context]
enabled = true
providers = ["critical_path", "dependency", "git_history"]

# Functional programming analysis (for Rust projects)
[functional_analysis]
enabled = true
profile = "balanced"

# God object detection
[god_object_detection]
enabled = true
min_methods = 15
min_lines = 200

# Display preferences
[display]
verbosity = 1
group_by_category = true
show_filter_stats = true

# Output format
[output]
format = "json"
path = ".debtmap/analysis.json"

# Performance settings
[performance]
parallel = true
jobs = 0  # Use all cores
```

## Configuration Discovery

To see where each setting is coming from:

```bash
debtmap analyze src/ --show-config-sources
```

This shows the value and source (CLI, project config, user config, or default) for each setting.

## Backwards Compatibility

**All existing CLI flags continue to work!** You can:
- Keep using CLI flags only (no config file needed)
- Mix config files and CLI flags
- Gradually migrate one setting at a time

### Deprecated Flags

The following flags are deprecated but still work:
- `--explain-score`: Use `-v` or `-vv` instead for verbosity

Deprecated flags will show a warning with migration hints.

## User-Level Defaults

Create `~/.config/debtmap/config.toml` for your personal defaults across all projects:

```toml
# Your personal preferences
[display]
verbosity = 1
plain = false  # Enable colors

[performance]
jobs = 8  # Always use 8 cores on your machine
```

Project configs override user configs, so team settings take precedence.

## Environment Variables

Config values can reference environment variables:

```toml
[coverage]
file = "${COVERAGE_FILE}"  # Uses $COVERAGE_FILE env var
```

## Validation

Validate your config file:

```bash
debtmap validate . --config .debtmap.toml
```

This checks for:
- Invalid TOML syntax
- Unknown configuration keys
- Invalid value types
- Conflicts between settings

## Automatic Migration Tool (Future)

We're planning an automatic migration tool:

```bash
# Future feature (not yet available)
debtmap migrate-config --from-history
```

This will analyze your shell history and suggest a config file based on your most common flags.

## FAQ

### Q: Do I need a config file?
**A:** No! CLI flags still work perfectly. Config files are optional for convenience.

### Q: Can I use multiple config files?
**A:** Yes! User config (`~/.config/debtmap/config.toml`) + project config (`./.debtmap.toml`) + CLI flags.

### Q: How do I override a config file setting?
**A:** Use the CLI flag - it has highest priority.

### Q: What if I have both `.debtmap.toml` and `.debtmap.yaml`?
**A:** TOML takes precedence. We recommend using TOML for better error messages.

### Q: Can I commit `.debtmap.toml` to git?
**A:** Yes! It's meant to be shared with your team to ensure consistent analysis standards.

### Q: How do I see the final merged configuration?
**A:** Use `--show-config-sources` to see the value and source of each setting.

## Getting Help

If you encounter issues during migration:
1. Run `debtmap analyze --help` to see all available flags
2. Use `--show-config-sources` to debug configuration
3. Check config syntax with `debtmap validate . --config .debtmap.toml`
4. Report issues at https://github.com/anthropics/debtmap/issues

## Related Documentation

- [Configuration Reference](./CONFIGURATION.md) - Complete config file syntax
- [CLI Reference](../book/src/cli-reference.md) - All command-line flags
- [Threshold Configuration](../book/src/threshold-configuration.md) - Understanding thresholds
