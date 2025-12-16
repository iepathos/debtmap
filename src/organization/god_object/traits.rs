//! # Trait-Mandated Method Detection (Spec 217)
//!
//! Pure functions for detecting trait-mandated methods in god object analysis.
//!
//! ## Problem
//!
//! God object detection doesn't distinguish between:
//! - **Trait-mandated methods** - Required by trait implementation, cannot be extracted
//! - **Self-chosen methods** - Author's design choice, potentially extractable
//!
//! This leads to misleading recommendations like "extract 5 sub-orchestrators"
//! for a struct where 18 of 32 methods are `syn::Visit` trait requirements.
//!
//! ## Solution
//!
//! This module provides:
//! - `KnownTraitRegistry` - Registry of known traits with their method patterns
//! - `TraitImplInfo` - Information about a trait implementation
//! - `MethodOrigin` - Classification of whether a method is trait-mandated or self-chosen
//! - `TraitMethodSummary` - Summary of trait-mandated vs extractable methods
//!
//! ## Weighting
//!
//! | Category | Weight | Rationale |
//! |----------|--------|-----------|
//! | Visitor (syn::Visit) | 0.1 | Structural, many required by design |
//! | Serialization | 0.1 | Usually derived, low weight |
//! | Iterator | 0.3 | Few required methods |
//! | Async (Future) | 0.3 | Typically just poll() |
//! | Standard (Clone, Default) | 0.2 | Well-known boilerplate |
//! | Comparison | 0.2 | Usually derived or trivial |
//! | Error | 0.3 | Error handling trait methods |
//! | Custom (unknown traits) | 0.4 | Moderate weight for unknown |
//! | Self-chosen | 1.0 | Full weight, author's design choice |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Core Types
// ============================================================================

/// Category of a known trait for recommendation adjustment.
///
/// Different trait categories have different implications for god object analysis:
/// - Visitors have many required methods by design (AST traversal)
/// - Serialization traits are often derived automatically
/// - Standard traits are well-known boilerplate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraitCategory {
    /// AST/tree visitor traits (syn::Visit, etc.)
    Visitor,
    /// Serialization traits (serde::Serialize, etc.)
    Serialization,
    /// Iterator/stream traits
    Iterator,
    /// Async runtime traits (Future, etc.)
    Async,
    /// Comparison/ordering traits (Eq, Ord, etc.)
    Comparison,
    /// Error handling traits (Error, etc.)
    Error,
    /// Standard library traits (Clone, Default, Drop, etc.)
    Standard,
    /// Unknown or custom traits
    Custom,
}

impl TraitCategory {
    /// Returns the default weight for methods in this trait category.
    ///
    /// Lower weights mean the method contributes less to god object score.
    #[must_use]
    pub fn default_weight(&self) -> f64 {
        match self {
            Self::Visitor => 0.1,       // Many methods required by design
            Self::Serialization => 0.1, // Usually derived, not author code
            Self::Iterator => 0.3,      // Few required methods
            Self::Async => 0.3,         // Typically just poll()
            Self::Standard => 0.2,      // Well-known boilerplate
            Self::Comparison => 0.2,    // Usually derived or trivial
            Self::Error => 0.3,         // Error trait methods
            Self::Custom => 0.4,        // Unknown traits, moderate weight
        }
    }

    /// Returns a human-readable description of this category.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Visitor => "AST/tree visitor",
            Self::Serialization => "serialization",
            Self::Iterator => "iterator/stream",
            Self::Async => "async runtime",
            Self::Standard => "standard library trait",
            Self::Comparison => "comparison/ordering",
            Self::Error => "error handling",
            Self::Custom => "custom trait",
        }
    }
}

impl Default for TraitCategory {
    fn default() -> Self {
        Self::Custom
    }
}

/// Pattern for matching trait method names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodPattern {
    /// Exact method name match (e.g., "next" for Iterator)
    Exact(String),
    /// Prefix match (e.g., "visit_" for syn::Visit)
    Prefix(String),
    /// Suffix match
    Suffix(String),
}

