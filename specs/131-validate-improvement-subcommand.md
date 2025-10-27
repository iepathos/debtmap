---
number: 131
title: Convert validate-improvement binary to debtmap subcommand
category: foundation
priority: medium
status: draft
dependencies: []
created: 2025-01-26
---

# Specification 131: Convert validate-improvement binary to debtmap subcommand

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Currently, `prodigy-validate-debtmap-improvement` exists as a separate binary (`src/bin/prodigy-validate-debtmap-improvement.rs`) that validates technical debt improvements by analyzing comparison output from `debtmap compare`. This binary:

- Is only used by Prodigy workflow automation (workflows/debtmap.yml:31)
- Has a specialized interface using environment variables (`$ARGUMENTS`, `PRODIGY_AUTOMATION`)
- Duplicates CLI infrastructure (argument parsing, output handling)
- Is not discoverable via `debtmap --help`
- Adds ~330 lines of code in a separate binary

The validation logic performs useful analysis that could benefit interactive users:
- Calculates improvement scores based on target item, project health, and regressions
- Provides detailed gap analysis for incomplete improvements
- Tracks validation progress across multiple attempts
- Generates actionable feedback

## Objective

Convert the `prodigy-validate-debtmap-improvement` binary into a `debtmap validate-improvement` subcommand while:
1. Maintaining backward compatibility for Prodigy workflows via environment variable support
2. Providing a standard CLI interface for interactive use
3. Reusing existing CLI infrastructure (clap, output formatting)
4. Reducing code duplication and binary bloat
5. Making the command discoverable and documented

## Requirements

### Functional Requirements

1. **New Subcommand Structure**
   - Add `ValidateImprovement` variant to `Commands` enum in `src/cli.rs`
   - Implement standard CLI argument parsing using clap
   - Support both CLI arguments and environment variable fallback (for Prodigy)

2. **Argument Interface**
   - `--comparison <path>`: Path to comparison JSON file (required)
   - `--output <path>`: Path to write validation results (default: `.prodigy/debtmap-validation.json`)
   - `--previous-validation <path>`: Path to previous validation for progress tracking (optional)
   - `--threshold <number>`: Improvement threshold percentage (default: 75.0)
   - `--format <json|terminal|markdown>`: Output format (default: json)
   - `--quiet`: Suppress progress output (same as `PRODIGY_AUTOMATION=true`)

3. **Environment Variable Support** (Backward Compatibility)
   - If `PRODIGY_AUTOMATION=true` or `PRODIGY_VALIDATION=true`: Enable quiet mode
   - If `ARGUMENTS` env var exists: Parse as space-separated arguments (legacy support)
   - Priority: CLI args > `ARGUMENTS` env var > defaults

4. **Validation Logic Migration**
   - Extract validation logic into `src/commands/validate_improvement.rs` module
   - Create pure functions for:
     - Argument parsing and validation
     - Comparison result loading and analysis
     - Improvement score calculation
     - Gap identification and recommendation generation
     - Output formatting (JSON, terminal, markdown)
   - Maintain existing calculation formulas:
     ```
     improvement_score =
       target_component * 0.5 +           // 50%: Target item improved
       project_health_component * 0.3 +   // 30%: Overall debt improved
       no_regression_component * 0.2      // 20%: No new critical items
     ```

5. **Output Formats**
   - **JSON**: Machine-readable format for Prodigy (existing format)
   - **Terminal**: Human-readable colored output for interactive use
   - **Markdown**: Documentation-friendly format

6. **Progress Tracking**
   - Support `--previous-validation` to track improvement across attempts
   - Calculate trend direction (progress/stable/regression)
   - Provide actionable recommendations based on trend

### Non-Functional Requirements

1. **Performance**: No performance degradation compared to separate binary
2. **Maintainability**: Pure functional design with I/O at edges
3. **Testability**: Unit tests for all calculation functions
4. **Documentation**: Update book and CLI reference with new subcommand
5. **Backward Compatibility**: Existing Prodigy workflows continue working unchanged

## Acceptance Criteria

