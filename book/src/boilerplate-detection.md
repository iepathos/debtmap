# Boilerplate Detection

Debtmap identifies repetitive code patterns that could benefit from macro-ification or other abstraction techniques. This helps reduce maintenance burden and improve code consistency.

## Overview

Boilerplate detection analyzes low-complexity repetitive code to identify opportunities for:

- **Macro-ification** - Convert repetitive patterns to declarative or procedural macros
- **Code generation** - Use build scripts to generate repetitive implementations
- **Generic abstractions** - Replace duplicate implementations with generic code
- **Trait derivation** - Use derive macros instead of manual implementations

Boilerplate detection runs automatically as part of the god object analysis pipeline. When a file has many impl blocks, it's classified as either a true god object (complex code needing splitting), a builder pattern (intentional fluent API), or a boilerplate pattern (low complexity needing macro-ification). This prevents false positives where repetitive low-complexity code is misclassified as god objects.

**Source**: Integration with god object detection in src/organization/god_object/classification_types.rs:45-50, src/analyzers/file_analyzer.rs:366-372

## Detection Criteria

Debtmap identifies boilerplate using trait pattern analysis (src/organization/trait_pattern_analyzer.rs:158-176):

- **Multiple similar trait implementations** - 20+ impl blocks with shared structure
- **High method uniformity** - 70%+ of implementations share the same methods
- **Low complexity repetitive code** - Average cyclomatic complexity < 2.0
- **Low complexity variance** - Consistent complexity across implementations
- **Single dominant trait** - One trait accounts for 80%+ of implementations

The TraitPatternAnalyzer computes these metrics:
- `impl_block_count` - Number of trait implementations in the file
- `unique_traits` - Set of distinct traits implemented
- `most_common_trait` - Most frequently implemented trait and count
- `method_uniformity` - Ratio of most common method appearance to total impls
- `shared_methods` - Methods appearing in 50%+ of implementations
- `avg_method_complexity` - Average cyclomatic complexity per method
- `complexity_variance` - Variance in complexity across methods
- `avg_method_lines` - Average lines of code per method

## Detection Signals

The boilerplate detector extracts detection signals (src/organization/boilerplate_detector.rs:246-253, 164-190):

- **HighImplCount(usize)** - Number of impl blocks exceeds threshold
- **HighMethodUniformity(f64)** - Methods are highly uniform across implementations
- **LowAvgComplexity(f64)** - Average complexity is below threshold
- **LowComplexityVariance(f64)** - Complexity variance is low (consistent complexity)
- **HighStructDensity(usize)** - Many structs with similar implementations

## Boilerplate Scoring Algorithm

The confidence score is calculated using weighted signals (src/organization/boilerplate_detector.rs:124-161):

1. **High impl count (30% weight)** - Files with 20+ impl blocks score higher
   - Normalized: `min(impl_count / 100, 1.0) × 30%`

2. **Method uniformity (25% weight)** - Methods shared across implementations
   - Score: `method_uniformity × 25%` (if ≥ 0.7 threshold)

3. **Low average complexity (20% weight)** - Simple, repetitive code
   - Score: `(1 - complexity / 2.0) × 20%` (if complexity < 2.0)

4. **Low complexity variance (15% weight)** - Consistent complexity
   - Score: `(1 - min(variance / 10.0, 1.0)) × 15%` (if variance < 2.0)

5. **Single dominant trait (10% weight)** - One trait dominates
   - Score: `trait_ratio × 10%` (if trait_ratio > 0.8)

**Threshold**: Patterns with confidence ≥ 0.7 (70%) are reported as boilerplate.

## Pattern Types

### Trait Implementation Boilerplate

Detected when a file has many similar trait implementations with low complexity.

**Example** (from tests/boilerplate_integration_test.rs:14-48):

```rust
// 26 From<Format> implementations with identical structure
pub enum Format { A, B, C, /* ... */ Z }
pub struct Target { name: String }

impl From<Format> for Target {
    fn from(f: Format) -> Self {
        match f {
            Format::A => Target { name: "a".to_string() },
            Format::B => Target { name: "b".to_string() },
            // ... 24 more identical patterns
        }
    }
}

// Detected: 26 impl blocks, 1.0 method uniformity, complexity ~2.0
```

**Recommendation**: Use declarative macro to reduce ~250 lines to ~30 lines.

### Builder Pattern

