---
number: 166
title: Test File Detection and Context-Aware Scoring
category: optimization
priority: high
status: draft
dependencies: [133, 116]
created: 2025-11-03
---

# Specification 166: Test File Detection and Context-Aware Scoring

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [133 - God Object Detection Refinement, 116 - Confidence Scoring System]

## Context

**Problem**: Debtmap currently treats test files identically to production code, leading to incorrect debt recommendations.

**Real-World Impact**: Analysis of the prodigy codebase revealed:
- Test file `git_context_diff_tests.rs` (354 lines, 7 tests) scored 86.5 [CRITICAL] - ranked #1
- Production file `executor.rs` (2257 lines, 91 functions) scored 69.4 [CRITICAL] - ranked #2
- Recommendations to "split test files into utilities modules" are nonsensical

**Root Cause**: Debtmap lacks semantic understanding of file context:
1. No detection of test vs production code
2. Test patterns (repetitive arrange-act-assert) flagged as duplication
3. Normal test organization (7-8 tests per file) flagged as "too many functions"
4. Test file characteristics penalized instead of production complexity

**User Impact**: 3 of top 5 recommendations were false positives on test files, severely reducing report actionability.

## Objective

Implement intelligent test file detection with context-aware scoring that:
1. **Identifies test files** using multiple heuristics (naming, structure, attributes)
2. **Adjusts scoring** to reflect test-specific acceptable patterns
3. **Provides test-specific recommendations** when tests genuinely need refactoring
4. **Maintains accuracy** on production code analysis

## Requirements

### Functional Requirements

1. **Test File Detection Heuristics**
   - **File naming patterns**: `*_test.rs`, `*_tests.rs`, `test_*.rs`, `tests.rs`
   - **Module structure**: Files containing only `#[cfg(test)] mod tests { ... }`
   - **Attribute presence**: High density of `#[test]`, `#[tokio::test]`, `#[cfg(test)]` attributes
   - **Import patterns**: Heavy use of test frameworks (proptest, quickcheck, etc.)
   - **Directory location**: Files in `tests/` directories or `*_tests/` subdirectories
   - **Multi-language support**:
     - Rust: `#[test]`, `#[cfg(test)]`, mod tests
     - Python: `test_*.py`, `*_test.py`, `class Test*`, `def test_*`
     - JavaScript/TypeScript: `*.test.js`, `*.spec.ts`, `describe()`, `it()`

2. **Test Confidence Scoring**
   ```rust
   struct TestFileConfidence {
       naming_match: f32,          // 0.0-1.0
       attribute_density: f32,      // Ratio of test attributes to total lines
       test_function_ratio: f32,    // Ratio of test functions to all functions
       test_imports: f32,           // Presence of test framework imports
       directory_context: f32,      // Located in test directory
       overall_confidence: f32,     // Weighted average
   }
   ```

3. **Context-Aware Scoring Adjustments**
   - **Test files (confidence > 0.8)**:
     - Reduce "too many functions" penalty by 80%
     - Ignore "similar function structures" (repetition is normal in tests)
     - Reduce line count penalty by 60%
     - Increase complexity threshold by 50% (test setup is naturally complex)

   - **Probable test files (confidence 0.5-0.8)**:
     - Reduce penalties by 40%
     - Add confidence disclaimer to recommendations

   - **Production files (confidence < 0.5)**:
     - No scoring adjustments
     - Standard debt detection

