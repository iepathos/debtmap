---
number: 161
title: Refined Unsafe Block Analysis
category: optimization
priority: low
status: draft
dependencies: [157]
created: 2025-11-01
---

# Specification 161: Refined Unsafe Block Analysis

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: Spec 157 (Mutation Distinction)

## Context

**Over-Conservative**: All `unsafe` blocks currently marked impure (`purity_detector.rs:248-251`), even pure operations like transmute or arithmetic.

## Objective

Classify unsafe operations to distinguish pure unsafe (transmute, pointer arithmetic) from impure unsafe (FFI, pointer writes, mutable statics).

## Requirements

1. **Unsafe Operation Classification**
   - Pure: transmute, pointer reads, union field reads, arithmetic
   - Impure: FFI calls, pointer writes, mutable statics, union field writes

2. **Confidence Adjustment**
   - Pure unsafe: reduce confidence by 15% (harder to verify)
   - Impure unsafe: mark as impure with high confidence

## Implementation

```rust
#[derive(Debug, Clone)]
pub enum UnsafeOp {
    // Pure operations
    Transmute,
    RawPointerRead,
    UnionFieldRead,
    PointerArithmetic,

    // Impure operations
    FFICall,
    RawPointerWrite,
    MutableStatic,
    UnionFieldWrite,
}

impl PurityDetector {
    fn analyze_unsafe_block(&mut self, block: &syn::Block) {
        let ops = self.classify_unsafe_operations(block);

        let has_impure = ops.iter().any(|op| matches!(op,
            UnsafeOp::FFICall |
            UnsafeOp::RawPointerWrite |
            UnsafeOp::MutableStatic |
            UnsafeOp::UnionFieldWrite
        ));

        if has_impure {
            self.modifies_external_state = true;
            self.side_effects.push(SideEffect::UnsafeImpure);
        } else {
            // Pure unsafe - reduce confidence slightly
            self.has_pure_unsafe = true;
            self.confidence *= 0.85;
        }
    }
}
```

## Testing

```rust
#[test]
fn test_transmute_is_pure_unsafe() {
    let code = r#"
        fn bytes_to_u32(bytes: [u8; 4]) -> u32 {
            unsafe { std::mem::transmute(bytes) }
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    assert!(analysis.purity_confidence > 0.80); // Reduced from 1.0
}

#[test]
fn test_ffi_call_is_impure() {
    let code = r#"
        extern "C" { fn external_func(); }

        fn call_external() {
            unsafe { external_func(); }
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
}
```
