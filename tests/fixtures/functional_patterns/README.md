# Functional Pattern Detection Test Corpus

This directory contains the test corpus for validating spec 111: AST-based functional pattern detection.

## Structure

- `positive/` - 65 files containing clear functional programming patterns (should be detected)
- `negative/` - 45 files containing imperative patterns (should NOT be detected)
- `edge_cases/` - 10 edge case files for boundary testing

## Test Coverage

### Positive Examples (65 files)

The positive examples cover various functional programming patterns in Rust:

**Iterator Chains (p01-p34)**:
- `map`, `filter`, `fold` operations
- `flat_map`, `filter_map`, `partition`
- `zip`, `scan`, `enumerate`, `chain`
- `skip`, `take`, `step_by`
- `any`, `all`, `find`, `position`
- `max`, `min`, `sum`, `product`
- `cloned`, `copied`, `inspect`, `rev`
- Complex multi-stage pipelines

**Collection Operations (p35-p44)**:
- Nested iterators
- Option/Result monadic chains
- HashMap/BTreeMap/HashSet operations
- String and byte processing
- Range iterations

**Advanced Patterns (p45-p65)**:
- `once`, `repeat_with`, `from_fn`, `successors`
- Partial consumption with `by_ref`
- `fuse`, `dedup`, `reduce`
- Parallel processing with rayon
- `take_while`, `skip_while`, `map_while`
- Cartesian products
- Collection building

### Negative Examples (45 files)

The negative examples represent imperative code patterns:

**Loop Constructs (n01-n15)**:
- For loops, while loops, index-based iteration
- Mutable vector manipulation
- Nested loops
- Counter patterns
- Accumulator patterns
- Early returns, break, continue
- Swap operations
- State machines

**Object-Oriented Patterns (n16-n23)**:
- Getter/setter methods
- HashMap/HashSet manual insertion
- String building with mutation
- Match-based categorization
- Option/Result unwrapping in loops

**Simple Functions (n24-n38)**:
- Basic arithmetic
- Conditional logic
- Struct/tuple access
- Array indexing
- String formatting
- Print statements

**I/O and Concurrency (n39-n45)**:
- File operations
- Thread spawning
- Mutex locking
- Arc/Rc operations
- Box dereferencing

### Edge Cases (10 files)

Boundary conditions and ambiguous patterns:

- Empty iterators
- Single-element collections
- Single-method calls (not true pipelines)
- Two-method minimal chains
- Nested closures
- Complex closure logic
- Mixed imperative/functional styles
- Macros in chains
- Type conversions
- Extremely long pipelines

## Validation Metrics

The test corpus is used to validate:

1. **Precision ≥ 90%**: Low false positives (imperative code not flagged as functional)
2. **Recall ≥ 85%**: Low false negatives (functional patterns properly detected)
3. **F1 Score ≥ 0.87**: Balanced accuracy measure

## Current Status

**Test Infrastructure**: ✓ Complete
- Test corpus: 120 files (65 positive, 45 negative, 10 edge cases)
- Test framework: `test_full_corpus_accuracy()` implemented
- Metrics calculation: AccuracyMetrics struct with precision/recall/F1

**Current Performance** (as of last run):
- Precision: 47.06% (target: ≥90%)
- Recall: 61.54% (target: ≥85%)
- F1 Score: 0.5333 (target: ≥0.87)

**Issues Identified**:
1. High false positive rate (45 false positives) - imperative code being classified as functional
2. Moderate false negative rate (25 false negatives) - functional patterns not being detected
3. Root cause: Current implementation assigns baseline composition_quality scores to all functions, even those without functional patterns
4. Need to adjust thresholds or improve detection logic to distinguish functional from imperative code

**Next Steps**:
1. Analyze false positives to understand why imperative code gets high composition_quality scores
2. Tune detection thresholds or improve pattern matching
3. Consider only assigning composition_metrics when actual functional patterns are detected
4. Re-run validation after improvements

## Usage

Run the full corpus validation:

```bash
cargo test test_full_corpus_accuracy --test functional_composition_validation_test -- --nocapture
```

Run the basic accuracy test (small sample):

```bash
cargo test test_accuracy_metrics --test functional_composition_validation_test -- --nocapture
```

## Test File Naming Convention

- `p##_description.rs` - Positive examples (functional patterns)
- `n##_description.rs` - Negative examples (imperative patterns)
- `e##_description.rs` - Edge cases
