//! Function-level git history analysis
//!
//! This module provides accurate git history analysis for individual functions,
//! rather than attributing file-level history to all functions in a file.
//!
//! # Problem Solved
//!
//! File-level analysis incorrectly attributes bug density to functions that were
//! never modified. For example:
//! - File has 8 commits, 3 bug fixes (37.5% bug density)
//! - Function `get_exclusions` was created once, never modified
//! - File-level: `get_exclusions` gets 37.5% bug density (INCORRECT)
//! - Function-level: `get_exclusions` gets 0% bug density (CORRECT)
//!
//! # Architecture
//!
//! Following the Stillwater philosophy of "pure core, imperative shell":
//! - Pure functions for parsing git output (easily testable without git)
//! - I/O wrapper functions for running git commands
//! - Function-level metrics calculation

use super::batched::is_bug_fix;
use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Information about a single commit from git log
#[derive(Debug, Clone, Default)]
pub struct CommitInfo {
    #[allow(dead_code)]
    pub hash: String,
    pub date: Option<DateTime<Utc>>,
    pub message: String,
    pub author: String,
}

/// History data for a specific function
#[derive(Debug, Clone, Default)]
pub struct FunctionHistory {
    /// Commit hash where function was introduced
    #[allow(dead_code)]
    pub introduction_commit: Option<String>,
    /// Total commits that modified this function after introduction
    pub total_commits: usize,
    /// Bug fix commits that modified this function
    pub bug_fix_count: usize,
    /// Authors who modified this function
    pub authors: HashSet<String>,
    /// When function was last modified
    #[allow(dead_code)]
    pub last_modified: Option<DateTime<Utc>>,
    /// When function was introduced
    pub introduced: Option<DateTime<Utc>>,
}

impl FunctionHistory {
    /// Calculate bug density for this function
    ///
    /// Pure function: returns 0.0 if function was never modified after introduction
    pub fn bug_density(&self) -> f64 {
        if self.total_commits == 0 {
            return 0.0; // Never modified = no bugs
        }
        self.bug_fix_count as f64 / self.total_commits as f64
    }

    /// Calculate change frequency (modifications per month)
    ///
    /// Pure function: returns 0.0 if function was never modified
    pub fn change_frequency(&self) -> f64 {
        let age_days = self.age_days();
        if age_days == 0 || self.total_commits == 0 {
            return 0.0;
        }
        (self.total_commits as f64 / age_days as f64) * 30.0
    }

    /// Calculate function age in days since introduction
    pub fn age_days(&self) -> u32 {
        self.introduced
            .map(|d| (Utc::now() - d).num_days().max(0) as u32)
            .unwrap_or(0)
    }
}

// =============================================================================
// Pure Functions (Testable Without Git)
// =============================================================================

/// Parse git log output to find introduction commit
///
/// Pure function - parses string input, returns Option<String>
///
/// # Arguments
/// * `git_output` - Output from `git log -S "fn function_name" --format="%H" --reverse`
///
/// # Returns
/// * The first (oldest) commit hash, or None if output is empty
pub fn parse_introduction_commit(git_output: &str) -> Option<String> {
    git_output
        .lines()
        .next()
        .map(|s| s.trim())
        .filter(|line| !line.is_empty())
        .map(|s| s.to_string())
}

/// Parse git log output to extract commit information
///
/// Pure function - parses formatted git log output
///
/// # Arguments
/// * `git_output` - Output from `git log --format=":::%H:::%cI:::%s:::%ae"`
///
/// # Returns
/// * Vector of parsed commits
pub fn parse_modification_commits(git_output: &str) -> Vec<CommitInfo> {
    git_output
        .lines()
        .filter(|line| line.starts_with(":::"))
        .filter_map(parse_commit_line)
        .collect()
}

/// Parse a single commit line from formatted output
///
/// Pure function - parses ":::%H:::%cI:::%s:::%ae" format
fn parse_commit_line(line: &str) -> Option<CommitInfo> {
    let parts: Vec<&str> = line.split(":::").collect();
    if parts.len() < 5 {
        return None;
    }
    let date = DateTime::parse_from_rfc3339(parts[2])
        .ok()
        .map(|d| d.with_timezone(&Utc));
    Some(CommitInfo {
        hash: parts[1].to_string(),
        date,
        message: parts[3].to_string(),
        author: parts[4].to_string(),
    })
}

/// Filter commits to only bug fixes
///
/// Pure function - uses existing is_bug_fix() logic
pub fn filter_bug_fix_commits(commits: &[CommitInfo]) -> Vec<&CommitInfo> {
    commits.iter().filter(|c| is_bug_fix(&c.message)).collect()
}

