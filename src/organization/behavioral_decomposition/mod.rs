/// Behavioral decomposition for god object refactoring recommendations.
///
/// This module implements Spec 178: shifting from struct-based organization
/// to behavioral method clustering for god object refactoring.

// Internal modules
mod analysis;
mod categorization;
mod clustering;
mod types;

// Re-export public types
pub use types::{BehaviorCategory, FieldAccessStats, MethodCluster};

// Re-export categorization functions
pub use categorization::{
    cluster_methods_by_behavior, is_test_method, BehavioralCategorizer,
};

// Re-export clustering functions
pub use clustering::{
    apply_community_detection, apply_hybrid_clustering, apply_production_ready_clustering,
    build_method_call_adjacency_matrix, build_method_call_adjacency_matrix_with_functions,
};

// Re-export analysis functions
pub use analysis::{
    detect_service_candidates, recommend_service_extraction, suggest_trait_extraction,
    FieldAccessTracker,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use syn::ItemImpl;

    #[test]
    fn test_categorize_lifecycle_methods() {
        // Per spec 208: "new" is Construction (checked before Lifecycle)
        assert_eq!(
            BehavioralCategorizer::categorize_method("new"),
            BehaviorCategory::Construction
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("initialize_system"),
            BehaviorCategory::Lifecycle
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("cleanup"),
            BehaviorCategory::Lifecycle
        );
    }

    #[test]
    fn test_categorize_parsing_methods() {
        // Per spec 208: Verify parsing methods are correctly categorized
        assert_eq!(
            BehavioralCategorizer::categorize_method("parse_json"),
            BehaviorCategory::Parsing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("read_config"),
            BehaviorCategory::Parsing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("extract_data"),
            BehaviorCategory::Parsing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("decode_message"),
            BehaviorCategory::Parsing
        );
    }

    #[test]
    fn test_categorize_construction_methods() {
        // Per spec 208: Construction methods checked before Lifecycle
        assert_eq!(
            BehavioralCategorizer::categorize_method("create_instance"),
            BehaviorCategory::Construction
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("build_object"),
            BehaviorCategory::Construction
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("make_widget"),
            BehaviorCategory::Construction
        );
    }

    #[test]
    fn test_categorize_filtering_methods() {
        // Per spec 208: Filtering methods correctly identified
        assert_eq!(
            BehavioralCategorizer::categorize_method("filter_results"),
            BehaviorCategory::Filtering
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("select_items"),
            BehaviorCategory::Filtering
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("find_matches"),
            BehaviorCategory::Filtering
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("search_database"),
            BehaviorCategory::Filtering
        );
    }

    #[test]
    fn test_categorize_transformation_methods() {
        // Per spec 208: Transformation methods correctly identified
        assert_eq!(
            BehavioralCategorizer::categorize_method("transform_data"),
            BehaviorCategory::Transformation
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("convert_to_json"),
            BehaviorCategory::Transformation
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("map_values"),
            BehaviorCategory::Transformation
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("apply_transformation"),
            BehaviorCategory::Transformation
        );
    }

    #[test]
    fn test_categorize_data_access_methods() {
        // Per spec 208: DataAccess checked before StateManagement for get_/set_
        assert_eq!(
            BehavioralCategorizer::categorize_method("get_value"),
            BehaviorCategory::DataAccess
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("set_property"),
            BehaviorCategory::DataAccess
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("fetch_record"),
            BehaviorCategory::DataAccess
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("retrieve_data"),
            BehaviorCategory::DataAccess
        );
    }

    #[test]
    fn test_categorize_processing_methods() {
        // Per spec 208: Processing methods correctly identified
        // Note: "handle_" prefix is EventHandling, so use "process", "execute", "run"
        assert_eq!(
            BehavioralCategorizer::categorize_method("process_request"),
            BehaviorCategory::Processing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("process_message"),
            BehaviorCategory::Processing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("execute_task"),
            BehaviorCategory::Processing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("run_pipeline"),
            BehaviorCategory::Processing
        );
    }

    #[test]
    fn test_categorize_communication_methods() {
        // Per spec 208: Communication methods correctly identified
        assert_eq!(
            BehavioralCategorizer::categorize_method("send_message"),
            BehaviorCategory::Communication
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("receive_data"),
            BehaviorCategory::Communication
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("transmit_packet"),
            BehaviorCategory::Communication
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("broadcast_update"),
            BehaviorCategory::Communication
        );
    }

    #[test]
    fn test_categorize_rendering_methods() {
        assert_eq!(
            BehavioralCategorizer::categorize_method("render"),
            BehaviorCategory::Rendering
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("draw_cursor"),
            BehaviorCategory::Rendering
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("paint_background"),
            BehaviorCategory::Rendering
        );
    }

    #[test]
    fn test_categorize_event_handling() {
        assert_eq!(
            BehavioralCategorizer::categorize_method("handle_keypress"),
            BehaviorCategory::EventHandling
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("on_mouse_down"),
            BehaviorCategory::EventHandling
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("dispatch_event"),
            BehaviorCategory::EventHandling
        );
    }

    #[test]
    fn test_categorize_persistence() {
        assert_eq!(
            BehavioralCategorizer::categorize_method("save_state"),
            BehaviorCategory::Persistence
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("load_config"),
            BehaviorCategory::Persistence
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("serialize"),
            BehaviorCategory::Persistence
        );
    }

    #[test]
    fn test_categorize_validation() {
        assert_eq!(
            BehavioralCategorizer::categorize_method("validate_input"),
            BehaviorCategory::Validation
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("check_bounds"),
            BehaviorCategory::Validation
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("verify_signature"),
            BehaviorCategory::Validation
        );
    }

    #[test]
    fn test_categorize_state_management() {
        // Per spec 208: get_/set_ are DataAccess (checked before StateManagement)
        assert_eq!(
            BehavioralCategorizer::categorize_method("get_value"),
            BehaviorCategory::DataAccess
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("set_name"),
            BehaviorCategory::DataAccess
        );
        // update_state contains "_state" so it's still StateManagement
        assert_eq!(
            BehavioralCategorizer::categorize_method("update_state"),
            BehaviorCategory::StateManagement
        );
    }

    #[test]
    fn test_cluster_methods_by_behavior() {
        let methods = vec![
            "render".to_string(),
            "draw_cursor".to_string(),
            "handle_keypress".to_string(),
            "on_mouse_down".to_string(),
            "validate_input".to_string(),
            "get_value".to_string(),
            "set_name".to_string(),
        ];

        let clusters = cluster_methods_by_behavior(&methods);

        assert!(clusters.contains_key(&BehaviorCategory::Rendering));
        assert!(clusters.contains_key(&BehaviorCategory::EventHandling));
        // Per spec 208: get_/set_ are now DataAccess (not StateManagement)
        assert!(clusters.contains_key(&BehaviorCategory::DataAccess));

        assert_eq!(clusters.get(&BehaviorCategory::Rendering).unwrap().len(), 2);
        assert_eq!(
            clusters
                .get(&BehaviorCategory::EventHandling)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            clusters.get(&BehaviorCategory::DataAccess).unwrap().len(),
            2
        );
    }

    #[test]
    fn test_method_cluster_cohesion() {
        let mut cluster = MethodCluster {
            category: BehaviorCategory::Rendering,
            methods: vec!["render".to_string(), "draw".to_string()],
            fields_accessed: vec!["display_map".to_string()],
            internal_calls: 8,
            external_calls: 2,
            cohesion_score: 0.0,
        };

        cluster.calculate_cohesion();
        assert_eq!(cluster.cohesion_score, 0.8); // 8 / (8 + 2)
    }

    #[test]
    fn test_good_extraction_candidate() {
        let cluster = MethodCluster {
            category: BehaviorCategory::Rendering,
            methods: (0..15).map(|i| format!("method{}", i)).collect(),
            fields_accessed: vec!["field1".to_string(), "field2".to_string()],
            internal_calls: 20,
            external_calls: 5,
            cohesion_score: 0.8,
        };

        assert!(cluster.is_good_extraction_candidate(10)); // 2/10 = 0.2 < 0.3
        assert!(!cluster.is_good_extraction_candidate(5)); // 2/5 = 0.4 > 0.3
    }

    #[test]
    fn test_suggest_trait_extraction() {
        let cluster = MethodCluster {
            category: BehaviorCategory::Rendering,
            methods: vec![
                "render".to_string(),
                "draw_cursor".to_string(),
                "paint_background".to_string(),
            ],
            fields_accessed: vec![],
            internal_calls: 0,
            external_calls: 0,
            cohesion_score: 0.0,
        };

        let suggestion = suggest_trait_extraction(&cluster, "Editor");
        assert!(suggestion.contains("trait Renderable"));
        assert!(suggestion.contains("fn render(&self);"));
        assert!(suggestion.contains("3 methods total"));
    }

    #[test]
    fn test_field_access_tracking() {
        let code = quote::quote! {
            impl Editor {
                fn render(&self) {
                    let x = self.display_map;
                    let y = self.cursor_position;
                }

                fn handle_input(&mut self) {
                    self.input_buffer.clear();
                }

                fn save(&self) {
                    let path = self.file_path;
                }
            }
        };

        let impl_block: ItemImpl = syn::parse2(code).unwrap();
        let mut tracker = FieldAccessTracker::new();
        tracker.analyze_impl(&impl_block);

        let render_fields = tracker.get_method_fields("render");
        assert_eq!(render_fields, vec!["cursor_position", "display_map"]);

        let input_fields = tracker.get_method_fields("handle_input");
        assert_eq!(input_fields, vec!["input_buffer"]);

        let save_fields = tracker.get_method_fields("save");
        assert_eq!(save_fields, vec!["file_path"]);
    }

    #[test]
    fn test_minimal_field_set() {
        let code = quote::quote! {
            impl Editor {
                fn render(&self) {
                    let x = self.display_map;
                    let y = self.cursor_position;
                }

                fn draw(&self) {
                    let z = self.display_map;
                }
            }
        };

        let impl_block: ItemImpl = syn::parse2(code).unwrap();
        let mut tracker = FieldAccessTracker::new();
        tracker.analyze_impl(&impl_block);

        let methods = vec!["render".to_string(), "draw".to_string()];
        let minimal_fields = tracker.get_minimal_field_set(&methods);
        assert_eq!(minimal_fields, vec!["cursor_position", "display_map"]);
    }

    #[test]
    fn test_hybrid_clustering_lcov_like_example() {
        // This test mimics the structure of lcov.rs with multiple behavioral categories
        // to ensure hybrid clustering correctly identifies diverse method groups
        let code = quote::quote! {
            pub struct LcovData {
                file_index: HashMap<String, Vec<String>>,
                function_cache: HashMap<String, CoverageData>,
                loc_counter: Option<LocCounter>,
            }

            impl LcovData {
                // Lifecycle methods
                pub fn new() -> Self {
                    Self {
                        file_index: HashMap::new(),
                        function_cache: HashMap::new(),
                        loc_counter: None,
                    }
                }

                pub fn create_empty() -> Self {
                    Self::new()
                }

                pub fn initialize(&mut self) {
                    self.build_index();
                }

                pub fn build_index(&mut self) {
                    // Build index logic
                }

                pub fn with_loc_counter(mut self, counter: LocCounter) -> Self {
                    self.loc_counter = Some(counter);
                    self
                }

                // Query methods - these call each other
                pub fn get_function_coverage(&self, file: &str, function: &str) -> Option<f64> {
                    let funcs = self.find_functions_by_path(file)?;
                    self.find_function_by_name(funcs, function)
                }

                pub fn get_file_coverage(&self, file: &str) -> Option<f64> {
                    let funcs = self.find_functions_by_path(file)?;
                    Some(self.calculate_average(funcs))
                }

                pub fn get_overall_coverage(&self) -> f64 {
                    let all_files = self.get_all_files();
                    self.calculate_weighted_average(&all_files)
                }

                pub fn batch_get_function_coverage(&self, queries: Vec<Query>) -> Vec<f64> {
                    queries.iter().map(|q| {
                        self.get_function_coverage(&q.file, &q.func).unwrap_or(0.0)
                    }).collect()
                }

                // Path matching methods - these call each other
                fn find_functions_by_path(&self, path: &str) -> Option<Vec<String>> {
                    if self.matches_suffix_strategy(path) {
                        Some(vec![])
                    } else {
                        self.apply_strategies_parallel(path)
                    }
                }

                fn matches_suffix_strategy(&self, path: &str) -> bool {
                    let normalized = normalize_path(path);
                    self.file_index.contains_key(&normalized)
                }

                fn apply_strategies_parallel(&self, path: &str) -> Option<Vec<String>> {
                    let results = self.apply_strategies_sequential(path);
                    results
                }

                fn apply_strategies_sequential(&self, path: &str) -> Option<Vec<String>> {
                    if self.matches_reverse_suffix(path) {
                        Some(vec![])
                    } else {
                        None
                    }
                }

                fn matches_reverse_suffix(&self, path: &str) -> bool {
                    false
                }

                // Helper methods for queries
                fn find_function_by_name(&self, funcs: Vec<String>, name: &str) -> Option<f64> {
                    let normalized = normalize_function_name(name);
                    Some(0.5)
                }

                fn calculate_average(&self, funcs: Vec<String>) -> f64 {
                    0.75
                }

                fn calculate_weighted_average(&self, files: &[String]) -> f64 {
                    0.85
                }

                fn get_all_files(&self) -> Vec<String> {
                    vec![]
                }
            }

            // Standalone normalization functions - should be tracked too
            fn normalize_path(path: &str) -> String {
                demangle_path_components(path)
            }

            fn demangle_path_components(path: &str) -> String {
                path.to_lowercase()
            }

            fn normalize_function_name(name: &str) -> String {
                demangle_function_name(name)
            }

            fn demangle_function_name(name: &str) -> String {
                strip_trailing_generics(name)
            }

            fn strip_trailing_generics(name: &str) -> String {
                name.trim_end_matches(">").to_string()
            }

            // Parsing functions
            pub fn parse_lcov_file(path: &str) -> Result<LcovData, String> {
                parse_lcov_file_with_progress(path, &ProgressBar::new())
            }

            pub fn parse_lcov_file_with_progress(path: &str, progress: &ProgressBar) -> Result<LcovData, String> {
                let data = parse_coverage_data(path)?;
                calculate_function_coverage_data(data)
            }

            fn parse_coverage_data(path: &str) -> Result<Vec<String>, String> {
                Ok(vec![])
            }

            fn process_function_coverage_parallel(path: &str) -> Result<Vec<String>, String> {
                Ok(vec![])
            }

            fn calculate_function_coverage_data(data: Vec<String>) -> Result<LcovData, String> {
                Ok(LcovData::new())
            }
        };

        let ast: syn::File = syn::parse2(code).unwrap();

        // Collect impl blocks
        let impl_blocks: Vec<&syn::ItemImpl> = ast
            .items
            .iter()
            .filter_map(|item| {
                if let syn::Item::Impl(impl_block) = item {
                    Some(impl_block)
                } else {
                    None
                }
            })
            .collect();

        // Collect standalone functions
        let standalone_functions: Vec<&syn::ItemFn> = ast
            .items
            .iter()
            .filter_map(|item| {
                if let syn::Item::Fn(func) = item {
                    Some(func)
                } else {
                    None
                }
            })
            .collect();

        // Collect all method names
        let mut all_methods = Vec::new();
        for impl_block in &impl_blocks {
            for item in &impl_block.items {
                if let syn::ImplItem::Fn(method) = item {
                    all_methods.push(method.sig.ident.to_string());
                }
            }
        }
        for func in &standalone_functions {
            all_methods.push(func.sig.ident.to_string());
        }

        // Build adjacency matrix with standalone function support
        let adjacency =
            build_method_call_adjacency_matrix_with_functions(&impl_blocks, &standalone_functions);

        // Apply hybrid clustering
        let clusters = apply_hybrid_clustering(&all_methods, &adjacency);

        // Verify we found multiple clusters (not just one big cluster)
        assert!(
            clusters.len() >= 3,
            "Expected at least 3 behavioral clusters, but found {}. Clusters: {:?}",
            clusters.len(),
            clusters
                .iter()
                .map(|c| (c.category.display_name(), c.methods.len()))
                .collect::<Vec<_>>()
        );

        // Verify that we have different behavioral categories
        let categories: HashSet<String> =
            clusters.iter().map(|c| c.category.display_name()).collect();

        assert!(
            categories.len() >= 2,
            "Expected diverse behavioral categories, but found only: {:?}",
            categories
        );

        // Per spec 208: Check that we have a Construction cluster (new, create_empty, build_*)
        let construction_cluster = clusters
            .iter()
            .find(|c| matches!(c.category, BehaviorCategory::Construction));
        assert!(
            construction_cluster.is_some(),
            "Expected to find Construction cluster for methods like 'new', 'create_empty', 'build_index'"
        );

        // Per spec 208: Check that we have a DataAccess cluster (get_* methods)
        let data_access_cluster = clusters
            .iter()
            .find(|c| matches!(c.category, BehaviorCategory::DataAccess));
        assert!(
            data_access_cluster.is_some(),
            "Expected to find DataAccess cluster for get_* methods"
        );

        // Per spec 208: Verify that Parsing cluster exists (parse_* methods checked before Persistence)
        let parsing_cluster = clusters
            .iter()
            .find(|c| matches!(c.category, BehaviorCategory::Parsing));
        assert!(
            parsing_cluster.is_some(),
            "Expected to find Parsing cluster for parse_* methods"
        );

        // Per spec 208: Precedence rules (Construction before Lifecycle, DataAccess before StateManagement,
        // Parsing before Persistence) may result in clusters of varying sizes, including single-method clusters.
        // The important verification is diversity of categories (above), not minimum cluster sizes.

        // Verify that standalone function calls were tracked
        // normalize_path calls demangle_path_components, so they should be in same cluster
        let normalize_cluster = clusters
            .iter()
            .find(|c| c.methods.contains(&"normalize_path".to_string()));

        if let Some(cluster) = normalize_cluster {
            // If normalize_path is in a cluster, demangle_path_components should be too
            // (they're related by call graph)
            let has_related_demangle = cluster.methods.iter().any(|m| m.contains("demangle"));
            assert!(
                has_related_demangle || cluster.methods.len() >= 3,
                "Expected normalize functions to be clustered together or in a reasonable cluster"
            );
        }

        println!("\n=== Hybrid Clustering Results ===");
        for (i, cluster) in clusters.iter().enumerate() {
            println!(
                "Cluster {}: {} ({} methods, cohesion: {:.2})",
                i + 1,
                cluster.category.display_name(),
                cluster.methods.len(),
                cluster.cohesion_score
            );
            println!("  Methods: {:?}", cluster.methods);
        }
        println!("=================================\n");
    }

    #[test]
    fn test_production_ready_clustering_filters_tests() {
        // This test verifies that production-ready clustering:
        // 1. Filters out test methods
        // 2. Subdivides oversized Domain clusters
        // 3. Merges tiny clusters
        // 4. Applies Rust-specific patterns

        let methods = vec![
            // Production methods - Parser group
            "parse_lcov_file".to_string(),
            "parse_lcov_file_with_progress".to_string(),
            "parse_coverage_data".to_string(),
            "read_file_contents".to_string(),
            // Production methods - Query group
            "get_function_coverage".to_string(),
            "get_file_coverage".to_string(),
            "get_overall_coverage".to_string(),
            "get_all_files".to_string(),
            "fetch_coverage_data".to_string(),
            // Production methods - Normalize group
            "normalize_path".to_string(),
            "normalize_function_name".to_string(),
            "normalize_demangled_name".to_string(),
            // Production methods - Find group
            "find_function_by_name".to_string(),
            "find_functions_by_path".to_string(),
            "find_function_by_line".to_string(),
            // Test methods - should be filtered out
            "test_parse_lcov_file".to_string(),
            "test_function_name_normalization".to_string(),
            "test_coverage_calculation".to_string(),
            "test_empty_data".to_string(),
            // Test helpers - should be filtered out
            "mock_coverage_data".to_string(),
            "fixture_test_file".to_string(),
        ];

        let adjacency = HashMap::new(); // Empty adjacency for simplicity

        // Apply production-ready clustering
        let clusters = apply_production_ready_clustering(&methods, &adjacency);

        // Verify tests are filtered out
        let all_cluster_methods: Vec<&String> = clusters.iter().flat_map(|c| &c.methods).collect();

        assert!(
            !all_cluster_methods.contains(&&"test_parse_lcov_file".to_string()),
            "Test methods should be filtered out"
        );
        assert!(
            !all_cluster_methods.contains(&&"mock_coverage_data".to_string()),
            "Test helper methods should be filtered out"
        );

        // Verify production methods are included
        assert!(
            all_cluster_methods.contains(&&"parse_lcov_file".to_string()),
            "Production methods should be included"
        );
        assert!(
            all_cluster_methods.contains(&&"get_function_coverage".to_string()),
            "Production methods should be included"
        );

        // Verify we have multiple clusters (not one big cluster)
        assert!(
            clusters.len() >= 3,
            "Should have multiple clusters, found {}",
            clusters.len()
        );

        // Verify proper categorization (either behavioral or Rust-specific patterns)
        // Clusters should be well-categorized, not just generic "Utilities"
        let has_good_categories = clusters.iter().all(|c| {
            !matches!(
                c.category,
                BehaviorCategory::Domain(ref name) if name == "Utilities" || name == "Operations"
            )
        });

        assert!(
            has_good_categories,
            "All clusters should have meaningful categories (not Utilities/Operations)"
        );

        println!("\n=== Production-Ready Clustering Results ===");
        println!("Total clusters: {}", clusters.len());
        println!(
            "Production methods: {} / {} total",
            all_cluster_methods.len(),
            methods.len()
        );
        for (i, cluster) in clusters.iter().enumerate() {
            println!(
                "Cluster {}: {} ({} methods)",
                i + 1,
                cluster.category.display_name(),
                cluster.methods.len()
            );
            println!("  Methods: {:?}", cluster.methods);
        }
        println!("==========================================\n");
    }

    #[test]
    fn test_no_method_loss_and_minimum_cluster_size() {
        // Phase 1 requirements test:
        // 1. All methods must be accounted for (no losses)
        // 2. No clusters smaller than 3 methods
        // 3. Low-cohesion methods kept in behavioral categories

        let methods = vec![
            // Rendering group (high cohesion)
            "render_text".to_string(),
            "render_cursor".to_string(),
            "paint_highlights".to_string(),
            "draw_gutter".to_string(),
            // Utilities (low cohesion, no internal calls)
            "format_timestamp".to_string(),
            "clamp_value".to_string(),
            // Single method categories
            "validate_config".to_string(),
            // State management
            "get_state".to_string(),
            "set_state".to_string(),
        ];

        let adjacency = HashMap::from([
            // Rendering cluster has internal calls
            (
                ("render_text".to_string(), "paint_highlights".to_string()),
                1,
            ),
            (("render_cursor".to_string(), "draw_gutter".to_string()), 1),
            // Utilities have zero internal calls (low cohesion)
            // Validation has no calls (isolated)
            // State methods call each other
            (("set_state".to_string(), "get_state".to_string()), 1),
        ]);

        let clusters = apply_production_ready_clustering(&methods, &adjacency);

        // REQUIREMENT 1: All methods must be accounted for
        let clustered_methods: std::collections::HashSet<String> =
            clusters.iter().flat_map(|c| &c.methods).cloned().collect();

        for method in &methods {
            assert!(
                clustered_methods.contains(method),
                "Method '{}' was lost during clustering!",
                method
            );
        }

        assert_eq!(
            clustered_methods.len(),
            methods.len(),
            "Total methods mismatch: {} clustered vs {} input",
            clustered_methods.len(),
            methods.len()
        );

        // REQUIREMENT 2: No clusters smaller than 3 methods
        for cluster in &clusters {
            assert!(
                cluster.methods.len() >= 3,
                "Cluster '{}' has only {} methods (minimum is 3)",
                cluster.category.display_name(),
                cluster.methods.len()
            );
        }

        // REQUIREMENT 3: Low-cohesion methods kept (not filtered out)
        // format_timestamp and clamp_value have zero cohesion but should be in a cluster
        assert!(
            clustered_methods.contains("format_timestamp"),
            "Low-cohesion method 'format_timestamp' should be kept"
        );
        assert!(
            clustered_methods.contains("clamp_value"),
            "Low-cohesion method 'clamp_value' should be kept"
        );

        println!("\n=== No Method Loss Test Results ===");
        println!("Total input methods: {}", methods.len());
        println!("Total clustered methods: {}", clustered_methods.len());
        println!("Clusters created: {}", clusters.len());
        for (i, cluster) in clusters.iter().enumerate() {
            println!(
                "Cluster {}: {} ({} methods, cohesion: {:.2})",
                i + 1,
                cluster.category.display_name(),
                cluster.methods.len(),
                cluster.cohesion_score
            );
            println!("  Methods: {:?}", cluster.methods);
        }
        println!("=====================================\n");
    }

    // Unit tests for predicate functions (Spec 208 requirement)

    #[test]
    fn test_is_parsing_predicate() {
        // Per spec 208: Test is_parsing predicate function
        assert!(BehavioralCategorizer::is_parsing("parse_json"));
        assert!(BehavioralCategorizer::is_parsing("read_file"));
        assert!(BehavioralCategorizer::is_parsing("extract_data"));
        assert!(BehavioralCategorizer::is_parsing("decode_base64"));
        assert!(BehavioralCategorizer::is_parsing("deserialize_xml"));
        assert!(BehavioralCategorizer::is_parsing("unmarshal_proto"));
        assert!(BehavioralCategorizer::is_parsing("scan_tokens"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_parsing("render_view"));
        assert!(!BehavioralCategorizer::is_parsing("calculate_sum"));
        assert!(!BehavioralCategorizer::is_parsing("validate_input"));
    }

    #[test]
    fn test_is_rendering_predicate() {
        // Per spec 208: Test is_rendering predicate function
        assert!(BehavioralCategorizer::is_rendering("render_template"));
        assert!(BehavioralCategorizer::is_rendering("draw_rectangle"));
        assert!(BehavioralCategorizer::is_rendering("paint_canvas"));
        assert!(BehavioralCategorizer::is_rendering("display_message"));
        assert!(BehavioralCategorizer::is_rendering("show_dialog"));
        assert!(BehavioralCategorizer::is_rendering("present_view"));
        assert!(BehavioralCategorizer::is_rendering("format_output"));
        assert!(BehavioralCategorizer::is_rendering("to_string"));
        assert!(BehavioralCategorizer::is_rendering("print_report"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_rendering("parse_json"));
        assert!(!BehavioralCategorizer::is_rendering("calculate_sum"));
        assert!(!BehavioralCategorizer::is_rendering("validate_input"));
    }

    #[test]
    fn test_is_filtering_predicate() {
        // Per spec 208: Test is_filtering predicate function
        assert!(BehavioralCategorizer::is_filtering("filter_results"));
        assert!(BehavioralCategorizer::is_filtering("select_items"));
        assert!(BehavioralCategorizer::is_filtering("find_matches"));
        assert!(BehavioralCategorizer::is_filtering("search_database"));
        assert!(BehavioralCategorizer::is_filtering("query_records"));
        assert!(BehavioralCategorizer::is_filtering("lookup_value"));
        assert!(BehavioralCategorizer::is_filtering("match_pattern"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_filtering("parse_json"));
        assert!(!BehavioralCategorizer::is_filtering("render_view"));
        assert!(!BehavioralCategorizer::is_filtering("calculate_sum"));
    }

    #[test]
    fn test_is_transformation_predicate() {
        // Per spec 208: Test is_transformation predicate function
        assert!(BehavioralCategorizer::is_transformation("transform_data"));
        assert!(BehavioralCategorizer::is_transformation("convert_format"));
        assert!(BehavioralCategorizer::is_transformation("map_values"));
        assert!(BehavioralCategorizer::is_transformation("apply_rules"));
        assert!(BehavioralCategorizer::is_transformation("adapt_schema"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_transformation("parse_json"));
        assert!(!BehavioralCategorizer::is_transformation("filter_results"));
        assert!(!BehavioralCategorizer::is_transformation("validate_input"));
    }

    #[test]
    fn test_is_construction_predicate() {
        // Per spec 208: Test is_construction predicate function (checked before Lifecycle)
        assert!(BehavioralCategorizer::is_construction("create_instance"));
        assert!(BehavioralCategorizer::is_construction("build_object"));
        assert!(BehavioralCategorizer::is_construction("new_connection"));
        assert!(BehavioralCategorizer::is_construction("make_widget"));
        assert!(BehavioralCategorizer::is_construction("construct_tree"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_construction("parse_json"));
        assert!(!BehavioralCategorizer::is_construction("render_view"));
        assert!(!BehavioralCategorizer::is_construction("validate_input"));
    }

    #[test]
    fn test_is_data_access_predicate() {
        // Per spec 208: Test is_data_access predicate function (checked before StateManagement)
        assert!(BehavioralCategorizer::is_data_access("get_value"));
        assert!(BehavioralCategorizer::is_data_access("set_property"));
        assert!(BehavioralCategorizer::is_data_access("fetch_record"));
        assert!(BehavioralCategorizer::is_data_access("retrieve_data"));
        assert!(BehavioralCategorizer::is_data_access("access_field"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_data_access("parse_json"));
        assert!(!BehavioralCategorizer::is_data_access("render_view"));
        assert!(!BehavioralCategorizer::is_data_access("validate_input"));
    }

    #[test]
    fn test_is_communication_predicate() {
        // Per spec 208: Test is_communication predicate function
        assert!(BehavioralCategorizer::is_communication("send_message"));
        assert!(BehavioralCategorizer::is_communication("receive_data"));
        assert!(BehavioralCategorizer::is_communication("transmit_packet"));
        assert!(BehavioralCategorizer::is_communication("broadcast_event"));
        assert!(BehavioralCategorizer::is_communication("notify_observers"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_communication("parse_json"));
        assert!(!BehavioralCategorizer::is_communication("render_view"));
        assert!(!BehavioralCategorizer::is_communication("validate_input"));
    }

    #[test]
    fn test_no_duplicate_responsibilities_integration() {
        // Per spec 208: Integration test to verify no duplicate responsibilities
        // with different capitalizations. This was a key objective of the spec.

        let methods = vec![
            // Rendering methods (should all map to "Rendering", not "output" or "rendering")
            "format_output".to_string(),
            "format_json".to_string(),
            "render_view".to_string(),
            "draw_chart".to_string(),
            // Parsing methods (should all map to "Parsing", not "parsing" or "PARSING")
            "parse_json".to_string(),
            "parse_xml".to_string(),
            "read_config".to_string(),
            // DataAccess methods (should all map to "Data Access", not "data_access" or "DataAccess")
            "get_value".to_string(),
            "set_property".to_string(),
            "fetch_record".to_string(),
            // Validation methods (should all map to "Validation", not "validation")
            "validate_input".to_string(),
            "check_bounds".to_string(),
            "is_valid".to_string(),
        ];

        let clusters = cluster_methods_by_behavior(&methods);

        // Collect all category display names (which should be Title Case)
        let category_names: Vec<String> = clusters.keys().map(|cat| cat.display_name()).collect();

        // Check for duplicates (case-insensitive comparison)
        let mut seen_lower = std::collections::HashSet::new();
        let mut duplicates = Vec::new();

        for name in &category_names {
            let lower = name.to_lowercase();
            if seen_lower.contains(&lower) {
                duplicates.push(name.clone());
            }
            seen_lower.insert(lower);
        }

        assert!(
            duplicates.is_empty(),
            "Found duplicate responsibilities with different capitalizations: {:?}\nAll categories: {:?}",
            duplicates,
            category_names
        );

        // Verify that all category names use consistent Title Case
        for name in &category_names {
            // Title Case means first letter uppercase, rest depend on context
            // For single-word categories: "Rendering", "Parsing", "Validation"
            // For multi-word: "Data Access", "State Management"
            let first_char = name.chars().next().unwrap();
            assert!(
                first_char.is_uppercase(),
                "Category '{}' should start with uppercase letter (Title Case)",
                name
            );
        }

        // Verify expected categories are present with correct casing
        let has_rendering = category_names.iter().any(|n| n == "Rendering");
        let has_parsing = category_names.iter().any(|n| n == "Parsing");
        let has_data_access = category_names.iter().any(|n| n == "Data Access");
        let has_validation = category_names.iter().any(|n| n == "Validation");

        assert!(has_rendering, "Expected 'Rendering' category (Title Case)");
        assert!(has_parsing, "Expected 'Parsing' category (Title Case)");
        assert!(
            has_data_access,
            "Expected 'Data Access' category (Title Case)"
        );
        assert!(
            has_validation,
            "Expected 'Validation' category (Title Case)"
        );

        // Verify NO lowercase versions exist
        assert!(
            !category_names.iter().any(|n| n == "output"),
            "Should not have lowercase 'output' category"
        );
        assert!(
            !category_names.iter().any(|n| n == "parsing"),
            "Should not have lowercase 'parsing' category"
        );
        assert!(
            !category_names.iter().any(|n| n == "data_access"),
            "Should not have snake_case 'data_access' category"
        );
        assert!(
            !category_names.iter().any(|n| n == "validation"),
            "Should not have lowercase 'validation' category"
        );
    }
}
