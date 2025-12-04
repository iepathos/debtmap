//! # God Object Detection Thresholds
//!
//! Constants and configuration values for god object detection.
//!
//! ## Stillwater Architecture
//!
//! This module contains pure configuration - no logic or side effects.
//! All constants are derived from empirical analysis of Rust codebases.
//!
//! ## Threshold Categories
//!
//! - **Hybrid Detection**: Constants for detecting god modules with mixed paradigms
//! - **Reserved Keywords**: Language keywords to avoid in generated names

/// Minimum standalone functions required to trigger hybrid detection.
///
/// Files with fewer standalone functions are assumed to have helper
/// functions that complement the primary struct's impl methods.
///
/// Chosen based on analysis of Rust projects: 50+ functions typically
/// indicates a functional module rather than helpers.
pub const HYBRID_STANDALONE_THRESHOLD: usize = 50;

/// Dominance ratio: standalone functions must exceed impl methods by this factor.
///
/// Prevents false positives for balanced OOP/functional modules. A ratio of 3:1
/// ensures standalone functions truly dominate the file's purpose.
///
/// Examples:
/// - 60 standalone, 15 impl → 60 > 45? Yes → Hybrid
/// - 60 standalone, 25 impl → 60 > 75? No → God Class
pub const HYBRID_DOMINANCE_RATIO: usize = 3;

/// Reserved keywords across Rust and Python.
///
/// Used to avoid generating module names that conflict with language keywords.
pub const RESERVED_KEYWORDS: &[&str] = &[
    // Rust
    "mod", "pub", "use", "type", "impl", "trait", "fn", "let", "mut", "const", "static", "self",
    "Self", "super", "crate", "as", "break", "continue", "else", "enum", "extern", "false", "for",
    "if", "in", "loop", "match", "move", "ref", "return", "struct", "true", "unsafe", "while",
    "where", "async", "await", "dyn", // Python
    "import", "from", "def", "class", "if", "elif", "else", "for", "while", "try", "except",
    "finally", "with", "lambda", "yield", "return", "pass", "break", "continue", "raise", "assert",
    "global", "nonlocal", "del", "and", "or", "not", "is", "in", "None", "True", "False",
];

/// Check if a name is a reserved keyword in any supported language.
pub fn is_reserved_keyword(name: &str) -> bool {
    RESERVED_KEYWORDS.contains(&name)
}

/// Ensure the name is not a reserved keyword by appending "_module" if needed.
pub fn ensure_not_reserved(mut name: String) -> String {
    if is_reserved_keyword(&name) {
        name.push_str("_module");
    }
    name
}
