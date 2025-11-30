---
number: 181
title: Split god_object_detector.rs into Focused Submodules
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 181: Split god_object_detector.rs into Focused Submodules

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The `src/god_object_detector.rs` file has grown to 4,363 lines, violating the Stillwater philosophy principle of "Composition Over Complexity" and the project's guideline of keeping files under 200 lines. This massive file contains multiple responsibilities:

- Detection logic for identifying god objects
- Classification of god object types
- Recommendation generation
- Scoring algorithms
- Output formatting

According to STILLWATER_EVALUATION.md, this file should be split into focused submodules, each with a single responsibility. Large files make it difficult to:

- Understand the code structure
- Navigate and maintain the codebase
- Test individual components
- Apply functional programming principles
- Collaborate effectively (merge conflicts)

## Objective

Refactor `src/god_object_detector.rs` (4,363 lines) into a modular structure under `src/god_object/` with clear separation of concerns:

- **detector.rs** - Pure detection logic (identifies god objects from metrics)
- **classifier.rs** - Pure classification logic (categorizes god object types)
- **recommender.rs** - Pure recommendation generation (suggests fixes)
- **scoring.rs** - Pure scoring algorithms (calculates god object severity)
- **types.rs** - Data structures and type definitions
- **mod.rs** - Public API and composition

Each module should be under 500 lines and follow functional programming principles with pure core functions and I/O at boundaries.

## Requirements

### Functional Requirements

1. **Module Structure**
   - Create `src/god_object/` directory
   - Split functionality into 6 focused modules
   - Each module has single responsibility
   - No module exceeds 500 lines

2. **Pure Core Implementation**
   - Detection logic operates on parsed metrics (no I/O)
   - Classification uses pure pattern matching
   - Recommendations are pure transformations
   - Scoring calculations are deterministic

3. **Public API**
   - Preserve existing public API in `mod.rs`
   - Re-export necessary types and functions
   - Maintain backward compatibility
   - Clear documentation of module boundaries

4. **Dependency Direction**
   - `types.rs` has no internal dependencies (foundation)
   - `detector.rs` depends only on `types.rs`
   - `classifier.rs` depends on `types.rs` and `detector.rs`
   - `recommender.rs` depends on `types.rs` and `classifier.rs`
   - `scoring.rs` depends on `types.rs`
   - `mod.rs` composes all modules

### Non-Functional Requirements

1. **Performance**
   - No performance regression from refactoring
   - Maintain parallel processing capabilities
   - Zero additional heap allocations

2. **Maintainability**
   - Each module independently testable
   - Clear boundaries between concerns
   - Comprehensive module-level documentation

3. **Testability**
   - Existing tests continue to pass
   - New tests added for individual modules
   - Unit tests for pure functions

## Acceptance Criteria

- [ ] Directory `src/god_object/` created with 6 module files
- [ ] `detector.rs` contains only detection logic (<500 lines)
- [ ] `classifier.rs` contains only classification logic (<500 lines)
- [ ] `recommender.rs` contains only recommendation logic (<500 lines)
- [ ] `scoring.rs` contains only scoring algorithms (<500 lines)
- [ ] `types.rs` contains shared data structures (<300 lines)
- [ ] `mod.rs` provides public API and composition (<200 lines)
- [ ] Original `god_object_detector.rs` deleted
- [ ] All existing tests pass without modification
- [ ] Each module has module-level documentation
- [ ] Pure functions separated from I/O operations
- [ ] No circular dependencies between modules
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes with no failures

## Technical Details

### Implementation Approach

**Phase 1: Analysis**
1. Read `src/god_object_detector.rs` in full
2. Identify all public API functions
3. Map dependencies between functions
4. Group related functions by responsibility
5. Identify pure vs impure functions

**Phase 2: Module Creation**
1. Create `src/god_object/` directory
2. Create `types.rs` with shared data structures
3. Extract detection logic to `detector.rs`
4. Extract classification logic to `classifier.rs`
5. Extract recommendation logic to `recommender.rs`
6. Extract scoring algorithms to `scoring.rs`
7. Create `mod.rs` with public API

**Phase 3: Integration**
1. Update imports throughout codebase
2. Re-export public types from `mod.rs`
3. Ensure backward compatibility
4. Update Cargo.toml if needed

**Phase 4: Validation**
1. Run full test suite
2. Run clippy checks
3. Verify no performance regression
4. Review module boundaries

### Module Responsibilities

**types.rs** (Foundation)
```rust
// Shared data structures
pub struct GodObject { ... }
pub struct GodObjectMetrics { ... }
pub enum GodObjectType { ... }
pub struct Recommendation { ... }
```

