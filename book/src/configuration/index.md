# Configuration

Debtmap is highly configurable through a TOML configuration file. This section covers all configuration options and best practices for tuning debtmap for your codebase.

## Quick Start

Create a `.debtmap.toml` file in your project root:

```toml
[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15

[thresholds]
complexity = 15
lines = 80
coverage = 0.8

[languages]
rust = true
python = true
javascript = true
```

## Configuration Topics

- [Scoring Configuration](scoring.md) - Tune debt scoring weights and role multipliers
- [Thresholds Configuration](thresholds.md) - Set complexity and coverage thresholds
- [Language Configuration](languages.md) - Enable/disable language support and tune language-specific settings
- [Display and Output](display-output.md) - Configure output formats and display options
- [Advanced Options](advanced.md) - Advanced configuration for power users
- [Best Practices](best-practices.md) - Guidelines for effective configuration

## Configuration File Location

Debtmap searches for configuration in the following order:

1. Path specified with `--config` flag
2. `.debtmap.toml` in current directory
3. `.debtmap.toml` in git repository root
4. Built-in defaults

## Validation

Debtmap validates your configuration on startup. Invalid configurations will produce clear error messages:

```bash
$ debtmap analyze .
Error: Invalid configuration
  - scoring.coverage + scoring.complexity + scoring.dependency must equal 1.0
  - Current sum: 1.10
```

## Default Values

All configuration options have sensible defaults. You only need to specify values you want to override from the defaults documented in each section.
