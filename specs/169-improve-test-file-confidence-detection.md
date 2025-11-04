---
number: 169
title: Improve Test File Confidence Detection for Component Tests
category: optimization
priority: high
status: draft
dependencies: [166, 168]
created: 2025-11-03
---

# Specification 169: Improve Test File Confidence Detection for Component Tests

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [166 - Test File Detection, 168 - File-Level Context Scoring]

## Context

**Problem**: Test files in `src/` directories (component tests) score too high due to insufficient confidence.

**Current Behavior**:
```
#3 SCORE: 51.9 [CRITICAL]
└─ ./src/cook/workflow/git_context_diff_tests.rs
```

**Expected Behavior**:
```
#50~ SCORE: 17.3 [MEDIUM]
└─ ./src/cook/workflow/git_context_diff_tests.rs [TEST FILE]
```

**Root Cause**: Confidence calculation over-weights directory location (40%):

```rust
overall_confidence =
    directory × 0.40 +     // ← 0.0 (NOT in tests/)
    attributes × 0.30 +    // ← 1.0 (#[test] attributes present)
    naming × 0.15 +        // ← 0.9 (_tests.rs suffix)
    functions × 0.10 +     // ← 1.0 (test_ prefix functions)
    imports × 0.05         // ← 1.0 (test framework imports)
```

For `src/cook/workflow/git_context_diff_tests.rs`:
```
0.0 × 0.40 + 1.0 × 0.30 + 0.9 × 0.15 + 1.0 × 0.10 + 1.0 × 0.05
= 0.585  (probable test → 40% reduction)
```

Should be:
```
>0.8 confidence (high confidence test → 80% reduction)
```

**Impact**: Component tests score 51.9 instead of 17.3, staying in top 10.

## Objective

Adjust test file confidence calculation to achieve >0.8 confidence for component tests with strong naming and attribute signals, even when not in `tests/` directory.

## Requirements

### Functional Requirements

1. **Rebalanced Weighting Formula**

   Reduce directory dominance, increase attribute/naming weight:
   ```rust
   overall_confidence =
       directory × 0.20 +      // ← Reduced from 0.40
       attributes × 0.35 +     // ← Increased from 0.30
       naming × 0.25 +         // ← Increased from 0.15
       functions × 0.15 +      // ← Increased from 0.10
       imports × 0.05          // ← Unchanged
   ```

2. **Strong Naming Boost**

   Files ending in `_tests.rs`/`_test.rs` get confidence boost:
   ```rust
   if filename.ends_with("_tests.rs") || filename.ends_with("_test.rs") {
       naming_score = 1.0;  // Perfect naming score
       naming_boost = 0.10; // Additional confidence boost
   }
   ```

3. **Multiple Strong Signals Rule**

   If 3+ signals are strong (>0.7), boost to high confidence:
   ```rust
   let strong_signals = [
       naming >= 0.7,
       attributes >= 0.7,
       functions >= 0.7,
   ].iter().filter(|&&x| x).count();

   if strong_signals >= 3 {
       confidence = confidence.max(0.85);
   }
   ```

4. **Result Validation**

   With new formula, `git_context_diff_tests.rs` should achieve:
   ```
   0.0 × 0.20 + 1.0 × 0.35 + 1.0 × 0.25 + 0.10 + 1.0 × 0.15 + 1.0 × 0.05
   = 0.0 + 0.35 + 0.25 + 0.10 + 0.15 + 0.05
   = 0.90  ← HIGH CONFIDENCE!
   ```

### Non-Functional Requirements

1. **Accuracy**: >95% correct classification on prodigy test files
2. **No False Positives**: Production files still score <0.5 confidence
3. **Backward Compatibility**: Integration tests in `tests/` still score 1.0
4. **Performance**: No performance regression

## Acceptance Criteria

- [ ] Component test files (*_tests.rs) achieve >0.8 confidence
- [ ] `git_context_diff_tests.rs` confidence: 0.585 → >0.85
- [ ] `git_context_uncommitted_tests.rs` confidence: >0.85
- [ ] `git_context_commit_tests.rs` confidence: >0.85
- [ ] Integration tests in `tests/` maintain ~1.0 confidence
- [ ] Production files (`executor.rs`) maintain <0.3 confidence
- [ ] Test files score reduced by 80% (not 40%)
- [ ] Test files outside top 10: git_context_*_tests.rs at rank >50
- [ ] All unit tests pass
- [ ] Integration test on prodigy validates ranking
- [ ] No compilation errors or warnings

## Technical Details

### Implementation Approach

**Phase 1: Update Weighting Constants**

