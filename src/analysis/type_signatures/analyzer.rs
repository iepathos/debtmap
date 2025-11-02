//! Type Signature Analyzer
//!
//! Analyzes function type signatures to infer responsibility:
//! - Pattern matching against type signature library
//! - Error type classification
//! - Generic bound analysis
//! - Caching for performance

use super::normalizer::{CanonicalType, TypeNormalizer};
use super::patterns::{TypeMatcher, TypePattern, TypePatternLibrary};
use crate::analysis::multi_signal_aggregation::ResponsibilityCategory;
use dashmap::DashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct TypeSignature {
    pub parameters: Vec<Parameter>,
    pub return_type: Option<CanonicalType>,
    pub generic_bounds: Vec<GenericBound>,
    pub error_type: Option<CanonicalType>,
}

impl Hash for TypeSignature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for param in &self.parameters {
            param.type_annotation.hash(state);
        }
        self.return_type.hash(state);
        for bound in &self.generic_bounds {
            bound.type_param.hash(state);
            for trait_bound in &bound.trait_bounds {
                trait_bound.hash(state);
            }
        }
        self.error_type.hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: CanonicalType,
    pub is_reference: bool,
    pub is_mutable: bool,
}

#[derive(Debug, Clone)]
pub struct GenericBound {
    pub type_param: String,
    pub trait_bounds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeBasedClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
    pub pattern_name: String,
}

pub struct TypeSignatureAnalyzer {
    pattern_library: TypePatternLibrary,
    normalizer: TypeNormalizer,
    cache: DashMap<u64, Option<TypeBasedClassification>>,
}

impl TypeSignatureAnalyzer {
    pub fn new() -> Self {
        Self {
            pattern_library: TypePatternLibrary::default_patterns(),
            normalizer: TypeNormalizer::new(),
            cache: DashMap::new(),
        }
    }

    pub fn analyze_signature(&self, signature: &TypeSignature) -> Option<TypeBasedClassification> {
        // Compute cache key from signature
        let mut hasher = DefaultHasher::new();
        signature.hash(&mut hasher);
        let cache_key = hasher.finish();

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        // Pattern library is pre-sorted by priority, so first match wins
        let result = self.analyze_signature_impl(signature);

        // Cache the result
        self.cache.insert(cache_key, result.clone());

        result
    }

