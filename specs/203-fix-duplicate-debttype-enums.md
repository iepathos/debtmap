---
number: 203
title: Fix Duplicate DebtType Enum Definitions
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-06
updated: 2025-12-06
---

# Specification 203: Fix Duplicate DebtType Enum Definitions

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The codebase currently has **two different definitions** of the `DebtType` enum, creating parallel systems where detector outputs are orphaned and never reach users:

1. **`src/core/mod.rs:219-234`** - Simple unit variants:
   ```rust
   #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
   pub enum DebtType {
       Todo,
       Fixme,
       CodeSmell,
       Duplication,
       Complexity,
       Dependency,
       ErrorSwallowing,  // ❌ Unit variant
       ResourceManagement,
       CodeOrganization,
       TestComplexity,
       TestTodo,
       TestDuplication,
       TestQuality,
   }
   ```

2. **`src/priority/mod.rs:164-270`** - Rich struct variants with data:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum DebtType {
       TestingGap { coverage: f64, cyclomatic: u32, cognitive: u32 },
       ComplexityHotspot { cyclomatic: u32, cognitive: u32, ... },
       ErrorSwallowing { pattern: String, context: Option<String> },  // ✓ Struct variant
       // ... 20+ more struct variants
   }
   ```

### The Real Problem: Parallel Systems

The duplicate enums have created **two independent data flow pipelines** that never integrate:

#### **System 1: Legacy Detector Path (Orphaned)**
```
Debt Detectors (error_swallowing, panic_patterns, etc.)
    ↓
Create core::DebtItem with core::DebtType (unit variants)
    ↓
Stored in FileMetrics.debt_items: Vec<DebtItem>
    ↓
[DEAD END - Never consumed by output pipeline]
```

**Location**: `src/debt/error_swallowing.rs:48`, `src/analyzers/rust.rs:321`
```rust
use crate::core::{DebtItem, DebtType, Priority};

self.items.push(DebtItem {
    debt_type: DebtType::ErrorSwallowing,  // ❌ Unit variant from core
    // ...
});
```

#### **System 2: Priority Scoring Path (Active)**
```
FunctionMetrics (from complexity analysis)
    ↓
create_unified_debt_item() in priority/scoring/construction.rs
    ↓
UnifiedDebtItem with priority::DebtType (struct variants)
    ↓
UnifiedAnalysis → Output formatters → User sees results ✓
```

**Location**: `src/priority/scoring/construction.rs:407-484`
```rust
use crate::priority::DebtType;

