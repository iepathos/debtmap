use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Information about a single commit from git log
#[derive(Debug, Clone)]
struct CommitInfo {
    #[allow(dead_code)]
    hash: String,
    date: DateTime<Utc>,
    message: String,
    author: String,
    files: Vec<FileChange>,
}

/// Information about a file changed in a commit
#[derive(Debug, Clone)]
struct FileChange {
    path: PathBuf,
    additions: usize,
    deletions: usize,
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
    fn calculate_change_frequency(&self) -> f64 {
        let age_days = self.calculate_age_days();
        if age_days > 0 {
            (self.total_commits as f64 / age_days as f64) * 30.0
        } else {
            0.0
        }
    }

    /// Pure function: Calculate file age in days
    fn calculate_age_days(&self) -> u32 {
        self.first_seen
            .map(|first| (Utc::now() - first).num_days().max(0) as u32)
            .unwrap_or(0)
    }

    /// Pure function: Calculate stability score
    fn calculate_stability(&self) -> f64 {
        if self.total_commits == 0 {
            return 1.0; // New file, assume stable
        }

        let age_days = self.calculate_age_days();

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
    /// Create a new batched git history by fetching all data upfront
    /// This is the "imperative shell" - all I/O happens here
    pub fn new(repo_root: &Path) -> Result<Self> {
        let raw_log = Self::fetch_git_log(repo_root)?;
        let commits = Self::parse_log(&raw_log)?;
        let file_histories = Self::build_file_maps(commits);
        Ok(Self { file_histories })
    }

    /// I/O boundary: Fetch comprehensive git log with all commit data
    fn fetch_git_log(repo_root: &Path) -> Result<String> {
        let output = Command::new("git")
            .args([
                "log",
                "--all",
                "--numstat",
                "--format=:::%H:::%cI:::%s:::%ae",
                "HEAD",
            ])
            .current_dir(repo_root)
            .output()
            .context("Failed to fetch git log")?;

        if !output.status.success() {
            anyhow::bail!(
                "Git log command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Pure function: Parse raw git log output into structured commits
    fn parse_log(raw_log: &str) -> Result<Vec<CommitInfo>> {
        let mut commits = Vec::new();
        let mut current_commit: Option<(String, DateTime<Utc>, String, String)> = None;
        let mut current_files = Vec::new();

        for line in raw_log.lines() {
            if line.starts_with(":::") {
                // Save previous commit if it exists
                if let Some((hash, date, message, author)) = current_commit.take() {
                    commits.push(CommitInfo {
                        hash,
                        date,
                        message,
                        author,
                        files: std::mem::take(&mut current_files),
                    });
                }

                // Parse new commit header
                let parts: Vec<&str> = line.split(":::").collect();
                if parts.len() >= 5 {
                    let hash = parts[1].to_string();
                    let date = DateTime::parse_from_rfc3339(parts[2])
                        .context("Failed to parse commit date")?
                        .with_timezone(&Utc);
                    let message = parts[3].to_string();
                    let author = parts[4].to_string();
                    current_commit = Some((hash, date, message, author));
                }
            } else if !line.is_empty() && line.contains('\t') {
                // Parse numstat line: "additions  deletions  path"
                if let Some(file_change) = Self::parse_numstat_line(line) {
                    current_files.push(file_change);
                }
            }
        }

        // Save last commit
        if let Some((hash, date, message, author)) = current_commit {
            commits.push(CommitInfo {
                hash,
                date,
                message,
                author,
                files: current_files,
            });
        }

        Ok(commits)
    }

    /// Pure function: Parse a single numstat line
    fn parse_numstat_line(line: &str) -> Option<FileChange> {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            // Handle binary files (shows "-" instead of numbers)
            let additions = parts[0].parse::<usize>().unwrap_or(0);
            let deletions = parts[1].parse::<usize>().unwrap_or(0);
            let path = PathBuf::from(parts[2]);

            Some(FileChange {
                path,
                additions,
                deletions,
            })
        } else {
            None
        }
    }

    /// Pure function: Build file history maps from commits
    fn build_file_maps(commits: Vec<CommitInfo>) -> HashMap<PathBuf, FileHistoryData> {
        let mut file_histories: HashMap<PathBuf, FileHistoryData> = HashMap::new();

        for commit in commits {
            for file_change in &commit.files {
                let history = file_histories
                    .entry(file_change.path.clone())
                    .or_default();

                let file_churn = file_change.additions + file_change.deletions;
                history.add_commit(&commit, file_churn);
            }
        }

        file_histories
    }

    /// Pure lookup: Get file history (no I/O after construction)
    pub fn get_file_history(&self, path: &Path) -> Option<&FileHistoryData> {
        self.file_histories.get(path)
    }

    /// Calculate metrics for a file
    #[allow(clippy::type_complexity)]
    pub fn calculate_metrics(
        &self,
        path: &Path,
    ) -> Option<(f64, usize, Option<DateTime<Utc>>, usize, f64, usize, u32)> {
        self.get_file_history(path).map(|history| {
            (
                history.calculate_change_frequency(),
                history.bug_fix_count,
                history.last_modified,
                history.authors.len(),
                history.calculate_stability(),
                history.total_commits,
                history.calculate_age_days(),
            )
        })
    }
}

/// Pure function: Determine if a commit message indicates a bug fix
/// Matches the logic from the original implementation
fn is_bug_fix(message: &str) -> bool {
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
    fn test_parse_numstat_line() {
        // Normal file change
        let result = BatchedGitHistory::parse_numstat_line("10\t5\tsrc/main.rs");
        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.additions, 10);
        assert_eq!(change.deletions, 5);
        assert_eq!(change.path, PathBuf::from("src/main.rs"));

        // Binary file (shows "-" instead of numbers)
        let result = BatchedGitHistory::parse_numstat_line("-\t-\timage.png");
        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.additions, 0);
        assert_eq!(change.deletions, 0);
        assert_eq!(change.path, PathBuf::from("image.png"));

        // Invalid line
        let result = BatchedGitHistory::parse_numstat_line("invalid");
        assert!(result.is_none());

        // File with spaces in path
        let result = BatchedGitHistory::parse_numstat_line("5\t3\tsrc/my file.rs");
        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.path, PathBuf::from("src/my file.rs"));
    }

