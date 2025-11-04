//! Custom Macro Heuristic Analysis
//!
//! Provides heuristic analysis of custom macro bodies to determine purity
//! without requiring full macro expansion.
//!
//! # Overview
//!
//! - Analyzes macro body tokens for known patterns
//! - Detects impure macro calls within expansions
//! - Handles nested macro invocations
//! - Provides confidence scoring for uncertain cases
//!
//! # Limitations
//!
//! - Cannot analyze procedural macros (no body to analyze)
//! - May miss indirect calls through variables/functions
//! - Complex conditional logic may be misclassified
//! - Cross-crate macros require dependency analysis

/// Purity classification for custom macros
#[derive(Debug, Clone, PartialEq)]
pub enum MacroPurity {
    /// The macro is pure (no side effects)
    Pure,

    /// The macro is impure (has side effects)
    Impure,

    /// The macro's purity depends on configuration
    Conditional {
        /// Purity in debug builds
        debug: Box<MacroPurity>,
        /// Purity in release builds
        release: Box<MacroPurity>,
    },

    /// Unable to determine purity with certainty
    Unknown {
        /// Confidence score (0.0 to 1.0)
        confidence: f32,
    },
}

/// Heuristic analyzer for custom macro bodies
pub struct CustomMacroAnalyzer {
    /// Known impure patterns
    impure_patterns: Vec<&'static str>,

    /// Known pure patterns
    pure_patterns: Vec<&'static str>,

    /// Conditional patterns
    conditional_patterns: Vec<&'static str>,
}

impl Default for CustomMacroAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomMacroAnalyzer {
    /// Create a new analyzer with default patterns
    pub fn new() -> Self {
        Self {
            impure_patterns: vec![
                "println!",
                "eprintln!",
                "print!",
                "eprint!",
                "dbg!",
                "write!",
                "writeln!",
                "panic!",
                "unimplemented!",
                "unreachable!",
                "todo!",
                "std::io::",
                "File::",
                "stdout",
                "stderr",
            ],
            pure_patterns: vec![
                "vec!",
                "format!",
                "concat!",
                "stringify!",
                "matches!",
                "include_str!",
                "include_bytes!",
            ],
            conditional_patterns: vec![
                "debug_assert!",
                "debug_assert_eq!",
                "debug_assert_ne!",
                "#[cfg(debug_assertions)]",
                "cfg!(debug_assertions)",
            ],
        }
    }

    /// Analyze a custom macro body
    pub fn analyze(&self, body: &str) -> MacroPurity {
        // Phase 1: Check for impure patterns
        if self.contains_impure_patterns(body) {
            return MacroPurity::Impure;
        }

        // Phase 2: Check for conditional patterns
        if self.contains_conditional_patterns(body) {
            return MacroPurity::Conditional {
                debug: Box::new(MacroPurity::Impure),
                release: Box::new(MacroPurity::Pure),
            };
        }

        // Phase 3: Check for pure patterns
        if self.contains_only_pure_patterns(body) {
            return MacroPurity::Pure;
        }

        // Phase 4: Complex analysis
        self.analyze_complex(body)
    }

    fn contains_impure_patterns(&self, body: &str) -> bool {
        // Normalize body by removing spaces for better pattern matching
        let normalized = body.replace(' ', "");
        self.impure_patterns
            .iter()
            .map(|p| p.replace(' ', ""))
            .any(|pattern| normalized.contains(&pattern))
    }

    fn contains_conditional_patterns(&self, body: &str) -> bool {
        // Conditional patterns might have whitespace variations
        let normalized = body.replace(' ', "");
        self.conditional_patterns
            .iter()
            .map(|p| p.replace(' ', ""))
            .any(|pattern| normalized.contains(&pattern))
    }

    fn contains_only_pure_patterns(&self, body: &str) -> bool {
        // Check if body only contains known pure constructs
        let normalized = body.replace(' ', "");
        let has_pure = self
            .pure_patterns
            .iter()
            .map(|p| p.replace(' ', ""))
            .any(|pattern| normalized.contains(&pattern));
        let has_impure = self.contains_impure_patterns(body);

        has_pure && !has_impure
    }

