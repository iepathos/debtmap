#[cfg(test)]
use super::blame_cache;
#[cfg(test)]
use super::function_level;
#[cfg(test)]
use super::git2_provider;
#[cfg(test)]
use super::stability::{self, StabilityStatus};
#[cfg(test)]
use super::test_helpers::{
    commit_with_message, create_test_file, modify_and_commit, setup_test_repo,
};
#[cfg(test)]
use super::{ContextProvider, GitHistoryProvider};
#[cfg(test)]
use crate::risk::context::{AnalysisTarget, ContextDetails};
#[cfg(test)]
use anyhow::Result;
#[cfg(test)]
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::Command;

#[cfg(test)]
fn function_history_git2(
    repo_path: &Path,
    file_path: &Path,
    function_name: &str,
    line_range: (usize, usize),
) -> Result<function_level::FunctionHistory> {
    let repo = git2_provider::Git2Repository::open(repo_path)?;
    let blame_cache = blame_cache::FileBlameCache::new(repo_path.to_path_buf());
    function_level::get_function_history_git2(
        &repo,
        file_path,
        function_name,
        line_range,
        &blame_cache,
    )
}

#[cfg(test)]
fn test_function_metric(file: PathBuf, name: &str) -> crate::core::FunctionMetrics {
    crate::core::FunctionMetrics {
        name: name.to_string(),
        file,
        line: 1,
        length: 1,
        cyclomatic: 1,
        cognitive: 1,
        nesting: 0,
        is_test: false,
        in_test_module: false,
        is_pure: None,
        visibility: None,
        is_trait_method: false,
        entropy_score: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis: None,
    }
}

#[test]
fn test_is_bug_fix_message() {
    use git2_provider::is_bug_fix_message;

    // Should match: genuine bug fixes
    assert!(is_bug_fix_message("fix: resolve login bug"));
    assert!(is_bug_fix_message("Fixed the payment issue"));
    assert!(is_bug_fix_message("Bug fix for issue #123"));
    assert!(is_bug_fix_message("hotfix: urgent fix"));

    // Should NOT match: conventional commit type exclusions
    assert!(!is_bug_fix_message("style: apply formatting fixes"));
    assert!(!is_bug_fix_message("chore: update dependencies"));
    assert!(!is_bug_fix_message("docs: fix typo"));
    assert!(!is_bug_fix_message("test: add unit tests"));

    // Should NOT match: maintenance keywords
    assert!(!is_bug_fix_message("refactor: improve prefix handling"));
    assert!(!is_bug_fix_message("apply linting rules"));
    assert!(!is_bug_fix_message("remove whitespace"));
    assert!(!is_bug_fix_message("fix: correct typo in documentation"));

    // Should match: refactor that mentions fix
    assert!(is_bug_fix_message("refactor: fix memory leak"));

    // Edge cases: case insensitivity
    assert!(!is_bug_fix_message("STYLE: Apply Formatting"));
    assert!(is_bug_fix_message("FIX: Resolve Bug"));
}

#[test]
fn test_git_history_provider_initialization() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    let provider = GitHistoryProvider::new(repo_path)?;
    assert_eq!(provider.cache.len(), 0);

    Ok(())
}

#[test]
fn test_file_history_analysis() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create and commit a test file
    let file_path = create_test_file(&repo_path, "test.rs", "fn main() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;

    // Make a bug fix commit
    std::fs::write(&file_path, "fn main() { println!(\"fixed\"); }")?;
    Command::new("git")
        .args(["add", "test.rs"])
        .current_dir(&repo_path)
        .output()?;
    commit_with_message(&repo_path, "fix: resolve printing issue")?;

    let mut provider = GitHistoryProvider::new(repo_path)?;
    let history = provider.analyze_file(Path::new("test.rs"))?;

    assert_eq!(history.total_commits, 2);
    assert_eq!(history.bug_fix_count, 1);
    assert_eq!(history.author_count, 1);

    Ok(())
}

#[test]
fn test_absolute_path_normalization() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create and commit a test file
    let file_path = create_test_file(&repo_path, "test.rs", "fn main() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;

    let mut provider = GitHistoryProvider::new(repo_path.clone())?;

    // Test with relative path
    let history_relative = provider.analyze_file(Path::new("test.rs"))?;

    // Test with absolute path (should produce same results)
    let history_absolute = provider.analyze_file(&file_path)?;

    // Both should return valid history with same commit count
    assert_eq!(
        history_relative.total_commits,
        history_absolute.total_commits
    );
    assert_eq!(history_relative.author_count, history_absolute.author_count);
    assert!(
        history_relative.total_commits > 0,
        "Should find commits with relative path"
    );
    assert!(
        history_absolute.total_commits > 0,
        "Should find commits with absolute path"
    );

    Ok(())
}

