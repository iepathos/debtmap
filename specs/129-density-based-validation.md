---
number: 129
title: Density-Based Validation Metrics
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-25
---

# Specification 129: Density-Based Validation Metrics

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The `debtmap validate` command currently uses a mix of scale-dependent absolute counts (debt items, high complexity functions) and scale-independent quality ratios (debt density, average complexity). This creates a maintenance burden where thresholds must be constantly adjusted as the codebase grows.

**Current Validation Metrics (debtmap codebase):**

```
Scale-Dependent (problematic):
  High complexity functions: 369 (threshold: 375)     ← Grows linearly with codebase
  Technical debt items: 20353 (threshold: 20500)      ← Grows linearly with codebase
  High-risk functions: 0 (threshold: 50)              ← Grows linearly with codebase

Scale-Independent (good):
  Average complexity: 2.0 (threshold: 10.0)           ← Stable quality metric ✓
  Debt density: 18.1 per 1K LOC (threshold: 50.0)    ← Stable quality metric ✓
  Codebase risk score: 0.0 (threshold: 7.0)           ← Stable quality metric ✓
  Total debt score: 2035 (threshold: 5000)            ← Capped scoring helps
```

**Problems with Scale-Dependent Metrics:**

1. **Constant Maintenance**
   - 112K LOC → 20K debt items (current)
   - 150K LOC → 27K debt items (need to raise threshold)
   - 200K LOC → 36K debt items (need to raise threshold again)
   - Team spends time adjusting configs instead of improving quality

2. **False Signals**
   - More code = more absolute debt items (even if quality is excellent)
   - Can "pass" validation by just deleting code
   - Can "fail" validation by adding well-written code
   - Doesn't distinguish between "big clean codebase" and "small messy codebase"

3. **Gaming the System**
   - Can pass by spreading complexity across more functions
   - Can pass by breaking up large functions into many small ones
   - Absolute counts encourage wrong optimizations

4. **Project Comparison Fails**
   - 10K LOC project with 2K debt items: Bad? Good?
   - 100K LOC project with 5K debt items: Better or worse?
   - Can't compare quality across different-sized projects

**Why Density-Based Metrics Work:**

Debt density (debt per 1000 LOC) provides a **normalized quality measure** that:
- ✅ Remains stable as codebase grows (if quality is maintained)
- ✅ Catches quality degradation immediately (density increases)
- ✅ Enables meaningful project comparisons
- ✅ Never requires threshold adjustments
- ✅ Aligns with industry standards (defects per KLOC)

**Real-World Example:**

Current debtmap metrics:
- LOC: ~112K
- Debt items: 20,353
- Debt score: 2,035
- **Debt density: 18.1 per 1K LOC** ← This is the real quality signal

This density is **excellent** (64% below threshold), indicating high code quality regardless of absolute size.

## Objective

Transition `debtmap validate` to use **density-based quality metrics** as primary validation criteria, removing scale-dependent absolute count thresholds. This provides stable, maintainable validation that measures actual code quality rather than codebase size.

## Requirements

### Functional Requirements

1. **Primary Validation Metrics (Scale-Independent)**
   - **Debt density**: Maximum debt per 1000 lines of code
   - **Average complexity**: Mean complexity across all functions
   - **Codebase risk score**: Weighted average risk score
   - **Coverage percentage**: Test coverage ratio (if coverage data available)

2. **Safety Net Metrics (Prevent Catastrophic Failure)**
   - **Total debt score**: High ceiling to catch extreme cases only
   - Purpose: Prevent runaway growth even if density stays low
   - Should rarely trigger in normal operation

3. **Removed Metrics (Scale-Dependent)**
   - ❌ Remove `max_high_complexity_count` - grows with codebase size
   - ❌ Remove `max_debt_items` - grows with codebase size
   - ❌ Remove `max_high_risk_functions` - grows with codebase size
   - These provide no quality signal independent of size

