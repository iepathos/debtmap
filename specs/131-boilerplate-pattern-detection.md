---
number: 131
title: Boilerplate Pattern Detection and Macro Recommendation System
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-26
---

# Specification 131: Boilerplate Pattern Detection and Macro Recommendation System

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently identifies large files with many functions as "god objects" and recommends splitting them into smaller modules. However, this recommendation is inappropriate when the file contains repetitive boilerplate code that follows a declarative pattern (e.g., many trait implementations, builder patterns, serialization code).

**Real-World Example**: Analyzing ripgrep's `crates/core/flags/defs.rs` (7,775 lines, 888 functions):
- Contains 104 implementations of the `Flag` trait
- Each implementation has identical method signatures (`is_switch`, `name_long`, `doc_category`, etc.)
- Average method complexity: 1.2 (very simple)
- 100% method name uniformity across all implementations
- Each flag requires ~75 lines of boilerplate

Current debtmap output recommends "splitting into modules," but the correct solution is to **macro-ify or code-generate** the boilerplate, reducing the file from ~7,775 lines to ~1,500 lines (81% reduction).

**The Problem**: Debtmap cannot distinguish between:
- **Complex code** → needs module splitting
- **Boilerplate code** → needs macro-ification or codegen

This leads to:
1. Inappropriate recommendations for boilerplate-heavy files
2. Missed opportunities to suggest Rust-idiomatic solutions (macros, derive)
3. Lower value for users analyzing projects with declarative patterns

## Objective

Implement a boilerplate pattern detection system that identifies when large files contain repetitive, low-complexity code patterns and provides context-specific recommendations (macro-ification, code generation, or derive macros) instead of generic module-splitting advice. Include comprehensive user-facing documentation explaining the distinction between complexity and boilerplate.

## Requirements

### Functional Requirements

1. **Pattern Detection**
   - Detect trait implementation boilerplate (many `impl Trait for Type` blocks)
   - Detect builder pattern boilerplate (multiple builder structs with similar methods)
   - Detect test boilerplate (repetitive test functions with similar structure)
   - Calculate boilerplate confidence score (0.0-1.0)
   - Identify shared method signatures across implementations

2. **Metric Collection**
   - Count trait implementations per file
   - Calculate method name uniformity (% of implementations sharing methods)
   - Measure average method complexity per implementation
   - Calculate complexity variance across methods
   - Track struct-to-implementation ratio

3. **Recommendation Engine**
   - Generate macro-specific recommendations for trait boilerplate
   - Suggest declarative macro patterns with code examples
   - Recommend derive macros where applicable
   - Suggest code generation from schema files (TOML/JSON)
   - Estimate line reduction from macro-ification
   - Provide alternative solutions (e.g., existing crates like `bon`, `typed-builder`)

4. **Integration with Existing Analysis**
   - Enhance god object detection to check for boilerplate first
   - Adjust scoring to de-prioritize simple boilerplate vs complex code
   - Preserve existing module-splitting recommendations for complex code
   - Add new debt category: "BOILERPLATE PATTERN"

5. **Configuration**
   - Allow users to enable/disable boilerplate detection
   - Configure sensitivity thresholds (min impl blocks, uniformity %, etc.)
   - Set confidence threshold for showing boilerplate recommendations
   - Configure which patterns to detect (trait impls, builders, tests)

### Non-Functional Requirements

1. **Performance**
   - Boilerplate detection must add <5% to analysis time
   - Should handle files with 1000+ functions efficiently
   - Use parallel analysis where possible

2. **Accuracy**
   - Minimize false positives (complex code misidentified as boilerplate)
   - Minimize false negatives (boilerplate missed)
   - Target 90%+ accuracy on known boilerplate patterns

3. **Usability**
   - Clear distinction in output between boilerplate and complexity
   - Actionable recommendations with code examples
   - Educational content explaining when to use macros vs modules

4. **Maintainability**
   - Modular design allowing new pattern detectors to be added
   - Well-documented detection algorithms
   - Comprehensive test coverage with real-world examples

