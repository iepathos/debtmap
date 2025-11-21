//! Domain term extraction from method names and descriptions.
//!
//! Analyzes method names to identify common domain terms and generate
//! meaningful module names based on the dominant terminology.

use super::{NameCandidate, NamingStrategy};
use std::collections::HashMap;

/// Extracts domain-specific terminology from method names
pub struct DomainTermExtractor {
    common_verbs: Vec<&'static str>,
    stop_words: Vec<&'static str>,
}

impl DomainTermExtractor {
    /// Create a new domain term extractor with default configuration
    pub fn new() -> Self {
        Self {
            common_verbs: vec![
                "get", "set", "is", "has", "can", "should", "with", "to", "from", "into", "as",
                "new", "default", "clone", "eq", "ne", "cmp", "hash", "manage",
            ],
            stop_words: vec![
                "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
                "by", "from", "as", "is", "was", "are", "were", "be", "been", "being", "have",
                "has", "had", "do", "does", "did", "will", "would", "should", "could", "may",
                "might", "must", "can", "this", "that", "these", "those", "data",
            ],
        }
    }

    /// Generate a domain-based name from method names
    ///
    /// Analyzes method names to extract domain terms and generates
    /// a name candidate with confidence scoring.
    ///
    /// # Arguments
    ///
    /// * `methods` - List of method names to analyze
    ///
    /// # Returns
    ///
    /// Name candidate if domain terms found, None otherwise
    pub fn generate_domain_name(&self, methods: &[String]) -> Option<NameCandidate> {
        if methods.is_empty() {
            return None;
        }

        let terms = self.extract_domain_terms(methods);
        if terms.is_empty() {
            return None;
        }

        // Try to find verb-noun pairs first (more specific)
        if let Some(name) = self.find_verb_noun_pair(&terms) {
            return Some(name);
        }

        // Use dominant single term
        let (primary_term, primary_freq) = &terms[0];

        // Require at least 30% frequency for single term
        if *primary_freq < 0.3 {
            return None;
        }

        Some(NameCandidate {
            module_name: primary_term.clone(),
            confidence: (primary_freq * 0.9).min(0.85), // Cap at 0.85 for single term
            specificity_score: self.calculate_term_specificity(primary_term),
            reasoning: format!(
                "Dominant term '{}' appears in {:.0}% of methods",
                primary_term,
                primary_freq * 100.0
            ),
            strategy: NamingStrategy::DomainTerms,
        })
    }

    /// Extract domain terms from a description string
    ///
    /// Used for extracting terms from responsibility descriptions.
    pub fn extract_from_description(&self, description: &str) -> Option<NameCandidate> {
        let tokens = self.tokenize_text(description);
        let significant_tokens: Vec<String> = tokens
            .into_iter()
            .filter(|t| !self.is_stop_word(t))
            .filter(|t| !self.is_common_verb(t))
            .filter(|t| t.len() > 3)
            .collect();

        if significant_tokens.is_empty() {
            return None;
        }

        // Count frequencies
        let mut freq_map: HashMap<String, usize> = HashMap::new();
        for token in &significant_tokens {
            *freq_map.entry(token.clone()).or_insert(0) += 1;
        }

        // Find most frequent significant term, prioritizing specificity and non-gerunds
        let mut terms: Vec<_> = freq_map.into_iter().collect();

        // Sort by frequency first, then by specificity, then prioritize non-gerund terms
        terms.sort_by(|a, b| {
            let freq_cmp = b.1.cmp(&a.1);
            if freq_cmp != std::cmp::Ordering::Equal {
                return freq_cmp;
            }
            // If frequencies are equal, prefer terms with higher specificity
            let a_specificity = self.calculate_term_specificity(&a.0);
            let b_specificity = self.calculate_term_specificity(&b.0);
            let specificity_cmp = b_specificity
                .partial_cmp(&a_specificity)
                .unwrap_or(std::cmp::Ordering::Equal);
            if specificity_cmp != std::cmp::Ordering::Equal {
                return specificity_cmp;
            }
            // If specificities are equal, prefer non-gerunds
            let a_is_gerund = a.0.ends_with("ing");
            let b_is_gerund = b.0.ends_with("ing");
            match (a_is_gerund, b_is_gerund) {
                (false, true) => std::cmp::Ordering::Less, // a is better
                (true, false) => std::cmp::Ordering::Greater, // b is better
                _ => std::cmp::Ordering::Equal,
            }
        });

        let (term, count) = &terms[0];
        let freq = *count as f64 / significant_tokens.len() as f64;

        if freq < 0.2 {
            return None;
        }

        Some(NameCandidate {
            module_name: term.clone(),
            confidence: 0.6, // Lower confidence from descriptions than from method analysis
            specificity_score: self.calculate_term_specificity(term),
            reasoning: format!("Extracted from description: '{}'", description),
            strategy: NamingStrategy::DomainTerms,
        })
    }

