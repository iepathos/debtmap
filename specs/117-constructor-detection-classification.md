---
number: 117
title: Constructor Detection and Classification
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-21
---

# Specification 117: Constructor Detection and Classification

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap v0.2.9 misclassifies simple constructor functions (like `ContextMatcher::any()`) as `PureLogic` (business logic), resulting in false positives with inflated severity scores.

**Real-World False Positive**:
```rust
// src/context/rules.rs:52
pub fn any() -> Self {
    Self {
        role: None,
        file_type: None,
        is_async: None,
        framework_pattern: None,
        name_pattern: None,
    }
}
```

**Current Analysis Output**:
```
#1 SCORE: 21.2 [🔴 UNTESTED] [CRITICAL]
├─ LOCATION: ./src/context/rules.rs:52 ContextMatcher::any()
└─ WHY: Business logic with 100% coverage gap
├─ COMPLEXITY: cyclomatic=1, branches=1, cognitive=0
├─ DEPENDENCIES: 17 upstream, 0 downstream
```

**What's Wrong**:
- Classified as `FunctionRole::PureLogic` (1.2x severity multiplier)
- Criticality multiplier of 2.0x for "business logic"
- Actual role should be `IOWrapper` or similar (0.7x multiplier)
- Risk score inflated from ~12 to ~21

**Root Cause**:
`semantic_classifier.rs:21` defaults to `FunctionRole::PureLogic` when no other pattern matches. Simple constructors don't match any existing patterns, so they get the highest priority classification.

## Objective

Accurately detect and classify constructor functions to prevent false positives and reduce inflated severity scores for trivial initialization code.

## Requirements

### Functional Requirements

**FR1: Name-Based Constructor Detection**
- Detect functions with constructor naming patterns:
  - `new`, `default`, `from_*`, `with_*`, `create_*`, `make_*`, `build_*`
  - Case-insensitive matching
  - Prefix and exact match support

**FR2: Complexity-Based Filtering**
- Apply complexity thresholds to distinguish constructors from complex factory functions:
  - Cyclomatic complexity ≤ 2
  - Function length < 15 lines
  - Nesting depth ≤ 1

**FR3: Structural Pattern Recognition**
- Identify constructor-specific patterns:
  - Returns `Self` or struct type
  - Body primarily consists of field initialization
  - Minimal or no branching logic
  - No loops or iteration

**FR4: Role Classification Update**
- Classify detected constructors as `FunctionRole::IOWrapper` (0.7x multiplier)
- Alternative: Create new `FunctionRole::Constructor` (0.5x multiplier)
- Update role classification priority to check constructors before `PatternMatch`

**FR5: Language-Specific Detection**
- **Rust**: Detect `new()`, `default()`, `from_*()`, `with_*()` patterns
- **Python**: Detect `__init__()`, `from_*()`, class method constructors
- **TypeScript/JavaScript**: Detect `constructor()`, static factory methods
- **Go**: Detect `New*()` functions returning struct types

### Non-Functional Requirements

**NFR1: Performance**
- Constructor detection adds < 1% overhead to analysis time
- Name pattern matching using pre-compiled regex or string operations
- No additional AST parsing required for initial implementation

**NFR2: Accuracy**
- Reduce false positive rate for constructor functions to < 5%
- Maintain existing accuracy for other function classifications
- No false negatives for actual business logic functions

**NFR3: Maintainability**
- Constructor patterns configurable via configuration file
- Extensible to new languages without core code changes
- Clear documentation of detection heuristics

## Acceptance Criteria

- [x] Constructor detection function `is_simple_constructor()` implemented in `src/priority/semantic_classifier.rs`
- [x] Name-based pattern matching supports all common constructor patterns (new, default, from_*, with_*, etc.)
- [x] Complexity thresholds correctly filter out complex factory functions
- [x] `classify_by_rules()` checks for constructors before other classifications
- [x] `ContextMatcher::any()` no longer classified as `PureLogic`
- [x] Test suite includes constructor detection test cases for all supported languages
- [x] False positive rate for constructors reduced by at least 80%
- [x] Risk score for simple constructors drops from CRITICAL (~20) to LOW/MODERATE (~5-12)
- [x] Documentation updated with constructor classification logic
- [x] Configuration file allows customization of constructor patterns

