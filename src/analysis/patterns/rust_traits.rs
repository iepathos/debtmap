//! Rust trait-based pattern recognition
//!
//! Detects design patterns implemented using Rust traits, such as:
//! - Trait-based observer patterns
//! - Visitor pattern through trait implementations

use super::{Implementation, PatternInstance, PatternType, UsageSite};
use crate::analysis::call_graph::TraitRegistry;
use std::sync::Arc;

pub struct RustTraitPatternRecognizer {
    trait_registry: Arc<TraitRegistry>,
}

impl RustTraitPatternRecognizer {
    pub fn new(trait_registry: Arc<TraitRegistry>) -> Self {
        Self { trait_registry }
    }

    /// Detect trait-based observer patterns in Rust
    pub fn detect_trait_observer_patterns(&self) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        let stats = self.trait_registry.get_statistics();
        if stats.total_traits == 0 {
            return patterns;
        }

        let unresolved_calls = self.trait_registry.get_unresolved_trait_calls();

        let mut trait_names = std::collections::HashSet::new();
        for call in unresolved_calls.iter() {
            if call.trait_name != "Unknown" {
                trait_names.insert(call.trait_name.clone());
            }
        }

        for trait_name in &trait_names {
            if let Some(implementations) = self.trait_registry.find_implementations(trait_name) {
                if !implementations.is_empty() {
                    let impls: Vec<Implementation> = implementations
                        .iter()
                        .map(|impl_info| Implementation {
                            file: impl_info.method_id.file.clone(),
                            class_name: None,
                            function_name: impl_info.method_name.clone(),
                            line: impl_info.method_id.line,
                        })
                        .collect();

                    let usage_sites = self.find_trait_method_calls(trait_name);

                    patterns.push(PatternInstance {
                        pattern_type: PatternType::Observer,
                        confidence: 0.95,
                        base_class: Some(trait_name.clone()),
                        implementations: impls,
                        usage_sites,
                        reasoning: format!(
                            "Rust trait {} with {} implementations",
                            trait_name,
                            implementations.len()
                        ),
                    });
                }
            }
        }

        patterns
    }

    fn find_trait_method_calls(&self, trait_name: &str) -> Vec<UsageSite> {
        self.trait_registry
            .get_unresolved_trait_calls()
            .iter()
            .filter(|call| call.trait_name == trait_name || call.trait_name == "Unknown")
            .map(|call| UsageSite {
                file: call.caller.file.clone(),
                line: call.line,
                context: format!("Trait method call: {}", call.method_name),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_trait_recognizer_creation() {
        let registry = Arc::new(TraitRegistry::new());
        let recognizer = RustTraitPatternRecognizer::new(registry);
        let patterns = recognizer.detect_trait_observer_patterns();
        assert_eq!(patterns.len(), 0);
    }
}