UnifiedDebtItem {
    debt_type: context.debt_type,  // ✓ Struct variant from priority
    // ...
}
```

### Why This Happens

The code **compiles successfully** because:
1. Both enums are valid Rust code
2. `DebtItem` uses `core::DebtType` - type checks ✓
3. `UnifiedDebtItem` uses `priority::DebtType` - type checks ✓
4. **They never interact**, so no type mismatch occurs

But at runtime:
1. Error swallowing detector creates `DebtItem`s → stored in `FileMetrics.debt_items`
2. Unified analysis creates `UnifiedDebtItem`s directly from `FunctionMetrics` - **never looks at `FileMetrics.debt_items`**
3. Output pipeline only shows `UnifiedDebtItem`s
4. **Result**: Detector output is orphaned - built but never displayed

### Impact

1. **Error swallowing detection completely broken** - Detector runs, creates items, but users see 0 results
2. **Silent failure** - No warning that detection failed, no indication of the problem
3. **False confidence** - Feature appears to exist (has tests, code runs) but doesn't work for users
4. **Wasted engineering effort** - Entire detection system built but never integrated
5. **User confusion** - "Why doesn't debtmap detect error swallowing in my code?"

**Meta-irony**: The error swallowing detector's output gets... error swallowed.

## Objective

Eliminate the duplicate `DebtType` enum definitions and choose an architectural direction to properly integrate detection output into the user-facing pipeline.

After this fix:
1. Single `DebtType` enum definition (source of truth)
2. Error swallowing detection integrated into output pipeline
3. Running debtmap on itself produces visible error swallowing results

## Architectural Decision

We must choose one of two approaches:

### Option A: Eliminate Detector Path (RECOMMENDED)

**Approach**: Remove legacy `DebtItem` creation, integrate detection into unified analysis.

**Rationale**:
- Cleaner single-pipeline architecture
- Error swallowing is a function-level concern (like complexity)
- Avoids conversion overhead between types
- Matches existing pattern for complexity hotspots

**Changes Required**:
1. Error swallowing detector enriches `FunctionMetrics.language_specific` instead of creating `DebtItem`s
2. Priority scoring reads enriched metrics and creates `UnifiedDebtItem`s with `priority::DebtType::ErrorSwallowing`
3. Remove `detect_error_swallowing()` function, replace with metric enrichment
4. Similar changes for other detectors (panic_patterns, async_errors, etc.)

**Pros**:
- Single data flow path
- No type conversion needed
- Consistent with complexity analysis approach
- Easier to maintain long-term

**Cons**:
- Larger refactor scope
- Affects multiple detector modules
- Requires redesigning detector interface

### Option B: Bridge the Two Systems

**Approach**: Keep both systems, create conversion layer.

**Rationale**:
- Smaller initial change
- Preserves existing detector architecture
- Can be done incrementally

**Changes Required**:
1. Implement `From<core::DebtType> for priority::DebtType` conversion
2. Create conversion function: `DebtItem → UnifiedDebtItem`
3. Modify unified analysis to consume `FileMetrics.debt_items`
4. Handle scoring for converted items

**Pros**:
- Smaller immediate scope
- Less architectural disruption
- Preserves detector abstraction

**Cons**:
- Two parallel systems long-term
- Conversion overhead
- More complex architecture
- Duplicate enum definitions remain (requires careful trait management)

### **Decision for This Spec: Option B** (with future Option A)

We'll implement **Option B** initially because:
1. Smaller, more manageable change
2. Can be completed in single spec
3. Proves integration works before larger refactor
4. Option A can follow in spec 204 (detector architecture redesign)

This spec focuses on:
- Consolidating enum definitions
- Creating conversion layer
- Integrating `FileMetrics.debt_items` into output

Future spec 204 will eliminate dual path entirely.

## Requirements

### Functional Requirements

1. **Single DebtType Definition**
   - Remove `core::DebtType` enum definition
   - Re-export `priority::DebtType` from `core` for compatibility:
     ```rust
     // src/core/mod.rs
     pub use crate::priority::DebtType;
     ```
   - All imports resolve to single definition

2. **DebtItem Type Update**
   - Change `DebtItem.debt_type` field from unit enum to struct enum:
     ```rust
     pub struct DebtItem {
         pub debt_type: DebtType,  // Now uses priority::DebtType
         // ...
     }
     ```

3. **Pattern Matching Updates**
   - Update all equality checks to use `matches!`:
     ```rust
     // Before (won't compile)
     if item.debt_type == DebtType::ErrorSwallowing { ... }

     // After
     if matches!(item.debt_type, DebtType::ErrorSwallowing { .. }) { ... }
     ```

4. **Detector Updates**
   - Update each detector to create struct variants:
     ```rust
     // src/debt/error_swallowing.rs
     debt_type: DebtType::ErrorSwallowing {
         pattern: pattern.to_string(),
         context: Some(context.to_string()),
     }
     ```

5. **Display Implementation**
   - Add `Display` impl for `priority::DebtType` to handle all struct variants
   - Add `Display` impl for `ErrorSwallowingPattern`

6. **Integration into Output Pipeline**
   - Modify unified analysis to consume `FileMetrics.debt_items`
   - Create stub `UnifiedDebtItem`s for detector-sourced items
   - Ensure all debt items appear in final output

### Non-Functional Requirements

1. **Type Safety**
   - All code compiles without warnings
   - Exhaustive pattern matching enforced
   - No `as` casts or unsafe conversions

2. **Performance**
   - No performance degradation
   - Minimal conversion overhead

3. **Maintainability**
   - Clear migration path for future detectors
   - Documented architecture decision

## Breaking Changes

### Internal Breaking Changes

**This is NOT a minor change.** Affects 50+ files across the codebase.

1. **Pattern Matching (30+ files)**
   ```rust
   // All these patterns MUST change:
   match item.debt_type {
       DebtType::ErrorSwallowing => { ... }  // ❌ Won't compile
   }

   // To:
   match item.debt_type {
       DebtType::ErrorSwallowing { .. } => { ... }  // ✓ Required
   }
   ```

2. **Equality Comparisons (20+ files)**
   ```rust
   // src/priority/debt_aggregator.rs:219 (test)
   assert_eq!(categorize_debt_type(&DebtType::ErrorSwallowing), ...);

   // Won't compile - ErrorSwallowing now requires fields
   ```

3. **categorize_debt_type Function**
   - Location: `src/priority/debt_aggregator.rs:81-104`
   - Currently matches unit variants
   - Must update to match struct variants with `..` pattern

4. **Suppression Logic**
   - Location: `src/debt/suppression.rs`
   - `is_suppressed()` compares `DebtType` for equality
   - Must implement variant-only comparison (ignore field data)

5. **Test Suite**
   - All detector tests use `DebtType` equality assertions
   - All pattern match tests need updating
   - Integration tests need struct variant construction

### Public API Changes

**IF debtmap is used as a library** (current status unclear):

- `core::DebtType` remains available (re-exported)
- But behavior changes - now has struct variants instead of unit variants
- **Semver: Major version bump required** if library usage exists

### Migration Required

Every module importing `DebtType` needs review:
- ✓ Imports still work (re-export handles)
- ⚠️ Pattern matches need updates
- ⚠️ Equality checks need updates
- ⚠️ Construction needs field data

## Implementation Approach

### Phase 1: Preparation and Analysis

**Identify all affected code:**
```bash
# Find all DebtType usages (expect 100+ matches)
rg "DebtType::" --type rust -l