4. **Test-Specific Recommendations**

   **When to flag test files**:
   - Individual test functions >100 lines (test is doing too much)
   - Test file >1000 lines AND >30 tests (consider test organization)
   - Excessive setup duplication (suggest test fixtures/utilities)
   - Overly complex assertions (suggest helper matchers)

   **Message format**:
   ```
   #1 SCORE: 35.2 [TEST FILE] [MEDIUM]
   └─ ./tests/integration_tests.rs (1200 lines, 45 tests)
   └─ FILE TYPE: Integration test suite (confidence: 95%)
   └─ WHY THIS MATTERS: Test file has grown large with 45 similar tests.
      While repetition in tests is acceptable, this size makes the suite
      slow to run and difficult to navigate.
   └─ ACTION: Consider organizing tests by feature:
      - Split into feature-focused test files (e.g., auth_tests.rs, api_tests.rs)
      - Extract common setup into test_utils.rs or fixtures
      - Keep related tests together (5-15 tests per file is ideal)
   └─ METRICS: Tests: 45, Avg test length: 27 lines, Setup duplication: 60%
   └─ SCORING: File type: TEST | Size: HIGH | Organization: MODERATE
   ```

5. **Metadata Enrichment**
   ```rust
   pub struct FileAnalysis {
       // Existing fields...
       pub file_context: FileContext,
   }

   pub enum FileContext {
       Production,
       Test {
           confidence: f32,
           test_framework: Option<String>,
           test_count: usize,
       },
       Generated {
           generator: String,
       },
       Configuration,
       Documentation,
   }
   ```

### Non-Functional Requirements

1. **Performance**: Test detection adds <2% overhead to analysis time
2. **Accuracy**: >95% correct classification on major Rust projects
3. **Language Support**: Rust (phase 1), Python/JS/TS (phase 2)
4. **Backward Compatibility**: Existing JSON output format extended, not changed
5. **Configurability**: Allow users to override test detection via config

## Acceptance Criteria

- [ ] Detect Rust test files with >95% accuracy on prodigy codebase
- [ ] Files named `*_tests.rs` with `#[test]` attributes classified as tests
- [ ] Test files score 50-80% lower than equivalent production files
- [ ] Test file recommendations mention "test organization" not "god object"
- [ ] Files in `tests/` directory automatically classified as tests
- [ ] Production files mis-classified as tests <1% of the time
- [ ] JSON output includes `file_context` field with test confidence
- [ ] Text output shows `[TEST FILE]` tag for detected test files
- [ ] Prodigy's `git_context_diff_tests.rs` classified as test (confidence >90%)
- [ ] Prodigy's `executor.rs` classified as production (confidence >95%)
- [ ] Running debtmap on prodigy shows 0 test files in top 10 recommendations
- [ ] New unit tests cover:
  - Rust inline test modules (`#[cfg(test)] mod tests`)
  - Separate test files (`*_tests.rs`)
  - Integration tests in `tests/` directory
  - False positive cases (production code with "test" in name)
- [ ] Documentation updated with test detection behavior

## Technical Details

### Implementation Approach

**Phase 1: Detection Framework**

