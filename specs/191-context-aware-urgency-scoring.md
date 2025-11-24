---
number: 191
title: Context-Aware Urgency Scoring for Example and Test Code
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-23
---

# Specification 191: Context-Aware Urgency Scoring for Example and Test Code

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None (uses existing context detection infrastructure)

## Context

Debtmap currently flags example and test code with the same urgency as production code, leading to false positives that dilute recommendation quality. Analysis of stillwater project examples revealed:

**Current Behavior**:
```
TOP 5 RECOMMENDATIONS (all in examples/):
#1 SCORE: 30.4 [CRITICAL] - examples/reader_pattern.rs:412 main()
#2 SCORE: 11.0 [CRITICAL] - examples/form_validation.rs:20 example_contact_form()
#3 SCORE: 10.1 [CRITICAL] - examples/form_validation.rs:352 example_cross_field_validation()
```

**Problem**: Example code uses inline helper functions and demonstration patterns that inflate complexity metrics but are pedagogically appropriate. This creates:
- High-urgency recommendations for low-priority code
- User confusion about what actually needs fixing
- Wasted effort investigating non-issues
- Reduced trust in debtmap's recommendations

**Existing Infrastructure**:

Debtmap ALREADY has context detection in `src/context/mod.rs`:

```rust
// File type detection (line 215)
pub fn detect_file_type(path: &Path) -> FileType {
    match () {
        _ if is_test_file(&path_str) => FileType::Test,
        _ if is_benchmark_file(&path_str) => FileType::Benchmark,
        _ if is_example_file(&path_str) => FileType::Example,  // ✓ Already implemented
        _ => FileType::Production,
    }
}

// Severity adjustment (line 200)
pub fn severity_adjustment(&self) -> i32 {
    match (self.role, self.file_type) {
        (_, FileType::Example | FileType::Documentation) => -2,  // ✓ Computed but not used!
        (_, FileType::Test) => -2,
        _ => 0,
    }
}
```

**Gap**: This severity adjustment is computed but **not integrated into urgency scoring**.

## Objective

Wire existing context detection into the urgency scoring pipeline to apply appropriate dampening factors for example, test, and benchmark code, reducing false positive urgency scores by 80-90%.

## Requirements

### Functional Requirements

**1. Integrate Context Detection into Unified Scoring**

Modify urgency score calculation to apply context-based multipliers:

```rust
// In src/priority/scoring/recommendation.rs or unified_scorer.rs
pub fn calculate_urgency_score(
    pattern: &ComplexityPattern,
    metrics: &FunctionMetrics,
    file_path: &Path,
) -> f64 {
    let base_score = calculate_base_score(pattern, metrics);

    // NEW: Apply context dampening
    let file_type = detect_file_type(file_path);
    let context_multiplier = get_context_multiplier(file_type);

    base_score * context_multiplier
}

fn get_context_multiplier(file_type: FileType) -> f64 {
    match file_type {
        FileType::Example | FileType::Documentation => 0.1,  // 90% reduction
        FileType::Test | FileType::Benchmark => 0.2,         // 80% reduction
        FileType::BuildScript => 0.3,                        // 70% reduction
        _ => 1.0,                                             // No reduction
    }
}
```

**2. Preserve Context Multiplier in Recommendation Output**

Store the applied multiplier for transparency:

```rust
pub struct DebtRecommendation {
    // ... existing fields ...
    pub context_multiplier: f64,        // NEW: Show what dampening was applied
    pub context_type: FileType,         // NEW: Show detected file type
}
```

**3. Update Output Formatting**

Show context information in recommendations:

```
#2 SCORE: 1.1 [LOW] (example code: 90% dampening applied)
├─ LOCATION: ./examples/form_validation.rs:20 example_contact_form()
├─ BASE COMPLEXITY: cyclomatic=24, cognitive=30
├─ CONTEXT: Example/demonstration code (pedagogical patterns accepted)
├─ RECOMMENDED ACTION: No action required for example code
```

### Non-Functional Requirements