#[test]
fn test_calculate_bug_density_with_commits() {
    assert_eq!(stability::calculate_bug_density(0, 10), 0.0);
    assert_eq!(stability::calculate_bug_density(5, 10), 0.5);
    assert_eq!(stability::calculate_bug_density(10, 10), 1.0);
    assert_eq!(stability::calculate_bug_density(3, 10), 0.3);
}

#[test]
fn test_calculate_bug_density_no_commits() {
    assert_eq!(stability::calculate_bug_density(0, 0), 0.0);
    assert_eq!(stability::calculate_bug_density(5, 0), 0.0);
}

#[test]
fn test_determine_stability_status_highly_unstable() {
    let status = stability::determine_stability_status(6.0, 0.4, 100);
    assert!(matches!(status, StabilityStatus::HighlyUnstable));

    let status = stability::determine_stability_status(5.1, 0.31, 100);
    assert!(matches!(status, StabilityStatus::HighlyUnstable));
}

#[test]
fn test_determine_stability_status_frequently_changed() {
    let status = stability::determine_stability_status(3.0, 0.1, 100);
    assert!(matches!(status, StabilityStatus::FrequentlyChanged));

    let status = stability::determine_stability_status(2.1, 0.05, 50);
    assert!(matches!(status, StabilityStatus::FrequentlyChanged));
}

#[test]
fn test_determine_stability_status_bug_prone() {
    let status = stability::determine_stability_status(1.0, 0.25, 100);
    assert!(matches!(status, StabilityStatus::BugProne));

    let status = stability::determine_stability_status(0.5, 0.21, 200);
    assert!(matches!(status, StabilityStatus::BugProne));
}

#[test]
fn test_determine_stability_status_mature_stable() {
    let status = stability::determine_stability_status(0.5, 0.1, 400);
    assert!(matches!(status, StabilityStatus::MatureStable));

    let status = stability::determine_stability_status(1.0, 0.15, 366);
    assert!(matches!(status, StabilityStatus::MatureStable));
}

#[test]
fn test_determine_stability_status_relatively_stable() {
    let status = stability::determine_stability_status(1.5, 0.15, 200);
    assert!(matches!(status, StabilityStatus::RelativelyStable));

    let status = stability::determine_stability_status(2.0, 0.2, 365);
    assert!(matches!(status, StabilityStatus::RelativelyStable));
}

#[test]
fn test_determine_stability_status_edge_cases() {
    let status = stability::determine_stability_status(6.0, 0.4, 400);
    assert!(matches!(status, StabilityStatus::HighlyUnstable));

    let status = stability::determine_stability_status(0.0, 0.0, 0);
    assert!(matches!(status, StabilityStatus::RelativelyStable));

    let status = stability::determine_stability_status(2.5, 0.25, 100);
    assert!(matches!(status, StabilityStatus::FrequentlyChanged));
}

#[test]
fn test_classify_risk_contribution_continuous_scaling() {
    // Test that contribution scales continuously with bug density
    // Formula: bug_density * 1.5 + min(freq/20, 0.5)

    // Stable: no bugs, no changes → zero contribution (no risk increase)
    let stable = stability::classify_risk_contribution(0.0, 0.0);
    assert!((stable - 0.0).abs() < 0.001, "Expected 0.0, got {stable}");

    // Low bug density (25%)
    let low_bugs = stability::classify_risk_contribution(0.0, 0.25);
    assert!(
        (low_bugs - 0.375).abs() < 0.001,
        "Expected 0.375, got {low_bugs}"
    );

    // Medium bug density (50%)
    let medium_bugs = stability::classify_risk_contribution(0.0, 0.5);
    assert!(
        (medium_bugs - 0.75).abs() < 0.001,
        "Expected 0.75, got {medium_bugs}"
    );

    // High bug density (100%)
    let high_bugs = stability::classify_risk_contribution(0.0, 1.0);
    assert!(
        (high_bugs - 1.5).abs() < 0.001,
        "Expected 1.5, got {high_bugs}"
    );

    // 100% bugs should be 4x higher than 25% bugs
    assert!(
        (high_bugs / low_bugs - 4.0).abs() < 0.001,
        "100% bugs ({high_bugs}) should be 4x higher than 25% bugs ({low_bugs})"
    );
}

#[test]
fn test_classify_risk_contribution_frequency_impact() {
    // Test that change frequency adds to the contribution

    // High frequency (10/month) saturates at 0.5
    let high_freq = stability::classify_risk_contribution(10.0, 0.0);
    assert!(
        (high_freq - 0.5).abs() < 0.001,
        "Expected 0.5, got {high_freq}"
    );

    // Medium frequency (5/month)
    let medium_freq = stability::classify_risk_contribution(5.0, 0.0);
    assert!(
        (medium_freq - 0.25).abs() < 0.001,
        "Expected 0.25, got {medium_freq}"
    );

    // Frequency contribution saturates at 10/month
    let very_high_freq = stability::classify_risk_contribution(20.0, 0.0);
    assert!(
        (very_high_freq - 0.5).abs() < 0.001,
        "Expected 0.5 (saturated), got {very_high_freq}"
    );
}