- [ ] `ValidateImprovement` subcommand added to `Commands` enum with appropriate fields
- [ ] `src/commands/validate_improvement.rs` module created with validation logic
- [ ] Command accepts standard CLI arguments (--comparison, --output, etc.)
- [ ] Command supports environment variable fallback (`ARGUMENTS`, `PRODIGY_AUTOMATION`)
- [ ] Validation logic extracted into pure, testable functions
- [ ] Composite score calculation matches existing formula (50/30/20 weights)
- [ ] JSON output format matches existing structure for Prodigy compatibility
- [ ] Terminal output format provides human-readable colored display
- [ ] Markdown output format suitable for documentation/reports
- [ ] Progress tracking supported via `--previous-validation` argument
- [ ] Quiet mode works via `--quiet` flag or `PRODIGY_AUTOMATION` env var
- [ ] Unit tests cover all calculation functions (>85% coverage)
- [ ] Integration test validates end-to-end workflow with sample data
- [ ] Existing test `tests/validate_integration_test.rs` updated and passing
- [ ] Binary definition removed from `Cargo.toml` [[bin]] section
- [ ] Binary file `src/bin/prodigy-validate-debtmap-improvement.rs` deleted
- [ ] Prodigy workflows updated to use `debtmap validate-improvement` instead
- [ ] Documentation updated (book, CLI reference, slash command docs)
- [ ] `debtmap validate-improvement --help` shows comprehensive usage
- [ ] Backward compatibility verified: existing workflows run unchanged

## Technical Details

### Implementation Approach

#### Phase 1: Create Command Module Structure

1. **Create `src/commands/validate_improvement.rs`**
   ```rust
   use anyhow::{Context, Result};
   use std::path::{Path, PathBuf};
   use crate::comparison::types::ComparisonResult;

   pub struct ValidateImprovementConfig {
       pub comparison_path: PathBuf,
       pub output_path: PathBuf,
       pub previous_validation: Option<PathBuf>,
       pub threshold: f64,
       pub format: OutputFormat,
       pub quiet: bool,
   }

   pub fn validate_improvement(config: ValidateImprovementConfig) -> Result<()> {
       // Implementation
   }
   ```

2. **Extract Pure Functions**
   - `load_comparison(path: &Path) -> Result<ComparisonResult>`
   - `load_previous_validation(path: &Path) -> Result<ValidationResult>`
   - `calculate_target_component(target: &TargetComparison) -> f64`
   - `calculate_regression_component(regressions: &[RegressionItem]) -> f64`
   - `calculate_project_health_component(health: &ProjectHealthComparison) -> f64`
   - `calculate_composite_score(target: f64, health: f64, regression: f64) -> f64`
   - `identify_gaps(comparison: &ComparisonResult, score: f64) -> HashMap<String, GapDetail>`
   - `generate_recommendations(score: f64, gaps: &HashMap<String, GapDetail>) -> Vec<String>`
   - `format_validation_json(result: &ValidationResult) -> Result<String>`
   - `format_validation_terminal(result: &ValidationResult) -> Result<String>`
   - `format_validation_markdown(result: &ValidationResult) -> Result<String>`

3. **Add to `src/commands/mod.rs`**
   ```rust
   pub mod validate_improvement;
   ```

#### Phase 2: Update CLI Definition

1. **Add to `src/cli.rs` Commands enum**
   ```rust
   #[derive(Subcommand, Debug)]
   pub enum Commands {
       // ... existing commands ...

       /// Validate technical debt improvement from comparison results
       ValidateImprovement {
           /// Path to comparison JSON file from 'debtmap compare'
           #[arg(long, value_name = "FILE")]
           comparison: PathBuf,

           /// Output file path for validation results
           #[arg(long, short = 'o', value_name = "FILE",
                 default_value = ".prodigy/debtmap-validation.json")]
           output: PathBuf,

           /// Path to previous validation for progress tracking
           #[arg(long, value_name = "FILE")]
           previous_validation: Option<PathBuf>,

           /// Improvement threshold percentage (0-100)
           #[arg(long, default_value = "75.0")]
           threshold: f64,

           /// Output format
           #[arg(short, long, value_enum, default_value = "json")]
           format: OutputFormat,

           /// Suppress progress output (automation mode)
           #[arg(long, short = 'q')]
           quiet: bool,
       },
   }
   ```

2. **Add command handler in `src/main.rs`**
   ```rust
   Commands::ValidateImprovement {
       comparison,
       output,
       previous_validation,
       threshold,
       format,
       quiet,
   } => {
       let config = ValidateImprovementConfig {
           comparison_path: comparison,
           output_path: output,
           previous_validation,
           threshold,
           format,
           quiet: quiet || is_automation_mode(),
       };
       debtmap::commands::validate_improvement::validate_improvement(config)?;
       Ok(())
   }
   ```