**Performance**:
- File type detection should add <1ms per function
- Use existing `detect_file_type()` (already optimized)
- Cache file type per file path to avoid repeated detection

**Compatibility**:
- Must work with all existing urgency scoring code
- Should not affect production code scores
- Maintain backward compatibility with output formats

**Configurability**:
- Allow users to customize dampening factors via config
- Provide option to disable context dampening entirely
- Support per-project overrides

## Acceptance Criteria

- [ ] Context multipliers applied to urgency scores for all non-production code
- [ ] Stillwater example code urgency scores reduced by 85-95%:
  - `reader_pattern.rs:412 main()`: 30.4 → ~3.0
  - `form_validation.rs:20 example_contact_form()`: 11.0 → ~1.1
  - `form_validation.rs:352 example_cross_field_validation()`: 10.1 → ~1.0
- [ ] Production code scores unchanged (multiplier = 1.0)
- [ ] Context information displayed in output (file type, multiplier applied)
- [ ] Configuration options added to `debtmap.toml`:
  ```toml
  [scoring.context_multipliers]
  examples = 0.1
  tests = 0.2
  benchmarks = 0.3
  build_scripts = 0.3
  ```
- [ ] Unit tests verify correct multiplier application
- [ ] Integration tests confirm no regression in production code scoring
- [ ] Documentation updated with context scoring explanation

## Technical Details

### Implementation Approach

**Phase 1: Wire Context Detection (1 hour)**

1. Modify `src/priority/unified_scorer.rs` or equivalent:
   ```rust
   use crate::context::{detect_file_type, FileType};

   // In calculate_urgency_score()
   let file_type = detect_file_type(&metrics.file);
   let multiplier = get_context_multiplier(file_type);
   let adjusted_score = base_score * multiplier;
   ```

2. Add configuration support:
   ```rust
   // src/config/scoring.rs
   #[derive(Deserialize)]
   pub struct ContextMultipliers {
       pub examples: f64,           // Default: 0.1
       pub tests: f64,              // Default: 0.2
       pub benchmarks: f64,         // Default: 0.3
       pub build_scripts: f64,      // Default: 0.3
       pub documentation: f64,      // Default: 0.1
   }

   impl Default for ContextMultipliers {
       fn default() -> Self {
           Self {
               examples: 0.1,
               tests: 0.2,
               benchmarks: 0.3,
               build_scripts: 0.3,
               documentation: 0.1,
           }
       }
   }
   ```

**Phase 2: Update Output (30 minutes)**

1. Add context fields to recommendation struct
2. Update formatters to display context information
3. Add explanation for dampened scores

**Phase 3: Testing (30 minutes)**

1. Unit tests for context detection integration
2. Property tests for multiplier application
3. Integration test with stillwater examples

### Architecture Changes

**Modified Components**:
- `src/priority/unified_scorer.rs` - Apply context multipliers
- `src/priority/scoring/recommendation.rs` - Store context info
- `src/output/*.rs` - Display context information
- `src/config/mod.rs` - Add context multiplier config

**Data Flow**:
```
File Path → detect_file_type() → FileType
                                      ↓
Base Score + FileType → get_context_multiplier() → Adjusted Score
                                                         ↓
                                            DebtRecommendation (with context)
```

### Configuration Schema

```toml
# debtmap.toml
[scoring.context_multipliers]
# Dampening factors for non-production code (0.0 - 1.0)
# Lower values = more dampening (less urgency)
examples = 0.1        # 90% urgency reduction
tests = 0.2           # 80% urgency reduction
benchmarks = 0.3      # 70% urgency reduction
build_scripts = 0.3   # 70% urgency reduction
documentation = 0.1   # 90% urgency reduction

# Set to false to disable context-aware scoring
enable_context_dampening = true
```

## Dependencies

**Prerequisites**: None (uses existing infrastructure)

**Affected Components**:
- Urgency scoring pipeline
- Recommendation formatting
- Configuration loading