4. **Configuration Migration**
   - Automatically migrate old configs to new density-based format
   - Warn users when scale-dependent thresholds are present
   - Provide migration guide in documentation
   - Support deprecated thresholds with warning during transition period

5. **Validation Output Enhancement**
   - Emphasize density metrics in validation summary
   - Show trend: "Density increased from 15.2 to 18.1 (+18%)" if available
   - Explain what density means: "18.1 debt points per 1000 LOC"
   - Compare to threshold: "64% below threshold (18.1 / 50.0)"

### Non-Functional Requirements

1. **Backward Compatibility**
   - Existing configs continue to work (with deprecation warnings)
   - CLI parameters unchanged
   - Output format maintains compatibility
   - Migration is opt-in initially, required in future major version

2. **Clear User Communication**
   - Validation messages explain why density matters
   - Help text emphasizes density-based approach
   - Documentation provides migration examples
   - Error messages guide users to correct configuration

3. **Performance**
   - No performance impact (same calculations already done)
   - Validation completes in same time
   - No additional computation required

4. **Maintainability**
   - Simpler validation logic (fewer thresholds to check)
   - Clearer code with focused quality metrics
   - Less configuration complexity
   - Reduced test surface area

## Acceptance Criteria

- [ ] `max_debt_density` is primary validation metric (not optional)
- [ ] `max_average_complexity` validates per-function quality
- [ ] `max_codebase_risk_score` validates overall risk level
- [ ] `min_coverage_percentage` validates test coverage (if coverage data present)
- [ ] `max_total_debt_score` exists as safety net with high ceiling (e.g., 10000)
- [ ] Scale-dependent metrics (`max_high_complexity_count`, `max_debt_items`, `max_high_risk_functions`) are deprecated
- [ ] Validation output prominently displays density and its meaning
- [ ] Validation output shows percentage of threshold used (e.g., "36% of max density")
- [ ] Configuration with old scale-dependent thresholds shows deprecation warning
- [ ] Migration guide exists in documentation
- [ ] All tests updated to use density-based validation
- [ ] Example configs in docs use density-based approach
- [ ] CI/CD workflows updated to use density thresholds

## Technical Details

### Implementation Approach

**Phase 1: Add Density Emphasis to Validation Logic**

Modify `validate_and_report()` in `commands/validate.rs`:

```rust
fn validate_with_risk(
    results: &AnalysisResults,
    insights: &risk::RiskInsight,
    lcov_data: Option<&risk::lcov::LcovData>,
    config: &ValidateConfig,
) -> (bool, ValidationDetails) {
    let thresholds = config::get_validation_thresholds();

    let unified = calculate_unified_analysis(results, lcov_data);
    let total_debt_score = unified.total_debt_score as u32;
    let debt_density = unified.debt_density;

    // === PRIMARY QUALITY METRICS (Scale-Independent) ===
    let avg_complexity_pass =
        results.complexity.summary.average_complexity <= thresholds.max_average_complexity;

    let debt_density_pass = debt_density <= thresholds.max_debt_density;

    let codebase_risk_pass =
        insights.codebase_risk_score <= thresholds.max_codebase_risk_score;

    // === SAFETY NET (Rarely triggers) ===
    let debt_score_pass = total_debt_score <= thresholds.max_total_debt_score;

    // === OPTIONAL: Coverage Requirement ===
    let coverage_percentage = lcov_data
        .map(|lcov| lcov.get_overall_coverage())
        .unwrap_or(0.0);
    let coverage_pass = coverage_percentage >= thresholds.min_coverage_percentage;

    // === DEPRECATED METRICS (Warn but allow) ===
    let deprecated_checks_pass = validate_deprecated_metrics(results, &thresholds);

    // Primary validation based on density and quality ratios
    let pass = avg_complexity_pass
        && debt_density_pass
        && codebase_risk_pass
        && debt_score_pass
        && coverage_pass
        && deprecated_checks_pass;

    // ... construct ValidationDetails with emphasis on density
}
```