```rust
// src/analysis/file_context.rs

impl FileContextDetector {
    fn weighted_average(&self,
        naming: f32,
        attributes: f32,
        functions: f32,
        imports: f32,
        directory: f32
    ) -> f32 {
        // OLD weights (directory-heavy)
        // directory * 0.40 +
        // attributes * 0.30 +
        // naming * 0.15 +
        // functions * 0.10 +
        // imports * 0.05

        // NEW weights (attribute/naming-heavy)
        directory * 0.20 +      // ← Reduced directory dominance
        attributes * 0.35 +     // ← Prioritize test attributes
        naming * 0.25 +         // ← Boost naming signal
        functions * 0.15 +      // ← Increase function naming
        imports * 0.05          // ← Unchanged
    }
}
```

**Phase 2: Add Naming Boost**

```rust
fn score_naming(&self, path: &Path) -> f32 {
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    match self.language {
        Language::Rust => {
            if filename.ends_with("_tests.rs") {
                1.0  // Perfect score for component tests
            } else if filename.ends_with("_test.rs") {
                1.0  // Perfect score for unit tests
            } else if filename == "tests.rs" {
                0.9  // High score for consolidated tests
            } else if filename.starts_with("test_") {
                0.8
            } else {
                0.0
            }
        }
        // ... other languages
    }
}
```

**Phase 3: Add Strong Signals Boost**

```rust
pub fn detect(&self, analysis: &FileAnalysis) -> FileContext {
    let test_score = self.calculate_test_score(analysis);
    let mut confidence = test_score.overall_confidence;

    // Boost confidence if multiple strong signals
    let strong_signals = [
        test_score.naming_match >= 0.7,
        test_score.attribute_density >= 0.7,
        test_score.test_function_ratio >= 0.7,
    ].iter().filter(|&&x| x).count();

    if strong_signals >= 3 {
        confidence = confidence.max(0.85);
    }

    if confidence > 0.8 {
        FileContext::Test {
            confidence,
            test_framework: self.detect_framework(analysis),
            test_count: self.count_tests(analysis),
        }
    } else if confidence > 0.5 {
        FileContext::Test {
            confidence,
            test_framework: self.detect_framework(analysis),
            test_count: self.count_tests(analysis),
        }
    } else {
        FileContext::Production
    }
}
```

**Phase 4: Expected Confidence Calculations**

For `git_context_diff_tests.rs`:

```
Component Analysis:
- Directory: src/cook/workflow/ → 0.0 (not tests/)
- Attributes: 7 #[test] attributes, 7 functions → 1.0 (100% density)
- Naming: _tests.rs suffix → 1.0 (perfect match)
- Functions: All start with test_ → 1.0 (100% ratio)
- Imports: Uses test utilities → 1.0

Formula:
0.0 × 0.20 + 1.0 × 0.35 + 1.0 × 0.25 + 1.0 × 0.15 + 1.0 × 0.05
= 0.0 + 0.35 + 0.25 + 0.15 + 0.05
= 0.80  ← Exactly at threshold!

Strong Signals Boost:
- naming: 1.0 ≥ 0.7 ✓
- attributes: 1.0 ≥ 0.7 ✓
- functions: 1.0 ≥ 0.7 ✓
→ 3 strong signals → boost to 0.85 ✓

Final Confidence: 0.85 → HIGH CONFIDENCE → 80% reduction
```

For `executor.rs` (production file):

```
- Directory: src/cook/workflow/ → 0.0
- Attributes: 0 #[test], 91 functions → 0.0
- Naming: No test suffix → 0.0
- Functions: No test_ prefix → 0.0
- Imports: No test frameworks → 0.0

Formula:
0.0 × 0.20 + 0.0 × 0.35 + 0.0 × 0.25 + 0.0 × 0.15 + 0.0 × 0.05
= 0.0 → PRODUCTION ✓
```

For `tests/integration_test.rs` (integration test):

```
- Directory: tests/ → 1.0
- Attributes: 10 #[test], 10 functions → 1.0
- Naming: _test.rs suffix → 1.0
- Functions: All test_ → 1.0
- Imports: Yes → 1.0

Formula:
1.0 × 0.20 + 1.0 × 0.35 + 1.0 × 0.25 + 1.0 × 0.15 + 1.0 × 0.05
= 0.20 + 0.35 + 0.25 + 0.15 + 0.05
= 1.00  ← PERFECT ✓
```

### Architecture Changes

1. **Modified constants** in `weighted_average()` function
2. **Enhanced** `score_naming()` for perfect scores on test suffixes
3. **New logic** for strong signals boost in `detect()`

