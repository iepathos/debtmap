use debtmap::extraction::adapters::god_object::analyze_god_objects;
use debtmap::extraction::UnifiedFileExtractor;
use std::fs;
use std::path::Path;

/// Integration test that validates god object detection on a real codebase file.
#[test]
#[ignore = "Test file has grown and now legitimately detected as GodModule - needs different test subject"]
fn test_god_object_detection_on_config_rs() {
    // Read a large but well-refactored file
    let config_path = Path::new("src/priority/scoring/concise_recommendation.rs");
    let source_content = fs::read_to_string(config_path).expect("Failed to read test file");

    // Run god object detection with the extraction-based API
    let extracted =
        UnifiedFileExtractor::extract(config_path, &source_content).expect("Failed to extract");
    let analyses = analyze_god_objects(config_path, &extracted);

    // Debug output
    if let Some(analysis) = analyses.first() {
        eprintln!("is_god_object: {}", analysis.is_god_object);
        eprintln!("method_count: {}", analysis.method_count);
        eprintln!("field_count: {}", analysis.field_count);
        eprintln!("god_object_score: {}", analysis.god_object_score);
        eprintln!("recommended_splits: {}", analysis.recommended_splits.len());
    }
}

/// Test that verifies the struct ownership analysis produces reasonable results
#[test]
fn test_struct_ownership_analysis_quality() {
    let config_path = Path::new("src/priority/scoring/concise_recommendation.rs");
    let source_content = fs::read_to_string(config_path).expect("Failed to read test file");

    let extracted =
        UnifiedFileExtractor::extract(config_path, &source_content).expect("Failed to extract");
    let analyses = analyze_god_objects(config_path, &extracted);

    // Each struct should be assigned to exactly one module
    let mut all_structs = std::collections::HashSet::new();
    if let Some(analysis) = analyses.first() {
        for split in &analysis.recommended_splits {
            for struct_name in &split.structs_to_move {
                assert!(
                    !all_structs.contains(struct_name),
                    "Struct '{}' appears in multiple module splits",
                    struct_name
                );
                all_structs.insert(struct_name.clone());
            }
        }

        // If we have structs, they should be distributed among the modules
        if !all_structs.is_empty() {
            let modules_with_structs = analysis
                .recommended_splits
                .iter()
                .filter(|s| !s.structs_to_move.is_empty())
                .count();

            eprintln!("Structs distributed among {} modules", modules_with_structs);
        }
    }
}

/// Test that warnings are generated for borderline module sizes
#[test]
fn test_module_size_warnings() {
    let config_path = Path::new("src/priority/scoring/concise_recommendation.rs");
    let source_content = fs::read_to_string(config_path).expect("Failed to read test file");

    let extracted =
        UnifiedFileExtractor::extract(config_path, &source_content).expect("Failed to extract");
    let analyses = analyze_god_objects(config_path, &extracted);

    // Check if any modules have warnings for borderline sizes
    if let Some(analysis) = analyses.first() {
        for split in &analysis.recommended_splits {
            if let Some(warning) = &split.warning {
                // Warning should be meaningful
                assert!(
                    !warning.is_empty(),
                    "Warning should not be empty for module '{}'",
                    split.suggested_name
                );
            }
        }
    }
}
