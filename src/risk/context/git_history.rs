use super::{AnalysisTarget, Context, ContextDetails, ContextProvider};
use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

mod batched;
mod function_level;

/// File history information from Git
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHistory {
    pub change_frequency: f64,
    pub bug_fix_count: usize,
    pub last_modified: Option<DateTime<Utc>>,
    pub author_count: usize,
    pub stability_score: f64,
    pub total_commits: usize,
    pub age_days: u32,
}

/// Provider for Git history context with lock-free caching
pub struct GitHistoryProvider {
    repo_root: PathBuf,
    cache: Arc<DashMap<PathBuf, FileHistory>>,
    batched_history: Option<batched::BatchedGitHistory>,
}

impl GitHistoryProvider {
    pub fn new(repo_root: PathBuf) -> Result<Self> {
        // Verify this is a git repository
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--git-dir")
            .current_dir(&repo_root)
            .output()
            .context("Failed to verify git repository")?;

        if !output.status.success() {
            anyhow::bail!("Not a git repository: {}", repo_root.display());
        }

        // Create batched history (fetch all git data upfront)
        let batched_history = match batched::BatchedGitHistory::new(&repo_root) {
            Ok(history) => {
                log::debug!("Batched git history loaded successfully");
                Some(history)
            }
            Err(e) => {
                log::warn!(
                    "Failed to load batched git history, will fall back to direct queries: {}",
                    e
                );
                None
            }
        };

        Ok(Self {
            repo_root,
            cache: Arc::new(DashMap::new()),
            batched_history,
        })
    }

    /// Get file history from cache or fetch it (immutable, thread-safe)
    fn get_or_fetch_history(&self, path: &Path) -> Result<FileHistory> {
        // Try cache first (lock-free read)
        if let Some(cached) = self.cache.get(path) {
            return Ok(cached.clone());
        }

        // Try batched history (fast O(1) HashMap lookup)
        if let Some(ref batched) = self.batched_history {
            if let Some((
                change_frequency,
                bug_fix_count,
                last_modified,
                author_count,
                stability_score,
                total_commits,
                age_days,
            )) = batched.calculate_metrics(path)
            {
                let history = FileHistory {
                    change_frequency,
                    bug_fix_count,
                    last_modified,
                    author_count,
                    stability_score,
                    total_commits,
                    age_days,
                };
                // Cache for future lookups (lock-free write)
                self.cache.insert(path.to_path_buf(), history.clone());
                return Ok(history);
            }
        }

        // Fallback to direct git queries (slow path)
        let history = self.fetch_history_direct(path)?;
        self.cache.insert(path.to_path_buf(), history.clone());
        Ok(history)
    }

    /// Fetch file history directly via git commands (fallback when batched fails)
    fn fetch_history_direct(&self, path: &Path) -> Result<FileHistory> {
        Ok(FileHistory {
            change_frequency: self.calculate_churn_rate(path)?,
            bug_fix_count: self.count_bug_fixes(path)?,
            last_modified: self.get_last_modified(path)?,
            author_count: self.count_unique_authors(path)?,
            stability_score: self.calculate_stability(path)?,
            total_commits: self.count_commits(path)?,
            age_days: self.get_file_age_days(path)?,
        })
    }

    /// Legacy mutable API - kept for backward compatibility
    pub fn analyze_file(&mut self, path: &Path) -> Result<FileHistory> {
        self.get_or_fetch_history(path)
    }

    fn calculate_churn_rate(&self, path: &Path) -> Result<f64> {
        let commits = self.count_commits(path)?;
        let age_days = self.get_file_age_days(path)?;

        if age_days > 0 {
            Ok((commits as f64) / (age_days as f64) * 30.0) // Monthly rate
        } else {
            Ok(0.0)
        }
    }