## Technical Details

### Implementation Approach

**Phase 1: Core Detection Function** (src/priority/semantic_classifier.rs)

```rust
/// Detect simple constructor functions that should not be classified as business logic
fn is_simple_constructor(func: &FunctionMetrics) -> bool {
    // Name-based detection
    let name_lower = func.name.to_lowercase();
    let constructor_patterns = [
        "new", "default", "from_", "with_", "create_",
        "make_", "build_", "of_", "empty", "zero"
    ];

    let matches_constructor_name = constructor_patterns
        .iter()
        .any(|pattern| {
            name_lower == *pattern ||
            name_lower.starts_with(pattern) ||
            name_lower.ends_with(pattern)
        });

    // Complexity-based filtering
    let is_simple = func.cyclomatic <= 2
        && func.length < 15
        && func.nesting <= 1;

    // Structural pattern: low cognitive complexity suggests simple initialization
    let is_initialization = func.cognitive <= 3;

    matches_constructor_name && is_simple && is_initialization
}
```

**Phase 2: Integration into Classification Pipeline**

```rust
fn classify_by_rules(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
) -> Option<FunctionRole> {
    // Entry point has highest precedence
    if is_entry_point(func_id, call_graph) {
        return Some(FunctionRole::EntryPoint);
    }

    // NEW: Check for constructors BEFORE pattern matching
    if is_simple_constructor(func) {
        return Some(FunctionRole::IOWrapper); // or FunctionRole::Constructor
    }

    // Check for pattern matching functions
    if is_pattern_matching_function(func, func_id) {
        return Some(FunctionRole::PatternMatch);
    }

    // ... rest of existing logic
}
```

**Phase 3: Language-Specific Extensions**

```rust
/// Language-specific constructor detection
fn is_constructor_for_language(func: &FunctionMetrics, language: Language) -> bool {
    match language {
        Language::Rust => is_rust_constructor(func),
        Language::Python => is_python_constructor(func),
        Language::TypeScript | Language::JavaScript => is_ts_constructor(func),
        Language::Go => is_go_constructor(func),
        _ => false,
    }
}

fn is_python_constructor(func: &FunctionMetrics) -> bool {
    func.name == "__init__" ||
    (func.name.starts_with("from_") && func.cyclomatic <= 2)
}

fn is_ts_constructor(func: &FunctionMetrics) -> bool {
    func.name == "constructor" ||
    (func.name.starts_with("create") && func.length < 15)
}
```

### Architecture Changes

**Modified Files**:
- `src/priority/semantic_classifier.rs`: Add `is_simple_constructor()` function
- `src/priority/semantic_classifier.rs`: Update `classify_by_rules()` to check constructors early
- `src/analyzers/language.rs`: Add language detection helper for constructor patterns (optional)

**New Configuration** (config.toml):
```toml
[classification.constructors]
enabled = true
patterns = ["new", "default", "from_", "with_", "create_", "make_", "build_"]
max_cyclomatic = 2
max_length = 15
max_nesting = 1
role = "IOWrapper"  # or "Constructor" if new role added
```

### Data Structures

**Option A: Use Existing Role**
- Classify as `FunctionRole::IOWrapper` (0.7x multiplier)
- Minimal code changes
- Semantically acceptable (constructors are "initialization wrappers")

**Option B: Add New Role**
```rust
pub enum FunctionRole {
    PureLogic,
    Orchestrator,
    IOWrapper,
    EntryPoint,
    PatternMatch,
    Constructor,  // NEW: 0.5x multiplier for trivial init code
    Unknown,
}
```

**Recommendation**: Start with Option A, add Option B later if needed.

### APIs and Interfaces

**Public API** (for external consumers):
```rust
pub fn is_simple_constructor(func: &FunctionMetrics) -> bool;
pub fn classify_constructor_type(func: &FunctionMetrics) -> ConstructorType;

pub enum ConstructorType {
    SimpleInitializer,  // Just field initialization
    FactoryMethod,      // Complex construction logic
    Builder,            // Builder pattern
    NotConstructor,
}
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/priority/semantic_classifier.rs` - Core classification logic
- `src/priority/unified_scorer.rs` - Uses role multipliers
- `src/risk/evidence/coverage_analyzer.rs` - Uses role criticality

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**Test Cases** (src/priority/semantic_classifier.rs):