    fn analyze_complex(&self, body: &str) -> MacroPurity {
        // Try to parse as expression to detect structure
        if let Ok(tokens) = body.parse::<proc_macro2::TokenStream>() {
            if syn::parse2::<syn::Expr>(tokens).is_ok() {
                // Successfully parsed as expression - likely pure computation
                return MacroPurity::Unknown { confidence: 0.8 };
            }
        }

        // Check for suspicious keywords
        let suspicious_keywords = ["unsafe", "transmute", "ptr::", "mut"];
        if suspicious_keywords.iter().any(|kw| body.contains(kw)) {
            return MacroPurity::Unknown { confidence: 0.5 };
        }

        // Default: unknown with moderate confidence
        MacroPurity::Unknown { confidence: 0.7 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_macro_with_io() {
        let analyzer = CustomMacroAnalyzer::new();

        let body = r#"
            eprintln!("[LOG] {}", format!($($arg)*))
        "#;

        let purity = analyzer.analyze(body);
        assert_eq!(purity, MacroPurity::Impure);
    }

    #[test]
    fn test_custom_macro_conditional() {
        let analyzer = CustomMacroAnalyzer::new();

        let body = r#"
            #[cfg(debug_assertions)]
            debug_assert!(x > 0);
            x * 2
        "#;

        let purity = analyzer.analyze(body);
        assert!(matches!(purity, MacroPurity::Conditional { .. }));
    }

    #[test]
    fn test_custom_macro_pure() {
        let analyzer = CustomMacroAnalyzer::new();

        let body = r#"
            vec![$($elem),*]
        "#;

        let purity = analyzer.analyze(body);
        assert_eq!(purity, MacroPurity::Pure);
    }

    #[test]
    fn test_complex_macro_unknown() {
        let analyzer = CustomMacroAnalyzer::new();

        let body = r#"
            if $condition {
                unsafe { transmute($value) }
            } else {
                $default
            }
        "#;

        let purity = analyzer.analyze(body);

        if let MacroPurity::Unknown { confidence } = purity {
            assert!(confidence < 0.8); // Low confidence due to unsafe
        } else {
            panic!("Expected Unknown purity");
        }
    }

    #[test]
    fn test_nested_macro_calls() {
        let analyzer = CustomMacroAnalyzer::new();

        let body = r#"
            let msg = format!($($arg)*);
            println!("{}", msg);
        "#;

        let purity = analyzer.analyze(body);
        assert_eq!(purity, MacroPurity::Impure);
    }

    #[test]
    fn test_pure_format_macro() {
        let analyzer = CustomMacroAnalyzer::new();

        let body = r#"
            format!("Value: {}", $x)
        "#;

        let purity = analyzer.analyze(body);
        assert_eq!(purity, MacroPurity::Pure);
    }

    #[test]
    fn test_panic_macro() {
        let analyzer = CustomMacroAnalyzer::new();

        let body = r#"
            panic!("Error: {}", $msg)
        "#;

        let purity = analyzer.analyze(body);
        assert_eq!(purity, MacroPurity::Impure);
    }

    #[test]
    fn test_dbg_macro() {
        let analyzer = CustomMacroAnalyzer::new();

        let body = r#"
            dbg!($value)
        "#;

        let purity = analyzer.analyze(body);
        assert_eq!(purity, MacroPurity::Impure);
    }

    #[test]
    fn test_mixed_pure_patterns() {
        let analyzer = CustomMacroAnalyzer::new();

        let body = r#"
            {
                let formatted = format!("{}", $x);
                let joined = concat!("prefix-", stringify!($y));
                vec![formatted, joined]
            }
        "#;

        let purity = analyzer.analyze(body);
        assert_eq!(purity, MacroPurity::Pure);
    }
}