# Find all pattern matches (expect 50+ matches)
rg "match.*debt_type|=> DebtType::" --type rust -C 3

# Find all equality checks (expect 30+ matches)
rg "debt_type ==|== DebtType::" --type rust

# Find all imports (expect 50+ files)
rg "use.*DebtType" --type rust
```

### Phase 2: Add Display Implementations

**Before any enum changes**, add required trait implementations:

1. **ErrorSwallowingPattern Display**
   ```rust
   // src/debt/error_swallowing.rs
   impl std::fmt::Display for ErrorSwallowingPattern {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
           write!(f, "{}", self.description())
       }
   }
   ```

2. **priority::DebtType Display**
   ```rust
   // src/priority/mod.rs
   impl std::fmt::Display for DebtType {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
           match self {
               DebtType::ErrorSwallowing { pattern, .. } =>
                   write!(f, "Error Swallowing: {}", pattern),
               DebtType::TestingGap { .. } => write!(f, "Testing Gap"),
               DebtType::ComplexityHotspot { .. } => write!(f, "Complexity Hotspot"),
               DebtType::DeadCode { .. } => write!(f, "Dead Code"),
               DebtType::Duplication { .. } => write!(f, "Duplication"),
               DebtType::Risk { .. } => write!(f, "Risk"),
               // ... all 20+ variants
           }
       }
   }
   ```

### Phase 3: Update core::DebtType Reference

1. **Remove enum definition from src/core/mod.rs:218-234**
2. **Add re-export**:
   ```rust
   // src/core/mod.rs (line 218)
   // Re-export from priority module (spec 203)
   pub use crate::priority::DebtType;
   ```

3. **Verify imports still resolve**:
   ```bash
   cargo check
   # Should see errors about missing struct fields - expected!
   ```

### Phase 4: Update categorize_debt_type

Critical function that many modules depend on:

```rust
// src/priority/debt_aggregator.rs:81
pub fn categorize_debt_type(debt_type: &DebtType) -> DebtCategory {
    match debt_type {
        // Match variants, ignore data with ..
        DebtType::Complexity { .. } => DebtCategory::Organization,

        DebtType::Todo { .. } | DebtType::Fixme { .. } => DebtCategory::Organization,
        DebtType::CodeOrganization { .. } => DebtCategory::Organization,
        DebtType::CodeSmell { .. } => DebtCategory::Organization,
        DebtType::Dependency { .. } => DebtCategory::Organization,

        DebtType::TestComplexity { .. }
        | DebtType::TestTodo { .. }
        | DebtType::TestDuplication { .. } => DebtCategory::Testing,
        DebtType::TestQuality { .. } => DebtCategory::Testing,

        DebtType::ErrorSwallowing { .. } => DebtCategory::Resource,
        DebtType::ResourceManagement { .. } => DebtCategory::Resource,

        DebtType::Duplication { .. } => DebtCategory::Duplication,

        // Add all priority::DebtType variants
        DebtType::TestingGap { .. } => DebtCategory::Testing,
        DebtType::ComplexityHotspot { .. } => DebtCategory::Organization,
        DebtType::DeadCode { .. } => DebtCategory::Organization,
        DebtType::Risk { .. } => DebtCategory::Resource,
        DebtType::AllocationInefficiency { .. } => DebtCategory::Resource,
        DebtType::StringConcatenation { .. } => DebtCategory::Resource,
        DebtType::NestedLoops { .. } => DebtCategory::Resource,
        DebtType::BlockingIO { .. } => DebtCategory::Resource,
        DebtType::SuboptimalDataStructure { .. } => DebtCategory::Resource,
        DebtType::GodObject { .. } => DebtCategory::Organization,
        DebtType::GodModule { .. } => DebtCategory::Organization,
        DebtType::FeatureEnvy { .. } => DebtCategory::Organization,
        DebtType::PrimitiveObsession { .. } => DebtCategory::Organization,
        DebtType::MagicValues { .. } => DebtCategory::Organization,
        DebtType::AssertionComplexity { .. } => DebtCategory::Testing,
        DebtType::FlakyTestPattern { .. } => DebtCategory::Testing,
        DebtType::AsyncMisuse { .. } => DebtCategory::Resource,
        DebtType::ResourceLeak { .. } => DebtCategory::Resource,
        DebtType::CollectionInefficiency { .. } => DebtCategory::Resource,
        DebtType::ScatteredType { .. } => DebtCategory::Organization,
        // Add any other variants...
    }
}
```

### Phase 5: Update Detectors to Create Struct Variants

For each detector in `src/debt/`:

**Example: error_swallowing.rs**
```rust
fn add_debt_item(&mut self, line: usize, pattern: ErrorSwallowingPattern, context: &str) {
    // Check suppression
    if let Some(checker) = self.suppression {
        // Update suppression check to use struct variant
        let debt_type_for_check = DebtType::ErrorSwallowing {
            pattern: pattern.to_string(),
            context: Some(context.to_string()),
        };
        if checker.is_suppressed(line, &debt_type_for_check) {
            return;
        }
    }

    let priority = self.determine_priority(&pattern);
    let message = format!("{}: {}", pattern.description(), pattern.remediation());

    self.items.push(DebtItem {
        id: format!("error-swallow-{}-{}", self.current_file.display(), line),
        debt_type: DebtType::ErrorSwallowing {
            pattern: pattern.to_string(),
            context: Some(context.to_string()),
        },
        priority,
        file: self.current_file.to_path_buf(),
        line,
        column: None,
        message,
        context: Some(context.to_string()),
    });
}
```

Repeat for:
- `src/debt/panic_patterns.rs` → Create appropriate struct variants
- `src/debt/async_errors.rs` → Create appropriate struct variants
- `src/debt/error_context.rs` → Create appropriate struct variants
- `src/debt/error_propagation.rs` → Create appropriate struct variants
- `src/debt/patterns.rs` (TODO/FIXME) → Create appropriate struct variants
- `src/debt/smells.rs` → Update to struct variants
- `src/resource/mod.rs` → Update to struct variants
- `src/testing/rust/mod.rs` → Update to struct variants

**Note**: Some detectors may need new `priority::DebtType` variants added if they don't exist yet.

### Phase 6: Update Suppression Logic

```rust
// src/debt/suppression.rs
impl SuppressionContext {
    pub fn is_suppressed(&self, line: usize, debt_type: &DebtType) -> bool {
        // Match on variant type only, ignore field data
        let type_matches = match (debt_type, &self.suppressed_type) {
            (DebtType::ErrorSwallowing { .. }, DebtType::ErrorSwallowing { .. }) => true,
            (DebtType::TestingGap { .. }, DebtType::TestingGap { .. }) => true,
            (DebtType::ComplexityHotspot { .. }, DebtType::ComplexityHotspot { .. }) => true,
            // Add all variants...
            _ => {
                // Use std::mem::discriminant for exhaustive comparison
                std::mem::discriminant(debt_type) == std::mem::discriminant(&self.suppressed_type)
            }
        };

        type_matches && self.line_range.contains(&line)
    }
}
```

### Phase 7: Update All Tests

**Pattern match updates** (30+ test files):
```rust
// Before
assert_eq!(items[0].debt_type, DebtType::ErrorSwallowing);

