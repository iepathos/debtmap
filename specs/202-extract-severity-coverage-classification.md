---
number: 202
title: Extract Severity and Coverage Classification
category: refactor
priority: critical
status: draft
dependencies: []
created: 2025-12-02
---

# Specification 202: Extract Severity and Coverage Classification

**Category**: refactor
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap's output formatting system has **three independent implementations** of severity classification with **inconsistent thresholds**, causing the same score to be classified differently across output formats (e.g., score 8.5 is "CRITICAL" in terminal but "HIGH" in markdown). Coverage classification logic is duplicated across **4+ locations** with inconsistent labels and thresholds.

### Current Problems

1. **Severity Inconsistency**:
   - `src/priority/formatter.rs:1921`: thresholds 8.0/6.0/4.0 → CRITICAL/HIGH/MEDIUM/LOW
   - `src/priority/formatter_markdown.rs:1069`: thresholds 9.0/7.0/5.0/3.0 → CRITICAL/HIGH/MEDIUM/LOW/MINIMAL
   - Different thresholds = inconsistent priorities across formats

2. **Coverage Duplication**:
   - `src/priority/formatter_verbosity.rs:10`: `classify_coverage_percentage()`
   - `src/priority/formatter_verbosity.rs:27`: `format_coverage_status()`
   - `src/priority/formatter_verbosity.rs:39`: `format_coverage_factor_description()`
   - `src/priority/formatter_markdown.rs:1276`: Inline coverage matching
   - Different labels: "[WARN LOW]" vs "Low coverage"

### Stillwater Violation

Current code violates **Pure Core, Imperative Shell**:
- Classification logic mixed with formatting logic
- No single source of truth
- Hard to test (mixed with I/O)
- Duplicated across multiple files

## Objective

Extract severity and coverage classification into **pure, testable, composable modules** that serve as the single source of truth for all output formats, following Stillwater's "Pure Core, Imperative Shell" principle.

## Requirements

### Functional Requirements

1. **Single Severity Classification**
   - One `Severity` enum with `from_score()` pure function
   - Consistent thresholds across all formats
   - Thresholds: 8.0 (Critical), 6.0 (High), 4.0 (Medium), <4.0 (Low)
   - Pure function: `score: f64 → Severity`

2. **Single Coverage Classification**
   - One `CoverageLevel` enum with `from_percentage()` pure function
   - Consistent thresholds: 0.0 (Untested), 20.0 (Low), 50.0 (Partial), 80.0 (Moderate), 95.0 (Good), 100.0 (Excellent)
   - Pure function: `percentage: f64 → CoverageLevel`

3. **Format-Specific Display Methods**
   - `Severity::as_str()` → static label
   - `Severity::color()` → Color for terminal
   - `CoverageLevel::status_tag()` → "[WARN LOW]", "[OK GOOD]", etc.
   - `CoverageLevel::description()` → "Low coverage", "Good coverage", etc.

4. **Backward Compatibility**
   - All existing output formats must produce identical results
   - No breaking changes to public APIs
   - Migrate incrementally with deprecated wrappers

### Non-Functional Requirements

1. **Pure Functions**: All classification logic must be pure (no I/O, no side effects)
2. **100% Test Coverage**: Every classification function must have comprehensive tests
3. **Performance**: Classification must be zero-cost (compile-time optimization)
4. **Maintainability**: Single location for threshold changes

## Acceptance Criteria

- [ ] `src/priority/classification/severity.rs` exists with pure `Severity` enum
- [ ] `src/priority/classification/coverage.rs` exists with pure `CoverageLevel` enum
- [ ] `Severity::from_score()` uses thresholds 8.0/6.0/4.0
- [ ] `CoverageLevel::from_percentage()` uses thresholds 0.0/20.0/50.0/80.0/95.0
- [ ] All tests pass with no output changes (backward compatible)
- [ ] `formatter.rs` uses new severity classification (old implementation removed)
- [ ] `formatter_markdown.rs` uses new severity classification (old implementation removed)
- [ ] `formatter_verbosity.rs` uses new coverage classification (4+ old implementations removed)
- [ ] 100% test coverage on classification logic
- [ ] Property-based tests verify monotonicity (higher score → same or higher severity)
- [ ] Documentation updated with new module structure

