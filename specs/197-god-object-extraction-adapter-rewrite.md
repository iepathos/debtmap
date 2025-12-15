---
number: 197
title: God Object Extraction Adapter Rewrite - Per-Struct Analysis
category: foundation
priority: critical
status: draft
dependencies: [201]
created: 2025-12-15
---

# Specification 197: God Object Extraction Adapter Rewrite - Per-Struct Analysis

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 201 (Per-Struct God Object Analysis)

## Context

Debtmap has **two competing code paths** for god object detection:

### Path 1: AST-based Detector (Correct)

Location: `src/organization/god_object/detector.rs`

```rust
// analyze_comprehensive() returns Vec<GodObjectAnalysis>
// One analysis PER STRUCT that qualifies as god object

for (struct_name, type_analysis) in visitor.types {
    // Skip DTOs (no methods)
    if type_analysis.method_count == 0 { continue; }

    // Analyze THIS struct's metrics only
    let method_names = &type_analysis.methods;
    let complexity_sum = calculate_complexity_for_struct(method_names);
    let responsibilities = group_methods_by_responsibility(method_names);

    // Score THIS struct independently
    if god_object_score >= 70.0 {
        analyses.push(GodObjectAnalysis {
            struct_name: Some(struct_name),
            method_count: type_analysis.method_count,  // This struct only
            field_count: type_analysis.field_count,    // This struct only
            responsibilities: behavioral_categories,   // "Parsing", "Validation", etc.
            ...
        });
    }
}
```

**Produces**: Correct per-struct analysis with behavioral responsibility categories.

### Path 2: Extraction Adapter (Wrong - Primary Path)

Location: `src/extraction/adapters/god_object.rs`

```rust
// Returns Option<GodObjectAnalysis> - single file-level analysis

// WRONG: Aggregates ALL types
let total_methods: usize = extracted.impls.iter()
    .map(|i| i.methods.len()).sum();
let total_fields: usize = extracted.structs.iter()
    .map(|s| s.fields.len()).sum();

// WRONG: Uses type/trait names as "responsibilities"
for impl_block in &extracted.impls {
    let name = impl_block.trait_name
        .unwrap_or_else(|| impl_block.type_name.clone());  // "UserMessage", "From", etc.
    responsibilities.push(name);
}

// WRONG: Takes first struct blindly
struct_name: extracted.structs.first().map(|s| s.name.clone()),
```

**Produces**: File-level aggregation with type names as responsibilities (incorrect).

### The Problem

The extraction adapter is the **PRIMARY code path** (used when extracted data is available), but it violates Spec 201's per-struct analysis requirement:

```rust
// In parallel_unified_analysis.rs:991-994
file_metrics.god_object_analysis = if skip_god_object_analysis {
    None
} else {
    // PRIMARY PATH - uses broken adapter
    crate::extraction::adapters::god_object::analyze_god_object(file_path, extracted)
};
```

**Real-world Impact** (analyzing `zed/crates/acp_thread/src/acp_thread.rs`):

| Metric | Adapter Output (Wrong) | Expected (Per-Struct) |
|--------|------------------------|----------------------|
| Location | `UserMessage` line 42 | `AcpThread` line 806 |
| Methods | 106 (all types combined) | 54 (AcpThread only) |
| Fields | 55 (all types combined) | 15 (AcpThread only) |
| Responsibilities | "usermessage", "acpthread", "from" | "Event Handling", "State Management", etc. |

### Following Stillwater Philosophy

Per `../stillwater/PHILOSOPHY.md`:

**1. Pure Core, Imperative Shell**
> Most code mixes business logic with I/O, making it hard to test... The Stillwater Way: separate pure logic from effects.

The adapter should be pure transformation: `ExtractedFileData -> Vec<GodObjectAnalysis>`.

**2. Composition Over Complexity**
> Build complex behavior from simple, composable pieces... Each piece does one thing, is easily testable, is reusable.

The adapter should compose these pure functions:
- `match_impls_to_structs()` - associates impl blocks with their types
- `calculate_struct_metrics()` - computes per-struct metrics
- `classify_responsibilities()` - behavioral categorization
- `score_struct()` - determines if struct is god object

**3. Types Guide, Don't Restrict**
> Use types to make wrong code hard to write, but keep them simple.

Return `Vec<GodObjectAnalysis>` (multiple results) not `Option<GodObjectAnalysis>` (single result).

