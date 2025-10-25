---
number: 119
title: Role-Based Coverage Expectations for Scoring
category: testing
priority: high
status: draft
dependencies: []
created: 2025-10-25
---

# Specification 119: Role-Based Coverage Expectations for Scoring

**Category**: testing
**Priority**: high
**Status**: draft
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
- [ ] Debug/diagnostic role correctly detected for functions matching patterns
- [ ] Coverage output shows: `Coverage: 0% (expected: 20-40% for debug)`
- [ ] Role-specific coverage expectations configurable in `.debtmap.toml`
- [ ] Scoring calculation shows: `Coverage Score: 5.2 (30% below expectation for role)`
- [ ] All existing high-value test recommendations remain (no false negatives)
- [ ] Documentation explains role-based expectations and adjustment rationale
- [ ] Property-based tests verify scoring monotonicity within role categories

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
    fn detect_debug_role(&self, function: &FunctionMetrics) -> bool {
        // Name-based detection
        if self.matches_debug_pattern(&function.name) {
            return true;
        }

        // Behavior-based detection
        let has_diagnostic_characteristics =
            function.has_primarily_io_operations() &&
            function.return_type.is_unit_or_simple() &&
            function.external_calls.len() < 5;

        has_diagnostic_characteristics
    }

    fn matches_debug_pattern(&self, name: &str) -> bool {
        name.starts_with("debug_") ||
        name.starts_with("print_") ||
        name.starts_with("dump_") ||
        name.starts_with("trace_") ||
        name.ends_with("_diagnostics") ||
        name.ends_with("_debug") ||
        name.ends_with("_stats") ||
        name.contains("validate_call_graph") ||
        name.contains("check_consistency")
    }
}
```

**Phase 2: Role-Based Coverage Expectations**

Create `src/priority/scoring/coverage_expectations.rs`:

```rust
pub struct CoverageExpectations {
    expectations: HashMap<FunctionRole, CoverageRange>,
}

#[derive(Debug, Clone)]
pub struct CoverageRange {
    pub min: f64,
    pub target: f64,
    pub max: f64,
}

impl Default for CoverageExpectations {
    fn default() -> Self {
        let mut expectations = HashMap::new();

        expectations.insert(FunctionRole::Pure, CoverageRange {
            min: 85.0, target: 95.0, max: 100.0
        });

        expectations.insert(FunctionRole::BusinessLogic, CoverageRange {
            min: 75.0, target: 85.0, max: 95.0
        });

        expectations.insert(FunctionRole::Orchestrator, CoverageRange {
            min: 50.0, target: 65.0, max: 75.0
        });

        expectations.insert(FunctionRole::EntryPoint, CoverageRange {
            min: 30.0, target: 50.0, max: 60.0
        });

        expectations.insert(FunctionRole::Debug, CoverageRange {
            min: 10.0, target: 30.0, max: 40.0
        });

        expectations.insert(FunctionRole::Accessor, CoverageRange {
            min: 60.0, target: 75.0, max: 85.0
        });

        expectations.insert(FunctionRole::Constructor, CoverageRange {
            min: 60.0, target: 75.0, max: 85.0
        });

        Self { expectations }
    }
}

impl CoverageExpectations {
    /// Calculate coverage gap relative to role expectation
    pub fn calculate_gap(&self, actual: f64, role: FunctionRole) -> CoverageGap {
        let expectation = self.expectations.get(&role)
            .unwrap_or(&CoverageRange { min: 80.0, target: 80.0, max: 95.0 });

        let gap = expectation.target - actual;
        let severity = self.categorize_gap(actual, expectation);

        CoverageGap {
            actual,
            expected: expectation.target,
            gap,
            severity,
            role,
        }
    }