/// Calculate function history from parsed commits
///
/// Pure function - aggregates commit data into history
pub fn calculate_function_history(
    introduction_commit: Option<String>,
    introduction_date: Option<DateTime<Utc>>,
    modification_commits: &[CommitInfo],
) -> FunctionHistory {
    let bug_fixes = filter_bug_fix_commits(modification_commits);

    FunctionHistory {
        introduction_commit,
        total_commits: modification_commits.len(),
        bug_fix_count: bug_fixes.len(),
        authors: modification_commits
            .iter()
            .map(|c| c.author.clone())
            .collect(),
        last_modified: modification_commits.iter().filter_map(|c| c.date).max(),
        introduced: introduction_date,
    }
}

// =============================================================================
// I/O Wrapper Functions (Imperative Shell)
// =============================================================================

/// Get function history from git (I/O Shell)
///
/// This is the imperative shell that orchestrates git commands.
/// Falls back to default (empty) history if function is not found.
pub fn get_function_history(
    repo_root: &Path,
    file_path: &Path,
    function_name: &str,
) -> Result<FunctionHistory> {
    // I/O: Find introduction commit
    let intro_output = run_git_log_introduction(repo_root, file_path, function_name)?;
    let intro_commit = parse_introduction_commit(&intro_output);

    // If no introduction found, function doesn't exist in git history
    let Some(ref intro) = intro_commit else {
        log::debug!(
            "Function '{}' not found in git history for {}",
            function_name,
            file_path.display()
        );
        return Ok(FunctionHistory::default());
    };

    // I/O: Get introduction date
    let intro_date = get_commit_date(repo_root, intro)?;

    // I/O: Find modifications after introduction
    let mods_output = run_git_log_modifications(repo_root, file_path, function_name, intro)?;
    let modification_commits = parse_modification_commits(&mods_output);

    // Pure: Calculate history from parsed data
    Ok(calculate_function_history(
        intro_commit,
        intro_date,
        &modification_commits,
    ))
}

