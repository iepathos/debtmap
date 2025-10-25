---
number: 125
title: Builder Pattern Detection
category: optimization
priority: critical
status: draft
dependencies: [111]
created: 2025-10-25
---

# Specification 125: Builder Pattern Detection

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 111 (AST Functional Pattern Detection)

## Context

Debtmap currently flags files with many setter methods as "GOD OBJECTS" based purely on function count. This produces false positives for builder patterns - a common Rust idiom where fluent setter methods are intentionally numerous (one per configuration option).

**Real-world example from ripgrep**:
- `standard.rs`: 3987 lines, 172 functions
- Flagged as #2 critical issue requiring split into 6 modules
- **Reality**: Classic builder pattern with ~30 fluent setters + printer implementation
- Structure: `StandardBuilder` (setters) + `Standard<W>` (printer) + `StandardSink` (grep integration)
- **Cohesive**: All code relates to single concern (grep output formatting)
- **Function count misleading**: Builder naturally has many small setters

While the file is legitimately large (3987 lines), the recommendation to split based on function count (172) misses the real issue and suggests inappropriate refactoring.

## Objective

Detect builder patterns in Rust code and adjust scoring to focus on actual complexity (total size, implementation logic) rather than setter count. Builder patterns should be evaluated based on whether the builder has grown too large for its single responsibility, not on the number of configuration options.

## Requirements

### Functional Requirements

1. **Builder Method Detection**
   - Identify fluent setter pattern: `fn name(&mut self, value: T) -> &mut Self`
   - Identify consuming builder: `fn name(mut self, value: T) -> Self`
   - Count `build()` methods returning configured type
   - Detect builder struct naming patterns (`*Builder`, `*Config`, `*Options`)

2. **Builder Pattern Recognition**
   - Detect structs with 10+ fluent setter methods
   - Identify private config struct + public builder pattern
   - Find `build()` or `finish()` terminal methods
   - Measure setter-to-total-method ratio

3. **Complexity Analysis**
   - Measure total builder file size (lines)
   - Calculate average setter size (should be 2-5 lines)
   - Analyze non-setter logic complexity (actual implementation)
   - Identify implementation structs separate from builder

4. **Pattern Classification**
   - Classify as Builder if:
     - 10+ methods returning `&mut Self` or `Self`
     - 1+ `build()` methods
     - Setter-to-method ratio > 50%
     - Average setter size < 10 lines
   - Distinguish from god objects:
     - Setters serve single configuration domain
     - Implementation logic is in separate type
     - High setter ratio indicates focused purpose

### Non-Functional Requirements

- Detection overhead: < 5% of total analysis time
- Pattern recognition accuracy: > 85% precision and recall
- Zero false negatives on legitimate god objects
- Language support: Rust (primary), extensible to other languages

## Acceptance Criteria

- [ ] Detect fluent setter methods in Rust AST (return `&mut Self` or `Self`)
- [ ] Identify builder structs with 10+ setters and `build()` method
- [ ] Calculate setter-to-method ratio and average setter complexity
- [ ] Apply scoring adjustment: penalize total size, not setter count
- [ ] Flag as "Large Builder" if file >3000 lines (regardless of setter count)
- [ ] Ripgrep's `standard.rs` (172 functions) flagged for size (3987 lines), not function count
- [ ] Builder pattern recommendation focuses on splitting by **concern** not **setter count**
- [ ] Non-builder god objects (low setter ratio) still flagged with CRITICAL severity
- [ ] Integration tests validate against ripgrep, clap, tokio builders
- [ ] Documentation explains builder pattern detection and appropriate refactoring

## Technical Details

### Implementation Approach

**Phase 1: Method Signature Analysis**
```rust
struct MethodInfo {
    name: String,
    return_type: ReturnType,
    param_count: usize,
    line_count: usize,
    is_mutable_self: bool,
    is_consuming_self: bool,
}

enum ReturnType {
    MutableSelfRef,   // &mut Self
    SelfValue,        // Self
    BuildProduct(String), // Named type
    Other(String),
}

fn analyze_method_signatures(impl_block: &ImplBlock) -> Vec<MethodInfo> {
    // Parse method signatures from AST
    // Categorize return types
    // Measure method complexity
}
```