    /// Extract domain terms with their frequencies
    ///
    /// Returns terms sorted by frequency (descending)
    fn extract_domain_terms(&self, methods: &[String]) -> Vec<(String, f64)> {
        // Tokenize all method names
        let all_tokens: Vec<String> = methods
            .iter()
            .flat_map(|m| self.tokenize_method_name(m))
            .collect();

        if all_tokens.is_empty() {
            return vec![];
        }

        // Count term frequencies
        let mut freq_map: HashMap<String, usize> = HashMap::new();
        for token in &all_tokens {
            *freq_map.entry(token.clone()).or_insert(0) += 1;
        }

        // Calculate relative frequencies and filter
        let total_tokens = all_tokens.len() as f64;
        let mut terms: Vec<(String, f64)> = freq_map
            .into_iter()
            .map(|(term, count)| (term, count as f64 / total_tokens))
            .filter(|(term, _)| !self.is_stop_word(term))
            .filter(|(term, _)| !self.is_common_verb(term))
            .filter(|(_term, freq)| *freq >= 0.15) // Appear in at least 15% of tokens
            .collect();

        // Sort by frequency descending
        terms.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        terms
    }

    /// Tokenize a method name into constituent terms
    ///
    /// Handles snake_case, camelCase, and PascalCase
    fn tokenize_method_name(&self, method: &str) -> Vec<String> {
        let mut tokens = Vec::new();

        // Split on underscores first
        for part in method.split('_') {
            // Then split camelCase within each part
            tokens.extend(self.split_camel_case(part));
        }

        tokens
            .into_iter()
            .map(|s| s.to_lowercase())
            .filter(|s| s.len() > 2) // Remove very short tokens
            .collect()
    }

    /// Tokenize regular text (for descriptions)
    fn tokenize_text(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    }

    /// Split camelCase string into words
    fn split_camel_case(&self, s: &str) -> Vec<String> {
        if s.is_empty() {
            return vec![];
        }

        let mut result = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = s.chars().collect();

        for i in 0..chars.len() {
            let ch = chars[i];
            let is_upper = ch.is_uppercase();
            let prev_was_lower = i > 0 && chars[i - 1].is_lowercase();
            let next_is_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();

            // Split before uppercase when:
            // 1. Previous was lowercase (normal camelCase boundary)
            // 2. Next is lowercase (handles acronyms like "APIKey" -> "API", "Key")
            if is_upper && (!current.is_empty() && (prev_was_lower || next_is_lower)) {
                // For acronyms, keep all caps together except last letter
                #[allow(clippy::if_same_then_else, clippy::len_zero)]
                if !prev_was_lower && next_is_lower && current.len() > 0 {
                    result.push(current.clone());
                    current.clear();
                } else if prev_was_lower {
                    result.push(current.clone());
                    current.clear();
                }
            }

            current.push(ch);
        }

        if !current.is_empty() {
            result.push(current);
        }

        result
    }