```rust
// src/analysis/file_context.rs

pub struct FileContextDetector {
    language: Language,
}

impl FileContextDetector {
    pub fn detect(&self, analysis: &FileAnalysis) -> FileContext {
        let test_score = self.calculate_test_score(analysis);

        if test_score.overall_confidence > 0.8 {
            FileContext::Test {
                confidence: test_score.overall_confidence,
                test_framework: self.detect_framework(analysis),
                test_count: self.count_tests(analysis),
            }
        } else if self.is_generated(analysis) {
            FileContext::Generated {
                generator: self.detect_generator(analysis),
            }
        } else {
            FileContext::Production
        }
    }

    fn calculate_test_score(&self, analysis: &FileAnalysis) -> TestFileConfidence {
        let naming = self.score_naming(&analysis.path);
        let attributes = self.score_attributes(analysis);
        let functions = self.score_test_functions(analysis);
        let imports = self.score_test_imports(analysis);
        let directory = self.score_directory(&analysis.path);

        TestFileConfidence {
            naming_match: naming,
            attribute_density: attributes,
            test_function_ratio: functions,
            test_imports: imports,
            directory_context: directory,
            overall_confidence: self.weighted_average(
                naming, attributes, functions, imports, directory
            ),
        }
    }

    fn score_naming(&self, path: &Path) -> f32 {
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        match self.language {
            Language::Rust => {
                if filename.ends_with("_tests.rs") { 0.9 }
                else if filename.ends_with("_test.rs") { 0.9 }
                else if filename == "tests.rs" { 0.8 }
                else if filename.starts_with("test_") { 0.7 }
                else { 0.0 }
            }
            Language::Python => {
                if filename.starts_with("test_") { 0.9 }
                else if filename.ends_with("_test.py") { 0.9 }
                else { 0.0 }
            }
            _ => 0.0,
        }
    }

    fn score_attributes(&self, analysis: &FileAnalysis) -> f32 {
        match self.language {
            Language::Rust => {
                let test_attrs = self.count_test_attributes(analysis);
                let total_functions = analysis.functions.len();

                if total_functions == 0 { return 0.0; }

                // Ratio of test attributes to functions
                let ratio = test_attrs as f32 / total_functions as f32;
                ratio.min(1.0)
            }
            _ => 0.0,
        }
    }

    fn count_test_attributes(&self, analysis: &FileAnalysis) -> usize {
        analysis.functions
            .iter()
            .filter(|f| {
                f.attributes.iter().any(|attr| {
                    attr.contains("test") ||
                    attr.contains("tokio::test") ||
                    attr.contains("proptest")
                })
            })
            .count()
    }

    fn score_test_functions(&self, analysis: &FileAnalysis) -> f32 {
        let test_functions = analysis.functions
            .iter()
            .filter(|f| f.name.starts_with("test_"))
            .count();

        let total_functions = analysis.functions.len();
        if total_functions == 0 { return 0.0; }

        test_functions as f32 / total_functions as f32
    }

    fn score_directory(&self, path: &Path) -> f32 {
        let path_str = path.to_string_lossy();

        if path_str.contains("/tests/") { 1.0 }
        else if path_str.contains("_tests/") { 0.9 }
        else if path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s == "tests")
            .unwrap_or(false)
        { 1.0 }
        else { 0.0 }
    }

    fn weighted_average(&self,
        naming: f32,
        attributes: f32,
        functions: f32,
        imports: f32,
        directory: f32
    ) -> f32 {
        // Weighted scoring:
        // - Directory location is strongest signal (40%)
        // - Attributes are second strongest (30%)
        // - Naming is third (15%)
        // - Function naming (10%)
        // - Imports (5%)

        directory * 0.40 +
        attributes * 0.30 +
        naming * 0.15 +
        functions * 0.10 +
        imports * 0.05
    }
}
```

**Phase 2: Scoring Adjustments**

```rust
// src/priority/scoring.rs

pub fn calculate_file_score(
    analysis: &FileAnalysis,
    context: &FileContext,
) -> f64 {
    let base_score = calculate_base_score(analysis);

    match context {
        FileContext::Test { confidence, .. } => {
            apply_test_adjustments(base_score, *confidence)
        }
        FileContext::Generated { .. } => {
            // Generated files get low priority
            base_score * 0.1
        }
        FileContext::Production => base_score,
        _ => base_score,
    }
}

fn apply_test_adjustments(base_score: f64, confidence: f32) -> f64 {
    if confidence > 0.8 {
        // High confidence test file
        base_score * 0.2  // Reduce score by 80%
    } else if confidence > 0.5 {
        // Probable test file
        base_score * 0.6  // Reduce score by 40%
    } else {
        base_score
    }
}
```

**Phase 3: Recommendation Generation**

