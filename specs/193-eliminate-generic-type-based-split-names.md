---
number: 193
title: Eliminate Generic Type-Based Split Names
category: optimization
priority: medium
status: draft
dependencies: [174, 175, 191, 192]
created: 2025-11-21
---

# Specification 193: Eliminate Generic Type-Based Split Names

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 174, 175, 191, 192

## Context

After implementing Fixes #1-4 (specs 174, 175, 191, 192), god object split recommendations show excellent split counts (3-6 modules instead of 32-47) but suffer from generic type-based names that bypass semantic naming quality checks.

### Current Problem

**Example output (god_object_analysis.rs)**:
```
- unknown.rs                (29 methods) ← Generic!
- transformations.rs        (8 methods)  ← Generic!
- computation.rs            (3 methods)  ← Generic!
```

**Example output (formatter.rs)**:
```
- unknown.rs                (9 methods)  ← Generic!
- formatting.rs             (5 methods)  ← Somewhat generic
- unifiedanalysis.rs        (13 methods) ← Type name leak
- filedebtitem.rs           (3 methods)  ← Type name leak
```

### Root Cause

Type-based clustering (a fallback when behavioral clustering doesn't produce results) generates category names from Rust type names:

```rust
Category: Manage [TypeName] data and its transformations

Where [TypeName] is:
- Unknown        → unknown.rs
- Self           → transformations.rs
- GodObjectThresholds → computation.rs
- ColoredFormatter    → formatting.rs
- FileDebtItem        → filedebtitem.rs
```

This bypasses semantic naming (Spec 191) because:
1. Type-based clustering sets the `responsibility` field to "Manage X data"
2. Semantic naming uses this responsibility field as input
3. Generic type names leak through to module names
4. Specificity scorer (Spec 191) doesn't reject these names

### Why Files Use Type-Based Clustering

Files that should use behavioral clustering (>50 methods AND >500 LOC) are falling back to type-based:
- god_object_analysis.rs: 161 functions, 3578 lines
- formatter.rs: 103 functions, 3004 lines

**Hypothesis**: Behavioral clustering returns empty results (splits.len() <= 1) causing fallback to type-based clustering which then produces generic names.

### Impact

**User Experience**:
- "unknown.rs" provides no semantic information about module contents
- "transformations.rs" is too vague to be useful
- "computation.rs" could mean anything
- Users can't understand module purpose from name alone

**Code Quality**:
- Module names don't follow Rust naming conventions
- Violates single responsibility principle (name should indicate responsibility)
- Makes codebase harder to navigate
- Reduces confidence in automated refactoring tools

## Objective

Eliminate generic type-based names from split recommendations by:
1. Strengthening semantic naming to reject generic type names
2. Improving behavioral clustering to reduce fallback to type-based
3. Adding domain-aware naming for type-based fallback path

**Success Criteria**: <2% of split names contain "unknown", "transformations", "computation", "formatting", "self", or other generic terms.

## Requirements

### Functional Requirements

**FR1: Generic Name Detection**
- Detect generic type names before they become module names
- Identify patterns: "Unknown", "Self", "*Formatter", "*Item", "*Analysis"
- Flag names that are too short (<5 chars) or too vague

**FR2: Enhanced Specificity Scoring**
- Extend Spec 191's specificity scorer to handle type-based names
- Add type-name-specific rejection criteria
- Increase threshold for type-based splits (0.60 → 0.75)

**FR3: Behavioral Clustering Diagnostics**
- Log why behavioral clustering returns empty results
- Identify which files fall back to type-based clustering
- Provide actionable diagnostics for improving behavioral clustering

**FR4: Domain-Aware Type-Based Naming**
- Integrate domain pattern detection (Spec 175) into type-based clustering
- Use method name analysis to infer semantic names
- Apply responsibility classification even for type-based splits

**FR5: Fallback Naming Strategy**
- When type name is generic, analyze method responsibilities
- Generate name from dominant method pattern (e.g., "validation", "parsing")
- Use method verb extraction as last resort (e.g., "calculate", "format")

### Non-Functional Requirements

**NFR1: Performance**
- Generic name detection adds <5ms per file
- Domain pattern integration adds <10ms per file
- Total analysis time increase <2%

**NFR2: Backward Compatibility**
- Existing good split names remain unchanged
- Only affect type-based clustering output
- No changes to behavioral clustering algorithm

**NFR3: Maintainability**
- Generic name patterns defined in configuration
- Easy to add new rejection patterns
- Clear separation of concerns (detection vs. correction)

## Acceptance Criteria

### Core Functionality

- [ ] **AC1**: Generic name detection identifies "unknown", "transformations", "computation", "formatting", "self"
- [ ] **AC2**: Specificity scorer rejects type-based names with score <0.75
- [ ] **AC3**: Domain pattern detection runs for all type-based splits
- [ ] **AC4**: Method verb extraction generates semantic fallback names
- [ ] **AC5**: No split recommendations contain "unknown.rs" or "self.rs"

### Quality Metrics

- [ ] **AC6**: <2% of split names are generic (down from ~15%)
- [ ] **AC7**: Type-based splits have average specificity score >0.70
- [ ] **AC8**: All split names are >5 characters (excluding file extension)
- [ ] **AC9**: Split names match Rust module naming conventions (snake_case)

### Diagnostics and Logging

- [ ] **AC10**: Log when behavioral clustering returns empty (with reason)
- [ ] **AC11**: Report which files use type-based fallback
- [ ] **AC12**: Show confidence scores for type-based split names

### Testing

- [ ] **AC13**: Unit tests for generic name detection (10+ test cases)
- [ ] **AC14**: Integration tests verify no generic names in output
- [ ] **AC15**: Property test: all split names have specificity score >0.60
- [ ] **AC16**: Test on god_object_analysis.rs and formatter.rs

## Technical Details

### Implementation Approach

**Phase 1: Generic Name Detection** (1-2 hours)
```rust
// Location: src/organization/semantic_naming/specificity_scorer.rs

/// Patterns that indicate generic, uninformative names
const GENERIC_TYPE_PATTERNS: &[&str] = &[
    "unknown", "self", "transformations", "computation",
    "formatting", "item", "data", "utils", "helpers",
    "misc", "other", "common", "shared", "base",
];

fn is_generic_type_name(name: &str) -> bool {
    let normalized = name.to_lowercase();

    // Check against known generic patterns
    if GENERIC_TYPE_PATTERNS.iter().any(|p| normalized.contains(p)) {
        return true;
    }

    // Too short to be meaningful
    if name.len() < 5 {
        return true;
    }

    // All caps or numbers (e.g., "T", "U", "X123")
    if name.chars().all(|c| c.is_uppercase() || c.is_numeric()) {
        return true;
    }

    false
}
```

**Phase 2: Enhanced Specificity Scoring** (1 hour)
```rust
// Location: src/organization/semantic_naming/specificity_scorer.rs

impl SpecificityScorer {
    /// Calculate specificity score with type-aware penalties
    pub fn score_with_type_awareness(&self, name: &str, is_type_based: bool) -> f64 {
        let mut score = self.score(name); // Base score from Spec 191

        // Apply stricter penalties for type-based splits
        if is_type_based {
            if is_generic_type_name(name) {
                score *= 0.3; // Heavy penalty for generic names
            }

            // Require higher threshold for type-based splits
            if score < 0.75 {
                score *= 0.8; // Additional penalty
            }
        }

        score
    }
}
```

**Phase 3: Domain-Aware Type-Based Naming** (2-3 hours)
```rust
// Location: src/organization/god_object_detector.rs

fn generate_type_based_splits_with_semantics(
    ast: &syn::File,
    base_name: &str,
    file_path: &Path,
) -> Vec<ModuleSplit> {
    // Existing type-based clustering
    let mut splits = Self::generate_type_based_splits(ast, base_name, file_path);

    // NEW: Apply domain pattern detection
    for split in &mut splits {
        let methods = &split.methods_to_move;

        // Try domain pattern detection first
        if let Some(domain_name) = detect_domain_pattern_for_methods(methods, ast) {
            split.responsibility = domain_name;
        }
        // Fallback to method verb extraction
        else if let Some(verb_name) = extract_dominant_verb(methods) {
            split.responsibility = format!("{} Operations", verb_name);
        }
    }

    // Apply semantic naming (will now have better input)
    Self::apply_semantic_naming_to_splits(&mut splits, file_path);

    // Filter out remaining generic names
    splits.retain(|s| {
        let specificity = score_with_type_awareness(&s.suggested_name, true);
        specificity >= 0.75
    });

    splits
}
```

**Phase 4: Method Verb Extraction** (1-2 hours)
```rust
// Location: src/organization/semantic_naming/domain_extractor.rs

/// Extract dominant verb from method names
fn extract_dominant_verb(methods: &[String]) -> Option<String> {
    use std::collections::HashMap;

    // Common method verbs in order of specificity
    let verbs = [
        "validate", "parse", "format", "calculate", "transform",
        "convert", "generate", "analyze", "detect", "classify",
        "build", "create", "update", "delete", "query",
    ];

    // Count verb occurrences
    let mut verb_counts: HashMap<&str, usize> = HashMap::new();
    for method in methods {
        let method_lower = method.to_lowercase();
        for verb in &verbs {
            if method_lower.starts_with(verb) {
                *verb_counts.entry(verb).or_default() += 1;
            }
        }
    }

    // Return most common verb if it covers >30% of methods
    verb_counts
        .into_iter()
        .filter(|(_, count)| (*count as f64 / methods.len() as f64) > 0.3)
        .max_by_key(|(_, count)| *count)
        .map(|(verb, _)| capitalize_first(verb))
}
```

**Phase 5: Behavioral Clustering Diagnostics** (1 hour)
```rust
// Location: src/organization/god_object_detector.rs

fn generate_behavioral_splits(...) -> Vec<ModuleSplit> {
    // ... existing clustering code ...

    // NEW: Log why we're returning empty
    if splits.is_empty() {
        eprintln!(
            "⚠ Behavioral clustering returned empty for {} ({} methods, {} lines)",
            file_name, all_methods.len(), estimate_loc(ast)
        );
        eprintln!("  Reasons:");
        if clusters.is_empty() {
            eprintln!("    - No clusters produced by algorithm");
        } else {
            eprintln!("    - {} clusters filtered out (< min size)", clusters.len());
        }
        eprintln!("  Falling back to type-based clustering");
    }

    splits
}
```

### Architecture Changes

**Modified Components**:
1. `src/organization/semantic_naming/specificity_scorer.rs`
   - Add `is_generic_type_name()` function
   - Add `score_with_type_awareness()` method
   - Extend rejection patterns

2. `src/organization/semantic_naming/domain_extractor.rs`
   - Add `extract_dominant_verb()` function
   - Add verb frequency analysis

3. `src/organization/god_object_detector.rs`
   - Modify `generate_type_based_splits()` to use domain detection
   - Add diagnostic logging for behavioral clustering failures
   - Filter out low-specificity type-based names

**New Data Structures**:
```rust
/// Configuration for generic name detection
pub struct GenericNameConfig {
    /// Patterns to reject
    pub reject_patterns: Vec<String>,
    /// Minimum name length
    pub min_length: usize,
    /// Minimum specificity score for type-based splits
    pub type_based_threshold: f64,
}

impl Default for GenericNameConfig {
    fn default() -> Self {
        Self {
            reject_patterns: vec![
                "unknown".to_string(),
                "self".to_string(),
                "transformations".to_string(),
                // ... more patterns
            ],
            min_length: 5,
            type_based_threshold: 0.75,
        }
    }
}
```

### Integration Points

**1. Type-Based Clustering Path**
- Before: `type_based_splits` → semantic naming → output
- After: `type_based_splits` → domain detection → verb extraction → semantic naming → quality filter → output

**2. Semantic Naming Integration**
- Provide better input to SemanticNameGenerator
- Use method analysis results as hint
- Apply stricter filtering for type-based origins

**3. Domain Pattern Detection**
- Reuse Spec 175's DomainPatternDetector
- Apply to methods in type-based splits
- Use confidence scores to choose between pattern name vs. verb name

## Dependencies

### Prerequisites
- **Spec 174**: Confidence-based responsibility classification (provides classification framework)
- **Spec 175**: Domain pattern detection (reuse for type-based splits)
- **Spec 191**: Semantic module naming (extend for type-based awareness)
- **Spec 192**: Improved responsibility clustering (clustering algorithm)

### Affected Components
- `src/organization/semantic_naming/` - All files (extend functionality)
- `src/organization/god_object_detector.rs` - Modify type-based path
- `src/organization/domain_patterns.rs` - Reuse for type-based splits

### External Dependencies
None (uses existing infrastructure)

## Testing Strategy

### Unit Tests

**Generic Name Detection** (`tests/generic_name_detection.rs`):
```rust
#[test]
fn test_detect_generic_unknown() {
    assert!(is_generic_type_name("unknown"));
    assert!(is_generic_type_name("Unknown"));
    assert!(is_generic_type_name("UNKNOWN"));
}

#[test]
fn test_detect_generic_transformations() {
    assert!(is_generic_type_name("transformations"));
    assert!(is_generic_type_name("Self"));
}

#[test]
fn test_accept_specific_names() {
    assert!(!is_generic_type_name("validation_rules"));
    assert!(!is_generic_type_name("responsibility_classifier"));
}
```

**Verb Extraction** (`tests/verb_extraction.rs`):
```rust
#[test]
fn test_extract_validation_verb() {
    let methods = vec![
        "validate_input".to_string(),
        "validate_output".to_string(),
        "validate_schema".to_string(),
    ];
    assert_eq!(extract_dominant_verb(&methods), Some("Validate".to_string()));
}

#[test]
fn test_no_dominant_verb() {
    let methods = vec![
        "process_a".to_string(),
        "handle_b".to_string(),
        "compute_c".to_string(),
    ];
    assert_eq!(extract_dominant_verb(&methods), None);
}
```

### Integration Tests

**End-to-End Generic Name Elimination** (`tests/no_generic_names.rs`):
```rust
#[test]
fn test_god_object_analysis_no_generic_names() {
    let source = include_str!("../src/organization/god_object_analysis.rs");
    let result = analyze_god_object(source);

    for split in &result.recommended_splits {
        let name = split.suggested_name.to_lowercase();

        // No generic patterns
        assert!(!name.contains("unknown"), "Found 'unknown' in {}", split.suggested_name);
        assert!(!name.contains("transformations"), "Found 'transformations' in {}", split.suggested_name);
        assert!(!name.contains("self"), "Found 'self' in {}", split.suggested_name);

        // Minimum length
        let module_name = name.split('/').last().unwrap().replace(".rs", "");
        assert!(module_name.len() >= 5, "Module name too short: {}", module_name);

        // Has specificity score
        assert!(split.naming_confidence.unwrap_or(0.0) >= 0.70);
    }
}
```

**Type-Based Fallback Quality** (`tests/type_based_quality.rs`):
```rust
#[test]
fn test_type_based_splits_have_semantic_names() {
    // Test on files that trigger type-based clustering
    let files = [
        "src/organization/god_object_analysis.rs",
        "src/priority/formatter.rs",
    ];

    for file_path in &files {
        let source = std::fs::read_to_string(file_path).unwrap();
        let result = analyze_god_object(&source);

        // All splits should have semantic names
        for split in &result.recommended_splits {
            // Check specificity score
            let score = split.naming_confidence.unwrap_or(0.0);
            assert!(score >= 0.70, "Low specificity score {} for {}", score, split.suggested_name);

            // Check against generic patterns
            assert!(!is_generic_type_name(&split.suggested_name));
        }
    }
}
```

### Property Tests

**Specificity Invariants** (`tests/property_tests.rs`):
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_all_splits_meet_threshold(
        methods in prop::collection::vec(any::<String>(), 10..100)
    ) {
        let splits = generate_splits_for_methods(&methods);

        for split in splits {
            let score = split.naming_confidence.unwrap_or(0.0);
            prop_assert!(score >= 0.60, "Split {} has low score {}", split.suggested_name, score);
        }
    }
}
```

### Performance Tests

**Benchmark Type-Based Naming** (`benches/type_based_naming.rs`):
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_generic_name_detection(c: &mut Criterion) {
    c.bench_function("is_generic_type_name", |b| {
        b.iter(|| {
            is_generic_type_name(black_box("unknown"));
            is_generic_type_name(black_box("validation_rules"));
        });
    });
}

fn benchmark_verb_extraction(c: &mut Criterion) {
    let methods = vec!["validate_input".to_string(); 20];

    c.bench_function("extract_dominant_verb", |b| {
        b.iter(|| extract_dominant_verb(black_box(&methods)));
    });
}

criterion_group!(benches, benchmark_generic_name_detection, benchmark_verb_extraction);
criterion_main!(benches);
```

