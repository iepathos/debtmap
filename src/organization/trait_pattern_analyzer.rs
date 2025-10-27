/// Trait pattern analysis for detecting repetitive trait implementations.
///
/// This analyzer examines Rust files for patterns like:
/// - Multiple implementations of the same trait
/// - Consistent method signatures across implementations
/// - Low complexity in implementation methods
///
/// These patterns indicate opportunities for macro-ification or code generation.
use std::collections::{HashMap, HashSet};
use syn::{self, visit::Visit};

/// Analyzer for trait implementation patterns
pub struct TraitPatternAnalyzer;

impl TraitPatternAnalyzer {
    /// Analyze a file for trait implementation patterns
    ///
    /// Returns metrics about trait implementations, method uniformity, and complexity.
    pub fn analyze_file(ast: &syn::File) -> TraitPatternMetrics {
        let mut visitor = ImplVisitor::new();
        visitor.visit_file(ast);

        let impl_block_count = visitor.impl_blocks.len();
        let unique_traits: HashSet<String> = visitor
            .impl_blocks
            .iter()
            .filter_map(|impl_block| impl_block.trait_name.clone())
            .collect();

        // Find most common trait
        let mut trait_counts: HashMap<String, usize> = HashMap::new();
        for impl_block in &visitor.impl_blocks {
            if let Some(trait_name) = &impl_block.trait_name {
                *trait_counts.entry(trait_name.clone()).or_insert(0) += 1;
            }
        }
        let most_common_trait = trait_counts.into_iter().max_by_key(|(_, count)| *count);

        // Calculate method uniformity
        let method_uniformity = Self::calculate_method_uniformity(&visitor.impl_blocks);

        // Detect shared methods
        let shared_methods = Self::detect_shared_methods(&visitor.impl_blocks);

        // Calculate complexity metrics
        let (avg_method_complexity, complexity_variance) =
            Self::calculate_complexity_metrics(&visitor.impl_blocks);

        // Calculate average method lines
        let avg_method_lines = Self::calculate_avg_method_lines(&visitor.impl_blocks);

        TraitPatternMetrics {
            impl_block_count,
            unique_traits,
            most_common_trait,
            method_uniformity,
            shared_methods,
            avg_method_complexity,
            complexity_variance,
            avg_method_lines,
        }
    }

    /// Calculate percentage of implementations sharing the same methods
    ///
    /// Returns a value between 0.0 and 1.0 indicating how uniform the method
    /// signatures are across all implementations.
    pub fn calculate_method_uniformity(impl_blocks: &[ImplBlockInfo]) -> f64 {
        if impl_blocks.is_empty() {
            return 0.0;
        }

        // Count method name occurrences
        let mut method_counts: HashMap<String, usize> = HashMap::new();
        for impl_block in impl_blocks {
            for method_name in &impl_block.method_names {
                *method_counts.entry(method_name.clone()).or_insert(0) += 1;
            }
        }

        if method_counts.is_empty() {
            return 0.0;
        }

        // Find the most common method count
        let max_count = method_counts.values().max().copied().unwrap_or(0);

        // Calculate uniformity as ratio of max_count to total impl blocks
        max_count as f64 / impl_blocks.len() as f64
    }

    /// Identify methods that appear in most implementations
    ///
    /// Returns list of (method_name, frequency_percentage) tuples.
    pub fn detect_shared_methods(impl_blocks: &[ImplBlockInfo]) -> Vec<(String, f64)> {
        if impl_blocks.is_empty() {
            return vec![];
        }

        let mut method_counts: HashMap<String, usize> = HashMap::new();
        for impl_block in impl_blocks {
            for method_name in &impl_block.method_names {
                *method_counts.entry(method_name.clone()).or_insert(0) += 1;
            }
        }

        let total_impls = impl_blocks.len();
        let mut shared: Vec<(String, f64)> = method_counts
            .into_iter()
            .map(|(name, count)| (name, count as f64 / total_impls as f64))
            .filter(|(_, freq)| *freq >= 0.5) // At least 50% of impls have this method
            .collect();

        shared.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        shared
    }