**detector.rs** (Detection Logic)
```rust
// Pure detection functions
pub fn detect_god_objects(metrics: &[FileMetrics]) -> Vec<GodObject>
pub fn is_god_object(metrics: &ComplexityMetrics) -> bool
fn calculate_method_count(metrics: &ComplexityMetrics) -> usize
```

**classifier.rs** (Classification Logic)
```rust
// Pure classification functions
pub fn classify_god_object(god_obj: &GodObject) -> GodObjectType
fn detect_data_class(god_obj: &GodObject) -> bool
fn detect_service_class(god_obj: &GodObject) -> bool
```

**recommender.rs** (Recommendation Generation)
```rust
// Pure recommendation functions
pub fn generate_recommendations(god_obj: &GodObject) -> Vec<Recommendation>
fn suggest_extraction(god_obj: &GodObject) -> Option<Recommendation>
fn suggest_splitting(god_obj: &GodObject) -> Option<Recommendation>
```

**scoring.rs** (Scoring Algorithms)
```rust
// Pure scoring functions
pub fn calculate_severity_score(god_obj: &GodObject) -> f64
fn complexity_weight(complexity: u32) -> f64
fn size_weight(lines: usize) -> f64
```

**mod.rs** (Public API)
```rust
// Re-exports and composition
pub use detector::detect_god_objects;
pub use classifier::{classify_god_object, GodObjectType};
pub use recommender::generate_recommendations;
pub use scoring::calculate_severity_score;
pub use types::*;

// High-level API functions
pub fn analyze_god_objects(files: &[FileMetrics]) -> AnalysisReport {
    let detected = detect_god_objects(files);
    let classified = detected.iter().map(classify_god_object);
    let recommendations = detected.iter().flat_map(generate_recommendations);
    AnalysisReport { detected, classified, recommendations }
}
```

### Architecture Changes

**Before:**
```
src/
  god_object_detector.rs (4,363 lines)
```

**After:**
```
src/
  god_object/
    mod.rs         (~200 lines - public API)
    types.rs       (~300 lines - data structures)
    detector.rs    (~500 lines - detection logic)
    classifier.rs  (~500 lines - classification)
    recommender.rs (~500 lines - recommendations)
    scoring.rs     (~300 lines - scoring algorithms)
```

### Data Structures

No new data structures needed. Existing types moved to `types.rs`:

```rust
// types.rs
pub struct GodObject {
    pub name: String,
    pub path: PathBuf,
    pub metrics: GodObjectMetrics,
    pub classification: Option<GodObjectType>,
}

pub struct GodObjectMetrics {
    pub method_count: usize,
    pub field_count: usize,
    pub complexity: u32,
    pub lines_of_code: usize,
}

pub enum GodObjectType {
    DataClass,
    ServiceClass,
    UtilityClass,
    ManagerClass,
}

pub struct Recommendation {
    pub description: String,
    pub priority: Priority,
    pub suggested_refactoring: RefactoringType,
}
```

### APIs and Interfaces

**Public API (preserved in mod.rs):**

```rust
// Main entry point
pub fn detect_god_objects(metrics: &[FileMetrics]) -> Vec<GodObject>;

// Classification
pub fn classify_god_object(god_obj: &GodObject) -> GodObjectType;

// Recommendations
pub fn generate_recommendations(god_obj: &GodObject) -> Vec<Recommendation>;

// Scoring
pub fn calculate_severity_score(god_obj: &GodObject) -> f64;

// High-level analysis
pub fn analyze_god_objects(files: &[FileMetrics]) -> AnalysisReport;
```

**Internal Module APIs:**

Each module exports focused functionality:
- `detector` - detection predicates and metrics extraction
- `classifier` - classification rules and type detection
- `recommender` - recommendation strategies
- `scoring` - scoring algorithms and weights

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/commands/analyze.rs` (imports god_object_detector)
  - `src/debt/mod.rs` (may use god object detection)
  - Any test files importing god_object_detector
- **External Dependencies**: None (uses existing dependencies)

## Testing Strategy

### Unit Tests

**Per-Module Testing:**

```rust
// detector.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_god_objects_pure() {
        let metrics = create_test_metrics();
        let result1 = detect_god_objects(&metrics);
        let result2 = detect_god_objects(&metrics);
        assert_eq!(result1, result2); // Deterministic
    }

    #[test]
    fn test_is_god_object_threshold() {
        let high_complexity = create_high_complexity_metrics();
        assert!(is_god_object(&high_complexity));
    }
}

// classifier.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_data_class() {
        let god_obj = create_data_class_example();
        assert_eq!(classify_god_object(&god_obj), GodObjectType::DataClass);
    }
}
```

### Integration Tests

```rust
// tests/god_object_integration.rs
#[test]
fn test_full_god_object_analysis() {
    let files = load_test_files();
    let report = god_object::analyze_god_objects(&files);

    assert!(!report.detected.is_empty());
    assert_eq!(report.detected.len(), report.recommendations.len());
}

