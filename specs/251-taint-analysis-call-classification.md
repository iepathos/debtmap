---
number: 251
title: Taint Analysis Call Classification
category: optimization
priority: medium
status: draft
dependencies: [248]
created: 2025-12-12
---

# Specification 251: Taint Analysis Call Classification

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 248 (Enhanced Expression Variable Extraction)

## Context

### Current Problem

The `is_source_tainted` function in `TaintAnalysis` (`src/analysis/data_flow.rs:1092-1105`) treats all function calls conservatively:

```rust
fn is_source_tainted(source: &Rvalue, tainted_vars: &HashSet<VarId>) -> bool {
    match source {
        // ...
        Rvalue::Call { args, .. } => args.iter().any(|arg| tainted_vars.contains(arg)),
        // ...
    }
}
```

This is **too conservative** for pure function calls. Currently:
- If **any** argument is tainted, the call result is considered tainted
- This ignores that pure functions don't add new taint sources
- Unknown calls should be treated conservatively (potential side effects)

### Why Call Classification Matters

Consider:
```rust
fn example(data: &mut Vec<i32>) {
    data.push(1);                    // data is now tainted (mutated)
    let len = data.len();            // len() is PURE - should only be tainted if data is
    let doubled = len * 2;           // Should be tainted (from len)
    let clamped = std::cmp::min(doubled, 100);  // min() is PURE - properly propagates taint
    let result = read_config();      // IMPURE - result is tainted (unknown source)
}
```

With proper call classification:
- **Pure calls**: Taint propagates only through arguments
- **Impure calls**: Result is always tainted (introduces new taint source)

### Current False Positives

Without call classification, taint analysis produces false positives:
1. Pure helper functions incorrectly taint results
2. Standard library pure functions over-taint
3. Mathematical operations on clean data marked tainted

## Objective

Extend `is_source_tainted` to classify function calls and apply appropriate taint propagation rules based on function purity.

## Requirements

### Functional Requirements

1. **Pure Function Classification**
   - Maintain a set of known pure functions (std library, common crates)
   - Support user-configured pure functions
   - Allow attribute-based pure marking (`#[pure]` or similar)

2. **Taint Propagation Rules**
   - Pure functions: taint if any argument is tainted (current behavior)
   - Impure functions: always taint the result (new source)
   - Unknown functions: configurable (conservative default)

3. **Standard Library Coverage**
   - Arithmetic operations (`+`, `-`, `*`, `/`, `%`)
   - Comparison operations (`==`, `!=`, `<`, `>`, etc.)
   - Option/Result combinators (`map`, `and_then`, `unwrap_or`)
   - Iterator methods (pure ones: `len`, `is_empty`, `first`, `last`)
   - String operations (`len`, `is_empty`, `trim`, `to_string`)
   - Collection accessors (`get`, `contains`, `iter`)

4. **Taint Source Tracking**
   - Record when taint comes from impure call vs argument propagation
   - Enable better error messages and debugging

### Non-Functional Requirements

- **Performance**: <0.5ms overhead for classification lookup
- **Extensibility**: Easy to add new pure function classifications
- **Accuracy**: Reduce false positive taint rate by >50%

## Acceptance Criteria

- [ ] Pure function database with 100+ std library functions
- [ ] `is_source_tainted` uses classification for calls
- [ ] Pure calls only taint through arguments
- [ ] Impure calls always taint result
- [ ] Unknown calls configurable (default: conservative)
- [ ] TaintSource tracks call classification
- [ ] Tests verify correct propagation behavior
- [ ] Performance under 1ms for classification
- [ ] False positive rate reduced by >50%

## Technical Details

### Implementation Approach

#### Phase 1: Pure Function Database