    fn count_commits(&self, path: &Path) -> Result<usize> {
        let output = Command::new("git")
            .args([
                "rev-list",
                "--count",
                "HEAD",
                "--",
                path.to_str().unwrap_or(""),
            ])
            .current_dir(&self.repo_root)
            .output()
            .context("Failed to count commits")?;

        if output.status.success() {
            let count_str = String::from_utf8_lossy(&output.stdout);
            Ok(count_str.trim().parse().unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    /// Counts bug fix commits for a file using word boundary matching
    /// to reduce false positives from substring matches like "prefix" or "debug".
    ///
    /// Matches patterns:
    /// - `\bfix\b`, `\bfixes\b`, `\bfixed\b`, `\bfixing\b` (matches "fix" but not "prefix")
    /// - `\bbug\b` (matches "bug" but not "debug")
    /// - `\bhotfix\b` (emergency fixes)
    ///
    /// Excludes non-bug commits via `is_excluded_commit` filter:
    /// - Styling commits (style:, formatting, linting)
    /// - Maintenance (chore:, whitespace, typo)
    /// - Documentation (docs:)
    /// - Tests (test:)
    /// - Refactoring without bug mentions
    fn count_bug_fixes(&self, path: &Path) -> Result<usize> {
        let output = Command::new("git")
            .args([
                "log",
                "--oneline",
                "--grep=\\bfix\\b",
                "--grep=\\bfixes\\b",
                "--grep=\\bfixed\\b",
                "--grep=\\bfixing\\b",
                "--grep=\\bbug\\b",
                "--grep=\\bhotfix\\b",
                "-i", // Case-insensitive matching
                "--",
                path.to_str().unwrap_or(""),
            ])
            .current_dir(&self.repo_root)
            .output()
            .context("Failed to count bug fixes")?;

        if output.status.success() {
            let lines = String::from_utf8_lossy(&output.stdout);
            let count = lines
                .lines()
                .filter(|line| !Self::is_excluded_commit(line))
                .count();
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Determines if a commit message indicates a non-bug change that should
    /// be excluded from bug fix counting.
    ///
    /// Excludes:
    /// - Conventional commit types: style, chore, docs, test
    /// - Maintenance keywords: formatting, linting, whitespace, typo
    /// - Refactoring without bug mentions
    ///
    /// # Examples
    ///
    /// ```ignore
    /// assert!(is_excluded_commit("style: apply formatting fixes"));  // Excluded
    /// assert!(is_excluded_commit("chore: update dependencies"));     // Excluded
    /// assert!(!is_excluded_commit("fix: resolve login bug"));        // Not excluded
    /// assert!(!is_excluded_commit("refactor: fix memory leak"));     // Not excluded (mentions fix)
    /// ```
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
        // Check for standalone words that indicate actual bug fixes
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

    fn get_last_modified(&self, path: &Path) -> Result<Option<DateTime<Utc>>> {
        let output = Command::new("git")
            .args([
                "log",
                "-1",
                "--format=%cI",
                "--",
                path.to_str().unwrap_or(""),
            ])
            .current_dir(&self.repo_root)
            .output()
            .context("Failed to get last modified date")?;

        if output.status.success() {
            let date_str = String::from_utf8_lossy(&output.stdout);
            let date_str = date_str.trim();
            if !date_str.is_empty() {
                match DateTime::parse_from_rfc3339(date_str) {
                    Ok(dt) => Ok(Some(dt.with_timezone(&Utc))),
                    Err(_) => Ok(None),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn count_unique_authors(&self, path: &Path) -> Result<usize> {
        let output = Command::new("git")
            .args(["log", "--format=%ae", "--", path.to_str().unwrap_or("")])
            .current_dir(&self.repo_root)
            .output()
            .context("Failed to count authors")?;

        if output.status.success() {
            let authors = String::from_utf8_lossy(&output.stdout);
            let unique_authors: std::collections::HashSet<_> = authors.lines().collect();
            Ok(unique_authors.len())
        } else {
            Ok(0)
        }
    }

    fn get_file_age_days(&self, path: &Path) -> Result<u32> {
        let output = Command::new("git")
            .args([
                "log",
                "--reverse",
                "--format=%cI",
                "--",
                path.to_str().unwrap_or(""),
            ])
            .current_dir(&self.repo_root)
            .output()
            .context("Failed to get file age")?;

        if output.status.success() {
            let dates = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = dates.lines().next() {
                if let Ok(first_date) = DateTime::parse_from_rfc3339(first_line.trim()) {
                    let now = Utc::now();
                    let age = now.signed_duration_since(first_date.with_timezone(&Utc));
                    return Ok(age.num_days().max(0) as u32);
                }
            }
        }

        Ok(0)
    }

    fn calculate_stability(&self, path: &Path) -> Result<f64> {
        let age_days = self.get_file_age_days(path)?;
        let commits = self.count_commits(path)?;
        let bug_fixes = self.count_bug_fixes(path)?;

        if commits == 0 {
            return Ok(1.0); // New file, assume stable
        }

        // Stability factors:
        // - Lower churn rate is more stable
        // - Fewer bug fixes relative to commits is more stable
        // - Older files with fewer recent changes are more stable

        let churn_factor = if age_days > 0 {
            let monthly_churn = (commits as f64) / (age_days as f64) * 30.0;
            1.0 / (1.0 + monthly_churn)
        } else {
            0.5
        };

        let bug_factor = 1.0 - (bug_fixes as f64 / commits as f64).min(1.0);
        let age_factor = (age_days as f64 / 365.0).min(1.0); // Max out at 1 year

        // Weighted average
        Ok((churn_factor * 0.4 + bug_factor * 0.4 + age_factor * 0.2).min(1.0))
    }
}

impl ContextProvider for GitHistoryProvider {
    fn name(&self) -> &str {
        "git_history"
    }

    fn gather(&self, target: &AnalysisTarget) -> Result<Context> {
        // Try function-level analysis if function name is provided
        if !target.function_name.is_empty() {
            match self.gather_for_function(target) {
                Ok(context) => return Ok(context),
                Err(e) => {
                    log::debug!(
                        "Function-level git analysis failed for '{}', falling back to file-level: {}",
                        target.function_name,
                        e
                    );
                }
            }
        }

        // Fall back to file-level analysis
        self.gather_for_file(target)
    }

    fn weight(&self) -> f64 {
        1.0 // Historical context has moderate weight
    }

    fn explain(&self, context: &Context) -> String {
        match &context.details {
            ContextDetails::Historical {
                change_frequency,
                bug_density,
                age_days,
                author_count,
            } => self.explain_historical_context(
                *change_frequency,
                *bug_density,
                (*age_days).into(),
                *author_count,
            ),
            _ => "No historical information".to_string(),
        }
    }
}

impl GitHistoryProvider {
    /// Gather context using function-level git history analysis
    ///
    /// Uses `git log -S` to track when the function was introduced and
    /// count only commits that modified that specific function.
    /// Uses `git blame` on current lines to identify contributors.
    fn gather_for_function(&self, target: &AnalysisTarget) -> Result<Context> {
        let history = function_level::get_function_history(
            &self.repo_root,
            &target.file_path,
            &target.function_name,
            target.line_range,
        )?;

        let contribution =
            Self::classify_risk_contribution(history.change_frequency(), history.bug_density());

        Ok(Context {
            provider: self.name().to_string(),
            weight: self.weight(),
            contribution,
            details: ContextDetails::Historical {
                change_frequency: history.change_frequency(),
                bug_density: history.bug_density(),
                age_days: history.age_days(),
                author_count: history.authors.len(),
            },
        })
    }

    /// Gather context using file-level git history analysis (fallback)
    fn gather_for_file(&self, target: &AnalysisTarget) -> Result<Context> {
        // Use cached/batched history (O(1) lookup, no git subprocess calls)
        let history = self.get_or_fetch_history(&target.file_path)?;

        // Calculate contribution based on instability
        let bug_density = Self::calculate_bug_density(history.bug_fix_count, history.total_commits);

        let contribution = Self::classify_risk_contribution(history.change_frequency, bug_density);

        Ok(Context {
            provider: self.name().to_string(),
            weight: self.weight(),
            contribution,
            details: ContextDetails::Historical {
                change_frequency: history.change_frequency,
                bug_density,
                age_days: history.age_days,
                author_count: history.author_count,
            },
        })
    }

    /// Calculate bug density as a ratio of bug fixes to total commits
    fn calculate_bug_density(bug_fix_count: usize, total_commits: usize) -> f64 {
        if total_commits > 0 {
            bug_fix_count as f64 / total_commits as f64
        } else {
            0.0
        }
    }

    /// Classify the risk contribution based on change frequency and bug density
    ///
    /// Uses continuous scoring instead of discrete thresholds for more accurate
    /// differentiation between risk levels.
    ///
    /// # Scoring model
    /// - **Bug density** (primary signal): scales linearly from 0 to 1.5
    ///   - 0% bugs → 0.0 contribution
    ///   - 50% bugs → 0.75 contribution
    ///   - 100% bugs → 1.5 contribution
    /// - **Change frequency** (secondary signal): scales from 0 to 0.5, saturates at 10/month
    ///   - 0/month → 0.0
    ///   - 5/month → 0.25
    ///   - 10+/month → 0.5
    ///
    /// Total is capped at 2.0 to prevent excessive score amplification.
    /// Stable code with no bugs and no changes contributes 0.0 (no risk increase).
    fn classify_risk_contribution(change_frequency: f64, bug_density: f64) -> f64 {
        let bug_contribution = bug_density * 1.5;
        let freq_contribution = (change_frequency / 20.0).min(0.5);

        (bug_contribution + freq_contribution).min(2.0)
    }

    fn explain_historical_context(
        &self,
        change_frequency: f64,
        bug_density: f64,
        age_days: u64,
        author_count: usize,
    ) -> String {
        let stability_status =
            self.determine_stability_status(change_frequency, bug_density, age_days);
        self.format_stability_message(
            stability_status,
            change_frequency,
            bug_density,
            age_days,
            author_count,
        )
    }

    fn determine_stability_status(
        &self,
        change_frequency: f64,
        bug_density: f64,
        age_days: u64,
    ) -> StabilityStatus {
        // Use pattern matching with early returns to reduce cognitive complexity
        match (change_frequency, bug_density, age_days) {
            (freq, bug, _) if freq > 5.0 && bug > 0.3 => StabilityStatus::HighlyUnstable,
            (freq, _, _) if freq > 2.0 => StabilityStatus::FrequentlyChanged,
            (_, bug, _) if bug > 0.2 => StabilityStatus::BugProne,
            (_, _, age) if age > 365 => StabilityStatus::MatureStable,
            _ => StabilityStatus::RelativelyStable,
        }
    }

    fn format_stability_message(
        &self,
        status: StabilityStatus,
        change_frequency: f64,
        bug_density: f64,
        age_days: u64,
        author_count: usize,
    ) -> String {
        match status {
            StabilityStatus::HighlyUnstable => format!(
                "Highly unstable: {:.1} changes/month, {:.0}% bug fixes",
                change_frequency,
                bug_density * 100.0
            ),
            StabilityStatus::FrequentlyChanged => format!(
                "Frequently changed: {change_frequency:.1} changes/month by {author_count} authors"
            ),
            StabilityStatus::BugProne => format!(
                "Bug-prone: {:.0}% of commits are bug fixes",
                bug_density * 100.0
            ),
            StabilityStatus::MatureStable => format!("Mature and stable: {age_days} days old"),
            StabilityStatus::RelativelyStable => {
                format!("Relatively stable: {change_frequency:.1} changes/month")
            }
        }
    }
}

enum StabilityStatus {
    HighlyUnstable,
    FrequentlyChanged,
    BugProne,
    MatureStable,
    RelativelyStable,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_repo() -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .current_dir(&repo_path)
            .output()?;

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()?;

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()?;

        Ok((temp_dir, repo_path))
    }

    fn create_test_file(repo_path: &Path, file_name: &str, content: &str) -> Result<PathBuf> {
        let file_path = repo_path.join(file_name);
        std::fs::write(&file_path, content)?;

        Command::new("git")
            .args(["add", file_name])
            .current_dir(repo_path)
            .output()?;

        Ok(file_path)
    }

    fn commit_with_message(repo_path: &Path, message: &str) -> Result<()> {
        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(repo_path)
            .output()?;

        Ok(())
    }

    fn modify_and_commit(
        repo_path: &Path,
        file_name: &str,
        content: &str,
        message: &str,
    ) -> Result<()> {
        let file_path = repo_path.join(file_name);
        std::fs::write(&file_path, content)?;

        Command::new("git")
            .args(["add", file_name])
            .current_dir(repo_path)
            .output()?;

        commit_with_message(repo_path, message)?;

        Ok(())
    }

    #[test]
    fn test_is_excluded_commit() {
        // Should exclude: conventional commit types
        assert!(GitHistoryProvider::is_excluded_commit(
            "style: apply formatting fixes"
        ));
        assert!(GitHistoryProvider::is_excluded_commit(
            "chore: update dependencies"
        ));
        assert!(GitHistoryProvider::is_excluded_commit("docs: fix typo"));
        assert!(GitHistoryProvider::is_excluded_commit(
            "test: add unit tests"
        ));

        // Should exclude: maintenance keywords
        assert!(GitHistoryProvider::is_excluded_commit(
            "refactor: improve prefix handling"
        ));
        assert!(GitHistoryProvider::is_excluded_commit(
            "8c45a3c5 style: apply automated formatting"
        ));
        assert!(GitHistoryProvider::is_excluded_commit(
            "apply linting rules"
        ));
        assert!(GitHistoryProvider::is_excluded_commit("remove whitespace"));
        assert!(GitHistoryProvider::is_excluded_commit(
            "fix: correct typo in documentation"
        ));

        // Should NOT exclude: genuine bug fixes
        assert!(!GitHistoryProvider::is_excluded_commit(
            "fix: resolve login bug"
        ));
        assert!(!GitHistoryProvider::is_excluded_commit(
            "Fixed the payment issue"
        ));
        assert!(!GitHistoryProvider::is_excluded_commit(
            "Bug fix for issue #123"
        ));
        assert!(!GitHistoryProvider::is_excluded_commit(
            "hotfix: urgent fix"
        ));

        // Should NOT exclude: refactor that mentions bug/issue
        assert!(!GitHistoryProvider::is_excluded_commit(
            "refactor: fix memory leak"
        ));
        assert!(!GitHistoryProvider::is_excluded_commit(
            "refactor: resolve issue #456"
        ));

        // Edge cases: case insensitivity
        assert!(GitHistoryProvider::is_excluded_commit(
            "STYLE: Apply Formatting"
        ));
        assert!(!GitHistoryProvider::is_excluded_commit("FIX: Resolve Bug"));
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
    fn test_calculate_bug_density_with_commits() {
        assert_eq!(GitHistoryProvider::calculate_bug_density(0, 10), 0.0);
        assert_eq!(GitHistoryProvider::calculate_bug_density(5, 10), 0.5);
        assert_eq!(GitHistoryProvider::calculate_bug_density(10, 10), 1.0);
        assert_eq!(GitHistoryProvider::calculate_bug_density(3, 10), 0.3);
    }

    #[test]
    fn test_calculate_bug_density_no_commits() {
        assert_eq!(GitHistoryProvider::calculate_bug_density(0, 0), 0.0);
        assert_eq!(GitHistoryProvider::calculate_bug_density(5, 0), 0.0);
    }

    #[test]
    fn test_determine_stability_status_highly_unstable() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        // Test highly unstable: high change frequency AND high bug density
        let status = provider.determine_stability_status(6.0, 0.4, 100);
        assert!(matches!(status, StabilityStatus::HighlyUnstable));

        // Edge case: exactly at thresholds
        let status = provider.determine_stability_status(5.1, 0.31, 100);
        assert!(matches!(status, StabilityStatus::HighlyUnstable));

        Ok(())
    }

    #[test]
    fn test_determine_stability_status_frequently_changed() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        // Test frequently changed: high change frequency but low bug density
        let status = provider.determine_stability_status(3.0, 0.1, 100);
        assert!(matches!(status, StabilityStatus::FrequentlyChanged));

        // Edge case: just above threshold
        let status = provider.determine_stability_status(2.1, 0.05, 50);
        assert!(matches!(status, StabilityStatus::FrequentlyChanged));

        Ok(())
    }

    #[test]
    fn test_determine_stability_status_bug_prone() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        // Test bug prone: high bug density but low change frequency
        let status = provider.determine_stability_status(1.0, 0.25, 100);
        assert!(matches!(status, StabilityStatus::BugProne));

        // Edge case: just above bug density threshold
        let status = provider.determine_stability_status(0.5, 0.21, 200);
        assert!(matches!(status, StabilityStatus::BugProne));

        Ok(())
    }

    #[test]
    fn test_determine_stability_status_mature_stable() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        // Test mature stable: old code with low change frequency and bug density
        let status = provider.determine_stability_status(0.5, 0.1, 400);
        assert!(matches!(status, StabilityStatus::MatureStable));

        // Edge case: exactly 366 days old
        let status = provider.determine_stability_status(1.0, 0.15, 366);
        assert!(matches!(status, StabilityStatus::MatureStable));

        Ok(())
    }

    #[test]
    fn test_determine_stability_status_relatively_stable() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        // Test relatively stable: doesn't meet any special criteria
        let status = provider.determine_stability_status(1.5, 0.15, 200);
        assert!(matches!(status, StabilityStatus::RelativelyStable));

        // Edge case: just below all thresholds
        let status = provider.determine_stability_status(2.0, 0.2, 365);
        assert!(matches!(status, StabilityStatus::RelativelyStable));

        Ok(())
    }

    #[test]
    fn test_determine_stability_status_edge_cases() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        // Test priority: highly unstable takes precedence
        let status = provider.determine_stability_status(6.0, 0.4, 400);
        assert!(matches!(status, StabilityStatus::HighlyUnstable));

        // Test zero values
        let status = provider.determine_stability_status(0.0, 0.0, 0);
        assert!(matches!(status, StabilityStatus::RelativelyStable));

        // Test boundary values for frequently changed vs bug prone
        let status = provider.determine_stability_status(2.5, 0.25, 100);
        assert!(matches!(status, StabilityStatus::FrequentlyChanged)); // freq change takes precedence

        Ok(())
    }

    #[test]
    fn test_classify_risk_contribution_continuous_scaling() {
        // Test that contribution scales continuously with bug density
        // Formula: bug_density * 1.5 + min(freq/20, 0.5)

        // Stable: no bugs, no changes → zero contribution (no risk increase)
        let stable = GitHistoryProvider::classify_risk_contribution(0.0, 0.0);
        assert!((stable - 0.0).abs() < 0.001, "Expected 0.0, got {stable}");

        // Low bug density (25%)
        let low_bugs = GitHistoryProvider::classify_risk_contribution(0.0, 0.25);
        assert!(
            (low_bugs - 0.375).abs() < 0.001,
            "Expected 0.375, got {low_bugs}"
        );

        // Medium bug density (50%)
        let medium_bugs = GitHistoryProvider::classify_risk_contribution(0.0, 0.5);
        assert!(
            (medium_bugs - 0.75).abs() < 0.001,
            "Expected 0.75, got {medium_bugs}"
        );

        // High bug density (100%)
        let high_bugs = GitHistoryProvider::classify_risk_contribution(0.0, 1.0);
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
        let high_freq = GitHistoryProvider::classify_risk_contribution(10.0, 0.0);
        assert!(
            (high_freq - 0.5).abs() < 0.001,
            "Expected 0.5, got {high_freq}"
        );

        // Medium frequency (5/month)
        let medium_freq = GitHistoryProvider::classify_risk_contribution(5.0, 0.0);
        assert!(
            (medium_freq - 0.25).abs() < 0.001,
            "Expected 0.25, got {medium_freq}"
        );

        // Frequency contribution saturates at 10/month
        let very_high_freq = GitHistoryProvider::classify_risk_contribution(20.0, 0.0);
        assert!(
            (very_high_freq - 0.5).abs() < 0.001,
            "Expected 0.5 (saturated), got {very_high_freq}"
        );
    }

    #[test]
    fn test_classify_risk_contribution_combined() {
        // Test combined effect of bugs and frequency

        // User's example: 25% bugs, 4.53 changes/month
        let example_low = GitHistoryProvider::classify_risk_contribution(4.53, 0.25);
        // bugs(0.375) + freq(0.2265) = 0.6015
        assert!(
            (example_low - 0.6015).abs() < 0.01,
            "Expected ~0.60, got {example_low}"
        );

        // User's example: 100% bugs, 0.59 changes/month
        let example_high = GitHistoryProvider::classify_risk_contribution(0.59, 1.0);
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
        let extreme = GitHistoryProvider::classify_risk_contribution(100.0, 1.5);
        assert!(
            (extreme - 2.0).abs() < 0.001,
            "Expected 2.0 (capped), got {extreme}"
        );
    }

    #[test]
    fn test_format_stability_message_highly_unstable() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        let message =
            provider.format_stability_message(StabilityStatus::HighlyUnstable, 8.5, 0.45, 180, 5);

        assert_eq!(message, "Highly unstable: 8.5 changes/month, 45% bug fixes");

        // Test with different values
        let message =
            provider.format_stability_message(StabilityStatus::HighlyUnstable, 12.3, 0.67, 90, 10);

        assert_eq!(
            message,
            "Highly unstable: 12.3 changes/month, 67% bug fixes"
        );

        Ok(())
    }

    #[test]
    fn test_format_stability_message_frequently_changed() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        let message = provider.format_stability_message(
            StabilityStatus::FrequentlyChanged,
            3.5,
            0.15,
            200,
            7,
        );

        assert_eq!(
            message,
            "Frequently changed: 3.5 changes/month by 7 authors"
        );

        // Test with single author
        let message = provider.format_stability_message(
            StabilityStatus::FrequentlyChanged,
            5.2,
            0.08,
            100,
            1,
        );

        assert_eq!(
            message,
            "Frequently changed: 5.2 changes/month by 1 authors"
        );

        Ok(())
    }

    #[test]
    fn test_format_stability_message_bug_prone() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        let message =
            provider.format_stability_message(StabilityStatus::BugProne, 1.2, 0.35, 150, 3);

        assert_eq!(message, "Bug-prone: 35% of commits are bug fixes");

        // Test with different bug density
        let message =
            provider.format_stability_message(StabilityStatus::BugProne, 0.8, 0.72, 300, 2);

        assert_eq!(message, "Bug-prone: 72% of commits are bug fixes");

        Ok(())
    }

