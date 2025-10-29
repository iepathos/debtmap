---
number: 135
title: Context-Aware File Size Heuristics
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-27
---

# Specification 135: Context-Aware File Size Heuristics

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Current file size recommendations are too aggressive and don't account for file type context. For example:
- Recommending reducing a 7775-line flags definition file to <500 lines (93.6% reduction)
- This file has 888 functions, making the target 0.56 lines per function (impossible)
- Generated code, declarative configurations, and test files have different appropriate sizes
- One-size-fits-all thresholds produce impractical recommendations

This undermines user trust when debtmap suggests impossible or architecturally inappropriate changes.

## Objective

Implement context-aware file size heuristics that adjust thresholds based on file type, purpose, and characteristics, producing practical and architecturally sound recommendations.

## Requirements

### Functional Requirements

1. **File Type Classification**
   - Detect generated code (markers like "DO NOT EDIT", code generation tools)
   - Identify declarative/configuration files (flag definitions, schema definitions)
   - Recognize test files (test modules, integration tests, property tests)
   - Distinguish business logic from infrastructure code
   - Identify procedural macros and build scripts

2. **Context-Based Thresholds**
   - Different size limits for different file types:
     - Business logic: 300-500 lines (strict)
     - Generated code: 2000+ lines (lenient, may suppress)
     - Test files: 500-800 lines (moderate)
     - Declarative config: 1000-1500 lines (lenient)
     - Procedural macros: 400-600 lines (moderate-strict)
   - Adjust thresholds based on function density (lines per function)
   - Consider architectural patterns (builder patterns may be verbose)

3. **Smart Reduction Targets**
   - Calculate achievable reduction targets based on:
     - Current function density
     - File type and purpose
     - Complexity distribution
     - Architectural constraints
   - Never suggest reduction targets that would require <3 lines per function
   - Provide phased reduction targets for large files (50% → 30% → final)

4. **Recommendation Quality**
   - Suppress or downgrade recommendations for generated code
   - Focus on high-value splitting opportunities (high complexity + large size)
   - Provide file-type-specific splitting strategies
   - Distinguish "must split" from "could split" recommendations

### Non-Functional Requirements

1. **Accuracy**: File type detection should be >95% accurate on common patterns
2. **Clarity**: Users should understand why different files have different thresholds
3. **Flexibility**: Thresholds should be configurable via CLI or config file
4. **Performance**: Classification should not significantly slow analysis

## Acceptance Criteria

- [ ] Generated code is correctly identified and gets lenient thresholds or suppressed warnings
- [ ] Declarative/config files (like flags/defs.rs) get appropriate thresholds (1000-1500 lines)
- [ ] Business logic files maintain strict thresholds (300-500 lines)
- [ ] Test files get moderate thresholds (500-800 lines)
- [ ] No recommendation suggests reducing to <3 lines per function
- [ ] Function density is factored into threshold calculation
- [ ] Phased reduction targets are provided for files >2x threshold
- [ ] File type classification is shown in output with rationale
- [ ] Ripgrep flags/defs.rs gets appropriate threshold (not 500 lines)
- [ ] Reduction targets are achievable and architecturally sound
- [ ] Users can override thresholds via configuration
- [ ] Documentation explains threshold rationale for each file type

## Technical Details

### Implementation Approach

1. **File Type Detection Pipeline**
   ```rust
   pub enum FileType {
       BusinessLogic,
       GeneratedCode { tool: Option<String> },
       TestCode { test_type: TestType },
       DeclarativeConfig { config_type: ConfigType },
       ProceduralMacro,
       BuildScript,
       Unknown,
   }

   pub fn classify_file(source: &str, path: &Path) -> FileType {
       // Multi-stage classification
       if is_generated_code(source) {
           FileType::GeneratedCode { tool: detect_generator(source) }
       } else if is_test_file(path, source) {
           FileType::TestCode { test_type: detect_test_type(source) }
       } else if is_declarative_config(source) {
           FileType::DeclarativeConfig { config_type: detect_config_type(source) }
       } else if is_proc_macro(path) {
           FileType::ProceduralMacro
       } else if is_build_script(path) {
           FileType::BuildScript
       } else {
           FileType::BusinessLogic
       }
   }
   ```

