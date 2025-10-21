---
number: 118
title: Estimated Branches Terminology Clarification
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-10-21
---

# Specification 118: Estimated Branches Terminology Clarification

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap v0.2.9 reports "branches=N" in complexity output, but this value is a **heuristic estimate**, not an actual branch count from AST analysis. This misleads users into thinking debtmap has precisely counted conditional branches.

**Current Output**:
```
COMPLEXITY: cyclomatic=1 (adj:0), branches=1, cognitive=0, nesting=0
```

**The Problem**:
```rust
// src/priority/scoring/recommendation.rs:132
let branches = func.nesting.max(1) * cyclomatic / 3;  // ESTIMATE, not measurement!
```

**For** `ContextMatcher::any()`:
- Cyclomatic = 1 (correct)
- Nesting = 0
- **Calculated "branches"** = `max(0, 1) × 1 ÷ 3 = 0.33` → rounds to **1**
- **Actual branches** = **0** (no if/match/loop statements)

**User Impact**:
- Users assume "branches=1" means debtmap detected 1 conditional statement
- Creates confusion when examining source code (no branches found)
- Undermines trust in debtmap's analysis accuracy
- Makes it unclear which metrics are measured vs estimated

## Objective

Clearly distinguish estimated metrics from measured metrics in debtmap output to improve transparency and user trust.

## Requirements

### Functional Requirements

**FR1: Rename "branches" to "est_branches"**
- Update all output formatting to use `est_branches=` prefix
- Maintain backward compatibility in JSON output (optional field)
- Add tooltip/help text explaining this is an estimate

**FR2: Add Documentation Comment**
- Document the estimation formula in code
- Explain why this estimate is useful
- Reference research basis if available

**FR3: Distinguish Measured vs Estimated in Output**
- Measured metrics: `cyclomatic`, `cognitive`, `nesting`
- Estimated metrics: `est_branches`, `est_test_cases`
- Clear visual separation (optional: use `~` prefix for estimates)

**FR4: Update Help Text**
- Add `--explain-metrics` flag showing metric definitions
- Include estimation formulas in user documentation
- Provide examples of measured vs estimated values

### Non-Functional Requirements

**NFR1: Backward Compatibility**
- JSON output includes both `branches` (deprecated) and `est_branches`
- Command-line output uses only `est_branches`
- Deprecation warning if old JSON field is accessed

**NFR2: Clarity**
- Users can immediately identify estimated vs measured metrics
- No ambiguity in output format
- Consistent terminology across all output formats

**NFR3: Performance**
- No performance impact (purely cosmetic/formatting change)
- Zero overhead in computation

## Acceptance Criteria

- [x] All output formatting uses `est_branches=` instead of `branches=`
- [x] Inline code comments document estimation formula
- [x] User documentation explains measured vs estimated metrics
- [x] `--explain-metrics` flag shows all metric definitions and formulas
- [x] JSON output includes deprecation notice for `branches` field
- [x] No user confusion in issues/discussions about "branches" metric
- [x] Test suite updated with new field names
- [x] Migration guide provided for users parsing JSON output

## Technical Details

### Implementation Approach

**Phase 1: Rename in Output Formatters**

**File**: `src/priority/formatter_verbosity.rs:770`
```rust
// OLD:
format!(
    "{} cyclomatic={}, branches={}, cognitive={}, nesting={}",
    label, cyclomatic, branches, cognitive, nesting
)

// NEW:
format!(
    "{} cyclomatic={}, est_branches={}, cognitive={}, nesting={}",
    label, cyclomatic, branches, cognitive, nesting
)
```

**File**: `src/priority/formatter_verbosity.rs:756`
```rust
// OLD:
format!(
    "{} {} cyclomatic={} (adj:{}), branches={}, cognitive={}, nesting={}, entropy={:.2}",
    emoji, label, cyclomatic, adjustment, branches, cognitive, nesting, entropy
)

// NEW:
format!(
    "{} {} cyclomatic={} (adj:{}), est_branches={}, cognitive={}, nesting={}, entropy={:.2}",
    emoji, label, cyclomatic, adjustment, branches, cognitive, nesting, entropy
)
```

**File**: `src/priority/formatter.rs:1476`
```rust
// Similar changes in non-verbose formatter
```

**Phase 2: Add Documentation**

**File**: `src/priority/scoring/recommendation.rs:132`
```rust
// OLD:
let branches = func.nesting.max(1) * cyclomatic / 3;

// NEW:
/// Estimate branch count from complexity and nesting
///
/// Formula: max(nesting, 1) × cyclomatic ÷ 3
///
/// This is a HEURISTIC approximation, not a precise count of
/// conditional branches. Use cyclomatic complexity for accurate
/// decision point counting.
///
/// Example: cyclo=12, nesting=2 → est_branches = 2 × 12 ÷ 3 = 8
let est_branches = func.nesting.max(1) * cyclomatic / 3;
```