## Objective

Rewrite the extraction adapter to implement per-struct god object analysis, aligning with the AST-based detector (Spec 201) while maintaining O(n) pure transformations without file I/O.

```rust
// Before (file-level aggregation)
pub fn analyze_god_object(path: &Path, extracted: &ExtractedFileData)
    -> Option<GodObjectAnalysis>

// After (per-struct analysis)
pub fn analyze_god_objects(path: &Path, extracted: &ExtractedFileData)
    -> Vec<GodObjectAnalysis>
```

**Key Changes**:
1. Return `Vec<GodObjectAnalysis>` - one per qualifying struct
2. Match impl blocks to their structs for accurate metrics
3. Use behavioral categorization for responsibilities
4. Identify the actual god object struct(s), not just the first one

## Requirements

### Functional Requirements

1. **Per-Struct Analysis**
   - Iterate each struct in `extracted.structs`
   - Match impl blocks to structs by `type_name`
   - Calculate metrics for each struct independently
   - Skip structs with no impl methods (DTOs)

2. **Accurate Impl-to-Struct Matching**
   - Build `HashMap<String, Vec<&ExtractedImplData>>` mapping type names to impls
   - Handle multiple impl blocks for same type (trait impls + inherent impls)
   - Track which methods belong to which struct

3. **Behavioral Responsibility Classification**
   - Use `group_methods_by_responsibility()` from `classifier.rs`
   - Pass method names (not type names) for classification
   - Return categories like "Parsing", "Validation", "Event Handling"

4. **Correct Struct Identification**
   - Set `struct_name` to the actual struct being analyzed
   - Set `struct_line` to the struct's declaration line
   - Only return structs that qualify as god objects (score >= 70)

5. **File-Level Analysis for Standalone Functions**
   - If no structs but many standalone functions, analyze file-level
   - Use `DetectionType::GodFile` or `DetectionType::GodModule`
   - This matches the AST-based detector's behavior

### Non-Functional Requirements

1. **Pure Transformation**
   - No file I/O, parsing, or side effects
   - Input: `&ExtractedFileData` (already extracted)
   - Output: `Vec<GodObjectAnalysis>` (pure data)

2. **O(n) Complexity**
   - Single pass to build impl-to-struct map
   - Single pass per struct for analysis
   - No nested loops over all impls for each struct

3. **Composable Design**
   - Extract pure helper functions for each transformation step
   - Enable unit testing of each step independently
   - Follow functional pipeline pattern

## Acceptance Criteria

- [ ] `analyze_god_objects()` returns `Vec<GodObjectAnalysis>` (renamed from `analyze_god_object`)
- [ ] Each returned analysis corresponds to a specific struct that qualifies
- [ ] `struct_name` correctly identifies the god object struct (not first struct)
- [ ] `struct_line` matches the struct's declaration line
- [ ] `method_count` reflects only methods in that struct's impl blocks
- [ ] `field_count` reflects only fields in that struct
- [ ] `responsibilities` contains behavioral categories ("Parsing", "Validation", etc.)
- [ ] `responsibility_method_counts` maps behavioral categories to method counts
- [ ] DTOs (structs with 0 impl methods) are skipped
- [ ] File-level analysis used when no structs but >50 standalone functions
- [ ] Existing tests pass (update as needed)
- [ ] New tests verify per-struct behavior

## Technical Details

### Implementation Approach

#### Step 1: Build Impl-to-Struct Map (Pure)

```rust
/// Pure function: build mapping from type names to their impl blocks.
fn build_impl_map(impls: &[ExtractedImplData]) -> HashMap<String, Vec<&ExtractedImplData>> {
    let mut map: HashMap<String, Vec<&ExtractedImplData>> = HashMap::new();
    for impl_block in impls {
        map.entry(impl_block.type_name.clone())
            .or_default()
            .push(impl_block);
    }
    map
}
```

#### Step 2: Calculate Per-Struct Metrics (Pure)

```rust
/// Pure function: calculate metrics for a single struct.
fn calculate_struct_metrics(
    struct_data: &ExtractedStructData,
    impl_blocks: &[&ExtractedImplData],
) -> StructMetrics {
    let method_count: usize = impl_blocks.iter()
        .map(|i| i.methods.len())
        .sum();

    let method_names: Vec<String> = impl_blocks.iter()
        .flat_map(|i| i.methods.iter().map(|m| m.name.clone()))
        .collect();

    StructMetrics {
        name: struct_data.name.clone(),
        line: struct_data.line,
        field_count: struct_data.fields.len(),
        method_count,
        method_names,
    }
}
```