#[test]
fn test_classify_risk_contribution_combined() {
    // Test combined effect of bugs and frequency

    // User's example: 25% bugs, 4.53 changes/month
    let example_low = stability::classify_risk_contribution(4.53, 0.25);
    // bugs(0.375) + freq(0.2265) = 0.6015
    assert!(
        (example_low - 0.6015).abs() < 0.01,
        "Expected ~0.60, got {example_low}"
    );

    // User's example: 100% bugs, 0.59 changes/month
    let example_high = stability::classify_risk_contribution(0.59, 1.0);
    // bugs(1.5) + freq(0.0295) = 1.5295
    assert!(
        (example_high - 1.5295).abs() < 0.01,
        "Expected ~1.53, got {example_high}"
    );

    // 100% bugs should be significantly higher than 25% bugs
    assert!(
        example_high > example_low * 2.0,
        "100% bugs ({example_high}) should be >2x higher than 25% bugs ({example_low})"
    );
}

#[test]
fn test_classify_risk_contribution_capped_at_max() {
    // Test that contribution is capped at 2.0
    let extreme = stability::classify_risk_contribution(100.0, 1.5);
    assert!(
        (extreme - 2.0).abs() < 0.001,
        "Expected 2.0 (capped), got {extreme}"
    );
}

#[test]
fn test_format_stability_message_highly_unstable() {
    let message =
        stability::format_stability_message(StabilityStatus::HighlyUnstable, 8.5, 0.45, 180, 5);
    assert_eq!(message, "Highly unstable: 8.5 changes/month, 45% bug fixes");

    let message =
        stability::format_stability_message(StabilityStatus::HighlyUnstable, 12.3, 0.67, 90, 10);
    assert_eq!(
        message,
        "Highly unstable: 12.3 changes/month, 67% bug fixes"
    );
}

#[test]
fn test_format_stability_message_frequently_changed() {
    let message =
        stability::format_stability_message(StabilityStatus::FrequentlyChanged, 3.5, 0.15, 200, 7);
    assert_eq!(
        message,
        "Frequently changed: 3.5 changes/month by 7 authors"
    );

    let message =
        stability::format_stability_message(StabilityStatus::FrequentlyChanged, 5.2, 0.08, 100, 1);
    assert_eq!(
        message,
        "Frequently changed: 5.2 changes/month by 1 authors"
    );
}

#[test]
fn test_format_stability_message_bug_prone() {
    let message = stability::format_stability_message(StabilityStatus::BugProne, 1.2, 0.35, 150, 3);
    assert_eq!(message, "Bug-prone: 35% of commits are bug fixes");

    let message = stability::format_stability_message(StabilityStatus::BugProne, 0.8, 0.72, 300, 2);
    assert_eq!(message, "Bug-prone: 72% of commits are bug fixes");
}

#[test]
fn test_format_stability_message_mature_stable() {
    let message =
        stability::format_stability_message(StabilityStatus::MatureStable, 0.5, 0.05, 730, 2);
    assert_eq!(message, "Mature and stable: 730 days old");

    let message =
        stability::format_stability_message(StabilityStatus::MatureStable, 0.3, 0.02, 1095, 1);
    assert_eq!(message, "Mature and stable: 1095 days old");
}

#[test]
fn test_format_stability_message_relatively_stable() {
    let message =
        stability::format_stability_message(StabilityStatus::RelativelyStable, 1.8, 0.12, 250, 4);
    assert_eq!(message, "Relatively stable: 1.8 changes/month");

    let message =
        stability::format_stability_message(StabilityStatus::RelativelyStable, 0.2, 0.0, 30, 1);
    assert_eq!(message, "Relatively stable: 0.2 changes/month");
}

#[test]
fn test_gather_integration() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create and commit a test file with multiple changes
    let file_path = create_test_file(&repo_path, "test.rs", "fn main() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;

    // Add more commits to create history
    for i in 1..=3 {
        std::fs::write(&file_path, format!("fn main() {{ /* change {i} */ }}"))?;
        Command::new("git")
            .args(["add", "test.rs"])
            .current_dir(&repo_path)
            .output()?;
        commit_with_message(&repo_path, &format!("fix: bug fix {i}"))?;
    }

    let provider = GitHistoryProvider::new(repo_path.clone())?;
    let target = AnalysisTarget {
        root_path: repo_path,
        file_path: PathBuf::from("test.rs"),
        function_name: "main".to_string(),
        line_range: (1, 10),
        reference_time: chrono::Utc::now(),
    };

    let context = provider.gather(&target)?;

    assert_eq!(context.provider, "git_history");
    assert_eq!(context.weight, 1.0);

    // Check that the contribution is calculated correctly
    if let ContextDetails::Historical { bug_density, .. } = context.details {
        // We have 3 bug fixes out of 4 commits = 0.75 bug density
        assert!(bug_density > 0.7);
        // With high bug density (>0.3), we expect high contribution
        assert!(context.contribution >= 1.0);
    } else {
        panic!("Expected Historical context details");
    }

    Ok(())
}

