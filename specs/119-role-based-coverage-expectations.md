---
number: 119
title: Role-Based Coverage Expectations for Scoring
category: testing
priority: high
status: ready
dependencies: []
created: 2025-10-25
updated: 2025-10-25
---

# Specification 119: Role-Based Coverage Expectations for Scoring

**Category**: testing
**Priority**: high
**Status**: ready
**Dependencies**: None

## Context

Debtmap currently treats all functions equally when scoring coverage gaps, using a uniform 80% coverage target. However, different function roles have fundamentally different testing requirements and coverage expectations.

**Current Problem**:

Item #4 from latest analysis:
```
#4 SCORE: 20.9 [🔴 UNTESTED] [CRITICAL]
├─ LOCATION: src/commands/analyze.rs:396 handle_call_graph_diagnostics()
├─ COVERAGE: 0.0%
└─ ACTION: Add 8 tests for 100% coverage gap
```

**Analysis of this function**:
- It's a debug/diagnostic function that prints call graph statistics
- Used for CLI debugging and troubleshooting
- Often tested manually or through integration tests
- Expecting 100% unit test coverage is unrealistic and low-value
- Yet it scores 20.9 (CRITICAL) primarily due to coverage gap

**Why Current Approach Fails**:

Different function roles have different testing characteristics:

| Role | Expected Coverage | Testing Approach | Example |
|------|------------------|------------------|---------|
| Pure business logic | 85-95% | Unit tests | `calculate_complexity()` |
| Entry points | 40-60% | Integration tests | `main()`, CLI handlers |
| Debug/diagnostic | 20-40% | Manual/integration | `print_debug_info()` |
| I/O orchestration | 50-70% | Integration tests | `read_and_parse_file()` |
| Pure functions | 90-100% | Unit tests | `add(a, b)` |

**Real-World Impact**:
- Diagnostic functions flagged as CRITICAL waste developer time
- Entry points get high scores despite being integration-tested
- Pure functions without tests correctly flagged
- Overall trust in recommendations reduced

## Objective

Implement role-aware coverage expectations that adjust scoring based on function role, reducing false positives while maintaining detection of genuinely untested business logic.

## Requirements

### Functional Requirements

1. **Role Classification**
   - Leverage existing `FunctionRole` enum (already implemented)
   - Roles: `EntryPoint`, `BusinessLogic`, `Orchestrator`, `Accessor`, `Pure`, `Constructor`
   - Add new role: `Debug` for diagnostic/debugging functions
   - Auto-detect debug functions based on naming patterns and usage

2. **Coverage Expectations by Role**
   - **Pure Functions**: 90-100% expected (high unit test coverage)
   - **Business Logic**: 80-95% expected (core functionality)
   - **Orchestrators**: 60-75% expected (coordination logic)
   - **Entry Points**: 40-60% expected (integration tested)
   - **Debug/Diagnostic**: 20-40% expected (manually tested)
   - **Accessors**: 70-85% expected (simple getters/setters)
   - **Constructors**: 70-85% expected (initialization logic)

3. **Adjusted Coverage Scoring**
   - Calculate coverage gap relative to role expectation
   - Scale coverage score based on actual gap, not absolute coverage
   - Apply role-specific weights to coverage factor in overall score
   - Maintain transparency: Show both actual coverage and expected coverage

4. **Debug/Diagnostic Detection**
   - Detect functions with names matching patterns:
     - `debug_*`, `print_*`, `dump_*`, `trace_*`
     - `*_diagnostics`, `*_debug`, `*_stats`
     - `validate_*`, `check_*` (when used for tooling)
   - Analyze function body for diagnostic characteristics:
     - Primarily I/O operations (println, eprintln, write)
     - No return value or returns simple status
     - Few external function calls (mostly formatting)

### Non-Functional Requirements

- Role detection must be accurate (>90% precision)
- Scoring adjustments must be transparent to users
- Performance impact <3% on analysis time
- Configurable coverage expectations per role
- Backward compatible with existing output format

## Acceptance Criteria