#### Step 3: Classify Responsibilities (Pure)

```rust
/// Pure function: classify methods into behavioral categories.
fn classify_responsibilities(method_names: &[String]) -> HashMap<String, Vec<String>> {
    // Reuse existing behavioral categorization
    crate::organization::god_object::classifier::group_methods_by_responsibility(method_names)
}
```

#### Step 4: Score and Filter (Pure)

```rust
/// Pure function: determine if struct qualifies as god object.
fn is_god_object(metrics: &StructMetrics, responsibilities: &HashMap<String, Vec<String>>) -> bool {
    let thresholds = GodObjectThresholds::default();

    metrics.method_count > thresholds.max_methods
        || metrics.field_count > thresholds.max_fields
        || responsibilities.len() > thresholds.max_traits
}
```

#### Step 5: Compose Pipeline (Pure)

```rust
/// Pure function: analyze extracted data for god object patterns.
pub fn analyze_god_objects(
    _path: &Path,
    extracted: &ExtractedFileData,
) -> Vec<GodObjectAnalysis> {
    // Build impl-to-struct mapping
    let impl_map = build_impl_map(&extracted.impls);

    // Analyze each struct independently
    extracted.structs.iter()
        .filter_map(|struct_data| {
            // Get impl blocks for this struct
            let impl_blocks = impl_map.get(&struct_data.name)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            // Skip DTOs (no methods)
            if impl_blocks.is_empty() || impl_blocks.iter().all(|i| i.methods.is_empty()) {
                return None;
            }

            // Calculate metrics for THIS struct
            let metrics = calculate_struct_metrics(struct_data, impl_blocks);

            // Classify responsibilities by behavior
            let responsibilities = classify_responsibilities(&metrics.method_names);

            // Check if this struct qualifies as god object
            if !is_god_object(&metrics, &responsibilities) {
                return None;
            }

            // Build analysis for this god object
            Some(build_god_object_analysis(struct_data, &metrics, responsibilities))
        })
        .collect()
}
```

### Architecture Changes

1. **Rename Function**
   - `analyze_god_object()` -> `analyze_god_objects()` (plural)
   - Return type: `Option<GodObjectAnalysis>` -> `Vec<GodObjectAnalysis>`

2. **Update Call Sites**
   - `parallel_unified_analysis.rs:994` - handle `Vec` return
   - Select highest-scoring god object for `file_metrics.god_object_analysis`
   - Or consider reporting all god objects

3. **Remove Aggregation Logic**
   - Delete `total_methods: usize = extracted.impls.iter().map(...)` file-level aggregation
   - Delete type-name-based responsibility extraction

### Data Structures

```rust
/// Internal struct for per-struct metric calculation.
struct StructMetrics {
    name: String,
    line: usize,
    field_count: usize,
    method_count: usize,
    method_names: Vec<String>,
}

/// Internal struct for impl block reference.
struct ImplInfo<'a> {
    type_name: &'a str,
    trait_name: Option<&'a str>,
    methods: &'a [MethodInfo],
}
```

### APIs and Interfaces

```rust
// Public API change
pub fn analyze_god_objects(
    path: &Path,
    extracted: &ExtractedFileData,
) -> Vec<GodObjectAnalysis>;

// Backward-compatible wrapper (optional)
pub fn analyze_god_object(
    path: &Path,
    extracted: &ExtractedFileData,
) -> Option<GodObjectAnalysis> {
    analyze_god_objects(path, extracted)
        .into_iter()
        .max_by(|a, b| a.god_object_score.cmp(&b.god_object_score))
}
```

## Dependencies

- **Prerequisites**:
  - Spec 201 (Per-Struct God Object Analysis) - defines the expected behavior
- **Affected Components**:
  - `src/extraction/adapters/god_object.rs` - primary changes
  - `src/builders/parallel_unified_analysis.rs` - call site update
  - `src/priority/file_metrics.rs` - may need to handle multiple god objects