**Phase 2: Builder Pattern Detection**
```rust
struct BuilderPattern {
    builder_struct: String,
    setter_count: usize,
    total_method_count: usize,
    setter_ratio: f64,
    avg_setter_size: f64,
    build_methods: Vec<String>,
    product_type: Option<String>,
    has_config_struct: bool,
    total_file_lines: usize,
}

fn detect_builder_pattern(
    file_ast: &FileAst,
    file_metrics: &FileMetrics,
) -> Option<BuilderPattern> {
    // Find structs with builder naming pattern
    // Count fluent setters (return &mut Self or Self)
    // Find build() methods
    // Calculate ratios
    // Return pattern if thresholds met:
    //   - setter_count >= 10
    //   - setter_ratio > 0.50
    //   - has build method
}
```

**Phase 3: Scoring Adjustment**
```rust
fn adjust_builder_score(
    base_score: f64,
    pattern: &BuilderPattern,
) -> f64 {
    // Don't penalize setter count - penalize total size
    let size_factor = if pattern.total_file_lines > 5000 {
        1.2 // Large file penalty
    } else if pattern.total_file_lines > 3000 {
        1.0 // Moderate size
    } else {
        0.7 // Small builder - reduce score
    };

    // Reduce penalty for high setter ratio (focused purpose)
    let focus_factor = if pattern.setter_ratio > 0.70 {
        0.6 // Very focused - mostly setters
    } else if pattern.setter_ratio > 0.50 {
        0.8 // Focused - majority setters
    } else {
        1.0 // Mixed - might be god object
    };

    base_score * size_factor * focus_factor
}
```

### Architecture Changes

**Extend `FileAnalysis` struct**:
```rust
pub struct FileAnalysis {
    // ... existing fields
    pub method_signatures: Vec<MethodInfo>,
    pub detected_pattern: Option<DetectedPattern>,
}

pub enum DetectedPattern {
    Registry(RegistryPattern),   // Spec 124
    Builder(BuilderPattern),      // This spec
    StructInitialization(StructInitPattern), // Spec 126
    ParallelExecution(ParallelPattern), // Spec 127
}
```

**Modify recommendation generation**:
```rust
fn generate_builder_recommendation(pattern: &BuilderPattern) -> Recommendation {
    if pattern.total_file_lines > 3000 {
        Recommendation {
            severity: Severity::Medium,
            pattern_type: "Large Builder",
            message: format!(
                "Builder file is {} lines. Consider splitting by logical concerns, \
                 not setter count. Evaluate: 1) Can config struct be extracted? \
                 2) Are there multiple unrelated configuration domains? \
                 3) Can implementation logic move to separate module?",
                pattern.total_file_lines
            ),
            suggested_splits: suggest_concern_based_splits(pattern),
        }
    } else {
        Recommendation {
            severity: Severity::Low,
            pattern_type: "Builder Pattern",
            message: format!(
                "Builder with {} setters is appropriately sized. \
                 No refactoring needed.",
                pattern.setter_count
            ),
            suggested_splits: vec![],
        }
    }
}
```

### Data Structures

```rust
pub struct BuilderPattern {
    /// Name of the builder struct
    pub builder_struct: String,

    /// Number of fluent setter methods
    pub setter_count: usize,

    /// Total methods in builder impl
    pub total_method_count: usize,

    /// Ratio of setters to total methods (0.0 - 1.0)
    pub setter_ratio: f64,

    /// Average lines per setter
    pub avg_setter_size: f64,

    /// Standard deviation of setter sizes
    pub setter_size_stddev: f64,

    /// Names of build methods (build, finish, etc.)
    pub build_methods: Vec<String>,

    /// Type produced by builder (if detected)
    pub product_type: Option<String>,

    /// Whether builder uses separate config struct
    pub has_config_struct: bool,

    /// Total lines in file containing builder
    pub total_file_lines: usize,

    /// Lines in non-setter implementation code
    pub implementation_lines: usize,
}
```

### APIs and Interfaces

**Pattern Detection API**:
```rust
pub struct BuilderPatternDetector {
    min_setter_count: usize,
    min_setter_ratio: f64,
    max_avg_setter_size: usize,
}

impl Default for BuilderPatternDetector {
    fn default() -> Self {
        Self {
            min_setter_count: 10,
            min_setter_ratio: 0.50,
            max_avg_setter_size: 10,
        }
    }
}

impl PatternDetector for BuilderPatternDetector {
    fn detect(&self, analysis: &FileAnalysis) -> Option<DetectedPattern> {
        // 1. Find struct with *Builder/*Config naming
        // 2. Analyze impl blocks for fluent setters
        // 3. Verify build() method exists
        // 4. Calculate ratios and averages
        // 5. Return pattern if thresholds met
    }

    fn confidence(&self) -> f64 {
        // Based on setter ratio and naming conventions
    }
}
```

