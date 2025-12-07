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
    let original_score = ItemFilterConfig::from_environment().min_score;

    let config = ItemFilterConfig::from_environment()
        .with_min_score(None)
        .with_min_cyclomatic(None)
        .with_min_cognitive(None);

    // Should remain unchanged when passing None
    assert_eq!(config.min_score, original_score);
}