2. **Context-Aware Thresholds**
   ```rust
   pub struct FileSizeThresholds {
       base_threshold: usize,
       max_threshold: usize,
       min_lines_per_function: f32,
   }

   pub fn get_threshold(file_type: &FileType, metrics: &FileMetrics) -> FileSizeThresholds {
       let base = match file_type {
           FileType::BusinessLogic => 400,
           FileType::GeneratedCode { .. } => 5000, // Lenient or suppressed
           FileType::TestCode { .. } => 650,
           FileType::DeclarativeConfig { .. } => 1200,
           FileType::ProceduralMacro => 500,
           FileType::BuildScript => 300,
           FileType::Unknown => 400,
       };

       // Adjust based on function density
       let density = metrics.lines as f32 / metrics.functions.max(1) as f32;
       let adjusted = adjust_for_density(base, density);

       FileSizeThresholds {
           base_threshold: adjusted,
           max_threshold: adjusted * 2,
           min_lines_per_function: 3.0,
       }
   }
   ```

3. **Smart Reduction Calculation**
   ```rust
   pub fn calculate_reduction_target(
       current_lines: usize,
       threshold: &FileSizeThresholds,
       function_count: usize
   ) -> ReductionTarget {
       // Minimum achievable size based on function count
       let min_achievable = (function_count as f32 * threshold.min_lines_per_function) as usize;

       // Don't suggest reducing below achievable minimum
       let target = threshold.base_threshold.max(min_achievable);

       if current_lines > threshold.base_threshold * 3 {
           // Phased reduction for very large files
           ReductionTarget::Phased {
               phase1: current_lines / 2,
               phase2: threshold.base_threshold * 1.5,
               final_target: target,
           }
       } else {
           ReductionTarget::Single(target)
       }
   }
   ```

### File Type Detection Strategies

1. **Generated Code Detection**
   ```rust
   fn is_generated_code(source: &str) -> bool {
       let markers = [
           "DO NOT EDIT",
           "automatically generated",
           "AUTO-GENERATED",
           "@generated",
           "Code generated by",
       ];
       source.lines().take(10).any(|line|
           markers.iter().any(|m| line.contains(m))
       )
   }

   fn detect_generator(source: &str) -> Option<String> {
       // Detect common generators: protobuf, thrift, diesel, sea-orm, etc.
       if source.contains("prost::Message") { Some("prost".to_string()) }
       else if source.contains("diesel::") { Some("diesel".to_string()) }
       else if source.contains("tonic::") { Some("tonic".to_string()) }
       else { None }
   }
   ```

2. **Declarative Config Detection**
   ```rust
   use once_cell::sync::Lazy;
   use regex::Regex;

   // Compile regexes once at startup using once_cell
   static FIELD_PATTERN: Lazy<Regex> = Lazy::new(|| {
       Regex::new(r"(?m)^\s*pub\s+\w+:\s+\w+,\s*$").unwrap()
   });

   static DERIVE_PATTERN: Lazy<Regex> = Lazy::new(|| {
       Regex::new(r"(?m)^\s*#\[derive\(").unwrap()
   });

   static BUILDER_METHOD_PATTERN: Lazy<Regex> = Lazy::new(|| {
       Regex::new(r"(?m)^\s*pub\s+fn\s+\w+\(mut\s+self").unwrap()
   });

   fn is_declarative_config(source: &str) -> bool {
       // High density of similar patterns using pre-compiled regexes
       let field_matches = FIELD_PATTERN.find_iter(source).count();
       let derive_matches = DERIVE_PATTERN.find_iter(source).count();
       let builder_matches = BUILDER_METHOD_PATTERN.find_iter(source).count();

       let total_matches = field_matches + derive_matches + builder_matches;
       let total_lines = source.lines().count();

       // If >70% of lines match declarative patterns
       (total_matches as f32 / total_lines as f32) > 0.7
   }
   ```