    /// Calculate average complexity and variance across methods
    fn calculate_complexity_metrics(impl_blocks: &[ImplBlockInfo]) -> (f64, f64) {
        let mut complexities = Vec::new();

        for impl_block in impl_blocks {
            for complexity in &impl_block.method_complexities {
                complexities.push(*complexity as f64);
            }
        }

        if complexities.is_empty() {
            return (0.0, 0.0);
        }

        let avg = complexities.iter().sum::<f64>() / complexities.len() as f64;

        let variance = if complexities.len() > 1 {
            let sum_sq_diff: f64 = complexities.iter().map(|x| (x - avg).powi(2)).sum();
            sum_sq_diff / complexities.len() as f64
        } else {
            0.0
        };

        (avg, variance)
    }

    /// Calculate average lines per method
    fn calculate_avg_method_lines(impl_blocks: &[ImplBlockInfo]) -> f64 {
        let total_lines: usize = impl_blocks.iter().map(|b| b.total_lines).sum();
        let total_methods: usize = impl_blocks.iter().map(|b| b.method_names.len()).sum();

        if total_methods == 0 {
            0.0
        } else {
            total_lines as f64 / total_methods as f64
        }
    }
}

/// Metrics about trait implementation patterns in a file
#[derive(Debug, Clone)]
pub struct TraitPatternMetrics {
    /// Total number of impl blocks
    pub impl_block_count: usize,
    /// Unique trait names implemented
    pub unique_traits: HashSet<String>,
    /// Most frequently implemented trait
    pub most_common_trait: Option<(String, usize)>,
    /// Percentage of implementations sharing the same methods (0.0-1.0)
    pub method_uniformity: f64,
    /// Methods that appear in 50%+ of implementations
    pub shared_methods: Vec<(String, f64)>,
    /// Average cyclomatic complexity of methods
    pub avg_method_complexity: f64,
    /// Variance in method complexity
    pub complexity_variance: f64,
    /// Average lines per method
    pub avg_method_lines: f64,
}

/// Information about a single impl block
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ImplBlockInfo {
    trait_name: Option<String>,
    type_name: String,
    method_names: Vec<String>,
    method_complexities: Vec<u32>,
    total_lines: usize,
}

/// Visitor to extract impl block information
struct ImplVisitor {
    impl_blocks: Vec<ImplBlockInfo>,
}

impl ImplVisitor {
    fn new() -> Self {
        Self {
            impl_blocks: Vec::new(),
        }
    }

    /// Extract type name from self_ty
    fn extract_type_name(self_ty: &syn::Type) -> String {
        match self_ty {
            syn::Type::Path(type_path) => type_path
                .path
                .get_ident()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            _ => "Unknown".to_string(),
        }
    }

    /// Extract trait name from trait reference
    fn extract_trait_name(
        trait_ref: &Option<(Option<syn::token::Not>, syn::Path, syn::token::For)>,
    ) -> Option<String> {
        trait_ref
            .as_ref()
            .and_then(|(_, path, _)| path.segments.last().map(|seg| seg.ident.to_string()))
    }

    /// Estimate cyclomatic complexity from a function block
    fn estimate_complexity(block: &syn::Block) -> u32 {
        crate::complexity::cyclomatic::calculate_cyclomatic(block)
    }
}

impl<'ast> Visit<'ast> for ImplVisitor {
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let type_name = Self::extract_type_name(&node.self_ty);
        let trait_name = Self::extract_trait_name(&node.trait_);

        let mut method_names = Vec::new();
        let mut method_complexities = Vec::new();
        let mut total_lines = 0;

        for item in &node.items {
            if let syn::ImplItem::Fn(method) = item {
                method_names.push(method.sig.ident.to_string());
                let complexity = Self::estimate_complexity(&method.block);
                method_complexities.push(complexity);

                // Estimate lines from the block (approximate)
                total_lines += 5; // Base overhead per method
            }
        }

        self.impl_blocks.push(ImplBlockInfo {
            trait_name,
            type_name,
            method_names,
            method_complexities,
            total_lines,
        });