// After
assert!(matches!(items[0].debt_type, DebtType::ErrorSwallowing { .. }));

// Or with field inspection:
match &items[0].debt_type {
    DebtType::ErrorSwallowing { pattern, context } => {
        assert!(pattern.contains("expected text"));
        assert!(context.is_some());
    }
    _ => panic!("Expected ErrorSwallowing variant"),
}
```

**Test in each detector file**:
- `src/debt/error_swallowing.rs` - 5+ tests
- `src/debt/panic_patterns.rs` - 3+ tests
- `src/debt/async_errors.rs` - 3+ tests
- `tests/debt_grouping_tests.rs`
- `tests/core_display_tests.rs`
- And 20+ more integration tests...

### Phase 8: Run Full Test Suite

```bash
# Run all tests
cargo test --all-features

# Fix compilation errors one by one
# Most will be pattern matching errors - update with `{ .. }`

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all
```

### Phase 9: Verify Integration (Critical!)

This phase ensures detector output reaches users:

1. **Check current state** - verify `FileMetrics.debt_items` is populated:
   ```rust
   // Add debug logging in src/analyzers/rust.rs
   eprintln!("FileMetrics for {:?}: {} debt_items", path, debt_items.len());
   ```

2. **Trace data flow** - verify items flow to output:
   ```rust
   // In src/builders/unified_analysis.rs or equivalent
   // Find where FileMetrics is consumed
   // Ensure debt_items are extracted and converted to UnifiedDebtItem
   ```

3. **Create integration** if missing:
   ```rust
   // In unified analysis construction
   for file_metrics in results.files {
       for debt_item in file_metrics.debt_items {
           // Convert DebtItem → UnifiedDebtItem
           // Add to unified_analysis.items
       }
   }
   ```

4. **Verify output**:
   ```bash
   cargo run -- analyze src/debt/error_swallowing.rs
   # Should show error swallowing items in output
   ```

## Acceptance Criteria

Core requirements:

- [ ] Only one `pub enum DebtType` definition exists in codebase (in `src/priority/mod.rs`)
- [ ] `src/core/mod.rs` re-exports `DebtType` from priority module
- [ ] All 50+ files using `DebtType` compile without errors
- [ ] All pattern matches use `{ .. }` syntax for struct variants
- [ ] `categorize_debt_type()` handles all variants with struct syntax
- [ ] All detectors create struct variants with appropriate field data
- [ ] `Display` impl exists for `priority::DebtType` with all variants
- [ ] `Display` impl exists for `ErrorSwallowingPattern`
- [ ] Suppression logic compares variant types (ignores field data)
- [ ] All unit tests pass (400+ tests)
- [ ] All integration tests pass
- [ ] No clippy warnings
- [ ] No compilation warnings

Verification requirements:

- [ ] **Critical**: Error swallowing detector output appears in user-facing results
- [ ] Run `cargo run -- analyze src/debt/error_swallowing.rs` shows error swallowing items
- [ ] Run `cargo run -- analyze src/` produces debt items for all detector types
- [ ] Verify `FileMetrics.debt_items` is consumed (not orphaned)
- [ ] Verify unified analysis includes detector-sourced items
- [ ] Manual inspection confirms detectors integrated into output pipeline

Performance requirements:

- [ ] No performance regression (run benchmarks if available)
- [ ] Self-analysis completes in reasonable time (<2 minutes for codebase)

## Testing Strategy

### Unit Tests

**Test each updated detector** (example for error_swallowing.rs):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    #[test]
    fn test_if_let_ok_creates_struct_variant() {
        let code = r#"
            fn example() {
                if let Ok(value) = some_function() {
                    println!("{}", value);
                }
            }
        "#;

        let file = parse_str::<File>(code).unwrap();
        let items = detect_error_swallowing(&file, Path::new("test.rs"), None);

        assert_eq!(items.len(), 1);

        // Verify struct variant with fields
        match &items[0].debt_type {
            DebtType::ErrorSwallowing { pattern, context } => {
                assert!(pattern.contains("if let Ok"));
                assert!(context.is_some());
            }
            _ => panic!("Expected ErrorSwallowing struct variant"),
        }
    }

    #[test]
    fn test_categorization_with_struct_variants() {
        use crate::priority::debt_aggregator::{categorize_debt_type, DebtCategory};

        let debt_type = DebtType::ErrorSwallowing {
            pattern: "test".to_string(),
            context: None,
        };

        assert_eq!(categorize_debt_type(&debt_type), DebtCategory::Resource);
    }
}
```