impl MethodPattern {
    /// Check if a method name matches this pattern.
    #[must_use]
    pub fn matches(&self, method_name: &str) -> bool {
        match self {
            Self::Exact(name) => method_name == name,
            Self::Prefix(prefix) => method_name.starts_with(prefix),
            Self::Suffix(suffix) => method_name.ends_with(suffix),
        }
    }
}

/// A known trait with its method patterns.
#[derive(Debug, Clone)]
pub struct KnownTrait {
    /// Full trait path (e.g., "syn::Visit")
    pub path: String,
    /// Alternative names/re-exports (e.g., ["syn::visit::Visit", "Visit"])
    pub aliases: Vec<String>,
    /// Method patterns that belong to this trait
    pub method_patterns: Vec<MethodPattern>,
    /// Category for recommendation adjustment
    pub category: TraitCategory,
}

impl KnownTrait {
    /// Check if a trait name matches this known trait.
    #[must_use]
    pub fn matches_trait_name(&self, trait_name: &str) -> bool {
        // Direct match on path
        if self.path == trait_name {
            return true;
        }

        // Check aliases
        if self.aliases.iter().any(|a| a == trait_name) {
            return true;
        }

        // Check if the trait name ends with the primary name (handles partial paths)
        // e.g., "Visit" matches "syn::Visit"
        let primary_name = self.path.rsplit("::").next().unwrap_or(&self.path);
        trait_name == primary_name || trait_name.ends_with(&format!("::{}", primary_name))
    }

    /// Check if a method name matches any of this trait's patterns.
    #[must_use]
    pub fn matches_method(&self, method_name: &str) -> bool {
        self.method_patterns.iter().any(|p| p.matches(method_name))
    }
}

// ============================================================================
// Known Trait Registry
// ============================================================================

/// Registry of well-known traits and their method signatures.
///
/// This registry is used to identify trait-mandated methods in god object analysis.
/// Methods that match known trait patterns get reduced weight because they are
/// structural requirements, not design choices that can be extracted.
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::traits::{KnownTraitRegistry, TraitCategory};
///
/// let registry = KnownTraitRegistry::default();
///
/// // Look up a trait by name
/// if let Some(info) = registry.get("syn::Visit") {
///     assert_eq!(info.category, TraitCategory::Visitor);
/// }
///
/// // Find category for a trait implementation
/// let category = registry.categorize_trait("Iterator");
/// assert_eq!(category, TraitCategory::Iterator);
/// ```
#[derive(Debug, Clone)]
pub struct KnownTraitRegistry {
    traits: HashMap<String, KnownTrait>,
}

impl KnownTraitRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            traits: HashMap::new(),
        }
    }

    /// Add a known trait to the registry.
    pub fn add(&mut self, known_trait: KnownTrait) {
        self.traits.insert(known_trait.path.clone(), known_trait);
    }

    /// Get a known trait by its primary path.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&KnownTrait> {
        self.traits.get(path)
    }

    /// Find a known trait by any of its names (path or aliases).
    #[must_use]
    pub fn find(&self, trait_name: &str) -> Option<&KnownTrait> {
        self.traits
            .values()
            .find(|t| t.matches_trait_name(trait_name))
    }

    /// Get the category for a trait name, defaulting to Custom if not found.
    #[must_use]
    pub fn categorize_trait(&self, trait_name: &str) -> TraitCategory {
        self.find(trait_name)
            .map(|t| t.category)
            .unwrap_or(TraitCategory::Custom)
    }

    /// Get the weight for a method based on its trait.
    ///
    /// If the trait is known, returns the category's default weight.
    /// If the trait is unknown, returns the Custom category weight (0.4).
    #[must_use]
    pub fn method_weight(&self, trait_name: &str) -> f64 {
        self.categorize_trait(trait_name).default_weight()
    }

    /// Iterate over all known traits.
    pub fn iter(&self) -> impl Iterator<Item = &KnownTrait> {
        self.traits.values()
    }

    /// Get the number of registered traits.
    #[must_use]
    pub fn len(&self) -> usize {
        self.traits.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.traits.is_empty()
    }
}