```rust
#[test]
fn test_simple_constructor_detection() {
    // Test: ContextMatcher::any() case
    let func = create_test_metrics("any", 1, 0, 9);
    assert!(is_simple_constructor(&func));

    // Test: Standard new() constructor
    let func = create_test_metrics("new", 1, 0, 5);
    assert!(is_simple_constructor(&func));

    // Test: from_* constructor
    let func = create_test_metrics("from_config", 1, 0, 8);
    assert!(is_simple_constructor(&func));

    // Test: Complex factory should NOT match
    let func = create_test_metrics("create_complex", 8, 12, 50);
    assert!(!is_simple_constructor(&func));
}

#[test]
fn test_constructor_classification_precedence() {
    let graph = CallGraph::new();
    let func = create_test_metrics("any", 1, 0, 9);
    let func_id = FunctionId {
        file: PathBuf::from("context/rules.rs"),
        name: "any".to_string(),
        line: 52,
    };

    let role = classify_function_role(&func, &func_id, &graph);
    assert_eq!(role, FunctionRole::IOWrapper,
        "Simple constructor should not be classified as PureLogic");
}

#[test]
fn test_language_specific_constructors() {
    // Python __init__
    let func = create_test_metrics("__init__", 1, 0, 10);
    assert!(is_python_constructor(&func));

    // TypeScript constructor
    let func = create_test_metrics("constructor", 1, 0, 12);
    assert!(is_ts_constructor(&func));

    // Go New function
    let func = create_test_metrics("NewClient", 1, 0, 8);
    assert!(is_go_constructor(&func));
}
```

### Integration Tests

**Regression Test** (tests/false_positive_regression_test.rs):
```rust
#[test]
fn test_context_matcher_any_no_longer_critical() {
    let analysis = analyze_file("src/context/rules.rs");

    let any_func = analysis.find_function("any", 52);
    assert!(any_func.is_some());

    let role = any_func.unwrap().role;
    assert_ne!(role, FunctionRole::PureLogic,
        "Constructor should not be business logic");

    let score = any_func.unwrap().unified_score.total;
    assert!(score < 15.0,
        "Constructor risk score should be < 15, got {}", score);
}
```

### Performance Tests

```rust
#[test]
fn test_constructor_detection_performance() {
    let metrics: Vec<FunctionMetrics> = load_large_codebase();

    let start = Instant::now();
    for func in &metrics {
        let _ = is_simple_constructor(func);
    }
    let duration = start.elapsed();

    // Should add < 1% overhead
    assert!(duration.as_millis() < metrics.len() as u64 / 10);
}
```

## Documentation Requirements

### Code Documentation

**Inline Documentation**:
```rust
/// Detect simple constructor functions to prevent false positive classifications.
///
/// A function is considered a simple constructor if it meets ALL criteria:
/// - Has a constructor-like name (new, default, from_*, with_*, etc.)
/// - Low cyclomatic complexity (≤ 2)
/// - Short length (< 15 lines)
/// - Minimal nesting (≤ 1 level)
/// - Low cognitive complexity (≤ 3)
///
/// # Examples
///
/// ```rust
/// // Simple constructor - matches
/// fn new() -> Self { Self { field: 0 } }
///
/// // Complex factory - does NOT match
/// fn create_with_validation(data: Data) -> Result<Self> {
///     validate(data)?;
///     // ... 30 lines of logic
///     Ok(Self { ... })
/// }
/// ```
///
/// # False Positive Prevention
///
/// This function specifically addresses the false positive in ContextMatcher::any()
/// where a trivial 9-line constructor was classified as CRITICAL business logic.
pub fn is_simple_constructor(func: &FunctionMetrics) -> bool
```

### User Documentation

**Update**: `book/src/classification-system.md`

Add section:
```markdown
## Constructor Detection

Debtmap automatically detects simple constructor functions and classifies them
with lower priority to avoid false positives.

### Constructor Patterns

Functions are identified as constructors if they match:
- **Names**: `new`, `default`, `from_*`, `with_*`, `create_*`, `make_*`
- **Complexity**: Cyclomatic ≤ 2, Length < 15 lines
- **Structure**: Primarily field initialization with minimal logic