#[test]
fn test_setup_test_repo_creates_temp_directory() -> Result<()> {
    let (temp_dir, repo_path) = setup_test_repo()?;

    // Verify temp directory exists
    assert!(temp_dir.path().exists());
    assert!(repo_path.exists());

    // Verify they point to the same location
    assert_eq!(temp_dir.path(), repo_path);

    Ok(())
}

#[test]
fn test_setup_test_repo_initializes_git_repository() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Verify .git directory exists
    let git_dir = repo_path.join(".git");
    assert!(git_dir.exists());
    assert!(git_dir.is_dir());

    // Verify it's a valid git repository
    let output = Command::new("git")
        .args(["status"])
        .current_dir(&repo_path)
        .output()?;
    assert!(output.status.success());

    Ok(())
}

#[test]
fn test_setup_test_repo_configures_user_email() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Verify user email is configured
    let output = Command::new("git")
        .args(["config", "user.email"])
        .current_dir(&repo_path)
        .output()?;

    assert!(output.status.success());
    let email = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(email, "test@example.com");

    Ok(())
}

#[test]
fn test_setup_test_repo_configures_user_name() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Verify user name is configured
    let output = Command::new("git")
        .args(["config", "user.name"])
        .current_dir(&repo_path)
        .output()?;

    assert!(output.status.success());
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(name, "Test User");

    Ok(())
}

#[test]
fn test_setup_test_repo_returns_valid_paths() -> Result<()> {
    let (temp_dir, repo_path) = setup_test_repo()?;

    // Verify both paths are absolute
    assert!(repo_path.is_absolute());
    assert!(temp_dir.path().is_absolute());

    // Verify we can create files in the repository
    let test_file = repo_path.join("test.txt");
    std::fs::write(&test_file, "test content")?;
    assert!(test_file.exists());

    // Verify we can run git commands in the repository
    let output = Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&repo_path)
        .output()?;
    assert!(output.status.success());

    Ok(())
}

#[test]
fn test_bug_fix_detection_precision() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create initial test file and commit
    create_test_file(&repo_path, "test.rs", "fn main() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;

    // True positives - these SHOULD be counted as bug fixes
    modify_and_commit(&repo_path, "test.rs", "v2", "fix: resolve login bug")?;
    modify_and_commit(&repo_path, "test.rs", "v3", "Fixed the payment issue")?;
    modify_and_commit(&repo_path, "test.rs", "v4", "Bug fix for issue #123")?;

    // False positives - these should NOT be counted (should be filtered out)
    modify_and_commit(&repo_path, "test.rs", "v5", "style: apply formatting fixes")?;
    modify_and_commit(
        &repo_path,
        "test.rs",
        "v6",
        "refactor: improve prefix handling",
    )?;
    modify_and_commit(&repo_path, "test.rs", "v7", "Add debugging tools")?;
    modify_and_commit(&repo_path, "test.rs", "v8", "chore: fix linting issues")?;

    let mut provider = GitHistoryProvider::new(repo_path)?;
    let history = provider.analyze_file(Path::new("test.rs"))?;

    // Should detect 3 bug fixes (true positives), not 7
    assert_eq!(
        history.bug_fix_count, 3,
        "Expected 3 bug fixes, got {}",
        history.bug_fix_count
    );

    // Total commits includes initial commit + 7 changes = 8
    assert_eq!(
        history.total_commits, 8,
        "Expected 8 total commits, got {}",
        history.total_commits
    );

    // Bug density should be 3/8 = 0.375, not 7/8 = 0.875
    let bug_density =
        stability::calculate_bug_density(history.bug_fix_count, history.total_commits);
    assert!(
        bug_density > 0.35 && bug_density < 0.40,
        "Expected bug density ~0.375, got {}",
        bug_density
    );

    Ok(())
}

#[test]
fn test_word_boundary_matching_precision() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create initial test file and commit
    create_test_file(&repo_path, "test.rs", "fn main() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;

    // Commits with word boundary false positives (should NOT match with word boundaries)
    modify_and_commit(
        &repo_path,
        "test.rs",
        "v2",
        "refactor: improve prefix handling logic",
    )?;
    modify_and_commit(
        &repo_path,
        "test.rs",
        "v3",
        "update: add fixture for testing",
    )?;
    modify_and_commit(&repo_path, "test.rs", "v4", "Add debugging utilities")?;

    // Commits that should match (actual bug fixes)
    modify_and_commit(&repo_path, "test.rs", "v5", "fix the authentication bug")?;
    modify_and_commit(&repo_path, "test.rs", "v6", "fixes issue with validation")?;

    let mut provider = GitHistoryProvider::new(repo_path)?;
    let history = provider.analyze_file(Path::new("test.rs"))?;

    // Should only detect 2 bug fixes (the ones with actual "fix"/"fixes" words)
    // NOT the ones with "prefix", "fixture", or "debugging"
    assert_eq!(
        history.bug_fix_count, 2,
        "Word boundary matching should find 2 bug fixes, got {}",
        history.bug_fix_count
    );

    Ok(())
}