**Phase 3: Update Variable Names**

Rename internal variable from `branches` to `est_branches` throughout:
- `src/priority/scoring/recommendation.rs`
- `src/priority/formatter.rs`
- `src/priority/formatter_verbosity.rs`
- `src/risk/insights.rs`

**Phase 4: JSON Backward Compatibility**

```rust
#[derive(Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,

    #[serde(rename = "est_branches")]
    pub estimated_branches: u32,

    // Deprecated field for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated(note = "Use est_branches instead")]
    pub branches: Option<u32>,
}

impl ComplexityMetrics {
    pub fn new(cyclomatic: u32, cognitive: u32, nesting: u32) -> Self {
        let est_branches = nesting.max(1) * cyclomatic / 3;
        Self {
            cyclomatic,
            cognitive,
            nesting,
            estimated_branches: est_branches,
            branches: Some(est_branches), // For backward compat
        }
    }
}
```

**Phase 5: Add --explain-metrics Flag**

```rust
// src/main.rs or src/cli.rs
#[derive(Parser)]
pub struct Cli {
    #[arg(long, help = "Show detailed metric definitions and formulas")]
    explain_metrics: bool,
}

fn explain_metrics() {
    println!("Debtmap Metrics Explained\n");

    println!("MEASURED METRICS (from AST analysis):");
    println!("  cyclomatic    - Number of independent execution paths");
    println!("  cognitive     - Weighted complexity based on nesting and control flow");
    println!("  nesting       - Maximum nesting depth of control structures");
    println!("  entropy       - Information entropy of variable names\n");

    println!("ESTIMATED METRICS (calculated heuristics):");
    println!("  est_branches  - Approximate branch count");
    println!("                  Formula: max(nesting, 1) × cyclomatic ÷ 3");
    println!("                  Use cyclomatic for precise decision point count\n");

    println!("RISK SCORES (derived from multiple factors):");
    println!("  coverage_gap  - Percentage of function not tested");
    println!("  risk_score    - Combined complexity, coverage, and dependency score\n");

    println!("For more details, see: https://docs.debtmap.io/metrics");
}
```

### Architecture Changes

**Modified Files**:
- `src/priority/formatter_verbosity.rs` - Update formatting strings
- `src/priority/formatter.rs` - Update formatting strings
- `src/priority/scoring/recommendation.rs` - Add documentation, rename variable
- `src/risk/insights.rs` - Update metric references
- `src/main.rs` or `src/cli.rs` - Add `--explain-metrics` flag

**New Files**: None

**Data Structure Changes**: Optional (JSON backward compat)

### APIs and Interfaces

**Command-Line Interface**:
```bash
# New flag
debtmap analyze --explain-metrics

# Output format change
# OLD: "cyclomatic=5, branches=3, cognitive=8"
# NEW: "cyclomatic=5, est_branches=3, cognitive=8"
```

**JSON Output** (backward compatible):
```json
{
  "complexity": {
    "cyclomatic": 5,
    "cognitive": 8,
    "nesting": 2,
    "est_branches": 3,
    "branches": 3  // Deprecated, will be removed in v1.0
  }
}
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- All output formatters (verbose, normal, JSON)
- User documentation
- Integration tests parsing output

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**Test Renaming** (search and replace in tests):
```rust
// OLD:
assert!(output.contains("branches=3"));

// NEW:
assert!(output.contains("est_branches=3"));
```

**Test Documentation**:
```rust
#[test]
fn test_estimated_branches_calculation() {
    let func = create_test_metrics("example", 12, 15, 50);
    func.nesting = 2;

    let est_branches = calculate_est_branches(func.nesting, func.cyclomatic);

    // Formula: max(2, 1) × 12 ÷ 3 = 2 × 12 ÷ 3 = 8
    assert_eq!(est_branches, 8);
}

#[test]
fn test_zero_nesting_uses_min_value() {
    let func = create_test_metrics("simple", 3, 2, 10);
    func.nesting = 0;

    let est_branches = calculate_est_branches(func.nesting, func.cyclomatic);

    // Formula: max(0, 1) × 3 ÷ 3 = 1
    assert_eq!(est_branches, 1);
}
```

### Integration Tests

**Output Format Test**:
```rust
#[test]
fn test_output_uses_est_branches_terminology() {
    let output = run_debtmap_analyze("tests/fixtures/simple.rs");

    assert!(output.contains("est_branches="));
    assert!(!output.contains("branches="), "Should not use ambiguous 'branches' term");
}

