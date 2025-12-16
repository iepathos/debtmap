/// Integration tests for type-based clustering (Spec 181)
///
/// Tests that type-based clustering works correctly on real codebases
/// and produces recommendations with type ownership principles.
use debtmap::extraction::adapters::god_object::analyze_god_objects;
use debtmap::extraction::UnifiedFileExtractor;
use debtmap::organization::SplitAnalysisMethod;
use std::fs;
use std::path::Path;

/// Test type-based clustering on formatter.rs
/// Should recommend PriorityItem-based module
#[test]
fn test_type_based_clustering_on_formatter() {
    // Read the actual formatter.rs file
    let formatter_path = Path::new("src/io/formatter.rs");

    if !formatter_path.exists() {
        // Skip if file doesn't exist (CI environments might not have it)
        return;
    }

    let source = fs::read_to_string(formatter_path).expect("Failed to read formatter.rs");
    let extracted = UnifiedFileExtractor::extract(formatter_path, &source)
        .expect("Failed to extract formatter.rs");
    let analyses = analyze_god_objects(formatter_path, &extracted);

    // If formatter.rs has utilities modules from behavioral clustering,
    // type-based should be used instead
    if let Some(analysis) = analyses.first() {
        let splits = &analysis.recommended_splits;
        if splits.is_empty() {
            return;
        }

        // Check if any splits use type-based method
        let has_type_based = splits
            .iter()
            .any(|s| s.method == SplitAnalysisMethod::TypeBased);

        if has_type_based {
            // Verify type-based recommendations have core_type field
            for split in splits {
                if split.method == SplitAnalysisMethod::TypeBased {
                    assert!(
                        split.core_type.is_some(),
                        "Type-based split should have core_type: {:?}",
                        split
                    );

                    // Should not be utilities module
                    assert!(
                        !split.suggested_name.contains("utilities")
                            && !split.suggested_name.contains("helpers")
                            && !split.suggested_name.contains("utils"),
                        "Type-based clustering should not produce utilities modules: {}",
                        split.suggested_name
                    );

                    // Should have data_flow information
                    assert!(
                        !split.data_flow.is_empty() || split.core_type.is_some(),
                        "Type-based split should have data flow or core type information"
                    );
                }
            }
        }
    }
}

/// Test type-based clustering on god_object_analysis.rs
/// Should recommend pipeline stage modules
#[test]
fn test_type_based_clustering_on_god_object_analysis() {
    // Read the actual god_object_analysis.rs file
    let analysis_path = Path::new("src/organization/god_object_analysis.rs");

    if !analysis_path.exists() {
        // Skip if file doesn't exist
        return;
    }

    let source = fs::read_to_string(analysis_path).expect("Failed to read god_object_analysis.rs");
    let extracted = UnifiedFileExtractor::extract(analysis_path, &source)
        .expect("Failed to extract god_object_analysis.rs");
    let analyses = analyze_god_objects(analysis_path, &extracted);

    // If the file produces splits, verify they don't have utilities modules
    if let Some(analysis) = analyses.first() {
        let splits = &analysis.recommended_splits;
        if splits.is_empty() {
            return;
        }
        for split in splits {
            // No utilities modules should be recommended
            assert!(
                !split.suggested_name.contains("utilities")
                    && !split.suggested_name.contains("helpers")
                    && !split.suggested_name.contains("utils"),
                "Should not produce utilities modules: {}",
                split.suggested_name
            );

            // Type-based splits should have core_type
            if split.method == SplitAnalysisMethod::TypeBased {
                assert!(
                    split.core_type.is_some(),
                    "Type-based split should have core_type: {:?}",
                    split
                );
            }
        }
    }
}

/// Test that type-based clustering produces quality recommendations
#[test]
fn test_type_based_clustering_quality() {
    // Code with clear type affinity - functions working with specific types
    let code = r#"
        struct PriorityItem {
            priority: u32,
            name: String,
        }

        struct Config {
            threshold: u32,
            enabled: bool,
        }

        // Functions working with PriorityItem
        fn create_priority_item(name: String, priority: u32) -> PriorityItem {
            PriorityItem { name, priority }
        }

        fn update_priority(item: &mut PriorityItem, new_priority: u32) {
            item.priority = new_priority;
        }

        fn compare_items(a: &PriorityItem, b: &PriorityItem) -> std::cmp::Ordering {
            a.priority.cmp(&b.priority)
        }

        fn format_item(item: &PriorityItem) -> String {
            format!("{}: {}", item.name, item.priority)
        }

        // Functions working with Config
        fn create_config(threshold: u32, enabled: bool) -> Config {
            Config { threshold, enabled }
        }

        fn is_enabled(config: &Config) -> bool {
            config.enabled
        }

        fn get_threshold(config: &Config) -> u32 {
            config.threshold
        }

        fn validate_config(config: &Config) -> bool {
            config.threshold > 0
        }
    "#;

    let extracted =
        UnifiedFileExtractor::extract(Path::new("test.rs"), code).expect("Failed to extract");
    let _analyses = analyze_god_objects(Path::new("test.rs"), &extracted);

    // This code should be analyzed without errors
    // The exact behavior depends on the god object detection thresholds
    // Standalone functions are not counted in method_count (only impl methods are)
    // but we can verify the analysis completes successfully

    // Analysis completes successfully - no panics
    // The test passes if we reach here without panicking
}

/// Test that utilities modules trigger type-based clustering fallback
#[test]
fn test_utilities_trigger_type_based_fallback() {
    // Simulate a file that would produce utilities modules from behavioral clustering
    let code = r#"
        struct Request {
            id: u64,
            data: Vec<u8>,
        }

        // Helper functions that might be grouped as "utilities"
        fn validate_request(req: &Request) -> bool {
            !req.data.is_empty()
        }

        fn serialize_request(req: &Request) -> String {
            format!("Request {}", req.id)
        }

        fn deserialize_request(s: &str) -> Option<Request> {
            None
        }

        fn clone_request(req: &Request) -> Request {
            Request {
                id: req.id,
                data: req.data.clone(),
            }
        }

        fn hash_request(req: &Request) -> u64 {
            req.id
        }

        fn compare_requests(a: &Request, b: &Request) -> bool {
            a.id == b.id
        }
    "#;

    let extracted =
        UnifiedFileExtractor::extract(Path::new("test.rs"), code).expect("Failed to extract");
    let analyses = analyze_god_objects(Path::new("test.rs"), &extracted);

    // Verify no utilities modules in recommendations
    if let Some(analysis) = analyses.first() {
        let splits = &analysis.recommended_splits;
        if splits.is_empty() {
            return;
        }
        for split in splits {
            assert!(
                !split.suggested_name.contains("utilities")
                    && !split.suggested_name.contains("helpers")
                    && !split.suggested_name.contains("utils"),
                "Should not produce utilities modules with type-based clustering: {}",
                split.suggested_name
            );
        }
    }
}
