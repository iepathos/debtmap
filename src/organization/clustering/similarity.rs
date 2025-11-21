//! Similarity calculation for method clustering
//!
//! Calculates similarity between methods using multiple signals:
//! - Call graph connectivity
//! - Data dependencies
//! - Naming patterns
//! - Behavioral patterns
//! - Architectural layer

use super::{ArchitecturalLayer, Method, Visibility};
#[cfg(test)]
use std::collections::HashMap;
use std::collections::HashSet;

/// Weights for different similarity signals
#[derive(Debug, Clone)]
pub struct SimilarityWeights {
    pub call_graph: f64,
    pub data_deps: f64,
    pub naming: f64,
    pub behavioral: f64,
    pub layer: f64,
}

impl Default for SimilarityWeights {
    fn default() -> Self {
        Self {
            call_graph: 0.40,
            data_deps: 0.25,
            naming: 0.20,
            behavioral: 0.10,
            layer: 0.05,
        }
    }
}

/// Call graph interface for similarity calculation
pub trait CallGraphProvider {
    /// Get number of calls from method1 to method2
    fn call_count(&self, from: &str, to: &str) -> usize;

    /// Get methods called by the given method
    fn callees(&self, method: &str) -> HashSet<String>;

    /// Get methods that call the given method
    fn callers(&self, method: &str) -> HashSet<String>;
}

/// Field access tracker interface
pub trait FieldAccessProvider {
    /// Get fields accessed by the given method
    fn fields_accessed_by(&self, method: &str) -> HashSet<String>;

    /// Check if method writes to a field
    fn writes_to_field(&self, method: &str, field: &str) -> bool;
}

/// Calculator for method similarity using multiple signals
pub struct ClusteringSimilarityCalculator<C, F> {
    call_graph: C,
    field_tracker: F,
    weights: SimilarityWeights,
}

impl<C: CallGraphProvider, F: FieldAccessProvider> ClusteringSimilarityCalculator<C, F> {
    pub fn new(call_graph: C, field_tracker: F) -> Self {
        Self {
            call_graph,
            field_tracker,
            weights: SimilarityWeights::default(),
        }
    }

    pub fn with_weights(call_graph: C, field_tracker: F, weights: SimilarityWeights) -> Self {
        Self {
            call_graph,
            field_tracker,
            weights,
        }
    }

    /// Calculate overall similarity between two methods
    pub fn calculate_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        let call_sim = self.call_graph_similarity(method1, method2);
        let data_sim = self.data_dependency_similarity(method1, method2);
        let naming_sim = self.naming_similarity(method1, method2);
        let behavior_sim = self.behavioral_similarity(method1, method2);
        let layer_sim = self.layer_similarity(method1, method2);

