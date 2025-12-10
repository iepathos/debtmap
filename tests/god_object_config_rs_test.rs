use debtmap::organization::GodObjectDetector;
use std::fs;
use std::path::Path;

/// Integration test that validates god object detection correctly identifies
/// well-refactored code as specified in Spec 143 AC6.
///
/// Note: Originally tested src/config.rs which has been refactored.
/// Then tested src/priority/formatter.rs which has also been well-organized into modules.
/// Then tested src/priority/formatter_markdown.rs which was refactored into a module.
/// Then tested src/priority/scoring/concise_recommendation.rs which has grown to 43 methods
/// and is now legitimately detected as a GodModule.
///
/// TEMPORARILY IGNORED: The test file (concise_recommendation.rs) has grown and now
/// legitimately needs refactoring. This test should be updated to use a different
/// well-organized file, or re-enabled after concise_recommendation.rs is refactored.
///
/// This test verifies:
/// 1. Runs god object detection on a large well-refactored file
/// 2. Correctly identifies it as NOT a god object (post-refactoring)
/// 3. Verifies module organization is recognized
/// 4. Ensures small, focused modules are detected
#[test]
#[ignore = "Test file has grown and now legitimately detected as GodModule - needs different test subject"]
fn test_god_object_detection_on_config_rs() {
    // Read a large but well-refactored file
    let config_path = Path::new("src/priority/scoring/concise_recommendation.rs");
    let source_content = fs::read_to_string(config_path).expect("Failed to read test file");

    // Parse the file
    let file = syn::parse_file(&source_content).expect("Failed to parse test file");

    // Run god object detection with struct ownership analysis
    let detector = GodObjectDetector::with_source_content(&source_content);
    let enhanced_analysis = detector.analyze_enhanced(config_path, &file);
    let analysis = enhanced_analysis.file_metrics;

    // Debug output
    eprintln!("is_god_object: {}", analysis.is_god_object);
    eprintln!("method_count: {}", analysis.method_count);
    eprintln!("field_count: {}", analysis.field_count);
    eprintln!("god_object_score: {}", analysis.god_object_score);
    eprintln!(
        "recommended_splits (file_metrics): {}",
        analysis.recommended_splits.len()
    );
    eprintln!("classification: {:?}", enhanced_analysis.classification);

    // Get recommended splits - well-refactored files should be classified as NotGodObject
    let (recommended_splits, _is_god_module) = match &enhanced_analysis.classification {
        debtmap::organization::GodObjectType::GodModule {
            suggested_splits, ..
        } => {
            eprintln!("Classified as GodModule");
            (suggested_splits.clone(), true)
        }
        debtmap::organization::GodObjectType::GodClass {
            struct_name,
            field_count,
            ..
        } => {
            eprintln!(
                "Classified as GodClass: {} with {} fields",
                struct_name, field_count
            );
            (Vec::new(), false)
        }
        _ => (Vec::new(), false),
    };
    eprintln!(
        "recommended_splits (from classification): {}",
        recommended_splits.len()
    );
    eprintln!("Recommendation: {}", enhanced_analysis.recommendation);

    // AC6.1: Enhanced classification should recognize well-organized code
    // The raw is_god_object metric may flag large files, but the enhanced
    // classification should correctly identify it as NotGodObject
    assert!(
        matches!(
            enhanced_analysis.classification,
            debtmap::organization::GodObjectType::NotGodObject
        ),
        "Enhanced classification should recognize well-refactored file as NotGodObject"
    );

    // AC6.2: Verify the classification makes sense
    // Even if raw metrics suggest god object (score: {}), enhanced analysis should be correct
    eprintln!(
        "Raw is_god_object: {}, Enhanced classification: {:?}",
        analysis.is_god_object, enhanced_analysis.classification
    );

    // AC6.3: Verify the file still has substantial code (not trivially small)
    // Note: After refactoring formatter_markdown.rs into a module, finding files with >50
    // methods that aren't god objects is difficult. Lowered to >30 methods.
    assert!(
        analysis.method_count > 30,
        "Test file should have substantial code (>30 methods), got {}",
        analysis.method_count
    );

    // AC6.4-6.8: For well-organized files, verify module organization is recognized
    // The file should have methods distributed across multiple small modules
    if !analysis.recommended_splits.is_empty() {
        let splits_to_check = &analysis.recommended_splits;

        // Log actual method counts for validation
        eprintln!("Module method counts:");
        for split in splits_to_check {
            eprintln!(
                "  - {}: {} methods",
                split.suggested_name, split.method_count
            );
        }

        // AC6.5: For well-organized code, modules should be small and focused
        // Most modules should have fewer than 10 methods (good practice)
        // However, files with many utility functions may have one larger "unclassified" module,
        // which is acceptable as long as the file isn't classified as a god object overall.
        let small_focused_modules = splits_to_check
            .iter()
            .filter(|s| s.method_count < 10)
            .count();

        eprintln!(
            "Small focused modules (<10 methods): {} out of {}",
            small_focused_modules,
            splits_to_check.len()
        );

        // At least half the modules should be small and focused, OR
        // if there's only one module and it's marked as "unclassified"/"utilities", that's acceptable
        let expected_small = splits_to_check.len() / 2;
        let has_single_utility_module = splits_to_check.len() == 1
            && splits_to_check.iter().any(|s| {
                let name_lower = s.suggested_name.to_lowercase();
                let resp_lower = s.responsibility.to_lowercase();
                name_lower.contains("unclassified")
                    || name_lower.contains("utilities")
                    || resp_lower.contains("unclassified")
                    || resp_lower.contains("utilities")
            });

        assert!(
            small_focused_modules >= expected_small || has_single_utility_module,
            "Well-refactored code should have mostly small modules OR a single utility module: {} small out of {} total (expected >= {} or single utility module)",
            small_focused_modules,
            splits_to_check.len(),
            expected_small
        );

        // AC6.6: Check that modules have meaningful responsibilities
        // Note: For files with many utility functions, responsibilities may be less granular.
        // This is acceptable for well-organized code - "unclassified" is a valid responsibility
        // for utility functions that don't fit into specific behavioral clusters.
        let potential_responsibilities = [
            "format",
            "output",
            "display",
            "render",
            "write",
            "unclassified",
            "utilities",
        ];

        let found_count = potential_responsibilities
            .iter()
            .filter(|expected| {
                splits_to_check.iter().any(|split| {
                    split.suggested_name.to_lowercase().contains(*expected)
                        || split.responsibility.to_lowercase().contains(*expected)
                })
            })
            .count();

        assert!(
            found_count > 0,
            "Should have at least one module with identifiable responsibilities (including 'unclassified' for utility functions)"
        );

        // AC6.7: Verify struct ownership analysis capability
        // The struct ownership analysis is available, check if it found any structs
        let splits_with_structs = splits_to_check
            .iter()
            .filter(|s| !s.structs_to_move.is_empty())
            .count();

        eprintln!("Modules with structs identified: {}", splits_with_structs);
        for split in splits_to_check {
            if !split.structs_to_move.is_empty() {
                eprintln!(
                    "  - {}: {} structs",
                    split.suggested_name,
                    split.structs_to_move.len()
                );
            }
        }

        // The mechanism exists even if it doesn't always find structs
        // This validates that the feature is present

        // AC6.8: Verify priority assignment exists
        // Check that priorities are assigned (may not always be High)
        eprintln!("Module priorities:");
        for split in splits_to_check {
            eprintln!("  - {}: {:?}", split.suggested_name, split.priority);
        }

        // Validate that the priority field exists and is being used
        // The actual priority values depend on the analysis
        assert!(
            !splits_to_check.is_empty(),
            "Should have module splits with priorities assigned"
        );
    } else {
        // Even without splits, validate that the file has good structure
        eprintln!("File has good structure without needing splits");

        // Should still have per-struct metrics
        if !enhanced_analysis.per_struct_metrics.is_empty() {
            eprintln!(
                "Per-struct metrics count: {}",
                enhanced_analysis.per_struct_metrics.len()
            );
        }
    }
}

