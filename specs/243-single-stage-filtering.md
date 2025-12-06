---
number: 243
title: Consolidate to Single-Stage Filtering
category: foundation
priority: high
status: draft
dependencies: [242]
created: 2025-01-06
---

# Specification 243: Consolidate to Single-Stage Filtering

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 242 (Pure Filter Predicates)

## Context

Currently, filtering happens in two separate stages with different thresholds:

**Stage 1**: During item construction (`add_item` in `unified_analysis_utils.rs`)
- Uses `min_debt_score` threshold (default: 1.0)
- Filters by complexity, risk, duplicates
- Invisible filtering (no output)

**Stage 2**: Before display (`filter_by_score_threshold` in `mod.rs`)
- Uses `min_score_threshold` (default: 3.0)
- Filters by score and tier
- Applied in analyze command before TUI/output

This creates several problems:
- **Confusing behavior**: Items pass first filter but fail second
- **Inconsistency**: `--no-tui` and TUI show different items (different filter paths)
- **Dual configuration**: Two different "minimum score" thresholds
- **God object bug**: Items with score 1.0-2.9 filtered unexpectedly
- **Hard to reason about**: Need to trace through two filter stages
- **No single source of truth**: Filter logic duplicated

The god object TUI bug was caused by this dual filtering: god objects had normalized scores around 1.0, which passed the first filter (>= 1.0) but failed the second filter (< 3.0).

## Objective

Consolidate all filtering into a single stage during item construction. Remove the `filter_by_score_threshold` function entirely. Use a single, unified filter configuration with clear precedence: CLI args > environment variables > config file > defaults.

This achieves:
- **Predictability**: Items in `UnifiedAnalysis` are exactly what gets displayed
- **Consistency**: `--no-tui` and TUI show identical items
- **Simplicity**: One filter stage, one configuration
- **Single source of truth**: Filter configuration centralized

## Requirements

### Functional Requirements

**FR1**: Create unified `ItemFilterConfig` struct
- Single source of filter configuration
- Fields: `min_score`, `min_cyclomatic`, `min_cognitive`, `min_risk`, `show_t4_items`
- Clear, documented defaults

**FR2**: Implement configuration precedence
- Priority: CLI args > env vars > config file > defaults
- Method: `ItemFilterConfig::from_environment()`
- All thresholds resolved through single mechanism
- Transparent to caller (returns fully configured struct)

**FR3**: Update `add_item` to use `ItemFilterConfig`
- Replace individual threshold calls with single config
- Use predicates from Spec 242
- Single configuration point (no dual thresholds)

**FR4**: Remove `filter_by_score_threshold` function
- Delete function from `src/priority/mod.rs`
- Remove all call sites in analyze command
- Update tests that relied on post-filtering

**FR5**: Update analyze command
- Remove `filter_by_score_threshold` call
- Items in `UnifiedAnalysis` are final (no post-filtering)
- Both TUI and `--no-tui` use same items

**FR6**: Consolidate threshold environment variables
- Deprecate `DEBTMAP_MIN_DEBT_SCORE` (rarely used, confusing)
- Use `DEBTMAP_MIN_SCORE_THRESHOLD` consistently
- Document in config and CLI help

### Non-Functional Requirements

**NFR1**: Backward compatibility
- Existing config files continue to work
- Environment variables continue to work
- CLI args continue to work
- May need to adjust default threshold

**NFR2**: Clear error messages
- If no items pass filter, explain why
- Suggest adjusting thresholds
- Show filter statistics

**NFR3**: Performance
- No regression from single-stage filtering
- Configuration lookup happens once, not per item

**NFR4**: Documentation
- Clearly document single filtering stage
- Explain configuration precedence
- Migration guide for users

## Acceptance Criteria

- [ ] **AC1**: `ItemFilterConfig` struct defined
  - Located in `src/priority/filter_config.rs`
  - Fields for all filter thresholds
  - Implements `Debug`, `Clone`
  - Documented with examples

- [ ] **AC2**: `from_environment()` implements precedence correctly
  - CLI args override env vars (handled by caller)
  - Env vars override config file
  - Config file overrides defaults
  - Integration test verifies precedence

- [ ] **AC3**: `add_item` uses `ItemFilterConfig`
  - Gets config once, not per threshold
  - Passes config values to predicates
  - No hardcoded thresholds