```rust
use std::collections::HashSet;
use once_cell::sync::Lazy;

/// Database of known pure functions by qualified name.
static KNOWN_PURE_FUNCTIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();

    // Numeric operations
    set.insert("std::cmp::min");
    set.insert("std::cmp::max");
    set.insert("std::cmp::Ord::cmp");
    set.insert("std::cmp::PartialOrd::partial_cmp");
    set.insert("i32::abs");
    set.insert("i64::abs");
    set.insert("f32::abs");
    set.insert("f64::abs");
    set.insert("f32::sqrt");
    set.insert("f64::sqrt");
    set.insert("f32::sin");
    set.insert("f64::sin");
    set.insert("f32::cos");
    set.insert("f64::cos");

    // Option methods
    set.insert("Option::is_some");
    set.insert("Option::is_none");
    set.insert("Option::as_ref");
    set.insert("Option::as_mut");
    set.insert("Option::unwrap_or");
    set.insert("Option::unwrap_or_else");
    set.insert("Option::unwrap_or_default");
    set.insert("Option::map");
    set.insert("Option::and_then");
    set.insert("Option::or");
    set.insert("Option::or_else");
    set.insert("Option::filter");

    // Result methods
    set.insert("Result::is_ok");
    set.insert("Result::is_err");
    set.insert("Result::as_ref");
    set.insert("Result::map");
    set.insert("Result::map_err");
    set.insert("Result::and_then");
    set.insert("Result::unwrap_or");
    set.insert("Result::unwrap_or_else");
    set.insert("Result::unwrap_or_default");

    // String methods
    set.insert("str::len");
    set.insert("str::is_empty");
    set.insert("str::trim");
    set.insert("str::trim_start");
    set.insert("str::trim_end");
    set.insert("str::to_lowercase");
    set.insert("str::to_uppercase");
    set.insert("str::contains");
    set.insert("str::starts_with");
    set.insert("str::ends_with");
    set.insert("str::split");
    set.insert("str::chars");
    set.insert("str::bytes");
    set.insert("String::len");
    set.insert("String::is_empty");
    set.insert("String::as_str");
    set.insert("String::as_bytes");

    // Vec/slice methods (pure accessors)
    set.insert("Vec::len");
    set.insert("Vec::is_empty");
    set.insert("Vec::capacity");
    set.insert("Vec::first");
    set.insert("Vec::last");
    set.insert("Vec::get");
    set.insert("Vec::contains");
    set.insert("Vec::iter");
    set.insert("Vec::as_slice");
    set.insert("[T]::len");
    set.insert("[T]::is_empty");
    set.insert("[T]::first");
    set.insert("[T]::last");
    set.insert("[T]::get");
    set.insert("[T]::contains");
    set.insert("[T]::iter");

    // HashMap methods (pure accessors)
    set.insert("HashMap::len");
    set.insert("HashMap::is_empty");
    set.insert("HashMap::contains_key");
    set.insert("HashMap::get");
    set.insert("HashMap::keys");
    set.insert("HashMap::values");
    set.insert("HashMap::iter");

    // Iterator methods (pure)
    set.insert("Iterator::count");
    set.insert("Iterator::map");
    set.insert("Iterator::filter");
    set.insert("Iterator::filter_map");
    set.insert("Iterator::flat_map");
    set.insert("Iterator::flatten");
    set.insert("Iterator::take");
    set.insert("Iterator::skip");
    set.insert("Iterator::zip");
    set.insert("Iterator::enumerate");
    set.insert("Iterator::peekable");
    set.insert("Iterator::chain");
    set.insert("Iterator::fold");
    set.insert("Iterator::reduce");
    set.insert("Iterator::all");
    set.insert("Iterator::any");
    set.insert("Iterator::find");
    set.insert("Iterator::position");
    set.insert("Iterator::sum");
    set.insert("Iterator::product");
    set.insert("Iterator::collect");

    // Clone/Copy
    set.insert("Clone::clone");
    set.insert("ToOwned::to_owned");

    // Display/Debug (pure - just formatting)
    set.insert("Display::fmt");
    set.insert("Debug::fmt");
    set.insert("ToString::to_string");

    // Conversion traits
    set.insert("From::from");
    set.insert("Into::into");
    set.insert("TryFrom::try_from");
    set.insert("TryInto::try_into");
    set.insert("AsRef::as_ref");
    set.insert("AsMut::as_mut");

    // Default
    set.insert("Default::default");

    set
});

/// Known impure functions (side effects).
static KNOWN_IMPURE_FUNCTIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();

    // I/O
    set.insert("std::io::Read::read");
    set.insert("std::io::Write::write");
    set.insert("std::fs::read");
    set.insert("std::fs::write");
    set.insert("std::fs::File::open");
    set.insert("std::fs::File::create");
    set.insert("println");
    set.insert("print");
    set.insert("eprintln");
    set.insert("eprint");
    set.insert("dbg");

    // Network
    set.insert("std::net::TcpStream::connect");
    set.insert("std::net::UdpSocket::bind");

    // Random/Time
    set.insert("rand::random");
    set.insert("rand::thread_rng");
    set.insert("std::time::Instant::now");
    set.insert("std::time::SystemTime::now");

    // Mutation methods
    set.insert("Vec::push");
    set.insert("Vec::pop");
    set.insert("Vec::insert");
    set.insert("Vec::remove");
    set.insert("Vec::clear");
    set.insert("Vec::truncate");
    set.insert("Vec::extend");
    set.insert("HashMap::insert");
    set.insert("HashMap::remove");
    set.insert("HashMap::clear");

    set
});
```