### Integration Tests

**Verify end-to-end flow**:

```rust
#[test]
fn test_detector_output_reaches_users() {
    use tempfile::tempdir;
    use std::fs;

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.rs");

    fs::write(&file_path, r#"
        fn has_errors() {
            let _ = risky_operation();  // Error swallowing
            result.ok();                // Error swallowing
        }
    "#).unwrap();

    // Run analysis
    let results = analyze_project(dir.path()).unwrap();

    // Verify error swallowing items appear in output
    let error_swallowing_items: Vec<_> = results
        .all_items()
        .filter(|item| matches!(item.debt_type, DebtType::ErrorSwallowing { .. }))
        .collect();

    assert!(
        error_swallowing_items.len() >= 2,
        "Expected at least 2 error swallowing detections, found {}",
        error_swallowing_items.len()
    );
}
```

### Manual Verification

After implementation:

```bash
# 1. Verify single enum definition
rg "pub enum DebtType" --type rust
# Should show only src/priority/mod.rs

# 2. Analyze single file with known errors
cargo run -- analyze src/debt/error_swallowing.rs

# Expected: See error swallowing items in output
# If not: detector output not integrated - CRITICAL BUG

# 3. Self-analysis
cargo run -- analyze src/ --output-format json > self_analysis.json

# 4. Count error swallowing detections
jq '[.items[] | select(.debt_type | has("ErrorSwallowing"))] | length' self_analysis.json

# Expected: >0 (if 0, integration failed)

# 5. Verify all detector types appear
jq '[.items[].debt_type | keys[0]] | unique' self_analysis.json

# Should include: ErrorSwallowing, and other detector types
```