3. **Function Density Analysis**
   ```rust
   fn adjust_for_density(base_threshold: usize, density: f32) -> usize {
       // Very low density (many small functions) → stricter threshold
       // High density (few large functions) → may need lenient threshold
       match density {
           d if d < 5.0 => base_threshold,           // Many small functions: strict
           d if d < 10.0 => (base_threshold as f32 * 1.2) as usize,
           d if d < 20.0 => (base_threshold as f32 * 1.5) as usize,
           _ => (base_threshold as f32 * 2.0) as usize,  // Few large functions: lenient
       }
   }
   ```

### Data Structures

```rust
#[derive(Debug, Clone)]
pub enum TestType {
    Unit,
    Integration,
    Property,
    Benchmark,
}

#[derive(Debug, Clone)]
pub enum ConfigType {
    Flags,
    Schema,
    Routes,
    Builder,
}

#[derive(Debug, Clone)]
pub enum ReductionTarget {
    Single(usize),
    Phased {
        phase1: usize,
        phase2: usize,
        final_target: usize,
    },
    NotRecommended { reason: String },
}

#[derive(Debug)]
pub struct FileSizeAnalysis {
    file_type: FileType,
    current_lines: usize,
    threshold: FileSizeThresholds,
    reduction_target: ReductionTarget,
    function_density: f32,
    recommendation_level: RecommendationLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum RecommendationLevel {
    Critical,  // >2x threshold, business logic
    High,      // >1.5x threshold, business logic
    Medium,    // >threshold but <1.5x
    Low,       // Slightly over threshold
    Suppressed, // Generated/declarative code
}
```

## Integration Points

This feature integrates with multiple components of the debtmap architecture:

### 1. Classification Module (New)
- **Location**: `src/organization/file_classifier.rs`
- **Rationale**: Classification logic belongs with organizational analysis (GodObjectType, BuilderPattern, BoilerplatePattern)
- **Exports**: `FileType`, `FileSizeThresholds`, `classify_file()`, `get_threshold()`, `calculate_reduction_target()`

### 2. File Metrics Collection
- **Location**: `src/priority/file_metrics.rs`
- **Integration**: Add `file_type: FileType` field to `FileDebtMetrics` struct
- **Call Site**: Invoke `classify_file()` during metrics collection in `FileDebtMetrics::new()`

### 3. Output Formatters (3 locations)
- **Terminal Output**: `src/priority/formatter.rs:1177`
  - Replace `classify_file_size()` with context-aware `get_threshold()`
  - Update recommendation text to include file type rationale
- **Markdown Output**: `src/priority/formatter_markdown.rs:531,571`
  - Apply same threshold logic as terminal output
  - Add file type classification to markdown headers
- **JSON Output**: `src/io/output.rs`
  - Add `file_type` and `threshold_rationale` fields to JSON schema
  - Mark new fields with `#[serde(default, skip_serializing_if = "Option::is_none")]`

### 4. Debt Scoring
- **Location**: `src/priority/debt_aggregator.rs`
- **Integration**: Use context-aware thresholds when calculating file size penalties
- **Impact**: Debt scores will change; document in migration guide

### 5. Configuration System
- **Location**: `src/config.rs`
- **Integration**: Load threshold overrides from `.debtmap.toml`
- **Precedence**: CLI flags > project `.debtmap.toml` > `~/.config/debtmap/config.toml` > defaults

## Configuration

### Configuration File Location and Loading

#### File Locations (in precedence order)
1. **CLI flags**: `--threshold-business-logic=500`
2. **Project config**: `./.debtmap.toml` (in project root)
3. **User config**: `~/.config/debtmap/config.toml` (global defaults)
4. **Built-in defaults**: Hard-coded in `file_classifier.rs`

#### Configuration Schema

```toml
# .debtmap.toml
[thresholds]
business_logic = 400
test_code = 650
declarative_config = 1200
generated_code = 5000
proc_macro = 500
build_script = 300

# Minimum lines per function (safety threshold)
min_lines_per_function = 3.0

[thresholds.overrides]
# File-specific overrides using glob patterns
"src/flags/defs.rs" = 2000
"tests/integration/*.rs" = 1000
"**/generated/**/*.rs" = 10000
```

#### CLI Flag Interface