3. **Environment Variable Support**
   ```rust
   fn is_automation_mode() -> bool {
       std::env::var("PRODIGY_AUTOMATION")
           .unwrap_or_default()
           .eq_ignore_ascii_case("true")
           || std::env::var("PRODIGY_VALIDATION")
               .unwrap_or_default()
               .eq_ignore_ascii_case("true")
   }

   fn parse_arguments_env() -> Option<Vec<String>> {
       std::env::var("ARGUMENTS").ok().map(|args| {
           args.split_whitespace()
               .map(String::from)
               .collect()
       })
   }
   ```

#### Phase 3: Migrate Validation Logic

1. **Data Structures** (from existing binary)
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ValidationResult {
       pub completion_percentage: f64,
       pub status: String,
       pub improvements: Vec<String>,
       pub remaining_issues: Vec<String>,
       pub gaps: HashMap<String, GapDetail>,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub target_summary: Option<TargetSummary>,
       pub project_summary: ProjectSummary,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct GapDetail {
       pub description: String,
       pub location: String,
       pub severity: String,
       pub suggested_fix: String,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub score_before: Option<f64>,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub score_after: Option<f64>,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub current_score: Option<f64>,
   }
   ```

2. **Core Calculation Functions** (maintain existing logic)
   - Extract from lines 177-315 of existing binary
   - Ensure pure functional design (no side effects)
   - Add comprehensive unit tests

3. **Output Formatting**
   - JSON: Reuse existing format for Prodigy compatibility
   - Terminal: Add colored, human-readable format using `colored` crate
   - Markdown: Create report-style output

#### Phase 4: Update Prodigy Integration

1. **Update workflows/debtmap.yml**
   ```yaml
   # Old (line 31):
   - claude: "/prodigy-validate-debtmap-improvement --comparison .prodigy/comparison.json --output .prodigy/debtmap-validation.json"

   # New:
   - shell: "debtmap validate-improvement --comparison .prodigy/comparison.json --output .prodigy/debtmap-validation.json"
   ```

2. **Update slash command `.claude/commands/prodigy-validate-debtmap-improvement.md`**
   - Change from invoking binary to invoking subcommand
   - Update usage examples
   - Document new CLI arguments

3. **Verify backward compatibility**
   - Test with `ARGUMENTS` env var (legacy support)
   - Test with `PRODIGY_AUTOMATION=true` (quiet mode)
   - Ensure JSON output format unchanged

#### Phase 5: Cleanup and Documentation

1. **Remove Binary Files**
   - Delete `src/bin/prodigy-validate-debtmap-improvement.rs`
   - Remove `[[bin]]` section from `Cargo.toml`

2. **Update Documentation**
   - `book/src/cli-reference.md`: Add validate-improvement subcommand
   - `book/src/prodigy-integration.md`: Update workflow examples
   - `README.md`: Add validate-improvement to command list

3. **Update Tests**
   - Migrate `tests/validate_integration_test.rs` to use subcommand
   - Add unit tests for new module functions

### Architecture Changes

**Before:**
```
debtmap (main binary)
  └─> src/main.rs
prodigy-validate-debtmap-improvement (separate binary)
  └─> src/bin/prodigy-validate-debtmap-improvement.rs
```

**After:**
```
debtmap (single binary)
  ├─> src/main.rs
  ├─> src/cli.rs (Commands::ValidateImprovement)
  └─> src/commands/validate_improvement.rs (validation logic)
```

**Module Structure:**
```
src/commands/validate_improvement.rs
├── pub struct ValidateImprovementConfig
├── pub fn validate_improvement(config) -> Result<()>  // Entry point
├── fn load_comparison(path) -> Result<ComparisonResult>
├── fn calculate_components(comparison) -> (f64, f64, f64)
├── fn calculate_composite_score(components) -> f64
├── fn identify_gaps(comparison, score) -> HashMap<String, GapDetail>
├── fn format_json(result) -> String
├── fn format_terminal(result) -> String
└── fn format_markdown(result) -> String
```

### Data Structures

**Validation Result** (existing, preserve for compatibility):
```rust
{
  "completion_percentage": 72.3,
  "status": "incomplete",
  "improvements": [
    "Target item score reduced by 40.2%",
    "Overall project debt reduced by 3.0%"
  ],
  "remaining_issues": [
    "1 new critical debt item introduced"
  ],
  "gaps": {
    "insufficient_target_improvement": {
      "description": "Target function still above complexity threshold",
      "location": "src/analyzers/rust_analyzer.rs:build_call_graph:523",
      "severity": "medium",
      "suggested_fix": "Further extract helper functions",
      "score_before": 81.9,
      "score_after": 49.0
    }
  },
  "target_summary": {
    "location": "src/analyzers/rust_analyzer.rs:build_call_graph:523",
    "score_before": 81.9,
    "score_after": 49.0,
    "improvement_percent": 40.2,
    "status": "moderately_improved"
  },
  "project_summary": {
    "total_debt_before": 1247.3,
    "total_debt_after": 1210.5,
    "improvement_percent": 3.0,
    "items_resolved": 8,
    "items_new": 2
  }
}
```

### APIs and Interfaces

**Command Line Interface:**
```bash
# Standard usage
debtmap validate-improvement \
  --comparison .prodigy/comparison.json \
  --output .prodigy/validation.json

# With progress tracking
debtmap validate-improvement \
  --comparison .prodigy/comparison.json \
  --output .prodigy/validation.json \
  --previous-validation .prodigy/validation-attempt1.json

# Custom threshold
debtmap validate-improvement \
  --comparison .prodigy/comparison.json \
  --threshold 80.0

# Terminal output for interactive use
debtmap validate-improvement \
  --comparison .prodigy/comparison.json \
  --format terminal

# Quiet mode (automation)
debtmap validate-improvement \
  --comparison .prodigy/comparison.json \
  --quiet
```

**Environment Variable Interface** (backward compatibility):
```bash
# Legacy Prodigy support
ARGUMENTS="--comparison .prodigy/comparison.json --output .prodigy/validation.json" \
PRODIGY_AUTOMATION=true \
debtmap validate-improvement
```

**Programmatic Interface:**
```rust
use debtmap::commands::validate_improvement::{
    validate_improvement, ValidateImprovementConfig
};

let config = ValidateImprovementConfig {
    comparison_path: PathBuf::from(".prodigy/comparison.json"),
    output_path: PathBuf::from(".prodigy/validation.json"),
    previous_validation: None,
    threshold: 75.0,
    format: OutputFormat::Json,
    quiet: false,
};

validate_improvement(config)?;
```

## Dependencies

**Prerequisites:**
- None (standalone refactoring)

**Affected Components:**
- `src/cli.rs`: Add new subcommand variant
- `src/main.rs`: Add command handler
- `src/commands/mod.rs`: Export new module
- `Cargo.toml`: Remove binary definition
- Prodigy workflows and slash commands

**External Dependencies:**
- No new crate dependencies required
- Reuse existing: `clap`, `serde_json`, `anyhow`, `colored`

## Testing Strategy

### Unit Tests

1. **Calculation Functions**
   ```rust
   #[test]
   fn test_calculate_target_component() {
       let target = create_test_target(81.9, 15.2);
       let component = calculate_target_component(&target);
       assert!((component - 81.4).abs() < 0.1);
   }

   #[test]
   fn test_calculate_regression_component() {
       let regressions = vec![create_regression(65.3)];
       let component = calculate_regression_component(&regressions);
       assert_eq!(component, 80.0); // 100 - (1 * 20)
   }

   #[test]
   fn test_composite_score_calculation() {
       let score = calculate_composite_score(80.0, 50.0, 100.0);
       // 80*0.5 + 50*0.3 + 100*0.2 = 40 + 15 + 20 = 75
       assert_eq!(score, 75.0);
   }
   ```

2. **Gap Identification**
   ```rust
   #[test]
   fn test_identify_gaps_insufficient_improvement() {
       let comparison = create_test_comparison(/* ... */);
       let gaps = identify_gaps(&comparison, 65.0);
       assert!(gaps.contains_key("insufficient_target_improvement"));
   }
   ```

3. **Output Formatting**
   ```rust
   #[test]
   fn test_format_validation_json() {
       let result = create_test_validation_result();
       let json = format_validation_json(&result).unwrap();
       let parsed: ValidationResult = serde_json::from_str(&json).unwrap();
       assert_eq!(parsed.completion_percentage, result.completion_percentage);
   }
   ```

### Integration Tests

1. **End-to-End Validation**
   ```rust
   #[test]
   fn test_validate_improvement_command() {
       let temp_dir = tempdir().unwrap();
       let comparison_path = temp_dir.path().join("comparison.json");
       let output_path = temp_dir.path().join("validation.json");

       // Create test comparison file
       create_test_comparison_file(&comparison_path);

       // Run validation
       let config = ValidateImprovementConfig {
           comparison_path,
           output_path: output_path.clone(),
           previous_validation: None,
           threshold: 75.0,
           format: OutputFormat::Json,
           quiet: true,
       };

       validate_improvement(config).unwrap();

       // Verify output
       let result: ValidationResult =
           serde_json::from_str(&fs::read_to_string(output_path).unwrap()).unwrap();
       assert!(result.completion_percentage > 0.0);
   }
   ```

2. **Backward Compatibility**
   ```rust
   #[test]
   fn test_environment_variable_support() {
       std::env::set_var("ARGUMENTS", "--comparison test.json --output out.json");
       std::env::set_var("PRODIGY_AUTOMATION", "true");

       // Parse environment variables
       let args = parse_arguments_env().unwrap();
       assert_eq!(args.len(), 4);

       std::env::remove_var("ARGUMENTS");
       std::env::remove_var("PRODIGY_AUTOMATION");
   }
   ```

3. **Workflow Integration**
   ```bash
   # Test Prodigy workflow compatibility
   just coverage-lcov
   debtmap analyze . --lcov target/coverage/lcov.info --output before.json
   # (make changes)
   debtmap analyze . --lcov target/coverage/lcov.info --output after.json
   debtmap compare --before before.json --after after.json --output comparison.json
   debtmap validate-improvement --comparison comparison.json --output validation.json
   # Verify validation.json has expected structure
   ```

### Performance Tests

```rust
#[bench]
fn bench_validate_improvement(b: &mut Bencher) {
    let comparison = create_large_comparison(1000); // 1000 items
    b.iter(|| {
        validate_improvement_internal(&comparison)
    });
}
```

### User Acceptance

- [ ] Command discoverable via `debtmap --help`
- [ ] Interactive terminal output is readable and helpful
- [ ] JSON output compatible with existing Prodigy workflows
- [ ] Progress tracking shows meaningful trend information
- [ ] Error messages are clear and actionable

## Documentation Requirements

### Code Documentation

1. **Module Documentation**
   ```rust
   //! Validation of technical debt improvements
   //!
   //! This module validates that technical debt improvements have been made
   //! by analyzing comparison output from `debtmap compare`.
   //!
   //! # Scoring Algorithm
   //!
   //! The validation score is a composite of three components:
   //! - Target improvement (50%): Did the specific target item improve?
   //! - Project health (30%): Did overall project debt decrease?
   //! - No regressions (20%): Were new critical items introduced?
   //!
   //! # Examples
   //!
   //! ```no_run
   //! use debtmap::commands::validate_improvement::*;
   //!
   //! let config = ValidateImprovementConfig {
   //!     comparison_path: PathBuf::from("comparison.json"),
   //!     output_path: PathBuf::from("validation.json"),
   //!     threshold: 75.0,
   //!     format: OutputFormat::Json,
   //!     quiet: false,
   //! };
   //!
   //! validate_improvement(config)?;
   //! ```
   ```

2. **Function Documentation**
   - Document all public functions with examples
   - Explain calculation formulas
   - Describe error conditions

### User Documentation

1. **CLI Reference** (book/src/cli-reference.md)
   ```markdown
   ## validate-improvement

   Validates technical debt improvements by analyzing comparison results.

   ### Usage

   debtmap validate-improvement --comparison <FILE> [OPTIONS]

   ### Options

   - `--comparison <FILE>`: Path to comparison JSON (required)
   - `--output <FILE>`: Output path (default: .prodigy/debtmap-validation.json)
   - `--previous-validation <FILE>`: Previous validation for progress tracking
   - `--threshold <NUM>`: Improvement threshold % (default: 75.0)
   - `--format <FORMAT>`: Output format: json, terminal, markdown (default: json)
   - `--quiet`: Suppress progress output

   ### Examples

   # Basic validation
   debtmap validate-improvement --comparison comparison.json

   # With progress tracking
   debtmap validate-improvement \
     --comparison comparison.json \
     --previous-validation validation-attempt1.json

   # Interactive terminal output
   debtmap validate-improvement \
     --comparison comparison.json \
     --format terminal
   ```

2. **Prodigy Integration** (book/src/prodigy-integration.md)
   - Update workflow examples
   - Document environment variable support
   - Explain backward compatibility

3. **Architecture Documentation** (ARCHITECTURE.md if needed)
   - Explain validation algorithm
   - Document composite scoring formula
   - Describe gap identification logic

## Implementation Notes

### Functional Programming Guidelines

1. **Pure Functions for Calculations**
   ```rust
   // Pure: Same inputs always produce same outputs
   fn calculate_composite_score(
       target_component: f64,
       health_component: f64,
       regression_component: f64,
   ) -> f64 {
       (target_component * 0.5
        + health_component * 0.3
        + regression_component * 0.2)
           .clamp(0.0, 100.0)
   }
   ```

2. **I/O at Edges**
   ```rust
   // I/O isolated at boundaries
   pub fn validate_improvement(config: ValidateImprovementConfig) -> Result<()> {
       // I/O: Load inputs
       let comparison = load_comparison(&config.comparison_path)?;
       let previous = config.previous_validation
           .as_ref()
           .map(load_previous_validation)
           .transpose()?;

       // Pure: Perform calculations
       let result = validate_improvement_internal(&comparison, previous.as_ref())?;

       // I/O: Write outputs
       write_validation_result(&config.output_path, &result, config.format)?;

       // I/O: Print to console (if not quiet)
       if !config.quiet {
           print_validation_summary(&result);
       }

       Ok(())
   }
   ```

3. **Testable Core Logic**
   ```rust
   // Internal pure function (easily testable)
   fn validate_improvement_internal(
       comparison: &ComparisonResult,
       previous: Option<&ValidationResult>,
   ) -> Result<ValidationResult> {
       // All pure calculations here
   }
   ```

### Error Handling

```rust
use anyhow::{Context, Result, bail};

fn load_comparison(path: &Path) -> Result<ComparisonResult> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read comparison file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse comparison JSON from: {}", path.display()))
}