- [ ] `handle_call_graph_diagnostics()` score drops from 20.9 to <12 (HIGH, not CRITICAL)
- [ ] Pure business logic functions maintain CRITICAL status when untested
- [ ] Entry points with 40%+ coverage not flagged as CRITICAL for coverage
- [ ] Debug/diagnostic role correctly detected for functions matching patterns (>90% precision on test set)
- [ ] Coverage output shows: `Coverage: 0% (expected: 20-40% for debug)`
- [ ] Role-specific coverage expectations configurable in `.debtmap.toml` with partial override support
- [ ] Scoring calculation shows: `Coverage Score: 5.2 (30% below expectation for role)`
- [ ] All existing high-value test recommendations remain (no false negatives)
- [ ] Regression test: All functions with score >18 pre-change remain >18 post-change (or justified)
- [ ] False negative rate: <2% of business logic functions incorrectly classified as non-critical roles
- [ ] Documentation explains role-based expectations and adjustment rationale
- [ ] Property-based tests verify scoring monotonicity within role categories
- [ ] Performance impact <3% on analysis time (verified with benchmarks)
- [ ] Manual override mechanism (`@debtmap-role` annotation) supported

## Technical Details

### Implementation Approach

**Phase 1: Extend Role Detection**

Modify `src/analysis/role_detection.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionRole {
    EntryPoint,
    BusinessLogic,
    Orchestrator,
    Accessor,
    Pure,
    Constructor,
    Debug,  // New role
}

pub struct RoleDetector {
    config: RoleDetectionConfig,
}

impl RoleDetector {
    /// Detect debug role using functional composition
    fn detect_debug_role(&self, function: &FunctionMetrics) -> bool {
        self.matches_debug_pattern(&function.name) ||
        self.has_diagnostic_characteristics(function)
    }

    /// Check if function name matches debug patterns
    fn matches_debug_pattern(&self, name: &str) -> bool {
        let prefixes = ["debug_", "print_", "dump_", "trace_"];
        let suffixes = ["_diagnostics", "_debug", "_stats"];
        let contains = ["validate_call_graph", "check_consistency"];

        prefixes.iter().any(|p| name.starts_with(p)) ||
        suffixes.iter().any(|s| name.ends_with(s)) ||
        contains.iter().any(|c| name.contains(c))
    }

    /// Check if function has diagnostic behavioral characteristics
    fn has_diagnostic_characteristics(&self, function: &FunctionMetrics) -> bool {
        function.has_primarily_io_operations() &&
        function.return_type.is_unit_or_simple() &&
        function.external_calls.len() < 5 &&
        function.complexity() < 10  // Avoid false positives for complex logic
    }

    /// Parse manual role annotation from function documentation
    /// Supports: `/// @debtmap-role: debug`
    fn parse_manual_role_override(&self, function: &FunctionMetrics) -> Option<FunctionRole> {
        function.doc_comments
            .iter()
            .find_map(|comment| {
                comment.strip_prefix("@debtmap-role:")
                    .map(|s| s.trim())
                    .and_then(|role_str| FunctionRole::from_str(role_str).ok())
            })
    }
}
```

**Phase 2: Role-Based Coverage Expectations**

Create `src/priority/scoring/coverage_expectations.rs`:

```rust
/// Type-safe coverage expectations using struct fields instead of HashMap
/// This ensures compile-time exhaustiveness checking when new roles are added
#[derive(Debug, Clone)]
pub struct CoverageExpectations {
    pub pure_function: CoverageRange,
    pub business_logic: CoverageRange,
    pub orchestrator: CoverageRange,
    pub entry_point: CoverageRange,
    pub debug: CoverageRange,
    pub accessor: CoverageRange,
    pub constructor: CoverageRange,
}

#[derive(Debug, Clone, Copy)]
pub struct CoverageRange {
    pub min: f64,
    pub target: f64,
    pub max: f64,
}

impl Default for CoverageExpectations {
    fn default() -> Self {
        Self {
            pure_function: CoverageRange { min: 85.0, target: 95.0, max: 100.0 },
            business_logic: CoverageRange { min: 75.0, target: 85.0, max: 95.0 },
            orchestrator: CoverageRange { min: 50.0, target: 65.0, max: 75.0 },
            entry_point: CoverageRange { min: 30.0, target: 50.0, max: 60.0 },
            debug: CoverageRange { min: 10.0, target: 30.0, max: 40.0 },
            accessor: CoverageRange { min: 60.0, target: 75.0, max: 85.0 },
            constructor: CoverageRange { min: 60.0, target: 75.0, max: 85.0 },
        }
    }
}

impl CoverageExpectations {
    /// Get coverage expectation for a role with exhaustive matching
    pub fn get_expectation(&self, role: FunctionRole) -> &CoverageRange {
        match role {
            FunctionRole::Pure => &self.pure_function,
            FunctionRole::BusinessLogic => &self.business_logic,
            FunctionRole::Orchestrator => &self.orchestrator,
            FunctionRole::EntryPoint => &self.entry_point,
            FunctionRole::Debug => &self.debug,
            FunctionRole::Accessor => &self.accessor,
            FunctionRole::Constructor => &self.constructor,
        }
    }

