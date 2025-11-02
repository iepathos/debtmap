//! Type Pattern Library
//!
//! Defines common type patterns for responsibility classification:
//! - Parser patterns (String → Result<T>)
//! - Formatter patterns (T → String)
//! - Validator patterns (T → Result<(), Error>)
//! - I/O patterns (error types, trait bounds)
//! - Builder patterns (Self → Self)
//! - Query patterns (&T → Option<U>)

use crate::analysis::multi_signal_aggregation::ResponsibilityCategory;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct TypePattern {
    pub name: String,
    pub input_pattern: TypeMatcher,
    pub output_pattern: TypeMatcher,
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub priority: u8, // Higher priority patterns checked first (0-255)
}

#[derive(Debug, Clone)]
pub enum TypeMatcher {
    /// Exact type match
    Exact(String),
    /// Regex pattern match (use sparingly, prefer AST-based matching)
    Regex(&'static Lazy<Regex>),
    /// Generic with constraint
    Generic { name: String, bounds: Vec<String> },
    /// Result with specific error type
    Result {
        ok_type: Box<TypeMatcher>,
        error_pattern: Box<TypeMatcher>,
    },
    /// Option type
    Option(Box<TypeMatcher>),
    /// Any type (wildcard)
    Any,
    /// Collection type (Vec, HashMap, etc.)
    Collection { element_type: Box<TypeMatcher> },
    /// Trait object (dyn Trait)
    TraitObject {
        trait_name: String,
        bounds: Vec<String>,
    },
    /// Impl Trait pattern
    ImplTrait { trait_name: String },
    /// Associated type
    AssociatedType { base: String, item: String },
    /// Function pointer
    FnPointer {
        inputs: Vec<TypeMatcher>,
        output: Box<TypeMatcher>,
    },
}

pub struct TypePatternLibrary {
    patterns: Vec<TypePattern>,
}

// Pre-compiled regex patterns using once_cell
static STR_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(&\s*)?str$|^String$").unwrap());
static VALIDATION_ERROR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r".*[Vv]alidation.*[Ee]rror").unwrap());
static IO_ERROR: Lazy<Regex> = Lazy::new(|| Regex::new(r"(std::)?io::Error").unwrap());
static PARSE_ERROR: Lazy<Regex> = Lazy::new(|| Regex::new(r".*[Pp]arse.*[Ee]rror").unwrap());
static FORMAT_ERROR: Lazy<Regex> = Lazy::new(|| Regex::new(r".*[Ff]ormat.*[Ee]rror").unwrap());
static REF_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^&").unwrap());

