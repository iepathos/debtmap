---
number: 21
title: Dead Code Detection Category for Unused Functions
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-08-12
---

# Specification 21: Dead Code Detection Category for Unused Functions

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current unified debt scoring system incorrectly categorizes unused functions as "General technical debt" with a generic 5.0 risk score. This creates false positives where perfectly clean, well-written functions are flagged simply because they're not called anywhere in the codebase.

Example case: `identify_untested_complex_functions()` and `identify_well_tested_complex_functions()` in `src/risk/priority.rs` are clean, single-purpose utility functions with low complexity, but get flagged as "high risk" debt because they fall through to the generic Risk category in the unified scorer's `determine_debt_type()` fallback.

The current categorization logic only handles:
1. **TestingGap** - functions with low coverage AND complexity > 3
2. **ComplexityHotspot** - functions with high cyclomatic/cognitive complexity
3. **Risk** (fallback) - everything else gets generic "General technical debt"

This produces unhelpful output that doesn't guide users toward actionable improvements.

## Objective

Add a new **DeadCode** debt type category that accurately identifies and categorizes unused functions, providing specific actionable guidance for cleanup decisions rather than generic "address technical debt" recommendations.

## Requirements

### Functional Requirements

1. **Dead Code Detection**
   - Detect public functions that are never called within the analyzed codebase
   - Distinguish between truly unused functions and library/API functions intended for external use
   - Handle different visibility patterns (pub, pub(crate), private)
   - Account for dynamic dispatch and trait implementations

2. **Call Graph Integration**
   - Leverage existing call graph analysis to determine function usage
   - Identify functions with zero incoming calls (except main/entry points)
   - Handle special cases: test functions, binary entry points, exported APIs
   - Support both direct calls and indirect calls through function pointers

3. **Categorization Enhancement**
   - Add `DebtType::DeadCode` variant with metadata about usage patterns
   - Provide specific recommendations: remove, document as API, add tests, etc.
   - Include context about why the function might be unused
   - Show complexity metrics to help with removal decisions

4. **Smart Filtering**
   - Exclude common false positives: main functions, test helpers, exported APIs
   - Configurable exclusion patterns via annotations or naming conventions
   - Handle procedural macros and code generation scenarios
   - Support allow-lists for intentionally unused functions

### Non-Functional Requirements

1. **Performance**
   - Minimal impact on existing analysis pipeline
   - Efficient call graph traversal for usage detection
   - No significant memory overhead for tracking usage patterns

2. **Accuracy**
   - Low false positive rate through smart exclusion rules
   - Clear distinction between dead code and intentional APIs
   - Proper handling of conditional compilation and feature flags

3. **Usability**
   - Clear recommendations distinguishing removal vs. documentation needs
   - Helpful context about function characteristics (complexity, visibility)
   - Guidance on safe removal practices

## Acceptance Criteria