```bash
# Override specific thresholds
debtmap analyze --threshold-business-logic=500 --threshold-test=800

# Restore legacy behavior
debtmap analyze --legacy-thresholds

# Preview threshold changes without applying
debtmap analyze --preview-thresholds
```

#### Configuration Loading Implementation

```rust
// src/config.rs
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    #[serde(default)]
    pub thresholds: ThresholdLimits,
    #[serde(default)]
    pub overrides: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdLimits {
    #[serde(default = "default_business_logic")]
    pub business_logic: usize,
    #[serde(default = "default_test_code")]
    pub test_code: usize,
    #[serde(default = "default_declarative_config")]
    pub declarative_config: usize,
    #[serde(default = "default_generated_code")]
    pub generated_code: usize,
    #[serde(default = "default_proc_macro")]
    pub proc_macro: usize,
    #[serde(default = "default_build_script")]
    pub build_script: usize,
    #[serde(default = "default_min_lines_per_function")]
    pub min_lines_per_function: f32,
}

fn default_business_logic() -> usize { 400 }
fn default_test_code() -> usize { 650 }
fn default_declarative_config() -> usize { 1200 }
fn default_generated_code() -> usize { 5000 }
fn default_proc_macro() -> usize { 500 }
fn default_build_script() -> usize { 300 }
fn default_min_lines_per_function() -> f32 { 3.0 }

impl Default for ThresholdLimits {
    fn default() -> Self {
        Self {
            business_logic: default_business_logic(),
            test_code: default_test_code(),
            declarative_config: default_declarative_config(),
            generated_code: default_generated_code(),
            proc_macro: default_proc_macro(),
            build_script: default_build_script(),
            min_lines_per_function: default_min_lines_per_function(),
        }
    }
}

impl ThresholdConfig {
    /// Load configuration with precedence: CLI > project > user > defaults
    pub fn load(
        project_root: Option<&Path>,
        cli_overrides: Option<ThresholdLimits>,
    ) -> Result<Self> {
        let mut config = ThresholdConfig::default();

        // 1. Load user config
        if let Some(user_config) = Self::load_user_config()? {
            config = config.merge(user_config);
        }

        // 2. Load project config
        if let Some(root) = project_root {
            if let Some(project_config) = Self::load_project_config(root)? {
                config = config.merge(project_config);
            }
        }

        // 3. Apply CLI overrides
        if let Some(cli) = cli_overrides {
            config.thresholds = cli;
        }

        Ok(config)
    }

    fn load_user_config() -> Result<Option<Self>> {
        let config_path = dirs::config_dir()
            .map(|p| p.join("debtmap/config.toml"));

        if let Some(path) = config_path {
            if path.exists() {
                let content = std::fs::read_to_string(&path)
                    .context("Failed to read user config")?;
                let config: ThresholdConfig = toml::from_str(&content)
                    .context("Failed to parse user config")?;
                return Ok(Some(config));
            }
        }
        Ok(None)
    }

    fn load_project_config(root: &Path) -> Result<Option<Self>> {
        let config_path = root.join(".debtmap.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .context("Failed to read project config")?;
            let config: ThresholdConfig = toml::from_str(&content)
                .context("Failed to parse project config")?;
            return Ok(Some(config));
        }
        Ok(None)
    }

    fn merge(mut self, other: Self) -> Self {
        // Later configs override earlier ones
        self.thresholds.business_logic = other.thresholds.business_logic;
        self.thresholds.test_code = other.thresholds.test_code;
        self.thresholds.declarative_config = other.thresholds.declarative_config;
        self.thresholds.generated_code = other.thresholds.generated_code;
        self.thresholds.proc_macro = other.thresholds.proc_macro;
        self.thresholds.build_script = other.thresholds.build_script;
        self.thresholds.min_lines_per_function = other.thresholds.min_lines_per_function;
        self.overrides.extend(other.overrides);
        self
    }
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            thresholds: ThresholdLimits::default(),
            overrides: std::collections::HashMap::new(),
        }
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/file_metrics.rs` - File-level analysis (add file_type field)
  - `src/priority/formatter.rs` - Terminal output (update classify_file_size)
  - `src/priority/formatter_markdown.rs` - Markdown output (update thresholds)
  - `src/priority/debt_aggregator.rs` - Scoring (use context-aware thresholds)
  - `src/io/output.rs` - JSON output (add new fields)
  - `src/config.rs` - Configuration loading (add ThresholdConfig)
  - `src/organization/file_classifier.rs` - New module for classification
