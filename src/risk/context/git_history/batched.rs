use super::git2_provider::FileCommitScan;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Commit metadata accumulated into per-file history during preload.
#[derive(Debug, Clone)]
struct CommitInfo {
    date: DateTime<Utc>,
    message: String,
    author: String,
}

/// Accumulated history for a single file
#[derive(Debug, Clone, Default)]
pub(super) struct FileHistoryData {
    total_commits: usize,
    bug_fix_count: usize,
    authors: HashSet<String>,
    last_modified: Option<DateTime<Utc>>,
    first_seen: Option<DateTime<Utc>>,
    total_churn: usize,
}

impl FileHistoryData {
    /// Pure function: Accumulate commit data into file history
    fn add_commit(&mut self, commit: &CommitInfo, file_churn: usize) {
        self.total_commits += 1;

        if is_bug_fix(&commit.message) {
            self.bug_fix_count += 1;
        }

        self.authors.insert(commit.author.clone());

        self.last_modified = Some(
            self.last_modified
                .map(|d| d.max(commit.date))
                .unwrap_or(commit.date),
        );

        self.first_seen = Some(
            self.first_seen
                .map(|d| d.min(commit.date))
                .unwrap_or(commit.date),
        );

        self.total_churn += file_churn;
    }

    /// Pure function: Calculate change frequency (commits per month)
    fn calculate_change_frequency(&self, now: DateTime<Utc>) -> f64 {
        let age_days = self.calculate_age_days(now);
        if age_days > 0 {
            (self.total_commits as f64 / age_days as f64) * 30.0
        } else {
            0.0
        }
    }

    /// Pure function: Calculate file age in days
    fn calculate_age_days(&self, now: DateTime<Utc>) -> u32 {
        self.first_seen
            .map(|first| now.signed_duration_since(first).num_days().max(0) as u32)
            .unwrap_or(0)
    }

    /// Pure function: Calculate stability score
    fn calculate_stability(&self, now: DateTime<Utc>) -> f64 {
        if self.total_commits == 0 {
            return 1.0; // New file, assume stable
        }

        let age_days = self.calculate_age_days(now);

        let churn_factor = if age_days > 0 {
            let monthly_churn = (self.total_commits as f64) / (age_days as f64) * 30.0;
            1.0 / (1.0 + monthly_churn)
        } else {
            0.5
        };

        let bug_factor = 1.0 - (self.bug_fix_count as f64 / self.total_commits as f64).min(1.0);
        let age_factor = (age_days as f64 / 365.0).min(1.0); // Max out at 1 year

        // Weighted average
        (churn_factor * 0.4 + bug_factor * 0.4 + age_factor * 0.2).min(1.0)
    }
}

/// Batched git history provider that fetches all history upfront
pub struct BatchedGitHistory {
    file_histories: HashMap<PathBuf, FileHistoryData>,
}

impl BatchedGitHistory {
    /// Create batched file history from commit scans produced by one repo walk.
    pub fn from_commit_scans(scans: &[FileCommitScan]) -> Self {
        let mut file_histories: HashMap<PathBuf, FileHistoryData> = HashMap::new();
        for scan in scans {
            let commit_info = CommitInfo {
                date: scan.date,
                message: scan.message.clone(),
                author: scan.author_email.clone(),
            };
            for (path, churn) in &scan.file_churn {
                file_histories
                    .entry(path.clone())
                    .or_default()
                    .add_commit(&commit_info, *churn);
            }
        }
        Self { file_histories }
    }

    /// Pure lookup: Get file history (no I/O after construction)
    fn get_file_history(&self, path: &Path) -> Option<&FileHistoryData> {
        self.file_histories.get(path)
    }

    /// Get all file paths stored in the history (for debugging/testing)
    #[cfg(test)]
    pub fn all_paths(&self) -> Vec<&PathBuf> {
        self.file_histories.keys().collect()
    }

    /// Check if a path exists in the history (for debugging/testing)
    #[cfg(test)]
    pub fn has_path(&self, path: &Path) -> bool {
        self.file_histories.contains_key(path)
    }

    /// Calculate metrics for a file
    #[allow(clippy::type_complexity)]
    pub fn calculate_metrics(
        &self,
        path: &Path,
        now: DateTime<Utc>,
    ) -> Option<(f64, usize, Option<DateTime<Utc>>, usize, f64, usize, u32)> {
        self.get_file_history(path).map(|history| {
            (
                history.calculate_change_frequency(now),
                history.bug_fix_count,
                history.last_modified,
                history.authors.len(),
                history.calculate_stability(now),
                history.total_commits,
                history.calculate_age_days(now),
            )
        })
    }
}

/// Pure function: Determine if a commit message indicates a bug fix
/// Matches the logic from the original implementation
pub fn is_bug_fix(message: &str) -> bool {
    if is_excluded_commit(message) {
        return false;
    }

    let lowercase = message.to_lowercase();
    let words: Vec<&str> = lowercase.split(|c: char| !c.is_alphanumeric()).collect();

    // Check for bug fix keywords as standalone words
    words.iter().any(|&word| {
        matches!(
            word,
            "bug" | "fix" | "fixes" | "fixed" | "fixing" | "hotfix"
        )
    })
}