```rust
// src/priority/recommendations.rs

pub fn generate_recommendation(
    analysis: &FileAnalysis,
    context: &FileContext,
) -> Recommendation {
    match context {
        FileContext::Test { test_count, confidence, .. } => {
            generate_test_recommendation(analysis, *test_count, *confidence)
        }
        FileContext::Production => {
            generate_production_recommendation(analysis)
        }
        _ => generate_generic_recommendation(analysis),
    }
}

fn generate_test_recommendation(
    analysis: &FileAnalysis,
    test_count: usize,
    confidence: f32,
) -> Recommendation {
    let avg_test_length = analysis.total_lines / test_count.max(1);

    let message = if test_count > 30 && analysis.total_lines > 1000 {
        format!(
            "Large test suite with {} tests across {} lines. \
             Consider splitting by feature area for better organization \
             and faster test execution.",
            test_count, analysis.total_lines
        )
    } else if avg_test_length > 100 {
        format!(
            "Some test functions are very long (avg {} lines). \
             Consider extracting setup into fixtures or helper functions.",
            avg_test_length
        )
    } else {
        format!(
            "Test file organization is acceptable for {} tests. \
             No refactoring needed unless test suite grows significantly.",
            test_count
        )
    };

    Recommendation {
        file_type: FileType::Test,
        confidence,
        message,
        priority: if test_count > 50 { Priority::Medium } else { Priority::Low },
        // ... other fields
    }
}
```

### Architecture Changes

1. **New Module**: `src/analysis/file_context.rs` - File context detection
2. **Modified Module**: `src/priority/scoring.rs` - Context-aware scoring
3. **Modified Module**: `src/priority/recommendations.rs` - Test-specific messages
4. **Extended Struct**: `FileAnalysis` - Add `file_context` field
5. **New Enum**: `FileContext` - Classify file types

### Data Structures

```rust
pub enum FileContext {
    Production,
    Test {
        confidence: f32,
        test_framework: Option<String>,
        test_count: usize,
    },
    Generated {
        generator: String,
    },
    Configuration,
    Documentation,
}

pub struct TestFileConfidence {
    pub naming_match: f32,
    pub attribute_density: f32,
    pub test_function_ratio: f32,
    pub test_imports: f32,
    pub directory_context: f32,
    pub overall_confidence: f32,
}
```

### JSON Output Format

```json
{
  "file": "src/workflow/git_context_diff_tests.rs",
  "lines": 354,
  "functions": 7,
  "score": 17.3,
  "priority": "LOW",
  "file_context": {
    "type": "Test",
    "confidence": 0.95,
    "test_framework": "rust-std",
    "test_count": 7
  },
  "recommendation": {
    "message": "Test file organization is acceptable for 7 tests. No refactoring needed.",
    "action": null
  }
}
```

## Dependencies

- **Spec 133**: God Object Detection Refinement - Similar dominance-based classification logic
- **Spec 116**: Confidence Scoring System - Reuses confidence scoring framework

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rust_test_file_by_naming() {
        let path = Path::new("src/foo_tests.rs");
        let score = FileContextDetector::new(Language::Rust)
            .score_naming(path);
        assert!(score >= 0.9);
    }

    #[test]
    fn detects_test_file_by_attributes() {
        let analysis = FileAnalysis {
            functions: vec![
                Function { attributes: vec!["#[test]".to_string()], .. },
                Function { attributes: vec!["#[test]".to_string()], .. },
                Function { attributes: vec![], .. },
            ],
            // ... other fields
        };

        let score = FileContextDetector::new(Language::Rust)
            .score_attributes(&analysis);
        assert!(score >= 0.6); // 2/3 functions have test attributes
    }

    #[test]
    fn production_file_not_classified_as_test() {
        let analysis = FileAnalysis {
            path: PathBuf::from("src/executor.rs"),
            functions: vec![/* no test attributes */],
            // ... other fields
        };

        let context = FileContextDetector::new(Language::Rust)
            .detect(&analysis);

        assert!(matches!(context, FileContext::Production));
    }

    #[test]
    fn test_file_scoring_reduced() {
        let production_score = 86.5;
        let test_context = FileContext::Test {
            confidence: 0.95,
            test_framework: Some("rust-std".to_string()),
            test_count: 7,
        };

        let adjusted = apply_test_adjustments(production_score, 0.95);
        assert!(adjusted < 20.0); // Should be significantly lower
    }
}
```

### Integration Tests

Run debtmap on known codebases:
- **Prodigy**: Verify `git_context_diff_tests.rs` classified as test
- **Debtmap itself**: Verify test files score lower than production
- **Tokio**: Large Rust project with extensive test suite
- **Serde**: Mix of unit and integration tests

### Validation Criteria

Test on 3 major Rust projects (prodigy, tokio, serde):
- True positive rate >95% (test files correctly identified)
- False positive rate <1% (production files mis-classified)
- Test file scores reduced by 60-80% on average
- Zero test files in top 10 recommendations
- Manual review: Recommendations make semantic sense

## Documentation Requirements

### Code Documentation

- Document test detection heuristics and thresholds in code comments
- Explain weighted scoring rationale
- Document decision tree for classification

### User Documentation

Update README.md:
```markdown
## Test File Detection