## Acceptance Criteria

- [ ] **Trait Implementation Detection**: Correctly identifies files with 20+ trait implementations and 70%+ method uniformity as boilerplate
- [ ] **Ripgrep Test Case**: Analyzing ripgrep's `defs.rs` produces "BOILERPLATE PATTERN" classification with 85%+ confidence
- [ ] **False Positive Prevention**: Complex files (e.g., `src/priority/unified_scorer.rs`) are NOT classified as boilerplate
- [ ] **Macro Recommendations**: Output includes specific macro suggestions with before/after code examples
- [ ] **Line Reduction Estimates**: Recommendations include estimated line count reduction (e.g., "7775 → ~1500 lines")
- [ ] **Configuration Support**: Users can adjust detection thresholds via config file
- [ ] **Performance**: Boilerplate detection adds <5% overhead to total analysis time
- [ ] **Documentation**: User guide chapter explaining boilerplate vs complexity distinction
- [ ] **Integration**: Boilerplate detection integrated into existing god object analysis without breaking changes
- [ ] **Test Coverage**: Comprehensive tests covering trait impls, builders, tests, and edge cases

## Technical Details

### Implementation Approach

#### Phase 1: Core Detection Infrastructure

**New Module**: `src/organization/boilerplate_detector.rs`

```rust
pub struct BoilerplateDetector {
    min_impl_blocks: usize,          // default: 20
    method_uniformity_threshold: f64, // default: 0.7 (70%)
    max_avg_complexity: f64,         // default: 2.0
    confidence_threshold: f64,       // default: 0.7
}

pub struct BoilerplateAnalysis {
    pub is_boilerplate: bool,
    pub confidence: f64,
    pub pattern_type: Option<BoilerplatePattern>,
    pub signals: Vec<DetectionSignal>,
    pub recommendation: String,
}

pub enum BoilerplatePattern {
    TraitImplementation {
        trait_name: String,
        impl_count: usize,
        shared_methods: Vec<String>,
        method_uniformity: f64,
    },
    BuilderPattern {
        builder_count: usize,
    },
    TestBoilerplate {
        test_count: usize,
        shared_structure: String,
    },
}

pub enum DetectionSignal {
    HighImplCount(usize),
    HighMethodUniformity(f64),
    LowAvgComplexity(f64),
    HighStructDensity(usize),
    LowComplexityVariance(f64),
}
```

**New Module**: `src/organization/trait_pattern_analyzer.rs`

```rust
pub struct TraitPatternAnalyzer;

impl TraitPatternAnalyzer {
    /// Analyze file for trait implementation patterns
    pub fn analyze_file(ast: &syn::File) -> TraitPatternMetrics {
        // Extract all impl blocks
        // Group by trait name
        // Calculate method uniformity
        // Return metrics
    }

    /// Calculate percentage of impls sharing the same methods
    pub fn calculate_method_uniformity(impls: &[ImplBlock]) -> f64 {
        // Count method names across all impls
        // Return % of most common methods
    }

    /// Identify methods that appear in most implementations
    pub fn detect_shared_methods(impls: &[ImplBlock]) -> Vec<(String, f64)> {
        // Return (method_name, frequency_percentage)
    }
}

pub struct TraitPatternMetrics {
    pub impl_block_count: usize,
    pub unique_traits: HashSet<String>,
    pub most_common_trait: Option<(String, usize)>,
    pub method_uniformity: f64,
    pub shared_methods: Vec<(String, f64)>,
    pub avg_method_complexity: f64,
    pub complexity_variance: f64,
    pub avg_method_lines: f64,
}
```

#### Phase 2: Scoring Algorithm

