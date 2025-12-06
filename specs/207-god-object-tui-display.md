---
number: 207
title: Enable God Object Display in TUI
category: compatibility
priority: high
status: draft
dependencies: [133]
created: 2025-12-06
---

# Specification 207: Enable God Object Display in TUI

**Category**: compatibility
**Priority**: high
**Status**: draft
**Dependencies**: [133 - God Object Detection Refinement]

## Context

God objects are correctly detected and displayed in `--no-tui` output but are completely missing from the TUI interface. This creates a confusing user experience where critical technical debt items visible in CLI output are invisible in the interactive TUI.

### Current State

**Working: --no-tui Output**
```
TOP 10 RECOMMENDATIONS

#1 SCORE: 50.4 [CRITICAL]
├─ LOCATION: ./src/main.rs
├─ IMPACT: -39 complexity, -248.4 maintainability improvement
├─ METRICS: 1616 lines, 91 functions, avg complexity: 2.2
├─ GOD OBJECT: 49 methods, 87 fields, 8 responsibilities (score: 1.0)
   Suggested: 3 recommended module splits
├─ ACTION: URGENT: 1616 lines, 91 functions! Split by data flow...
```

**Broken: TUI Display**
- God objects completely absent from the items list
- No god object items shown in any tier or category
- Users cannot see or interact with god object recommendations in TUI

### Root Cause Analysis

There are **two separate blocking issues**:

#### Issue #1: No Conversion from Indicators to UnifiedDebtItem

**Location**: `src/builders/unified_analysis.rs:1519-1554`

- God objects ARE detected via `GodObjectDetector`
- Results ARE stored in `FileDebtMetrics.god_object_indicators`
- The `create_god_object_analysis()` function creates `GodObjectAnalysis` structs
- **BUT**: No code creates `UnifiedDebtItem` instances with `DebtType::GodObject` from these indicators
- The TUI only displays `UnifiedDebtItem` instances, so god objects never appear

**Why --no-tui works differently**:
- Uses legacy analysis path in `src/analyzers/rust.rs:1404-1528`
- Converts god objects to `DebtType::CodeOrganization` (not `DebtType::GodObject`)
- Uses `pattern_to_message_context()` and `convert_organization_pattern_to_debt_item()`
- Bypasses the unified item system entirely

#### Issue #2: Complexity Threshold Filtering Blocks God Objects

**Location**: `src/priority/unified_analysis_utils.rs:117-132`

```rust
// Filter out trivial functions based on configured complexity thresholds.
// Test-related items are exempt as they have different complexity characteristics.
if !matches!(
    item.debt_type,
    DebtType::TestComplexityHotspot { .. }
        | DebtType::TestTodo { .. }
        | DebtType::TestDuplication { .. }
) {
    // Enforce cyclomatic complexity threshold
    if item.cyclomatic_complexity < min_cyclomatic {
        return;  // God objects would be rejected here!
    }

    // Enforce cognitive complexity threshold
    if item.cognitive_complexity < min_cognitive {
        return;  // Or here!
    }
}
```

**The Problem**:
- `DebtType::GodObject` is NOT in the exemption list
- God objects are **file-level** metrics, not function-level
- They typically have LOW cyclomatic/cognitive complexity for the file itself
- Default thresholds (cyclomatic: 3, cognitive: 5) would reject most god objects
- Even if Issue #1 is fixed, god objects would still be filtered out

#### Issue #3: Missing File-Level Item Support

**Location**: `src/priority/unified_analysis_utils.rs:77-94`

The `UnifiedDebtCollection` has separate storage for file-level and function-level items:
- `items: Vector<UnifiedDebtItem>` - function-level items
- `file_items: Vector<UnifiedDebtItem>` - file-level items

However:
- `add_file_item()` (lines 77-94) exists for file-level items
- God objects should use `add_file_item()` to bypass complexity filtering
- Currently, no code path creates file-level god object items

### Architecture Overview

```
Detection Path (Works):
  GodObjectDetector → GodObjectAnalysis → FileDebtMetrics.god_object_indicators

Unified Item Creation (Missing):
  god_object_indicators → ??? → UnifiedDebtItem(DebtType::GodObject) → MISSING!

TUI Display (Works if items exist):
  UnifiedDebtCollection.file_items → TUI list view → User sees items
```