**Phase 2: Configuration Structure Changes**

Update `config.rs` `ValidationThresholds`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationThresholds {
    // === PRIMARY METRICS (Required) ===

    /// Maximum allowed average complexity per function (default: 10.0)
    #[serde(default = "default_max_avg_complexity")]
    pub max_average_complexity: f64,

    /// Maximum allowed debt density per 1000 LOC (default: 50.0)
    /// This is the PRIMARY quality metric for validation
    #[serde(default = "default_max_debt_density")]
    pub max_debt_density: f64,

    /// Maximum allowed codebase risk score (default: 7.0)
    #[serde(default = "default_max_codebase_risk")]
    pub max_codebase_risk_score: f64,

    // === OPTIONAL METRICS ===

    /// Minimum required code coverage percentage (default: 0.0 - disabled)
    #[serde(default = "default_min_coverage")]
    pub min_coverage_percentage: f64,

    // === SAFETY NET ===

    /// Maximum total debt score - safety net to catch extreme cases (default: 10000)
    #[serde(default = "default_max_total_debt_score_high")]
    pub max_total_debt_score: u32,

    // === DEPRECATED (Will be removed in v1.0) ===

    /// DEPRECATED: Use max_debt_density instead
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.3.0", note = "Use max_debt_density instead")]
    pub max_high_complexity_count: Option<usize>,

    /// DEPRECATED: Use max_debt_density instead
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.3.0", note = "Use max_debt_density instead")]
    pub max_debt_items: Option<usize>,

    /// DEPRECATED: Use max_debt_density and max_codebase_risk_score instead
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.3.0", note = "Use max_debt_density instead")]
    pub max_high_risk_functions: Option<usize>,
}

fn default_max_total_debt_score_high() -> u32 {
    10000 // High ceiling - 5x typical project
}
```

**Phase 3: Enhanced Validation Output**

Modify `validation_printer.rs` to emphasize density:

```rust
pub fn print_validation_summary(details: &ValidationDetails) {
    eprintln!("\n  Metrics Summary:");

    // Emphasize density as primary metric
    eprintln!("    📊 Debt Density: {:.1} per 1K LOC (threshold: {:.1})",
        details.debt_density,
        details.max_debt_density
    );

    // Show percentage of threshold used
    let density_usage = (details.debt_density / details.max_debt_density) * 100.0;
    let density_headroom = 100.0 - density_usage;
    eprintln!("       └─ Using {:.0}% of max density ({:.0}% headroom)",
        density_usage, density_headroom
    );

    // Show other quality metrics
    eprintln!("    Average complexity: {:.1} (threshold: {:.1})",
        details.average_complexity,
        details.max_average_complexity
    );

    eprintln!("    Codebase risk score: {:.1} (threshold: {:.1})",
        details.codebase_risk_score,
        details.max_codebase_risk_score
    );

    if details.coverage_percentage > 0.0 {
        eprintln!("    Code coverage: {:.1}% (minimum: {:.1}%)",
            details.coverage_percentage,
            details.min_coverage_percentage
        );
    }

    // Show absolute counts as informational (not validation criteria)
    eprintln!("\n  📈 Codebase Statistics (informational):");
    eprintln!("    Total LOC: {}", details.total_lines_of_code);
    eprintln!("    High complexity functions: {}", details.high_complexity_count);
    eprintln!("    Technical debt items: {}", details.debt_items);
    eprintln!("    Total debt score: {}", details.total_debt_score);
}
```

**Phase 4: Deprecation Warnings**

Add warning for old configurations:

```rust
pub fn warn_deprecated_thresholds(thresholds: &ValidationThresholds) {
    let mut deprecated = Vec::new();

    if thresholds.max_high_complexity_count.is_some() {
        deprecated.push("max_high_complexity_count");
    }
    if thresholds.max_debt_items.is_some() {
        deprecated.push("max_debt_items");
    }
    if thresholds.max_high_risk_functions.is_some() {
        deprecated.push("max_high_risk_functions");
    }

    if !deprecated.is_empty() {
        eprintln!("\n⚠️  DEPRECATION WARNING:");
        eprintln!("   The following validation thresholds are deprecated:");
        for metric in &deprecated {
            eprintln!("   - {}", metric);
        }
        eprintln!("\n   These scale-dependent metrics will be removed in v1.0.");
        eprintln!("   Please migrate to density-based validation:");
        eprintln!("     - Use 'max_debt_density' instead of absolute counts");
        eprintln!("     - See migration guide: https://docs.debtmap.io/validation-migration\n");
    }
}
```

### Architecture Changes

**Modified Files:**
1. `src/config.rs` - Update `ValidationThresholds` structure with deprecations
2. `src/commands/validate.rs` - Reorder validation logic to emphasize density
3. `src/utils/validation_printer.rs` - Enhanced output with density emphasis
4. `.debtmap.toml` - Example configuration using density-based approach
5. `README.md` - Document density-based validation
6. `ARCHITECTURE.md` - Explain validation philosophy

**No Breaking Changes:**
- Old configs continue to work (with warnings)
- All existing CLI parameters supported
- Output format maintains compatibility
- Gradual migration path provided

### Data Structures

**Updated ValidationThresholds:**
```rust
pub struct ValidationThresholds {
    // Primary metrics (always validated)
    pub max_average_complexity: f64,        // Default: 10.0
    pub max_debt_density: f64,              // Default: 50.0 ← PRIMARY
    pub max_codebase_risk_score: f64,       // Default: 7.0