Debtmap automatically detects test files and adjusts scoring to avoid false positives:

- **Rust**: Files ending in `_tests.rs` or containing `#[test]` attributes
- **Python**: Files matching `test_*.py` or `*_test.py`
- **JavaScript/TypeScript**: Files matching `*.test.js` or `*.spec.ts`

Test files receive different recommendations focused on test organization
rather than general complexity reduction.

To override test detection, use `--treat-tests-as-production` flag.
```

### Architecture Updates

Add to ARCHITECTURE.md:
```markdown
## File Context Detection

The analysis pipeline includes file context detection:

1. **Detection Phase**: Classify files (production, test, generated, config)
2. **Scoring Phase**: Apply context-aware score adjustments
3. **Recommendation Phase**: Generate context-appropriate recommendations

Test files are scored 60-80% lower than equivalent production code to
reflect acceptable patterns (repetition, length, setup complexity).
```

## Implementation Notes

### Edge Cases

1. **Hybrid files**: Production code and tests in same file
   - Use ratio-based classification (>70% test functions → test file)

2. **Test utilities**: Helper functions in `test_utils.rs`
   - Classify based on location and naming, not usage

3. **Benchmark files**: Similar to tests but for performance
   - Detect via `#[bench]` attribute, score similar to tests

4. **Example code**: Might have `test_` prefix but not actual tests
   - Check for test framework imports and attributes

5. **False negatives acceptable**: Missing some test files is okay
   - Better to under-detect than over-detect and penalize production code

### Performance Optimization

- Cache file context detection results
- Lazy evaluation of expensive checks (import analysis)
- Short-circuit on high-confidence signals (directory location)

### Configuration Options

```toml
# debtmap.toml

[analysis]
detect_test_files = true
test_score_reduction = 0.8  # Reduce test file scores by 80%
test_confidence_threshold = 0.8  # Minimum confidence to treat as test

[test_detection.rust]
file_patterns = ["*_tests.rs", "*_test.rs", "tests.rs"]
attributes = ["test", "tokio::test", "proptest"]
```

## Migration and Compatibility

### Backward Compatibility

- Existing JSON output format extended with optional `file_context` field
- Existing scores may change (by design - fixing false positives)
- Old analysis JSON files remain parseable

### Migration Path

1. **Phase 1**: Deploy with test detection enabled by default
2. **Phase 2**: Monitor for false positives on diverse codebases
3. **Phase 3**: Tune thresholds based on real-world data
4. **Phase 4**: Extend to Python/JS/TS (separate spec)

### Breaking Changes

None - this is purely additive functionality.

## Success Metrics

Track after implementation:
- **False positive reduction**: Test files in top 10 (before) vs (after)
- **User satisfaction**: GitHub issues about test file recommendations
- **Accuracy**: Manual review of 100 random files across 10 projects
- **Performance**: Analysis time increase (should be <2%)

Target outcomes:
- 90% reduction in test file false positives
- Zero complaints about test file recommendations in issue tracker
- >95% classification accuracy on manual review
- <2% performance overhead

## Future Enhancements (Not in Scope)

- Python/JavaScript/TypeScript test detection (separate spec)
- Generated file detection (protobuf, swagger, etc.)
- Documentation file detection (markdown, rustdoc)
- Configuration file detection (TOML, JSON, YAML)
- Smart test suite organization suggestions (cluster by feature)
- Test smell detection (overly complex assertions, poor isolation)