**External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn applies_example_code_multiplier() {
    let example_path = PathBuf::from("examples/demo.rs");
    let base_score = 30.0;

    let adjusted = calculate_urgency_with_context(base_score, &example_path);

    assert_eq!(adjusted, 3.0); // 30.0 * 0.1
}

#[test]
fn applies_no_multiplier_to_production_code() {
    let prod_path = PathBuf::from("src/main.rs");
    let base_score = 30.0;

    let adjusted = calculate_urgency_with_context(base_score, &prod_path);

    assert_eq!(adjusted, 30.0); // No change
}

#[test]
fn respects_custom_multipliers_from_config() {
    let config = Config {
        context_multipliers: ContextMultipliers {
            examples: 0.5,
            ..Default::default()
        },
        ..Default::default()
    };

    let example_path = PathBuf::from("examples/demo.rs");
    let adjusted = calculate_urgency_with_config(30.0, &example_path, &config);

    assert_eq!(adjusted, 15.0); // 30.0 * 0.5 (custom multiplier)
}
```

### Property Tests

```rust
#[test]
fn context_multiplier_never_increases_score() {
    proptest!(|(base_score in 0.0..100.0f64, file_type in file_type_strategy())| {
        let adjusted = apply_context_multiplier(base_score, file_type);
        prop_assert!(adjusted <= base_score);
    });
}

#[test]
fn example_code_always_has_lower_urgency_than_production() {
    proptest!(|(base_score in 1.0..100.0f64)| {
        let example_score = apply_context_multiplier(base_score, FileType::Example);
        let prod_score = apply_context_multiplier(base_score, FileType::Production);
        prop_assert!(example_score < prod_score);
    });
}
```

### Integration Tests

```rust
#[test]
fn stillwater_examples_have_low_urgency() {
    // Run debtmap on stillwater project
    let results = run_debtmap_on_project("../stillwater");

    // All example file recommendations should be LOW priority
    for rec in results.recommendations {
        if rec.file_path.starts_with("examples/") {
            assert!(rec.urgency_score < 5.0,
                "Example code {} has urgency {}, expected < 5.0",
                rec.location, rec.urgency_score);
            assert_eq!(rec.severity, Severity::Low);
        }
    }
}

#[test]
fn production_code_scoring_unchanged() {
    // Run on debtmap's own codebase
    let results = run_debtmap_on_project(".");

    // Production code in src/ should have normal scoring
    let prod_recs: Vec<_> = results.recommendations.iter()
        .filter(|r| r.file_path.starts_with("src/"))
        .collect();

    assert!(prod_recs.iter().any(|r| r.severity >= Severity::High),
        "Should still find high-severity issues in production code");
}
```

### Manual Validation

1. Run debtmap on stillwater:
   ```bash
   cd ../stillwater
   ../debtmap/target/release/debtmap --output markdown
   ```

2. Verify top recommendations:
   - No example files in top 10
   - Example files show LOW severity
   - Context multiplier displayed in output

3. Run on debtmap itself:
   ```bash
   cargo run -- --output markdown
   ```

4. Verify production code:
   - High-severity issues still flagged
   - No false negatives introduced

## Documentation Requirements

### Code Documentation

```rust
/// Calculate urgency score with context-aware dampening.
///
/// Applies dampening factors for non-production code (examples, tests, benchmarks)
/// to reduce false positive urgency scores while preserving production code scoring.
///
/// # Context Multipliers
///
/// - Examples: 0.1 (90% reduction) - Pedagogical patterns accepted
/// - Tests: 0.2 (80% reduction) - Test helper complexity acceptable
/// - Benchmarks: 0.3 (70% reduction) - Performance test patterns allowed
/// - Production: 1.0 (no reduction) - Full scoring applied
///
/// # Examples
///
/// ```rust
/// let example_path = PathBuf::from("examples/demo.rs");
/// let score = calculate_urgency_with_context(30.0, &example_path);
/// assert_eq!(score, 3.0); // 90% reduction
/// ```
pub fn calculate_urgency_with_context(base_score: f64, file_path: &Path) -> f64
```

### User Documentation

Add to README.md:

```markdown
## Context-Aware Scoring