    // Optional metrics
    pub min_coverage_percentage: f64,       // Default: 0.0 (disabled)

    // Safety nets
    pub max_total_debt_score: u32,          // Default: 10000 (high ceiling)

    // Deprecated (Optional, will warn)
    pub max_high_complexity_count: Option<usize>,
    pub max_debt_items: Option<usize>,
    pub max_high_risk_functions: Option<usize>,
}
```

**Updated ValidationDetails:**
```rust
pub struct ValidationDetails {
    // Primary quality metrics (emphasized in output)
    pub debt_density: f64,
    pub max_debt_density: f64,
    pub average_complexity: f64,
    pub max_average_complexity: f64,
    pub codebase_risk_score: f64,
    pub max_codebase_risk_score: f64,

    // Optional coverage
    pub coverage_percentage: f64,
    pub min_coverage_percentage: f64,

    // Informational statistics (not validation criteria)
    pub total_lines_of_code: usize,
    pub high_complexity_count: usize,
    pub debt_items: usize,
    pub total_debt_score: u32,
    pub max_total_debt_score: u32,

    // Deprecated (still computed for backward compat)
    pub max_high_complexity_count: usize,
    pub max_debt_items: usize,
    pub max_high_risk_functions: usize,
}
```

### APIs and Interfaces

**Recommended Configuration Format:**

```toml
[thresholds.validation]
# === PRIMARY QUALITY METRICS ===
# These validate code quality regardless of codebase size

max_average_complexity = 10.0      # Average complexity per function
max_debt_density = 50.0            # Debt per 1000 LOC ← MOST IMPORTANT
max_codebase_risk_score = 7.0      # Overall risk (weighted average)

# === OPTIONAL: Test Coverage ===
min_coverage_percentage = 60.0     # Require reasonable test coverage

# === SAFETY NET ===
# High ceiling to catch extreme cases only
max_total_debt_score = 10000

# === DO NOT USE (Deprecated) ===
# These scale-dependent metrics will be removed in v1.0:
# max_high_complexity_count = 375  ← DON'T USE
# max_debt_items = 20500           ← DON'T USE
# max_high_risk_functions = 50     ← DON'T USE
```

**CLI Usage (unchanged):**
```bash
# Validate using density-based thresholds
debtmap validate .

