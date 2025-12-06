---
number: 205
title: CLI Parameter Grouping and Hierarchical Configuration
category: optimization
priority: medium
status: draft
dependencies: [204]
created: 2025-12-06
---

# Specification 205: CLI Parameter Grouping and Hierarchical Configuration

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 204 (Refactor build_analyze_config)

## Context

The `analyze` command currently has 65+ flat command-line parameters, making it difficult for users to understand, discover, and correctly use features. This flat structure also creates maintenance challenges as every new feature adds another top-level flag.

Current CLI structure:
```bash
debtmap analyze <path> \
  --threshold-complexity 50 \
  --threshold-duplication 10 \
  --threshold-preset strict \
  --public-api-threshold 0.5 \
  --format json \
  --output results.json \
  --verbose \
  --compact \
  --no-color \
  --show-dependencies \
  --max-callers 10 \
  --max-callees 10 \
  # ... 50+ more flags
```

**Problems:**
1. **Discoverability**: Users can't find related options
2. **Usability**: Hard to remember 65+ flag names
3. **Documentation**: Help text is overwhelming
4. **Maintenance**: Adding features increases complexity
5. **Validation**: Cross-parameter validation is difficult

This spec proposes grouping related parameters into logical subcommands or structured configuration, informed by the configuration groups created in spec 204.

## Objective

Redesign the CLI parameter structure for the `analyze` command to use logical grouping, improving usability and maintainability while maintaining backward compatibility through aliases and migration helpers.

Goals:
- Group 65+ flat parameters into 8-10 logical groups
- Support hierarchical configuration (CLI, config files, env vars)
- Maintain backward compatibility with existing scripts
- Improve help text organization and discoverability
- Enable configuration presets and profiles

## Requirements

### Functional Requirements

1. **Hierarchical Parameter Structure**
   - Group related flags into subgroups (thresholds, display, features)
   - Support both flat (backward compat) and grouped (new) syntax
   - Allow configuration via CLI, config files, and environment variables
   - Enable preset configurations (e.g., `--preset strict`)

2. **Configuration File Support**
   - TOML-based configuration file format
   - Support project-level `.debtmap.toml` files
   - User-level `~/.config/debtmap/config.toml`
   - Override order: CLI flags > project config > user config > defaults

3. **Backward Compatibility**
   - All existing flags continue to work
   - Deprecated flags show migration hints
   - Graceful transition period (2-3 major versions)
   - Clear documentation of migration path

4. **Configuration Presets**
   - Built-in presets (strict, balanced, permissive)
   - Custom preset support via config files
   - Preset overrides (e.g., `--preset strict --threshold-complexity 30`)
   - Preset documentation and examples

### Non-Functional Requirements

1. **Usability**
   - Grouped help text that's easier to scan
   - Logical parameter organization
   - Clear parameter naming conventions
   - Helpful error messages for invalid combinations

2. **Maintainability**
   - Easy to add new parameters to existing groups
   - Clear separation of concerns
   - Type-safe configuration validation
   - Reduced parameter passing complexity

3. **Documentation**
   - Comprehensive examples for common use cases
   - Migration guide for existing users
   - Configuration file reference documentation
   - Preset descriptions and recommendations

## Acceptance Criteria

- [ ] TOML configuration file format defined and documented
- [ ] Configuration loading supports CLI > project > user > defaults hierarchy
- [ ] All 65+ existing flags continue to work (backward compatibility)
- [ ] Grouped help text implemented with logical sections
- [ ] Configuration presets (strict, balanced, permissive) implemented
- [ ] Preset override mechanism works correctly
- [ ] Migration guide written for existing users
- [ ] Configuration file examples provided
- [ ] All existing tests pass with new structure
- [ ] New tests for configuration file loading
- [ ] Deprecated flag warnings display migration hints

## Technical Details

### Implementation Approach

#### Proposed CLI Structure

