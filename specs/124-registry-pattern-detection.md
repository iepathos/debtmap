---
number: 124
title: Registry/Catalog Pattern Detection
category: optimization
priority: critical
status: draft
dependencies: [111]
created: 2025-10-25
---

# Specification 124: Registry/Catalog Pattern Detection

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 111 (AST Functional Pattern Detection)

## Context

Debtmap currently flags large files with many trait implementations as "GOD MODULES" requiring urgent splitting. This produces critical false positives when analyzing registry/catalog patterns - a common Rust idiom where many unit structs implement the same trait in one centralized location.

**Real-world example from ripgrep**:
- `defs.rs`: 7775 lines, 888 functions
- Flagged as #1 critical issue requiring split into 8 modules
- **Reality**: Centralized flag registry with 100+ unit structs implementing `Flag` trait
- Each implementation: 5-10 lines of simple trait methods
- **Highly cohesive**: Single responsibility (flag definitions)
- **Splitting would harm code quality**: Reduces discoverability and consistency

This false positive appears as the top recommendation, misleading developers into harmful refactoring.

## Objective

Detect registry/catalog patterns in Rust code and apply appropriate scoring adjustments to prevent false positives. When many small trait implementations are centralized in one file for discoverability, this should be recognized as intentional architecture, not technical debt.

## Requirements

### Functional Requirements

1. **Trait Implementation Detection**
   - Identify `impl TraitName for Type` blocks in AST
   - Count trait implementations per file
   - Measure implementation size (lines per impl block)
   - Detect repeated trait implementations (same trait, different types)

2. **Registry Pattern Recognition**
   - Detect files with 20+ trait implementations of the same trait
   - Calculate average implementation size
   - Check for unit struct pattern (`struct Name;` with trait impl)
   - Identify static arrays referencing trait objects (`const ITEMS: &[&dyn Trait]`)

3. **Cohesion Analysis**
   - Verify all implementations serve single domain (e.g., all flag definitions)
   - Check naming consistency across implementations
   - Measure cross-implementation coupling (low for registries)

4. **Pattern Classification**
   - Classify as Registry if:
     - 20+ implementations of same trait
     - Average impl size < 15 lines
     - 80%+ of file is trait implementations
     - Unit struct pattern detected
   - Distinguish from legitimate god objects:
     - Multiple unrelated traits → not registry
     - Large implementation sizes → not registry
     - High cross-function coupling → not registry

### Non-Functional Requirements

- Detection overhead: < 5% of total analysis time
- Pattern recognition accuracy: > 90% precision and recall
- Zero false negatives on legitimate god objects
- Compatible with existing AST analysis pipeline

## Acceptance Criteria

- [ ] Detect trait implementations in Rust AST with accurate line counts
- [ ] Calculate average implementation size across all impls of same trait
- [ ] Identify registry pattern based on impl count, size, and cohesion metrics
- [ ] Apply 70% score reduction for confirmed registry patterns (avg impl size < 15 lines)
- [ ] Flag as "Large Registry" instead of "God Module" with reduced severity
- [ ] Ripgrep's `defs.rs` (888 functions, avg 8 lines/impl) no longer appears as #1 critical issue
- [ ] Registry pattern scoring: severity drops from CRITICAL to LOW/INFO
- [ ] Non-registry god objects still flagged with CRITICAL severity
- [ ] Integration tests validate against ripgrep, servo, and rust-analyzer codebases
- [ ] Documentation includes pattern detection logic and scoring adjustments

## Technical Details

### Implementation Approach

**Phase 1: Trait Implementation Extraction**
```rust
struct TraitImplInfo {
    trait_name: String,
    type_name: String,
    line_count: usize,
    start_line: usize,
    end_line: usize,
    is_unit_struct: bool,
}

fn extract_trait_impls(file_ast: &FileAst) -> Vec<TraitImplInfo> {
    // Parse impl blocks from AST
    // Count lines excluding comments and whitespace
    // Detect unit struct pattern
}
```

**Phase 2: Registry Pattern Detection**
```rust
struct RegistryPattern {
    trait_name: String,
    impl_count: usize,
    avg_impl_size: f64,
    total_lines: usize,
    unit_struct_ratio: f64,
    has_static_registry: bool,
}

fn detect_registry_pattern(
    file_path: &Path,
    trait_impls: &[TraitImplInfo],
    file_metrics: &FileMetrics,
) -> Option<RegistryPattern> {
    // Group impls by trait name
    // Calculate averages and ratios
    // Check for static registry array
    // Return pattern if thresholds met:
    //   - impl_count >= 20
    //   - avg_impl_size < 15 lines
    //   - trait_impl_coverage > 0.80 (80% of file)
}
```