#[test]
fn test_explain_metrics_flag() {
    let output = run_command(&["debtmap", "analyze", "--explain-metrics"]);

    assert!(output.contains("MEASURED METRICS"));
    assert!(output.contains("ESTIMATED METRICS"));
    assert!(output.contains("est_branches"));
    assert!(output.contains("Formula:"));
}
```

**JSON Backward Compatibility Test**:
```rust
#[test]
fn test_json_includes_both_branches_fields() {
    let json_output = run_debtmap_json("tests/fixtures/simple.rs");
    let data: Value = serde_json::from_str(&json_output).unwrap();

    // New field is present
    assert!(data["complexity"]["est_branches"].is_number());

    // Deprecated field still present for compatibility
    assert!(data["complexity"]["branches"].is_number());

    // Values should match
    assert_eq!(
        data["complexity"]["est_branches"],
        data["complexity"]["branches"]
    );
}
```

### User Acceptance Testing

**Scenario 1**: User runs analysis and sees clear metric distinction
```bash
$ debtmap analyze src/
COMPLEXITY: cyclomatic=5, est_branches=2, cognitive=8, nesting=2
```

**Scenario 2**: User wants to understand metrics
```bash
$ debtmap analyze --explain-metrics
ESTIMATED METRICS (calculated heuristics):
  est_branches - Approximate branch count
                Formula: max(nesting, 1) × cyclomatic ÷ 3
```

**Scenario 3**: JSON parser reads both old and new fields
```python
# Works with new field
est_branches = data['complexity']['est_branches']

# Still works with old field (deprecated)
branches = data['complexity'].get('branches', est_branches)
```

## Documentation Requirements

### Code Documentation

**Inline Comments**:
```rust
/// Calculate estimated branch count from nesting and cyclomatic complexity.
///
/// This is a heuristic approximation useful for quick estimation,
/// NOT a precise count of conditional branches from AST analysis.
///
/// # Formula
///
/// `est_branches = max(nesting, 1) × cyclomatic ÷ 3`
///
/// # Why This Formula?
///
/// - Nesting correlates with conditional structures (if/match/loops)
/// - Cyclomatic complexity counts decision points
/// - Division by 3 accounts for average branch factor
///
/// # Limitations
///
/// - Overestimates for deeply nested simple functions
/// - Underestimates for flat complex functions
/// - Use `cyclomatic` for accurate decision point counting
///
/// # Examples
///
/// ```
/// // Function with cyclo=12, nesting=3
/// est_branches = max(3, 1) × 12 ÷ 3 = 12 branches
///
/// // Function with cyclo=1, nesting=0 (constructor)
/// est_branches = max(0, 1) × 1 ÷ 3 = 0 branches
/// ```
pub fn calculate_est_branches(nesting: u32, cyclomatic: u32) -> u32 {
    nesting.max(1) * cyclomatic / 3
}
```

### User Documentation

**Update**: `book/src/metrics-reference.md`

```markdown
## Complexity Metrics

### Measured Metrics

These metrics are **precisely extracted from AST analysis**:

- **`cyclomatic`** - Number of independent execution paths through a function
  - Based on control flow graph analysis
  - Formula: Edges - Nodes + 2 × Connected Components
  - Example: 3 if-statements → cyclomatic = 4

- **`cognitive`** - Weighted complexity based on nesting and control flow
  - Penalizes deeply nested structures more than flat complexity
  - Research: "Cognitive Complexity" by G. Ann Campbell

- **`nesting`** - Maximum depth of nested control structures
  - Counts nested if/match/loop/try blocks
  - Higher nesting = harder to understand

### Estimated Metrics

These metrics are **calculated heuristics**, not direct measurements:

- **`est_branches`** - Approximate number of conditional branches
  - Formula: `max(nesting, 1) × cyclomatic ÷ 3`
  - **Use Case**: Quick estimation for testing scope
  - **Limitation**: Not a precise count - use `cyclomatic` for accuracy
  - Example: cyclo=12, nesting=2 → est_branches = 8

### When to Use Each Metric

| Metric | Use For | Don't Use For |
|--------|---------|---------------|
| `cyclomatic` | Test case planning, code review thresholds | Readability assessment |
| `cognitive` | Readability and maintainability assessment | Test coverage calculation |
| `nesting` | Refactoring priority | Execution path counting |
| `est_branches` | Quick testing scope estimation | Precise branch coverage planning |
```

**Update**: `book/src/faq.md`

```markdown
## FAQ