## Documentation Requirements

### Code Documentation

**1. Module-Level Documentation**
- Document generic name detection patterns
- Explain verb extraction algorithm
- Describe type-based naming strategy

**2. Function Documentation**
```rust
/// Detects if a module name is too generic to be useful.
///
/// Generic names include:
/// - "unknown", "self", "transformations" (provide no semantic info)
/// - Type names like "Item", "Data", "Formatter" (too vague)
/// - Short names (<5 chars) (likely abbreviations or type vars)
///
/// # Examples
/// ```
/// assert!(is_generic_type_name("unknown"));
/// assert!(!is_generic_type_name("validation_rules"));
/// ```
pub fn is_generic_type_name(name: &str) -> bool {
    // ...
}
```

### User Documentation

**1. Update EVALUATION.md**
- Remove "Remaining Issue: Generic Names" section
- Add "Resolved: Generic Names" section
- Update success metrics

**2. Update FIXES_IMPLEMENTED.md**
- Add "Fix #5: Generic Name Elimination" section
- Document new approach and results

**3. Add Migration Guide** (if needed)
- Explain changes to type-based split naming
- Show before/after examples
- Provide configuration options

### Architecture Documentation

**Update ARCHITECTURE.md**:
- Document generic name detection mechanism
- Explain type-based vs. behavioral split naming
- Describe verb extraction algorithm
- Add decision flowchart for split naming

## Implementation Notes

### Gotchas and Best Practices

**1. Type Name Extraction**
- Rust type names may be qualified (`std::vec::Vec` → `Vec`)
- Generic types need special handling (`Option<T>` → `Option`)
- Trait names may be confused with type names

**2. Verb Extraction Accuracy**
- Method prefixes may be misleading (`format_data` vs `format_string`)
- Require >30% coverage to ensure dominant pattern
- Fall back to method name prefix extraction if no verb matches

**3. Performance Considerations**
- Generic name detection is O(n) in pattern count (small n)
- Verb extraction is O(methods × verbs) - keep verb list small
- Domain pattern detection is expensive - cache results

**4. Edge Cases**
- Files with no clear responsibility pattern (use "mixed_operations")
- All methods are getters/setters (use "accessors")
- Type name is actually descriptive (e.g., "ValidationRules" → keep it)

### Configuration

**Recommended Defaults**:
```toml
[split_naming]
# Generic name detection
reject_patterns = ["unknown", "self", "transformations", "computation"]
min_name_length = 5