```rust
impl BoilerplateDetector {
    /// Calculate boilerplate score (0.0-100.0)
    fn calculate_score(&self, metrics: &FileMetrics) -> f64 {
        let mut score = 0.0;
        let mut max_score = 0.0;

        // Signal 1: Many trait implementations (weight: 30%)
        if metrics.impl_block_count > self.min_impl_blocks {
            let normalized = (metrics.impl_block_count as f64 / 100.0).min(1.0);
            score += 30.0 * normalized;
        }
        max_score += 30.0;

        // Signal 2: Method uniformity (weight: 25%)
        if metrics.method_uniformity > self.method_uniformity_threshold {
            score += 25.0 * metrics.method_uniformity;
        }
        max_score += 25.0;

        // Signal 3: Low complexity (weight: 20%)
        if metrics.avg_method_complexity < self.max_avg_complexity {
            let inverse_complexity = 1.0 - (metrics.avg_method_complexity / self.max_avg_complexity);
            score += 20.0 * inverse_complexity;
        }
        max_score += 20.0;

        // Signal 4: High struct density (weight: 15%)
        if metrics.struct_count > 10 {
            let normalized = (metrics.struct_count as f64 / 100.0).min(1.0);
            score += 15.0 * normalized;
        }
        max_score += 15.0;

        // Signal 5: Low complexity variance (weight: 10%)
        if metrics.complexity_variance < 2.0 {
            let normalized = 1.0 - (metrics.complexity_variance / 10.0).min(1.0);
            score += 10.0 * normalized;
        }
        max_score += 10.0;

        (score / max_score) * 100.0
    }
}
```

#### Phase 3: Integration with God Object Detection

**Modify**: `src/organization/god_object_detector.rs`

```rust
impl GodObjectDetector {
    pub fn classify_god_object(...) -> GodObjectType {
        // NEW: Check for boilerplate first
        let boilerplate_detector = BoilerplateDetector::default();
        let boilerplate_analysis = boilerplate_detector.detect(path, ast);

        if boilerplate_analysis.confidence > 0.7 {
            return GodObjectType::BoilerplatePattern {
                pattern: boilerplate_analysis.pattern_type,
                recommendation: boilerplate_analysis.recommendation,
                confidence: boilerplate_analysis.confidence,
            };
        }

        // Existing logic for GodClass vs GodModule
        // ...
    }
}
```

**Extend**: `src/organization/mod.rs`

```rust
pub enum GodObjectType {
    GodClass { /* existing */ },
    GodModule { /* existing */ },
    // NEW:
    BoilerplatePattern {
        pattern: Option<BoilerplatePattern>,
        recommendation: String,
        confidence: f64,
    },
}
```

#### Phase 4: Recommendation Generation

**New Module**: `src/organization/macro_recommendations.rs`

```rust
pub struct MacroRecommendationEngine;

impl MacroRecommendationEngine {
    pub fn generate_recommendation(
        pattern: &BoilerplatePattern,
        file_path: &Path,
    ) -> String {
        match pattern {
            BoilerplatePattern::TraitImplementation { trait_name, impl_count, .. } => {
                self.generate_trait_macro_recommendation(trait_name, *impl_count, file_path)
            }
            // Other patterns...
        }
    }

    fn generate_trait_macro_recommendation(
        &self,
        trait_name: &str,
        impl_count: usize,
        file_path: &Path,
    ) -> String {
        let current_lines = impl_count * 75; // estimate
        let macro_lines = impl_count * 8;    // estimate
        let reduction_pct = ((current_lines - macro_lines) as f64 / current_lines as f64) * 100.0;

        format!(
            "BOILERPLATE DETECTED: {} implementations of {} trait.\n\
             \n\
             This file contains repetitive trait implementations that should be \n\
             macro-ified or code-generated.\n\
             \n\
             RECOMMENDED APPROACH:\n\
             1. Create a declarative macro to generate {} implementations\n\
             2. Replace {} trait impl blocks with macro invocations\n\
             3. Expected reduction: {} lines → ~{} lines ({:.0}% reduction)\n\
             \n\
             EXAMPLE TRANSFORMATION:\n\
             \n\
             // Before: ~75 lines per implementation\n\
             struct MyFlag;\n\
             impl {} for MyFlag {{\n\
             fn name_long(&self) -> &'static str {{ \"my-flag\" }}\n\
             // ... 70 more boilerplate lines\n\
             }}\n\
             \n\
             // After: ~8 lines per implementation\n\
             flag! {{\n\
             MyFlag {{\n\
             long: \"my-flag\",\n\
             short: 'm',\n\
             // ... declarative config\n\
             }}\n\
             }}\n\
             \n\
             ALTERNATIVES:\n\
             - Build-time code generation from schema file (JSON/TOML)\n\
             - Use existing derive macro crates if applicable\n\
             \n\
             CONFIDENCE: {:.0}%",
            impl_count,
            trait_name,
            trait_name,
            impl_count,
            current_lines,
            macro_lines,
            reduction_pct,
            trait_name,
            (impl_count as f64 / 104.0 * 95.0) // confidence estimate
        )
    }
}
```