        // Continue visiting nested items
        syn::visit::visit_item_impl(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_method_uniformity_perfect() {
        let impl_blocks = vec![
            ImplBlockInfo {
                trait_name: Some("Flag".to_string()),
                type_name: "Flag1".to_string(),
                method_names: vec!["name_long".to_string(), "is_switch".to_string()],
                method_complexities: vec![1, 1],
                total_lines: 10,
            },
            ImplBlockInfo {
                trait_name: Some("Flag".to_string()),
                type_name: "Flag2".to_string(),
                method_names: vec!["name_long".to_string(), "is_switch".to_string()],
                method_complexities: vec![1, 1],
                total_lines: 10,
            },
            ImplBlockInfo {
                trait_name: Some("Flag".to_string()),
                type_name: "Flag3".to_string(),
                method_names: vec!["name_long".to_string(), "is_switch".to_string()],
                method_complexities: vec![1, 1],
                total_lines: 10,
            },
        ];

        let uniformity = TraitPatternAnalyzer::calculate_method_uniformity(&impl_blocks);
        assert_eq!(uniformity, 1.0); // All 3 impls have the same methods
    }

    #[test]
    fn test_calculate_method_uniformity_partial() {
        let impl_blocks = vec![
            ImplBlockInfo {
                trait_name: Some("Flag".to_string()),
                type_name: "Flag1".to_string(),
                method_names: vec!["name_long".to_string(), "is_switch".to_string()],
                method_complexities: vec![1, 1],
                total_lines: 10,
            },
            ImplBlockInfo {
                trait_name: Some("Flag".to_string()),
                type_name: "Flag2".to_string(),
                method_names: vec!["name_long".to_string(), "is_switch".to_string()],
                method_complexities: vec![1, 1],
                total_lines: 10,
            },
            ImplBlockInfo {
                trait_name: Some("Flag".to_string()),
                type_name: "Flag3".to_string(),
                method_names: vec!["name_long".to_string()], // Missing is_switch
                method_complexities: vec![1],
                total_lines: 5,
            },
        ];

        let uniformity = TraitPatternAnalyzer::calculate_method_uniformity(&impl_blocks);
        // The most common method appears in all 3 impls (name_long and is_switch)
        // So uniformity = 3/3 = 1.0
        assert_eq!(uniformity, 1.0);
    }

    #[test]
    fn test_detect_shared_methods() {
        let impl_blocks = vec![
            ImplBlockInfo {
                trait_name: Some("Flag".to_string()),
                type_name: "Flag1".to_string(),
                method_names: vec![
                    "name_long".to_string(),
                    "is_switch".to_string(),
                    "unique_method".to_string(),
                ],
                method_complexities: vec![1, 1, 2],
                total_lines: 15,
            },
            ImplBlockInfo {
                trait_name: Some("Flag".to_string()),
                type_name: "Flag2".to_string(),
                method_names: vec!["name_long".to_string(), "is_switch".to_string()],
                method_complexities: vec![1, 1],
                total_lines: 10,
            },
        ];

        let shared = TraitPatternAnalyzer::detect_shared_methods(&impl_blocks);

        // name_long and is_switch should appear in both (100%)
        // unique_method appears in 1/2 = 50%, so it should also be included
        assert_eq!(shared.len(), 3);
        assert!(shared
            .iter()
            .any(|(name, freq)| name == "name_long" && *freq == 1.0));
        assert!(shared
            .iter()
            .any(|(name, freq)| name == "is_switch" && *freq == 1.0));
        assert!(shared
            .iter()
            .any(|(name, freq)| name == "unique_method" && *freq == 0.5));
    }

    #[test]
    fn test_calculate_complexity_metrics() {
        let impl_blocks = vec![
            ImplBlockInfo {
                trait_name: Some("Flag".to_string()),
                type_name: "Flag1".to_string(),
                method_names: vec!["method1".to_string(), "method2".to_string()],
                method_complexities: vec![1, 1],
                total_lines: 10,
            },
            ImplBlockInfo {
                trait_name: Some("Flag".to_string()),
                type_name: "Flag2".to_string(),
                method_names: vec!["method1".to_string(), "method2".to_string()],
                method_complexities: vec![1, 2],
                total_lines: 10,
            },
        ];

        let (avg, variance) = TraitPatternAnalyzer::calculate_complexity_metrics(&impl_blocks);
        assert_eq!(avg, 1.25); // (1 + 1 + 1 + 2) / 4
        assert!(variance > 0.0); // Some variance in complexity
    }
}
