---
number: 253
title: Unify God Object DebtType Variants
category: foundation
priority: medium
status: draft
dependencies: [252]
created: 2025-12-07
---

# Specification 253: Unify God Object DebtType Variants

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 252 (unified action recommendations)

## Context

Spec 252 unified the recommendation text for god objects, but left an artificial separation in the underlying data structure. Currently, we have:

1. **Three detection types** in `DetectionType` enum:
   - `GodClass` - struct with excessive impl methods
   - `GodFile` - file with excessive standalone functions (no structs)
   - `GodModule` - hybrid file with structs AND many standalone functions

2. **Two DebtType variants** that map to these detection types:
   - `DebtType::GodObject` (from `GodClass`) - has `methods`, `fields`, `responsibilities`, `god_object_score`
   - `DebtType::GodModule` (from `GodFile` and `GodModule`) - has `functions`, `lines`, `responsibilities`

This creates several issues:

### Data Duplication
All three detection types store their complete analysis in the same field: `UnifiedDebtItem.god_object_indicators: Option<GodObjectAnalysis>`. This analysis contains:
- `method_count` (used for all three types)
- `field_count` (present for all types, but only copied to `DebtType::GodObject`)
- `god_object_score` (present for all types, but only copied to `DebtType::GodObject`)
- `lines_of_code` (present for all types, but only copied to `DebtType::GodModule`)
- `detection_type` (tells us which type it is)

### Inconsistent TUI Display
The TUI detail view (src/tui/results/detail_pages/dependencies.rs:59-100) only shows responsibilities for `DebtType::GodObject` because:
- It checks `if let Some(indicators) = &item.god_object_indicators`
- But god modules have the same `god_object_indicators` data
- The TUI can't display it consistently because the `DebtType` enum shape is different

### Artificial Distinction in Code
The mapping code (src/builders/unified_analysis.rs:1580-1592) shows the problem:
```rust
let debt_type = match god_analysis.detection_type {
    DetectionType::GodClass => DebtType::GodObject {
        methods: god_analysis.method_count as u32,
        fields: god_analysis.field_count as u32,
        responsibilities: god_analysis.responsibility_count as u32,
        god_object_score: god_analysis.god_object_score,
    },
    DetectionType::GodFile | DetectionType::GodModule => DebtType::GodModule {
        functions: god_analysis.method_count as u32,  // SAME source!
        lines: god_analysis.lines_of_code as u32,
        responsibilities: god_analysis.responsibility_count as u32,
        // Missing: god_object_score, fields (even though they exist!)
    },
};
```

The `method_count` from `GodObjectAnalysis` is copied to:
- `methods` field in `DebtType::GodObject`
- `functions` field in `DebtType::GodModule`

But they're the same metric! The naming difference is purely cosmetic.

## Objective

Complete the unification started in spec 252 by eliminating the artificial `DebtType::GodObject` vs `DebtType::GodModule` distinction. Use a single `DebtType::GodObject` variant that can represent all three detection types (GodClass, GodFile, GodModule).

The `GodObjectAnalysis.detection_type` field already provides all the information needed to distinguish between class, file, and module god objects - we don't need separate enum variants.

## Requirements

### Functional Requirements

1. **Single DebtType variant for all god object detections**
   - Replace both `DebtType::GodObject` and `DebtType::GodModule` with a single unified variant
   - Use `DebtType::GodObject` as the canonical name (since all three are god object patterns)
   - Include all metrics with optional fields where appropriate

2. **Preserve detection type information**
   - Keep `GodObjectAnalysis.detection_type` field as source of truth
   - Use this field in TUI and formatters to customize display
   - Don't lose the distinction between class/file/module in output

3. **Consistent metric availability**
   - All god objects should expose the same metrics
   - Use Option<T> for fields that may not apply to all detection types
   - Field count is N/A for GodFile and GodModule (use None)

4. **Unified TUI display**
   - Dependencies page should show responsibilities for all god object types
   - Overview page should handle all types with same code path
   - Use detection_type to customize labels ("methods" vs "functions")

### Non-Functional Requirements

