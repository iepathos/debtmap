use super::{AnalysisTarget, Context, ContextDetails, ContextProvider};
use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use im::HashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Cache for file history to avoid repeated Git calls
#[derive(Debug, Clone)]
pub struct HistoryCache {
    entries: HashMap<PathBuf, FileHistory>,
}

impl Default for HistoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, path: &Path) -> Option<FileHistory> {
        self.entries.get(path).cloned()
    }

    pub fn set(&mut self, path: PathBuf, history: FileHistory) {
        self.entries.insert(path, history);
    }
}

/// Provider for Git history context
pub struct GitHistoryProvider {
    repo_root: PathBuf,
    cache: HistoryCache,
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

        Ok(Self {
            repo_root,
            cache: HistoryCache::new(),
        })
    }

    pub fn analyze_file(&mut self, path: &Path) -> Result<FileHistory> {
        if let Some(cached) = self.cache.get(path) {
            return Ok(cached);
        }

        let history = FileHistory {
            change_frequency: self.calculate_churn_rate(path)?,
            bug_fix_count: self.count_bug_fixes(path)?,
            last_modified: self.get_last_modified(path)?,
            author_count: self.count_unique_authors(path)?,
            stability_score: self.calculate_stability(path)?,
            total_commits: self.count_commits(path)?,
            age_days: self.get_file_age_days(path)?,
        };

        self.cache.set(path.to_path_buf(), history.clone());
        Ok(history)
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

    fn count_bug_fixes(&self, path: &Path) -> Result<usize> {
        let output = Command::new("git")
            .args([
                "log",
                "--oneline",
                "--grep=fix",
                "--grep=bug",
                "--grep=Fix",
                "--grep=Bug",
                "--",
                path.to_str().unwrap_or(""),
            ])
            .current_dir(&self.repo_root)
            .output()
            .context("Failed to count bug fixes")?;

        if output.status.success() {
            let lines = String::from_utf8_lossy(&output.stdout);
            Ok(lines.lines().count())
        } else {
            Ok(0)
        }
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
        let mut provider = GitHistoryProvider::new(self.repo_root.clone())?;
        let history = provider.analyze_file(&target.file_path)?;

        // Calculate contribution based on instability
        let _instability = 1.0 - history.stability_score;
        let bug_density =
            GitHistoryProvider::calculate_bug_density(history.bug_fix_count, history.total_commits);

        let contribution =
            GitHistoryProvider::classify_risk_contribution(history.change_frequency, bug_density);

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
    /// Calculate bug density as a ratio of bug fixes to total commits
    fn calculate_bug_density(bug_fix_count: usize, total_commits: usize) -> f64 {
        if total_commits > 0 {
            bug_fix_count as f64 / total_commits as f64
        } else {
            0.0
        }
    }

    /// Classify the risk contribution based on change frequency and bug density
    fn classify_risk_contribution(change_frequency: f64, bug_density: f64) -> f64 {
        match () {
            _ if change_frequency > 5.0 && bug_density > 0.3 => 2.0, // Very unstable, high risk
            _ if change_frequency > 2.0 || bug_density > 0.2 => 1.0, // Moderately unstable
            _ if change_frequency > 1.0 || bug_density > 0.1 => 0.5, // Slightly unstable
            _ => 0.1,                                                // Stable
        }
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

    #[test]
    fn test_git_history_provider_initialization() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        let provider = GitHistoryProvider::new(repo_path)?;
        assert_eq!(provider.cache.entries.len(), 0);

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
    fn test_classify_risk_contribution_very_unstable() {
        // Very unstable: high frequency AND high bug density
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(6.0, 0.4),
            2.0
        );
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(10.0, 0.5),
            2.0
        );
    }

    #[test]
    fn test_classify_risk_contribution_moderately_unstable() {
        // Moderately unstable: high frequency OR high bug density
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(3.0, 0.1),
            1.0
        );
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(1.5, 0.25),
            1.0
        );
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(2.5, 0.0),
            1.0
        );
    }

    #[test]
    fn test_classify_risk_contribution_slightly_unstable() {
        // Slightly unstable: moderate frequency OR moderate bug density
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(1.5, 0.05),
            0.5
        );
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(0.5, 0.15),
            0.5
        );
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(1.2, 0.0),
            0.5
        );
    }

    #[test]
    fn test_classify_risk_contribution_stable() {
        // Stable: low frequency and low bug density
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(0.5, 0.05),
            0.1
        );
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(0.0, 0.0),
            0.1
        );
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(0.9, 0.09),
            0.1
        );
    }

    #[test]
    fn test_classify_risk_contribution_edge_cases() {
        // Test boundary values
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(5.0, 0.3),
            1.0
        ); // Not quite very unstable (needs BOTH conditions)
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(5.1, 0.31),
            2.0
        ); // Just over the threshold for very unstable
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(2.0, 0.2),
            0.5
        ); // On the boundary (falls through to slightly unstable)
        assert_eq!(
            GitHistoryProvider::classify_risk_contribution(1.0, 0.1),
            0.1
        ); // On the boundary (falls through to stable)
    }

    #[test]
    fn test_gather_integration() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        // Create and commit a test file with multiple changes
        let file_path = create_test_file(&repo_path, "test.rs", "fn main() {}")?;
        commit_with_message(&repo_path, "Initial commit")?;

        // Add more commits to create history
        for i in 1..=3 {
            std::fs::write(&file_path, format!("fn main() {{ /* change {} */ }}", i))?;
            Command::new("git")
                .args(["add", "test.rs"])
                .current_dir(&repo_path)
                .output()?;
            commit_with_message(&repo_path, &format!("fix: bug fix {}", i))?;
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
}