    /// Calculate coverage gap using functional pipeline
    pub fn calculate_gap(&self, actual: f64, role: FunctionRole) -> CoverageGap {
        let expectation = self.get_expectation(role);

        CoverageGap {
            actual,
            expected: expectation.target,
            gap: expectation.target - actual,
            severity: categorize_gap_severity(actual, expectation),
            role,
        }
    }

    /// Support partial configuration overrides
    pub fn with_overrides(self, config: &RoleCoverageConfig) -> Self {
        Self {
            pure_function: config.pure_function.unwrap_or(self.pure_function),
            business_logic: config.business_logic.unwrap_or(self.business_logic),
            orchestrator: config.orchestrator.unwrap_or(self.orchestrator),
            entry_point: config.entry_point.unwrap_or(self.entry_point),
            debug: config.debug.unwrap_or(self.debug),
            accessor: config.accessor.unwrap_or(self.accessor),
            constructor: config.constructor.unwrap_or(self.constructor),
        }
    }
}

/// Pure function to categorize gap severity
fn categorize_gap_severity(actual: f64, expectation: &CoverageRange) -> GapSeverity {
    if actual >= expectation.target {
        GapSeverity::None
    } else if actual >= expectation.min {
        GapSeverity::Minor
    } else if actual >= expectation.min * 0.5 {
        GapSeverity::Moderate
    } else {
        GapSeverity::Critical
    }
}

#[derive(Debug, Clone)]
pub struct CoverageGap {
    pub actual: f64,
    pub expected: f64,
    pub gap: f64,
    pub severity: GapSeverity,
    pub role: FunctionRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapSeverity {
    None,
    Minor,
    Moderate,
    Critical,
}
```

**Phase 3: Adjust Coverage Scoring**

Modify `src/priority/scoring/coverage_scoring.rs`:

```rust
use crate::priority::scoring::coverage_expectations::{CoverageExpectations, CoverageGap, GapSeverity};

/// Calculate coverage score using functional pipeline
pub fn calculate_coverage_score(
    function: &FunctionMetrics,
    coverage_expectations: &CoverageExpectations,
) -> CoverageScore {
    let role = function.role.unwrap_or(FunctionRole::BusinessLogic);

    calculate_gap(function.coverage, role, coverage_expectations)
        .pipe(|gap| weight_by_severity(&gap))
        .pipe(|(gap, base)| weight_by_role(gap, base, role))
        .pipe(|(gap, base, weighted)| CoverageScore {
            raw_score: base,
            weighted_score: weighted,
            gap,
            role,
        })
}

/// Pure function: Calculate coverage gap
fn calculate_gap(
    actual: f64,
    role: FunctionRole,
    expectations: &CoverageExpectations,
) -> CoverageGap {
    expectations.calculate_gap(actual, role)
}

/// Pure function: Weight by gap severity
fn weight_by_severity(gap: &CoverageGap) -> (CoverageGap, f64) {
    let base_score = match gap.severity {
        GapSeverity::None => 0.0,
        GapSeverity::Minor => gap.gap * 0.5,     // Reduced weight
        GapSeverity::Moderate => gap.gap * 1.0,
        GapSeverity::Critical => gap.gap * 1.5,  // Increased weight
    };
    (gap.clone(), base_score)
}

/// Pure function: Weight by role
fn weight_by_role(gap: CoverageGap, base_score: f64, role: FunctionRole) -> (CoverageGap, f64, f64) {
    let role_multiplier = get_role_coverage_weight(role);
    let weighted_score = base_score * role_multiplier;
    (gap, base_score, weighted_score)
}

/// Pure function: Get role-specific weight with exhaustive matching
fn get_role_coverage_weight(role: FunctionRole) -> f64 {
    match role {
        FunctionRole::Pure => 2.0,           // Most critical to test
        FunctionRole::BusinessLogic => 1.8,  // Very important
        FunctionRole::Accessor => 1.2,       // Important but simple
        FunctionRole::Constructor => 1.2,    // Important initialization
        FunctionRole::Orchestrator => 1.0,   // Integration tested
        FunctionRole::EntryPoint => 0.6,     // Integration tested
        FunctionRole::Debug => 0.3,          // Manually tested
    }
}

#[derive(Debug, Clone)]
pub struct CoverageScore {
    pub raw_score: f64,
    pub weighted_score: f64,
    pub gap: CoverageGap,
    pub role: FunctionRole,
}
```

**Phase 4: Update Output Formatting**

Modify `src/io/formatter.rs`:

```rust
fn format_coverage_details(&self, score: &CoverageScore) -> String {
    let emoji = match score.gap.severity {
        GapSeverity::None => "🟢",
        GapSeverity::Minor => "🟡",
        GapSeverity::Moderate => "🟠",
        GapSeverity::Critical => "🔴",
    };

    format!(
        "{} {}% (expected: {}% for {:?})",
        emoji,
        score.gap.actual,
        score.gap.expected,
        score.role
    )
}
```

### Architecture Changes

**Module Structure**:
```
src/priority/scoring/
├── mod.rs                      # Re-exports and public API
├── coverage_expectations.rs    # NEW: Role-based expectations
├── coverage_scoring.rs         # MODIFIED: Pure functional scoring pipeline
├── complexity_scoring.rs       # Existing
└── risk_scoring.rs            # Existing

src/analysis/
└── role_detection.rs          # MODIFIED: Add Debug role + manual overrides
```

**New Modules**:
- `src/priority/scoring/coverage_expectations.rs` - Type-safe role expectations
- `src/priority/scoring/coverage_scoring.rs` - Split from mod.rs for clarity

**Modified Modules**:
- `src/priority/scoring/mod.rs` - Re-export new scoring functions
- `src/analysis/role_detection.rs` - Add Debug role detection + manual annotations
- `src/io/formatter.rs` - Display expected vs actual coverage
- `src/config.rs` - Add configuration for coverage expectations with partial overrides

### Data Structures

```rust
/// Configuration structure supporting partial overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleCoverageConfig {
    /// Enable role-based coverage expectations
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Coverage expectations per role (all optional for partial config)
    #[serde(default)]
    pub pure_function: Option<CoverageRange>,
    #[serde(default)]
    pub business_logic: Option<CoverageRange>,
    #[serde(default)]
    pub orchestrator: Option<CoverageRange>,
    #[serde(default)]
    pub entry_point: Option<CoverageRange>,
    #[serde(default)]
    pub debug: Option<CoverageRange>,
    #[serde(default)]
    pub accessor: Option<CoverageRange>,
    #[serde(default)]
    pub constructor: Option<CoverageRange>,
}