// =========================================================================
// Function-Level History Integration Tests
// =========================================================================

/// Test that function-level analysis returns 0 bug density for functions
/// that were introduced but never modified.
#[test]
fn test_function_level_never_modified() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create file with two functions
    let content = r#"fn my_func() {}

fn other_func() {}
"#;
    create_test_file(&repo_path, "test.rs", content)?;
    commit_with_message(&repo_path, "Initial commit")?;

    // Modify only other_func (not my_func)
    let content_v2 = r#"fn my_func() {}

fn other_func() {
println!("modified");
}
"#;
    modify_and_commit(&repo_path, "test.rs", content_v2, "fix: bug in other_func")?;

    // Get function-level history for my_func
    // Use line range (1, 10) to cover the function for git blame
    let history = function_history_git2(&repo_path, Path::new("test.rs"), "my_func", (1, 10))?;

    // my_func was introduced but never modified after introduction
    assert_eq!(
        history.total_commits, 0,
        "my_func should have 0 modifications, got {}",
        history.total_commits
    );
    assert_eq!(
        history.bug_density(),
        0.0,
        "my_func should have 0% bug density"
    );
    assert_eq!(
        history.change_frequency(chrono::Utc::now()),
        0.0,
        "my_func should have 0 change frequency"
    );

    Ok(())
}

/// git2-backed history must match subprocess pickaxe results (accuracy contract).
#[test]
fn test_function_history_git2_matches_subprocess() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    create_test_file(&repo_path, "test.rs", "fn my_func() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;
    modify_and_commit(
        &repo_path,
        "test.rs",
        "fn my_func() { println!(\"v2\"); }",
        "fix: bug in my_func",
    )?;
    modify_and_commit(
        &repo_path,
        "test.rs",
        "fn my_func() { println!(\"v3\"); }",
        "feat: improve my_func",
    )?;

    let blame_cache = blame_cache::FileBlameCache::new(repo_path.clone());
    let git2_repo = super::git2_provider::Git2Repository::open(&repo_path)?;

    let subprocess = function_level::get_function_history_subprocess(
        &repo_path,
        Path::new("test.rs"),
        "my_func",
        (1, 5),
        &blame_cache,
        chrono::Utc::now(),
    )?;

    let git2 = function_level::get_function_history_git2(
        &git2_repo,
        Path::new("test.rs"),
        "my_func",
        (1, 5),
        &blame_cache,
    )?;

    assert_eq!(git2.total_commits, subprocess.total_commits);
    assert_eq!(git2.bug_fix_count, subprocess.bug_fix_count);
    assert_eq!(git2.introduced, subprocess.introduced);
    assert_eq!(git2.bug_density(), subprocess.bug_density());

    Ok(())
}

#[test]
fn test_batched_function_preload_matches_direct_lookup() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    create_test_file(&repo_path, "test.rs", "fn alpha() {}\nfn beta() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;
    modify_and_commit(
        &repo_path,
        "test.rs",
        "fn alpha() {}\nfn beta() { println!(\"x\"); }",
        "fix: beta",
    )?;

    let mut provider = GitHistoryProvider::new(repo_path.clone())?;
    let metrics = vec![
        crate::core::FunctionMetrics {
            name: "alpha".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            length: 1,
            cyclomatic: 1,
            cognitive: 1,
            nesting: 0,
            is_test: false,
            in_test_module: false,
            is_pure: None,
            visibility: None,
            is_trait_method: false,
            entropy_score: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        },
        crate::core::FunctionMetrics {
            name: "beta".to_string(),
            file: PathBuf::from("test.rs"),
            line: 2,
            length: 3,
            cyclomatic: 1,
            cognitive: 1,
            nesting: 0,
            is_test: false,
            in_test_module: false,
            is_pure: None,
            visibility: None,
            is_trait_method: false,
            entropy_score: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        },
    ];

    provider.preload_function_histories(&metrics)?;

    let target_alpha = AnalysisTarget {
        root_path: repo_path.clone(),
        file_path: PathBuf::from("test.rs"),
        function_name: "alpha".to_string(),
        line_range: (1, 1),
        reference_time: chrono::Utc::now(),
    };
    let ctx_alpha = provider.gather(&target_alpha)?;
    assert_eq!(ctx_alpha.provider, "git_history");

    let target_beta = AnalysisTarget {
        root_path: repo_path.clone(),
        file_path: PathBuf::from("test.rs"),
        function_name: "beta".to_string(),
        line_range: (2, 4),
        reference_time: chrono::Utc::now(),
    };
    let ctx_beta = provider.gather(&target_beta)?;
    if let ContextDetails::Historical {
        total_commits,
        bug_fix_count,
        ..
    } = ctx_beta.details
    {
        assert_eq!(total_commits, 2, "beta: intro + one modification");
        assert_eq!(bug_fix_count, 1);
    } else {
        panic!("expected historical context");
    }

    Ok(())
}

