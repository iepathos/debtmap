---
number: 160
title: Macro Expansion Analysis for Purity Detection
category: optimization
priority: medium
status: draft
dependencies: [156, 157]
created: 2025-11-01
---

# Specification 160: Macro Expansion Analysis for Purity Detection

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 156, 157

## Context

**Current Limitation** (`purity_detector.rs:282-302`): Macros analyzed by name only, not expansion.

Problems:
- Custom I/O macros not detected (`my_println!`)
- `debug_assert!` incorrectly marked as I/O in all builds
- Macro-heavy code gets low confidence scores

## Objective

Analyze macro expansions to accurately determine side effects, with special handling for conditional compilation.

## Requirements

1. **Macro Classification**
   - Built-in known-pure macros (vec!, format!)
   - Built-in known-impure macros (println!, eprintln!)
   - Build-dependent macros (assert!, debug_assert!)
   - Unknown macros (require expansion or conservative assumption)

2. **Conditional Compilation**
   - `debug_assert!` / `assert!` pure in release, impure in debug
   - `cfg!()` macro awareness
   - Feature-gated macros

3. **Custom Macro Expansion**
   - Cache common macro expansions
   - Optional: integrate with `cargo expand` for unknown macros
   - Fallback to conservative classification

## Implementation

```rust
pub struct MacroClassifier {
    // Cached expansions
    expansions: DashMap<String, MacroPurity>,
}

#[derive(Debug, Clone)]
pub enum MacroPurity {
    Pure,
    Impure,
    Conditional { debug: MacroPurity, release: MacroPurity },
    Unknown,
}

impl MacroClassifier {
    pub fn classify(&self, mac: &syn::Macro) -> MacroPurity {
        let name = mac.path.to_token_stream().to_string();

        match name.as_str() {
            // Always pure
            "vec" | "format" | "concat" | "stringify" => MacroPurity::Pure,

            // Always impure
            "println" | "eprintln" | "dbg" | "write" | "writeln" =>
                MacroPurity::Impure,

            // Conditional
            "assert" | "debug_assert" => MacroPurity::Conditional {
                debug: MacroPurity::Impure,    // panic! in debug
                release: MacroPurity::Pure,     // compiled out
            },

            // Unknown - check cache or expand
            _ => self.classify_unknown(mac),
        }
    }

    fn classify_unknown(&self, mac: &syn::Macro) -> MacroPurity {
        let name = mac.path.to_token_stream().to_string();

        // Check cache
        if let Some(purity) = self.expansions.get(&name) {
            return purity.clone();
        }

        // Conservative: assume might have side effects
        MacroPurity::Unknown
    }
}

impl<'ast> Visit<'ast> for PurityDetector {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let purity = self.macro_classifier.classify(mac);

        match purity {
            MacroPurity::Impure => {
                self.side_effects.push(SideEffect::Macro(
                    mac.path.to_token_stream().to_string()
                ));
            }
            MacroPurity::Conditional { debug, release } => {
                if cfg!(debug_assertions) {
                    if matches!(debug, MacroPurity::Impure) {
                        self.side_effects.push(SideEffect::Macro(
                            mac.path.to_token_stream().to_string()
                        ));
                    }
                }
                // In release, use release classification
            }
            MacroPurity::Unknown => {
                // Conservative: assume might be impure
                self.confidence *= 0.9;
            }
            MacroPurity::Pure => {
                // No effect on purity
            }
        }

        visit::visit_macro(self, mac);
    }
}
```

## Testing

```rust
#[test]
fn test_debug_assert_pure_in_release() {
    let code = r#"
        fn check_bounds(x: usize) -> bool {
            debug_assert!(x < 100);
            x < 100
        }
    "#;

    #[cfg(not(debug_assertions))]
    {
        let analysis = analyze_purity(code).unwrap();
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }
}

#[test]
fn test_println_always_impure() {
    let code = r#"
        fn log_value(x: i32) {
            println!("{}", x);
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
}
```

## Documentation

Add macro classification table to docs:

| Macro | Debug Build | Release Build |
|-------|-------------|---------------|
| `println!` | Impure | Impure |
| `debug_assert!` | Impure | Pure |
| `vec!` | Pure | Pure |
| Custom | Conservative (unknown) | Conservative |
