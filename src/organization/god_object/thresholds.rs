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

/// Configuration thresholds for god object detection
#[derive(Debug, Clone)]
pub struct GodObjectThresholds {
    pub max_methods: usize,
    pub max_fields: usize,
    pub max_traits: usize,
    pub max_lines: usize,
    pub max_complexity: u32,
}

impl Default for GodObjectThresholds {
    fn default() -> Self {
        Self {
            max_methods: 20,
            max_fields: 15,
            max_traits: 5,
            max_lines: 1000,
            max_complexity: 200,
        }
    }
}

impl GodObjectThresholds {
    pub fn for_rust() -> Self {
        Self {
            max_methods: 20,
            max_fields: 15,
            max_traits: 5,
            max_lines: 1000,
            max_complexity: 200,
        }
    }

    pub fn for_python() -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_traits: 3,
            max_lines: 500,
            max_complexity: 150,
        }
    }

    pub fn for_javascript() -> Self {
        Self {
            max_methods: 15,
            max_fields: 20,
            max_traits: 3,
            max_lines: 500,
            max_complexity: 150,
        }
    }
}

// ============================================================================
// Spec 211: Complexity Thresholds for Method Complexity Weighting
// ============================================================================

/// Configuration thresholds for method complexity weighting in God Object scoring (Spec 211).
///
/// These thresholds define the "expected" complexity levels. Methods with complexity
/// at or below these thresholds are considered normal; methods exceeding them
/// increase the God Object score.
///
/// ## Usage
///
/// Used by `calculate_complexity_factor` to produce a multiplier for God Object scoring.
///
/// ## Stillwater Principle: Pure Configuration
///
/// This struct contains only configuration data - no behavior.
#[derive(Debug, Clone)]
pub struct ComplexityThresholds {
    /// Target average cyclomatic complexity per method.
    ///
    /// Methods averaging above this contribute to higher scores.
    /// Default: 5.0 (typical for clean code)
    pub target_avg_complexity: f64,

    /// Maximum acceptable cyclomatic complexity for any single method.
    ///
    /// Methods exceeding this are considered excessively complex.
    /// Default: 15 (matches typical lint warnings)
    pub max_method_complexity: u32,

    /// Target total complexity budget for all methods.
    ///
    /// Calculated as max_methods * target_avg_complexity.
    /// Default: 75.0 (15 methods * 5 avg)
    pub target_total_complexity: f64,
}

impl Default for ComplexityThresholds {
    fn default() -> Self {
        Self {
            target_avg_complexity: 5.0,
            max_method_complexity: 15,
            target_total_complexity: 75.0, // 15 methods * 5 avg
        }
    }
}

impl ComplexityThresholds {
    /// Create thresholds tuned for Rust codebases.
    ///
    /// Rust tends to have more complex pattern matching, so thresholds
    /// are slightly higher than other languages.
    pub fn for_rust() -> Self {
        Self {
            target_avg_complexity: 5.0,
            max_method_complexity: 15,
            target_total_complexity: 100.0, // 20 methods * 5 avg
        }
    }

    /// Create thresholds tuned for Python codebases.
    pub fn for_python() -> Self {
        Self {
            target_avg_complexity: 4.0,
            max_method_complexity: 12,
            target_total_complexity: 60.0, // 15 methods * 4 avg
        }
    }

    /// Create thresholds tuned for JavaScript/TypeScript codebases.
    pub fn for_javascript() -> Self {
        Self {
            target_avg_complexity: 4.0,
            max_method_complexity: 12,
            target_total_complexity: 60.0,
        }
    }
}