## Technical Details

### Implementation Approach

**Stage 1: Create Pure Classification Modules**

```rust
// src/priority/classification/mod.rs
pub mod severity;
pub mod coverage;

// src/priority/classification/severity.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    /// Pure function: score → severity
    pub const fn from_score(score: f64) -> Self {
        if score >= 8.0 {
            Self::Critical
        } else if score >= 6.0 {
            Self::High
        } else if score >= 4.0 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }

    pub const fn color(self) -> Color {
        match self {
            Self::Critical => Color::BrightRed,
            Self::High => Color::Red,
            Self::Medium => Color::Yellow,
            Self::Low => Color::White,
        }
    }
}

// src/priority/classification/coverage.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoverageLevel {
    Untested,
    Low,
    Partial,
    Moderate,
    Good,
    Excellent,
}

impl CoverageLevel {
    /// Pure function: percentage → level
    pub const fn from_percentage(pct: f64) -> Self {
        if pct == 0.0 {
            Self::Untested
        } else if pct < 20.0 {
            Self::Low
        } else if pct < 50.0 {
            Self::Partial
        } else if pct < 80.0 {
            Self::Moderate
        } else if pct < 95.0 {
            Self::Good
        } else {
            Self::Excellent
        }
    }

    pub const fn status_tag(self) -> &'static str {
        match self {
            Self::Untested => "[UNTESTED]",
            Self::Low => "[WARN LOW]",
            Self::Partial => "[WARN PARTIAL]",
            Self::Moderate => "[INFO MODERATE]",
            Self::Good => "[OK GOOD]",
            Self::Excellent => "[OK EXCELLENT]",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Untested => "No test coverage",
            Self::Low => "Low coverage",
            Self::Partial => "Partial coverage",
            Self::Moderate => "Moderate coverage",
            Self::Good => "Good coverage",
            Self::Excellent => "Excellent coverage",
        }
    }
}
```

**Stage 2: Migrate formatter.rs**

1. Add `use crate::priority::classification::severity::Severity;`
2. Replace `get_severity_label()` calls with `Severity::from_score(score).as_str()`
3. Mark old `get_severity_label()` as `#[deprecated]`
4. Run tests to verify output unchanged

**Stage 3: Migrate formatter_markdown.rs**

1. Remove duplicate `get_severity_label()` function
2. Use `Severity::from_score()` from classification module
3. Update thresholds to match (9.0 → 8.0, etc.)
4. Verify markdown output unchanged

**Stage 4: Migrate Coverage Classification**

1. Replace all inline coverage matching with `CoverageLevel::from_percentage()`
2. Remove `classify_coverage_percentage()`, `format_coverage_status()`, etc.
3. Update all coverage display logic to use enum methods
4. Verify all formats produce identical output

**Stage 5: Remove Deprecated Code**

1. Delete old `get_severity_label()` functions
2. Delete old coverage classification functions
3. Update all imports
4. Final test verification

### Architecture Changes

**Before**:
```
src/priority/
├── formatter.rs (contains get_severity_label)
└── formatter_markdown.rs (contains duplicate get_severity_label)
```

**After**:
```
src/priority/
├── classification/
│   ├── mod.rs
│   ├── severity.rs (single source of truth)
│   └── coverage.rs (single source of truth)
├── formatter.rs (uses classification::severity)
└── formatter_markdown.rs (uses classification::severity)
```

### Data Structures

```rust
pub enum Severity {
    Low,      // score < 4.0
    Medium,   // score >= 4.0 && < 6.0
    High,     // score >= 6.0 && < 8.0
    Critical, // score >= 8.0
}

pub enum CoverageLevel {
    Untested,   // 0.0%
    Low,        // < 20%
    Partial,    // >= 20% && < 50%
    Moderate,   // >= 50% && < 80%
    Good,       // >= 80% && < 95%
    Excellent,  // >= 95%
}
```