fn default_true() -> bool { true }

impl Default for RoleCoverageConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pure_function: None,
            business_logic: None,
            orchestrator: None,
            entry_point: None,
            debug: None,
            accessor: None,
            constructor: None,
        }
    }
}

impl RoleCoverageConfig {
    /// Merge with defaults, only overriding specified values
    pub fn apply_to(&self, defaults: CoverageExpectations) -> CoverageExpectations {
        if !self.enabled {
            return CoverageExpectations::uniform(); // Fallback to uniform expectations
        }

        CoverageExpectations {
            pure_function: self.pure_function.unwrap_or(defaults.pure_function),
            business_logic: self.business_logic.unwrap_or(defaults.business_logic),
            orchestrator: self.orchestrator.unwrap_or(defaults.orchestrator),
            entry_point: self.entry_point.unwrap_or(defaults.entry_point),
            debug: self.debug.unwrap_or(defaults.debug),
            accessor: self.accessor.unwrap_or(defaults.accessor),
            constructor: self.constructor.unwrap_or(defaults.constructor),
        }
    }
}
```

### Configuration

Add to `.debtmap.toml` (supports partial overrides):

```toml
[coverage.role_expectations]
enabled = true

# Only override what you need - others use defaults
[coverage.role_expectations.entry_point]
target = 60.0  # Increase from default 50.0 for API-heavy projects

[coverage.role_expectations.business_logic]
target = 90.0  # Stricter for library projects
```

**Full configuration example** (all defaults shown):
```toml
[coverage.role_expectations]
enabled = true

[coverage.role_expectations.pure_function]
min = 85.0
target = 95.0
max = 100.0

[coverage.role_expectations.business_logic]
min = 75.0
target = 85.0
max = 95.0

[coverage.role_expectations.orchestrator]
min = 50.0
target = 65.0
max = 75.0

[coverage.role_expectations.entry_point]
min = 30.0
target = 50.0
max = 60.0

[coverage.role_expectations.debug]
min = 10.0
target = 30.0
max = 40.0