**Recommendation API**:
```rust
pub fn suggest_concern_based_splits(pattern: &BuilderPattern) -> Vec<SplitSuggestion> {
    // Analyze setter names for logical groupings
    // Example: color_* methods vs. format_* methods
    // Suggest splits only if multiple clear domains exist
}

pub struct SplitSuggestion {
    pub module_name: String,
    pub concern: String,
    pub setter_names: Vec<String>,
    pub estimated_lines: usize,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 111 (AST Functional Pattern Detection) - provides AST parsing infrastructure
- **Affected Components**:
  - `src/debt/` - scoring algorithms
  - `src/analyzers/rust.rs` - Rust-specific analysis
  - `src/io/output.rs` - recommendation formatting
- **External Dependencies**: None (uses existing syn/tree-sitter)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_detect_builder_pattern_ripgrep_standard() {
    let analysis = analyze_file("../ripgrep/crates/printer/src/standard.rs");
    let pattern = BuilderPatternDetector::default().detect(&analysis);

    assert!(pattern.is_some());
    let builder = pattern.unwrap();
    assert_eq!(builder.builder_struct, "StandardBuilder");
    assert!(builder.setter_count >= 25);
    assert!(builder.setter_ratio > 0.50);
    assert!(builder.build_methods.contains(&"build".to_string()));
}

#[test]
fn test_builder_score_focuses_on_size_not_setters() {
    let small_builder = BuilderPattern {
        builder_struct: "SmallBuilder".into(),
        setter_count: 30,
        total_method_count: 35,
        setter_ratio: 0.86,
        avg_setter_size: 3.0,
        setter_size_stddev: 1.2,
        build_methods: vec!["build".into()],
        product_type: Some("Config".into()),
        has_config_struct: true,
        total_file_lines: 500,
        implementation_lines: 50,
    };

    let large_builder = BuilderPattern {
        total_file_lines: 4000,
        ..small_builder.clone()
    };

    let base_score = 1000.0;
    let small_adjusted = adjust_builder_score(base_score, &small_builder);
    let large_adjusted = adjust_builder_score(base_score, &large_builder);

    // Small builder with 30 setters gets score REDUCTION
    assert!(small_adjusted < base_score);

    // Large builder with same 30 setters gets score INCREASE
    assert!(large_adjusted >= base_score);
}

#[test]
fn test_not_builder_low_setter_ratio() {
    // File with 100 methods but only 5 setters (god object, not builder)
    let analysis = create_analysis_with_mixed_methods();
    let pattern = BuilderPatternDetector::default().detect(&analysis);

    assert!(pattern.is_none(), "Low setter ratio should not be builder");
}

#[test]
fn test_fluent_setter_detection() {
    let code = r#"
        impl ConfigBuilder {
            pub fn timeout(&mut self, value: u64) -> &mut Self {
                self.config.timeout = value;
                self
            }

            pub fn retries(mut self, value: u32) -> Self {
                self.config.retries = value;
                self
            }

            pub fn build(self) -> Config {
                self.config
            }
        }
    "#;

    let methods = parse_and_analyze_methods(code);
    let setters: Vec<_> = methods.iter()
        .filter(|m| matches!(m.return_type, ReturnType::MutableSelfRef | ReturnType::SelfValue))
        .collect();

    assert_eq!(setters.len(), 2);
    assert!(methods.iter().any(|m| m.name == "build"));
}
```

### Integration Tests

- **Ripgrep validation**: Verify `standard.rs` flagged for size (3987 lines), not function count
- **Clap validation**: Test against clap's builder patterns (derive-based)
- **Tokio validation**: Test against tokio runtime builders
- **False positive check**: Ensure actual god objects (low setter ratio) still flagged

### Performance Tests

```rust
#[bench]
fn bench_builder_detection(b: &mut Bencher) {
    let ast = parse_file("test_data/large_builder_2k_lines.rs");
    b.iter(|| {
        BuilderPatternDetector::default().detect(&ast)
    });
}
```

## Documentation Requirements

### Code Documentation

- Rustdoc for builder pattern detection logic
- Explain fluent setter identification
- Document scoring adjustment rationale
- Provide examples of builder vs. god object