#[test]
fn test_function_level_with_modifications() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create file with a function
    create_test_file(&repo_path, "test.rs", "fn my_func() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;

    // Modify my_func twice
    modify_and_commit(
        &repo_path,
        "test.rs",
        "fn my_func() { println!(\"v2\"); }",
        "fix: bug in my_func",
    )?;
    modify_and_commit(
        &repo_path,
        "test.rs",
        "fn my_func() { println!(\"v3\"); }",
        "feat: improve my_func",
    )?;

    let history = function_history_git2(&repo_path, Path::new("test.rs"), "my_func", (1, 5))?;

    // my_func has 2 modifications after introduction, 1 is a bug fix
    assert_eq!(
        history.total_commits, 2,
        "my_func should have 2 modifications, got {}",
        history.total_commits
    );
    assert_eq!(
        history.bug_fix_count, 1,
        "my_func should have 1 bug fix, got {}",
        history.bug_fix_count
    );
    assert!(
        (history.bug_density() - 0.5).abs() < 0.01,
        "my_func should have 50% bug density, got {}",
        history.bug_density()
    );

    Ok(())
}

/// Test that GitHistoryProvider::gather uses function-level analysis
/// when function_name is provided.
#[test]
fn test_gather_uses_function_level_analysis() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create file with two functions
    let content = r#"fn stable_func() {}

fn buggy_func() {}
"#;
    create_test_file(&repo_path, "test.rs", content)?;
    commit_with_message(&repo_path, "Initial commit")?;

    // Add bug fixes only to buggy_func
    let content_v2 = r#"fn stable_func() {}

fn buggy_func() {
println!("fixed");
}
"#;
    modify_and_commit(&repo_path, "test.rs", content_v2, "fix: bug in buggy_func")?;

    let provider = GitHistoryProvider::new(repo_path.clone())?;

    // Analyze stable_func - should have 0 bug density
    let target_stable = AnalysisTarget {
        root_path: repo_path.clone(),
        file_path: PathBuf::from("test.rs"),
        function_name: "stable_func".to_string(),
        line_range: (1, 1),
        reference_time: chrono::Utc::now(),
    };
    let context_stable = provider.gather(&target_stable)?;
    if let ContextDetails::Historical { bug_density, .. } = context_stable.details {
        assert_eq!(
            bug_density, 0.0,
            "stable_func should have 0% bug density, got {}",
            bug_density
        );
    } else {
        panic!("Expected Historical context details");
    }

    // Analyze buggy_func - should have high bug density
    let target_buggy = AnalysisTarget {
        root_path: repo_path,
        file_path: PathBuf::from("test.rs"),
        function_name: "buggy_func".to_string(),
        line_range: (3, 5),
        reference_time: chrono::Utc::now(),
    };
    let context_buggy = provider.gather(&target_buggy)?;
    if let ContextDetails::Historical { bug_density, .. } = context_buggy.details {
        assert!(
            bug_density > 0.9,
            "buggy_func should have 100% bug density, got {}",
            bug_density
        );
    } else {
        panic!("Expected Historical context details");
    }

    Ok(())
}

/// Test that file-level analysis is used when function_name is empty.
#[test]
fn test_gather_falls_back_to_file_level() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    create_test_file(&repo_path, "test.rs", "fn main() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;
    modify_and_commit(&repo_path, "test.rs", "fn main() { /* v2 */ }", "fix: bug")?;

    let provider = GitHistoryProvider::new(repo_path.clone())?;

    // Analyze without function_name - should use file-level
    let target = AnalysisTarget {
        root_path: repo_path,
        file_path: PathBuf::from("test.rs"),
        function_name: String::new(), // Empty - triggers fallback
        line_range: (1, 1),
        reference_time: chrono::Utc::now(),
    };
    let context = provider.gather(&target)?;

    // Should successfully return file-level context
    assert_eq!(context.provider, "git_history");
    if let ContextDetails::Historical {
        change_frequency,
        bug_density,
        ..
    } = context.details
    {
        // File-level should show the bug fix
        assert!(
            bug_density > 0.0,
            "File-level should detect bug fix, got {}",
            bug_density
        );
        assert!(
            change_frequency >= 0.0,
            "Change frequency should be non-negative"
        );
    } else {
        panic!("Expected Historical context details");
    }

    Ok(())
}