# Type-based specificity
type_based_threshold = 0.75
behavioral_threshold = 0.60

# Verb extraction
min_verb_coverage = 0.30
max_verbs = 20
```

### Debugging Support

**Debug Flag**: `--debug-split-naming`
```
$ debtmap analyze . --debug-split-naming

Analyzing: god_object_analysis.rs
  Behavioral clustering: EMPTY (no clusters)
  Falling back to type-based clustering

  Type-based splits:
    - "Unknown" → rejected (generic pattern)
    - "Self" → rejected (generic pattern)
    - "GodObjectThresholds" → "computation" → rejected (specificity: 0.45)

  Applying domain pattern detection:
    - Methods: validate, total, validate_name → "Validation" (confidence: 0.72)

  Final splits:
    - validation_operations.rs (29 methods, specificity: 0.82)
```

## Migration and Compatibility

### Breaking Changes
None - this only affects the names of recommended splits, not actual code.

### Behavioral Changes

**1. Fewer Type-Based Splits**
- Before: Accept all type-based splits
- After: Filter out low-specificity splits
- Impact: Some files may show fewer split recommendations

**2. Different Split Names**
- Before: "unknown.rs", "transformations.rs"
- After: "validation_operations.rs", "formatting_helpers.rs"
- Impact: Users see more semantic names

### Configuration Migration

**Old Configuration** (if any):
```toml
[split_naming]
min_specificity = 0.60
```

**New Configuration**:
```toml
[split_naming]
min_specificity = 0.60  # Behavioral splits
type_based_threshold = 0.75  # Type-based splits (stricter)
```

### Rollback Strategy

If issues arise:
1. Set `type_based_threshold` back to 0.60
2. Disable generic name filtering: `reject_patterns = []`
3. Fall back to previous type-based naming (no domain detection)

## Success Metrics

### Quantitative Metrics
- **Generic name rate**: <2% (currently ~15%)
- **Average specificity score**: >0.75 for type-based splits (currently ~0.50)
- **User satisfaction**: >80% find split names useful (survey)

### Qualitative Metrics
- Split names clearly indicate module responsibility
- Users can navigate to module without reading implementation
- Names follow Rust naming conventions

### Before/After Comparison

**Before (Current)**:
```
god_object_analysis.rs:
  - unknown.rs (29 methods)
  - transformations.rs (8 methods)
  - computation.rs (3 methods)