/// Pure function: Determine if a commit should be excluded from bug fix counting
/// This matches the exact logic from git_history.rs
fn is_excluded_commit(commit_line: &str) -> bool {
    let lowercase = commit_line.to_lowercase();

    // Conventional commit type exclusions
    if lowercase.contains("style:")
        || lowercase.contains("chore:")
        || lowercase.contains("docs:")
        || lowercase.contains("test:")
    {
        return true;
    }

    // Maintenance keyword exclusions
    let exclusion_keywords = ["formatting", "linting", "whitespace", "typo"];

    for keyword in &exclusion_keywords {
        if lowercase.contains(keyword) {
            return true;
        }
    }

    // Refactoring exclusion (unless it mentions bug-related keywords)
    if lowercase.contains("refactor:") {
        let words: Vec<&str> = lowercase.split(|c: char| !c.is_alphanumeric()).collect();

        let has_bug_keyword = words.iter().any(|&word| {
            matches!(
                word,
                "bug" | "fix" | "fixes" | "fixed" | "fixing" | "issue" | "hotfix"
            )
        });

        // Exclude refactor commits that don't mention bug-related keywords
        if !has_bug_keyword {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_bug_fix() {
        // Should match: standalone bug fix keywords
        assert!(is_bug_fix("fix: resolve login bug"));
        assert!(is_bug_fix("Fixed the payment issue"));
        assert!(is_bug_fix("Bug fix for issue #123"));
        assert!(is_bug_fix("hotfix: urgent fix"));
        assert!(is_bug_fix("fixes issue with validation"));

        // Should NOT match: excluded commits
        assert!(!is_bug_fix("style: apply formatting fixes"));
        assert!(!is_bug_fix("chore: update dependencies"));
        assert!(!is_bug_fix("docs: fix typo"));
        assert!(!is_bug_fix("refactor: improve prefix handling"));

        // Should NOT match: false positives from substring
        assert!(!is_bug_fix("Add debugging utilities"));
        assert!(!is_bug_fix("update: add fixture for testing"));

        // Should match: refactor with bug mention
        assert!(is_bug_fix("refactor: fix memory leak"));
    }

    #[test]
    fn test_is_excluded_commit() {
        // Should exclude: conventional commit types
        assert!(is_excluded_commit("style: apply formatting fixes"));
        assert!(is_excluded_commit("chore: update dependencies"));
        assert!(is_excluded_commit("docs: fix typo"));
        assert!(is_excluded_commit("test: add unit tests"));

        // Should exclude: maintenance keywords
        assert!(is_excluded_commit("refactor: improve prefix handling"));
        assert!(is_excluded_commit("apply linting rules"));
        assert!(is_excluded_commit("remove whitespace"));

        // Should NOT exclude: genuine bug fixes
        assert!(!is_excluded_commit("fix: resolve login bug"));
        assert!(!is_excluded_commit("Fixed the payment issue"));
        assert!(!is_excluded_commit("refactor: fix memory leak"));
    }

    #[test]
    fn test_file_history_data_accumulation() {
        let mut history = FileHistoryData::default();

        let commit1 = CommitInfo {
            date: DateTime::parse_from_rfc3339("2025-01-01T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            message: "fix: resolve bug".to_string(),
            author: "author1@example.com".to_string(),
        };

        let commit2 = CommitInfo {
            date: DateTime::parse_from_rfc3339("2025-01-02T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            message: "feat: add feature".to_string(),
            author: "author2@example.com".to_string(),
        };

        history.add_commit(&commit1, 15);
        history.add_commit(&commit2, 10);

        assert_eq!(history.total_commits, 2);
        assert_eq!(history.bug_fix_count, 1);
        assert_eq!(history.authors.len(), 2);
        assert_eq!(history.total_churn, 25);
        assert!(history.last_modified.is_some());
        assert!(history.first_seen.is_some());
    }

    #[test]
    fn test_calculate_change_frequency() {
        let now = Utc::now();
        let ten_days_ago = now - chrono::Duration::days(10);
        let history = FileHistoryData {
            total_commits: 10,
            first_seen: Some(ten_days_ago),
            ..Default::default()
        };

        // With 10 days age and 10 commits, expect ~30 commits/month
        let freq = history.calculate_change_frequency(now);
        assert!(freq > 25.0 && freq < 35.0); // Allow some tolerance for timing
    }

    #[test]
    fn test_calculate_stability_new_file() {
        let now = Utc::now();
        let history = FileHistoryData::default();
        let stability = history.calculate_stability(now);
        assert_eq!(stability, 1.0); // New file assumed stable
    }

    #[test]
    fn test_calculate_stability_with_commits() {
        let now = Utc::now();
        let history = FileHistoryData {
            total_commits: 10,
            bug_fix_count: 2,
            first_seen: Some(
                DateTime::parse_from_rfc3339("2024-01-01T10:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            ..Default::default()
        };

        let stability = history.calculate_stability(now);
        // Should be between 0 and 1
        assert!((0.0..=1.0).contains(&stability));
    }
}