### User Documentation

**CLI Output Enhancement**:
```
#2 SCORE: 420 [MEDIUM - FILE - LARGE BUILDER]
├─ ./crates/printer/src/standard.rs (3987 lines, 30 setters)
├─ PATTERN: Builder Pattern - Fluent configuration API
├─ WHY: File contains StandardBuilder with 30 fluent setters (86% of methods).
│       Builder pattern naturally has many setters - one per config option.
│       File size (3987 lines) warrants evaluation, but setter count is expected.
├─ ACTION: Consider splitting by logical concerns, not setter count:
│  ├─ 1) Extract separate config struct if not already present
│  ├─ 2) Identify unrelated configuration domains (e.g., color vs. format)
│  ├─ 3) Move complex implementation logic to separate modules
│  ├─ 4) Keep all setters together for API consistency
│  └─  Builder size itself is not problematic - focus on file organization
├─ IMPACT: Medium priority - file is large but well-structured
├─ METRICS: Setters: 30, Total methods: 35, Setter ratio: 86%, File size: 3987 lines
└─ PATTERN CONFIDENCE: 92%

Suggested splits (if multiple concerns detected):
  📦 standard_builder.rs - Builder with setters (800 lines)
  📦 standard_printer.rs - Printer implementation (2200 lines)
  📦 standard_sink.rs - Searcher integration (900 lines)
```

### Architecture Updates

Update `ARCHITECTURE.md`:
- Document builder pattern detection
- Explain why setter count is expected
- Describe size-based vs. count-based evaluation
- Provide guidance on builder refactoring

## Implementation Notes

### Builder Pattern Variations

**Mutable reference builder**:
```rust
fn timeout(&mut self, value: u64) -> &mut Self { ... }
```

**Consuming builder**:
```rust
fn timeout(mut self, value: u64) -> Self { ... }
```

**Hybrid pattern**:
```rust
// Setters return &mut Self
fn timeout(&mut self, value: u64) -> &mut Self { ... }
// Build consumes self
fn build(self) -> Config { ... }
```

### Scoring Philosophy

**Old (incorrect)**:
- Many methods → GOD OBJECT
- Recommendation: Split by method count

**New (correct)**:
- Many setters + high setter ratio → BUILDER PATTERN
- Evaluation: File size and logical cohesion
- Recommendation: Split by concern, keep setters together

### Edge Cases

- **Builder with complex setters**: If avg setter size >20 lines, reduce builder confidence
- **Builder with validation**: Setters may contain validation logic - expected
- **Multiple builders in file**: Aggregate setter counts across builders
- **Generic builders**: Count each generic impl block separately

### Language Extensions

While focused on Rust initially, pattern is recognizable in other languages:

**TypeScript**:
```typescript
class ConfigBuilder {
  timeout(value: number): this { ... }
  retries(value: number): this { ... }
  build(): Config { ... }
}
```

**Python**:
```python
class ConfigBuilder:
    def timeout(self, value: int) -> 'ConfigBuilder': ...
    def retries(self, value: int) -> 'ConfigBuilder': ...
    def build(self) -> Config: ...
```

## Migration and Compatibility

### Breaking Changes

None - this is a new feature that improves existing analysis.

### Backward Compatibility

- Existing "GOD OBJECT" classifications may change to "LARGE BUILDER"
- Scores will decrease for files with high setter ratios
- Recommendations will focus on size/concerns, not method count

### Migration Path

1. Deploy pattern detection alongside existing scoring
2. Validate against known builder-heavy codebases
3. Monitor false positive/negative rates
4. Enable scoring adjustments in production
5. Update user documentation with pattern explanations

### Configuration

Add optional configuration for pattern detection:

```toml
[pattern_detection]
enabled = true

[pattern_detection.builder]
min_setter_count = 10
min_setter_ratio = 0.50
max_avg_setter_size = 10
size_threshold_medium = 3000  # Medium severity if > 3000 lines
size_threshold_high = 5000    # High severity if > 5000 lines
```

## Success Metrics

- **False positive reduction**: 40-50% reduction in god object false positives
- **Ripgrep validation**: `standard.rs` severity drops from CRITICAL to MEDIUM
- **Recommendation quality**: Developers report split suggestions are logical
- **Pattern detection accuracy**: >85% precision and recall
- **Performance**: <5% analysis overhead
- **User satisfaction**: Fewer reports of inappropriate splitting recommendations