#[test]
fn test_dot_slash_prefix_normalization() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create src directory and test file
    std::fs::create_dir_all(repo_path.join("src"))?;
    let _file_path = create_test_file(&repo_path, "src/test.rs", "fn main() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;

    // Add more commits
    modify_and_commit(
        &repo_path,
        "src/test.rs",
        "fn main() { /* v2 */ }",
        "fix: bug fix",
    )?;

    let mut provider = GitHistoryProvider::new(repo_path)?;

    // Test with ./ prefix - should find the same history
    let history_dot_slash = provider.analyze_file(Path::new("./src/test.rs"))?;

    // Test without ./ prefix
    let history_no_prefix = provider.analyze_file(Path::new("src/test.rs"))?;

    // Both should return valid history with same commit count
    assert_eq!(
        history_dot_slash.total_commits, history_no_prefix.total_commits,
        "./ prefix should be normalized: {} vs {}",
        history_dot_slash.total_commits, history_no_prefix.total_commits
    );
    assert!(
        history_dot_slash.total_commits > 0,
        "Should find commits with ./ prefix path"
    );
    assert!(
        history_no_prefix.total_commits > 0,
        "Should find commits without prefix"
    );

    Ok(())
}

/// Diagnostic test: Expose exactly what paths are stored vs looked up
/// This test intentionally prints debug info to help identify path mismatches
#[test]
fn test_batched_history_path_matching_diagnostic() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create src directory and test file
    std::fs::create_dir_all(repo_path.join("src"))?;
    let _file_path = create_test_file(&repo_path, "src/test.rs", "fn main() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;

    let mut provider = GitHistoryProvider::new(repo_path.clone())?;
    provider.preload_function_histories(&[test_function_metric(
        PathBuf::from("src/test.rs"),
        "main",
    )])?;

    // Debug: Print what paths are stored in batched history
    let stored_paths = provider.batched_paths();
    eprintln!("=== BATCHED HISTORY DEBUG ===");
    eprintln!("Repo root: {:?}", provider.repo_root());
    eprintln!(
        "Stored paths in batched history ({} total):",
        stored_paths.len()
    );
    for path in &stored_paths {
        eprintln!("  - {:?}", path);
    }

    // Check if the path we expect exists
    let expected_path = Path::new("src/test.rs");
    let has_path = provider.batched_has_path(expected_path);
    eprintln!("Has 'src/test.rs': {}", has_path);

    // The batched history should have the file
    assert!(
        !stored_paths.is_empty(),
        "Batched history should not be empty after commit"
    );

    // This is the critical assertion - the path format should match
    assert!(
        has_path,
        "Batched history should contain 'src/test.rs'. Stored paths: {:?}",
        stored_paths
    );

    Ok(())
}

/// Test against the actual debtmap codebase to see if git history works
/// This is the real scenario that's failing
#[test]
#[ignore]
fn test_git_history_on_real_repo() -> Result<()> {
    // Use the actual debtmap repo (the repo we're in)
    let cwd = std::env::current_dir()?;
    eprintln!("=== REAL REPO TEST ===");
    eprintln!("Current working directory: {:?}", cwd);

    let mut provider = GitHistoryProvider::new(cwd.clone())?;
    provider
        .preload_function_histories(&[test_function_metric(PathBuf::from("src/lib.rs"), "main")])?;
    eprintln!("Provider repo root: {:?}", provider.repo_root());

    // List some paths in batched history
    let stored_paths = provider.batched_paths();
    eprintln!("Total paths in batched history: {}", stored_paths.len());
    eprintln!("First 10 paths:");
    for (i, path) in stored_paths.iter().take(10).enumerate() {
        eprintln!("  {}: {:?}", i, path);
    }

    // Try to find a known file
    let test_path = Path::new("src/lib.rs");
    let has_lib = provider.batched_has_path(test_path);
    eprintln!("Has 'src/lib.rs' in batched: {}", has_lib);

    // The repo should have history for src/lib.rs
    assert!(
        !stored_paths.is_empty(),
        "Batched history should not be empty for a real repo with commits"
    );

    // Now try to get actual history
    let history = provider.analyze_file(test_path)?;
    eprintln!(
        "History for src/lib.rs: commits={}, authors={}, age_days={}",
        history.total_commits, history.author_count, history.age_days
    );

    assert!(
        history.total_commits > 0,
        "src/lib.rs should have commits in a real repo. Got: {}",
        history.total_commits
    );

    Ok(())
}

