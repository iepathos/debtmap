---
number: 261
title: Call Graph Purity Propagation
category: optimization
priority: medium
status: draft
dependencies: [259]
created: 2025-12-12
---

# Specification 261: Call Graph Purity Propagation

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 259 (Fix Constants False Positive)

## Context

**Current Problem**: Purity analysis is performed per-function in isolation. A function that only calls other pure functions should be considered pure, but currently each function is analyzed independently:

```rust
// Currently: all three analyzed independently
fn helper_pure(x: i32) -> i32 { x + 1 }  // Pure

fn helper_also_pure(x: i32) -> i32 { x * 2 }  // Pure

fn caller(x: i32) -> i32 {
    helper_pure(helper_also_pure(x))  // Should be pure! But call analysis is missing
}
```

**Impact**: Functions that compose pure operations are incorrectly marked with lower confidence or even impure due to "unknown function calls" penalty.

## Objective

Propagate purity information through the call graph so that functions calling only pure functions inherit purity classification with high confidence.

## Requirements

### Functional Requirements

1. **Callee Purity Resolution**
   - Resolve function calls to their purity classification
   - Support calls to functions in same file
   - Support calls to functions in same crate
   - Handle method calls on known types

2. **Purity Propagation**
   - If all callees are pure → caller is pure (high confidence)
   - If any callee is impure → caller is impure
   - If callee purity unknown → reduce caller confidence (don't mark impure)

3. **Standard Library Awareness**
   - Maintain whitelist of known pure std functions
   - Include iterator methods: `map`, `filter`, `fold`, etc.
   - Include Option/Result methods: `map`, `and_then`, `unwrap_or`, etc.

4. **Cycle Handling**
   - Detect and handle recursive calls
   - Use conservative classification for cycles

### Non-Functional Requirements

- Performance: Use call graph already built in pipeline
- Memory: No significant additional storage
- Accuracy: No false positives (impure marked pure)

## Implementation

### Phase 1: Known Pure Standard Library Functions

```rust
/// Standard library functions known to be pure
/// These take inputs and return outputs without side effects
const KNOWN_PURE_STD_FUNCTIONS: &[&str] = &[
    // Option methods
    "Option::map",
    "Option::and_then",
    "Option::or_else",
    "Option::unwrap_or",
    "Option::unwrap_or_else",
    "Option::unwrap_or_default",
    "Option::filter",
    "Option::flatten",
    "Option::zip",
    "Option::ok_or",
    "Option::ok_or_else",
    "Option::is_some",
    "Option::is_none",
    "Option::as_ref",
    "Option::as_mut",
    "Option::cloned",
    "Option::copied",

    // Result methods
    "Result::map",
    "Result::map_err",
    "Result::and_then",
    "Result::or_else",
    "Result::unwrap_or",
    "Result::unwrap_or_else",
    "Result::unwrap_or_default",
    "Result::is_ok",
    "Result::is_err",
    "Result::ok",
    "Result::err",
    "Result::as_ref",

    // Iterator methods (pure when closure is pure)
    "Iterator::map",
    "Iterator::filter",
    "Iterator::filter_map",
    "Iterator::flat_map",
    "Iterator::fold",
    "Iterator::reduce",
    "Iterator::take",
    "Iterator::skip",
    "Iterator::take_while",
    "Iterator::skip_while",
    "Iterator::enumerate",
    "Iterator::zip",
    "Iterator::chain",
    "Iterator::collect",
    "Iterator::count",
    "Iterator::sum",
    "Iterator::product",
    "Iterator::any",
    "Iterator::all",
    "Iterator::find",
    "Iterator::position",
    "Iterator::max",
    "Iterator::min",
    "Iterator::max_by",
    "Iterator::min_by",
    "Iterator::max_by_key",
    "Iterator::min_by_key",
    "Iterator::rev",
    "Iterator::cloned",
    "Iterator::copied",
    "Iterator::peekable",
    "Iterator::fuse",
    "Iterator::flatten",

    // Slice methods
    "slice::iter",
    "slice::len",
    "slice::is_empty",
    "slice::first",
    "slice::last",
    "slice::get",
    "slice::split_at",
    "slice::chunks",
    "slice::windows",
    "slice::contains",
    "slice::starts_with",
    "slice::ends_with",
    "slice::binary_search",

    // String methods
    "str::len",
    "str::is_empty",
    "str::chars",
    "str::bytes",
    "str::contains",
    "str::starts_with",
    "str::ends_with",
    "str::find",
    "str::rfind",
    "str::split",
    "str::trim",
    "str::trim_start",
    "str::trim_end",
    "str::to_lowercase",
    "str::to_uppercase",
    "str::to_string",
    "str::parse",

    // Vec methods (read-only)
    "Vec::len",
    "Vec::is_empty",
    "Vec::capacity",
    "Vec::iter",
    "Vec::first",
    "Vec::last",
    "Vec::get",
    "Vec::contains",

    // HashMap methods (read-only)
    "HashMap::len",
    "HashMap::is_empty",
    "HashMap::get",
    "HashMap::contains_key",
    "HashMap::keys",
    "HashMap::values",
    "HashMap::iter",

    // Clone trait
    "Clone::clone",

    // Default trait
    "Default::default",

    // From/Into traits
    "From::from",
    "Into::into",

    // Comparison traits
    "PartialEq::eq",
    "PartialEq::ne",
    "PartialOrd::partial_cmp",
    "Ord::cmp",

    // Conversion functions
    "std::convert::identity",
    "std::mem::size_of",
    "std::mem::align_of",
    "std::mem::replace",
    "std::mem::take",
    "std::mem::swap",
];

fn is_known_pure_call(method_name: &str, receiver_type: Option<&str>) -> bool {
    // Check direct function matches
    let full_name = match receiver_type {
        Some(ty) => format!("{}::{}", ty, method_name),
        None => method_name.to_string(),
    };

    KNOWN_PURE_STD_FUNCTIONS
        .iter()
        .any(|pure_fn| full_name.ends_with(pure_fn) || pure_fn.ends_with(&full_name))
}
```

### Phase 2: Call Graph Integration

```rust
/// Extended purity analysis with call graph awareness
pub struct CallGraphAwarePurityAnalyzer<'a> {
    call_graph: &'a CallGraph,
    purity_cache: HashMap<FunctionId, PurityClassification>,
}

#[derive(Debug, Clone)]
pub struct PurityClassification {
    pub is_pure: bool,
    pub confidence: f32,
    pub reasons: Vec<String>,
    pub callee_evidence: Vec<CalleeEvidence>,
}

#[derive(Debug, Clone)]
pub struct CalleeEvidence {
    pub callee_name: String,
    pub callee_purity: CalleePurity,
}

#[derive(Debug, Clone)]
pub enum CalleePurity {
    KnownPure,           // Standard library, whitelisted
    AnalyzedPure(f32),   // Analyzed in this crate, with confidence
    AnalyzedImpure,      // Analyzed in this crate, impure
    Unknown,             // External, not in whitelist
}

impl<'a> CallGraphAwarePurityAnalyzer<'a> {
    pub fn analyze_with_call_graph(
        &mut self,
        func_id: &FunctionId,
        base_analysis: &PurityAnalysis,
    ) -> PurityClassification {
        // Start with base analysis
        let mut classification = PurityClassification {
            is_pure: base_analysis.is_pure,
            confidence: base_analysis.confidence,
            reasons: base_analysis.reasons.iter().map(|r| r.description().to_string()).collect(),
            callee_evidence: Vec::new(),
        };

        // If already impure from local analysis, don't upgrade
        if !base_analysis.is_pure && !base_analysis.reasons.is_empty() {
            return classification;
        }

        // Get callees from call graph
        let callees = self.call_graph.get_callees(func_id);

        for callee in callees {
            let callee_purity = self.resolve_callee_purity(&callee);

            classification.callee_evidence.push(CalleeEvidence {
                callee_name: callee.name.clone(),
                callee_purity: callee_purity.clone(),
            });

            match callee_purity {
                CalleePurity::KnownPure => {
                    // Known pure - boost confidence
                    classification.confidence *= 1.02;
                }
                CalleePurity::AnalyzedPure(conf) => {
                    // Propagate confidence from callee
                    classification.confidence *= conf;
                }
                CalleePurity::AnalyzedImpure => {
                    // Callee is impure - caller is impure
                    classification.is_pure = false;
                    classification.confidence = 0.95;
                    classification.reasons.push(format!(
                        "Calls impure function: {}",
                        callee.name
                    ));
                }
                CalleePurity::Unknown => {
                    // Unknown callee - reduce confidence but don't mark impure
                    classification.confidence *= 0.9;
                    if classification.confidence < 0.6 {
                        classification.reasons.push(format!(
                            "Calls unknown function: {}",
                            callee.name
                        ));
                    }
                }
            }
        }

        // Clamp confidence
        classification.confidence = classification.confidence.clamp(0.3, 1.0);

        // If no impure evidence and confidence > 0.8, mark as pure
        if classification.confidence > 0.8
            && classification.reasons.is_empty()
            && !classification.callee_evidence.iter().any(|e| matches!(e.callee_purity, CalleePurity::AnalyzedImpure))
        {
            classification.is_pure = true;
        }

        classification
    }

    fn resolve_callee_purity(&self, callee: &FunctionId) -> CalleePurity {
        // 1. Check if it's a known pure std function
        if is_known_pure_call(&callee.name, None) {
            return CalleePurity::KnownPure;
        }

        // 2. Check cache for already-analyzed functions
        if let Some(cached) = self.purity_cache.get(callee) {
            return if cached.is_pure {
                CalleePurity::AnalyzedPure(cached.confidence)
            } else {
                CalleePurity::AnalyzedImpure
            };
        }

        // 3. Unknown
        CalleePurity::Unknown
    }
}
```

### Phase 3: Analysis Order (Topological)

```rust
/// Analyze functions in dependency order for accurate propagation
pub fn analyze_with_propagation(
    call_graph: &CallGraph,
    functions: &[FunctionMetrics],
) -> HashMap<FunctionId, PurityClassification> {
    let mut results = HashMap::new();

    // Sort functions in reverse topological order (leaf functions first)
    let sorted = call_graph.topological_sort_reverse();

    let mut analyzer = CallGraphAwarePurityAnalyzer {
        call_graph,
        purity_cache: HashMap::new(),
    };

    for func_id in sorted {
        if let Some(func) = functions.iter().find(|f| FunctionId::from_metrics(f) == func_id) {
            // Run base analysis
            let base_analysis = analyze_function_purity(func);

            // Enhance with call graph information
            let classification = analyzer.analyze_with_call_graph(&func_id, &base_analysis);

            // Cache result for callees to use
            analyzer.purity_cache.insert(func_id.clone(), classification.clone());
            results.insert(func_id, classification);
        }
    }

    results
}
```

## Acceptance Criteria

- [ ] Function calling only known pure std functions is classified as pure
- [ ] Function calling analyzed pure functions inherits pure classification
- [ ] Function calling impure function is marked impure with reason
- [ ] Unknown function calls reduce confidence but don't mark impure
- [ ] Iterator method chains (`.map().filter().collect()`) preserve purity
- [ ] Option/Result method chains preserve purity
- [ ] Recursive functions handled without infinite loops
- [ ] Analysis completes in reasonable time (<100ms for typical codebase)

## Technical Details

### Files to Modify/Create

| File | Changes |
|------|---------|
| `src/analyzers/purity_detector.rs` | Add `KNOWN_PURE_STD_FUNCTIONS` |
| `src/analysis/purity_propagation.rs` (new) | Call graph aware analysis |
| `src/pipeline/stages/purity.rs` | Integrate propagation |

### Integration Points

1. **Call Graph** (`src/priority/call_graph.rs`): Use existing call graph data
2. **Purity Detector** (`src/analyzers/purity_detector.rs`): Base analysis
3. **Pipeline** (`src/pipeline/stages/purity.rs`): Orchestrate analysis

## Dependencies

- **Prerequisites**: Spec 259 (better constant handling improves base analysis)
- **Affected Components**: Purity analysis, scoring, recommendations
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test individual function purity propagation
- **Integration Tests**: Test chains of pure function calls
- **Edge Cases**: Recursive calls, mutual recursion, long call chains

### Test Cases

```rust
#[test]
fn test_pure_caller_of_pure() {
    // helper is pure, caller should be pure
    let code = r#"
        fn helper(x: i32) -> i32 { x + 1 }
        fn caller(x: i32) -> i32 { helper(x) }
    "#;
    let results = analyze_with_propagation(code);
    assert!(results.get("caller").unwrap().is_pure);
}

#[test]
fn test_impure_caller_of_impure() {
    let code = r#"
        fn impure_helper(x: i32) { println!("{}", x); }
        fn caller(x: i32) { impure_helper(x); }
    "#;
    let results = analyze_with_propagation(code);
    assert!(!results.get("caller").unwrap().is_pure);
    assert!(results.get("caller").unwrap().reasons.iter()
        .any(|r| r.contains("impure_helper")));
}

#[test]
fn test_std_iterator_chain_is_pure() {
    let code = r#"
        fn sum_doubled(items: &[i32]) -> i32 {
            items.iter().map(|x| x * 2).sum()
        }
    "#;
    let results = analyze_with_propagation(code);
    assert!(results.get("sum_doubled").unwrap().is_pure);
}

#[test]
fn test_option_chain_is_pure() {
    let code = r#"
        fn process_option(opt: Option<i32>) -> Option<i32> {
            opt.map(|x| x + 1).filter(|x| *x > 0)
        }
    "#;
    let results = analyze_with_propagation(code);
    assert!(results.get("process_option").unwrap().is_pure);
}
```

## Documentation Requirements

- Document known pure functions list
- Explain propagation algorithm in architecture docs

## Implementation Notes

- Start with Phase 1 (known pure std functions) for quick impact
- Phase 2 (call graph integration) requires careful cycle handling
- Consider caching propagated purity for incremental analysis
- Monitor for false positives (impure marked pure)

## Migration and Compatibility

- No breaking changes
- Functions may be upgraded from impure/low-confidence to pure
- This is accuracy improvement, not semantic change