#### Phase 2: Call Classification Logic

```rust
/// Classification result for a function call
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallPurity {
    /// Known pure function - only taints through arguments
    Pure,
    /// Known impure function - always taints result
    Impure,
    /// Unknown function - use configured default
    Unknown,
}

/// Classify a function call by its name.
pub fn classify_call(func_name: &str) -> CallPurity {
    // Check known pure functions
    if is_known_pure(func_name) {
        return CallPurity::Pure;
    }

    // Check known impure functions
    if is_known_impure(func_name) {
        return CallPurity::Impure;
    }

    // Unknown - will use configured default
    CallPurity::Unknown
}

fn is_known_pure(func_name: &str) -> bool {
    // Exact match
    if KNOWN_PURE_FUNCTIONS.contains(func_name) {
        return true;
    }

    // Method name match (for unqualified calls)
    let method_name = func_name.rsplit("::").next().unwrap_or(func_name);

    // Pure method patterns
    let pure_method_patterns = [
        "len", "is_empty", "is_some", "is_none", "is_ok", "is_err",
        "as_ref", "as_mut", "as_str", "as_slice", "as_bytes",
        "get", "first", "last", "contains",
        "clone", "to_owned", "to_string",
        "map", "filter", "and_then", "or_else", "unwrap_or",
        "iter", "into_iter", "chars", "bytes",
        "trim", "to_lowercase", "to_uppercase",
        "abs", "sqrt", "sin", "cos", "min", "max",
        "cmp", "partial_cmp", "eq", "ne", "lt", "le", "gt", "ge",
    ];

    pure_method_patterns.contains(&method_name)
}

fn is_known_impure(func_name: &str) -> bool {
    // Exact match
    if KNOWN_IMPURE_FUNCTIONS.contains(func_name) {
        return true;
    }

    // Method name match
    let method_name = func_name.rsplit("::").next().unwrap_or(func_name);

    // Impure method patterns
    let impure_method_patterns = [
        "push", "pop", "insert", "remove", "clear", "truncate",
        "extend", "append", "drain", "retain",
        "read", "write", "flush", "seek",
        "connect", "bind", "listen", "accept",
        "spawn", "join",
        "lock", "unlock",
        "now", "elapsed",
        "random", "gen", "shuffle",
    ];

    impure_method_patterns.contains(&method_name)
}
```