[coverage.role_expectations.accessor]
min = 60.0
target = 75.0
max = 85.0

[coverage.role_expectations.constructor]
min = 60.0
target = 75.0
max = 85.0
```

## Dependencies

- **Prerequisites**: Existing `FunctionRole` detection (already implemented)
- **Affected Components**:
  - `src/priority/scoring/mod.rs` - Coverage scoring
  - `src/analysis/role_detection.rs` - Role detection
  - `src/io/formatter.rs` - Output formatting
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

**Role Detection Tests**:
```rust
#[cfg(test)]
mod role_detection_tests {
    use super::*;

    #[test]
    fn detects_debug_diagnostics_functions() {
        let function = create_test_function("handle_call_graph_diagnostics");
        let role = detect_role(&function);
        assert_eq!(role, FunctionRole::Debug);
    }

    #[test]
    fn detects_pure_business_logic() {
        let function = create_test_function("calculate_complexity");
        let role = detect_role(&function);
        assert_eq!(role, FunctionRole::BusinessLogic);
    }

    #[test]
    fn respects_manual_role_annotation() {
        let mut function = create_test_function("my_function");
        function.doc_comments = vec!["@debtmap-role: debug".to_string()];
        let role = detect_role(&function);
        assert_eq!(role, FunctionRole::Debug);
    }

    #[test]
    fn complex_debug_named_function_not_debug_role() {
        // High complexity should override debug naming pattern
        let mut function = create_test_function("debug_complex_calculation");
        function.complexity = 15; // High complexity
        let role = detect_role(&function);
        assert_ne!(role, FunctionRole::Debug);
    }
}
```

**Coverage Gap Calculation Tests**:
```rust
#[cfg(test)]
mod coverage_gap_tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn debug_function_with_zero_coverage_not_critical() {
        let expectations = CoverageExpectations::default();
        let gap = expectations.calculate_gap(0.0, FunctionRole::Debug);

        assert_eq!(gap.expected, 30.0);
        assert_eq!(gap.gap, 30.0);
        assert_eq!(gap.severity, GapSeverity::Critical);

        // But weighted score should be low due to role multiplier
        let function = create_test_function_with_role("debug_fn", FunctionRole::Debug, 0.0);
        let score = calculate_coverage_score(&function, &expectations);
        assert!(score.weighted_score < 5.0, "Debug function should score low even at 0% coverage");
    }

    #[test]
    fn pure_function_with_zero_coverage_is_critical() {
        let expectations = CoverageExpectations::default();
        let gap = expectations.calculate_gap(0.0, FunctionRole::Pure);

        assert_eq!(gap.expected, 95.0);
        assert_eq!(gap.severity, GapSeverity::Critical);

        let function = create_test_function_with_role("pure_fn", FunctionRole::Pure, 0.0);
        let score = calculate_coverage_score(&function, &expectations);
        assert!(score.weighted_score > 15.0, "Pure function should score high at 0% coverage");
    }

    #[test]
    fn edge_case_exactly_at_target() {
        let expectations = CoverageExpectations::default();
        let gap = expectations.calculate_gap(85.0, FunctionRole::BusinessLogic);
        assert_eq!(gap.severity, GapSeverity::None);
        assert_eq!(gap.gap, 0.0);
    }

    #[test]
    fn edge_case_hundred_percent_coverage() {
        let expectations = CoverageExpectations::default();
        let gap = expectations.calculate_gap(100.0, FunctionRole::Pure);
        assert_eq!(gap.severity, GapSeverity::None);
    }
}
```

**Role Detection Precision Tests**:
```rust
#[cfg(test)]
mod precision_tests {
    use super::*;
    use std::fs;