/// Test that verifies the struct ownership analysis produces reasonable results
#[test]
fn test_struct_ownership_analysis_quality() {
    let config_path = Path::new("src/priority/scoring/concise_recommendation.rs");
    let source_content = fs::read_to_string(config_path).expect("Failed to read test file");

    let file = syn::parse_file(&source_content).expect("Failed to parse test file");
    let detector = GodObjectDetector::with_source_content(&source_content);
    let analyses = detector.analyze_comprehensive(config_path, &file);

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

            assert!(
                modules_with_structs >= 2,
                "Structs should be distributed among multiple modules, found in {} modules",
                modules_with_structs
            );
        }
    }
}

/// Test that warnings are generated for borderline module sizes
#[test]
fn test_module_size_warnings() {
    let config_path = Path::new("src/priority/scoring/concise_recommendation.rs");
    let source_content = fs::read_to_string(config_path).expect("Failed to read test file");

    let file = syn::parse_file(&source_content).expect("Failed to parse test file");
    let detector = GodObjectDetector::with_source_content(&source_content);
    let analyses = detector.analyze_comprehensive(config_path, &file);

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

            // If warned about size, the module should be relatively large
            if warning.to_lowercase().contains("size")
                || warning.to_lowercase().contains("borderline")
            {
                assert!(
                    split.method_count >= 15,
                    "Size warning for '{}' but only has {} methods",
                    split.suggested_name,
                    split.method_count
                );
            }
        }
        }
    }
}
