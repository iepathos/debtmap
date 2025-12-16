---
number: 214
title: Test Code Exclusion from God Object LOC Metrics
category: optimization
priority: medium
status: draft
dependencies: [207]
created: 2025-12-15
---

# Specification 214: Test Code Exclusion from God Object LOC Metrics

**Category**: optimization
**Priority**: medium (P1)
**Status**: draft
**Dependencies**: Spec 207 (LOC Calculation Fix)

## Context

The current God Object detection includes test code in Lines of Code (LOC) metrics, inflating scores for well-tested modules. This causes false positives where test coverage is penalized.

### Current Problem

```rust
// call_resolution.rs - 867 lines total
pub struct CallResolver<'a> {  // Production code starts
    call_graph: &'a CallGraph,
    current_file: &'a PathBuf,
    function_index: HashMap<String, Vec<FunctionId>>,
}

impl<'a> CallResolver<'a> {
    // ... 580 lines of production code ...
}  // Production code ends at line ~600

#[cfg(test)]
mod tests {
    // ... 250+ lines of tests ...
    // This inflates LOC by 40%!
}
```

Current metrics report:
- LOC: 867 (includes tests)
- Size factor contribution: High

Actual production metrics should be:
- LOC: ~620 (production only)
- Size factor contribution: Medium

### Why This Matters

1. **Penalizes Good Practices**: Well-tested code gets flagged more often
2. **Misleading Metrics**: LOC should reflect production complexity, not test coverage
3. **Inconsistent Reporting**: Tests are excluded from function count but included in LOC

### Impact on Real Code

The `CallResolver` file:
- Total lines: 867
- Test lines: ~250 (29%)
- Production lines: ~620

With tests: LOC factor = 867/600 = 1.44 (above threshold)
Without tests: LOC factor = 620/600 = 1.03 (at threshold)

## Objective

Exclude test code from LOC calculations in God Object detection to:
1. Accurately measure production code complexity
2. Not penalize comprehensive test coverage
3. Provide consistent metrics across all dimensions

## Requirements

### Functional Requirements

1. **Test Code Detection**: Identify test code regions:
   - `#[cfg(test)]` modules
   - `#[test]` attributed functions
   - Files in `tests/` directory
   - `mod tests { }` blocks

2. **LOC Calculation Fix**: Exclude detected test regions from LOC:
   - Calculate production LOC = Total LOC - Test LOC
   - Use production LOC in god object scoring formula

3. **Reporting Enhancement**:
   - Show "LOC: 620 (867 with tests)" in output
   - Provide separate test coverage indicator if useful

4. **Configuration Option**:
   - `--include-test-loc` flag for backwards compatibility
   - Default: exclude test code from LOC

### Non-Functional Requirements

- Detection must be efficient (line-level granularity not needed)
- Should work with standard Rust test patterns
- Must produce deterministic results

## Acceptance Criteria

- [ ] `#[cfg(test)]` modules are detected and excluded from LOC
- [ ] Nested test modules are properly detected
- [ ] Production LOC is used in god object scoring
- [ ] Output shows both total and production LOC
- [ ] `CallResolver` LOC drops from 867 to ~620
- [ ] Files with 50%+ test code show significant score reduction
- [ ] Existing tests continue to pass

## Technical Details

### Implementation Approach

#### 1. Test Region Detection

```rust
/// Represents a region of test code in a file
#[derive(Debug, Clone)]
pub struct TestRegion {
    pub start_line: usize,
    pub end_line: usize,
    pub kind: TestRegionKind,
}

#[derive(Debug, Clone)]
pub enum TestRegionKind {
    CfgTestModule,      // #[cfg(test)] mod tests { }
    TestModule,         // mod tests { } (heuristic)
    TestFunction,       // #[test] fn test_foo() { }
}

/// Detect test regions in AST
pub fn detect_test_regions(file: &syn::File) -> Vec<TestRegion> {
    let mut regions = Vec::new();
    let mut visitor = TestRegionVisitor::new();
    visitor.visit_file(file);
    regions.extend(visitor.regions);
    regions
}

struct TestRegionVisitor {
    regions: Vec<TestRegion>,
}

impl<'ast> Visit<'ast> for TestRegionVisitor {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        // Check for #[cfg(test)] attribute
        let is_test_module = module.attrs.iter().any(|attr| {
            if attr.path().is_ident("cfg") {
                // Parse the cfg attribute to check for "test"
                if let Ok(meta) = attr.meta.require_list() {
                    return meta.tokens.to_string().contains("test");
                }
            }
            false
        });

        // Also detect `mod tests` by name (common convention)
        let is_tests_by_name = module.ident == "tests";

        if is_test_module || is_tests_by_name {
            if let Some(content) = &module.content {
                // Get line span from module
                let start = module.mod_token.span.start().line;
                let end = content.1.last()
                    .map(|item| get_item_end_line(item))
                    .unwrap_or(start);

                self.regions.push(TestRegion {
                    start_line: start,
                    end_line: end,
                    kind: if is_test_module {
                        TestRegionKind::CfgTestModule
                    } else {
                        TestRegionKind::TestModule
                    },
                });
            }
        }

        // Continue visiting for nested test modules
        syn::visit::visit_item_mod(self, module);
    }
}
```

#### 2. LOC Calculation Update