- [ ] **AC4**: `filter_by_score_threshold` function removed
  - Function deleted from `src/priority/mod.rs`
  - All call sites removed from `src/commands/analyze.rs`
  - Tests updated or removed

- [ ] **AC5**: Analyze command doesn't post-filter
  - Items in `UnifiedAnalysis` are final
  - No filtering between analysis and display
  - Same items for TUI and `--no-tui`

- [ ] **AC6**: TUI and `--no-tui` consistency verified
  - Integration test: Compare item counts
  - Integration test: Compare actual items
  - Test with various filter configurations

- [ ] **AC7**: God objects always appear
  - Integration test: God objects with score > 50 appear
  - Test: God objects bypass complexity filter (exemption)
  - Test: God objects compared against single threshold

- [ ] **AC8**: Configuration precedence tested
  - Test: Env var overrides config file
  - Test: Config file overrides default
  - Test: All thresholds have correct precedence

- [ ] **AC9**: Empty results handling
  - If no items pass filter, show helpful message
  - Suggest adjusting `--min-score` CLI arg
  - Show filter statistics with `--show-filter-stats`

- [ ] **AC10**: Documentation complete
  - Config file documents `min_score_threshold`
  - CLI help explains filtering
  - ARCHITECTURE.md updated
  - Migration guide for users

- [ ] **AC11**: All tests pass
  - Unit tests for `ItemFilterConfig`
  - Integration tests for filtering
  - TUI tests with filtered items
  - Consistency tests (TUI vs --no-tui)

- [ ] **AC12**: Performance verified
  - No regression vs current implementation
  - Configuration lookup once per analysis
  - Benchmarks pass

## Technical Details

### Implementation Approach

**Phase 1: Create Filter Configuration Module**

Create `src/priority/filter_config.rs`:

```rust
//! Unified filter configuration for debt items.
//!
//! This module provides a single source of truth for all filtering thresholds.
//! Configuration precedence: CLI args > env vars > config file > defaults.

use crate::config;

/// Unified filter configuration for debt items.
///
/// All filtering happens during item construction using these thresholds.
/// There is no post-filtering stage.
///
/// # Configuration Precedence
///
/// 1. CLI arguments (handled by caller, passed to constructor)
/// 2. Environment variables
/// 3. Config file (`Config.toml`)
/// 4. Hardcoded defaults
///
/// # Examples
///
/// ```rust
/// // Get configuration from environment
/// let config = ItemFilterConfig::from_environment();
///
/// // Override with CLI args
/// let config = ItemFilterConfig::from_environment()
///     .with_min_score(Some(10.0));
///
/// // Use in filtering
/// if meets_score_threshold(&item, config.min_score) { ... }
/// ```
#[derive(Debug, Clone)]
pub struct ItemFilterConfig {
    /// Minimum unified score threshold (0-100 scale)
    pub min_score: f64,

    /// Minimum cyclomatic complexity threshold
    pub min_cyclomatic: u32,

    /// Minimum cognitive complexity threshold
    pub min_cognitive: u32,

    /// Minimum risk score threshold (0-1 scale)
    pub min_risk: f64,

    /// Whether to show T4 (low priority) items
    pub show_t4_items: bool,
}

impl ItemFilterConfig {
    /// Create configuration from environment (env vars, config file, defaults).
    ///
    /// Precedence: env vars > config file > defaults
    ///
    /// # Environment Variables
    ///
    /// - `DEBTMAP_MIN_SCORE_THRESHOLD`: Minimum score (0-100)
    /// - `DEBTMAP_MIN_CYCLOMATIC`: Minimum cyclomatic complexity
    /// - `DEBTMAP_MIN_COGNITIVE`: Minimum cognitive complexity
    /// - `DEBTMAP_MIN_RISK`: Minimum risk score (0-1)
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Get from environment
    /// let config = ItemFilterConfig::from_environment();
    ///
    /// // Override min_score from CLI
    /// let config = config.with_min_score(Some(10.0));
    /// ```
    pub fn from_environment() -> Self {
        Self {
            min_score: get_min_score_threshold(),
            min_cyclomatic: config::get_minimum_cyclomatic_complexity(),
            min_cognitive: config::get_minimum_cognitive_complexity(),
            min_risk: config::get_minimum_risk_score(),
            show_t4_items: get_show_t4_items(),
        }
    }