#[test]
fn test_backward_compatibility() {
    // Ensure existing API still works
    let metrics = create_test_metrics();
    let result = god_object::detect_god_objects(&metrics);
    assert!(result.is_ok());
}
```

### Performance Tests

```rust
#[test]
fn benchmark_detection_performance() {
    let large_dataset = create_large_test_dataset(1000);

    let start = Instant::now();
    let result = detect_god_objects(&large_dataset);
    let duration = start.elapsed();

    // Ensure no performance regression
    assert!(duration < Duration::from_secs(1));
}
```

## Documentation Requirements

### Code Documentation

**Module-level docs for each file:**

```rust
// detector.rs
//! God Object Detection
//!
//! This module contains pure functions for detecting god objects in code metrics.
//! All functions are deterministic and side-effect free.
//!
//! # Examples
//!
//! ```
//! use debtmap::god_object::detector::detect_god_objects;
//!
//! let metrics = load_file_metrics();
//! let god_objects = detect_god_objects(&metrics);
//! ```

// Each public function
/// Detects god objects from file metrics.
///
/// # Arguments
///
/// * `metrics` - Slice of file metrics to analyze
///
/// # Returns
///
/// Vector of detected god objects with their metrics
///
/// # Examples
///
/// ```
/// let detected = detect_god_objects(&metrics);
/// assert!(detected.iter().all(|g| g.metrics.method_count > 20));
/// ```
pub fn detect_god_objects(metrics: &[FileMetrics]) -> Vec<GodObject> {
    // ...
}
```

### User Documentation

No user-facing documentation changes needed (internal refactoring).

### Architecture Updates

Add section to `ARCHITECTURE.md`:

```markdown
## God Object Detection Module

The god object detection system is organized as follows:

- `god_object::detector` - Pure detection logic
- `god_object::classifier` - Pure classification rules
- `god_object::recommender` - Pure recommendation generation
- `god_object::scoring` - Pure scoring algorithms
- `god_object::types` - Shared data structures

All modules follow functional programming principles with pure core logic
and no I/O operations.
```

## Implementation Notes

### Refactoring Strategy

1. **Create new module structure** without deleting original
2. **Copy functions** to appropriate modules (don't move yet)
3. **Update imports** in new modules
4. **Test each module** independently
5. **Update public API** in mod.rs
6. **Switch codebase** to use new modules
7. **Delete original** file only after all tests pass

### Common Pitfalls

1. **Circular dependencies** - Ensure dependency graph is acyclic
2. **Lost functionality** - Verify all functions moved
3. **Test breakage** - Update test imports
4. **Performance regression** - Benchmark before/after

### Pure Function Extraction

When splitting, ensure functions remain pure:

```rust
// ✓ Pure (good)
fn is_god_object(metrics: &ComplexityMetrics) -> bool {
    metrics.method_count > 20 && metrics.complexity > 50
}

// ✗ Impure (needs fixing)
fn detect_god_objects_bad(path: &Path) -> Vec<GodObject> {
    let content = std::fs::read_to_string(path)?; // I/O!
    parse_and_detect(&content)
}

// ✓ I/O separated to boundary
// In io module:
fn load_metrics(path: &Path) -> Result<FileMetrics> {
    let content = std::fs::read_to_string(path)?;
    parse_metrics(&content)
}

// In detector module (pure):
fn detect_god_objects(metrics: &FileMetrics) -> Vec<GodObject> {
    // Pure logic only
}
```

## Migration and Compatibility

### Breaking Changes

**None** - This is an internal refactoring. Public API remains identical.

### Migration Steps

1. **No user action required** - Internal change only
2. **Developers** must update imports:
   ```rust
   // Before
   use debtmap::god_object_detector::{detect_god_objects, GodObject};

   // After
   use debtmap::god_object::{detect_god_objects, GodObject};
   ```

### Compatibility Considerations

- Maintain all existing public functions
- Preserve function signatures
- Keep same return types
- No changes to CLI interface

### Rollback Plan

If issues arise:
1. Revert commit
2. Restore original `god_object_detector.rs`
3. Fix issues in new structure
4. Re-attempt split

## Success Metrics

- ✅ 6 modules created, each under 500 lines
- ✅ 100% of original functionality preserved
- ✅ All tests pass
- ✅ No clippy warnings
- ✅ No performance regression (benchmark within 5%)
- ✅ Module-level documentation complete
- ✅ Clear separation of pure/impure functions

## References

- **STILLWATER_EVALUATION.md** - Section "Composition Over Complexity"
- **CLAUDE.md** - Module boundary guidelines
- **Stillwater Philosophy** - Pure core, imperative shell pattern