    fn analyze_signature_impl(&self, signature: &TypeSignature) -> Option<TypeBasedClassification> {
        // Try to match against known patterns (sorted by priority)
        for pattern in self.pattern_library.patterns() {
            if self.matches_pattern(signature, pattern) {
                return Some(TypeBasedClassification {
                    category: pattern.category,
                    confidence: pattern.confidence,
                    evidence: format!(
                        "Matches '{}' pattern: {} → {}",
                        pattern.name,
                        self.format_inputs(signature),
                        self.format_type(&signature.return_type)
                    ),
                    pattern_name: pattern.name.clone(),
                });
            }
        }

        // Check error type for classification (if no pattern matched)
        if let Some(ref error_type) = signature.error_type {
            if let Some(category) = self.classify_by_error_type(error_type) {
                return Some(TypeBasedClassification {
                    category,
                    confidence: 0.80,
                    evidence: format!("Error type suggests category: {}", error_type.base),
                    pattern_name: "Error Type Classification".into(),
                });
            }
        }

        // Check generic bounds (lowest priority)
        if let Some(category) = self.classify_by_generic_bounds(&signature.generic_bounds) {
            return Some(TypeBasedClassification {
                category,
                confidence: 0.70,
                evidence: format!(
                    "Generic bounds: {}",
                    signature
                        .generic_bounds
                        .iter()
                        .map(|b| format!("{}: {}", b.type_param, b.trait_bounds.join(" + ")))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                pattern_name: "Generic Bound Classification".into(),
            });
        }

        None
    }

    fn matches_pattern(&self, signature: &TypeSignature, pattern: &TypePattern) -> bool {
        // Match input pattern
        let input_match = match &pattern.input_pattern {
            TypeMatcher::Any => true, // Any pattern always matches
            _ => {
                if signature.parameters.is_empty() {
                    false // If pattern requires specific input but we have no params
                } else {
                    signature.parameters.iter().any(|param| {
                        self.canonical_matches(&param.type_annotation, &pattern.input_pattern)
                    })
                }
            }
        };

        // Match output pattern
        let output_match = signature
            .return_type
            .as_ref()
            .map(|rt| self.canonical_matches(rt, &pattern.output_pattern))
            .unwrap_or(false);

        input_match && output_match
    }

    // AST-based type matching (no string parsing!)
    #[allow(clippy::only_used_in_recursion)]
    fn canonical_matches(&self, canonical: &CanonicalType, matcher: &TypeMatcher) -> bool {
        match matcher {
            TypeMatcher::Exact(expected) => &canonical.base == expected,
            TypeMatcher::Regex(regex) => regex.is_match(&canonical.base),
            TypeMatcher::Any => true,

            // Proper AST-based Result matching
            TypeMatcher::Result {
                ok_type,
                error_pattern,
            } => {
                canonical.base == "Result"
                    && canonical.generics.len() == 2
                    && self.canonical_matches(&canonical.generics[0], ok_type)
                    && self.canonical_matches(&canonical.generics[1], error_pattern)
            }

            // Proper AST-based Option matching
            TypeMatcher::Option(inner) => {
                canonical.base == "Option"
                    && canonical.generics.len() == 1
                    && self.canonical_matches(&canonical.generics[0], inner)
            }

            // Collection matching
            TypeMatcher::Collection { element_type } => {
                matches!(
                    canonical.base.as_str(),
                    "Vec" | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet"
                ) && !canonical.generics.is_empty()
                    && self.canonical_matches(&canonical.generics[0], element_type)
            }

            // Generic with bounds checking
            TypeMatcher::Generic { name, .. } => &canonical.base == name,

            // Trait object matching
            TypeMatcher::TraitObject { trait_name, .. } => {
                canonical.base.starts_with("dyn ") && canonical.base.contains(trait_name)
            }

            // Impl Trait matching
            TypeMatcher::ImplTrait { trait_name } => {
                canonical.base.starts_with("impl ") && canonical.base.contains(trait_name)
            }

            // Associated type matching
            TypeMatcher::AssociatedType { base, item } => {
                canonical.base == *base || canonical.base.contains(&format!("{}::{}", base, item))
            }

            TypeMatcher::FnPointer { .. } => canonical.base.starts_with("fn("),
        }
    }

    fn classify_by_error_type(&self, error_type: &CanonicalType) -> Option<ResponsibilityCategory> {
        let lower = error_type.base.to_lowercase();

        if lower.contains("io") || lower.contains("file") || lower.contains("network") {
            return Some(ResponsibilityCategory::FileIO);
        }

        if lower.contains("validation") || lower.contains("validate") {
            return Some(ResponsibilityCategory::Validation);
        }

        if lower.contains("parse") {
            return Some(ResponsibilityCategory::Parsing);
        }

        if lower.contains("format") {
            return Some(ResponsibilityCategory::Formatting);
        }

        if lower.contains("database") || lower.contains("sql") {
            return Some(ResponsibilityCategory::DatabaseIO);
        }

        None
    }

    fn classify_by_generic_bounds(
        &self,
        bounds: &[GenericBound],
    ) -> Option<ResponsibilityCategory> {
        for bound in bounds {
            // Check for I/O trait bounds
            if bound
                .trait_bounds
                .iter()
                .any(|t| t.contains("Read") || t.contains("Write"))
            {
                return Some(ResponsibilityCategory::FileIO);
            }

            // Check for iterator trait bounds
            if bound
                .trait_bounds
                .iter()
                .any(|t| t.contains("Iterator") || t.contains("IntoIterator"))
            {
                return Some(ResponsibilityCategory::Transformation);
            }

            // Check for serialization
            if bound
                .trait_bounds
                .iter()
                .any(|t| t.contains("Serialize") || t.contains("Deserialize"))
            {
                return Some(ResponsibilityCategory::Transformation);
            }
        }

        None
    }

    fn format_inputs(&self, signature: &TypeSignature) -> String {
        if signature.parameters.is_empty() {
            "()".into()
        } else {
            signature
                .parameters
                .iter()
                .map(|p| self.normalizer.canonical_to_string(&p.type_annotation))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    fn format_type(&self, ty: &Option<CanonicalType>) -> String {
        ty.as_ref()
            .map(|t| self.normalizer.canonical_to_string(t))
            .unwrap_or_else(|| "()".into())
    }

    pub fn normalizer(&self) -> &TypeNormalizer {
        &self.normalizer
    }
}

impl Default for TypeSignatureAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_signature(
        param_types: Vec<(&str, bool, bool)>,
        return_type: Option<&str>,
        error_type: Option<&str>,
    ) -> TypeSignature {
        let normalizer = TypeNormalizer::new();

        let parameters = param_types
            .into_iter()
            .map(|(ty_str, is_ref, is_mut)| {
                let ty: syn::Type = syn::parse_str(ty_str).unwrap();
                let mut canonical = normalizer.normalize(&ty);
                canonical.is_reference = is_ref;
                canonical.is_mutable = is_mut;

                Parameter {
                    name: "param".into(),
                    type_annotation: canonical,
                    is_reference: is_ref,
                    is_mutable: is_mut,
                }
            })
            .collect();

        let return_type = return_type.map(|rt| {
            let ty: syn::Type = syn::parse_str(rt).unwrap();
            normalizer.normalize(&ty)
        });

        let error_type = error_type.map(|et| {
            let ty: syn::Type = syn::parse_str(et).unwrap();
            normalizer.normalize(&ty)
        });

        TypeSignature {
            parameters,
            return_type,
            generic_bounds: vec![],
            error_type,
        }
    }

    #[test]
    fn parser_pattern() {
        let signature = create_test_signature(
            vec![("&str", true, false)],
            Some("Result<Config, ParseError>"),
            Some("ParseError"),
        );

        let analyzer = TypeSignatureAnalyzer::new();
        let classification = analyzer.analyze_signature(&signature).unwrap();

        assert_eq!(classification.category, ResponsibilityCategory::Parsing);
        assert!(classification.confidence > 0.80);
    }

    #[test]
    fn formatter_pattern() {
        let signature = create_test_signature(vec![("&User", true, false)], Some("String"), None);

        let analyzer = TypeSignatureAnalyzer::new();
        let classification = analyzer.analyze_signature(&signature).unwrap();

        assert_eq!(classification.category, ResponsibilityCategory::Formatting);
    }

    #[test]
    fn validator_pattern() {
        let signature = create_test_signature(
            vec![("&str", true, false)],
            Some("Result<(), ValidationError>"),
            Some("ValidationError"),
        );

        let analyzer = TypeSignatureAnalyzer::new();
        let classification = analyzer.analyze_signature(&signature).unwrap();

        assert_eq!(classification.category, ResponsibilityCategory::Validation);
        assert!(classification.confidence > 0.85);
    }

    #[test]
    fn io_error_classification() {
        let signature = create_test_signature(
            vec![],
            Some("Result<String, std::io::Error>"),
            Some("std::io::Error"),
        );

        let analyzer = TypeSignatureAnalyzer::new();
        let classification = analyzer.analyze_signature(&signature).unwrap();

        assert_eq!(classification.category, ResponsibilityCategory::FileIO);
    }

    #[test]
    fn caching_works() {
        let signature = create_test_signature(vec![("&str", true, false)], Some("String"), None);

        let analyzer = TypeSignatureAnalyzer::new();

        // First call - cache miss
        let result1 = analyzer.analyze_signature(&signature);

        // Second call - should hit cache
        let cache_size_before = analyzer.cache.len();
        let result2 = analyzer.analyze_signature(&signature);
        let cache_size_after = analyzer.cache.len();

        assert_eq!(result1, result2);
        assert_eq!(cache_size_before, cache_size_after);
    }
}