    /// Override minimum score (for CLI args).
    pub fn with_min_score(mut self, min_score: Option<f64>) -> Self {
        if let Some(score) = min_score {
            self.min_score = score;
        }
        self
    }

    /// Override minimum cyclomatic complexity (for CLI args).
    pub fn with_min_cyclomatic(mut self, min_cyclomatic: Option<u32>) -> Self {
        if let Some(cyc) = min_cyclomatic {
            self.min_cyclomatic = cyc;
        }
        self
    }

    /// Override minimum cognitive complexity (for CLI args).
    pub fn with_min_cognitive(mut self, min_cognitive: Option<u32>) -> Self {
        if let Some(cog) = min_cognitive {
            self.min_cognitive = cog;
        }
        self
    }

    /// Create permissive configuration (for testing).
    pub fn permissive() -> Self {
        Self {
            min_score: 0.0,
            min_cyclomatic: 0,
            min_cognitive: 0,
            min_risk: 0.0,
            show_t4_items: true,
        }
    }
}

/// Get minimum score threshold with precedence.
fn get_min_score_threshold() -> f64 {
    // Environment variable takes precedence
    if let Ok(env_value) = std::env::var("DEBTMAP_MIN_SCORE_THRESHOLD") {
        if let Ok(threshold) = env_value.parse::<f64>() {
            return threshold;
        }
    }

    // Fallback to config file
    config::get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.min_score_threshold)
        .unwrap_or(3.0) // Default
}

/// Get show T4 items setting.
fn get_show_t4_items() -> bool {
    config::get_config()
        .tier_config
        .as_ref()
        .map(|t| t.show_t4_in_main_report)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration_has_reasonable_thresholds() {
        let config = ItemFilterConfig::from_environment();

        assert!(config.min_score >= 0.0);
        assert!(config.min_cyclomatic >= 0);
        assert!(config.min_cognitive >= 0);
        assert!(config.min_risk >= 0.0);
    }

    #[test]
    fn with_min_score_overrides() {
        let config = ItemFilterConfig::from_environment()
            .with_min_score(Some(10.0));

        assert_eq!(config.min_score, 10.0);
    }

    #[test]
    fn permissive_config_allows_everything() {
        let config = ItemFilterConfig::permissive();

        assert_eq!(config.min_score, 0.0);
        assert_eq!(config.min_cyclomatic, 0);
        assert_eq!(config.min_cognitive, 0);
        assert!(config.show_t4_items);
    }
}
```

**Phase 2: Update add_item**

```rust
// src/priority/unified_analysis_utils.rs

use crate::priority::filter_config::ItemFilterConfig;
use crate::priority::filter_predicates::*;

impl UnifiedAnalysisUtils for UnifiedAnalysis {
    fn add_item(&mut self, item: UnifiedDebtItem) {
        self.stats.total_items_processed += 1;

        // Get unified filter configuration (once, not per threshold)
        let config = ItemFilterConfig::from_environment();

        // Apply filters using pure predicates
        if !meets_score_threshold(&item, config.min_score) {
            self.stats.filtered_by_score += 1;
            return;
        }

        if !meets_risk_threshold(&item, config.min_risk) {
            self.stats.filtered_by_risk += 1;
            return;
        }

        if !meets_complexity_thresholds(&item, config.min_cyclomatic, config.min_cognitive) {
            self.stats.filtered_by_complexity += 1;
            return;
        }

        if self.items.iter().any(|existing| is_duplicate_of(&item, existing)) {
            self.stats.filtered_as_duplicate += 1;
            return;
        }

        // Item passed all filters
        self.items.push_back(item);
        self.stats.items_added += 1;
    }
}
```

**Phase 3: Update Analyze Command**

```rust
// src/commands/analyze.rs

pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    // ... analysis setup ...

    let mut unified_analysis = unified_analysis::perform_unified_analysis_with_options(...)?;

    // NO MORE filter_by_score_threshold call
    // Items in unified_analysis are final

    // Apply file context adjustments
    unified_analysis.apply_file_context_adjustments(&results.file_contexts);

    // Cleanup TUI BEFORE writing output
    if let Some(manager) = ProgressManager::global() {
        manager.tui_set_progress(1.0);
        manager.tui_cleanup();
    }

    // Show filter statistics if requested
    if config.show_filter_stats {
        unified_analysis.log_filter_summary();
    }

    // Check if filtering removed all items
    if unified_analysis.items.is_empty() && !quiet_mode {
        eprintln!("Warning: No items passed filtering thresholds.");
        eprintln!("Try lowering thresholds:");
        eprintln!("  --min-score 1.0");
        eprintln!("  --min-cyclomatic 1");
        eprintln!("  --min-cognitive 1");
        eprintln!("\nOr view filter statistics:");
        eprintln!("  DEBTMAP_SHOW_FILTER_STATS=1 debtmap analyze");
    }

    // Determine output mode
    if should_use_tui(&config) {
        let mut explorer = ResultsExplorer::new(unified_analysis)?;
        explorer.run()?;
    } else {
        output::output_unified_priorities_with_config(
            unified_analysis,
            output_config,
            &results,
            config.coverage_file.as_ref(),
        )?;
    }

    Ok(())
}
```

**Phase 4: Remove filter_by_score_threshold**

```rust
// src/priority/mod.rs

// DELETE THIS FUNCTION:
// pub fn filter_by_score_threshold(&self, min_score: f64) -> Self { ... }

// UPDATE: Remove from trait if it's in UnifiedAnalysisUtils
```

**Phase 5: Add Integration Test**

```rust
// tests/single_stage_filtering_test.rs

#[test]
fn tui_and_no_tui_show_identical_items() {
    // Run analysis
    let analysis = run_test_analysis("test_project");

    // Simulate TUI path (no post-filtering)
    let tui_items: Vec<_> = analysis.items.iter().collect();

    // Simulate --no-tui path (no post-filtering)
    let no_tui_items: Vec<_> = analysis.items.iter().collect();

    // Should be identical
    assert_eq!(tui_items.len(), no_tui_items.len());
    for (tui_item, no_tui_item) in tui_items.iter().zip(no_tui_items.iter()) {
        assert_eq!(tui_item.location, no_tui_item.location);
        assert_eq!(tui_item.unified_score.final_score, no_tui_item.unified_score.final_score);
    }
}

#[test]
fn god_objects_always_appear_with_default_config() {
    let analysis = run_test_analysis("project_with_god_objects");

    // God objects should be present
    let god_objects: Vec<_> = analysis
        .items
        .iter()
        .filter(|item| matches!(item.debt_type, DebtType::GodObject { .. }))
        .collect();

    assert!(!god_objects.is_empty(), "God objects should appear in analysis");

    // Verify they all have reasonable scores
    for god_object in god_objects {
        assert!(god_object.unified_score.final_score >= 50.0,
            "God object scores should be >= 50.0");
    }
}

#[test]
fn configuration_precedence_works() {
    // Set env var
    std::env::set_var("DEBTMAP_MIN_SCORE_THRESHOLD", "15.0");

    let config = ItemFilterConfig::from_environment();
    assert_eq!(config.min_score, 15.0);

    // CLI override
    let config = config.with_min_score(Some(20.0));
    assert_eq!(config.min_score, 20.0);

    std::env::remove_var("DEBTMAP_MIN_SCORE_THRESHOLD");
}
```

### Architecture Changes

**New Module**: `src/priority/filter_config.rs`
- `ItemFilterConfig` struct
- Configuration precedence logic
- CLI override methods

**Updated Module**: `src/priority/unified_analysis_utils.rs`
- `add_item` uses `ItemFilterConfig`
- Single configuration point

**Deleted Function**: `src/priority/mod.rs::filter_by_score_threshold`
- Remove function and all call sites

**Updated Module**: `src/commands/analyze.rs`
- Remove post-filtering call
- Add empty results handling
- Add filter stats logging

### Data Flow

```
Old (Dual-Stage):
  ┌─────────────┐
  │ Item Created│
  └──────┬──────┘
         ↓
  ┌──────────────────┐
  │ add_item filter  │ (min_debt_score: 1.0)
  │  - Score >= 1.0  │
  │  - Complexity    │
  │  - Duplicates    │
  └──────┬───────────┘
         ↓
  ┌────────────────────┐
  │ UnifiedAnalysis    │
  └──────┬─────────────┘
         ↓
  ┌────────────────────────────┐
  │ filter_by_score_threshold  │ (min_score_threshold: 3.0)
  │  - Score >= 3.0            │
  │  - Tier filtering          │
  └──────┬─────────────────────┘
         ↓
  ┌──────────────┐
  │ Display/TUI  │
  └──────────────┘