## Migration and Compatibility

### For Internal Development

No migration needed beyond this spec's implementation. After merge:
- New code automatically uses struct variants (only option)
- Import paths unchanged (`use crate::core::DebtType` still works)

### For External Users (if library)

**Breaking change notice required:**

```markdown
## Breaking Changes in v0.10.0

### DebtType Enum Consolidated

The `DebtType` enum has been consolidated from unit variants to struct variants:

**Before:**
```rust
let debt = DebtType::ErrorSwallowing;  // Unit variant
```

**After:**
```rust
let debt = DebtType::ErrorSwallowing {
    pattern: "pattern_name".to_string(),
    context: Some("additional context".to_string()),
};
```

**Migration:**
- Pattern matches must use `{ .. }` syntax
- Equality checks must use `matches!` macro or discriminant comparison
- Construction requires field data

See migration guide: docs/migration-0.10.md
```

## Success Metrics

Upon completion:

- ✅ Single `DebtType` enum definition (verified by grep)
- ✅ All 400+ tests pass
- ✅ No clippy warnings
- ✅ Error swallowing detector output visible in user results
- ✅ All detector types integrated into output pipeline
- ✅ No orphaned `DebtItem`s in `FileMetrics`
- ✅ Self-analysis produces detector results
- ✅ API compatibility maintained through re-export