    #[test]
    fn test_file_history_data_accumulation() {
        let mut history = FileHistoryData::default();

        let commit1 = CommitInfo {
            hash: "abc123".to_string(),
            date: DateTime::parse_from_rfc3339("2025-01-01T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            message: "fix: resolve bug".to_string(),
            author: "author1@example.com".to_string(),
            files: vec![],
        };

        let commit2 = CommitInfo {
            hash: "def456".to_string(),
            date: DateTime::parse_from_rfc3339("2025-01-02T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            message: "feat: add feature".to_string(),
            author: "author2@example.com".to_string(),
            files: vec![],
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
    fn test_parse_log_with_multiple_commits() {
        let raw_log = r#":::abc123:::2025-01-01T10:00:00Z:::fix: resolve bug:::author1@example.com
10	5	src/main.rs
5	3	src/lib.rs

:::def456:::2025-01-02T10:00:00Z:::feat: add feature:::author2@example.com
20	0	src/feature.rs
"#;

        let commits = BatchedGitHistory::parse_log(raw_log).unwrap();
        assert_eq!(commits.len(), 2);

        assert_eq!(commits[0].hash, "abc123");
        assert_eq!(commits[0].message, "fix: resolve bug");
        assert_eq!(commits[0].author, "author1@example.com");
        assert_eq!(commits[0].files.len(), 2);

        assert_eq!(commits[1].hash, "def456");
        assert_eq!(commits[1].message, "feat: add feature");
        assert_eq!(commits[1].files.len(), 1);
    }

    #[test]
    fn test_parse_log_with_binary_files() {
        let raw_log = r#":::abc123:::2025-01-01T10:00:00Z:::add image:::author@example.com
-	-	image.png
10	5	src/main.rs
"#;

        let commits = BatchedGitHistory::parse_log(raw_log).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].files.len(), 2);

        // Binary file should have 0 additions/deletions
        assert_eq!(commits[0].files[0].additions, 0);
        assert_eq!(commits[0].files[0].deletions, 0);
    }

    #[test]
    fn test_build_file_maps() {
        let commits = vec![
            CommitInfo {
                hash: "abc123".to_string(),
                date: DateTime::parse_from_rfc3339("2025-01-01T10:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                message: "fix: bug".to_string(),
                author: "author1@example.com".to_string(),
                files: vec![FileChange {
                    path: PathBuf::from("src/main.rs"),
                    additions: 10,
                    deletions: 5,
                }],
            },
            CommitInfo {
                hash: "def456".to_string(),
                date: DateTime::parse_from_rfc3339("2025-01-02T10:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                message: "feat: feature".to_string(),
                author: "author2@example.com".to_string(),
                files: vec![FileChange {
                    path: PathBuf::from("src/main.rs"),
                    additions: 20,
                    deletions: 0,
                }],
            },
        ];

        let file_maps = BatchedGitHistory::build_file_maps(commits);
        assert_eq!(file_maps.len(), 1);

        let main_rs_history = file_maps.get(&PathBuf::from("src/main.rs")).unwrap();
        assert_eq!(main_rs_history.total_commits, 2);
        assert_eq!(main_rs_history.bug_fix_count, 1);
        assert_eq!(main_rs_history.authors.len(), 2);
        assert_eq!(main_rs_history.total_churn, 35); // 10+5+20+0
    }

    #[test]
    fn test_calculate_change_frequency() {
        let ten_days_ago = Utc::now() - chrono::Duration::days(10);
        let history = FileHistoryData {
            total_commits: 10,
            first_seen: Some(ten_days_ago),
            ..Default::default()
        };

        // With 10 days age and 10 commits, expect ~30 commits/month
        let freq = history.calculate_change_frequency();
        assert!(freq > 25.0 && freq < 35.0); // Allow some tolerance for timing
    }

    #[test]
    fn test_calculate_stability_new_file() {
        let history = FileHistoryData::default();
        let stability = history.calculate_stability();
        assert_eq!(stability, 1.0); // New file assumed stable
    }

    #[test]
    fn test_calculate_stability_with_commits() {
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

        let stability = history.calculate_stability();
        // Should be between 0 and 1
        assert!((0.0..=1.0).contains(&stability));
    }
}