# Override density threshold
debtmap validate . --max-debt-density 40.0

# Validate with coverage requirement
debtmap validate . --coverage-file lcov.info
```

## Dependencies

**Prerequisites**: None - enhances existing validation functionality

**Affected Components**:
- `src/config.rs` - Configuration structure updates
- `src/commands/validate.rs` - Validation logic reordering
- `src/utils/validation_printer.rs` - Output formatting
- `.debtmap.toml` - Example configuration
- Documentation files

**Related Specifications**:
- Spec 128: Parallel Validation Command (independent, can be implemented in either order)

## Testing Strategy

### Unit Tests

1. **Density Calculation Validation**
   ```rust
   #[test]
   fn test_debt_density_primary_metric() {
       let config = create_test_config();

       // Good density, high absolute counts → Should PASS
       let results = AnalysisResults {
           total_loc: 200_000,
           debt_score: 4000,  // High absolute
           // Density: (4000 / 200_000) * 1000 = 20.0 ← Under 50.0 threshold
       };

       assert!(validate_with_density(&config, &results).0);
   }

   #[test]
   fn test_bad_density_low_absolute_counts() {
       let config = create_test_config();

       // Bad density, low absolute counts → Should FAIL
       let results = AnalysisResults {
           total_loc: 10_000,
           debt_score: 600,  // Low absolute
           // Density: (600 / 10_000) * 1000 = 60.0 ← Over 50.0 threshold
       };

       assert!(!validate_with_density(&config, &results).0);
   }
   ```

2. **Deprecation Warning Tests**
   ```rust
   #[test]
   fn test_deprecated_threshold_warnings() {
       let config = ValidationThresholds {
           max_debt_density: 50.0,
           max_high_complexity_count: Some(375),  // Deprecated
           ..Default::default()
       };

       let warnings = get_deprecation_warnings(&config);
       assert!(warnings.contains("max_high_complexity_count"));
   }
   ```

3. **Backward Compatibility Tests**
   ```rust
   #[test]
   fn test_old_config_still_works() {
       // Old config with scale-dependent thresholds
       let old_config = r#"
           [thresholds.validation]
           max_high_complexity_count = 375
           max_debt_items = 20500
       "#;

       let config: ValidationThresholds = toml::from_str(old_config).unwrap();

       // Should parse successfully (with defaults for new fields)
       assert_eq!(config.max_debt_density, 50.0);  // Default
       assert_eq!(config.max_high_complexity_count, Some(375));
   }
   ```

### Integration Tests

1. **Validation Output Format**
   - Verify density is prominently displayed
   - Check that absolute counts shown as "informational"
   - Ensure deprecation warnings appear when appropriate
   - Validate percentage calculations are correct

2. **Migration Scenarios**
   ```bash
   # Test old config with warnings
   echo '[thresholds.validation]
   max_high_complexity_count = 375
   max_debt_items = 20500' > test.toml

   debtmap validate . --config test.toml
   # Should show deprecation warning but still validate
   ```

3. **Density Threshold Override**
   ```bash
   # CLI override of density threshold
   debtmap validate . --max-debt-density 40.0
   # Should use 40.0 instead of config value
   ```

### Performance Tests

**No Performance Impact Expected** - same calculations already performed:
- Debt density already calculated in unified analysis
- Just changing which metrics are primary validation criteria
- No additional computation required

### User Acceptance

1. **Clear Communication**
   - Validation output clearly explains density
   - Help text describes density-based approach
   - Error messages guide to correct configuration
   - Documentation provides migration examples

2. **Smooth Migration**
   - Existing configs continue to work
   - Deprecation warnings are helpful, not cryptic
   - Migration guide is easy to follow
   - Benefits of density-based approach are clear

## Documentation Requirements

### Code Documentation

1. **Inline Comments**
   ```rust
   // Debt density is the primary quality metric for validation.
   // It measures debt per 1000 LOC, providing a scale-independent
   // quality signal that remains stable as the codebase grows.
   //
   // Formula: (total_debt_score / total_loc) * 1000
   //
   // Example: 2035 debt score / 112000 LOC * 1000 = 18.1 per 1K LOC
   //
   // This is superior to absolute counts because:
   // - 20K debt items in 200K LOC (density: 10) is excellent
   // - 5K debt items in 25K LOC (density: 20) is worse quality
   let debt_density_pass = debt_density <= thresholds.max_debt_density;
   ```

2. **Configuration Documentation**
   ```rust
   /// Validation thresholds for `debtmap validate` command.
   ///
   /// # Density-Based Validation
   ///
   /// This configuration uses density-based metrics as primary validation
   /// criteria. Debt density (debt per 1000 LOC) provides a stable quality
   /// measure that doesn't require adjustment as the codebase grows.
   ///
   /// ## Recommended Configuration
   ///
   /// ```toml
   /// [thresholds.validation]
   /// max_debt_density = 50.0             # Primary quality metric
   /// max_average_complexity = 10.0       # Per-function quality
   /// max_codebase_risk_score = 7.0       # Overall risk level
   /// ```
   ///
   /// ## Migration from Scale-Dependent Metrics
   ///
   /// Old scale-dependent metrics are deprecated:
   /// - `max_high_complexity_count` → Use `max_debt_density`
   /// - `max_debt_items` → Use `max_debt_density`
   /// - `max_high_risk_functions` → Use `max_debt_density` + `max_codebase_risk_score`
   pub struct ValidationThresholds { ... }
   ```

### User Documentation

1. **README.md Update**

Add section on validation:

```markdown
## Validation