Debtmap automatically applies dampening to non-production code:

- **Example files** (`examples/`): 90% urgency reduction
- **Test files** (`tests/`, `*_test.rs`): 80% urgency reduction
- **Benchmarks** (`benches/`): 70% urgency reduction

This prevents false positives from pedagogical patterns in examples
and test helper complexity.

### Customizing Dampening

```toml
# debtmap.toml
[scoring.context_multipliers]
examples = 0.1      # Default: 90% reduction
tests = 0.2         # Default: 80% reduction
benchmarks = 0.3    # Default: 70% reduction
```

### Disabling Context Dampening

```toml
[scoring]
enable_context_dampening = false
```
```

### Architecture Documentation

Update `ARCHITECTURE.md`:

```markdown
## Context-Aware Scoring

Debtmap applies context-based dampening to urgency scores to reduce false
positives in non-production code:

1. **File Type Detection** (`src/context/mod.rs`):
   - Detects example, test, benchmark files
   - Uses path-based heuristics

2. **Context Multiplier Application** (`src/priority/unified_scorer.rs`):
   - Applies dampening factor based on file type
   - Configurable via `debtmap.toml`

3. **Output Formatting** (`src/output/*.rs`):
   - Displays context information
   - Shows applied multiplier for transparency
```

## Implementation Notes

### Edge Cases

**1. Mixed Production and Example Code**:
- Some projects have examples in `src/bin/examples/`
- Detection should prioritize path patterns over file location
- Document detection precedence

**2. Test Modules in Production Files**:
```rust
// src/lib.rs
#[cfg(test)]
mod tests {
    fn helper() { ... }  // Should this be dampened?
}
```
- Current approach: whole file dampening (based on path)
- Alternative: function-level detection (more complex)
- **Decision**: Start with file-level, add function-level if needed

**3. Zero Scores**:
- Very low base scores (< 1.0) * 0.1 → ~0.0
- Should these be filtered from output entirely?
- **Decision**: Keep in output but mark as "negligible"

### Performance Considerations

**File Type Detection Caching**:
```rust
use std::collections::HashMap;

struct ScoringContext {
    file_type_cache: HashMap<PathBuf, FileType>,
}

impl ScoringContext {
    fn get_file_type(&mut self, path: &Path) -> FileType {
        self.file_type_cache
            .entry(path.to_path_buf())
            .or_insert_with(|| detect_file_type(path))
            .clone()
    }
}
```

**Benchmarking**:
- Measure before/after scoring performance
- Target: <5% overhead from context detection
- Profile with `cargo bench --bench scoring_performance`

## Migration and Compatibility

### Breaking Changes

None - purely additive feature.

### Backward Compatibility

**Output Format**:
- New fields added to recommendation output
- Existing fields unchanged
- JSON output gets new optional fields

**Configuration**:
- All new config keys have defaults
- Existing configs work without modification
- `enable_context_dampening = false` restores old behavior

### Migration Path

1. No migration required for existing users
2. Automatic benefits from context dampening
3. Can opt-out via config if needed

### Rollback Plan

If issues discovered:
1. Set `enable_context_dampening = false` in config
2. Revert to previous behavior immediately
3. No data migration needed

## Success Metrics

**Quantitative**:
- 85-95% reduction in example file urgency scores
- 0% change in production code urgency scores
- <5% performance overhead
- 0 user-reported false negatives

**Qualitative**:
- User feedback: "Recommendations are more actionable"
- Reduced confusion about example code issues
- Increased trust in debtmap's prioritization

## Future Enhancements

**Function-Level Context Detection** (Spec 192):
- Detect `#[cfg(test)]` modules within production files
- Apply dampening to specific functions, not whole files
- More granular control

**Dynamic Multiplier Learning** (future):
- Learn optimal multipliers from user feedback
- Adjust based on codebase characteristics
- ML-based context detection

**Custom Context Rules** (future):
- User-defined context patterns
- Per-directory multipliers
- Regex-based file type detection