    fn categorize_gap(&self, actual: f64, expectation: &CoverageRange) -> GapSeverity {
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

Modify `src/priority/scoring/mod.rs`:

```rust
pub fn calculate_coverage_score(
    function: &FunctionMetrics,
    coverage_expectations: &CoverageExpectations,
) -> CoverageScore {
    let role = function.role.unwrap_or(FunctionRole::BusinessLogic);
    let coverage_gap = coverage_expectations.calculate_gap(function.coverage, role);

    // Scale score based on gap severity, not absolute coverage
    let base_score = match coverage_gap.severity {
        GapSeverity::None => 0.0,
        GapSeverity::Minor => coverage_gap.gap * 0.5,  // Reduced weight
        GapSeverity::Moderate => coverage_gap.gap * 1.0,
        GapSeverity::Critical => coverage_gap.gap * 1.5,  // Increased weight
    };

    // Apply role-specific multiplier
    let role_multiplier = get_role_coverage_weight(role);
    let weighted_score = base_score * role_multiplier;

    CoverageScore {
        raw_score: base_score,
        weighted_score,
        gap: coverage_gap,
        role,
    }
}

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

**New Modules**:
- `src/priority/scoring/coverage_expectations.rs` - Role-based expectations
- Extend: `src/analysis/role_detection.rs` - Add Debug role detection

**Modified Modules**:
- `src/priority/scoring/mod.rs` - Use role-aware coverage scoring
- `src/io/formatter.rs` - Display expected vs actual coverage
- `src/config.rs` - Add configuration for coverage expectations

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleCoverageConfig {
    /// Coverage expectations per role
    pub pure_function: CoverageRangeConfig,
    pub business_logic: CoverageRangeConfig,
    pub orchestrator: CoverageRangeConfig,
    pub entry_point: CoverageRangeConfig,
    pub debug: CoverageRangeConfig,
    pub accessor: CoverageRangeConfig,
    pub constructor: CoverageRangeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageRangeConfig {
    #[serde(default)]
    pub min: f64,
    #[serde(default)]
    pub target: f64,
    #[serde(default)]
    pub max: f64,
}

impl Default for RoleCoverageConfig {
    fn default() -> Self {
        Self {
            pure_function: CoverageRangeConfig { min: 85.0, target: 95.0, max: 100.0 },
            business_logic: CoverageRangeConfig { min: 75.0, target: 85.0, max: 95.0 },
            orchestrator: CoverageRangeConfig { min: 50.0, target: 65.0, max: 75.0 },
            entry_point: CoverageRangeConfig { min: 30.0, target: 50.0, max: 60.0 },
            debug: CoverageRangeConfig { min: 10.0, target: 30.0, max: 40.0 },
            accessor: CoverageRangeConfig { min: 60.0, target: 75.0, max: 85.0 },
            constructor: CoverageRangeConfig { min: 60.0, target: 75.0, max: 85.0 },
        }
    }
}
```

### Configuration

Add to `.debtmap.toml`:
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

[coverage.role_expectations.entry_point]
min = 30.0
target = 50.0
max = 60.0

[coverage.role_expectations.debug]
min = 10.0
target = 30.0
max = 40.0

# ... other roles
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
```

**Coverage Gap Calculation Tests**:
```rust
#[test]
fn debug_function_with_zero_coverage_not_critical() {
    let expectations = CoverageExpectations::default();
    let gap = expectations.calculate_gap(0.0, FunctionRole::Debug);

    assert_eq!(gap.expected, 30.0);
    assert_eq!(gap.gap, 30.0);
    assert_eq!(gap.severity, GapSeverity::Critical);

    // But weighted score should be low due to role multiplier
    let score = calculate_coverage_score_with_gap(gap);
    assert!(score < 5.0, "Debug function should score low even at 0% coverage");
}

#[test]
fn pure_function_with_zero_coverage_is_critical() {
    let expectations = CoverageExpectations::default();
    let gap = expectations.calculate_gap(0.0, FunctionRole::Pure);

    assert_eq!(gap.expected, 95.0);
    assert_eq!(gap.severity, GapSeverity::Critical);

    let score = calculate_coverage_score_with_gap(gap);
    assert!(score > 15.0, "Pure function should score high at 0% coverage");
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
#[test]
fn maintains_critical_flags_for_untested_business_logic() {
    // Ensure we don't create false negatives
    let test_cases = vec![
        ("calculate_risk", FunctionRole::BusinessLogic, 0.0),
        ("validate_input", FunctionRole::Pure, 10.0),
        ("process_data", FunctionRole::BusinessLogic, 30.0),
    ];

    for (name, role, coverage) in test_cases {
        let score = calculate_test_score(name, role, coverage);
        assert!(score > 12.0, "{} with {}% coverage should be CRITICAL", name, coverage);
    }
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

A debug function with 0% coverage may score LOW instead of CRITICAL,
while a pure business logic function with 50% coverage remains CRITICAL.

**Example Output**:
```
Coverage: 0% (expected: 20-40% for Debug) 🟠 MODERATE
Coverage: 45% (expected: 85% for BusinessLogic) 🔴 CRITICAL
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
3. **Combination** (highest precision, ~92%)

**False Positive Handling**:
- If function has complex logic (complexity >10), not debug even if name matches
- If function has many external calls (>10), likely orchestrator not debug

**False Negative Handling**:
- Conservative detection: Better to flag debug function than miss business logic
- Allow manual role annotation in comments: `// @debtmap-role: debug`

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

**Migration**:
1. Run analysis with `--dry-run` to preview changes
2. Update CI thresholds if needed
3. Document score changes in release notes

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