Validate your codebase against quality thresholds:

```bash
debtmap validate .
```

### Density-Based Validation

Debtmap uses **debt density** as the primary quality metric. This measures
debt per 1000 lines of code, providing a stable quality signal that doesn't
require adjustment as your codebase grows.

**Why density matters:**
- 20K debt items in 200K LOC (density: 10) → Excellent quality
- 5K debt items in 25K LOC (density: 20) → Lower quality
- Absolute counts can't distinguish these cases

**Recommended thresholds:**
- `max_debt_density`: 50.0 per 1K LOC (adjust based on project standards)
- `max_average_complexity`: 10.0 per function
- `max_codebase_risk_score`: 7.0 overall risk

See [Validation Guide](docs/validation.md) for detailed configuration.
```

2. **Migration Guide** (docs/validation-migration.md)

```markdown
# Migrating to Density-Based Validation

## Why Migrate?

Scale-dependent metrics (`max_high_complexity_count`, `max_debt_items`)
require constant adjustment as your codebase grows. Density-based metrics
provide stable quality validation at any codebase size.

## Migration Steps

1. **Identify Current Density**
   ```bash
   debtmap validate . | grep "Debt density"
   ```

2. **Update Configuration**

   Old (deprecated):
   ```toml
   [thresholds.validation]
   max_high_complexity_count = 375
   max_debt_items = 20500
   ```

   New (recommended):
   ```toml
   [thresholds.validation]
   max_debt_density = 50.0
   max_average_complexity = 10.0
   ```

3. **Verify Validation**
   ```bash
   debtmap validate .
   ```

## Threshold Selection

Choose thresholds based on project maturity:

- **Greenfield projects**: 20-30 per 1K LOC
- **Mature projects**: 30-50 per 1K LOC
- **Legacy projects**: 50-100 per 1K LOC

Your current density indicates where you are today. Set threshold
slightly above current to prevent degradation while improving.
```

3. **ARCHITECTURE.md Update**

Add section explaining validation philosophy:

```markdown
## Validation Philosophy

Debtmap validation uses **density-based metrics** to ensure consistent
quality measurement regardless of codebase size.

### Primary Metrics

1. **Debt Density** (debt per 1K LOC)
   - Scale-independent quality measure
   - Stable as codebase grows
   - Primary validation criterion