/// Run git log -S to find function introduction (I/O)
///
/// Uses pickaxe search to find commits that added the function signature.
fn run_git_log_introduction(
    repo_root: &Path,
    file_path: &Path,
    function_name: &str,
) -> Result<String> {
    // Search for function definition (fn function_name)
    let search_pattern = format!("fn {function_name}");
    let output = Command::new("git")
        .args([
            "log",
            "-S",
            &search_pattern,
            "--format=%H",
            "--reverse",
            "--",
            &file_path.to_string_lossy(),
        ])
        .current_dir(repo_root)
        .output()
        .context("Failed to run git log -S for function introduction")?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run git log to find modifications after introduction (I/O)
///
/// Uses `-G` (regex-based diff search) to find commits that modified
/// lines containing the function name. This is more inclusive than `-S`
/// (pickaxe) which only finds additions/deletions of the exact string count.
fn run_git_log_modifications(
    repo_root: &Path,
    file_path: &Path,
    function_name: &str,
    intro_commit: &str,
) -> Result<String> {
    let range = format!("{intro_commit}..HEAD");
    // Use -G to find commits where the diff matches the function name pattern
    // This catches any modification to lines containing the function
    let output = Command::new("git")
        .args([
            "log",
            &range,
            "-G",
            function_name,
            "--format=:::%H:::%cI:::%s:::%ae",
            "--",
            &file_path.to_string_lossy(),
        ])
        .current_dir(repo_root)
        .output()
        .context("Failed to run git log range for function modifications")?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get the date of a specific commit (I/O)
fn get_commit_date(repo_root: &Path, commit_hash: &str) -> Result<Option<DateTime<Utc>>> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%cI", commit_hash])
        .current_dir(repo_root)
        .output()
        .context("Failed to get commit date")?;

    if output.status.success() {
        let date_str = String::from_utf8_lossy(&output.stdout);
        let date_str = date_str.trim();
        if !date_str.is_empty() {
            return Ok(DateTime::parse_from_rfc3339(date_str)
                .ok()
                .map(|d| d.with_timezone(&Utc)));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Pure Function Tests (No Git Required)
    // =========================================================================

    #[test]
    fn test_parse_introduction_commit_found() {
        let output = "abc123def456\n";
        let result = parse_introduction_commit(output);
        assert_eq!(result, Some("abc123def456".to_string()));
    }

    #[test]
    fn test_parse_introduction_commit_multiple_lines() {
        // First line is oldest (due to --reverse flag)
        let output = "abc123def456\nxyz789xyz789\n";
        let result = parse_introduction_commit(output);
        assert_eq!(result, Some("abc123def456".to_string()));
    }

    #[test]
    fn test_parse_introduction_commit_empty() {
        let output = "";
        let result = parse_introduction_commit(output);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_introduction_commit_whitespace_only() {
        let output = "   \n\n";
        let result = parse_introduction_commit(output);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_modification_commits_multiple() {
        let output = r#":::abc123:::2025-01-01T10:00:00Z:::fix: bug:::author1@example.com
:::def456:::2025-01-02T10:00:00Z:::feat: feature:::author2@example.com"#;

        let commits = parse_modification_commits(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc123");
        assert_eq!(commits[0].message, "fix: bug");
        assert_eq!(commits[0].author, "author1@example.com");
        assert_eq!(commits[1].hash, "def456");
        assert_eq!(commits[1].message, "feat: feature");
        assert_eq!(commits[1].author, "author2@example.com");
    }

    #[test]
    fn test_parse_modification_commits_empty() {
        let output = "";
        let commits = parse_modification_commits(output);
        assert!(commits.is_empty());
    }

    #[test]
    fn test_parse_modification_commits_invalid_lines() {
        let output = r#"some garbage
:::abc123:::2025-01-01T10:00:00Z:::fix: bug:::author@example.com
more garbage"#;

        let commits = parse_modification_commits(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].hash, "abc123");
    }

    #[test]
    fn test_parse_modification_commits_missing_fields() {
        // Line with insufficient fields should be skipped
        let output = ":::abc123:::2025-01-01T10:00:00Z:::fix: bug";
        let commits = parse_modification_commits(output);
        assert!(commits.is_empty());
    }

    #[test]
    fn test_filter_bug_fix_commits() {
        let commits = vec![
            CommitInfo {
                message: "fix: bug".to_string(),
                ..Default::default()
            },
            CommitInfo {
                message: "feat: feature".to_string(),
                ..Default::default()
            },
            CommitInfo {
                message: "hotfix: urgent".to_string(),
                ..Default::default()
            },
            CommitInfo {
                message: "chore: cleanup".to_string(),
                ..Default::default()
            },
        ];

        let bug_fixes = filter_bug_fix_commits(&commits);
        assert_eq!(bug_fixes.len(), 2);
        assert_eq!(bug_fixes[0].message, "fix: bug");
        assert_eq!(bug_fixes[1].message, "hotfix: urgent");
    }

    #[test]
    fn test_function_history_never_modified() {
        let history = FunctionHistory {
            total_commits: 0,
            bug_fix_count: 0,
            introduced: Some(Utc::now() - chrono::Duration::days(30)),
            ..Default::default()
        };

        assert_eq!(history.bug_density(), 0.0);
        assert_eq!(history.change_frequency(), 0.0);
    }

    #[test]
    fn test_function_history_with_modifications() {
        let introduced = Utc::now() - chrono::Duration::days(30);
        let history = FunctionHistory {
            introduction_commit: Some("abc123".to_string()),
            total_commits: 4,
            bug_fix_count: 1,
            introduced: Some(introduced),
            ..Default::default()
        };

        assert_eq!(history.bug_density(), 0.25);
        // ~4 commits in 30 days = ~4 commits/month
        let freq = history.change_frequency();
        assert!(freq > 3.5 && freq < 4.5, "Expected ~4.0, got {freq}");
    }

    #[test]
    fn test_function_history_all_bug_fixes() {
        let history = FunctionHistory {
            total_commits: 5,
            bug_fix_count: 5,
            introduced: Some(Utc::now() - chrono::Duration::days(10)),
            ..Default::default()
        };

        assert_eq!(history.bug_density(), 1.0);
    }

    #[test]
    fn test_function_history_age_days() {
        let ten_days_ago = Utc::now() - chrono::Duration::days(10);
        let history = FunctionHistory {
            introduced: Some(ten_days_ago),
            ..Default::default()
        };

        let age = history.age_days();
        // Allow some tolerance for timing
        assert!(age >= 9 && age <= 11, "Expected ~10 days, got {age}");
    }

    #[test]
    fn test_calculate_function_history() {
        let introduced = Utc::now() - chrono::Duration::days(60);
        let commits = vec![
            CommitInfo {
                hash: "abc123".to_string(),
                date: Some(introduced + chrono::Duration::days(10)),
                message: "fix: first bug".to_string(),
                author: "dev1@example.com".to_string(),
            },
            CommitInfo {
                hash: "def456".to_string(),
                date: Some(introduced + chrono::Duration::days(20)),
                message: "feat: add feature".to_string(),
                author: "dev2@example.com".to_string(),
            },
            CommitInfo {
                hash: "ghi789".to_string(),
                date: Some(introduced + chrono::Duration::days(30)),
                message: "fix: second bug".to_string(),
                author: "dev1@example.com".to_string(),
            },
        ];

        let history =
            calculate_function_history(Some("intro123".to_string()), Some(introduced), &commits);

        assert_eq!(history.introduction_commit, Some("intro123".to_string()));
        assert_eq!(history.total_commits, 3);
        assert_eq!(history.bug_fix_count, 2);
        assert_eq!(history.authors.len(), 2);
        assert!(history.last_modified.is_some());
        assert!(history.introduced.is_some());
        assert!((history.bug_density() - 0.666).abs() < 0.01);
    }
}