**Phase 3: Scoring Adjustment**
```rust
fn adjust_file_score(
    base_score: f64,
    pattern: &RegistryPattern,
) -> f64 {
    if pattern.avg_impl_size < 10 {
        base_score * 0.2 // 80% reduction - very small impls
    } else if pattern.avg_impl_size < 15 {
        base_score * 0.3 // 70% reduction - small impls
    } else {
        base_score * 0.5 // 50% reduction - moderate impls
    }
}
```

### Architecture Changes

**Extend `FileAnalysis` struct**:
```rust
pub struct FileAnalysis {
    // ... existing fields
    pub trait_implementations: Vec<TraitImplInfo>,
    pub detected_pattern: Option<DetectedPattern>,
}

pub enum DetectedPattern {
    Registry(RegistryPattern),
    Builder(BuilderPattern),  // Spec 125
    StructInitialization(StructInitPattern), // Spec 126
    ParallelExecution(ParallelPattern), // Spec 127
}
```

**Modify debt scoring**:
- Apply pattern detection before scoring
- Adjust severity levels based on pattern
- Include pattern info in recommendation output

### Data Structures

```rust
pub struct RegistryPattern {
    /// Name of the trait being implemented repeatedly
    pub trait_name: String,

    /// Number of implementations found
    pub impl_count: usize,

    /// Average lines per implementation
    pub avg_impl_size: f64,

    /// Standard deviation of impl sizes
    pub impl_size_stddev: f64,

    /// Total lines in file
    pub total_lines: usize,

    /// Percentage of implementations that are unit structs
    pub unit_struct_ratio: f64,

    /// Whether file contains static registry array
    pub has_static_registry: bool,

    /// Coverage: trait impl lines / total lines
    pub trait_impl_coverage: f64,
}
```

### APIs and Interfaces

**Pattern Detection API**:
```rust
pub trait PatternDetector {
    fn detect(&self, analysis: &FileAnalysis) -> Option<DetectedPattern>;
    fn confidence(&self) -> f64;
}

pub struct RegistryPatternDetector {
    min_impl_count: usize,
    max_avg_impl_size: usize,
    min_coverage: f64,
}

impl PatternDetector for RegistryPatternDetector {
    fn detect(&self, analysis: &FileAnalysis) -> Option<DetectedPattern> {
        // Detection logic
    }

    fn confidence(&self) -> f64 {
        // Confidence based on pattern strength
    }
}
```

**Integration with Scoring**:
```rust
pub fn calculate_file_debt_score(
    metrics: &FileMetrics,
    pattern: Option<&DetectedPattern>,
) -> DebtScore {
    let base_score = calculate_base_score(metrics);

    match pattern {
        Some(DetectedPattern::Registry(reg)) => {
            adjust_registry_score(base_score, reg)
        }
        // Other patterns...
        None => base_score,
    }
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
fn test_detect_registry_pattern_ripgrep_defs() {
    // Parse ripgrep's defs.rs
    let analysis = analyze_file("../ripgrep/crates/core/flags/defs.rs");
    let pattern = RegistryPatternDetector::default().detect(&analysis);

    assert!(pattern.is_some());
    let registry = pattern.unwrap();
    assert_eq!(registry.trait_name, "Flag");
    assert!(registry.impl_count > 100);
    assert!(registry.avg_impl_size < 15.0);
    assert!(registry.trait_impl_coverage > 0.80);
}

#[test]
fn test_registry_score_reduction() {
    let pattern = RegistryPattern {
        trait_name: "Flag".into(),
        impl_count: 150,
        avg_impl_size: 8.0,
        total_lines: 7775,
        unit_struct_ratio: 0.95,
        has_static_registry: true,
        trait_impl_coverage: 0.90,
        impl_size_stddev: 2.5,
    };

    let base_score = 1000.0;
    let adjusted = adjust_registry_score(base_score, &pattern);

    // 80% reduction for avg_impl_size < 10
    assert!((adjusted - 200.0).abs() < 1.0);
}

#[test]
fn test_not_registry_multiple_traits() {
    // File with many different trait impls (not registry)
    let analysis = create_analysis_with_mixed_traits();
    let pattern = RegistryPatternDetector::default().detect(&analysis);

    assert!(pattern.is_none(), "Multiple unrelated traits should not be registry");
}

#[test]
fn test_not_registry_large_impls() {
    // File with 50 impls but avg 40 lines each (complex logic, not registry)
    let analysis = create_analysis_with_large_impls();
    let pattern = RegistryPatternDetector::default().detect(&analysis);

    assert!(pattern.is_none(), "Large implementations should not be registry");
}
```