1. **Backward compatibility in serialization**
   - Existing JSON output should still deserialize (with migration)
   - Use serde attributes to handle field renames if needed
   - Document migration path for consumers

2. **Code simplification**
   - Remove duplicate handling code for GodObject vs GodModule
   - Consolidate TUI rendering logic
   - Simplify formatter pattern matching

3. **Type safety**
   - Use Rust's type system to prevent invalid states
   - Make impossible states unrepresentable where practical
   - Clear documentation on when fields are None

## Acceptance Criteria

- [ ] Single `DebtType::GodObject` variant replaces both old variants
- [ ] Variant includes all metrics: `methods`, `fields`, `responsibilities`, `god_object_score`, `lines`
- [ ] `fields` field is `Option<u32>` (Some for GodClass, None for GodFile/GodModule)
- [ ] Mapping code uses `detection_type` to populate fields appropriately
- [ ] TUI dependencies page shows responsibilities for all god object types
- [ ] TUI overview page uses detection_type to customize labels
- [ ] All existing tests pass with updated enum variant
- [ ] Formatter code handles all three detection types uniformly
- [ ] No loss of information - all metrics still accessible
- [ ] Documentation updated to explain unified structure

## Technical Details

### Implementation Approach

#### 1. Update DebtType Enum (src/priority/mod.rs)

**Current:**
```rust
GodObject {
    methods: u32,
    fields: u32,
    responsibilities: u32,
    god_object_score: Score0To100,
},
GodModule {
    functions: u32,
    lines: u32,
    responsibilities: u32,
},
```

**New:**
```rust
GodObject {
    methods: u32,              // Always present (class methods or module functions)
    fields: Option<u32>,       // Some for GodClass, None for GodFile/GodModule
    responsibilities: u32,
    god_object_score: Score0To100,
    lines: u32,                // Total LOC (useful for all types)
},
```

#### 2. Update Mapping Logic (src/builders/unified_analysis.rs)

**Current:**
```rust
let debt_type = match god_analysis.detection_type {
    DetectionType::GodClass => DebtType::GodObject { ... },
    DetectionType::GodFile | DetectionType::GodModule => DebtType::GodModule { ... },
};
```

**New:**
```rust
let debt_type = DebtType::GodObject {
    methods: god_analysis.method_count as u32,
    fields: match god_analysis.detection_type {
        DetectionType::GodClass => Some(god_analysis.field_count as u32),
        DetectionType::GodFile | DetectionType::GodModule => None,
    },
    responsibilities: god_analysis.responsibility_count as u32,
    god_object_score: god_analysis.god_object_score,
    lines: god_analysis.lines_of_code as u32,
};
```

#### 3. Update TUI Overview Page (src/tui/results/detail_pages/overview.rs)

**Current approach:**
- Different match arms for `DebtType::GodObject` and `DebtType::GodModule`
- Different section headers ("god object structure" vs "god module structure")

**New approach:**
- Single match arm for `DebtType::GodObject`
- Use `item.god_object_indicators.detection_type` to customize labels:
  ```rust
  DebtType::GodObject { methods, fields, responsibilities, .. } => {
      let detection_type = item.god_object_indicators
          .as_ref()
          .map(|i| &i.detection_type);

      let header = match detection_type {
          Some(DetectionType::GodClass) => "god object structure",
          Some(DetectionType::GodFile) => "god file structure",
          Some(DetectionType::GodModule) => "god module structure",
          None => "god object structure",
      };

      add_section_header(&mut lines, header, theme);

      let method_label = match detection_type {
          Some(DetectionType::GodClass) => "methods",
          _ => "functions",
      };
      add_label_value(&mut lines, method_label, methods.to_string(), theme, area.width);

      if let Some(field_count) = fields {
          add_label_value(&mut lines, "fields", field_count.to_string(), theme, area.width);
      }

      // ... rest of display logic
  }
  ```

#### 4. Update TUI Dependencies Page (src/tui/results/detail_pages/dependencies.rs)

**Current:**
- Only shows responsibilities for `DebtType::GodObject`
- Ignores god modules even though they have the same data

**New:**
- Show responsibilities for all god objects
- Already checks `god_object_indicators`, so should work automatically once DebtType is unified