impl Default for KnownTraitRegistry {
    fn default() -> Self {
        build_default_registry()
    }
}

/// Build the default registry with well-known Rust traits.
fn build_default_registry() -> KnownTraitRegistry {
    let mut registry = KnownTraitRegistry::new();

    // Visitor patterns (syn, etc.)
    registry.add(KnownTrait {
        path: "syn::Visit".into(),
        aliases: vec![
            "syn::visit::Visit".into(),
            "Visit".into(),
            "syn::Visit<'ast>".into(),
        ],
        method_patterns: vec![MethodPattern::Prefix("visit_".into())],
        category: TraitCategory::Visitor,
    });

    registry.add(KnownTrait {
        path: "syn::VisitMut".into(),
        aliases: vec!["syn::visit_mut::VisitMut".into(), "VisitMut".into()],
        method_patterns: vec![MethodPattern::Prefix("visit_".into())],
        category: TraitCategory::Visitor,
    });

    registry.add(KnownTrait {
        path: "syn::Fold".into(),
        aliases: vec!["syn::fold::Fold".into(), "Fold".into()],
        method_patterns: vec![MethodPattern::Prefix("fold_".into())],
        category: TraitCategory::Visitor,
    });

    // Serialization traits
    registry.add(KnownTrait {
        path: "serde::Serialize".into(),
        aliases: vec!["Serialize".into(), "serde::ser::Serialize".into()],
        method_patterns: vec![MethodPattern::Exact("serialize".into())],
        category: TraitCategory::Serialization,
    });

    registry.add(KnownTrait {
        path: "serde::Deserialize".into(),
        aliases: vec!["Deserialize".into(), "serde::de::Deserialize".into()],
        method_patterns: vec![MethodPattern::Exact("deserialize".into())],
        category: TraitCategory::Serialization,
    });

    registry.add(KnownTrait {
        path: "serde::Serializer".into(),
        aliases: vec!["Serializer".into()],
        method_patterns: vec![MethodPattern::Prefix("serialize_".into())],
        category: TraitCategory::Serialization,
    });

    registry.add(KnownTrait {
        path: "serde::Deserializer".into(),
        aliases: vec!["Deserializer".into()],
        method_patterns: vec![MethodPattern::Prefix("deserialize_".into())],
        category: TraitCategory::Serialization,
    });

    // Iterator traits
    registry.add(KnownTrait {
        path: "Iterator".into(),
        aliases: vec!["std::iter::Iterator".into(), "core::iter::Iterator".into()],
        method_patterns: vec![
            MethodPattern::Exact("next".into()),
            MethodPattern::Exact("size_hint".into()),
            MethodPattern::Exact("count".into()),
            MethodPattern::Exact("last".into()),
            MethodPattern::Exact("nth".into()),
        ],
        category: TraitCategory::Iterator,
    });

    registry.add(KnownTrait {
        path: "IntoIterator".into(),
        aliases: vec![
            "std::iter::IntoIterator".into(),
            "core::iter::IntoIterator".into(),
        ],
        method_patterns: vec![MethodPattern::Exact("into_iter".into())],
        category: TraitCategory::Iterator,
    });

    registry.add(KnownTrait {
        path: "FromIterator".into(),
        aliases: vec![
            "std::iter::FromIterator".into(),
            "core::iter::FromIterator".into(),
        ],
        method_patterns: vec![MethodPattern::Exact("from_iter".into())],
        category: TraitCategory::Iterator,
    });

    registry.add(KnownTrait {
        path: "ExactSizeIterator".into(),
        aliases: vec![
            "std::iter::ExactSizeIterator".into(),
            "core::iter::ExactSizeIterator".into(),
        ],
        method_patterns: vec![MethodPattern::Exact("len".into())],
        category: TraitCategory::Iterator,
    });

    registry.add(KnownTrait {
        path: "DoubleEndedIterator".into(),
        aliases: vec![
            "std::iter::DoubleEndedIterator".into(),
            "core::iter::DoubleEndedIterator".into(),
        ],
        method_patterns: vec![
            MethodPattern::Exact("next_back".into()),
            MethodPattern::Exact("nth_back".into()),
        ],
        category: TraitCategory::Iterator,
    });

    // Async traits
    registry.add(KnownTrait {
        path: "Future".into(),
        aliases: vec![
            "std::future::Future".into(),
            "core::future::Future".into(),
            "futures::Future".into(),
        ],
        method_patterns: vec![MethodPattern::Exact("poll".into())],
        category: TraitCategory::Async,
    });

    registry.add(KnownTrait {
        path: "Stream".into(),
        aliases: vec![
            "futures::Stream".into(),
            "futures_core::Stream".into(),
            "tokio_stream::Stream".into(),
        ],
        method_patterns: vec![
            MethodPattern::Exact("poll_next".into()),
            MethodPattern::Exact("size_hint".into()),
        ],
        category: TraitCategory::Async,
    });

    registry.add(KnownTrait {
        path: "Sink".into(),
        aliases: vec!["futures::Sink".into(), "futures_sink::Sink".into()],
        method_patterns: vec![
            MethodPattern::Exact("poll_ready".into()),
            MethodPattern::Exact("start_send".into()),
            MethodPattern::Exact("poll_flush".into()),
            MethodPattern::Exact("poll_close".into()),
        ],
        category: TraitCategory::Async,
    });

    // Comparison traits
    registry.add(KnownTrait {
        path: "PartialEq".into(),
        aliases: vec!["std::cmp::PartialEq".into(), "core::cmp::PartialEq".into()],
        method_patterns: vec![
            MethodPattern::Exact("eq".into()),
            MethodPattern::Exact("ne".into()),
        ],
        category: TraitCategory::Comparison,
    });

    registry.add(KnownTrait {
        path: "Eq".into(),
        aliases: vec!["std::cmp::Eq".into(), "core::cmp::Eq".into()],
        method_patterns: vec![],
        category: TraitCategory::Comparison,
    });

    registry.add(KnownTrait {
        path: "PartialOrd".into(),
        aliases: vec![
            "std::cmp::PartialOrd".into(),
            "core::cmp::PartialOrd".into(),
        ],
        method_patterns: vec![
            MethodPattern::Exact("partial_cmp".into()),
            MethodPattern::Exact("lt".into()),
            MethodPattern::Exact("le".into()),
            MethodPattern::Exact("gt".into()),
            MethodPattern::Exact("ge".into()),
        ],
        category: TraitCategory::Comparison,
    });

    registry.add(KnownTrait {
        path: "Ord".into(),
        aliases: vec!["std::cmp::Ord".into(), "core::cmp::Ord".into()],
        method_patterns: vec![
            MethodPattern::Exact("cmp".into()),
            MethodPattern::Exact("max".into()),
            MethodPattern::Exact("min".into()),
            MethodPattern::Exact("clamp".into()),
        ],
        category: TraitCategory::Comparison,
    });

    registry.add(KnownTrait {
        path: "Hash".into(),
        aliases: vec!["std::hash::Hash".into(), "core::hash::Hash".into()],
        method_patterns: vec![
            MethodPattern::Exact("hash".into()),
            MethodPattern::Exact("hash_slice".into()),
        ],
        category: TraitCategory::Comparison,
    });

    // Error traits
    registry.add(KnownTrait {
        path: "Error".into(),
        aliases: vec!["std::error::Error".into()],
        method_patterns: vec![
            MethodPattern::Exact("source".into()),
            MethodPattern::Exact("description".into()),
            MethodPattern::Exact("cause".into()),
        ],
        category: TraitCategory::Error,
    });

    // Standard traits
    registry.add(KnownTrait {
        path: "Default".into(),
        aliases: vec![
            "std::default::Default".into(),
            "core::default::Default".into(),
        ],
        method_patterns: vec![MethodPattern::Exact("default".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "Clone".into(),
        aliases: vec!["std::clone::Clone".into(), "core::clone::Clone".into()],
        method_patterns: vec![
            MethodPattern::Exact("clone".into()),
            MethodPattern::Exact("clone_from".into()),
        ],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "Drop".into(),
        aliases: vec!["std::ops::Drop".into(), "core::ops::Drop".into()],
        method_patterns: vec![MethodPattern::Exact("drop".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "Display".into(),
        aliases: vec!["std::fmt::Display".into(), "core::fmt::Display".into()],
        method_patterns: vec![MethodPattern::Exact("fmt".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "Debug".into(),
        aliases: vec!["std::fmt::Debug".into(), "core::fmt::Debug".into()],
        method_patterns: vec![MethodPattern::Exact("fmt".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "From".into(),
        aliases: vec!["std::convert::From".into(), "core::convert::From".into()],
        method_patterns: vec![MethodPattern::Exact("from".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "Into".into(),
        aliases: vec!["std::convert::Into".into(), "core::convert::Into".into()],
        method_patterns: vec![MethodPattern::Exact("into".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "TryFrom".into(),
        aliases: vec![
            "std::convert::TryFrom".into(),
            "core::convert::TryFrom".into(),
        ],
        method_patterns: vec![MethodPattern::Exact("try_from".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "TryInto".into(),
        aliases: vec![
            "std::convert::TryInto".into(),
            "core::convert::TryInto".into(),
        ],
        method_patterns: vec![MethodPattern::Exact("try_into".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "AsRef".into(),
        aliases: vec!["std::convert::AsRef".into(), "core::convert::AsRef".into()],
        method_patterns: vec![MethodPattern::Exact("as_ref".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "AsMut".into(),
        aliases: vec!["std::convert::AsMut".into(), "core::convert::AsMut".into()],
        method_patterns: vec![MethodPattern::Exact("as_mut".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "Deref".into(),
        aliases: vec!["std::ops::Deref".into(), "core::ops::Deref".into()],
        method_patterns: vec![MethodPattern::Exact("deref".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "DerefMut".into(),
        aliases: vec!["std::ops::DerefMut".into(), "core::ops::DerefMut".into()],
        method_patterns: vec![MethodPattern::Exact("deref_mut".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "Borrow".into(),
        aliases: vec!["std::borrow::Borrow".into(), "core::borrow::Borrow".into()],
        method_patterns: vec![MethodPattern::Exact("borrow".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "BorrowMut".into(),
        aliases: vec![
            "std::borrow::BorrowMut".into(),
            "core::borrow::BorrowMut".into(),
        ],
        method_patterns: vec![MethodPattern::Exact("borrow_mut".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "Index".into(),
        aliases: vec!["std::ops::Index".into(), "core::ops::Index".into()],
        method_patterns: vec![MethodPattern::Exact("index".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "IndexMut".into(),
        aliases: vec!["std::ops::IndexMut".into(), "core::ops::IndexMut".into()],
        method_patterns: vec![MethodPattern::Exact("index_mut".into())],
        category: TraitCategory::Standard,
    });

    registry
}

// ============================================================================
// Method Origin Classification
// ============================================================================

/// Classification of whether a method is trait-mandated or self-chosen.
///
/// This is the primary output of trait-mandated method detection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MethodOrigin {
    /// Method is required by a trait implementation (cannot be extracted)
    TraitMandated {
        /// Name of the trait requiring this method
        trait_name: String,
        /// Category of the trait
        category: TraitCategory,
    },
    /// Method is the author's design choice (potentially extractable)
    SelfChosen,
}

impl MethodOrigin {
    /// Returns the weight for this method origin.
    ///
    /// Trait-mandated methods get reduced weight based on their category.
    /// Self-chosen methods get full weight.
    #[must_use]
    pub fn weight(&self) -> f64 {
        match self {
            Self::TraitMandated { category, .. } => category.default_weight(),
            Self::SelfChosen => 1.0,
        }
    }

    /// Returns whether this method is trait-mandated.
    #[must_use]
    pub fn is_trait_mandated(&self) -> bool {
        matches!(self, Self::TraitMandated { .. })
    }

    /// Returns whether this method is self-chosen (extractable).
    #[must_use]
    pub fn is_extractable(&self) -> bool {
        matches!(self, Self::SelfChosen)
    }
}

impl Default for MethodOrigin {
    fn default() -> Self {
        Self::SelfChosen
    }
}

/// A method with its origin classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedMethod {
    /// Name of the method
    pub name: String,
    /// Origin classification (trait-mandated or self-chosen)
    pub origin: MethodOrigin,
    /// Weight for god object scoring (0.0-1.0)
    pub weight: f64,
}

impl ClassifiedMethod {
    /// Create a new classified method.
    #[must_use]
    pub fn new(name: String, origin: MethodOrigin) -> Self {
        let weight = origin.weight();
        Self {
            name,
            origin,
            weight,
        }
    }
}

// ============================================================================
// Trait Implementation Info
// ============================================================================

/// Information about a trait implementation on a struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitImplInfo {
    /// Trait being implemented (e.g., "syn::Visit", "Iterator")
    pub trait_path: String,
    /// Methods provided in this impl block
    pub method_names: Vec<String>,
    /// Category of the trait
    pub category: TraitCategory,
}

impl TraitImplInfo {
    /// Create a new trait impl info.
    #[must_use]
    pub fn new(trait_path: String, method_names: Vec<String>, category: TraitCategory) -> Self {
        Self {
            trait_path,
            method_names,
            category,
        }
    }

    /// Check if a method name is part of this trait implementation.
    #[must_use]
    pub fn contains_method(&self, method_name: &str) -> bool {
        self.method_names.iter().any(|m| m == method_name)
    }
}

// ============================================================================
// Trait Method Summary
// ============================================================================

/// Summary of trait-mandated vs extractable methods for god object analysis.
///
/// This provides the breakdown shown in the TUI:
/// ```text
/// methods                   32 (14 trait-mandated, 18 extractable)
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraitMethodSummary {
    /// Total trait-mandated methods
    pub mandated_count: usize,
    /// Breakdown by trait
    pub by_trait: HashMap<String, usize>,
    /// Weighted method count (after applying trait discounts)
    pub weighted_count: f64,
    /// Self-chosen (extractable) method count
    pub extractable_count: usize,
    /// Total methods (mandated + extractable)
    pub total_methods: usize,
}

impl TraitMethodSummary {
    /// Create a new empty summary.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a summary from classified methods.
    #[must_use]
    pub fn from_classifications(methods: &[ClassifiedMethod]) -> Self {
        let mut by_trait: HashMap<String, usize> = HashMap::new();
        let mut mandated_count = 0;
        let mut extractable_count = 0;
        let mut weighted_count = 0.0;

        for method in methods {
            weighted_count += method.weight;
            match &method.origin {
                MethodOrigin::TraitMandated { trait_name, .. } => {
                    mandated_count += 1;
                    *by_trait.entry(trait_name.clone()).or_insert(0) += 1;
                }
                MethodOrigin::SelfChosen => {
                    extractable_count += 1;
                }
            }
        }

        Self {
            mandated_count,
            by_trait,
            weighted_count,
            extractable_count,
            total_methods: methods.len(),
        }
    }

    /// Returns the ratio of extractable methods (0.0 to 1.0).
    #[must_use]
    pub fn extractable_ratio(&self) -> f64 {
        if self.total_methods == 0 {
            return 1.0;
        }
        self.extractable_count as f64 / self.total_methods as f64
    }

    /// Returns the ratio of mandated methods (0.0 to 1.0).
    #[must_use]
    pub fn mandated_ratio(&self) -> f64 {
        if self.total_methods == 0 {
            return 0.0;
        }
        self.mandated_count as f64 / self.total_methods as f64
    }

    /// Returns true if trait methods dominate (>50% mandated).
    #[must_use]
    pub fn is_trait_dominated(&self) -> bool {
        self.mandated_ratio() > 0.5
    }

    /// Format the trait breakdown for display.
    #[must_use]
    pub fn format_trait_breakdown(&self) -> String {
        if self.by_trait.is_empty() {
            return String::new();
        }

        let mut items: Vec<_> = self.by_trait.iter().collect();
        items.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

        items
            .iter()
            .map(|(trait_name, count)| format!("{} ({})", trait_name, count))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl std::fmt::Display for TraitMethodSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({} trait-mandated, {} extractable)",
            self.total_methods, self.mandated_count, self.extractable_count
        )
    }
}

// ============================================================================
// Classification Functions (Pure)
// ============================================================================

/// Classify the origin of a method given trait implementations.
///
/// This is a pure function that determines whether a method is trait-mandated
/// (required by a trait impl) or self-chosen (author's design choice).
///
/// # Arguments
///
/// * `method_name` - Name of the method to classify
/// * `trait_impls` - List of trait implementations for this struct
/// * `registry` - Known trait registry for category lookup
///
/// # Returns
///
/// `MethodOrigin::TraitMandated` if the method is in a trait impl,
/// `MethodOrigin::SelfChosen` otherwise.
#[must_use]
pub fn classify_method_origin(
    method_name: &str,
    trait_impls: &[TraitImplInfo],
    registry: &KnownTraitRegistry,
) -> MethodOrigin {
    for impl_info in trait_impls {
        if impl_info.contains_method(method_name) {
            let category = registry.categorize_trait(&impl_info.trait_path);
            return MethodOrigin::TraitMandated {
                trait_name: impl_info.trait_path.clone(),
                category,
            };
        }
    }
    MethodOrigin::SelfChosen
}

/// Classify all methods for a struct.
///
/// # Arguments
///
/// * `method_names` - All method names for this struct
/// * `trait_impls` - List of trait implementations
/// * `registry` - Known trait registry
///
/// # Returns
///
/// A vector of `ClassifiedMethod` with origin and weight information.
#[must_use]
pub fn classify_all_methods(
    method_names: &[String],
    trait_impls: &[TraitImplInfo],
    registry: &KnownTraitRegistry,
) -> Vec<ClassifiedMethod> {
    method_names
        .iter()
        .map(|name| {
            let origin = classify_method_origin(name, trait_impls, registry);
            ClassifiedMethod::new(name.clone(), origin)
        })
        .collect()
}

/// Calculate the weighted method count with trait adjustments.
///
/// This applies trait-based weights on top of existing method complexity weights.
///
/// # Arguments
///
/// * `methods` - Classified methods with origin information
///
/// # Returns
///
/// Sum of weights for all methods (trait-mandated methods have reduced weight).
#[must_use]
pub fn calculate_trait_weighted_count(methods: &[ClassifiedMethod]) -> f64 {
    methods.iter().map(|m| m.weight).sum()
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_pattern_exact() {
        let pattern = MethodPattern::Exact("next".into());
        assert!(pattern.matches("next"));
        assert!(!pattern.matches("next_back"));
        assert!(!pattern.matches(""));
    }

    #[test]
    fn test_method_pattern_prefix() {
        let pattern = MethodPattern::Prefix("visit_".into());
        assert!(pattern.matches("visit_expr"));
        assert!(pattern.matches("visit_stmt"));
        assert!(!pattern.matches("fold_expr"));
        assert!(!pattern.matches("visit"));
    }

    #[test]
    fn test_method_pattern_suffix() {
        let pattern = MethodPattern::Suffix("_mut".into());
        assert!(pattern.matches("get_mut"));
        assert!(pattern.matches("index_mut"));
        assert!(!pattern.matches("mut_get"));
    }

    #[test]
    fn test_known_trait_matches() {
        let trait_info = KnownTrait {
            path: "syn::Visit".into(),
            aliases: vec!["Visit".into()],
            method_patterns: vec![MethodPattern::Prefix("visit_".into())],
            category: TraitCategory::Visitor,
        };

        // Full path
        assert!(trait_info.matches_trait_name("syn::Visit"));
        // Alias
        assert!(trait_info.matches_trait_name("Visit"));
        // Partial path
        assert!(trait_info.matches_trait_name("Visit"));
        // Non-matching
        assert!(!trait_info.matches_trait_name("Iterator"));
    }

    #[test]
    fn test_registry_default() {
        let registry = KnownTraitRegistry::default();
        assert!(!registry.is_empty());

        // Check some known traits
        assert!(registry.find("syn::Visit").is_some());
        assert!(registry.find("Iterator").is_some());
        assert!(registry.find("Default").is_some());
        assert!(registry.find("Clone").is_some());
    }

    #[test]
    fn test_registry_categorize() {
        let registry = KnownTraitRegistry::default();

        assert_eq!(
            registry.categorize_trait("syn::Visit"),
            TraitCategory::Visitor
        );
        assert_eq!(
            registry.categorize_trait("Iterator"),
            TraitCategory::Iterator
        );
        assert_eq!(
            registry.categorize_trait("Default"),
            TraitCategory::Standard
        );
        assert_eq!(
            registry.categorize_trait("UnknownTrait"),
            TraitCategory::Custom
        );
    }

    #[test]
    fn test_trait_category_weights() {
        assert!((TraitCategory::Visitor.default_weight() - 0.1).abs() < f64::EPSILON);
        assert!((TraitCategory::Serialization.default_weight() - 0.1).abs() < f64::EPSILON);
        assert!((TraitCategory::Iterator.default_weight() - 0.3).abs() < f64::EPSILON);
        assert!((TraitCategory::Standard.default_weight() - 0.2).abs() < f64::EPSILON);
        assert!((TraitCategory::Custom.default_weight() - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn test_method_origin_weights() {
        let mandated = MethodOrigin::TraitMandated {
            trait_name: "syn::Visit".into(),
            category: TraitCategory::Visitor,
        };
        assert!((mandated.weight() - 0.1).abs() < f64::EPSILON);

        let self_chosen = MethodOrigin::SelfChosen;
        assert!((self_chosen.weight() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_classify_method_origin() {
        let registry = KnownTraitRegistry::default();
        let trait_impls = vec![TraitImplInfo::new(
            "syn::Visit".into(),
            vec!["visit_expr".into(), "visit_stmt".into()],
            TraitCategory::Visitor,
        )];

        // Trait-mandated
        let origin = classify_method_origin("visit_expr", &trait_impls, &registry);
        assert!(origin.is_trait_mandated());

        // Self-chosen
        let origin = classify_method_origin("process_data", &trait_impls, &registry);
        assert!(origin.is_extractable());
    }

    #[test]
    fn test_trait_method_summary() {
        let registry = KnownTraitRegistry::default();
        let trait_impls = vec![TraitImplInfo::new(
            "syn::Visit".into(),
            vec!["visit_expr".into(), "visit_stmt".into()],
            TraitCategory::Visitor,
        )];

        let method_names = vec![
            "visit_expr".into(),
            "visit_stmt".into(),
            "process_data".into(),
            "helper".into(),
        ];

        let classified = classify_all_methods(&method_names, &trait_impls, &registry);
        let summary = TraitMethodSummary::from_classifications(&classified);

        assert_eq!(summary.total_methods, 4);
        assert_eq!(summary.mandated_count, 2);
        assert_eq!(summary.extractable_count, 2);
        assert_eq!(*summary.by_trait.get("syn::Visit").unwrap(), 2);

        // Weighted: 2 * 0.1 (visitor) + 2 * 1.0 (self-chosen) = 2.2
        assert!((summary.weighted_count - 2.2).abs() < 0.01);
    }

    #[test]
    fn test_trait_method_summary_display() {
        let summary = TraitMethodSummary {
            mandated_count: 14,
            by_trait: [("syn::Visit".into(), 14)].into_iter().collect(),
            weighted_count: 15.8,
            extractable_count: 18,
            total_methods: 32,
        };

        assert_eq!(
            summary.to_string(),
            "32 (14 trait-mandated, 18 extractable)"
        );
    }
}
