//! Macro handling for purity analysis
//!
//! Classification and handling of macros during purity analysis.

use crate::analyzers::custom_macro_analyzer::MacroPurity;

/// Classify a built-in macro for purity
pub fn classify_builtin(name: &str) -> Option<MacroPurity> {
    match name {
        // Pure macros - no side effects
        "vec" | "format" | "concat" | "stringify" | "matches" | "include_str" | "include_bytes"
        | "env" | "option_env" => Some(MacroPurity::Pure),

        // I/O macros - always impure
        "println" | "eprintln" | "print" | "eprint" | "dbg" | "write" | "writeln" => {
            Some(MacroPurity::Impure)
        }

        // Panic macros - always impure
        "panic" | "unimplemented" | "unreachable" | "todo" => Some(MacroPurity::Impure),

        // Debug-only assertions - conditional purity
        "debug_assert" | "debug_assert_eq" | "debug_assert_ne" => Some(MacroPurity::Conditional {
            debug: Box::new(MacroPurity::Impure),
            release: Box::new(MacroPurity::Pure),
        }),

        // Regular assertions - always impure (panic on failure)
        "assert" | "assert_eq" | "assert_ne" => Some(MacroPurity::Impure),

        _ => None,
    }
}

/// Extract the last segment of a macro path
/// e.g., "std::println" -> "println", "assert_eq" -> "assert_eq"
pub fn extract_macro_name(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|seg| seg.ident.to_string())
        .unwrap_or_default()
}

/// State updates from applying macro purity
pub struct MacroPurityEffects {
    pub has_side_effects: bool,
    pub has_io_operations: bool,
    pub unknown_macro_detected: bool,
}

/// Apply macro purity classification to determine effects
pub fn apply_purity(purity: MacroPurity) -> MacroPurityEffects {
    match purity {
        MacroPurity::Impure => MacroPurityEffects {
            has_side_effects: true,
            has_io_operations: true,
            unknown_macro_detected: false,
        },
        MacroPurity::Conditional { debug, release } => {
            // Apply purity based on build configuration
            #[cfg(debug_assertions)]
            {
                let _ = release; // Mark as used to avoid warnings
                apply_purity(*debug)
            }
            #[cfg(not(debug_assertions))]
            {
                let _ = debug; // Mark as used to avoid warnings
                apply_purity(*release)
            }
        }
        MacroPurity::Unknown { confidence: _ } => MacroPurityEffects {
            has_side_effects: false,
            has_io_operations: false,
            unknown_macro_detected: true,
        },
        MacroPurity::Pure => MacroPurityEffects {
            has_side_effects: false,
            has_io_operations: false,
            unknown_macro_detected: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_builtin_pure() {
        assert!(matches!(classify_builtin("vec"), Some(MacroPurity::Pure)));
        assert!(matches!(
            classify_builtin("format"),
            Some(MacroPurity::Pure)
        ));
    }

    #[test]
    fn test_classify_builtin_impure() {
        assert!(matches!(
            classify_builtin("println"),
            Some(MacroPurity::Impure)
        ));
        assert!(matches!(
            classify_builtin("panic"),
            Some(MacroPurity::Impure)
        ));
        assert!(matches!(
            classify_builtin("assert"),
            Some(MacroPurity::Impure)
        ));
    }

    #[test]
    fn test_classify_builtin_conditional() {
        assert!(matches!(
            classify_builtin("debug_assert"),
            Some(MacroPurity::Conditional { .. })
        ));
    }

    #[test]
    fn test_classify_builtin_unknown() {
        assert!(classify_builtin("unknown_macro").is_none());
    }

    #[test]
    fn test_apply_purity_impure() {
        let effects = apply_purity(MacroPurity::Impure);
        assert!(effects.has_side_effects);
        assert!(effects.has_io_operations);
        assert!(!effects.unknown_macro_detected);
    }

    #[test]
    fn test_apply_purity_pure() {
        let effects = apply_purity(MacroPurity::Pure);
        assert!(!effects.has_side_effects);
        assert!(!effects.has_io_operations);
        assert!(!effects.unknown_macro_detected);
    }

    #[test]
    fn test_apply_purity_unknown() {
        let effects = apply_purity(MacroPurity::Unknown { confidence: 0.5 });
        assert!(!effects.has_side_effects);
        assert!(!effects.has_io_operations);
        assert!(effects.unknown_macro_detected);
    }
}