    /// Try to find a meaningful verb-noun pair
    fn find_verb_noun_pair(&self, terms: &[(String, f64)]) -> Option<NameCandidate> {
        if terms.len() < 2 {
            return None;
        }

        // Common action verbs that pair well with nouns
        let action_verbs = [
            "format",
            "parse",
            "validate",
            "calculate",
            "compute",
            "analyze",
            "process",
            "transform",
            "convert",
            "serialize",
            "deserialize",
            "encode",
            "decode",
            "render",
            "display",
            "print",
            "write",
            "read",
            "load",
            "save",
            "create",
            "build",
            "generate",
        ];

        // Look for verb + noun combinations
        for (verb_candidate, verb_freq) in terms {
            if action_verbs.contains(&verb_candidate.as_str()) {
                // Find a noun to pair with
                for (noun_candidate, noun_freq) in terms {
                    if noun_candidate != verb_candidate
                        && !action_verbs.contains(&noun_candidate.as_str())
                    {
                        let combined_freq = (verb_freq + noun_freq) / 2.0;
                        if combined_freq > 0.25 {
                            let module_name = format!("{}_{}", verb_candidate, noun_candidate);
                            return Some(NameCandidate {
                                module_name,
                                confidence: 0.85, // High confidence for verb-noun pairs
                                specificity_score: 0.8,
                                reasoning: format!(
                                    "Identified verb-noun pattern: '{}' + '{}' (avg frequency: {:.0}%)",
                                    verb_candidate, noun_candidate, combined_freq * 100.0
                                ),
                                strategy: NamingStrategy::DomainTerms,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    /// Calculate specificity score for a term
    fn calculate_term_specificity(&self, term: &str) -> f64 {
        // Generic terms get low scores
        let generic_terms = [
            "data", "value", "item", "object", "type", "info", "element", "thing",
        ];

        if generic_terms.contains(&term) {
            return 0.3;
        }

        // Domain-specific terms get high scores
        let domain_terms = [
            ("coverage", 0.9),
            ("metrics", 0.85),
            ("complexity", 0.9),
            ("analysis", 0.8),
            ("validation", 0.85),
            ("formatting", 0.85),
            ("parsing", 0.9),
            ("serialization", 0.9),
            ("computation", 0.85),
            ("optimization", 0.9),
        ];

        for (domain_term, score) in &domain_terms {
            if term.contains(domain_term) {
                return *score;
            }
        }

        // Default: moderate specificity
        0.6
    }

    fn is_stop_word(&self, word: &str) -> bool {
        self.stop_words.contains(&word)
    }

    fn is_common_verb(&self, word: &str) -> bool {
        self.common_verbs.contains(&word)
    }
}

impl Default for DomainTermExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_snake_case() {
        let extractor = DomainTermExtractor::new();
        let tokens = extractor.tokenize_method_name("format_coverage_status");

        assert_eq!(tokens, vec!["format", "coverage", "status"]);
    }

    #[test]
    fn test_tokenize_camel_case() {
        let extractor = DomainTermExtractor::new();
        let tokens = extractor.tokenize_method_name("calculateMetrics");

        assert_eq!(tokens, vec!["calculate", "metrics"]);
    }

    #[test]
    fn test_tokenize_mixed_case() {
        let extractor = DomainTermExtractor::new();
        let tokens = extractor.tokenize_method_name("get_APIKey");

        assert!(tokens.contains(&"api".to_string()));
        assert!(tokens.contains(&"key".to_string()));
    }

    #[test]
    fn test_extract_domain_terms() {
        let extractor = DomainTermExtractor::new();
        let methods = vec![
            "format_coverage_status".to_string(),
            "format_coverage_factor".to_string(),
            "calculate_coverage_percentage".to_string(),
        ];

        let terms = extractor.extract_domain_terms(&methods);

        // Should find "coverage" as dominant term
        assert!(!terms.is_empty());
        let coverage_term = terms.iter().find(|(term, _)| term == "coverage");
        assert!(coverage_term.is_some());

        let (_, freq) = coverage_term.unwrap();
        assert!(*freq > 0.3); // Should appear frequently
    }

    #[test]
    fn test_filters_common_verbs() {
        let extractor = DomainTermExtractor::new();
        let methods = vec![
            "get_value".to_string(),
            "set_value".to_string(),
            "is_valid".to_string(),
        ];

        let terms = extractor.extract_domain_terms(&methods);

        // Should not include common verbs like "get", "set", "is"
        assert!(!terms.iter().any(|(term, _)| term == "get"));
        assert!(!terms.iter().any(|(term, _)| term == "set"));
    }

    #[test]
    fn test_verb_noun_extraction() {
        let extractor = DomainTermExtractor::new();
        let methods = vec![
            "format_coverage".to_string(),
            "format_status".to_string(),
            "parse_coverage".to_string(),
        ];

        let name = extractor.generate_domain_name(&methods);

        assert!(name.is_some());
        let candidate = name.unwrap();
        // Should find either "format_coverage" or similar verb-noun pair
        assert!(
            candidate.module_name.contains("format") || candidate.module_name.contains("coverage")
        );
    }

    #[test]
    fn test_extract_from_description() {
        let extractor = DomainTermExtractor::new();
        let description = "Manage coverage data and its transformations";

        let name = extractor.extract_from_description(description);

        assert!(name.is_some());
        let candidate = name.unwrap();
        assert_eq!(candidate.module_name, "coverage");
    }

    #[test]
    fn test_specificity_scoring() {
        let extractor = DomainTermExtractor::new();

        // Domain-specific terms should score high
        assert!(extractor.calculate_term_specificity("coverage") > 0.8);
        assert!(extractor.calculate_term_specificity("complexity") > 0.8);

        // Generic terms should score low
        assert!(extractor.calculate_term_specificity("data") < 0.5);
        assert!(extractor.calculate_term_specificity("value") < 0.5);
    }
}