#### 5. Update List View (src/tui/results/list_view.rs)

**Current:**
- Different match arms for GodObject and GodModule
- Different display formats

**New:**
- Single match arm with detection_type-based customization
- Consistent display format with appropriate labels

#### 6. Update Formatters

Files to update:
- `src/priority/formatter/pure.rs`
- `src/priority/formatter_markdown.rs`
- `src/io/writers/enhanced_markdown/debt_writer.rs`
- Any other formatter code that pattern matches on DebtType

Pattern:
- Remove separate GodModule match arms
- Handle all god objects in single arm
- Use detection_type for customization where needed

#### 7. Update Hash Implementation

The `DebtType` enum has a custom `Hash` implementation that must be updated to handle the new structure:

```rust
DebtType::GodObject { methods, fields, responsibilities, god_object_score, lines } => {
    methods.hash(state);
    fields.hash(state);  // Option<u32> is Hashable
    responsibilities.hash(state);
    god_object_score.value().to_bits().hash(state);
    lines.hash(state);
}
```

### Data Structures

**Before:**
```rust
// Two separate variants with different shapes
enum DebtType {
    GodObject { methods, fields, responsibilities, god_object_score },
    GodModule { functions, lines, responsibilities },
}
```

**After:**
```rust
// Single variant with optional fields
enum DebtType {
    GodObject {
        methods: u32,           // Unified: class methods or module functions
        fields: Option<u32>,    // Some for classes, None for files/modules
        responsibilities: u32,
        god_object_score: Score0To100,
        lines: u32,             // Total LOC
    },
}
```

### Migration Strategy

For existing JSON files that use the old format:

1. **Serde migration**
   - Use `#[serde(alias = "GodModule")]` if needed for compatibility
   - Consider a custom deserializer that handles both old and new formats
   - Document that old GodModule will deserialize as GodObject with fields=None

2. **Test data updates**
   - Update all test fixtures to use new format
   - Add migration tests that verify old data still loads

3. **Comparison tool updates**
   - Update comparison logic to handle old vs new formats
   - Treat GodModule in old data as equivalent to GodObject with fields=None

## Dependencies

- **Prerequisites**: Spec 252 (unified action recommendations) - already implemented
- **Affected Components**:
  - `src/priority/mod.rs` - DebtType enum definition
  - `src/builders/unified_analysis.rs` - god object creation
  - `src/tui/results/detail_pages/overview.rs` - detail view
  - `src/tui/results/detail_pages/dependencies.rs` - dependencies view
  - `src/tui/results/list_view.rs` - list view
  - All formatter modules
  - All god object-related tests
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **DebtType construction tests**
   - Test creating GodObject for each detection type
   - Verify fields is Some for GodClass
   - Verify fields is None for GodFile and GodModule
   - Test Hash and Eq implementations

2. **Mapping tests**
   - Test god_analysis → DebtType conversion
   - Verify all fields populated correctly
   - Test all three detection types

### Integration Tests

1. **TUI rendering tests**
   - Test overview page displays all types correctly
   - Test dependencies page shows responsibilities for all types
   - Test list view uses appropriate labels
   - Verify detection_type-based customization works

2. **Formatter tests**
   - Test markdown output for all three types
   - Test JSON serialization/deserialization
   - Test enhanced markdown formatting

3. **God object analysis tests**
   - Test end-to-end analysis for GodClass
   - Test end-to-end analysis for GodFile
   - Test end-to-end analysis for GodModule
   - Verify no loss of information

### Migration Tests

1. **Deserialization compatibility**
   - Test loading old JSON with GodModule variant
   - Test loading old JSON with GodObject variant
   - Verify data migrates correctly

2. **Comparison tool tests**
   - Test comparing old vs new format
   - Verify semantic equivalence detected
   - Test mixed old/new comparisons

## Documentation Requirements

### Code Documentation

1. **DebtType enum**
   - Document that GodObject represents all god object detection types
   - Explain when fields is Some vs None
   - Note that detection_type in god_object_indicators provides type distinction