fn validate_threshold(threshold: f64) -> Result<()> {
    if !(0.0..=100.0).contains(&threshold) {
        bail!("Threshold must be between 0 and 100, got: {}", threshold);
    }
    Ok(())
}
```

### Output Formatting Patterns

```rust
fn format_validation_terminal(result: &ValidationResult) -> Result<String> {
    use colored::*;

    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "{}\n",
        "═══ Validation Results ═══".bold()
    ));

    // Score with color based on status
    let score_str = format!("{:.1}%", result.completion_percentage);
    let colored_score = if result.status == "complete" {
        score_str.green()
    } else {
        score_str.yellow()
    };

    output.push_str(&format!("Completion: {}\n", colored_score));
    output.push_str(&format!("Status: {}\n\n", result.status));

    // Improvements (green)
    if !result.improvements.is_empty() {
        output.push_str(&"✓ Improvements:\n".green().to_string());
        for improvement in &result.improvements {
            output.push_str(&format!("  • {}\n", improvement));
        }
        output.push('\n');
    }

    // Remaining issues (yellow/red)
    if !result.remaining_issues.is_empty() {
        output.push_str(&"⚠ Remaining Issues:\n".yellow().to_string());
        for issue in &result.remaining_issues {
            output.push_str(&format!("  • {}\n", issue));
        }
    }

    Ok(output)
}
```

## Migration and Compatibility

### Breaking Changes

**None** - This is a purely additive change with backward compatibility.

### Backward Compatibility Strategy

1. **Environment Variable Support**
   - Continue supporting `ARGUMENTS` env var for legacy workflows
   - Maintain `PRODIGY_AUTOMATION` and `PRODIGY_VALIDATION` flags

2. **JSON Output Format**
   - Preserve exact JSON structure for Prodigy compatibility
   - No changes to field names or types

3. **Default Behavior**
   - Same defaults as existing binary
   - Same calculation formulas

### Migration Path for Users

**For Prodigy Workflows:**
1. Update `workflows/debtmap.yml` to use subcommand
2. Test workflow execution
3. No other changes required (environment variables still work)

**For Interactive Users:**
1. New feature - no migration needed
2. Discover via `debtmap --help`
3. Use standard CLI arguments

### Deprecation Timeline

**Immediate:**
- Remove binary from `Cargo.toml`
- Delete `src/bin/prodigy-validate-debtmap-improvement.rs`

**No deprecation warnings needed** - this is internal tooling.

## Success Metrics

- [ ] Binary size reduced (one executable instead of two)
- [ ] All existing Prodigy workflows pass without modification
- [ ] Unit test coverage >= 85% for new module
- [ ] Integration tests pass for all usage scenarios
- [ ] Documentation complete and accurate
- [ ] Command discoverable via `--help`
- [ ] Interactive users can run validation with clear output

## References

- Existing binary: `src/bin/prodigy-validate-debtmap-improvement.rs`
- Comparison logic: `src/comparison/comparator.rs`
- Prodigy workflow: `workflows/debtmap.yml`
- Slash command: `.claude/commands/prodigy-validate-debtmap-improvement.md`
- Related specs: None (standalone feature)