/// Test that simulates analysis flow with file paths from different origins
/// This tests the exact scenario where paths come from analysis results
#[test]
#[ignore]
fn test_git_history_with_analysis_style_paths() -> Result<()> {
    let cwd = std::env::current_dir()?;
    eprintln!("=== ANALYSIS STYLE PATHS TEST ===");
    eprintln!("CWD: {:?}", cwd);

    // This simulates creating GitHistoryProvider with "." path
    // (like when running `debtmap analyze .`)
    let dot_path = PathBuf::from(".");

    // First, check that the provider creation succeeds (this mirrors create_git_history_provider)
    let provider_result = GitHistoryProvider::new(dot_path.clone());
    eprintln!("Provider creation result: {:?}", provider_result.is_ok());
    if let Err(ref e) = provider_result {
        eprintln!("Error creating provider with '.': {}", e);
    }
    let provider_from_dot = provider_result?;
    eprintln!(
        "Provider repo root (from '.'): {:?}",
        provider_from_dot.repo_root()
    );

    // Test paths that might come from analysis results:
    // 1. Relative from CWD (like metrics might have)
    let rel_path = PathBuf::from("src/lib.rs");

    // 2. Absolute path (like metrics might have if canonicalized)
    let abs_path = cwd.join("src/lib.rs");

    // 3. With ./ prefix
    let dot_slash_path = PathBuf::from("./src/lib.rs");

    eprintln!("Testing paths:");
    eprintln!("  rel_path: {:?}", rel_path);
    eprintln!("  abs_path: {:?}", abs_path);
    eprintln!("  dot_slash_path: {:?}", dot_slash_path);

    let mut provider = provider_from_dot;

    // All three should work
    let history_rel = provider.analyze_file(&rel_path)?;
    eprintln!(
        "Relative path result: commits={}",
        history_rel.total_commits
    );
    assert!(
        history_rel.total_commits > 0,
        "Relative path 'src/lib.rs' should find commits. Got: {}",
        history_rel.total_commits
    );

    let history_abs = provider.analyze_file(&abs_path)?;
    eprintln!(
        "Absolute path result: commits={}",
        history_abs.total_commits
    );
    assert!(
        history_abs.total_commits > 0,
        "Absolute path should find commits. Got: {}",
        history_abs.total_commits
    );

    let history_dot = provider.analyze_file(&dot_slash_path)?;
    eprintln!(
        "Dot-slash path result: commits={}",
        history_dot.total_commits
    );
    assert!(
        history_dot.total_commits > 0,
        "./src/lib.rs should find commits. Got: {}",
        history_dot.total_commits
    );

    Ok(())
}

/// Test using the full ContextAggregator flow with git history.
/// This simulates the actual analysis pipeline and verifies that when
/// function-level git history fails (function not found), it falls back
/// to file-level history which should have commit data.
#[test]
#[ignore]
fn test_git_history_via_context_aggregator() -> Result<()> {
    use crate::risk::context::{AnalysisTarget, ContextAggregator, ContextDetails};

    let dot_path = PathBuf::from(".");
    let git_provider = GitHistoryProvider::new(dot_path.clone())?;

    let aggregator = ContextAggregator::new().with_provider(Box::new(git_provider));

    // Use a function name that doesn't exist - this tests the fallback
    // from function-level to file-level analysis
    let target = AnalysisTarget {
        root_path: dot_path.clone(),
        file_path: PathBuf::from("src/lib.rs"),
        function_name: "nonexistent_function".to_string(),
        line_range: (1, 100),
        reference_time: chrono::Utc::now(),
    };

    let context_map = aggregator.analyze(&target);

    // Verify git history is included with non-zero values
    let git_context = context_map
        .contexts
        .get("git_history")
        .expect("git_history context should be present");

    if let ContextDetails::Historical {
        total_commits,
        author_count,
        ..
    } = &git_context.details
    {
        assert!(
            *total_commits > 0,
            "Git history should have commits (fallback to file-level). Got: {} commits",
            total_commits
        );
        assert!(
            *author_count > 0,
            "Git history should have authors. Got: {} authors",
            author_count
        );
        // Note: age_days not checked - can be 0 in CI with shallow clones
    } else {
        panic!("Expected Historical context details");
    }

    Ok(())
}

/// Test that verifies git history works when called from project subdirectory
/// This simulates `cd project && debtmap analyze .`
#[test]
fn test_git_history_from_subdirectory() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create nested structure
    std::fs::create_dir_all(repo_path.join("src/utils"))?;
    let _file_path = create_test_file(&repo_path, "src/utils/helper.rs", "fn help() {}")?;
    commit_with_message(&repo_path, "Add helper")?;

    // Add more commits
    modify_and_commit(
        &repo_path,
        "src/utils/helper.rs",
        "fn help() { /* v2 */ }",
        "fix: improve helper",
    )?;

    let mut provider = GitHistoryProvider::new(repo_path.clone())?;

    // Test various path formats that might be used in real scenarios:
    // 1. Relative from repo root
    let history1 = provider.analyze_file(Path::new("src/utils/helper.rs"))?;
    assert!(
        history1.total_commits >= 2,
        "Should find 2+ commits for relative path. Got: {}",
        history1.total_commits
    );

    // 2. With ./ prefix (common when running from working directory)
    let history2 = provider.analyze_file(Path::new("./src/utils/helper.rs"))?;
    assert!(
        history2.total_commits >= 2,
        "Should find 2+ commits for ./ prefixed path. Got: {}",
        history2.total_commits
    );

    // 3. Absolute path
    let abs_path = repo_path.join("src/utils/helper.rs");
    let history3 = provider.analyze_file(&abs_path)?;
    assert!(
        history3.total_commits >= 2,
        "Should find 2+ commits for absolute path. Got: {}",
        history3.total_commits
    );

    Ok(())
}