    #[test]
    fn role_detection_precision_on_labeled_samples() {
        // Load pre-labeled test samples
        let samples: Vec<LabeledFunction> =
            serde_json::from_str(&fs::read_to_string("tests/fixtures/role_samples.json").unwrap())
                .unwrap();

        let results: Vec<_> = samples.iter()
            .map(|sample| {
                let detected = detect_role(&sample.function);
                (detected == sample.expected_role, sample.expected_role)
            })
            .collect();

        let precision = results.iter().filter(|(correct, _)| *correct).count() as f64
            / results.len() as f64;

        assert!(
            precision > 0.90,
            "Role detection precision: {:.2}% (expected >90%)",
            precision * 100.0
        );

        // Per-role precision
        for role in [FunctionRole::Debug, FunctionRole::Pure, FunctionRole::BusinessLogic] {
            let role_results: Vec<_> = results.iter()
                .filter(|(_, expected)| *expected == role)
                .collect();

            let role_precision = role_results.iter()
                .filter(|(correct, _)| **correct)
                .count() as f64 / role_results.len() as f64;

            println!("{:?} precision: {:.2}%", role, role_precision * 100.0);
        }
    }
}
```

### Integration Tests

```rust
#[test]
fn end_to_end_role_based_scoring() {
    let config = DebtmapConfig::default();
    let analysis = analyze_file("src/commands/analyze.rs", &config);

    let diagnostics_fn = analysis.find_function("handle_call_graph_diagnostics").unwrap();
    assert_eq!(diagnostics_fn.role, FunctionRole::Debug);
    assert!(diagnostics_fn.score < 12.0, "Should not be CRITICAL");

    let business_fn = analysis.find_function("calculate_complexity").unwrap();
    assert_eq!(business_fn.role, FunctionRole::BusinessLogic);
    if business_fn.coverage < 50.0 {
        assert!(business_fn.score > 15.0, "Should be CRITICAL");
    }
}
```

### Property-Based Tests

```rust
proptest! {
    #[test]
    fn higher_coverage_always_better_within_role(
        coverage1 in 0.0f64..100.0,
        coverage2 in 0.0f64..100.0,
        role in prop::sample::select(vec![
            FunctionRole::Pure,
            FunctionRole::BusinessLogic,
            FunctionRole::Debug,
        ])
    ) {
        prop_assume!(coverage1 < coverage2);

        let score1 = calculate_coverage_score(coverage1, role);
        let score2 = calculate_coverage_score(coverage2, role);

        prop_assert!(score1 > score2, "Higher coverage should always yield lower score");
    }
}
```

### Regression Tests

```rust
#[cfg(test)]
mod regression_tests {
    use super::*;

    #[test]
    fn maintains_critical_flags_for_untested_business_logic() {
        // Ensure we don't create false negatives
        let expectations = CoverageExpectations::default();
        let test_cases = vec![
            ("calculate_risk", FunctionRole::BusinessLogic, 0.0),
            ("validate_input", FunctionRole::Pure, 10.0),
            ("process_data", FunctionRole::BusinessLogic, 30.0),
        ];

        for (name, role, coverage) in test_cases {
            let function = create_test_function_with_role(name, role, coverage);
            let score = calculate_coverage_score(&function, &expectations);
            assert!(
                score.weighted_score > 12.0,
                "{} with {}% coverage should be CRITICAL (score: {})",
                name, coverage, score.weighted_score
            );
        }
    }

    #[test]
    fn existing_critical_items_remain_critical() {
        // Regression test: Functions with score >18 pre-change should remain >18 post-change
        // This test should be run against actual codebase with baseline scores
        let expectations = CoverageExpectations::default();

        // Load pre-change baseline (would come from file in real test)
        let baseline_critical_functions = vec![
            ("complex_algorithm", FunctionRole::BusinessLogic, 5.0, 18.5),
            ("risk_calculator", FunctionRole::Pure, 0.0, 20.0),
        ];

        for (name, role, coverage, baseline_score) in baseline_critical_functions {
            let function = create_test_function_with_role(name, role, coverage);
            let new_score = calculate_coverage_score(&function, &expectations);

            assert!(
                new_score.weighted_score >= baseline_score * 0.9, // Allow 10% tolerance
                "{} score dropped too much: {} -> {}",
                name, baseline_score, new_score.weighted_score
            );
        }
    }
}
```

### Performance Benchmarks

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn benchmark_role_detection(c: &mut Criterion) {
        let functions = create_test_codebase(1000); // 1000 functions

        c.bench_function("role_detection_1000_functions", |b| {
            b.iter(|| {
                for function in &functions {
                    black_box(detect_role(function));
                }
            })
        });
    }

    fn benchmark_coverage_scoring(c: &mut Criterion) {
        let functions = create_test_codebase(1000);
        let expectations = CoverageExpectations::default();

        c.bench_function("coverage_scoring_1000_functions", |b| {
            b.iter(|| {
                for function in &functions {
                    black_box(calculate_coverage_score(function, &expectations));
                }
            })
        });
    }

    fn benchmark_parallel_processing(c: &mut Criterion) {
        use rayon::prelude::*;

        let functions = create_test_codebase(10000);
        let expectations = CoverageExpectations::default();

        c.bench_function("parallel_scoring_10k_functions", |b| {
            b.iter(|| {
                functions.par_iter()
                    .map(|f| calculate_coverage_score(f, &expectations))
                    .collect::<Vec<_>>()
            })
        });
    }

    criterion_group!(
        benches,
        benchmark_role_detection,
        benchmark_coverage_scoring,
        benchmark_parallel_processing
    );
    criterion_main!(benches);
}
```