#### Phase 3: Enhanced is_source_tainted

```rust
impl TaintAnalysis {
    /// Configuration for unknown call handling.
    #[derive(Debug, Clone, Copy)]
    pub enum UnknownCallBehavior {
        /// Conservative: unknown calls always taint (current behavior)
        Conservative,
        /// Optimistic: unknown calls only taint through arguments
        Optimistic,
    }

    fn is_source_tainted_with_classification(
        source: &Rvalue,
        tainted_vars: &HashSet<VarId>,
        unknown_behavior: UnknownCallBehavior,
    ) -> (bool, Option<TaintReason>) {
        match source {
            Rvalue::Use(var) => {
                let tainted = tainted_vars.contains(var);
                (tainted, tainted.then_some(TaintReason::DirectUse(*var)))
            }

            Rvalue::BinaryOp { left, right, .. } => {
                let left_tainted = tainted_vars.contains(left);
                let right_tainted = tainted_vars.contains(right);
                let tainted = left_tainted || right_tainted;
                (tainted, tainted.then_some(TaintReason::BinaryOp {
                    left_tainted,
                    right_tainted,
                }))
            }

            Rvalue::UnaryOp { operand, .. } => {
                let tainted = tainted_vars.contains(operand);
                (tainted, tainted.then_some(TaintReason::UnaryOp(*operand)))
            }

            Rvalue::Call { func, args } => {
                let classification = classify_call(func);
                let args_tainted = args.iter().any(|arg| tainted_vars.contains(arg));

                match classification {
                    CallPurity::Pure => {
                        // Pure: only taint through arguments
                        (args_tainted, args_tainted.then_some(TaintReason::PureCall {
                            func: func.clone(),
                            tainted_args: args.iter()
                                .filter(|a| tainted_vars.contains(a))
                                .copied()
                                .collect(),
                        }))
                    }
                    CallPurity::Impure => {
                        // Impure: always taint (new source)
                        (true, Some(TaintReason::ImpureCall {
                            func: func.clone(),
                        }))
                    }
                    CallPurity::Unknown => {
                        match unknown_behavior {
                            UnknownCallBehavior::Conservative => {
                                // Conservative: treat as impure
                                (true, Some(TaintReason::UnknownCall {
                                    func: func.clone(),
                                }))
                            }
                            UnknownCallBehavior::Optimistic => {
                                // Optimistic: treat as pure
                                (args_tainted, args_tainted.then_some(TaintReason::UnknownCall {
                                    func: func.clone(),
                                }))
                            }
                        }
                    }
                }
            }

            Rvalue::FieldAccess { base, .. } | Rvalue::Ref { var: base, .. } => {
                let tainted = tainted_vars.contains(base);
                (tainted, tainted.then_some(TaintReason::FieldAccess(*base)))
            }

            Rvalue::Constant => (false, None),
        }
    }
}

/// Reason why a value is tainted.
#[derive(Debug, Clone)]
pub enum TaintReason {
    DirectUse(VarId),
    BinaryOp { left_tainted: bool, right_tainted: bool },
    UnaryOp(VarId),
    PureCall { func: String, tainted_args: Vec<VarId> },
    ImpureCall { func: String },
    UnknownCall { func: String },
    FieldAccess(VarId),
}
```

#### Phase 4: Updated analyze() Method