```rust
/// Calculate production LOC excluding test regions
pub fn calculate_production_loc(
    total_loc: usize,
    test_regions: &[TestRegion],
) -> usize {
    let test_lines: usize = test_regions.iter()
        .map(|r| r.end_line.saturating_sub(r.start_line) + 1)
        .sum();

    total_loc.saturating_sub(test_lines)
}

/// Enhanced LOC metrics
#[derive(Debug, Clone)]
pub struct LocMetrics {
    pub total_loc: usize,
    pub production_loc: usize,
    pub test_loc: usize,
    pub test_percentage: f64,
}

impl LocMetrics {
    pub fn new(total: usize, test_regions: &[TestRegion]) -> Self {
        let test_loc: usize = test_regions.iter()
            .map(|r| r.end_line.saturating_sub(r.start_line) + 1)
            .sum();
        let production_loc = total.saturating_sub(test_loc);
        let test_percentage = if total > 0 {
            (test_loc as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            total_loc: total,
            production_loc,
            test_loc,
            test_percentage,
        }
    }
}
```

#### 3. Integration with Scoring

```rust
// In extraction adapter or scoring module
fn calculate_god_object_score_with_test_exclusion(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    loc_metrics: &LocMetrics,
    thresholds: &GodObjectThresholds,
    include_test_loc: bool,
) -> f64 {
    let effective_loc = if include_test_loc {
        loc_metrics.total_loc
    } else {
        loc_metrics.production_loc
    };

    calculate_god_object_score(
        method_count,
        field_count,
        responsibility_count,
        effective_loc,
        thresholds,
    )
}
```

#### 4. Output Format Update

```
god object structure
  methods                   24
  fields                    3
  responsibilities          7
  loc                       620 (867 with tests)
  test coverage             29% (247 lines)

complexity
  accumulated cyclomatic    116
```

### Alternative Approach: Span-Based Detection

If line-number based detection proves unreliable, use span tracking:

```rust
// Track spans during AST visiting
pub struct SpanTracker {
    test_spans: Vec<(usize, usize)>,  // (start_byte, end_byte)
}

impl SpanTracker {
    pub fn count_production_lines(&self, source: &str) -> usize {
        let total_lines = source.lines().count();
        let test_line_count = self.test_spans.iter()
            .flat_map(|(start, end)| {
                // Convert byte offsets to line numbers
                let start_line = source[..*start].lines().count();
                let end_line = source[..*end].lines().count();
                start_line..=end_line
            })
            .collect::<std::collections::HashSet<_>>()
            .len();

        total_lines.saturating_sub(test_line_count)
    }
}
```

## Dependencies

- **Prerequisites**:
  - Spec 207: LOC calculation infrastructure
- **Affected Components**:
  - `ast_visitor.rs`: Add test region detection
  - `extraction/adapters/god_object.rs`: Use production LOC
  - `scoring.rs`: Accept LocMetrics parameter
  - `types.rs`: Add TestRegion, LocMetrics types

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_detect_cfg_test_module() {
    let code = r#"
        pub struct Foo {}
        impl Foo {
            pub fn method(&self) {}
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn test_method() {}
        }
    "#;

    let file = syn::parse_file(code).unwrap();
    let regions = detect_test_regions(&file);

    assert_eq!(regions.len(), 1);
    assert!(matches!(regions[0].kind, TestRegionKind::CfgTestModule));
}

#[test]
fn test_loc_metrics_calculation() {
    let regions = vec![TestRegion {
        start_line: 50,
        end_line: 100,
        kind: TestRegionKind::CfgTestModule,
    }];

    let metrics = LocMetrics::new(150, &regions);

    assert_eq!(metrics.total_loc, 150);
    assert_eq!(metrics.test_loc, 51);  // 100 - 50 + 1
    assert_eq!(metrics.production_loc, 99);
}

#[test]
fn test_nested_test_modules() {
    let code = r#"
        #[cfg(test)]
        mod tests {
            mod integration_tests {
                #[test]
                fn test_integration() {}
            }
        }
    "#;

    let file = syn::parse_file(code).unwrap();
    let regions = detect_test_regions(&file);

    // Should detect outer #[cfg(test)] module
    assert!(!regions.is_empty());
}
```

### Integration Tests

```rust
#[test]
fn test_call_resolver_production_loc() {
    let content = include_str!("../../src/analyzers/call_graph/call_resolution.rs");
    let file = syn::parse_file(content).unwrap();
    let regions = detect_test_regions(&file);

    let total_lines = content.lines().count();
    let metrics = LocMetrics::new(total_lines, &regions);

    // CallResolver has ~250 lines of tests
    assert!(metrics.test_loc >= 200, "Expected 200+ test lines, got {}", metrics.test_loc);
    assert!(metrics.production_loc <= 650, "Expected <=650 production lines, got {}", metrics.production_loc);
    assert!(metrics.test_percentage >= 25.0, "Expected 25%+ tests, got {}%", metrics.test_percentage);
}
```

## Documentation Requirements

- **Code Documentation**: Document test region detection in ast_visitor.rs
- **User Documentation**: Explain how test exclusion affects LOC metrics
- **Output Documentation**: Document new LOC format with test breakdown

## Implementation Notes

1. **Edge Cases**:
   - Nested `#[cfg(test)]` modules
   - Test code outside `mod tests` (e.g., `#[test]` in main module)
   - Conditional compilation with multiple cfg attributes
   - Doc tests (in `///` comments) - typically not an issue

2. **Performance**:
   - Single pass detection during existing AST traversal
   - Avoid re-parsing just for test detection

3. **Accuracy**:
   - May slightly under-count test lines if detection misses edge cases
   - Prefer under-counting to over-counting (conservative approach)

## Migration and Compatibility

- Default behavior changes to exclude test LOC
- Add `--include-test-loc` flag for old behavior
- Show both values in output for transparency
- Document change in release notes

## Success Metrics

- `CallResolver` LOC: 867 -> ~620 (production only)
- Score reduction for well-tested files
- No false negatives (actual god objects still detected)