**Option A: Structured Flags (Backward Compatible)**

```bash
# New grouped syntax (preferred)
debtmap analyze <path> \
  --threshold.complexity 50 \
  --threshold.duplication 10 \
  --threshold.preset strict \
  --display.format json \
  --display.verbosity 2 \
  --display.compact \
  --output.path results.json

# Old flat syntax (still supported via aliases)
debtmap analyze <path> \
  --threshold-complexity 50 \
  --format json \
  --output results.json
```

**Option B: Configuration Subcommands**

```bash
# Configuration via subgroups
debtmap analyze <path> \
  threshold --complexity 50 --duplication 10 --preset strict \
  display --format json --verbosity 2 \
  output --path results.json
```

**Option C: Config File First (Recommended)**

```bash
# Primary: Use configuration file
debtmap analyze <path> --config .debtmap.toml

# Override specific values
debtmap analyze <path> --config .debtmap.toml --threshold-complexity 30

# Fall back to CLI flags
debtmap analyze <path> --threshold-complexity 50 --format json
```

**Recommendation**: Option C with gradual migration to dotted syntax.

#### Configuration File Format

`.debtmap.toml`:

```toml
# Analysis configuration for debtmap
version = "1.0"

[paths]
output = "analysis-results.json"
coverage_file = "coverage.xml"
max_files = 1000

[thresholds]
complexity = 50
duplication = 10
preset = "strict"
public_api_threshold = 0.5

[analysis.features]
enable_context = true
context_providers = ["git", "github"]
semantic_off = false
no_pattern_detection = false
patterns = ["singleton", "factory"]
pattern_threshold = 0.7
ast_functional_analysis = true
validate_loc = true

[display]
format = "json"
verbosity = 2
compact = false
summary = true
group_by_category = true
show_attribution = true
detail_level = "normal"
no_tui = false

[display.formatting]
plain = false
show_dependencies = true
max_callers = 10
max_callees = 10
show_external = false
show_std_lib = false
show_splits = true

[filters]
min_priority = "medium"
min_score = 5.0
filter_categories = ["complexity", "duplication"]
min_problematic = 3

[performance]
parallel = true
jobs = 4
multi_pass = true
aggregate_only = false

[debug]
verbose_macro_warnings = false
show_macro_stats = false
debug_call_graph = false
trace_functions = []
call_graph_stats_only = false
show_pattern_warnings = true

[languages]
enabled = ["rust", "python", "javascript"]
aggregation_method = "weighted"
```

#### Configuration Presets

**Built-in Presets:**

```rust
// src/config/presets.rs

pub enum PresetLevel {
    Strict,
    Balanced,
    Permissive,
}

impl PresetLevel {
    pub fn to_config(&self) -> PartialConfig {
        match self {
            PresetLevel::Strict => PartialConfig {
                thresholds: ThresholdConfig {
                    complexity: 20,
                    duplication: 5,
                    public_api_threshold: 0.8,
                    ..Default::default()
                },
                analysis: AnalysisFeatureConfig {
                    validate_loc: true,
                    validate_call_graph: true,
                    ast_functional_analysis: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            PresetLevel::Balanced => PartialConfig {
                thresholds: ThresholdConfig {
                    complexity: 50,
                    duplication: 10,
                    public_api_threshold: 0.5,
                    ..Default::default()
                },
                ..Default::default()
            },
            PresetLevel::Permissive => PartialConfig {
                thresholds: ThresholdConfig {
                    complexity: 100,
                    duplication: 20,
                    public_api_threshold: 0.3,
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}
```

#### Configuration Loading Hierarchy