2. **Average Complexity** (mean per function)
   - Measures typical function complexity
   - Catches overall code quality issues

3. **Codebase Risk Score** (weighted average)
   - Overall risk level across project
   - Combines complexity, coverage, criticality

### Why Not Absolute Counts?

Absolute metrics (total debt items, high complexity count) grow
linearly with codebase size, making them poor quality indicators:

- Can't compare across different-sized projects
- Require constant threshold adjustment
- Can be gamed by adding/removing code
- Don't measure actual quality

Density provides a normalized view that works at any scale.
```

## Implementation Notes

### Phased Rollout Strategy

**Phase 1: Add Deprecation Warnings (v0.3.0)**
- Warn when scale-dependent thresholds are used
- Document migration path
- All existing configs continue to work

**Phase 2: Change Defaults (v0.4.0)**
- New projects get density-based defaults
- Example configs use density approach
- Migration guide prominently linked

**Phase 3: Remove Deprecated Metrics (v1.0.0)**
- Breaking change: Remove scale-dependent thresholds
- Only density-based validation supported
- Clear upgrade guide provided

### Threshold Selection Guidelines

**For New Projects:**
- Start with restrictive thresholds (30-40 per 1K LOC)
- Enforce quality from the beginning
- Easier to maintain than to fix later

**For Existing Projects:**
- Calculate current density: `debtmap validate . | grep density`
- Set threshold 10-20% above current density
- Gradually lower threshold as you pay down debt
- Never set threshold below current density (instant failure)

**General Guidelines:**
- **< 20 per 1K LOC**: Excellent quality
- **20-30**: Good quality
- **30-50**: Acceptable quality
- **50-100**: Needs improvement
- **> 100**: Significant technical debt

### Common Pitfalls

1. **Setting Threshold Too Low Initially**
   - Problem: Immediate validation failure
   - Solution: Set threshold above current density, improve gradually

2. **Ignoring Density Trends**
   - Problem: Slow quality degradation goes unnoticed
   - Solution: Track density over time, fail on significant increases

3. **Comparing Density Across Different Languages**
   - Problem: Rust naturally has different density than Python
   - Solution: Set language-specific thresholds if needed

4. **Confusing Density with Absolute Counts**
   - Problem: "We have 20K debt items, that's too many!"
   - Solution: Look at density - might be excellent for large codebase

## Migration and Compatibility

### Breaking Changes

**None in initial release** - this is a configuration migration:
- Old configs continue to work (with warnings)
- New configs use density-based approach
- Both approaches produce same validation pass/fail

**Future breaking change (v1.0.0)**:
- Remove `max_high_complexity_count`
- Remove `max_debt_items`
- Remove `max_high_risk_functions`
- Only density-based metrics supported

### Migration Requirements

**For Users:**
1. Run current validation to see density
2. Update `.debtmap.toml` to use density thresholds
3. Remove deprecated scale-dependent thresholds
4. Commit new configuration

**For CI/CD:**
1. Update validation thresholds in config
2. Verify validation passes with new thresholds
3. Update documentation referencing old metrics

### Compatibility Considerations

**Version Compatibility:**
- v0.2.x: No density-based validation
- v0.3.x: Density added, scale-dependent deprecated
- v0.4.x: Density default, scale-dependent warned
- v1.0.x: Only density-based validation

**Cross-Version Projects:**
- Use density thresholds in config
- Works with v0.3.x and later
- Graceful degradation on older versions

### Example Migrations

**Small Project (10K LOC, 500 debt items):**
```toml
# Old
max_debt_items = 500

# New (density: (500/10000)*1000 = 50)
max_debt_density = 50.0
```

**Large Project (200K LOC, 10K debt items):**
```toml
# Old
max_debt_items = 10000

# New (density: (10000/200000)*1000 = 50)
max_debt_density = 50.0
```

**Notice**: Same density threshold works for both sizes!