Detected when a file has repetitive setter methods returning `Self`.

**Example** (from book/src/boilerplate-detection.md:46-61):

```rust
impl ConfigBuilder {
    pub fn host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
    // ... more identical setter methods
}
```

**Recommendation**: Use `derive_builder` crate or custom derive macro.

### Test Boilerplate

Detected when test functions have shared structure and repetitive assertions (src/organization/boilerplate_detector.rs:238-243).

**Example**:

```rust
#[test]
fn test_case_a() {
    let input = create_input_a();
    let result = process(input);
    assert_eq!(result.status, Status::Success);
}

#[test]
fn test_case_b() {
    let input = create_input_b();
    let result = process(input);
    assert_eq!(result.status, Status::Success);
}
// ... 20 more similar test functions
```

**Recommendation**: Use parameterized tests with `rstest` or table-driven tests.

## Configuration

Boilerplate detection is controlled via configuration file (src/organization/boilerplate_detector.rs:256-322):

```toml
[boilerplate_detection]
# Enable boilerplate detection (default: true)
enabled = true

# Minimum impl blocks to consider (default: 20)
min_impl_blocks = 20

# Method uniformity threshold 0.0-1.0 (default: 0.7)
method_uniformity_threshold = 0.7

# Maximum average complexity for boilerplate (default: 2.0)
max_avg_complexity = 2.0

# Minimum confidence to report 0.0-1.0 (default: 0.7)
confidence_threshold = 0.7

# Enable trait implementation detection (default: true)
detect_trait_impls = true

# Enable builder pattern detection (default: true)
detect_builders = true

# Enable test boilerplate detection (default: true)
detect_test_boilerplate = true
```

**Field reference** (src/organization/boilerplate_detector.rs:48-57, 310-322):
- All fields have serde defaults
- Configuration can be provided via TOML or JSON
- Missing fields use default values

## Usage

Boilerplate detection runs automatically when enabled in configuration:

```bash
# Run analysis with default configuration
debtmap analyze .

# Use custom config file
debtmap analyze . --config custom-config.toml

# Show where configuration values came from
debtmap analyze . --show-config-sources
```

**Note**: There are no dedicated CLI flags like `--detect-boilerplate` or `--show-macro-suggestions`. Boilerplate detection is integrated into the god object analysis pipeline and controlled via configuration file only (verified in src/cli.rs - no boilerplate-specific flags exist).

## Macro Recommendations

The MacroRecommendationEngine generates specific refactoring guidance (src/organization/macro_recommendations.rs:13-150):

### For Trait Implementations

```
Detected boilerplate: 25 implementations of From trait
Estimated line reduction: 220 lines → 35 lines (84% reduction)

Recommendation:
- Use declarative macro (macro_rules!) for simple conversions
- Use procedural derive macro for complex transformations
- Consider code generation in build.rs for large enums
```

### For Builder Patterns

```
Detected boilerplate: 15 setter methods in ConfigBuilder
Estimated line reduction: 75 lines → 10 lines (87% reduction)

Recommendation:
- Add derive_builder to Cargo.toml
- Use #[derive(Builder)] on struct
- Configure with #[builder(setter(into))] for ergonomics
```

### For Test Boilerplate

```
Detected boilerplate: 20 similar test functions
Estimated line reduction: 120 lines → 25 lines (79% reduction)

Recommendation:
- Use rstest with #[rstest] and #[case(...)] for parameterized tests
- Extract common test setup into fixture functions
- Use table-driven tests with Vec<TestCase> for data-driven testing
```

## Integration with God Object Detection

Boilerplate detection prevents false positives in god object analysis:

1. **File with many impl blocks detected** → Analyze trait patterns
2. **High complexity + many impls** → Classified as GodObject (needs module splitting)
3. **Low complexity + many impls** → Classified as BoilerplatePattern (needs macro-ification)
4. **Builder pattern detected** → Classified as BuilderPattern (intentional design)

This distinction ensures appropriate recommendations for different code patterns.

**Source**: src/organization/god_object/classification_types.rs:45-50

## See Also

- [God Object Detection](god-object-detection.md) - Complexity-based refactoring
- [Design Pattern Detection](design-patterns.md) - Higher-level pattern recognition
- [Boilerplate vs Complexity](boilerplate-vs-complexity.md) - Understanding the distinction
- [Configuration](configuration.md) - Full configuration reference