New (Single-Stage):
  ┌─────────────┐
  │ Item Created│
  └──────┬──────┘
         ↓
  ┌─────────────────────┐
  │ add_item filter     │ (unified config: 3.0)
  │  - Score >= 3.0     │
  │  - Complexity       │
  │  - Risk             │
  │  - Duplicates       │
  └──────┬──────────────┘
         ↓
  ┌────────────────────┐
  │ UnifiedAnalysis    │ (Final!)
  └──────┬─────────────┘
         ↓
  ┌──────────────┐
  │ Display/TUI  │ (No more filtering)
  └──────────────┘
```

### APIs and Interfaces

**Public API**:
```rust
// Create configuration
let config = ItemFilterConfig::from_environment();

// Override from CLI
let config = config.with_min_score(cli_args.min_score);

// Use in filtering (internal)
if meets_score_threshold(&item, config.min_score) { ... }
```

## Dependencies

### Prerequisites
- **Spec 242**: Pure filter predicates must exist
- Predicates provide the filtering logic
- Statistics track filtering behavior

### Affected Components
- `src/priority/filter_config.rs` - New module
- `src/priority/unified_analysis_utils.rs` - Updated `add_item`
- `src/priority/mod.rs` - Remove `filter_by_score_threshold`
- `src/commands/analyze.rs` - Remove post-filtering
- `src/config/accessors.rs` - May deprecate `get_minimum_debt_score`

### External Dependencies
None

## Testing Strategy

### Unit Tests
- Test `ItemFilterConfig::from_environment()`
- Test configuration precedence
- Test CLI override methods
- Test permissive configuration

### Integration Tests
- Test TUI and --no-tui consistency
- Test god objects always appear
- Test empty results handling
- Test filter statistics accuracy

### Migration Tests
- Test existing config files work
- Test environment variables work
- Test default behavior reasonable

## Documentation Requirements

### Code Documentation
- Module docs for `filter_config.rs`
- Rustdoc for configuration precedence
- Examples of CLI overrides

### User Documentation
- Update CLI help for `--min-score`
- Document configuration precedence
- Explain single filtering stage
- Migration guide if defaults change

### Architecture Updates
Update `ARCHITECTURE.md`:
- Document single-stage filtering
- Explain configuration precedence
- Remove dual-filtering section

## Implementation Notes

### Configuration Caching
- Could cache `ItemFilterConfig` in `UnifiedAnalysis`
- Avoid re-creating on every `add_item` call
- Trade-off: Slightly more complex vs minimal performance gain

### Default Threshold
- Current: `min_score_threshold = 3.0`
- May need to adjust after removing dual filtering
- Monitor user feedback on item counts

### Empty Results
- Provide helpful message if no items pass
- Suggest lowering thresholds
- Show filter statistics automatically

## Migration and Compatibility

### Breaking Changes
- Items with score 1.0-2.9 will now appear (if they pass other filters)
- For god objects, this is desired (fixes bug)
- For other items, may need to adjust default threshold

### Migration Steps
1. Create `filter_config.rs` module
2. Add `ItemFilterConfig` struct
3. Update `add_item` to use config
4. Remove `filter_by_score_threshold` function
5. Update analyze command
6. Add integration tests
7. Monitor for unexpected items appearing

### Backward Compatibility
- Existing config files work (same keys)
- Environment variables work
- CLI args work
- May need to adjust default threshold

### Rollback Plan
If issues:
1. Revert analyze command changes
2. Restore `filter_by_score_threshold` function
3. Keep `ItemFilterConfig` (useful for future)
4. Document why rollback needed

## Success Metrics

- [ ] Single filtering stage (no post-filtering)
- [ ] TUI and --no-tui show identical items
- [ ] God objects consistently appear
- [ ] Configuration precedence works correctly
- [ ] Empty results handled gracefully
- [ ] Filter statistics available for debugging
- [ ] No performance regression
- [ ] User confusion eliminated (single threshold)