- **External Dependencies**: None (reuses existing `classifier.rs`)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_per_struct_analysis_multiple_structs() {
    let extracted = ExtractedFileData {
        structs: vec![
            ExtractedStructData { name: "SmallDTO".into(), fields: vec![...], line: 1 },
            ExtractedStructData { name: "GodClass".into(), fields: vec![...], line: 100 },
        ],
        impls: vec![
            // No impl for SmallDTO (it's a DTO)
            ExtractedImplData { type_name: "GodClass".into(), methods: vec![...] },
        ],
        ..
    };

    let results = analyze_god_objects(&Path::new("test.rs"), &extracted);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].struct_name, Some("GodClass".to_string()));
    assert_eq!(results[0].struct_line, Some(100));
}

#[test]
fn test_behavioral_responsibilities() {
    let extracted = create_extracted_with_methods(&[
        "parse_json", "validate_input", "render_output", "handle_event"
    ]);

    let results = analyze_god_objects(&Path::new("test.rs"), &extracted);

    let responsibilities = &results[0].responsibilities;
    assert!(responsibilities.contains(&"Parsing".to_string()));
    assert!(responsibilities.contains(&"Validation".to_string()));
    assert!(responsibilities.contains(&"Rendering".to_string()));
    assert!(responsibilities.contains(&"Event Handling".to_string()));
}

#[test]
fn test_dto_skipped() {
    let extracted = ExtractedFileData {
        structs: vec![
            ExtractedStructData { name: "DataOnly".into(), fields: vec![field1, field2], line: 1 },
        ],
        impls: vec![], // No impl blocks
        ..
    };

    let results = analyze_god_objects(&Path::new("test.rs"), &extracted);

    assert!(results.is_empty(), "DTOs should not be flagged");
}
```

### Integration Tests

```rust
#[test]
fn test_real_file_acp_thread_structure() {
    // Test with extracted data similar to zed's acp_thread.rs
    let extracted = create_acp_thread_like_structure();

    let results = analyze_god_objects(&Path::new("acp_thread.rs"), &extracted);

    // Should identify AcpThread as god object, not UserMessage
    assert!(results.iter().any(|r| r.struct_name == Some("AcpThread".to_string())));
    assert!(results.iter().all(|r| r.struct_name != Some("UserMessage".to_string())));
}
```

### Property Tests

```rust
proptest! {
    #[test]
    fn prop_method_count_equals_sum_of_impl_methods(
        structs in vec(struct_strategy(), 1..5),
        impls in vec(impl_strategy(), 0..20),
    ) {
        let extracted = ExtractedFileData { structs, impls, .. };
        let results = analyze_god_objects(&Path::new("test.rs"), &extracted);

        for result in results {
            let struct_name = result.struct_name.unwrap();
            let expected_count: usize = impls.iter()
                .filter(|i| i.type_name == struct_name)
                .map(|i| i.methods.len())
                .sum();

            prop_assert_eq!(result.method_count, expected_count);
        }
    }
}
```

## Documentation Requirements

- **Code Documentation**:
  - Document each pure helper function with examples
  - Explain the pipeline composition pattern
- **User Documentation**: None required (internal change)
- **Architecture Updates**:
  - Update god object detection section in ARCHITECTURE.md
  - Document the two analysis paths and when each is used

## Implementation Notes

### Stillwater Principles Applied

1. **Pure Core**: All helper functions are pure transformations
2. **Composition**: Pipeline composes small, testable functions
3. **Types Guide**: `Vec<GodObjectAnalysis>` makes multiple results explicit
4. **Pragmatism**: Reuses existing `classifier.rs` behavioral categorization

### Migration Strategy

1. **Phase 1**: Implement new `analyze_god_objects()` alongside existing function
2. **Phase 2**: Update call sites to use new function
3. **Phase 3**: Deprecate and remove old `analyze_god_object()` (or keep as wrapper)

### Performance Considerations

- O(n) complexity maintained - no nested loops
- Single-pass impl map construction
- Reuses existing optimized behavioral categorizer
- No file I/O (pure transformation of already-extracted data)

## Migration and Compatibility

### Breaking Changes

- Function renamed: `analyze_god_object` -> `analyze_god_objects`
- Return type changed: `Option<GodObjectAnalysis>` -> `Vec<GodObjectAnalysis>`

### Migration Path

1. Add backward-compatible wrapper that returns highest-scoring god object
2. Update internal call sites to use new function
3. Eventually remove wrapper after confirming all uses updated

### Compatibility

- Existing `GodObjectAnalysis` struct unchanged
- Existing thresholds and scoring logic reused
- Output format to users unchanged (still shows single primary god object)