## Follow-up Work

After completing this spec:

1. **Spec 204: Eliminate Dual Pipeline** (Architectural Improvement)
   - Implement Option A: Remove detector path entirely
   - Error swallowing enriches `FunctionMetrics.language_specific`
   - Priority scoring creates all `UnifiedDebtItem`s
   - Cleaner single-pipeline architecture

2. **Spec 205: Enhanced Error Swallowing Detection**
   - Add `.filter_map(Result::ok)` pattern
   - Add `.collect::<Result<Vec<_>>>().ok()` pattern
   - Improve pattern accuracy

3. **All Other Detectors Review**
   - Audit panic_patterns, async_errors, error_context, error_propagation
   - Verify all create appropriate struct variants
   - Check integration into output

## Implementation Notes

### Critical Success Factors

1. **Verify integration EARLY** - Don't assume detector output flows to users
2. **Update tests incrementally** - Don't batch 100+ test updates
3. **Use compiler as guide** - Compilation errors show what needs updating
4. **Test after each phase** - Catch issues early

### Common Pitfalls

1. **Forgetting `..` in patterns**:
   ```rust
   // ❌ Won't compile
   DebtType::ErrorSwallowing => { ... }

   // ✓ Required
   DebtType::ErrorSwallowing { .. } => { ... }
   ```

2. **Using equality on struct variants**:
   ```rust
   // ❌ Won't compile
   if debt_type == DebtType::ErrorSwallowing { ... }

   // ✓ Use matches!
   if matches!(debt_type, DebtType::ErrorSwallowing { .. }) { ... }
   ```

3. **Assuming detector output is integrated**:
   - **Must verify** `FileMetrics.debt_items` reaches output
   - If not integrated, this entire spec only fixes compilation - users still see nothing!

4. **Missing struct variant fields**:
   ```rust
   // ❌ Won't compile
   DebtType::ErrorSwallowing

   // ✓ Required
   DebtType::ErrorSwallowing {
       pattern: "...".to_string(),
       context: Some("...".to_string()),
   }
   ```

### Verification Checklist

After implementation:

- [ ] Run: `rg "pub enum DebtType" --type rust` → Only 1 result
- [ ] Run: `cargo test --all-features` → All pass
- [ ] Run: `cargo clippy --all-targets` → No warnings
- [ ] Run: `cargo run -- analyze src/debt/error_swallowing.rs` → See error swallowing in output
- [ ] Check: `FileMetrics.debt_items` consumption verified in code
- [ ] Verify: All detector types appear in self-analysis output

## References

- **Discovery**: 2025-12-06 evaluation session
- **Root cause**: Parallel data pipelines with orphaned detector output
- **Affected files**: 50+ files import DebtType, 30+ files pattern match
- **Architecture**: Two systems (detector → DebtItem, scoring → UnifiedDebtItem) never integrate
- **Priority rationale**: Entire detection system built but never shown to users - critical waste and false confidence