**The Gap**: No code bridges `god_object_indicators` to `UnifiedDebtItem` creation.

## Objective

Enable god objects to appear in the TUI by:
1. Creating `UnifiedDebtItem` instances from `god_object_indicators`
2. Storing them as file-level items to bypass complexity filtering
3. Ensuring they display correctly in the TUI with proper formatting

## Requirements

### Functional Requirements

1. **UnifiedDebtItem Creation from God Object Indicators**
   - When `FileDebtMetrics.god_object_indicators.is_god_object == true`, create a `UnifiedDebtItem`
   - Use `DebtType::GodObject` (not `CodeOrganization`)
   - Populate all required fields from god object analysis data
   - Preserve detection type (GodClass vs GodFile) from indicators

2. **File-Level Item Storage**
   - Add god object items to `UnifiedDebtCollection.file_items` (not `items`)
   - Use `add_file_item()` instead of `add_item()` to bypass complexity filtering
   - File-level items should skip cyclomatic/cognitive complexity thresholds

3. **Scoring and Prioritization**
   - Calculate unified scores for god object items
   - Use god object score (0.0-1.0) as basis for final score
   - Apply appropriate tier classification (should be Tier 1 - Critical)
   - Preserve impact metrics (complexity reduction, maintainability improvement)

4. **TUI Display Integration**
   - God objects must appear in TUI list view alongside other items
   - Display in correct tier (Tier 1 - Critical)
   - Show proper icon/indicator for god objects
   - Detail view should show god object-specific metrics (methods, fields, responsibilities)

5. **Consistency with --no-tui Output**
   - God objects should appear in both TUI and --no-tui output
   - Same items identified in both modes
   - Same scoring and prioritization
   - Same action recommendations

### Non-Functional Requirements

1. **Performance**: God object item creation should add <2% overhead to analysis time
2. **Memory**: File-level items stored efficiently without duplication
3. **Maintainability**: Clear separation between file-level and function-level item creation
4. **Testability**: Unit tests for god object item creation and filtering logic

## Acceptance Criteria

- [ ] God objects detected in analysis appear as items in TUI list view
- [ ] God object items bypass cyclomatic/cognitive complexity filtering
- [ ] God objects are stored in `file_items` collection, not `items`
- [ ] TUI displays god objects with correct tier (Tier 1 - Critical)
- [ ] Detail view shows god object-specific metrics (methods, fields, responsibilities)
- [ ] Detection type (GodClass vs GodFile) is preserved and displayed
- [ ] Recommended splits appear in detail view
- [ ] God objects appear in correct priority order based on unified score
- [ ] Both TUI and --no-tui show the same god object items
- [ ] Running debtmap on itself shows god objects in TUI (e.g., src/main.rs)
- [ ] File-level items don't get filtered by function-level complexity thresholds
- [ ] All existing tests continue to pass
- [ ] New tests verify god object item creation and TUI display

## Technical Details

### Implementation Approach

#### Phase 1: Create UnifiedDebtItem from God Object Indicators

**Location**: `src/builders/unified_analysis.rs` (after line 1554)

Add new function to convert god object analysis to unified debt item:

```rust
/// Pure function to create a UnifiedDebtItem from god object indicators
fn create_god_object_debt_item(
    file_path: &Path,
    file_metrics: &FileDebtMetrics,
    god_analysis: &crate::organization::GodObjectAnalysis,
) -> UnifiedDebtItem {
    // Calculate unified score based on god object score
    let base_score = god_analysis.god_object_score; // 0-100 scale
    let unified_score = UnifiedScore {
        final_score: base_score,
        tier: if base_score >= 50.0 { 1 } else { 2 },
        raw_scores: RawScores {
            complexity_score: file_metrics.total_complexity as f64,
            maintainability_score: calculate_maintainability_impact(file_metrics),
            risk_score: calculate_god_object_risk(god_analysis),
        },
        weights: ScoreWeights::default(),
    };

    UnifiedDebtItem {
        location: ItemLocation {
            file: file_path.to_string_lossy().to_string(),
            line: Some(1), // File-level item starts at line 1
            column: None,
            name: extract_god_object_name(file_path, &god_analysis.detection_type),
        },
        debt_type: DebtType::GodObject,
        cyclomatic_complexity: 0, // File-level metric, not function-level
        cognitive_complexity: 0,  // File-level metric, not function-level
        unified_score,
        impact_metrics: calculate_god_object_impact(file_metrics, god_analysis),
        context: GodObjectContext {
            method_count: god_analysis.method_count,
            field_count: god_analysis.field_count,
            responsibility_count: god_analysis.responsibility_count,
            recommended_splits: god_analysis.recommended_splits.clone(),
            detection_type: god_analysis.detection_type,
        },
    }
}

/// Extract name for god object based on detection type
fn extract_god_object_name(file_path: &Path, detection_type: &DetectionType) -> String {
    match detection_type {
        DetectionType::GodClass => {
            // Try to extract primary struct/class name from file
            // Fallback to filename if not found
            extract_primary_type_name(file_path)
                .unwrap_or_else(|| format!("{} (God Class)", file_path.file_stem().unwrap().to_string_lossy()))
        }
        DetectionType::GodFile | DetectionType::GodModule => {
            format!("{} (God Module)", file_path.file_stem().unwrap().to_string_lossy())
        }
    }
}

/// Calculate maintainability impact from file metrics
fn calculate_maintainability_impact(file_metrics: &FileDebtMetrics) -> f64 {
    // Higher complexity and more lines = higher maintainability impact
    let complexity_factor = (file_metrics.total_complexity as f64 / 100.0).min(10.0);
    let lines_factor = (file_metrics.total_lines as f64 / 500.0).min(10.0);

    (complexity_factor + lines_factor) * 10.0 // Scale to 0-200 range
}

/// Calculate risk score for god object
fn calculate_god_object_risk(god_analysis: &GodObjectAnalysis) -> f64 {
    // More responsibilities and methods = higher risk
    let responsibility_risk = god_analysis.responsibility_count as f64 * 10.0;
    let method_risk = (god_analysis.method_count as f64 / 10.0).min(50.0);

    (responsibility_risk + method_risk).min(100.0)
}

/// Calculate impact metrics for god object
fn calculate_god_object_impact(
    file_metrics: &FileDebtMetrics,
    god_analysis: &GodObjectAnalysis,
) -> ImpactMetrics {
    ImpactMetrics {
        complexity_reduction: file_metrics.total_complexity / god_analysis.recommended_splits.len().max(1),
        maintainability_improvement: calculate_maintainability_impact(file_metrics),
        risk_reduction: calculate_god_object_risk(god_analysis),
        effort_estimate: estimate_split_effort(file_metrics, god_analysis),
    }
}

/// Estimate effort to split god object
fn estimate_split_effort(
    file_metrics: &FileDebtMetrics,
    god_analysis: &GodObjectAnalysis,
) -> EffortLevel {
    let total_functions = god_analysis.method_count;
    let total_lines = file_metrics.total_lines;

    if total_functions > 100 || total_lines > 2000 {
        EffortLevel::High
    } else if total_functions > 50 || total_lines > 1000 {
        EffortLevel::Medium
    } else {
        EffortLevel::Low
    }
}
```

#### Phase 2: Integrate into Analysis Pipeline

**Location**: `src/builders/unified_analysis.rs` in `apply_file_analysis_results()`

Modify the function that applies file analysis results to create god object items:

```rust
fn apply_file_analysis_results(
    unified: &mut UnifiedAnalysis,
    processed_files: Vec<ProcessedFileData>,
) {
    for file_data in processed_files {
        // EXISTING: Update god object indicators for functions
        if let Some(god_analysis) = &file_data.god_analysis {
            update_function_god_indicators(unified, &file_data.file_path, god_analysis);

            // NEW: Create file-level god object debt item
            let god_item = create_god_object_debt_item(
                &file_data.file_path,
                &file_data.metrics,
                god_analysis,
            );

            // Add as file-level item (bypasses complexity filtering)
            unified.debt_collection.add_file_item(god_item);
        }

        // ... rest of existing code
    }
}
```

#### Phase 3: Ensure File-Level Items Bypass Complexity Filtering

**Location**: `src/priority/unified_analysis_utils.rs:77-94`

The `add_file_item()` function already exists and correctly bypasses complexity filtering. Verify it works as expected:

```rust
fn add_file_item(&mut self, item: UnifiedDebtItem) {
    // File-level items use simpler filtering - only check score threshold
    let min_score = crate::config::get_minimum_debt_score();

    if item.unified_score.final_score < min_score {
        return;
    }

    // Check for duplicates (file-level items by file path + debt type)
    let is_duplicate = self.file_items.iter().any(|existing| {
        existing.location.file == item.location.file
            && std::mem::discriminant(&existing.debt_type)
                == std::mem::discriminant(&item.debt_type)
    });

    if !is_duplicate {
        self.file_items.push_back(item);
    }
}
```

**Verification**: File-level items should NOT be subject to cyclomatic/cognitive complexity thresholds.

#### Phase 4: TUI Display Support

**Locations**:
- `src/tui/results/list_view.rs` - List view rendering
- `src/tui/results/detail_pages/overview.rs` - Detail view (already has god object support)
- `src/tui/results/grouping.rs` - Tier grouping

**Required Changes**:

1. **Ensure file_items are included in TUI list view**:
   - Merge `file_items` and `items` when building TUI display list
   - Maintain separate tracking for proper filtering/sorting

2. **Display god object icon/indicator**:
   - Use appropriate icon for god objects in list view
   - Show "God Object" or "God Module" based on detection type

3. **Detail view formatting** (likely already works):
   - Verify `src/tui/results/detail_pages/overview.rs` displays god objects correctly
   - Show methods, fields, responsibilities counts
   - Display recommended splits

### Architecture Changes

**Files to modify**:
1. `src/builders/unified_analysis.rs`:
   - Add `create_god_object_debt_item()` and helper functions
   - Modify `apply_file_analysis_results()` to create god object items

2. `src/priority/unified_analysis_utils.rs`:
   - Verify `add_file_item()` correctly bypasses complexity filtering
   - Ensure file-level items are included in sorting/filtering

3. `src/tui/results/list_view.rs`:
   - Merge file-level items into TUI display list
   - Add god object icon/indicator

4. `src/tui/results/detail_pages/overview.rs`:
   - Verify existing god object display code works with new items

**New data structures**:
```rust
// Add to UnifiedDebtItem context variants
pub struct GodObjectContext {
    pub method_count: usize,
    pub field_count: usize,
    pub responsibility_count: usize,
    pub recommended_splits: Vec<String>,
    pub detection_type: DetectionType,
}

// Add to ImpactMetrics
pub struct ImpactMetrics {
    pub complexity_reduction: u32,
    pub maintainability_improvement: f64,
    pub risk_reduction: f64,
    pub effort_estimate: EffortLevel,
}

pub enum EffortLevel {
    Low,
    Medium,
    High,
}
```

### Data Flow

```
1. Analysis Phase:
   GodObjectDetector
   ↓
   GodObjectAnalysis (stored in FileDebtMetrics.god_object_indicators)

2. Unified Item Creation (NEW):
   god_object_indicators
   ↓
   create_god_object_debt_item()
   ↓
   UnifiedDebtItem(DebtType::GodObject)
   ↓
   add_file_item() (bypasses complexity filtering)
   ↓
   UnifiedDebtCollection.file_items

3. TUI Display:
   file_items + items merged
   ↓
   Tier grouping and sorting
   ↓
   List view rendering with god object icons
   ↓
   Detail view shows god object metrics
```

## Dependencies

- **Prerequisites**:
  - Spec 133 (God Object Detection Refinement) - Detection type classification
  - Current `GodObjectDetector` implementation
  - Current `UnifiedDebtCollection` with file_items support

- **Affected Components**:
  - `src/builders/unified_analysis.rs` - Main item creation logic
  - `src/priority/unified_analysis_utils.rs` - Collection management
  - `src/tui/results/list_view.rs` - TUI display
  - `src/tui/results/detail_pages/overview.rs` - Detail view