```rust
// src/config/loader.rs

pub struct ConfigLoader {
    // Loads configuration with precedence:
    // CLI > Project (.debtmap.toml) > User (~/.config/debtmap/config.toml) > Defaults
}

impl ConfigLoader {
    pub fn load(cli_args: CliArgs) -> Result<AnalyzeConfig> {
        // 1. Start with defaults
        let mut config = AnalyzeConfig::default();

        // 2. Load user-level config if exists
        if let Some(user_config) = Self::load_user_config()? {
            config = config.merge(user_config);
        }

        // 3. Load project-level config if exists
        if let Some(project_config) = Self::load_project_config()? {
            config = config.merge(project_config);
        }

        // 4. Load explicit config file if specified
        if let Some(config_path) = cli_args.config_file {
            let file_config = Self::load_config_file(&config_path)?;
            config = config.merge(file_config);
        }

        // 5. Apply CLI overrides (highest priority)
        config = config.merge_cli_args(cli_args);

        // 6. Validate final configuration
        config.validate()?;

        Ok(config)
    }

    fn load_user_config() -> Result<Option<PartialConfig>> {
        let path = dirs::config_dir()
            .map(|p| p.join("debtmap/config.toml"));

        match path {
            Some(p) if p.exists() => Ok(Some(Self::parse_config_file(&p)?)),
            _ => Ok(None),
        }
    }

    fn load_project_config() -> Result<Option<PartialConfig>> {
        let path = std::env::current_dir()?
            .join(".debtmap.toml");

        if path.exists() {
            Ok(Some(Self::parse_config_file(&path)?))
        } else {
            Ok(None)
        }
    }

    fn parse_config_file(path: &Path) -> Result<PartialConfig> {
        let contents = std::fs::read_to_string(path)?;
        let config: PartialConfig = toml::from_str(&contents)
            .context("Failed to parse configuration file")?;
        Ok(config)
    }
}
```

#### Backward Compatibility Strategy

**Phase 1: Dual Support (Current → v2.0)**
- Support both old flat flags and new grouped syntax
- No deprecation warnings
- Document new syntax in examples

**Phase 2: Soft Deprecation (v2.0 → v3.0)**
- Warn when using old flags: "Flag --threshold-complexity is deprecated, use --threshold.complexity or config file"
- Show migration hint in warning
- Continue full support

**Phase 3: Hard Deprecation (v3.0+)**
- Remove old flat flags
- Provide migration tool: `debtmap migrate-config`
- Only support grouped syntax and config files

```rust
// Backward compatibility aliases
impl Cli {
    fn parse_with_compat() -> Self {
        let mut cli = Cli::parse();

        // Map old flags to new structure
        if let Some(old_threshold) = cli.threshold_complexity_old {
            if cli.threshold.is_none() {
                cli.threshold = Some(ThresholdConfig::default());
            }
            cli.threshold.as_mut().unwrap().complexity = old_threshold;

            // Show deprecation warning
            eprintln!(
                "Warning: --threshold-complexity is deprecated. \
                 Use --threshold.complexity or configure in .debtmap.toml"
            );
        }

        cli
    }
}
```

### Architecture Changes

**Before:**
```
main.rs
  ├─ Cli (65+ flat fields)
  ├─ handle_analyze_command (destructure 65 params)
  └─ build_analyze_config (60+ params)
```

**After:**
```
main.rs
  ├─ Cli (grouped structure)
  │   ├─ PathGroup (4 fields)
  │   ├─ ThresholdGroup (4 fields)
  │   ├─ AnalysisGroup (15 fields)
  │   └─ ... (5 more groups)
  ├─ ConfigLoader
  │   ├─ load_user_config()
  │   ├─ load_project_config()
  │   ├─ load_config_file()
  │   └─ merge_with_cli()
  └─ handle_analyze_command (8 grouped params from spec 204)
```

### Data Structures