### APIs and Interfaces

**Public API (zero breaking changes)**:

```rust
// Old API (deprecated but still works)
#[deprecated(since = "0.8.0", note = "Use Severity::from_score() instead")]
pub fn get_severity_label(score: f64) -> &'static str {
    Severity::from_score(score).as_str()
}

// New API (recommended)
use debtmap::priority::classification::Severity;
let severity = Severity::from_score(8.5);
println!("{}", severity.as_str()); // "CRITICAL"
```

## Dependencies

- **Prerequisites**: None (standalone refactoring)
- **Affected Components**:
  - `src/priority/formatter.rs` (uses severity)
  - `src/priority/formatter_markdown.rs` (uses severity)
  - `src/priority/formatter_verbosity.rs` (uses coverage)
- **External Dependencies**: None (uses std only)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_thresholds() {
        assert_eq!(Severity::from_score(10.0), Severity::Critical);
        assert_eq!(Severity::from_score(8.0), Severity::Critical);
        assert_eq!(Severity::from_score(7.9), Severity::High);
        assert_eq!(Severity::from_score(6.0), Severity::High);
        assert_eq!(Severity::from_score(5.9), Severity::Medium);
        assert_eq!(Severity::from_score(4.0), Severity::Medium);
        assert_eq!(Severity::from_score(3.9), Severity::Low);
        assert_eq!(Severity::from_score(0.0), Severity::Low);
    }

    #[test]
    fn coverage_thresholds() {
        assert_eq!(CoverageLevel::from_percentage(0.0), CoverageLevel::Untested);
        assert_eq!(CoverageLevel::from_percentage(10.0), CoverageLevel::Low);
        assert_eq!(CoverageLevel::from_percentage(30.0), CoverageLevel::Partial);
        assert_eq!(CoverageLevel::from_percentage(60.0), CoverageLevel::Moderate);
        assert_eq!(CoverageLevel::from_percentage(85.0), CoverageLevel::Good);
        assert_eq!(CoverageLevel::from_percentage(100.0), CoverageLevel::Excellent);
    }

    #[test]
    fn severity_labels() {
        assert_eq!(Severity::Critical.as_str(), "CRITICAL");
        assert_eq!(Severity::High.as_str(), "HIGH");
        assert_eq!(Severity::Medium.as_str(), "MEDIUM");
        assert_eq!(Severity::Low.as_str(), "LOW");
    }

    #[test]
    fn coverage_labels() {
        assert_eq!(CoverageLevel::Untested.status_tag(), "[UNTESTED]");
        assert_eq!(CoverageLevel::Low.status_tag(), "[WARN LOW]");
        assert_eq!(CoverageLevel::Good.status_tag(), "[OK GOOD]");
    }
}
```

### Property-Based Tests

```rust
use quickcheck::quickcheck;