- **External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **God Object Item Creation Tests**
   ```rust
   #[test]
   fn test_create_god_object_debt_item_from_indicators() {
       let file_metrics = create_test_file_metrics_with_god_object();
       let god_analysis = create_test_god_object_analysis();

       let item = create_god_object_debt_item(
           Path::new("src/main.rs"),
           &file_metrics,
           &god_analysis,
       );

       assert_eq!(item.debt_type, DebtType::GodObject);
       assert_eq!(item.location.file, "src/main.rs");
       assert!(item.unified_score.final_score > 0.0);
       assert_eq!(item.cyclomatic_complexity, 0); // File-level
       assert_eq!(item.cognitive_complexity, 0);  // File-level
   }

   #[test]
   fn test_god_object_bypasses_complexity_filtering() {
       let mut collection = UnifiedDebtCollection::new();
       let god_item = create_god_object_item_with_low_complexity(); // cyclomatic = 0

       collection.add_file_item(god_item.clone());

       // Should be added despite low complexity
       assert_eq!(collection.file_items.len(), 1);
       assert_eq!(collection.file_items[0].debt_type, DebtType::GodObject);
   }

   #[test]
   fn test_god_class_vs_god_file_naming() {
       let god_class_item = create_god_object_item(DetectionType::GodClass);
       let god_file_item = create_god_object_item(DetectionType::GodFile);

       assert!(god_class_item.location.name.contains("God Class"));
       assert!(god_file_item.location.name.contains("God Module"));
   }
   ```

2. **Scoring Tests**
   ```rust
   #[test]
   fn test_god_object_scoring_high_priority() {
       let god_analysis = create_high_impact_god_object(); // Many responsibilities
       let item = create_god_object_debt_item_from_analysis(&god_analysis);

       assert!(item.unified_score.final_score >= 50.0); // Should be Tier 1
       assert_eq!(item.unified_score.tier, 1);
   }

   #[test]
   fn test_god_object_impact_metrics() {
       let file_metrics = create_large_file_metrics(); // 2000 lines, 100 complexity
       let god_analysis = create_god_object_with_splits(3);

       let item = create_god_object_debt_item(&file_metrics, &god_analysis);

       assert!(item.impact_metrics.complexity_reduction > 0);
       assert!(item.impact_metrics.maintainability_improvement > 0.0);
       assert_eq!(item.impact_metrics.effort_estimate, EffortLevel::High);
   }
   ```

### Integration Tests

1. **End-to-End TUI Display Test**
   ```rust
   #[test]
   fn test_god_objects_appear_in_tui() {
       // Run analysis on codebase with known god object
       let analysis = run_analysis_on_test_project();

       // Verify god object items exist in collection
       let god_items: Vec<_> = analysis.debt_collection.file_items.iter()
           .filter(|item| matches!(item.debt_type, DebtType::GodObject))
           .collect();

       assert!(!god_items.is_empty(), "No god object items found");

       // Verify items have correct tier
       for item in god_items {
           assert!(item.unified_score.tier <= 2, "God objects should be high priority");
       }
   }

   #[test]
   fn test_tui_and_cli_output_consistency() {
       let analysis = run_analysis_on_debtmap_itself();

       // Get god objects from TUI collection
       let tui_god_objects = get_god_objects_from_unified_collection(&analysis);

       // Get god objects from legacy CLI output
       let cli_god_objects = get_god_objects_from_legacy_output(&analysis);

       // Should identify same files as god objects
       assert_eq!(tui_god_objects.len(), cli_god_objects.len());

       for (tui_item, cli_item) in tui_god_objects.iter().zip(cli_god_objects.iter()) {
           assert_eq!(tui_item.location.file, cli_item.location.file);
       }
   }
   ```

2. **Filtering Tests**
   ```rust
   #[test]
   fn test_file_items_not_filtered_by_complexity() {
       let config = create_config_with_high_complexity_thresholds(); // min_cyclomatic: 10

       let god_item = create_god_object_item(); // cyclomatic: 0
       let mut collection = UnifiedDebtCollection::new();

       collection.add_file_item(god_item);

       // Should NOT be filtered despite complexity thresholds
       assert_eq!(collection.file_items.len(), 1);
   }
   ```

### Regression Tests

- All existing god object detection tests must pass
- All existing TUI tests must pass
- All existing unified analysis tests must pass
- Verify --no-tui output still works correctly

### Manual Testing

1. **Run debtmap on itself**:
   ```bash
   cargo build
   ./target/debug/debtmap analyze . --tui
   ```

2. **Verify src/main.rs appears as god object in TUI**:
   - Should show in Tier 1 (Critical)
   - Should display "God Object" or "God Module" indicator
   - Detail view should show methods, fields, responsibilities

3. **Compare TUI and --no-tui output**:
   ```bash
   ./target/debug/debtmap analyze . --no-tui > cli_output.txt
   ./target/debug/debtmap analyze . --tui  # Check items match
   ```