### Classification

Constructors receive:
- **Role**: IOWrapper (0.7x multiplier) instead of PureLogic (1.2x)
- **Priority**: Lower severity for uncovered constructors
- **Rationale**: Simple initialization code has lower test priority

### Examples

```rust
// Detected as constructor - LOW priority
pub fn any() -> Self {
    Self { role: None, file_type: None }
}

// Not a constructor - NORMAL priority
pub fn create_with_defaults(config: Config) -> Result<Self> {
    validate_config(&config)?;
    let processed = process_data(config.data)?;
    Ok(Self { config, processed })
}
```
```

### Architecture Updates

**Update**: `ARCHITECTURE.md`

Add to "Function Classification" section:
```markdown
### Constructor Detection (Spec 117)

Constructor functions are detected using a multi-signal approach:

1. **Name Pattern Matching**: Common constructor names (new, from_*, etc.)
2. **Complexity Thresholds**: Cyclomatic ≤ 2, Length < 15 lines
3. **Structural Analysis**: Low cognitive complexity, minimal nesting

Constructors are classified as `IOWrapper` role to reduce false positives
for trivial initialization code.
```

## Implementation Notes

### Best Practices

1. **Conservative Detection**: Better to miss a constructor than misclassify business logic
2. **Language Awareness**: Use language-specific patterns when available
3. **Configuration First**: Make patterns configurable before hardcoding
4. **Progressive Enhancement**: Start with Rust, add other languages incrementally

### Gotchas

**Avoid Over-Matching**:
- `create_user_with_validation()` - Complex, NOT a constructor
- `new_from_database()` - May have I/O, be conservative

**Edge Cases**:
- Builders with `build()` method - Different from constructors
- Factory methods with complex logic - Should NOT match
- Macros generating constructors - May need special handling

### Testing Recommendations

1. Test with real-world false positive examples (ContextMatcher::any)
2. Verify no regression for existing correct classifications
3. Add golden file tests for constructor detection
4. Monitor false positive rate in production

## Migration and Compatibility

### Breaking Changes

**None** - This is a pure enhancement to classification accuracy.

### Compatibility Considerations

- Existing code classifications may change (expected improvement)
- Some functions previously marked CRITICAL may become LOW/MODERATE
- Users should re-run analysis after upgrade for accurate results

### Migration Steps

1. Upgrade to version with this feature
2. Re-run analysis: `debtmap analyze`
3. Review updated classifications for constructors
4. Optional: Customize constructor patterns in config.toml

### Rollback Plan

If classification becomes less accurate:
1. Disable constructor detection via config: `classification.constructors.enabled = false`
2. File bug report with problematic function examples
3. Revert to previous version if critical

## Success Metrics

### Quantitative Metrics

- **False Positive Reduction**: 80% reduction in constructor false positives
- **Score Accuracy**: Constructor scores drop from ~20 to ~5-12 range
- **Performance Impact**: < 1% increase in analysis time
- **Coverage**: 95% of common constructor patterns detected

### Qualitative Metrics

- **User Satisfaction**: Fewer complaints about "trivial function marked CRITICAL"
- **Trust**: Increased confidence in debtmap's classification accuracy
- **Adoption**: More users enable strict analysis modes

### Validation

**Before Implementation**:
```
ContextMatcher::any() - SCORE: 21.2 [CRITICAL]
Role: PureLogic (1.2x multiplier)
```

**After Implementation**:
```
ContextMatcher::any() - SCORE: 8.5 [LOW]
Role: IOWrapper (0.7x multiplier)
```

## Future Enhancements

### Phase 2: AST-Based Detection (Spec 122)
- Parse function body to detect struct initialization patterns
- More accurate detection for non-standard constructor names
- Distinguish between initialization and complex factory logic

### Phase 3: Builder Pattern Detection
- Detect builder pattern methods (e.g., `with_field()`)
- Chain detection for fluent interfaces
- Separate classification for builders vs constructors

### Phase 4: ML-Based Classification
- Train model on labeled constructor/business logic examples
- Learn from user feedback and corrections
- Adaptive pattern recognition
