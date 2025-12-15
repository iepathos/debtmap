# Spec 207: Fix Per-Struct LOC Calculation for God Object Detection

## Problem Statement

The per-struct LOC (Lines of Code) calculation in god object detection only counts lines within the struct definition itself, not the associated impl blocks. This results in artificially low LOC values that affect scoring accuracy.

### Current Behavior

```rust
// In detector.rs:analyze_single_struct
let lines_of_code = type_analysis
    .location
    .end_line
    .unwrap_or(type_analysis.location.line)
    .saturating_sub(type_analysis.location.line)
    + 1;
```

For a struct like:
```rust
pub struct ApplicationManager {  // line 2
    db: Database,
    mailer: Mailer,
    // ... 8 more fields
}  // line 13

impl ApplicationManager {  // line 15
    pub fn parse_json(&self) { ... }
    pub fn render_html(&self) { ... }
    // ... 25 more methods
}  // line 65
```

**Current result**: `loc = 13 - 2 + 1 = 12` (only struct definition)
**Expected result**: `loc = 65 - 2 + 1 = 64` (struct + impl blocks)

### Impact

- God object scores are artificially low because LOC is a factor in the scoring formula
- Large structs with many impl methods may not reach the score threshold of 70
- Example: A struct with 26 methods, 11 fields, and 8 responsibilities only scored 50 due to low LOC

## Solution Design

### Approach: Include Impl Block LOC in TypeAnalysis

Modify `TypeVisitor` to track impl block locations and aggregate LOC across struct definition + all impl blocks.

### Data Structure Changes

```rust
// In ast_visitor.rs
pub struct TypeAnalysis {
    pub name: String,
    pub methods: Vec<String>,
    pub method_count: usize,
    pub field_count: usize,
    pub location: Location,
    // NEW: Track impl block locations
    pub impl_locations: Vec<Location>,
}
```

### Implementation Changes

#### 1. TypeVisitor: Track impl blocks (ast_visitor.rs)

```rust
impl<'ast> Visit<'ast> for TypeVisitor {
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        // Get the type name this impl is for
        if let syn::Type::Path(type_path) = &*item.self_ty {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();

                // Record impl block location for this type
                if let Some(type_analysis) = self.types.get_mut(&type_name) {
                    let impl_location = Location {
                        line: item.impl_token.span.start().line,
                        end_line: Some(item.brace_token.span.close().line),
                        // ... other fields
                    };
                    type_analysis.impl_locations.push(impl_location);
                }
            }
        }

        // Continue visiting to collect methods
        syn::visit::visit_item_impl(self, item);
    }
}
```

#### 2. Detector: Calculate total LOC (detector.rs)

```rust
fn analyze_single_struct(...) -> Option<GodObjectAnalysis> {
    // Calculate LOC including impl blocks
    let struct_loc = type_analysis
        .location
        .end_line
        .unwrap_or(type_analysis.location.line)
        .saturating_sub(type_analysis.location.line)
        + 1;

    let impl_loc: usize = type_analysis
        .impl_locations
        .iter()
        .map(|loc| {
            loc.end_line
                .unwrap_or(loc.line)
                .saturating_sub(loc.line)
                + 1
        })
        .sum();

    let lines_of_code = struct_loc + impl_loc;
    // ...
}
```

### Alternative Approach: Use File-Level LOC Estimate

If tracking impl blocks is too complex, use a heuristic:

```rust
// Estimate LOC from method count (average ~5 lines per method)
let estimated_impl_loc = type_analysis.method_count * 5;
let struct_loc = /* existing calculation */;
let lines_of_code = struct_loc + estimated_impl_loc;
```

This is simpler but less accurate.

## Testing

### Unit Tests

```rust
#[test]
fn test_loc_includes_impl_blocks() {
    let content = r#"
pub struct Foo {
    a: i32,
    b: i32,
}

impl Foo {
    pub fn method1(&self) -> i32 {
        self.a + self.b
    }

    pub fn method2(&self) -> i32 {
        self.a * self.b
    }
}
"#;

    let ast = syn::parse_file(content).expect("parse");
    let mut visitor = TypeVisitor::new();
    visitor.visit_file(&ast);

    let foo = visitor.types.get("Foo").unwrap();
    // Struct: lines 2-5 (4 lines)
    // Impl: lines 7-16 (10 lines)
    // Total: 14 lines

    // After fix, LOC should be ~14, not just 4
    let total_loc = calculate_total_loc(foo);
    assert!(total_loc >= 10, "LOC should include impl blocks, got {}", total_loc);
}

#[test]
fn test_loc_with_multiple_impl_blocks() {
    let content = r#"
pub struct Bar { a: i32 }

impl Bar {
    pub fn new() -> Self { Self { a: 0 } }
}

impl Default for Bar {
    fn default() -> Self { Self::new() }
}

impl Clone for Bar {
    fn clone(&self) -> Self { Self { a: self.a } }
}
"#;

    let ast = syn::parse_file(content).expect("parse");
    let mut visitor = TypeVisitor::new();
    visitor.visit_file(&ast);

    let bar = visitor.types.get("Bar").unwrap();
    // Should aggregate LOC from all 3 impl blocks
    assert!(bar.impl_locations.len() >= 1, "Should track impl blocks");
}
```

### Integration Tests

Verify that the scoring threshold test now passes:

```rust
#[test]
fn test_large_god_object_is_flagged() {
    // A struct with 25+ methods across many domains
    let content = /* ApplicationManager test case */;

    let detector = GodObjectDetector::with_source_content(content);
    let analyses = detector.analyze_comprehensive(Path::new("test.rs"), &ast);

    // With correct LOC, score should be >= 70
    assert!(!analyses.is_empty(), "Large god object should be flagged");
}
```

## Success Criteria

1. LOC calculation includes both struct definition and impl blocks
2. Existing tests continue to pass
3. Large structs with many methods score appropriately
4. No performance regression in god object detection

## Risks

- May flag more structs as god objects (correct behavior)
- Need to handle multiple impl blocks for the same type
- Trait implementations might inflate LOC (may need filtering)

## Implementation Order

1. Add `impl_locations` field to `TypeAnalysis`
2. Update `TypeVisitor` to track impl block locations
3. Update `analyze_single_struct` to calculate total LOC
4. Add unit tests for LOC calculation
5. Verify integration test passes
6. Run full test suite

## Estimated Effort

- Implementation: ~2 hours
- Testing: ~1 hour
- Total: ~3 hours