## Documentation Requirements

### User Documentation

**README.md section**:
```markdown
## Coverage Expectations by Function Role

Debtmap adjusts coverage expectations based on function role:

| Role | Expected Coverage | Rationale |
|------|------------------|-----------|
| Pure Functions | 90-100% | Easy to unit test, high value |
| Business Logic | 80-95% | Core functionality, must be tested |
| Orchestrators | 60-75% | Coordination logic, integration tested |
| Entry Points | 40-60% | CLI/API handlers, integration tested |
| Debug/Diagnostic | 20-40% | Tooling code, often manually tested |
| Accessors | 70-85% | Simple getters/setters |
| Constructors | 70-85% | Initialization logic |

A debug function with 0% coverage may score LOW instead of CRITICAL,
while a pure business logic function with 50% coverage remains CRITICAL.

### Manual Role Override

If role detection is incorrect, you can manually specify the role:

```rust
/// Calculate total complexity across all functions
///
/// @debtmap-role: business_logic
fn calculate_total_complexity(functions: &[Function]) -> u32 {
    // ...
}
```

### Customizing Expectations

Override expectations in `.debtmap.toml`:

```toml
[coverage.role_expectations]
enabled = true

# Only specify what you want to change
[coverage.role_expectations.entry_point]
target = 60.0  # Stricter for API-heavy projects
```

**Example Output**:
```
Coverage: 0% (expected: 20-40% for Debug) 🟠 MODERATE
Coverage: 45% (expected: 85% for BusinessLogic) 🔴 CRITICAL
```

### Troubleshooting

**Q: Why is my business logic function classified as Debug?**

A: Check if the function name matches debug patterns (`debug_*`, `print_*`, etc.).
   Use manual annotation `@debtmap-role: business_logic` to override.

**Q: How do I adjust expectations for my project type?**

A: Library projects may want stricter expectations:
   ```toml
   [coverage.role_expectations.business_logic]
   target = 90.0  # Instead of default 85.0
   ```

   Embedded systems may want relaxed expectations:
   ```toml
   [coverage.role_expectations.business_logic]
   target = 75.0  # Instead of default 85.0
   ```
```

### Architecture Documentation

Add to `ARCHITECTURE.md`:
```markdown
## Role-Based Testing Philosophy

### Coverage Expectations

Different function roles have different testing characteristics:

1. **Pure Functions** (target: 95%)
   - Easy to unit test (no dependencies)
   - High ROI on tests
   - Missing tests = high risk

2. **Business Logic** (target: 85%)
   - Core value of application
   - Complex logic requires tests
   - Integration points tested separately

3. **Entry Points** (target: 50%)
   - Primarily integration tested
   - Unit tests less valuable
   - Focus on happy path + major errors

4. **Debug/Diagnostic** (target: 30%)
   - Tooling and troubleshooting code
   - Often manually tested
   - Low test ROI

### Scoring Adjustments

Coverage scoring uses role-specific multipliers:
- Pure: 2.0× (most critical to test)
- BusinessLogic: 1.8×
- EntryPoint: 0.6×
- Debug: 0.3× (least critical to test)
```

## Implementation Notes

### Role Detection Accuracy

Debug role detection uses multiple signals:
1. **Name patterns** (high precision, ~95%)
2. **Behavior analysis** (medium precision, ~80%)
3. **Manual annotations** (100% precision when provided)
4. **Combination** (highest precision, ~92%)

**False Positive Handling**:
- If function has complex logic (complexity >10), not debug even if name matches
- If function has many external calls (>10), likely orchestrator not debug
- Manual annotations always override automatic detection

**False Negative Handling**:
- Conservative detection: Better to flag debug function than miss business logic
- Allow manual role annotation: `/// @debtmap-role: debug`
- Monitor false negative rate in production via metrics

### Performance Optimization

To achieve <3% performance impact:

1. **Lazy Evaluation**: Only calculate expectations for functions being scored
2. **Caching**: Pre-compute role-specific multipliers as constants
3. **Parallel Processing**: Use `rayon` for role detection across functions
   ```rust
   functions.par_iter()
       .map(|f| (f, detect_role(f)))
       .collect()
   ```
