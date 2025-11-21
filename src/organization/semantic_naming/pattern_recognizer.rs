//! Behavioral pattern recognition for module naming.
//!
//! Identifies common software patterns (formatting, validation, parsing, etc.)
//! by analyzing method names and their behavioral characteristics.

use super::{NameCandidate, NamingStrategy};

/// Recognizes behavioral patterns in method collections
pub struct PatternRecognizer {
    patterns: Vec<BehaviorPattern>,
}

/// A behavioral pattern with associated verbs and confidence threshold
#[derive(Debug, Clone)]
struct BehaviorPattern {
    name: String,
    verbs: Vec<String>,
    confidence_threshold: f64,
}

impl PatternRecognizer {
    /// Create a new pattern recognizer with default patterns
    pub fn new() -> Self {
        Self {
            patterns: vec![
                BehaviorPattern {
                    name: "formatting".to_string(),
                    verbs: vec![
                        "format", "display", "render", "print", "show", "write", "output",
                    ]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "validation".to_string(),
                    verbs: vec!["validate", "verify", "check", "ensure", "assert", "confirm"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "parsing".to_string(),
                    verbs: vec!["parse", "extract", "read", "decode", "interpret", "scan"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "computation".to_string(),
                    verbs: vec![
                        "calculate",
                        "compute",
                        "evaluate",
                        "measure",
                        "analyze",
                        "count",
                    ]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "transformation".to_string(),
                    verbs: vec![
                        "convert",
                        "transform",
                        "map",
                        "translate",
                        "adapt",
                        "change",
                    ]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "serialization".to_string(),
                    verbs: vec![
                        "serialize",
                        "deserialize",
                        "encode",
                        "decode",
                        "marshal",
                        "unmarshal",
                    ]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                    confidence_threshold: 0.7, // Higher threshold for more specific pattern
                },
                BehaviorPattern {
                    name: "persistence".to_string(),
                    verbs: vec!["save", "load", "store", "fetch", "retrieve", "persist"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "events".to_string(),
                    verbs: vec!["handle", "process", "dispatch", "trigger", "emit", "listen"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "lifecycle".to_string(),
                    verbs: vec![
                        "initialize",
                        "init",
                        "setup",
                        "teardown",
                        "cleanup",
                        "destroy",
                    ]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                    confidence_threshold: 0.65,
                },
            ],
        }
    }

    /// Recognize behavioral pattern in a collection of methods
    ///
    /// # Arguments
    ///
    /// * `methods` - List of method names to analyze
    ///
    /// # Returns
    ///
    /// Name candidate if a clear pattern is recognized, None otherwise
    pub fn recognize_pattern(&self, methods: &[String]) -> Option<NameCandidate> {
        if methods.is_empty() {
            return None;
        }

        let mut best_match: Option<(NameCandidate, f64)> = None;

        for pattern in &self.patterns {
            let match_score = self.calculate_pattern_match(methods, pattern);

            if match_score >= pattern.confidence_threshold {
                let candidate = NameCandidate {
                    module_name: pattern.name.clone(),
                    confidence: match_score,
                    specificity_score: 0.75, // Patterns are moderately specific
                    reasoning: format!(
                        "Recognized {} pattern ({:.0}% of methods match pattern verbs)",
                        pattern.name,
                        match_score * 100.0
                    ),
                    strategy: NamingStrategy::BehavioralPattern,
                };

                // Keep the best match
                match &best_match {
                    None => best_match = Some((candidate, match_score)),
                    Some((_, current_score)) => {
                        if match_score > *current_score {
                            best_match = Some((candidate, match_score));
                        }
                    }
                }
            }
        }

        best_match.map(|(candidate, _)| candidate)
    }

    /// Calculate how well a method collection matches a behavioral pattern
    ///
    /// Returns a score from 0.0 to 1.0 indicating the percentage of methods
    /// that match the pattern's verbs.
    fn calculate_pattern_match(&self, methods: &[String], pattern: &BehaviorPattern) -> f64 {
        if methods.is_empty() {
            return 0.0;
        }

        let matching_methods = methods
            .iter()
            .filter(|method| {
                let method_lower = method.to_lowercase();
                pattern
                    .verbs
                    .iter()
                    .any(|verb| self.method_contains_verb(&method_lower, verb))
            })
            .count();

        matching_methods as f64 / methods.len() as f64
    }

    /// Check if a method name contains a verb
    ///
    /// Uses word boundary detection to avoid false matches
    fn method_contains_verb(&self, method: &str, verb: &str) -> bool {
        // Check for verb at start (e.g., "format_item")
        if method.starts_with(verb) {
            if method.len() == verb.len() {
                return true; // Exact match
            }
            // Check for word boundary after verb
            let next_char = method.chars().nth(verb.len());
            if let Some(ch) = next_char {
                if ch == '_' || ch.is_uppercase() {
                    return true;
                }
            }
        }

        // Check for verb after underscore (e.g., "get_format")
        if method.contains(&format!("_{}", verb)) {
            return true;
        }

        // Check for verb in camelCase (e.g., "getFormat")
        if method.len() > verb.len() {
            for i in 0..method.len() - verb.len() {
                if method[i..].starts_with(verb) {
                    // Check if preceded by lowercase and followed by uppercase (camelCase boundary)
                    if i > 0 {
                        let prev_char = method.chars().nth(i - 1);
                        let next_char = method.chars().nth(i + verb.len());
                        if let (Some(prev), Some(next)) = (prev_char, next_char) {
                            if prev.is_lowercase() && next.is_uppercase() {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }
}

impl Default for PatternRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recognize_formatting_pattern() {
        let recognizer = PatternRecognizer::new();
        let methods = vec![
            "format_item".to_string(),
            "format_details".to_string(),
            "format_summary".to_string(),
            "display_result".to_string(),
        ];

        let pattern = recognizer.recognize_pattern(&methods);

        assert!(pattern.is_some());
        let candidate = pattern.unwrap();
        assert_eq!(candidate.module_name, "formatting");
        assert!(candidate.confidence >= 0.7);
    }

    #[test]
    fn test_recognize_validation_pattern() {
        let recognizer = PatternRecognizer::new();
        let methods = vec![
            "validate_input".to_string(),
            "validate_output".to_string(),
            "check_constraints".to_string(),
            "verify_data".to_string(),
        ];

        let pattern = recognizer.recognize_pattern(&methods);

        assert!(pattern.is_some());
        let candidate = pattern.unwrap();
        assert_eq!(candidate.module_name, "validation");
    }

    #[test]
    fn test_recognize_computation_pattern() {
        let recognizer = PatternRecognizer::new();
        let methods = vec![
            "calculate_total".to_string(),
            "compute_average".to_string(),
            "evaluate_score".to_string(),
        ];

        let pattern = recognizer.recognize_pattern(&methods);

        assert!(pattern.is_some());
        let candidate = pattern.unwrap();
        assert_eq!(candidate.module_name, "computation");
    }

    #[test]
    fn test_no_pattern_for_mixed_methods() {
        let recognizer = PatternRecognizer::new();
        let methods = vec![
            "do_something".to_string(),
            "handle_stuff".to_string(),
            "process_things".to_string(),
        ];

        let pattern = recognizer.recognize_pattern(&methods);

        // Should either find a weak match or none
        if let Some(candidate) = pattern {
            // If found, should have moderate confidence
            assert!(candidate.confidence < 0.8);
        }
    }

    #[test]
    fn test_method_contains_verb_snake_case() {
        let recognizer = PatternRecognizer::new();

        assert!(recognizer.method_contains_verb("format_item", "format"));
        assert!(recognizer.method_contains_verb("get_format", "format"));
        assert!(!recognizer.method_contains_verb("formatting", "format")); // Partial match, not exact
    }

    #[test]
    fn test_method_contains_verb_camel_case() {
        let recognizer = PatternRecognizer::new();

        assert!(recognizer.method_contains_verb("formatitem", "format"));
        assert!(recognizer.method_contains_verb("getformat", "format"));
    }

    #[test]
    fn test_confidence_threshold() {
        let recognizer = PatternRecognizer::new();
        // Only 1 out of 3 methods match - should not meet threshold
        let methods = vec![
            "format_item".to_string(),
            "do_something".to_string(),
            "process_data".to_string(),
        ];

        let pattern = recognizer.recognize_pattern(&methods);

        // Should not match with only 33% coverage
        assert!(pattern.is_none() || pattern.unwrap().confidence < 0.6);
    }

    #[test]
    fn test_empty_methods() {
        let recognizer = PatternRecognizer::new();
        let methods: Vec<String> = vec![];

        let pattern = recognizer.recognize_pattern(&methods);

        assert!(pattern.is_none());
    }

    #[test]
    fn test_best_match_selection() {
        let recognizer = PatternRecognizer::new();
        // Methods that could match multiple patterns
        let methods = vec![
            "calculate_value".to_string(),
            "compute_result".to_string(),
            "transform_data".to_string(), // Also matches transformation
        ];

        let pattern = recognizer.recognize_pattern(&methods);

        assert!(pattern.is_some());
        // Should pick the best match (highest confidence)
        let candidate = pattern.unwrap();
        assert!(candidate.confidence > 0.6);
    }
}