formatter.rs:
  - unknown.rs (9 methods)
  - formatting.rs (5 methods)
```

**After (Target)**:
```
god_object_analysis.rs:
  - responsibility_classification.rs (29 methods)
  - type_transformations.rs (8 methods)
  - scoring_calculations.rs (3 methods)

formatter.rs:
  - terminal_formatting.rs (9 methods)
  - priority_formatting.rs (5 methods)
```

## Timeline Estimate

- **Phase 1**: Generic name detection (1-2 hours)
- **Phase 2**: Enhanced specificity scoring (1 hour)
- **Phase 3**: Domain-aware type-based naming (2-3 hours)
- **Phase 4**: Method verb extraction (1-2 hours)
- **Phase 5**: Behavioral clustering diagnostics (1 hour)
- **Testing**: Unit and integration tests (2-3 hours)
- **Documentation**: Code and user docs (1-2 hours)

**Total Estimated Effort**: 10-14 hours (1.5-2 days)

## References

- Spec 174: Confidence-based responsibility classification
- Spec 175: Domain pattern detection for semantic clustering
- Spec 191: Semantic module naming
- Spec 192: Improved responsibility clustering
- FIX4_SUMMARY.md: Current state and remaining issues
- EVALUATION.md: Detailed analysis of generic name problem