```rust
// src/cli.rs (updated)

#[derive(Parser, Debug)]
#[command(name = "debtmap")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Analyze {
        /// Path to analyze
        path: PathBuf,

        /// Configuration file
        #[arg(long, short)]
        config: Option<PathBuf>,

        /// Configuration preset
        #[arg(long, value_enum)]
        preset: Option<PresetLevel>,

        // Grouped parameters (new)
        #[command(flatten)]
        paths: Option<PathGroup>,

        #[command(flatten)]
        thresholds: Option<ThresholdGroup>,

        #[command(flatten)]
        display: Option<DisplayGroup>,

        // ... other groups

        // Backward compatibility (deprecated)
        #[arg(long, hide = true)]
        threshold_complexity: Option<u32>,

        #[arg(long, hide = true)]
        format: Option<OutputFormat>,

        // ... other deprecated flat flags
    },
    // ... other commands
}

// Clap argument groups
#[derive(Args, Debug)]
#[group(id = "thresholds")]
pub struct ThresholdGroup {
    #[arg(long = "threshold.complexity")]
    pub complexity: Option<u32>,

    #[arg(long = "threshold.duplication")]
    pub duplication: Option<usize>,

    #[arg(long = "threshold.preset")]
    pub preset: Option<String>,

    #[arg(long = "threshold.public-api")]
    pub public_api_threshold: Option<f32>,
}
```

### Configuration Merging

```rust
// src/config/merge.rs

pub trait ConfigMerge {
    fn merge(self, other: Self) -> Self;
}

impl ConfigMerge for AnalyzeConfig {
    fn merge(self, other: Self) -> Self {
        AnalyzeConfig {
            // Use 'other' value if Some, else keep 'self'
            path: other.path,
            threshold_complexity: other.threshold_complexity
                .or(Some(self.threshold_complexity))
                .unwrap(),
            output: other.output.or(self.output),
            // ... merge all fields
        }
    }
}

impl ConfigMerge for PartialConfig {
    fn merge(mut self, other: PartialConfig) -> Self {
        if let Some(t) = other.thresholds {
            self.thresholds = Some(
                self.thresholds.unwrap_or_default().merge(t)
            );
        }
        // ... merge all groups
        self
    }
}
```

## Dependencies

- **Prerequisites**: Spec 204 (provides configuration group structures)
- **Affected Components**:
  - `src/cli.rs` - CLI argument parsing
  - `src/main.rs` - Command handling
  - `src/config/` - New module for config loading
  - Documentation - Extensive updates
- **External Dependencies**:
  - `toml` crate for TOML parsing
  - `dirs` crate for user config directory
  - `serde` for serialization

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_merge_cli_overrides_file() {
        let file_config = PartialConfig {
            thresholds: Some(ThresholdConfig {
                complexity: 50,
                ..Default::default()
            }),
            ..Default::default()
        };

        let cli_config = PartialConfig {
            thresholds: Some(ThresholdConfig {
                complexity: 30,
                ..Default::default()
            }),
            ..Default::default()
        };

        let merged = file_config.merge(cli_config);
        assert_eq!(merged.thresholds.unwrap().complexity, 30);
    }

    #[test]
    fn config_loader_precedence() {
        // Test: CLI > Project > User > Defaults
        // ... comprehensive precedence testing
    }

    #[test]
    fn backward_compat_flags_still_work() {
        let cli = Cli::parse_from(vec![
            "debtmap",
            "analyze",
            "src",
            "--threshold-complexity",
            "50",
        ]);

        // Should map to new structure
        assert_eq!(
            cli.command.thresholds.unwrap().complexity,
            Some(50)
        );
    }
}
```

### Integration Tests

```rust
#[test]
fn config_file_loading() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join(".debtmap.toml");

    std::fs::write(
        &config_path,
        r#"
        [thresholds]
        complexity = 40
        duplication = 8
        "#,
    )
    .unwrap();

    let config = ConfigLoader::load_config_file(&config_path).unwrap();
    assert_eq!(config.thresholds.unwrap().complexity, 40);
}

#[test]
fn preset_application() {
    let preset = PresetLevel::Strict.to_config();
    assert_eq!(preset.thresholds.unwrap().complexity, 20);
}
```

## Documentation Requirements

### Migration Guide

Create `docs/MIGRATION_CONFIG.md`:

```markdown
# Configuration Migration Guide

