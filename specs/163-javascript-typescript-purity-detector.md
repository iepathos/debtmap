---
number: 163
title: JavaScript/TypeScript Purity Detector
category: foundation
priority: medium
status: draft
dependencies: [156, 157]
created: 2025-11-01
---

# Specification 163: JavaScript/TypeScript Purity Detector

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 156, 157

## Context

**Missing Feature**: No JavaScript/TypeScript purity analysis exists. Current coverage is Rust and Python only.

**Need**: Extend debtmap's purity analysis to JS/TS codebases.

## Objective

Implement purity detector for JavaScript/TypeScript using `swc_ecma_ast` parser.

## Requirements

1. **AST Parsing**
   - Use `swc_ecma_parser` for JS/TS
   - Support modern ES6+ syntax
   - Handle TypeScript type annotations

2. **Side Effect Detection**
   - DOM access (`document.*`, `window.*`)
   - Network calls (`fetch`, `XMLHttpRequest`)
   - Console I/O (`console.log`, `console.error`)
   - Non-deterministic operations (`Date.now()`, `Math.random()`)
   - LocalStorage/SessionStorage
   - Global variable mutations

3. **Pure Function Recognition**
   - Array methods (map, filter, reduce)
   - String methods (toLowerCase, slice, etc.)
   - Math operations (except random)
   - Object immutable operations

## Implementation

```rust
// src/analyzers/typescript_purity.rs

use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};

pub struct TSPurityDetector {
    side_effects: Vec<SideEffect>,
    accesses_dom: bool,
    accesses_window: bool,
    has_network: bool,
    mutates_params: bool,
}

impl Visit for TSPurityDetector {
    fn visit_call_expr(&mut self, call: &CallExpr) {
        match &call.callee {
            Callee::Expr(box Expr::Ident(ident)) => {
                match ident.sym.as_ref() {
                    "fetch" | "XMLHttpRequest" => {
                        self.has_network = true;
                        self.side_effects.push(SideEffect::Network);
                    }
                    _ => {}
                }
            }
            Callee::Expr(box Expr::Member(member)) => {
                self.check_member_call(member);
            }
            _ => {}
        }
        call.visit_children_with(self);
    }

    fn check_member_call(&mut self, member: &MemberExpr) {
        // console.log, console.error, etc.
        if self.is_console_call(member) {
            self.side_effects.push(SideEffect::Logging);
        }

        // document.*, window.*
        if self.is_dom_access(member) {
            self.accesses_dom = true;
            self.side_effects.push(SideEffect::DomAccess);
        }

        // Date.now(), Math.random()
        if self.is_nondeterministic(member) {
            self.side_effects.push(SideEffect::NonDeterministic);
        }
    }

    fn visit_member_expr(&mut self, member: &MemberExpr) {
        // Check for mutations: obj.field = value
        // (handled in assignment visitor)
        member.visit_children_with(self);
    }
}

const PURE_JS_FUNCTIONS: &[&str] = &[
    // Array
    "map", "filter", "reduce", "slice", "concat",
    // String
    "toLowerCase", "toUpperCase", "trim", "substring",
    // Math
    "Math.abs", "Math.floor", "Math.ceil", "Math.max",
    // Object
    "Object.keys", "Object.values", "Object.entries",
];
```

## Testing

```rust
#[test]
fn test_pure_array_map() {
    let code = "
        function double(nums) {
            return nums.map(x => x * 2);
        }
    ";

    let analysis = analyze_ts_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_console_log_impure() {
    let code = "
        function log(msg) {
            console.log(msg);
        }
    ";

    let analysis = analyze_ts_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
}

#[test]
fn test_fetch_impure() {
    let code = "
        async function getData() {
            return await fetch('/api/data');
        }
    ";

    let analysis = analyze_ts_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
}
```

## Documentation

Add JS/TS examples to purity docs:

```markdown
## JavaScript/TypeScript Support

Debtmap analyzes purity in JavaScript and TypeScript:

**Pure**:
```javascript
function calculate(nums) {
    return nums.map(x => x * 2).filter(x => x > 10);
}
```

**Impure**:
```javascript
function fetchData() {
    return fetch('/api/data');  // Network I/O
}

function logResult(x) {
    console.log(x);  // Console I/O
}
```
```

## Migration

- Add `swc_ecma_parser` dependency
- Integrate TS detector into analysis pipeline
- Add JS/TS test corpus
- Update documentation with JS/TS examples