2. **Mapping functions**
   - Document the detection_type → fields logic
   - Explain why fields is optional

3. **TUI components**
   - Document how to use detection_type for customization
   - Provide examples of label customization

### User Documentation

1. **Update book/src/architecture.md**
   - Explain unified god object representation
   - Document the three detection types
   - Show how to distinguish them in output

2. **Migration guide** (if needed)
   - Document changes to JSON format
   - Explain compatibility considerations
   - Provide migration examples

### Architecture Updates

Update ARCHITECTURE.md to reflect:
- Unified god object representation
- Single DebtType variant for all god object types
- Detection type as source of truth for customization

## Implementation Notes

### Key Principles

1. **Detection type is source of truth**
   - `GodObjectAnalysis.detection_type` tells us what kind of god object
   - DebtType enum variant doesn't need to encode this distinction
   - Use detection_type for display customization

2. **Avoid premature abstraction**
   - Keep the solution simple
   - Use Option<u32> for fields rather than complex type system
   - Let detection_type drive behavior, not enum shape

3. **Preserve all information**
   - Don't lose any metrics in the unification
   - All data from GodObjectAnalysis should remain accessible
   - TUI should be able to show everything it could before

### Potential Gotchas

1. **Pattern matching exhaustiveness**
   - Removing GodModule variant will cause compile errors in all match statements
   - Use compiler errors as checklist of places to update
   - Ensure all formatters and TUI components are updated

2. **Serialization compatibility**
   - Old JSON with GodModule will fail to deserialize by default
   - Need explicit migration strategy or serde attributes
   - Test with real-world saved analysis files

3. **Test fixture updates**
   - Many tests create GodModule instances
   - All need to be updated to GodObject with fields=None
   - Use compiler to find them all

4. **Display string consistency**
   - Ensure "God Object", "God File", "God Module" labels still appear correctly
   - Use detection_type to customize display
   - Don't lose the user-facing distinction

## Migration and Compatibility

### Breaking Changes

1. **JSON format change**
   - Old: `{"GodModule": {"functions": 100, ...}}`
   - New: `{"GodObject": {"methods": 100, "fields": null, ...}}`
   - Impact: Saved analysis files won't deserialize

### Compatibility Strategy

1. **Option A: Serde migration** (Recommended)
   ```rust
   #[derive(Deserialize)]
   #[serde(untagged)]
   enum DebtTypeCompat {
       New(DebtType),
       OldGodModule {
           functions: u32,
           lines: u32,
           responsibilities: u32,
       },
   }
   ```
   - Deserialize old format and convert to new
   - Transparent to users
   - Requires custom deserializer

2. **Option B: Breaking change with migration tool**
   - Provide CLI tool to migrate old JSON files
   - Document the breaking change
   - Simpler implementation but requires user action

3. **Option C: Dual deserialization** (Simplest)
   - Use `#[serde(alias = "GodModule")]` on GodObject variant
   - Map old field names to new ones
   - No custom deserializer needed

**Recommendation**: Start with Option C for simplicity. Add Option A if real-world migration issues arise.

### Version Compatibility

- Document minimum version for new format
- Note when old format support will be removed
- Provide migration examples in CHANGELOG

## Success Metrics

1. **Code reduction**
   - Remove duplicate handling logic for GodObject vs GodModule
   - Reduce pattern matching complexity in formatters
   - Consolidate TUI rendering paths

2. **Consistency improvement**
   - All god objects handled uniformly
   - TUI shows same information for all types
   - Formatters use consistent logic

3. **Information preservation**
   - All metrics from old format still accessible
   - No loss of distinction between class/file/module
   - TUI can still customize display per detection type

## Future Work

After this unification:

1. **Further TUI enhancements**
   - Consider showing module_structure details for god modules
   - Display component breakdown from ModuleStructure
   - Show facade_info for well-organized god objects

2. **Recommendation improvements**
   - Use detection_type to further customize recommendations
   - Provide type-specific refactoring guidance
   - Generate type-specific action steps

3. **Pattern detection refinement**
   - Improve detection criteria based on real-world usage
   - Add more sophisticated god module detection
   - Better distinguish healthy modules from god modules
