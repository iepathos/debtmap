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
max_function_length = 80

[thresholds.validation]
min_coverage_percentage = 80.0

[languages]
enabled = ["rust", "python", "javascript", "typescript"]

[languages.rust]
detect_dead_code = false

[languages.python]
detect_dead_code = true
```

## Configuration Topics

- [Scoring Configuration](scoring.md) - Tune debt scoring weights and role multipliers
- [Thresholds Configuration](thresholds.md) - Set complexity and coverage thresholds
- [Language Configuration](languages.md) - Enable/disable language support and tune language-specific settings
- [Display and Output](display-output.md) - Configure output formats and display options
- [Advanced Options](advanced.md) - Advanced configuration for power users
- [Best Practices](best-practices.md) - Guidelines for effective configuration

## Configuration Sources

Debtmap layers configuration from lowest to highest precedence:

1. Built-in defaults
2. User config (`~/.config/debtmap/config.toml`, or the platform equivalent)
3. The nearest `.debtmap.toml` found from the current directory
4. A custom path from `--config` or `DEBTMAP_CONFIG`
5. Supported `DEBTMAP_*` field overrides

Higher-precedence files replace only the top-level sections they define; other sections continue
to come from lower-precedence sources. Within a replaced section, omitted values use that
section's defaults rather than values from the lower-precedence section. Use
`debtmap --show-config-sources analyze .` to inspect the effective source order.
Run `debtmap config check` to reject unknown keys and invalid values before analysis or in CI.

## Validation

Debtmap validates every discovered configuration before analysis. A malformed or invalid file
stops the command instead of silently falling back to a weaker configuration:

```bash
$ debtmap analyze .
Error: Invalid configuration
  - scoring.coverage + scoring.complexity + scoring.dependency must equal 1.0
  - Current sum: 1.10
```

## Default Values

All configuration options have sensible defaults. You only need to specify values you want to override from the defaults documented in each section.
