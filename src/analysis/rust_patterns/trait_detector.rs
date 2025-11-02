use crate::analysis::multi_signal_aggregation::ResponsibilityCategory;
use crate::analysis::rust_patterns::context::RustFunctionContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StandardTrait {
    // Formatting
    Display,
    Debug,

    // Conversions
    From,
    Into,
    TryFrom,
    TryInto,
    AsRef,
    AsMut,

    // Construction
    Default,
    Clone,

    // Resource Management
    Drop,

    // Iteration
    Iterator,
    IntoIterator,

    // Operators
    Add,
    Sub,
    Mul,
    Div,
    Deref,
    DerefMut,

    // Comparison
    PartialEq,
    Eq,
    PartialOrd,
    Ord,

    // Hashing
    Hash,

    // Serialization (common crates)
    Serialize,
    Deserialize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraitImplClassification {
    pub trait_name: String,
    pub standard_trait: Option<StandardTrait>,
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
}

pub struct RustTraitDetector {
    trait_patterns: HashMap<StandardTrait, ResponsibilityCategory>,
}

impl RustTraitDetector {
    pub fn new() -> Self {
        let mut trait_patterns = HashMap::new();

        trait_patterns.insert(StandardTrait::Display, ResponsibilityCategory::Formatting);
        trait_patterns.insert(StandardTrait::Debug, ResponsibilityCategory::Formatting);

        // For type conversions, we'll need to add TypeConversion category to ResponsibilityCategory
        // For now, map to Transformation
        trait_patterns.insert(StandardTrait::From, ResponsibilityCategory::Transformation);
        trait_patterns.insert(StandardTrait::Into, ResponsibilityCategory::Transformation);
        trait_patterns.insert(
            StandardTrait::TryFrom,
            ResponsibilityCategory::Transformation,
        );
        trait_patterns.insert(
            StandardTrait::TryInto,
            ResponsibilityCategory::Transformation,
        );

        trait_patterns.insert(
            StandardTrait::Default,
            ResponsibilityCategory::PureComputation,
        );
        trait_patterns.insert(
            StandardTrait::Clone,
            ResponsibilityCategory::PureComputation,
        );

        // Drop will need ResourceCleanup category - map to SideEffects for now
        trait_patterns.insert(StandardTrait::Drop, ResponsibilityCategory::SideEffects);

        // Iterator patterns - map to Transformation for now
        trait_patterns.insert(
            StandardTrait::Iterator,
            ResponsibilityCategory::Transformation,
        );
        trait_patterns.insert(
            StandardTrait::IntoIterator,
            ResponsibilityCategory::Transformation,
        );

        // Operators are "Computation"
        for op_trait in [
            StandardTrait::Add,
            StandardTrait::Sub,
            StandardTrait::Mul,
            StandardTrait::Div,
        ] {
            trait_patterns.insert(op_trait, ResponsibilityCategory::PureComputation);
        }

        // Deref operations
        trait_patterns.insert(
            StandardTrait::Deref,
            ResponsibilityCategory::PureComputation,
        );
        trait_patterns.insert(
            StandardTrait::DerefMut,
            ResponsibilityCategory::PureComputation,
        );

        // Comparison traits
        for cmp_trait in [
            StandardTrait::PartialEq,
            StandardTrait::Eq,
            StandardTrait::PartialOrd,
            StandardTrait::Ord,
        ] {
            trait_patterns.insert(cmp_trait, ResponsibilityCategory::PureComputation);
        }

        trait_patterns.insert(StandardTrait::Hash, ResponsibilityCategory::PureComputation);

        // Serialization
        trait_patterns.insert(
            StandardTrait::Serialize,
            ResponsibilityCategory::Transformation,
        );
        trait_patterns.insert(StandardTrait::Deserialize, ResponsibilityCategory::Parsing);

        RustTraitDetector { trait_patterns }
    }

    /// Detect trait implementation from context
    pub fn detect_trait_impl(
        &self,
        context: &RustFunctionContext,
    ) -> Option<TraitImplClassification> {
        // Check if this function is a trait method
        if !context.is_trait_impl() {
            return None;
        }

        let trait_name = context.trait_name()?;

        // Match against standard traits
        let standard_trait = self.match_standard_trait(trait_name);
        let category = standard_trait
            .as_ref()
            .and_then(|st| self.trait_patterns.get(st))
            .copied()
            .unwrap_or(ResponsibilityCategory::Unknown);

        Some(TraitImplClassification {
            trait_name: trait_name.to_string(),
            standard_trait,
            category,
            confidence: 0.95, // High confidence for trait impls
            evidence: format!("Implements {} trait", trait_name),
        })
    }

    /// Match trait name to standard trait enum
    /// Handles both simple names and qualified paths
    fn match_standard_trait(&self, trait_name: &str) -> Option<StandardTrait> {
        // Extract final segment for matching
        let simple_name = trait_name.split("::").last()?;

        match simple_name {
            "Display" => Some(StandardTrait::Display),
            "Debug" => Some(StandardTrait::Debug),
            "From" => Some(StandardTrait::From),
            "Into" => Some(StandardTrait::Into),
            "TryFrom" => Some(StandardTrait::TryFrom),
            "TryInto" => Some(StandardTrait::TryInto),
            "AsRef" => Some(StandardTrait::AsRef),
            "AsMut" => Some(StandardTrait::AsMut),
            "Default" => Some(StandardTrait::Default),
            "Clone" => Some(StandardTrait::Clone),
            "Drop" => Some(StandardTrait::Drop),
            "Iterator" => Some(StandardTrait::Iterator),
            "IntoIterator" => Some(StandardTrait::IntoIterator),
            "Add" => Some(StandardTrait::Add),
            "Sub" => Some(StandardTrait::Sub),
            "Mul" => Some(StandardTrait::Mul),
            "Div" => Some(StandardTrait::Div),
            "Deref" => Some(StandardTrait::Deref),
            "DerefMut" => Some(StandardTrait::DerefMut),
            "PartialEq" => Some(StandardTrait::PartialEq),
            "Eq" => Some(StandardTrait::Eq),
            "PartialOrd" => Some(StandardTrait::PartialOrd),
            "Ord" => Some(StandardTrait::Ord),
            "Hash" => Some(StandardTrait::Hash),
            "Serialize" => Some(StandardTrait::Serialize),
            "Deserialize" => Some(StandardTrait::Deserialize),
            _ => None,
        }
    }
}

impl Default for RustTraitDetector {
    fn default() -> Self {
        Self::new()
    }
}