### Integration Tests

- **Ripgrep validation**: Verify `defs.rs` no longer flagged as #1 critical issue
- **False positive regression**: Ensure actual god objects still detected
- **Cross-project validation**: Test against servo, rust-analyzer, tokio
- **Performance benchmark**: Registry detection < 5% overhead

### Performance Tests

```rust
#[bench]
fn bench_registry_detection_large_file(b: &mut Bencher) {
    let ast = parse_file("test_data/large_registry_10k_lines.rs");
    b.iter(|| {
        RegistryPatternDetector::default().detect(&ast)
    });
}
```

## Documentation Requirements

### Code Documentation

- Rustdoc for all pattern detection types and functions
- Explain registry pattern characteristics and detection heuristics
- Document scoring adjustment rationale
- Provide examples of registry vs. god object distinction

### User Documentation

**CLI Output Enhancement**:
```
#1 SCORE: 200 [INFO - FILE - LARGE REGISTRY]
├─ ./crates/core/flags/defs.rs (7775 lines, 150 trait implementations)
├─ PATTERN: Registry/Catalog - Centralized trait implementations
├─ WHY: File contains 150 implementations of the Flag trait (avg 8 lines each).
│       This is an intentional registry pattern for discoverability and
│       consistency, not a god object requiring splitting.
├─ ACTION: Consider if registry has grown too large for navigation:
│  ├─ If trait impls exceed 200, consider logical grouping (e.g., by category)
│  ├─ Ensure consistent naming and documentation across implementations
│  └─ Add table-of-contents or index for discoverability
├─ IMPACT: Low priority - pattern is cohesive and intentional
├─ METRICS: Trait impls: 150, Avg impl size: 8 lines, Coverage: 90%
└─ PATTERN CONFIDENCE: 95%
```

### Architecture Updates

Update `ARCHITECTURE.md`:
- Document pattern detection pipeline
- Explain registry pattern characteristics
- Describe scoring adjustment strategy
- Add decision tree for pattern classification

## Implementation Notes

### Pattern Recognition Challenges

1. **Distinguishing registries from god objects**:
   - Registry: Many small impls of **same** trait
   - God object: Many unrelated methods with **different** purposes

2. **Handling mixed patterns**:
   - File may have registry + helper functions
   - Apply pattern detection to dominant pattern
   - Deduct helper function lines from coverage calculation

3. **Language-specific considerations**:
   - Rust: Trait impls, unit structs, `const` arrays
   - Python: Class registration decorators, metaclasses
   - TypeScript: Interface implementations, class registries

### Heuristic Tuning

**Conservative thresholds** (start here):
- Min impl count: 20
- Max avg impl size: 15 lines
- Min coverage: 80%

**Aggressive thresholds** (if false negatives):
- Min impl count: 15
- Max avg impl size: 20 lines
- Min coverage: 70%

Test against real codebases and adjust based on precision/recall metrics.

### Edge Cases

- **Registry with tests**: Exclude test functions from coverage calculation
- **Registry with macros**: Measure expanded code, not macro invocations
- **Generic trait impls**: Count each generic specialization separately
- **Nested modules in file**: Aggregate impl counts across modules

## Migration and Compatibility

### Breaking Changes

None - this is a new feature that improves existing analysis.

### Backward Compatibility

- Existing debt scores may decrease for files with registry patterns
- Recommendations will shift to lower priority items
- No changes to CLI flags or output format (only content changes)

### Migration Path

1. Deploy pattern detection alongside existing scoring
2. Log pattern detection results for analysis
3. Validate against known codebases (ripgrep, servo, tokio)
4. Enable scoring adjustments in production
5. Monitor false positive/negative rates

### Configuration

Add optional configuration for pattern detection:

```toml
[pattern_detection]
enabled = true

[pattern_detection.registry]
min_impl_count = 20
max_avg_impl_size = 15
min_coverage = 0.80
score_reduction = 0.70  # 70% score reduction
```

## Success Metrics

- **False positive reduction**: 60-70% reduction in god object false positives
- **Ripgrep validation**: `defs.rs` score drops from #1 (score: 1422) to <100
- **Pattern detection accuracy**: >90% precision and recall
- **Performance**: <5% analysis overhead
- **User feedback**: Developers confirm registry recommendations are accurate