### Architecture Changes

1. **New Modules**:
   - `src/organization/boilerplate_detector.rs` - Core detection logic
   - `src/organization/trait_pattern_analyzer.rs` - Trait pattern analysis
   - `src/organization/macro_recommendations.rs` - Recommendation generation

2. **Modified Modules**:
   - `src/organization/god_object_detector.rs` - Integration with boilerplate detection
   - `src/organization/mod.rs` - New enum variants and exports
   - `src/priority/scoring/rust_recommendations.rs` - Handle boilerplate recommendations
   - `src/config.rs` - Add boilerplate detection configuration

3. **Data Flow**:
   ```
   File Analysis
   ↓
   God Object Detection
   ↓
   Boilerplate Detection (NEW)
   ↓
   Classification:
   - BoilerplatePattern → Macro recommendations
   - GodClass → Split class
   - GodModule → Split module
   ↓
   Output Formatting
   ```

### Data Structures

```rust
// Configuration (add to src/config.rs)
pub struct BoilerplateDetectionConfig {
    pub enabled: bool,
    pub min_impl_blocks: usize,          // default: 20
    pub method_uniformity_threshold: f64, // default: 0.7
    pub max_avg_complexity: f64,         // default: 2.0
    pub confidence_threshold: f64,       // default: 0.7
    pub detect_trait_impls: bool,        // default: true
    pub detect_builders: bool,           // default: true
    pub detect_test_boilerplate: bool,   // default: true
}

impl Default for BoilerplateDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_impl_blocks: 20,
            method_uniformity_threshold: 0.7,
            max_avg_complexity: 2.0,
            confidence_threshold: 0.7,
            detect_trait_impls: true,
            detect_builders: true,
            detect_test_boilerplate: true,
        }
    }
}
```

### APIs and Interfaces

```rust
// Public API
pub trait BoilerplateDetection {
    fn detect(&self, path: &Path, ast: &syn::File) -> BoilerplateAnalysis;
    fn calculate_confidence(&self, metrics: &FileMetrics) -> f64;
    fn generate_recommendation(&self, analysis: &BoilerplateAnalysis) -> String;
}

impl BoilerplateDetection for BoilerplateDetector {
    // Implementation
}
```

## Dependencies

### Prerequisites
- None (builds on existing god object detection)

### Affected Components
- `src/organization/god_object_detector.rs` - Enhanced classification
- `src/priority/scoring/rust_recommendations.rs` - New recommendation types
- `src/config.rs` - New configuration section
- `src/io/writers/enhanced_markdown/recommendation_writer.rs` - Output formatting

### External Dependencies
- No new external crates required
- Uses existing `syn` for AST analysis
- Uses existing `im` for immutable collections

## Testing Strategy

### Unit Tests

**File**: `tests/boilerplate_detection_test.rs`