### Expected Score Changes

| File | Old Confidence | New Confidence | Old Score | New Score | Rank |
|------|----------------|----------------|-----------|-----------|------|
| git_context_diff_tests.rs | 0.585 | 0.85 | 51.9 | 17.3 | ~#50 |
| git_context_uncommitted_tests.rs | ~0.58 | ~0.85 | 39.4 | 13.1 | ~#70 |
| git_context_commit_tests.rs | ~0.58 | ~0.85 | 30.1 | 10.0 | ~#90 |

## Dependencies

- **Spec 166**: Provides `FileContextDetector` and confidence calculation
- **Spec 168**: Applies adjusted scores to file items

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_test_achieves_high_confidence() {
        let path = PathBuf::from("src/cook/workflow/git_context_diff_tests.rs");
        let analysis = create_test_file_analysis(&path, 7, 7); // 7 tests, 7 functions

        let detector = FileContextDetector::new(Language::Rust);
        let context = detector.detect(&analysis);

        match context {
            FileContext::Test { confidence, .. } => {
                assert!(confidence > 0.8,
                    "Component test should have >0.8 confidence, got {}",
                    confidence);
            }
            _ => panic!("Should be detected as test file"),
        }
    }

    #[test]
    fn test_rebalanced_weights() {
        let detector = FileContextDetector::new(Language::Rust);

        // Component test: strong attributes/naming, no directory
        let confidence = detector.weighted_average(
            1.0,  // naming (_tests.rs)
            1.0,  // attributes (100% #[test])
            1.0,  // functions (100% test_)
            1.0,  // imports
            0.0   // directory (not in tests/)
        );

        assert!(confidence >= 0.80,
            "Should achieve at least 0.80 confidence, got {}",
            confidence);
    }

    #[test]
    fn test_strong_signals_boost() {
        // Simulate file with 3 strong signals
        let test_score = TestFileConfidence {
            naming_match: 1.0,
            attribute_density: 1.0,
            test_function_ratio: 1.0,
            test_imports: 0.0,
            directory_context: 0.0,
            overall_confidence: 0.80,
        };

        let boosted = apply_strong_signals_boost(test_score);
        assert!(boosted >= 0.85);
    }

    #[test]
    fn test_production_file_unchanged() {
        let path = PathBuf::from("src/cook/workflow/executor.rs");
        let analysis = create_production_file_analysis(&path);

        let detector = FileContextDetector::new(Language::Rust);
        let context = detector.detect(&analysis);

        match context {
            FileContext::Production => {}, // Expected
            _ => panic!("Production file misclassified as test"),
        }
    }

    #[test]
    fn test_integration_test_still_perfect() {
        let path = PathBuf::from("tests/integration_test.rs");
        let analysis = create_test_file_analysis(&path, 10, 10);

        let detector = FileContextDetector::new(Language::Rust);
        let context = detector.detect(&analysis);

        match context {
            FileContext::Test { confidence, .. } => {
                assert!(confidence >= 0.95,
                    "Integration test should have ~1.0 confidence, got {}",
                    confidence);
            }
            _ => panic!("Should be detected as test file"),
        }
    }
}
```

### Integration Tests

```bash
#!/bin/bash
# tests/test_component_test_detection.sh

# Run debtmap on prodigy
OUTPUT=$(cargo run -- analyze ../prodigy --top 100 2>&1)

# Check test files NOT in top 10
if echo "$OUTPUT" | head -n 50 | grep -q "git_context.*tests.rs"; then
    echo "FAIL: Test files still in top 10"
    exit 1
fi

# Check test files scored correctly (~17, ~13, ~10)
if ! echo "$OUTPUT" | grep "git_context_diff_tests.rs" | grep -q "SCORE: 1[0-9]\.[0-9]"; then
    echo "FAIL: git_context_diff_tests.rs not scored in 10-19 range"
    exit 1
fi

# Check test files have high confidence
if ! echo "$OUTPUT" | grep -A 5 "git_context_diff_tests.rs" | grep -q "confidence.*0\.[89]"; then
    echo "FAIL: Test file confidence not >0.8"
    exit 1
fi