```rust
impl TaintAnalysis {
    pub fn analyze(
        cfg: &ControlFlowGraph,
        liveness: &LivenessInfo,
        escape: &EscapeAnalysis,
    ) -> Self {
        Self::analyze_with_config(
            cfg,
            liveness,
            escape,
            UnknownCallBehavior::Conservative, // Default: conservative
        )
    }

    pub fn analyze_with_config(
        cfg: &ControlFlowGraph,
        liveness: &LivenessInfo,
        escape: &EscapeAnalysis,
        unknown_behavior: UnknownCallBehavior,
    ) -> Self {
        let mut tainted_vars = HashSet::new();
        let mut taint_sources = HashMap::new();

        let mut changed = true;
        while changed {
            changed = false;

            for block in &cfg.blocks {
                for stmt in &block.statements {
                    match stmt {
                        Statement::Assign { target, source, .. } => {
                            let (is_tainted, reason) = Self::is_source_tainted_with_classification(
                                source,
                                &tainted_vars,
                                unknown_behavior,
                            );

                            if is_tainted && tainted_vars.insert(*target) {
                                changed = true;
                                if let Some(reason) = reason {
                                    taint_sources.insert(*target, Self::reason_to_source(reason));
                                }
                            }
                        }
                        Statement::Declare {
                            var,
                            init: Some(init),
                            ..
                        } => {
                            let (is_tainted, reason) = Self::is_source_tainted_with_classification(
                                init,
                                &tainted_vars,
                                unknown_behavior,
                            );

                            if is_tainted && tainted_vars.insert(*var) {
                                changed = true;
                                if let Some(reason) = reason {
                                    taint_sources.insert(*var, Self::reason_to_source(reason));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        tainted_vars.retain(|var| !liveness.dead_stores.contains(var));

        let return_tainted = tainted_vars
            .iter()
            .any(|var| escape.return_dependencies.contains(var));

        TaintAnalysis {
            tainted_vars,
            taint_sources,
            return_tainted,
        }
    }

    fn reason_to_source(reason: TaintReason) -> TaintSource {
        match reason {
            TaintReason::ImpureCall { func } => TaintSource::ImpureCall {
                callee: func,
                line: None,
            },
            TaintReason::UnknownCall { func } => TaintSource::ImpureCall {
                callee: format!("unknown:{}", func),
                line: None,
            },
            _ => TaintSource::LocalMutation { line: None },
        }
    }
}
```

### Architecture Changes

1. **New module**: `call_classification.rs` (or inline in data_flow.rs)
2. **Static databases**: KNOWN_PURE_FUNCTIONS, KNOWN_IMPURE_FUNCTIONS
3. **New enum**: `CallPurity`, `TaintReason`, `UnknownCallBehavior`
4. **Enhanced method**: `is_source_tainted_with_classification`

### Data Structures

```rust
pub enum CallPurity { Pure, Impure, Unknown }
pub enum UnknownCallBehavior { Conservative, Optimistic }
pub enum TaintReason { DirectUse, BinaryOp, UnaryOp, PureCall, ImpureCall, UnknownCall, FieldAccess }
```

### APIs and Interfaces

- `classify_call(func_name: &str) -> CallPurity`
- `TaintAnalysis::analyze_with_config(cfg, liveness, escape, behavior) -> Self`
- Backward compatible: `analyze()` uses conservative default

## Dependencies

- **Prerequisites**: Spec 248 (for proper func name extraction in Rvalue::Call)
- **Affected Components**: `src/analysis/data_flow.rs`
- **External Dependencies**: `once_cell` for lazy static (already in deps)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_pure_call_propagates_taint() {
    // len() on tainted vec should produce tainted result
    let block: Block = parse_quote!({
        let mut v = vec![1, 2, 3];
        v.push(4);  // v is now tainted
        let len = v.len();  // Pure - len should be tainted from v
        len
    });

    let cfg = ControlFlowGraph::from_block(&block);
    let liveness = LivenessInfo::analyze(&cfg);
    let escape = EscapeAnalysis::analyze(&cfg);
    let taint = TaintAnalysis::analyze(&cfg, &liveness, &escape);

    // len should be tainted (derived from tainted v)
    assert!(taint.return_tainted);
}