```rust
#[test]
fn test_ripgrep_defs_trait_boilerplate() {
    let content = read_to_string("../ripgrep/crates/core/flags/defs.rs").unwrap();
    let ast = syn::parse_file(&content).unwrap();

    let detector = BoilerplateDetector::default();
    let analysis = detector.detect(Path::new("defs.rs"), &ast);

    assert!(analysis.is_boilerplate);
    assert!(analysis.confidence > 0.8);

    match analysis.pattern_type {
        Some(BoilerplatePattern::TraitImplementation {
            trait_name, impl_count, method_uniformity, ..
        }) => {
            assert_eq!(trait_name, "Flag");
            assert!(impl_count > 100);
            assert!(method_uniformity > 0.9);
        }
        _ => panic!("Expected TraitImplementation pattern"),
    }

    assert!(analysis.recommendation.contains("macro"));
    assert!(analysis.recommendation.contains("81% reduction"));
}

#[test]
fn test_complex_code_not_boilerplate() {
    let content = read_to_string("src/priority/unified_scorer.rs").unwrap();
    let ast = syn::parse_file(&content).unwrap();

    let detector = BoilerplateDetector::default();
    let analysis = detector.detect(Path::new("unified_scorer.rs"), &ast);

    assert!(!analysis.is_boilerplate || analysis.confidence < 0.5);
}

#[test]
fn test_method_uniformity_calculation() {
    // Test with synthetic AST
    let impls = vec![
        create_test_impl(vec!["method_a", "method_b", "method_c"]),
        create_test_impl(vec!["method_a", "method_b", "method_c"]),
        create_test_impl(vec!["method_a", "method_b"]),
    ];

    let uniformity = TraitPatternAnalyzer::calculate_method_uniformity(&impls);
    assert!((uniformity - 0.66).abs() < 0.1); // 2/3 have all methods
}

#[test]
fn test_boilerplate_scoring() {
    let metrics = FileMetrics {
        impl_block_count: 50,
        method_uniformity: 0.85,
        avg_method_complexity: 1.5,
        struct_count: 50,
        complexity_variance: 1.0,
    };

    let detector = BoilerplateDetector::default();
    let score = detector.calculate_score(&metrics);

    assert!(score > 75.0); // High score for clear boilerplate
}

#[test]
fn test_configuration_thresholds() {
    let config = BoilerplateDetectionConfig {
        min_impl_blocks: 50,
        confidence_threshold: 0.9,
        ..Default::default()
    };

    let detector = BoilerplateDetector::from_config(&config);
    // Test with edge cases
}
```

### Integration Tests

**File**: `tests/boilerplate_integration_test.rs`

```rust
#[test]
fn test_end_to_end_boilerplate_detection() {
    // Run full debtmap analysis on ripgrep
    let config = Config::default();
    let results = analyze_project("../ripgrep", &config).unwrap();

    // Find defs.rs in results
    let defs_item = results.debt_items.iter()
        .find(|item| item.location.file.contains("defs.rs"))
        .expect("Should find defs.rs");

    // Verify classification
    match &defs_item.god_object_type {
        Some(GodObjectType::BoilerplatePattern { confidence, .. }) => {
            assert!(*confidence > 0.8);
        }
        _ => panic!("Expected BoilerplatePattern classification"),
    }

    // Verify recommendation content
    assert!(defs_item.recommendation.contains("macro"));
    assert!(defs_item.recommendation.contains("EXAMPLE TRANSFORMATION"));
}
```

### Performance Tests

```rust
#[test]
fn test_boilerplate_detection_performance() {
    let large_file = generate_synthetic_file_with_impls(500); // 500 trait impls
    let start = Instant::now();

    let detector = BoilerplateDetector::default();
    let _ = detector.detect(Path::new("synthetic.rs"), &large_file);

    let duration = start.elapsed();
    assert!(duration < Duration::from_millis(100)); // Should be fast
}
```

## Documentation Requirements

### Code Documentation

1. **Module-level docs** for all new modules explaining:
   - Purpose and design rationale
   - Key algorithms and their complexity
   - Usage examples
   - Configuration options

2. **Function-level docs** with:
   - Purpose and behavior
   - Parameter descriptions
   - Return value descriptions
   - Example usage for public APIs

3. **Algorithm documentation**:
   - Scoring algorithm explanation
   - Why each signal is weighted as it is
   - Edge cases and limitations

### User Documentation

**To be created upon implementation**: `book/src/boilerplate-vs-complexity.md`

This chapter should explain:

1. **The Distinction** between complex code and boilerplate:
   - Complex code characteristics (high cyclomatic complexity, diverse logic, irregular structure)
   - Boilerplate characteristics (low complexity, repetitive patterns, regular structure)
   - When to split modules vs when to macro-ify

2. **Real Example**: Ripgrep's flag definitions
   - Show current boilerplate state (~75 lines per flag)
   - Show macro-ified version (~8 lines per flag)
   - Calculate impact (89% line reduction)

3. **Detection Methodology**:
   - Five detection signals and their weights
   - Confidence score calculation
   - Configuration options

4. **Common Patterns**:
   - Trait implementations → declarative macros
   - Builder patterns → `bon`, `typed-builder`
   - Test boilerplate → parameterized tests
   - Serialization → derive macros

5. **Decision Table**:
   - Comparison matrix: complexity, uniformity, structure, purpose
   - Clear guidance on when to use each approach

6. **Best Practices**:
   - Before macro-ification checklist
   - Macro creation guidelines
   - Alternatives to macros

**Update**: `book/src/SUMMARY.md` - Add chapter link when implemented

**Update**: `.prodigy/book-config.json` - Add to `analysis_targets`:
```json
{
  "area": "boilerplate_detection",
  "source_files": [
    "src/organization/boilerplate_detector.rs",
    "src/organization/trait_pattern_analyzer.rs",
    "src/organization/macro_recommendations.rs"
  ],
  "feature_categories": [
    "pattern_detection",
    "trait_analysis",
    "macro_recommendations",
    "boilerplate_scoring"
  ]
}
```

## Implementation Notes

### Algorithm Complexity
- Trait pattern analysis: O(n) where n = number of impl blocks
- Method uniformity: O(n × m) where m = average methods per impl
- Expected performance: <5% overhead for typical files

### Edge Cases

1. **Mixed files** (boilerplate + complex logic):
   - Calculate separate scores for each section
   - Report dominant pattern
   - Suggest splitting if mixed

2. **Generated code**:
   - May be flagged as boilerplate
   - Add option to exclude generated files
   - Detect `#[automatically_derived]` attribute

3. **Macro-heavy codebases**:
   - Already using macros effectively
   - Lower priority for boilerplate recommendation
   - Detect existing macro usage

### False Positive Prevention

1. **Complexity check**: Require avg complexity < 2.0
2. **Uniformity check**: Require >70% shared methods
3. **Count check**: Require 20+ implementations
4. **Confidence threshold**: Only show if >70% confident

### Rust-Specific Considerations

1. **Derive macros**: Detect when standard derives would work
2. **Procedural macros**: Suggest when declarative won't suffice
3. **Build scripts**: Recommend for large-scale generation
4. **Existing crates**: Suggest `bon`, `typed-builder`, `strum`, etc.

## Migration and Compatibility

### Breaking Changes
- None (purely additive feature)

### Configuration Migration
- New `[boilerplate_detection]` section in config
- All options have sensible defaults
- Feature can be disabled with `enabled = false`

### Backward Compatibility
- Existing god object detection unchanged when boilerplate not detected
- Output format extended but compatible
- No changes to existing APIs

### Future Extensions

1. **Additional patterns**:
   - Builder pattern detection
   - Test boilerplate detection
   - Serialization boilerplate

2. **Cross-language support**:
   - Python metaclasses/decorators
   - JavaScript prototypes
   - TypeScript generics

3. **Macro generation**:
   - Auto-generate suggested macros
   - Provide complete refactoring diffs

4. **Integration**:
   - IDE plugins for macro refactoring
   - CI/CD automation for boilerplate metrics

## Success Metrics

- Ripgrep `defs.rs` classified as boilerplate with 85%+ confidence
- Zero false positives on debtmap's own codebase
- User feedback: recommendations are actionable
- Analysis time overhead < 5%
- Documentation clarity: users understand the distinction

## References

- Ripgrep source code: `crates/core/flags/defs.rs`
- Rust macro documentation
- Related tools: rust-analyzer, clippy
- Academic: "Code Smell Detection" literature