- [ ] New `DebtType::DeadCode` variant added to debt type enum
- [ ] `determine_debt_type()` enhanced to detect unused functions before fallback
- [ ] Dead code detection uses call graph to identify zero-usage functions
- [ ] Excludes common false positives (main, tests, pub APIs with #[no_mangle], etc.)
- [ ] Provides specific recommendations: "Remove unused function", "Document as API", etc.
- [ ] Shows complexity metrics for dead code items (to help prioritize removal)
- [ ] Output format includes usage context and removal guidance
- [ ] Previously flagged functions like `identify_*_functions` now show as DeadCode instead of Risk
- [ ] Integration tests demonstrate correct categorization of unused vs. used functions
- [ ] Performance impact < 5% of total analysis time

## Technical Details

### Implementation Approach

#### 1. Dead Code Detection Logic (`src/priority/unified_scorer.rs`)

Enhance `determine_debt_type()` to detect dead code before falling back to generic Risk:

```rust
fn determine_debt_type(
    func: &FunctionMetrics, 
    coverage: &Option<TransitiveCoverage>,
    call_graph: &CallGraph,
    func_id: &str
) -> DebtType {
    // Existing TestingGap detection
    if let Some(cov) = coverage {
        if cov.direct < 0.2 && func.cyclomatic > 3 {
            return DebtType::TestingGap {
                coverage: cov.direct,
                cyclomatic: func.cyclomatic,
                cognitive: func.cognitive,
            };
        }
    }

    // Existing ComplexityHotspot detection
    if func.cyclomatic > 10 || func.cognitive > 15 {
        return DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        };
    }

    // NEW: Dead code detection
    if is_dead_code(func, call_graph, func_id) {
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        };
    }

    // Fallback to risk-based debt (now much more targeted)
    DebtType::Risk {
        risk_score: 5.0,
        factors: vec!["General technical debt".to_string()],
    }
}
```

#### 2. Dead Code Detection Implementation

```rust
fn is_dead_code(func: &FunctionMetrics, call_graph: &CallGraph, func_id: &str) -> bool {
    // Skip obvious false positives
    if is_excluded_from_dead_code_analysis(func) {
        return false;
    }
    
    // Check if function has incoming calls
    let callers = call_graph.get_callers(func_id);
    callers.is_empty()
}

fn is_excluded_from_dead_code_analysis(func: &FunctionMetrics) -> bool {
    // Entry points
    if func.name == "main" || func.name.starts_with("_start") {
        return true;
    }
    
    // Test functions
    if func.name.starts_with("test_") || func.file.contains("/tests/") {
        return true;
    }
    
    // Exported functions (likely FFI or API)
    if func.name.contains("extern") || has_no_mangle_attribute(func) {
        return true;
    }
    
    // Common framework patterns
    if is_framework_callback(func) {
        return true;
    }
    
    false
}

fn determine_visibility(func: &FunctionMetrics) -> FunctionVisibility {
    // Parse function signature to determine visibility
    if func.signature.starts_with("pub fn") {
        if func.signature.contains("pub(crate)") {
            FunctionVisibility::Crate
        } else {
            FunctionVisibility::Public
        }
    } else {
        FunctionVisibility::Private
    }
}

fn generate_usage_hints(func: &FunctionMetrics, call_graph: &CallGraph, func_id: &str) -> Vec<String> {
    let mut hints = Vec::new();
    
    // Check if it calls other functions (might be incomplete implementation)
    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        hints.push("Function has no dependencies - safe to remove".to_string());
    } else {
        hints.push(format!("Function calls {} other functions", callees.len()));
    }
    
    // Check complexity for removal priority
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        hints.push("Low complexity - low impact removal".to_string());
    } else {
        hints.push("High complexity - removing may eliminate significant unused code".to_string());
    }
    
    // Check if it's part of a larger unused module
    if appears_to_be_unused_module_member(func, call_graph) {
        hints.push("May be part of larger unused module".to_string());
    }
    
    hints
}
```

#### 3. New Data Structures

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum DebtType {
    TestingGap {
        coverage: f64,
        cyclomatic: u32,
        cognitive: u32,
    },
    ComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
    },
    TestComplexityHotspot {
        coverage: f64,
        cyclomatic: u32,
        cognitive: u32,
    },
    // NEW: Dead code category
    DeadCode {
        visibility: FunctionVisibility,
        cyclomatic: u32,
        cognitive: u32,
        usage_hints: Vec<String>,
    },
    Risk {
        risk_score: f64,
        factors: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionVisibility {
    Private,       // fn (module-private)
    Crate,         // pub(crate) fn
    Public,        // pub fn
}
```

#### 4. Enhanced Recommendations (`src/priority/unified_scorer.rs`)

```rust
fn generate_recommendation(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    score: &UnifiedScore,
) -> ActionableRecommendation {
    match debt_type {
        DebtType::DeadCode { visibility, usage_hints, .. } => {
            let (action, rationale, steps) = match visibility {
                FunctionVisibility::Private => (
                    "Remove unused private function".to_string(),
                    format!("Private function '{}' has no callers and can be safely removed", func.name),
                    vec![
                        "Verify no dynamic calls or reflection usage".to_string(),
                        "Remove function definition".to_string(),
                        "Remove associated tests if any".to_string(),
                        "Check if removal enables further cleanup".to_string(),
                    ]
                ),
                FunctionVisibility::Crate => (
                    "Remove or document unused crate function".to_string(),
                    format!("Crate-public function '{}' has no internal callers", func.name),
                    vec![
                        "Check if function is intended as internal API".to_string(),
                        "Add documentation if keeping for future use".to_string(),
                        "Remove if truly unused".to_string(),
                        "Consider making private if only locally needed".to_string(),
                    ]
                ),
                FunctionVisibility::Public => (
                    "Document or deprecate unused public function".to_string(),
                    format!("Public function '{}' has no internal callers - may be external API", func.name),
                    vec![
                        "Verify no external callers exist".to_string(),
                        "Add comprehensive documentation if keeping".to_string(),
                        "Mark as deprecated if phasing out".to_string(),
                        "Consider adding usage examples or tests".to_string(),
                    ]
                ),
            };
            
            ActionableRecommendation {
                primary_action: action,
                rationale,
                implementation_steps: steps,
                related_items: vec![], // Could link to other unused functions in same module
            }
        }
        // ... existing cases for other debt types
    }
}
```

#### 5. Formatter Updates (`src/priority/formatter.rs`)

Update `extract_complexity_info()` to handle DeadCode:

```rust
fn extract_complexity_info(item: &UnifiedDebtItem) -> (u32, u32, u32, u32, usize) {
    let (cyclomatic, cognitive, branch_count) = match &item.debt_type {
        DebtType::TestingGap { cyclomatic, cognitive, .. } => {
            (*cyclomatic, *cognitive, *cyclomatic)
        }
        DebtType::ComplexityHotspot { cyclomatic, cognitive } => {
            (*cyclomatic, *cognitive, *cyclomatic)
        }
        DebtType::TestComplexityHotspot { cyclomatic, cognitive, .. } => {
            (*cyclomatic, *cognitive, *cyclomatic)
        }
        // NEW: Show complexity for dead code too
        DebtType::DeadCode { cyclomatic, cognitive, .. } => {
            (*cyclomatic, *cognitive, *cyclomatic)
        }
        _ => (0, 0, 0),
    };

    (cyclomatic, cognitive, branch_count, item.nesting_depth, item.function_length)
}
```

Add dead code specific formatting:

```rust
fn format_priority_item(output: &mut String, index: usize, item: &UnifiedDebtItem) {
    // ... existing header logic

    // Type-specific information
    match &item.debt_type {
        DebtType::DeadCode { visibility, usage_hints, .. } => {
            writeln!(
                output,
                "├─ DEAD CODE: {} function with no callers",
                format_visibility(*visibility).yellow()
            ).unwrap();
            
            for hint in usage_hints {
                writeln!(output, "│  • {}", hint.dimmed()).unwrap();
            }
        }
        // ... existing cases
    }
    
    // ... rest of formatting
}

fn format_visibility(visibility: FunctionVisibility) -> &'static str {
    match visibility {
        FunctionVisibility::Private => "private",
        FunctionVisibility::Crate => "crate-public", 
        FunctionVisibility::Public => "public",
    }
}
```

### Architecture Changes

#### Modified Components
- `src/priority/unified_scorer.rs`: Enhanced debt type detection and recommendation generation
- `src/priority/formatter.rs`: Support for DeadCode formatting and complexity display
- `src/core/metrics.rs`: Add function visibility parsing if not already present
- `src/priority/call_graph.rs`: Ensure usage tracking is available for dead code detection

#### New Functionality
- Dead code detection algorithm with smart exclusions
- Function visibility determination from signatures
- Usage pattern analysis and hint generation
- Dead code specific recommendations and formatting

### Integration Points

#### Call Graph Dependency
The dead code detector relies on the existing call graph infrastructure to determine function usage. This requires:
- Accurate call relationship tracking
- Proper handling of dynamic dispatch
- Support for cross-module call detection

#### Configuration Integration
```rust
// In debtmap.toml or CLI flags
[dead_code]
exclude_patterns = ["test_*", "*_test", "bench_*"]
exclude_public = false  # Whether to flag unused public functions
exclude_crate = false   # Whether to flag unused crate functions
```

## Dependencies

### Prerequisites
- Existing call graph analysis system
- Function signature parsing for visibility detection
- Current debt categorization framework

### Affected Components
- Unified scoring system (core categorization logic)
- Priority formatter (display and complexity extraction)
- Call graph analyzer (usage tracking)
- Output generation (new debt type display)

### External Dependencies
- No new external crates required
- Uses existing tree-sitter parsing for signature analysis
- Leverages current call graph implementation

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod dead_code_tests {
    use super::*;

    #[test]
    fn test_detect_unused_private_function() {
        let mut call_graph = CallGraph::new();
        let func = create_test_function("unused_helper", "fn unused_helper() {}", 2, 1);
        
        // Function exists but has no callers
        let debt_type = determine_debt_type(&func, &None, &call_graph, "unused_helper");
        
        match debt_type {
            DebtType::DeadCode { visibility: FunctionVisibility::Private, .. } => (),
            _ => panic!("Expected DeadCode for unused private function"),
        }
    }

    #[test]
    fn test_exclude_main_function() {
        let call_graph = CallGraph::new();
        let func = create_test_function("main", "fn main() {}", 1, 5);
        
        let debt_type = determine_debt_type(&func, &None, &call_graph, "main");
        
        // Main should not be flagged as dead code
        match debt_type {
            DebtType::DeadCode { .. } => panic!("Main function should not be flagged as dead code"),
            _ => (),
        }
    }

    #[test]
    fn test_exclude_test_functions() {
        let call_graph = CallGraph::new();
        let func = create_test_function("test_helper", "fn test_helper() {}", 1, 3);
        
        let debt_type = determine_debt_type(&func, &None, &call_graph, "test_helper");
        
        match debt_type {
            DebtType::DeadCode { .. } => panic!("Test functions should not be flagged as dead code"),
            _ => (),
        }
    }

    #[test]
    fn test_public_function_recommendation() {
        let func = create_test_function("api_func", "pub fn api_func() {}", 1, 5);
        let debt_type = DebtType::DeadCode {
            visibility: FunctionVisibility::Public,
            cyclomatic: 1,
            cognitive: 1,
            usage_hints: vec!["No internal callers".to_string()],
        };
        
        let recommendation = generate_recommendation(&func, &debt_type, FunctionRole::PureLogic, &create_test_score());
        
        assert!(recommendation.primary_action.contains("Document or deprecate"));
        assert!(recommendation.implementation_steps.iter().any(|s| s.contains("external callers")));
    }

    #[test]
    fn test_complexity_shown_for_dead_code() {
        let item = create_dead_code_item(5, 8); // cyclomatic=5, cognitive=8
        
        let (cyclomatic, cognitive, _branches, _nesting, _length) = extract_complexity_info(&item);
        
        assert_eq!(cyclomatic, 5);
        assert_eq!(cognitive, 8);
    }
}
```

### Integration Tests

```rust
// tests/dead_code_integration.rs
#[test]
fn test_dead_code_detection_real_codebase() {
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/unused_functions"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("DEAD CODE"));
    assert!(stdout.contains("private function with no callers"));
}

#[test]
fn test_dead_code_excludes_main() {
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/simple_binary"])
        .output()
        .expect("Failed to execute debtmap");

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Main function should not appear in dead code results
    assert!(!stdout.contains("main") || !stdout.contains("DEAD CODE"));
}

#[test]
fn test_previously_flagged_functions_now_dead_code() {
    // Test that identify_untested_complex_functions is now properly categorized
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "src/risk/priority.rs", "--format", "json"])
        .output()
        .expect("Failed to execute debtmap");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    // Find the identify_untested_complex_functions item
    let items = json["priority_analysis"]["items"].as_array().unwrap();
    let identify_item = items.iter().find(|item| {
        item["location"]["function"].as_str().unwrap().contains("identify_untested_complex_functions")
    }).expect("Should find identify_untested_complex_functions");
    
    // Should now be DeadCode, not Risk
    assert!(identify_item["debt_type"]["DeadCode"].is_object());
}
```

### Performance Tests

```rust
#[test]
fn test_dead_code_detection_performance() {
    let start = std::time::Instant::now();
    
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "src/", "--format", "json"])
        .output()
        .expect("Failed to execute debtmap");
    
    let elapsed = start.elapsed();
    
    // Dead code detection should not significantly impact performance
    assert!(elapsed.as_secs() < 30, "Analysis took too long: {:?}", elapsed);
    assert!(output.status.success());
}
```

## Documentation Requirements

### Code Documentation
- Comprehensive doc comments for all new dead code detection functions
- Examples of excluded patterns and edge cases
- Performance characteristics and limitations

### User Documentation Updates

Add to README.md:
```markdown
## Dead Code Detection

Debtmap automatically identifies unused functions that may be candidates for removal:

```bash
# Analyze for dead code among other debt types
debtmap analyze src/

# Focus on dead code with detailed recommendations
debtmap analyze src/ --format json | jq '.items[] | select(.debt_type.DeadCode)'
```

### Dead Code Categories

- **Private Functions**: Safe to remove if truly unused
- **Crate Functions**: May be internal APIs - verify before removing  
- **Public Functions**: Likely external APIs - document or deprecate carefully

### Exclusions

The following are automatically excluded from dead code detection:
- Entry points (`main`, `_start`)
- Test functions (`test_*`, functions in `/tests/`)
- Exported functions (`#[no_mangle]`, `extern` functions)
- Framework callbacks and procedural macros
```

### Architecture Documentation Updates

Update ARCHITECTURE.md to document:
- Dead code detection algorithm and exclusion rules
- Integration with call graph analysis
- Performance impact and optimization strategies
- Extension points for custom exclusion patterns

## Implementation Notes

### Phased Implementation
1. **Phase 1**: Basic unused function detection using call graph
2. **Phase 2**: Smart exclusion rules for common false positives
3. **Phase 3**: Visibility-aware recommendations and formatting
4. **Phase 4**: Usage hint generation and advanced analysis
5. **Phase 5**: Configuration options and custom exclusion patterns

### Edge Cases to Consider
- Functions called only through macros or code generation
- Conditional compilation and feature-gated code
- Dynamic dispatch through trait objects
- Foreign function interfaces (FFI)
- Test helper functions in non-standard locations

### False Positive Mitigation
- Conservative exclusion rules (better to miss dead code than flag live code)
- Clear documentation of what gets excluded and why
- Configuration options to adjust sensitivity
- Manual override annotations for special cases

## Expected Impact

After implementation:

1. **Accurate Categorization**: Functions like `identify_untested_complex_functions` will be correctly identified as dead code rather than generic "technical debt"

2. **Actionable Guidance**: Instead of "Address technical debt", users get specific recommendations like "Remove unused private function" or "Document unused public API"

3. **Complexity Visibility**: Dead code items will show their complexity metrics, helping prioritize removal of high-complexity unused code

4. **Reduced False Positives**: The generic Risk category will be reserved for actual risk factors, not clean unused code

5. **Better Tool Adoption**: More accurate categorization leads to higher user trust and adoption

## Migration and Compatibility

- **Breaking Changes**: None - purely additive enhancement
- **Configuration Migration**: No existing configurations affected  
- **Output Changes**: Previously "Risk" items may now appear as "DeadCode" - this is an improvement in accuracy
- **API Stability**: New DebtType variant is additive, existing variants unchanged

The dead code detection enhancement provides more accurate debt categorization and actionable guidance while maintaining full backward compatibility with existing functionality.