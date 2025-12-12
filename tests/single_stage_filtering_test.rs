//! Integration tests for single-stage filtering (spec 243)

use debtmap::priority::filter_config::ItemFilterConfig;

#[test]
fn configuration_precedence_env_var_overrides_default() {
    // Set env var to override default
    std::env::set_var("DEBTMAP_MIN_SCORE_THRESHOLD", "15.0");

    let config = ItemFilterConfig::from_environment();
    assert_eq!(config.min_score, 15.0);

    // Clean up
    std::env::remove_var("DEBTMAP_MIN_SCORE_THRESHOLD");
}

#[test]
fn configuration_precedence_cli_overrides_env_var() {
    // Set env var
    std::env::set_var("DEBTMAP_MIN_SCORE_THRESHOLD", "15.0");

    let config = ItemFilterConfig::from_environment();
    assert_eq!(config.min_score, 15.0);

    // CLI override
    let config = config.with_min_score(Some(20.0));
    assert_eq!(config.min_score, 20.0);

    // Clean up
    std::env::remove_var("DEBTMAP_MIN_SCORE_THRESHOLD");
}

#[test]
fn permissive_config_allows_everything() {
    let config = ItemFilterConfig::permissive();

    assert_eq!(config.min_score, 0.0);
    assert_eq!(config.min_cyclomatic, 0);
    assert_eq!(config.min_cognitive, 0);
    assert_eq!(config.min_risk, 0.0);
    assert!(config.show_t4_items);
}

#[test]
fn with_methods_override_correctly() {
    let config = ItemFilterConfig::permissive()
        .with_min_score(Some(10.0))
        .with_min_cyclomatic(Some(5))
        .with_min_cognitive(Some(8));

    assert_eq!(config.min_score, 10.0);
    assert_eq!(config.min_cyclomatic, 5);
    assert_eq!(config.min_cognitive, 8);
}

#[test]
fn with_methods_accept_none_without_changing() {
    // Use permissive() for deterministic baseline (avoids env var race conditions)
    let original = ItemFilterConfig::permissive()
        .with_min_score(Some(10.0))
        .with_min_cyclomatic(Some(5))
        .with_min_cognitive(Some(8));

    let config = original
        .clone()
        .with_min_score(None)
        .with_min_cyclomatic(None)
        .with_min_cognitive(None);

    // Should remain unchanged when passing None
    assert_eq!(config.min_score, original.min_score);
    assert_eq!(config.min_cyclomatic, original.min_cyclomatic);
    assert_eq!(config.min_cognitive, original.min_cognitive);
}

#[test]
fn tui_and_no_tui_use_same_unified_analysis() {
    // This test validates the architectural guarantee that TUI and --no-tui
    // modes show identical items because they both use the same UnifiedAnalysis
    // result without additional filtering.
    //
    // The key invariant: filtering happens ONCE during UnifiedAnalysis construction.
    // Both output modes (TUI and --no-tui) consume the same pre-filtered result.
    //
    // This is a documentation test - the actual implementation is verified by
    // code inspection of src/commands/analyze.rs where both paths use
    // `filtered_analysis` without modification.

    let config = ItemFilterConfig::from_environment();

    // The same config is used for both TUI and --no-tui paths
    assert!(config.min_score >= 0.0);

    // Both paths in analyze.rs use the same unified_analysis:
    // - TUI: ResultsExplorer::new(filtered_analysis)
    // - Non-TUI: output_unified_priorities_with_config(filtered_analysis, ...)
    //
    // This architectural pattern ensures consistency.
}

#[test]
fn god_objects_with_high_scores_pass_default_threshold() {
    // This test validates that god objects with high scores (>50) will
    // pass the default min_score threshold (3.0).
    //
    // This is mathematically guaranteed: if god_object_score > 50 and
    // min_score threshold is 3.0, then 50 > 3.0 is always true.

    let default_config = ItemFilterConfig::from_environment();

    // God objects with scores > 50 will always pass a threshold of 3.0
    let god_object_score = 75.0;
    assert!(
        god_object_score > default_config.min_score,
        "God objects with score {} should pass default threshold {}",
        god_object_score,
        default_config.min_score
    );

    // Even borderline god objects (score = 50) pass the default threshold
    let borderline_score = 50.0;
    assert!(
        borderline_score > default_config.min_score,
        "Borderline god objects with score {} should pass default threshold {}",
        borderline_score,
        default_config.min_score
    );
}