- **External Dependencies**:
  - `regex` (already in use)
  - `once_cell` (already in Cargo.toml) - for regex compilation
  - `dirs` - for user config directory location (~/.config)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_generated_code_detection() {
    let generated = r#"
        // DO NOT EDIT
        // This file is automatically generated
        pub struct Generated {}
    "#;
    assert!(is_generated_code(generated));
}

#[test]
fn test_declarative_config_detection() {
    let flags = r#"
        pub struct Flags {
            pub verbose: bool,
            pub quiet: bool,
            pub output: PathBuf,
            // ... 800 more lines of similar fields
        }
    "#;
    assert!(is_declarative_config(flags));
}

#[test]
fn test_reduction_target_respects_function_count() {
    let threshold = FileSizeThresholds {
        base_threshold: 500,
        max_threshold: 1000,
        min_lines_per_function: 3.0,
    };

    let target = calculate_reduction_target(2000, &threshold, 600);
    // Should not suggest <1800 lines (600 functions * 3 lines)
    match target {
        ReductionTarget::Single(t) => assert!(t >= 1800),
        _ => panic!("Expected single target"),
    }
}

#[test]
fn test_function_density_adjustment() {
    let low_density = adjust_for_density(400, 4.0); // Many small functions
    let high_density = adjust_for_density(400, 25.0); // Few large functions

    assert_eq!(low_density, 400); // Strict threshold
    assert!(high_density > 600); // More lenient
}
```

### Integration Tests

```rust
#[test]
fn test_ripgrep_flags_defs_appropriate_threshold() {
    let analysis = analyze_file("../ripgrep/crates/core/flags/defs.rs").unwrap();

    // Should be classified as declarative config
    assert!(matches!(analysis.file_type, FileType::DeclarativeConfig { .. }));

    // Should get lenient threshold (not 500 lines)
    assert!(analysis.threshold.base_threshold >= 1000);

    // Reduction target should be achievable
    if let ReductionTarget::Single(target) = analysis.reduction_target {
        let min_achievable = (888 * 3) as usize; // 888 functions * 3 lines
        assert!(target >= min_achievable);
    }
}

#[test]
fn test_business_logic_strict_threshold() {
    let analysis = analyze_file("src/debt/god_object.rs").unwrap();

    // Business logic should get strict threshold
    assert!(matches!(analysis.file_type, FileType::BusinessLogic));
    assert!(analysis.threshold.base_threshold <= 500);
}
```

### Performance Benchmarks

Performance is critical for maintaining debtmap's fast analysis times. Classification should add minimal overhead.

```rust
// benches/file_classification_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use debtmap::organization::file_classifier::{classify_file, FileType};
use std::path::Path;