impl TypePatternLibrary {
    pub fn default_patterns() -> Self {
        let mut patterns = Vec::new();

        // Validator: T → Result<(), ValidationError> (VERY HIGH PRIORITY)
        patterns.push(TypePattern {
            name: "Validator".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Result {
                ok_type: Box::new(TypeMatcher::Exact("()".into())),
                error_pattern: Box::new(TypeMatcher::Regex(&VALIDATION_ERROR)),
            },
            category: ResponsibilityCategory::Validation,
            confidence: 0.90,
            priority: 220,
        });

        // I/O Operation: Returns io::Result or io::Error (HIGH PRIORITY)
        patterns.push(TypePattern {
            name: "I/O Operation".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Result {
                ok_type: Box::new(TypeMatcher::Any),
                error_pattern: Box::new(TypeMatcher::Regex(&IO_ERROR)),
            },
            category: ResponsibilityCategory::FileIO,
            confidence: 0.85,
            priority: 210,
        });

        // Parser: String/&str → Result<T, ParseError> (HIGH PRIORITY)
        patterns.push(TypePattern {
            name: "String Parser".into(),
            input_pattern: TypeMatcher::Regex(&STR_PATTERN),
            output_pattern: TypeMatcher::Result {
                ok_type: Box::new(TypeMatcher::Any),
                error_pattern: Box::new(TypeMatcher::Regex(&PARSE_ERROR)),
            },
            category: ResponsibilityCategory::Parsing,
            confidence: 0.90,
            priority: 210,
        });

        // Parser: String/&str → Result<T, E> (MEDIUM-HIGH PRIORITY)
        patterns.push(TypePattern {
            name: "String Parser (Generic)".into(),
            input_pattern: TypeMatcher::Regex(&STR_PATTERN),
            output_pattern: TypeMatcher::Result {
                ok_type: Box::new(TypeMatcher::Any),
                error_pattern: Box::new(TypeMatcher::Any),
            },
            category: ResponsibilityCategory::Parsing,
            confidence: 0.85,
            priority: 200,
        });

        // Builder: Self → Self (HIGH PRIORITY)
        patterns.push(TypePattern {
            name: "Builder Method".into(),
            input_pattern: TypeMatcher::Exact("Self".into()),
            output_pattern: TypeMatcher::Exact("Self".into()),
            category: ResponsibilityCategory::Transformation,
            confidence: 0.80,
            priority: 180,
        });

        // Formatter: T → String (MEDIUM PRIORITY)
        patterns.push(TypePattern {
            name: "String Formatter".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Exact("String".into()),
            category: ResponsibilityCategory::Formatting,
            confidence: 0.75,
            priority: 150,
        });

        // Formatter: T → Result<String, FormatError>
        patterns.push(TypePattern {
            name: "Fallible Formatter".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Result {
                ok_type: Box::new(TypeMatcher::Exact("String".into())),
                error_pattern: Box::new(TypeMatcher::Regex(&FORMAT_ERROR)),
            },
            category: ResponsibilityCategory::Formatting,
            confidence: 0.85,
            priority: 190,
        });

        // Query: &T → Option<U> (MEDIUM PRIORITY)
        patterns.push(TypePattern {
            name: "Query/Lookup".into(),
            input_pattern: TypeMatcher::Regex(&REF_PATTERN),
            output_pattern: TypeMatcher::Option(Box::new(TypeMatcher::Any)),
            category: ResponsibilityCategory::Transformation,
            confidence: 0.70,
            priority: 140,
        });

        // Collection Query: &T → Vec<U> (MEDIUM PRIORITY)
        patterns.push(TypePattern {
            name: "Collection Query".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Collection {
                element_type: Box::new(TypeMatcher::Any),
            },
            category: ResponsibilityCategory::Transformation,
            confidence: 0.65,
            priority: 120,
        });

        // Iterator: impl Iterator<Item = T>
        patterns.push(TypePattern {
            name: "Iterator".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::ImplTrait {
                trait_name: "Iterator".into(),
            },
            category: ResponsibilityCategory::Transformation,
            confidence: 0.70,
            priority: 130,
        });

        // Transformer: T → U (different types) - LOWEST PRIORITY
        patterns.push(TypePattern {
            name: "Data Transformation".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Any,
            category: ResponsibilityCategory::Transformation,
            confidence: 0.50, // Low confidence, very generic
            priority: 50,     // Very low priority - catch-all pattern
        });

        // Sort patterns by priority (highest first)
        patterns.sort_by(|a, b| b.priority.cmp(&a.priority));

        TypePatternLibrary { patterns }
    }

    pub fn patterns(&self) -> &[TypePattern] {
        &self.patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_sorted_by_priority() {
        let library = TypePatternLibrary::default_patterns();
        let patterns = library.patterns();

        // Verify patterns are sorted by priority (highest first)
        for i in 1..patterns.len() {
            assert!(
                patterns[i - 1].priority >= patterns[i].priority,
                "Pattern at {} has lower priority than pattern at {}",
                i - 1,
                i
            );
        }
    }

    #[test]
    fn validator_pattern_has_highest_priority() {
        let library = TypePatternLibrary::default_patterns();
        let patterns = library.patterns();

        let validator = patterns
            .iter()
            .find(|p| p.name == "Validator")
            .expect("Validator pattern should exist");

        assert_eq!(validator.priority, 220);
        assert!(validator.confidence >= 0.90);
    }

    #[test]
    fn transformer_pattern_has_lowest_priority() {
        let library = TypePatternLibrary::default_patterns();
        let patterns = library.patterns();

        let transformer = patterns
            .iter()
            .find(|p| p.name == "Data Transformation")
            .expect("Data Transformation pattern should exist");

        assert_eq!(transformer.priority, 50);
        assert_eq!(transformer.confidence, 0.50);
    }
}