        self.weights.call_graph * call_sim
            + self.weights.data_deps * data_sim
            + self.weights.naming * naming_sim
            + self.weights.behavioral * behavior_sim
            + self.weights.layer * layer_sim
    }

    /// Calculate call graph similarity (40% weight)
    fn call_graph_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        // Bidirectional call strength
        let calls_1_to_2 = self.call_graph.call_count(&method1.name, &method2.name);
        let calls_2_to_1 = self.call_graph.call_count(&method2.name, &method1.name);

        // Shared callees (both call same methods)
        let callees1 = self.call_graph.callees(&method1.name);
        let callees2 = self.call_graph.callees(&method2.name);
        let shared_callees = callees1.intersection(&callees2).count();

        // Shared callers (both are called by same methods)
        let callers1 = self.call_graph.callers(&method1.name);
        let callers2 = self.call_graph.callers(&method2.name);
        let shared_callers = callers1.intersection(&callers2).count();

        // Combine signals
        let direct_calls = (calls_1_to_2 + calls_2_to_1) as f64;
        let shared = (shared_callees + shared_callers) as f64;

        // Normalize
        if direct_calls > 0.0 {
            1.0 // Direct calls = strongest signal
        } else if shared > 0.0 {
            0.5 + (shared / 20.0).min(0.4) // Shared connections = medium signal
        } else {
            0.0
        }
    }

    /// Calculate data dependency similarity (25% weight)
    fn data_dependency_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        // Shared field accesses
        let fields1 = self.field_tracker.fields_accessed_by(&method1.name);
        let fields2 = self.field_tracker.fields_accessed_by(&method2.name);

        let shared_fields = fields1.intersection(&fields2).count();
        let total_fields = fields1.union(&fields2).count();

        if total_fields == 0 {
            return 0.0;
        }

        let jaccard = shared_fields as f64 / total_fields as f64;

        // Bonus: Both write to same field
        let shared_writes = fields1
            .iter()
            .filter(|f| {
                self.field_tracker.writes_to_field(&method1.name, f)
                    && self.field_tracker.writes_to_field(&method2.name, f)
            })
            .count();

        if shared_writes > 0 {
            (jaccard + 0.3).min(1.0)
        } else {
            jaccard
        }
    }

    /// Calculate naming similarity (20% weight)
    fn naming_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        let name1 = &method1.name;
        let name2 = &method2.name;

        // Common prefix
        let common_prefix_len = name1
            .chars()
            .zip(name2.chars())
            .take_while(|(a, b)| a == b)
            .count();

        if common_prefix_len >= 4 {
            return 0.8;
        }

        // Tokenize and compare
        let tokens1 = tokenize_method_name(name1);
        let tokens2 = tokenize_method_name(name2);

        let shared_tokens = tokens1.intersection(&tokens2).count();
        let total_tokens = tokens1.union(&tokens2).count();

        if total_tokens == 0 {
            return 0.0;
        }

        shared_tokens as f64 / total_tokens as f64
    }

    /// Calculate behavioral similarity (10% weight)
    fn behavioral_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        let mut score = 0.0;

        // Same purity
        if method1.is_pure == method2.is_pure {
            score += 0.3;
        }

        // Same visibility
        if method1.visibility == method2.visibility {
            score += 0.2;
        }

        // Similar complexity
        let complexity_ratio = if method2.complexity > 0 {
            (method1.complexity as f64 / method2.complexity as f64).min(1.0)
        } else {
            1.0
        };
        score += 0.3 * complexity_ratio;

        // Similar I/O patterns
        if (method1.has_io && method2.has_io) || (!method1.has_io && !method2.has_io) {
            score += 0.2;
        }

        score.min(1.0)
    }

    /// Calculate layer similarity (5% weight)
    fn layer_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        let layer1 = self.detect_layer(method1);
        let layer2 = self.detect_layer(method2);

        if layer1 == layer2 {
            1.0
        } else if layer1.is_adjacent_to(&layer2) {
            0.5
        } else {
            0.0
        }
    }

    /// Detect architectural layer for a method
    fn detect_layer(&self, method: &Method) -> ArchitecturalLayer {
        if method.visibility == Visibility::Public {
            ArchitecturalLayer::Api
        } else if method.complexity > 20 {
            ArchitecturalLayer::Core
        } else if method.complexity < 5 {
            ArchitecturalLayer::Utility
        } else {
            ArchitecturalLayer::Internal
        }
    }
}

/// Tokenize method name into semantic tokens
fn tokenize_method_name(name: &str) -> HashSet<String> {
    name.split('_')
        .filter(|token| !token.is_empty())
        .map(|token| token.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCallGraph {
        calls: HashMap<(String, String), usize>,
        callees: HashMap<String, HashSet<String>>,
        callers: HashMap<String, HashSet<String>>,
    }

    impl CallGraphProvider for MockCallGraph {
        fn call_count(&self, from: &str, to: &str) -> usize {
            *self
                .calls
                .get(&(from.to_string(), to.to_string()))
                .unwrap_or(&0)
        }

        fn callees(&self, method: &str) -> HashSet<String> {
            self.callees.get(method).cloned().unwrap_or_default()
        }

        fn callers(&self, method: &str) -> HashSet<String> {
            self.callers.get(method).cloned().unwrap_or_default()
        }
    }

    struct MockFieldAccess {
        fields: HashMap<String, HashSet<String>>,
        writes: HashMap<(String, String), bool>,
    }

    impl FieldAccessProvider for MockFieldAccess {
        fn fields_accessed_by(&self, method: &str) -> HashSet<String> {
            self.fields.get(method).cloned().unwrap_or_default()
        }

        fn writes_to_field(&self, method: &str, field: &str) -> bool {
            *self
                .writes
                .get(&(method.to_string(), field.to_string()))
                .unwrap_or(&false)
        }
    }

    fn create_method(name: &str) -> Method {
        Method {
            name: name.to_string(),
            is_pure: false,
            visibility: Visibility::Private,
            complexity: 10,
            has_io: false,
        }
    }

    #[test]
    fn test_naming_similarity_common_prefix() {
        let call_graph = MockCallGraph {
            calls: HashMap::new(),
            callees: HashMap::new(),
            callers: HashMap::new(),
        };
        let field_tracker = MockFieldAccess {
            fields: HashMap::new(),
            writes: HashMap::new(),
        };

        let calc = ClusteringSimilarityCalculator::new(call_graph, field_tracker);

        let method1 = create_method("format_item");
        let method2 = create_method("format_details");

        let similarity = calc.naming_similarity(&method1, &method2);
        assert!(
            similarity > 0.7,
            "Common prefix should give high similarity"
        );
    }

    #[test]
    fn test_tokenize_method_name() {
        let tokens = tokenize_method_name("format_item_details");
        assert_eq!(tokens.len(), 3);
        assert!(tokens.contains("format"));
        assert!(tokens.contains("item"));
        assert!(tokens.contains("details"));
    }
}