4. **Zero-copy**: Pass references throughout the pipeline
5. **Benchmarking**: Use criterion to validate performance targets

### Coverage Expectation Tuning

Projects may need different expectations:
```toml
# Embedded systems: Lower expectations overall
[coverage.role_expectations.business_logic]
target = 75.0  # Instead of 85.0

# Libraries: Higher expectations for public API
[coverage.role_expectations.business_logic]
target = 95.0  # Instead of 85.0
```

### Backward Compatibility

Scoring changes will affect existing baselines:
- Debug functions: Score will drop (good - fewer false positives)
- Pure functions: Score may increase (good - higher standards)
- Entry points: Score will drop (good - already integration tested)

**Migration Strategy**:
1. Feature flag: `role_based_coverage.enabled = false` for gradual rollout
2. Run analysis with `--dry-run` to preview changes
3. Generate score comparison report
4. Update CI thresholds if needed
5. Document score changes in release notes
6. Provide migration guide for users

## Success Metrics

- False positive rate reduced by 20-30% (especially for debug/entry functions)
- `handle_call_graph_diagnostics()` scores <12 (HIGH instead of CRITICAL)
- Zero false negatives on business logic with <60% coverage
- User feedback: More actionable recommendations
- CI failure rate stable or reduced (not more sensitive)

## Future Enhancements

- **Test type detection**: Distinguish unit vs integration test coverage
- **Dynamic role learning**: ML model learns role patterns from codebase
- **Project-specific roles**: Allow custom role definitions
- **Coverage quality**: Not just line coverage, but branch/condition coverage
- **Test quality scoring**: Detect test code smells (mocking everything, brittle tests)
- **Role confidence scores**: Return confidence level with role detection for ambiguous cases
- **Multi-role support**: Handle functions that serve multiple roles

## Revision History

**Revision 1 (2025-10-25)**: Enhanced specification with functional programming patterns

### Key Changes from Original Draft:

#### 1. Functional Programming Alignment
- **Role Detection**: Refactored from imperative style to functional composition
- **Coverage Scoring**: Split into pure functional pipeline (`calculate_gap` → `weight_by_severity` → `weight_by_role`)
- **Pattern Matching**: Use iterator combinators (`any()`, `filter()`) instead of loops

#### 2. Type Safety Improvements
- **CoverageExpectations**: Changed from `HashMap<FunctionRole, CoverageRange>` to struct with explicit fields
- **Exhaustive Matching**: Ensures compile-time checking when new roles are added
- **No Runtime Failures**: Eliminated `unwrap_or()` fallback by using exhaustive match

#### 3. Configuration Enhancements
- **Partial Overrides**: Support optional configuration fields (all fields are `Option<T>`)
- **Default Merging**: `apply_to()` method merges config with defaults
- **Simpler TOML**: Users only specify what they want to change

#### 4. Testing Enhancements
- **Precision Tests**: Added test for >90% role detection precision with labeled samples
- **Edge Case Tests**: Tests for boundary values (0%, 100%, exactly at target)
- **Regression Tests**: Verify existing CRITICAL items remain CRITICAL
- **Performance Benchmarks**: Criterion benchmarks to validate <3% impact

#### 5. Feature Additions
- **Manual Role Override**: Support `@debtmap-role` annotation in doc comments
- **Complexity Guard**: Prevent high-complexity functions from being classified as Debug
- **Performance Strategy**: Detailed plan for achieving <3% performance impact using parallel processing

#### 6. Documentation Improvements
- **Troubleshooting Section**: Q&A for common role detection issues
- **Migration Guide**: Step-by-step process for adopting role-based scoring
- **Module Structure**: Clear diagram of module organization
- **Configuration Examples**: Both minimal and full configuration examples

#### 7. Acceptance Criteria Strengthening
- Added: "Debug role detection >90% precision on test set"
- Added: "Regression test: Functions with score >18 remain >18"
- Added: "False negative rate: <2%"
- Added: "Performance verified with benchmarks"
- Added: "Manual override mechanism supported"

### Rationale

These changes align the specification with debtmap's functional-first architecture:
- Pure functions are easier to test and compose
- Type safety prevents runtime errors and ensures exhaustiveness
- Performance considerations are explicit and measurable
- Configuration is more user-friendly with partial overrides
- Testing strategy is comprehensive with measurable quality gates