    #[test]
    fn test_format_stability_message_mature_stable() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        let message =
            provider.format_stability_message(StabilityStatus::MatureStable, 0.5, 0.05, 730, 2);

        assert_eq!(message, "Mature and stable: 730 days old");

        // Test with different age
        let message =
            provider.format_stability_message(StabilityStatus::MatureStable, 0.3, 0.02, 1095, 1);

        assert_eq!(message, "Mature and stable: 1095 days old");

        Ok(())
    }

    #[test]
    fn test_format_stability_message_relatively_stable() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;
        let provider = GitHistoryProvider::new(repo_path)?;

        let message =
            provider.format_stability_message(StabilityStatus::RelativelyStable, 1.8, 0.12, 250, 4);

        assert_eq!(message, "Relatively stable: 1.8 changes/month");

        // Test with different change frequency
        let message =
            provider.format_stability_message(StabilityStatus::RelativelyStable, 0.2, 0.0, 30, 1);

        assert_eq!(message, "Relatively stable: 0.2 changes/month");

        Ok(())
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
            GitHistoryProvider::calculate_bug_density(history.bug_fix_count, history.total_commits);
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
        let history = function_level::get_function_history(
            &repo_path,
            Path::new("test.rs"),
            "my_func",
            (1, 10),
        )?;

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
            history.change_frequency(),
            0.0,
            "my_func should have 0 change frequency"
        );

        Ok(())
    }

    /// Test that function-level analysis correctly counts modifications
    /// to a specific function.
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

        let history = function_level::get_function_history(
            &repo_path,
            Path::new("test.rs"),
            "my_func",
            (1, 5),
        )?;

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
}