fn classification_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_classification");

    // Test files of varying sizes
    let test_cases = vec![
        ("small", include_str!("../tests/fixtures/small_business_logic.rs"), 150),
        ("medium", include_str!("../tests/fixtures/medium_test_file.rs"), 500),
        ("large", include_str!("../tests/fixtures/large_declarative_config.rs"), 2000),
        ("huge", include_str!("../tests/fixtures/huge_generated_code.rs"), 10000),
    ];

    for (name, content, lines) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("classify_file", format!("{name}_{lines}lines")),
            &content,
            |b, content| {
                b.iter(|| {
                    classify_file(black_box(content), black_box(Path::new("test.rs")))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, classification_benchmark);
criterion_main!(benches);
```

#### Performance Targets

**Acceptable Overhead**: Classification should add <5% to total analysis time

| File Size | Classification Time (Target) | Total Analysis Time | Overhead % |
|-----------|------------------------------|---------------------|------------|
| 150 lines | <0.1ms | ~2ms | <5% |
| 500 lines | <0.3ms | ~6ms | <5% |
| 2000 lines | <1ms | ~25ms | <4% |
| 10000 lines | <5ms | ~150ms | <3.3% |

**Baseline Measurement**: Run benchmarks on debtmap's own codebase (80+ files) to establish baseline before implementation.

**Optimization Strategy**:
- Use `once_cell::sync::Lazy` for compiled regexes (avoid re-compilation)
- Early-exit detection (check generated code markers first, most deterministic)
- Cache classification results in file metrics (no re-classification needed)
- Parallel classification via `rayon::par_iter()` during file analysis phase

#### Performance Validation Test

```rust
#[test]
fn classification_performance_regression() {
    use std::time::Instant;

    let large_file = include_str!("../tests/fixtures/large_declarative_config.rs");
    let iterations = 1000;

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = classify_file(large_file, Path::new("test.rs"));
    }
    let duration = start.elapsed();

    let avg_time = duration.as_micros() / iterations;

    // Should classify 2000-line file in <1ms (1000μs)
    assert!(
        avg_time < 1000,
        "Classification too slow: {}μs (expected <1000μs)",
        avg_time
    );
}
```

### Property-Based Tests

```rust
proptest! {
    #[test]
    fn reduction_target_never_below_minimum(
        lines in 100..10000usize,
        functions in 10..1000usize
    ) {
        let threshold = FileSizeThresholds::default();
        let target = calculate_reduction_target(lines, &threshold, functions);

        let min_achievable = functions * 3;
        match target {
            ReductionTarget::Single(t) => prop_assert!(t >= min_achievable),
            ReductionTarget::Phased { final_target, .. } => {
                prop_assert!(final_target >= min_achievable)
            }
            _ => {}
        }
    }
}
```

## Documentation Requirements

### Code Documentation

- Document file type classification algorithm and heuristics
- Explain threshold calculation and density adjustments
- Provide examples of each file type with rationale
- Document configuration options for threshold overrides

### User Documentation

- Explain why different files have different thresholds
- Provide guidance on when to split vs when to accept large files
- Document how to configure custom thresholds
- Show examples of context-appropriate recommendations

### Architecture Updates

Update ARCHITECTURE.md:
- Add section on file classification pipeline
- Document threshold calculation strategy
- Explain the balance between strictness and practicality

## Implementation Notes

### Classification Accuracy

Start with high-confidence heuristics:
1. **Generated code**: Very clear markers, low false positive rate
2. **Test files**: File path + `#[test]` attributes
3. **Declarative config**: Pattern density analysis

Iterate on less obvious cases:
- Build scripts (build.rs)
- Procedural macros (proc_macro crate)
- Mixed files (both business logic and config)

### Configuration Interface

```toml
# .debtmap.toml
[thresholds]
business_logic = 400
test_code = 650
declarative_config = 1200
generated_code = 5000

[thresholds.overrides]
# File-specific overrides
"src/flags/defs.rs" = 2000
"tests/integration/*.rs" = 1000
```

### Edge Cases

- **Mixed-purpose files**: Use weighted classification
- **Incrementally generated code**: User might edit generated files
- **Large test matrices**: Property tests can be legitimately large
- **Macro-heavy code**: May appear declarative but isn't

### Phased Rollout

1. **Phase 1**: Implement classification, log detected types (no behavior change)
2. **Phase 2**: Apply different thresholds, make recommendations
3. **Phase 3**: Add configuration support
4. **Phase 4**: Refine based on user feedback

## Migration and Compatibility

### Breaking Changes

- File size recommendations will change (some will get more lenient thresholds)
- Debt scores may change due to different thresholds

### Backward Compatibility

- Add `--legacy-thresholds` flag for old behavior
- Configuration file can restore previous thresholds if needed
- JSON output includes `file_type` and `threshold_rationale` fields

### Migration Path

1. Announce threshold changes in release notes
2. Provide migration guide showing new vs old thresholds
3. Offer configuration to restore legacy behavior
4. Deprecate legacy mode after 2-3 releases

## Success Metrics

- Zero "impossible" reduction targets (<3 lines per function)
- Generated code recommendations suppressed or downgraded
- User satisfaction with recommendation practicality
- Reduction in GitHub issues about unrealistic recommendations
- >95% file type classification accuracy on test corpus