4. **Test filtering**:
   - Set high complexity thresholds in config
   - Verify god objects still appear in TUI
   - Verify function-level items are filtered correctly

## Documentation Requirements

### Code Documentation

1. **Function Documentation**:
   ```rust
   /// Creates a UnifiedDebtItem from god object analysis indicators.
   ///
   /// God objects are file-level technical debt items representing files with
   /// too many responsibilities, methods, or fields. They bypass function-level
   /// complexity filtering since they represent architectural issues rather than
   /// individual function complexity.
   ///
   /// # Arguments
   /// * `file_path` - Path to the file containing the god object
   /// * `file_metrics` - File-level metrics (lines, complexity, etc.)
   /// * `god_analysis` - God object detection results
   ///
   /// # Returns
   /// A UnifiedDebtItem with DebtType::GodObject, suitable for file_items collection
   fn create_god_object_debt_item(...) -> UnifiedDebtItem;
   ```

2. **Inline Comments**:
   - Explain why god objects bypass complexity filtering
   - Document scoring algorithm for god object priority
   - Note differences between GodClass and GodFile/GodModule

### User Documentation

Update ARCHITECTURE.md or relevant docs with:
- Explanation of god object detection and display
- How god objects are prioritized vs function-level items
- Differences between God Object (GodClass) and God Module (GodFile)
- Why god objects appear in Tier 1 (Critical)

### Architecture Documentation

Document the dual-path analysis:
- Legacy path: `GodObjectDetector` → `CodeOrganization` → --no-tui output
- Unified path: `GodObjectDetector` → `UnifiedDebtItem(GodObject)` → TUI display
- Eventually deprecate legacy path once unified path is stable

## Implementation Notes

### Complexity vs File-Level Metrics

God objects represent **file-level** architectural issues, not function-level complexity:
- Setting `cyclomatic_complexity = 0` and `cognitive_complexity = 0` is intentional
- This signals "not applicable" for file-level items
- Allows file-level items to bypass function-level filtering

### Scoring Algorithm

God object scores (0-100) are calculated from:
- Responsibility count (higher = worse)
- Method count (higher = worse)
- Field count (higher = worse)
- Lines of code (higher = worse)
- Complexity sum (higher = worse)

Mapping to unified score:
- god_object_score >= 50.0 → Tier 1 (Critical)
- god_object_score >= 30.0 → Tier 2 (High)
- god_object_score < 30.0 → Tier 3 (Medium)

### Detection Type Preservation

Preserve `DetectionType` from god object analysis:
- `GodClass` → Display as "God Object" (class/struct with many methods)
- `GodFile` / `GodModule` → Display as "God Module" (file with many functions)

This aligns with Spec 133's classification refinement.

### Performance Considerations

- God object item creation is O(1) per file with god object
- Typically <10 god objects per codebase
- Negligible performance impact (<1% overhead)
- File-level items stored separately to avoid mixing with function-level items

## Migration and Compatibility

### Breaking Changes

None. This is purely additive:
- Adds god object items to TUI (previously missing)
- Doesn't change --no-tui output
- Doesn't change god object detection logic

### Data Migration

No data migration needed. Existing cached results will be regenerated on next analysis.

### Compatibility Considerations

- Serialization format unchanged (UnifiedDebtItem already supports GodObject)
- TUI already has god object detail view code (src/tui/results/detail_pages/overview.rs)
- No breaking changes to public APIs

### Deprecation Path

Eventually, the legacy god object path (`CodeOrganization` in --no-tui) can be deprecated once:
1. Unified path is proven stable
2. Both outputs are verified identical
3. Users migrated to unified output format

## Success Metrics

1. **Functional Completeness**: God objects appear in TUI 100% of the time they appear in --no-tui
2. **Consistency**: Same god objects identified in both TUI and CLI output
3. **Performance**: <2% increase in analysis time
4. **User Satisfaction**: No user complaints about missing god objects in TUI

## Future Enhancements

Potential improvements beyond this spec:
- Interactive god object splitting wizard in TUI
- Visualization of responsibility distribution
- Suggested module boundaries based on function clustering
- Before/after comparison showing benefits of splitting
- Integration with refactoring tools for automated splitting