## Migrating from Flat Flags to Configuration Files

### Old Approach (Still Supported)
```bash
debtmap analyze src \
  --threshold-complexity 50 \
  --threshold-duplication 10 \
  --format json \
  --output results.json
```

### New Approach (Recommended)

Create `.debtmap.toml`:
```toml
[thresholds]
complexity = 50
duplication = 10

[display]
format = "json"

[paths]
output = "results.json"
```

Then run:
```bash
debtmap analyze src
```

## Automatic Migration Tool

```bash
# Convert your existing command to config file
debtmap migrate-config --from-flags "debtmap analyze src --threshold-complexity 50 ..." > .debtmap.toml
```
```

### Configuration Reference

Create `docs/CONFIGURATION.md`:

```markdown
# Configuration Reference

## Configuration File Locations

1. User config: `~/.config/debtmap/config.toml` (global defaults)
2. Project config: `.debtmap.toml` (project-specific)
3. Explicit config: `--config custom.toml` (override)

## Configuration Sections

### [paths]
- `output`: Output file path
- `coverage_file`: Coverage data file
- `max_files`: Maximum files to analyze

### [thresholds]
- `complexity`: Complexity threshold (default: 50)
- `duplication`: Duplication threshold (default: 10)
- `preset`: Preset level (strict|balanced|permissive)

... (document all sections)
```

## Implementation Notes

### Refactoring Steps

1. **Add TOML dependencies** to `Cargo.toml`
2. **Create `src/config/` module** with loader, merger, presets
3. **Update `src/cli.rs`** with grouped parameter structures
4. **Implement ConfigLoader** with hierarchy support
5. **Add backward compatibility** aliases and warnings
6. **Update command handlers** to use ConfigLoader
7. **Write comprehensive tests** for all config scenarios
8. **Document migration path** and configuration reference
9. **Create example config files**

### Common Pitfalls

1. **Merge order bugs** - Test precedence thoroughly
2. **Partial config handling** - Some fields may be None
3. **Validation timing** - Validate after all merging
4. **Path resolution** - Handle relative paths correctly
5. **Default values** - Ensure defaults match original behavior

## Migration and Compatibility

### Breaking Changes

**Phase 1 (v1.x → v2.0)**: None (additive only)
**Phase 2 (v2.0 → v3.0)**: Deprecation warnings
**Phase 3 (v3.0+)**: Remove old flat flags

### Migration Steps

**For Users:**
1. Continue using old flags (v1.x - v2.x)
2. Create `.debtmap.toml` for new projects (v2.0+)
3. Run `debtmap migrate-config` to convert old flags (v2.0+)
4. Update scripts to use config files before v3.0

**For Maintainers:**
1. Implement dual support system (this spec)
2. Monitor usage telemetry for old vs new flags
3. Communicate deprecation timeline
4. Provide migration tooling
5. Remove old code in v3.0

## Success Metrics

- ✅ Configuration file format documented
- ✅ Config loading hierarchy implemented (CLI > Project > User > Defaults)
- ✅ All 65+ existing flags continue to work
- ✅ Presets (strict, balanced, permissive) work correctly
- ✅ Migration guide published
- ✅ Example config files in repository
- ✅ All tests pass
- ✅ Zero breaking changes in initial release

## Follow-up Work

After this implementation:

- **Spec 206**: Command handler type system improvements
- Add configuration validation rules
- Implement configuration schema documentation
- Create interactive config builder tool
- Add telemetry for configuration usage patterns

## References

- **Spec 204**: Refactor build_analyze_config (provides config groups)
- **Spec 182**: Refactor handle_analyze_command
- **Clap Documentation**: Argument grouping and flattening
- **TOML Specification**: Configuration file format
- **XDG Base Directory**: Standard config locations