### Why does debtmap show "est_branches" instead of "branches"?

The `est_branches` metric is a **heuristic estimate**, not a precise count of
conditional branches. We renamed it from `branches` to make this clear.

**Old output** (ambiguous):
```
COMPLEXITY: cyclomatic=5, branches=2, cognitive=8
```

**New output** (clear):
```
COMPLEXITY: cyclomatic=5, est_branches=2, cognitive=8
```

For precise decision point counting, use the `cyclomatic` metric which is
measured directly from AST analysis.

### How is `est_branches` calculated?

Formula: `max(nesting, 1) × cyclomatic ÷ 3`

This provides a rough approximation but is not a substitute for actual
branch counting. It's useful for quick testing scope estimation.
```

### Architecture Updates

**Update**: `ARCHITECTURE.md`

Add to "Metrics Calculation" section:
```markdown
### Metric Categories (Spec 118)

Debtmap distinguishes between two categories of metrics:

#### Measured Metrics
Extracted directly from AST analysis:
- Cyclomatic complexity (control flow graph)
- Cognitive complexity (weighted nesting)
- Nesting depth (maximum structure depth)

#### Estimated Metrics
Calculated heuristics for convenience:
- `est_branches` = max(nesting, 1) × cyclomatic ÷ 3

Estimated metrics are clearly labeled with `est_` prefix to avoid confusion.
```

## Implementation Notes

### Rollout Strategy

1. **Version N**: Add `est_branches` alongside `branches` (both present)
2. **Version N+1**: Deprecation warning when parsing old JSON field
3. **Version N+2**: Remove `branches` field (breaking change)

### Communication Plan

**Changelog Entry**:
```markdown
### Changed
- Renamed `branches` metric to `est_branches` to clarify it's an estimate
- Added `--explain-metrics` flag for metric definitions
- Updated documentation to distinguish measured vs estimated metrics

### Deprecated
- JSON field `branches` is deprecated, use `est_branches` instead
- Old field will be removed in v1.0.0

### Migration
- CLI users: No action needed (output format updated automatically)
- JSON parsers: Update to read `est_branches` field instead of `branches`
```

**Blog Post**:
```
Title: "Making Debtmap Metrics More Transparent"

We're renaming the `branches` metric to `est_branches` to make it clear
this is a heuristic estimate, not a precise measurement.

Why? Users assumed "branches=5" meant debtmap counted 5 conditional
branches from source code. In reality, it's calculated using a formula.

Use `cyclomatic` for precise decision point counting.
```

## Migration and Compatibility

### Breaking Changes

**None in initial release** - Both fields present for backward compatibility.

**Future breaking change** (v1.0.0):
- Remove `branches` field from JSON output
- Only `est_branches` will be available

### Migration Steps for JSON Consumers

**Python Example**:
```python
# OLD CODE (will break in v1.0)
branches = data['complexity']['branches']

# NEW CODE (works now and future)
est_branches = data['complexity'].get('est_branches',
                                      data['complexity'].get('branches'))
```

**TypeScript Example**:
```typescript
// OLD CODE
const branches = data.complexity.branches;

// NEW CODE
const estBranches = data.complexity.est_branches
                    ?? data.complexity.branches;
```

### Rollback Plan

If renaming causes user confusion:
1. Revert terminology change
2. Add `(estimated)` suffix: `branches (estimated)=3`
3. Gather user feedback before re-attempting

## Success Metrics

### Quantitative Metrics

- **Zero regression**: All existing tests pass with terminology change
- **JSON compatibility**: 100% of integrations work with new field name
- **Documentation coverage**: Every occurrence of "branches" updated

### Qualitative Metrics

- **User clarity**: No GitHub issues about "branches" confusion
- **Adoption**: Users reference `est_branches` in discussions
- **Understanding**: Users correctly interpret estimated vs measured metrics

### Validation

**Before Implementation**:
```
User question: "Why does debtmap say branches=1 when my function has no if statements?"
```

**After Implementation**:
```
Output clearly shows: "est_branches=1 (estimated from nesting and cyclomatic)"
User understands this is an approximation, not a precise count.
```

## Future Enhancements

### Phase 2: Actual Branch Counting
- Parse AST to count real conditional branches
- Report both `actual_branches` and `est_branches`
- Compare accuracy of estimation formula

### Phase 3: Confidence Intervals
- Show estimation error bounds: `est_branches=5 (±2)`
- Learn from actual counts to improve formula
- Adaptive estimation based on language/patterns

### Phase 4: Metric Explanations in UI
- Hover tooltips showing metric definitions
- Interactive `--explain-metrics` with examples
- Contextual help in IDE integrations