quickcheck! {
    fn severity_is_monotonic(score1: f64, delta: f64) -> bool {
        if !score1.is_finite() || !delta.is_finite() || delta < 0.0 {
            return true; // Skip invalid inputs
        }
        let score2 = score1 + delta;
        let sev1 = Severity::from_score(score1);
        let sev2 = Severity::from_score(score2);
        sev2 >= sev1  // Higher score → same or higher severity
    }

    fn coverage_is_monotonic(pct1: f64, delta: f64) -> bool {
        if pct1 < 0.0 || pct1 > 100.0 || delta < 0.0 {
            return true;
        }
        let pct2 = (pct1 + delta).min(100.0);
        let level1 = CoverageLevel::from_percentage(pct1);
        let level2 = CoverageLevel::from_percentage(pct2);
        level2 >= level1
    }
}
```

### Integration Tests

```rust
#[test]
fn formatter_uses_consistent_severity() {
    let analysis = create_test_analysis();

    let terminal = format_priorities_terminal(&analysis);
    let markdown = format_priorities_markdown(&analysis);

    // Both should classify same score identically
    assert!(terminal.contains("[CRITICAL]"));
    assert!(markdown.contains("[CRITICAL]"));
}
```

### Regression Tests

```rust
#[test]
fn output_unchanged_after_refactor() {
    let analysis = load_test_analysis();

    // Capture output before refactor
    let expected_terminal = include_str!("../test_data/expected_terminal.txt");
    let expected_markdown = include_str!("../test_data/expected_markdown.md");

    // Generate output after refactor
    let actual_terminal = format_priorities_terminal(&analysis);
    let actual_markdown = format_priorities_markdown(&analysis);

    assert_eq!(actual_terminal, expected_terminal);
    assert_eq!(actual_markdown, expected_markdown);
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Severity classification for technical debt items.
///
/// Classifies debt scores into four levels based on priority thresholds:
/// - **Critical** (≥8.0): Immediate action required
/// - **High** (≥6.0): High priority, address soon
/// - **Medium** (≥4.0): Moderate priority
/// - **Low** (<4.0): Low priority
///
/// # Examples
///
/// ```
/// use debtmap::priority::classification::Severity;
///
/// let sev = Severity::from_score(8.5);
/// assert_eq!(sev, Severity::Critical);
/// assert_eq!(sev.as_str(), "CRITICAL");
/// ```
pub enum Severity { ... }
```

### User Documentation

Update README.md:

```markdown
## Priority Classification

Debtmap classifies technical debt using consistent severity levels:

| Severity | Score Range | Description |
|----------|-------------|-------------|
| CRITICAL | ≥ 8.0 | Immediate action required |
| HIGH | 6.0 - 7.9 | High priority, address soon |
| MEDIUM | 4.0 - 5.9 | Moderate priority |
| LOW | < 4.0 | Low priority |
```

### Architecture Updates

Update ARCHITECTURE.md:

```markdown
## Priority Classification

The `priority::classification` module provides pure classification logic:

- `severity.rs`: Severity enum with score → severity mapping
- `coverage.rs`: CoverageLevel enum with percentage → level mapping

All output formatters use these modules as single source of truth.
```

## Implementation Notes

### Key Principles

1. **Pure Functions Only**: No I/O, no side effects, no mutable state
2. **Single Source of Truth**: One threshold definition, used everywhere
3. **Backward Compatible**: Deprecated wrappers preserve old API
4. **Test Before Refactor**: Capture current output as regression tests
5. **Incremental Migration**: Migrate one formatter at a time

### Common Pitfalls

- **Don't** change thresholds during migration (breaks backward compatibility)
- **Don't** mix pure classification with formatting in one function
- **Do** verify output is identical after each migration step
- **Do** run full test suite after each change

### Performance Considerations

- All functions are `const` where possible (compile-time evaluation)
- Enums are Copy (no heap allocation)
- Match expressions compile to jump tables (O(1) lookup)
- Zero runtime cost compared to current implementation

## Migration and Compatibility

### Breaking Changes

None. This is a pure refactoring with backward-compatible deprecated wrappers.

### Migration Path

1. **v0.8.0**: Introduce classification module with deprecated wrappers
2. **v0.8.1**: Update all internal code to use new API
3. **v0.9.0**: Remove deprecated functions

### Rollback Strategy

If issues are discovered:
1. Revert to previous implementation (old functions still exist)
2. Fix issues in classification module
3. Re-run migration

## Success Metrics

- **Duplication Reduction**: Remove 3 severity implementations → 1
- **Coverage Reduction**: Remove 4+ coverage implementations → 1
- **Test Coverage**: Achieve 100% coverage on classification logic
- **Consistency**: Same score produces identical classification in all formats
- **Maintainability**: Single location for threshold changes

## Timeline

- **Day 1**: Create classification module with tests (4h)
- **Day 2**: Migrate formatter.rs (3h)
- **Day 3**: Migrate formatter_markdown.rs (3h)
- **Day 4**: Migrate coverage classification (4h)
- **Day 5**: Remove deprecated code, final testing (2h)

**Total Effort**: 16 hours (2 person-days)