#[test]
fn test_pure_call_no_taint_without_tainted_args() {
    // len() on clean vec should not taint result
    let block: Block = parse_quote!({
        let v = vec![1, 2, 3];
        let len = v.len();
        len
    });

    let cfg = ControlFlowGraph::from_block(&block);
    let liveness = LivenessInfo::analyze(&cfg);
    let escape = EscapeAnalysis::analyze(&cfg);
    let taint = TaintAnalysis::analyze(&cfg, &liveness, &escape);

    // No mutations -> no taint
    assert!(!taint.return_tainted);
}

#[test]
fn test_impure_call_always_taints() {
    let block: Block = parse_quote!({
        let result = std::fs::read("file.txt");
        result
    });

    let cfg = ControlFlowGraph::from_block(&block);
    let liveness = LivenessInfo::analyze(&cfg);
    let escape = EscapeAnalysis::analyze(&cfg);
    let taint = TaintAnalysis::analyze(&cfg, &liveness, &escape);

    // I/O call should taint result
    assert!(taint.return_tainted);
}

#[test]
fn test_classify_std_functions() {
    assert_eq!(classify_call("Vec::len"), CallPurity::Pure);
    assert_eq!(classify_call("Option::map"), CallPurity::Pure);
    assert_eq!(classify_call("Vec::push"), CallPurity::Impure);
    assert_eq!(classify_call("std::fs::read"), CallPurity::Impure);
    assert_eq!(classify_call("my_custom_func"), CallPurity::Unknown);
}

#[test]
fn test_method_pattern_matching() {
    // Pure patterns
    assert!(is_known_pure("len"));
    assert!(is_known_pure("is_empty"));
    assert!(is_known_pure("clone"));
    assert!(is_known_pure("to_string"));

    // Impure patterns
    assert!(is_known_impure("push"));
    assert!(is_known_impure("write"));
    assert!(is_known_impure("now"));
}

#[test]
fn test_unknown_conservative_vs_optimistic() {
    let block: Block = parse_quote!({
        let x = unknown_function();
        x
    });

    let cfg = ControlFlowGraph::from_block(&block);
    let liveness = LivenessInfo::analyze(&cfg);
    let escape = EscapeAnalysis::analyze(&cfg);

    // Conservative: unknown taints
    let taint_conservative = TaintAnalysis::analyze_with_config(
        &cfg, &liveness, &escape,
        UnknownCallBehavior::Conservative,
    );
    assert!(taint_conservative.return_tainted);

    // Optimistic: unknown doesn't taint (no tainted args)
    let taint_optimistic = TaintAnalysis::analyze_with_config(
        &cfg, &liveness, &escape,
        UnknownCallBehavior::Optimistic,
    );
    assert!(!taint_optimistic.return_tainted);
}
```

### Performance Tests

```rust
#[test]
fn test_classification_performance() {
    use std::time::Instant;

    let funcs = vec![
        "Vec::len", "Option::map", "std::fs::read", "unknown_func",
        "Iterator::collect", "HashMap::insert", "String::trim",
    ];

    let start = Instant::now();
    for _ in 0..10000 {
        for func in &funcs {
            let _ = classify_call(func);
        }
    }
    let elapsed = start.elapsed();

    // 70000 classifications should complete in <100ms
    assert!(elapsed.as_millis() < 100, "Took {:?}", elapsed);
}
```

## Documentation Requirements

- **Code Documentation**: Document classify_call and all new types
- **User Documentation**: No changes (internal optimization)
- **Architecture Updates**: Document taint analysis improvements

## Implementation Notes

### Extensibility

To add new pure functions:
1. Add to `KNOWN_PURE_FUNCTIONS` set
2. Or add pattern to `pure_method_patterns` array

### Future Enhancements

1. **Project-specific purity**: Allow per-project pure function lists
2. **Attribute detection**: Detect `#[pure]` attributes
3. **Inter-procedural**: Use purity analysis results for project functions

## Migration and Compatibility

- **Backward compatible**: Default behavior unchanged (conservative)
- **New API optional**: `analyze_with_config` for explicit control
- **No breaking changes**: Existing code continues to work