echo "PASS: Component test detection working correctly"
```

### Validation on Real Codebases

Test on 3 major Rust projects:
- **Prodigy**: Component tests in src/
- **Tokio**: Mix of unit and integration tests
- **Serde**: Extensive test suite

Validate:
- Component tests: confidence >0.8
- Integration tests: confidence ~1.0
- Production files: confidence <0.3
- False positive rate <1%

## Documentation Requirements

### Code Documentation

```rust
/// Calculate weighted average confidence from individual detection signals.
///
/// # Weighting Strategy (Spec 169)
///
/// The weights are carefully balanced to handle component tests (in src/)
/// without requiring tests/ directory location:
///
/// - **Attributes (35%)**: Strongest signal - #[test] is unambiguous
/// - **Naming (25%)**: Strong signal - *_test.rs is conventional
/// - **Directory (20%)**: Reduced from 40% to avoid false negatives on component tests
/// - **Functions (15%)**: test_* prefix is conventional
/// - **Imports (5%)**: Weakest signal - many utils import test frameworks
///
/// # Example Scores
///
/// - Component test (src/foo_tests.rs): 0.80-0.90 (high confidence)
/// - Integration test (tests/foo.rs): 0.95-1.0 (perfect confidence)
/// - Production file: 0.0-0.3 (not a test)
fn weighted_average(...) -> f32 { ... }
```

### User Documentation

Update README.md:

```markdown
## Test File Detection Accuracy

Debtmap detects test files using a multi-signal approach:

**Detection Signals** (in order of weight):
1. **Test Attributes (35%)**: `#[test]`, `#[tokio::test]`, etc.
2. **File Naming (25%)**: `*_test.rs`, `*_tests.rs`, `test_*.py`
3. **Directory Location (20%)**: `tests/`, `*_tests/`
4. **Function Naming (15%)**: `test_*` function names
5. **Framework Imports (5%)**: `use proptest`, `import pytest`

**Confidence Levels**:
- **High (>0.8)**: 80% score reduction (component and integration tests)
- **Probable (0.5-0.8)**: 40% score reduction
- **Low (<0.5)**: No reduction (production code)

**Special Handling**:
- Component tests in `src/` achieve high confidence via naming + attributes
- Integration tests in `tests/` achieve perfect ~1.0 confidence
- Strong naming (`*_tests.rs`) boosts confidence even without directory signal
```

## Implementation Notes

### Weight Tuning Rationale

**Why reduce directory weight from 40% to 20%?**
- Rust convention: Component tests live in src/ next to code
- Directory is binary (0 or 1), not gradual
- Other signals provide more nuance

**Why increase attributes to 35%?**
- `#[test]` is unambiguous indicator
- High precision, low false positive rate
- Directly reflects developer intent

**Why increase naming to 25%?**
- `_tests.rs` suffix is strong convention
- Rarely used for non-test files
- Complements attribute detection

### False Positive Mitigation

Edge cases where production files might be misclassified:

1. **Benchmark files** (`*_bench.rs`): Won't have `#[test]` → safe
2. **Example files** (`examples/*.rs`): Different naming, no attributes → safe
3. **Mock utilities** (`test_utils.rs`): Might have helper functions, but no `#[test]` → safe
4. **Integration test helpers**: Put in `tests/common/` → acceptable if flagged

### Backward Compatibility

Existing tests in `tests/` directory:
- Old confidence: ~0.95-1.0
- New confidence: ~0.95-1.0 (unchanged)
- Score: Still 90% reduction (unchanged)

Component tests (newly affected):
- Old confidence: 0.58 (probable)
- New confidence: 0.85 (high)
- Score reduction: 40% → 80% (improved)

## Migration and Compatibility

### Breaking Changes

None - this is purely a confidence calculation improvement.

### Configuration Override

Future enhancement (not in this spec):
```toml
[test_detection]
weights = { directory = 0.20, attributes = 0.35, naming = 0.25, functions = 0.15, imports = 0.05 }
strong_signals_threshold = 3
strong_signals_boost = 0.05
```

## Success Metrics

**Validation Criteria**:

Before:
- Component test confidence: ~0.58 (probable)
- Component test rank: #3 (false positive)
- Score: 51.9 (too high)

After:
- Component test confidence: >0.85 (high)
- Component test rank: >50 (correct)
- Score: ~17 (correct)

**Target Metrics**:
- ✅ Component tests: >0.8 confidence
- ✅ Integration tests: >0.95 confidence
- ✅ Production files: <0.3 confidence
- ✅ False positive rate: <1%
- ✅ Test files outside top 10

## Related Issues

- Completes spec 166 detection accuracy
- Enables spec 168 to apply correct reductions
- Fixes false positive on component tests
- Improves user trust in debtmap recommendations

## Future Enhancements (Not in Scope)

- Language-specific weight tuning (Python, JS, TS)
- Machine learning confidence calibration
- User feedback loop for confidence tuning
- Adaptive weights based on codebase patterns
