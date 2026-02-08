//! Git2 library wrapper for reliable git operations
//!
//! This module provides a thread-safe wrapper around libgit2 for git operations,
//! replacing subprocess calls to the git CLI.
//!
//! # Benefits Over Subprocess
//!
//! - **Reliable path handling**: libgit2 handles path resolution internally
//! - **Proper error handling**: Rust `Result` types instead of parsing failures
//! - **Consistent API**: Single library for all git operations
//! - **Better performance**: No process spawning overhead
//! - **Type safety**: Strongly typed git objects instead of string parsing
//!
//! # Architecture
//!
//! Following the Stillwater philosophy:
//! - Pure functions for data transformation
//! - I/O isolated to Git2Repository methods
//! - Thread-safe design (creates new Repository per operation for rayon compatibility)

use anyhow::{Context as _, Result};
use chrono::{DateTime, TimeZone, Utc};
use git2::{BlameOptions, DiffOptions, Oid, Repository, Sort};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Statistics for a single commit
#[derive(Debug, Clone)]
pub struct CommitStats {
    pub hash: git2::Oid,
    pub date: DateTime<Utc>,
    pub message: String,
    pub author_email: String,
    pub files: Vec<FileStats>,
}

/// Statistics for a file change in a commit
#[derive(Debug, Clone)]
pub struct FileStats {
    pub path: PathBuf,
    pub additions: usize,
    pub deletions: usize,
}

/// Blame information for a single line
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameLineInfo {
    pub author: String,
    pub commit_hash: String,
}

/// Blame information for a file
#[derive(Debug, Clone, Default)]
pub struct BlameData {
    pub lines: HashMap<usize, BlameLineInfo>,
}

/// Thread-safe wrapper around git2::Repository
///
/// Note: git2::Repository is not Send/Sync, so this wrapper opens a new
/// Repository instance for each operation. This is the recommended pattern
/// for parallel analysis with rayon.
pub struct Git2Repository {
    repo_path: PathBuf,
}

impl Git2Repository {
    /// Open a repository, discovering the root from any subdirectory
    ///
    /// # Arguments
    /// * `path` - Path to any directory within the git repository
    ///
    /// # Returns
    /// * A Git2Repository instance if the path is within a git repository
    /// * An error if the path is not within a git repository
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::discover(path)
            .with_context(|| format!("Failed to discover git repository at {}", path.display()))?;

        let repo_path = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Bare repositories are not supported"))?
            .to_path_buf();

        Ok(Self { repo_path })
    }

    /// Get the repository root path
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Open a fresh Repository instance (internal helper)
    fn open_repo(&self) -> Result<Repository> {
        Repository::open(&self.repo_path)
            .with_context(|| format!("Failed to open repository at {}", self.repo_path.display()))
    }

    /// Count commits touching a specific file
    ///
    /// # Arguments
    /// * `file_path` - Path to the file, relative to repository root
    ///
    /// # Returns
    /// * Number of commits that touched this file
    pub fn count_file_commits(&self, file_path: &Path) -> Result<usize> {
        let repo = self.open_repo()?;
        let relative_path = self.to_relative_path(file_path);

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let count = revwalk
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| repo.find_commit(oid).ok())
            .filter(|commit| self.commit_touches_file(&repo, commit, &relative_path))
            .count();

        Ok(count)
    }

    /// Get file age in days since first commit
    ///
    /// # Arguments
    /// * `file_path` - Path to the file, relative to repository root
    ///
    /// # Returns
    /// * Age in days, or 0 if file has no git history
    pub fn file_age_days(&self, file_path: &Path) -> Result<u32> {
        let repo = self.open_repo()?;
        let relative_path = self.to_relative_path(file_path);

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME | Sort::REVERSE)?; // Oldest first

        let first_commit = revwalk
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| repo.find_commit(oid).ok())
            .find(|commit| self.commit_touches_file(&repo, commit, &relative_path));

        match first_commit {
            Some(commit) => {
                let time = commit.time();
                let commit_date = Utc.timestamp_opt(time.seconds(), 0).single();
                match commit_date {
                    Some(date) => {
                        let age = Utc::now().signed_duration_since(date);
                        Ok(age.num_days().max(0) as u32)
                    }
                    None => Ok(0),
                }
            }
            None => Ok(0),
        }
    }

    /// Get unique author emails for a file
    ///
    /// # Arguments
    /// * `file_path` - Path to the file, relative to repository root
    ///
    /// # Returns
    /// * Set of unique author emails
    pub fn file_authors(&self, file_path: &Path) -> Result<HashSet<String>> {
        let repo = self.open_repo()?;
        let relative_path = self.to_relative_path(file_path);

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let authors: HashSet<String> = revwalk
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| repo.find_commit(oid).ok())
            .filter(|commit| self.commit_touches_file(&repo, commit, &relative_path))
            .filter_map(|commit| commit.author().email().map(String::from))
            .collect();

        Ok(authors)
    }

    /// Get last modified date for a file
    ///
    /// # Arguments
    /// * `file_path` - Path to the file, relative to repository root
    ///
    /// # Returns
    /// * Last modified date, or None if file has no git history
    pub fn file_last_modified(&self, file_path: &Path) -> Result<Option<DateTime<Utc>>> {
        let repo = self.open_repo()?;
        let relative_path = self.to_relative_path(file_path);

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let last_commit = revwalk
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| repo.find_commit(oid).ok())
            .find(|commit| self.commit_touches_file(&repo, commit, &relative_path));

        match last_commit {
            Some(commit) => {
                let time = commit.time();
                Ok(Utc.timestamp_opt(time.seconds(), 0).single())
            }
            None => Ok(None),
        }
    }

    /// Get all commits with file changes (for batched analysis)
    ///
    /// This is the main entry point for BatchedGitHistory, replacing the
    /// subprocess call to `git log --all --numstat`.
    ///
    /// Uses parallel processing with rayon for better performance on
    /// repositories with many commits.
    ///
    /// # Returns
    /// * Vector of CommitStats with file change information
    pub fn all_commits_with_stats(&self) -> Result<Vec<CommitStats>> {
        // Phase 1: Collect all OIDs (sequential - revwalk can't be parallelized)
        let oids: Vec<Oid> = {
            let repo = self.open_repo()?;
            let mut revwalk = repo.revwalk()?;
            revwalk.push_head()?;
            revwalk.set_sorting(Sort::TIME)?;
            revwalk.filter_map(|r| r.ok()).collect()
        };

        // Phase 2: Process commits in parallel
        // Each thread opens its own Repository (git2::Repository isn't Send)
        let repo_path = self.repo_path.clone();
        let commits: Vec<CommitStats> = oids
            .into_par_iter()
            .filter_map(|oid| {
                let repo = Repository::open(&repo_path).ok()?;
                let commit = repo.find_commit(oid).ok()?;
                Self::commit_to_stats_static(&repo, &commit).ok().flatten()
            })
            .collect();

        Ok(commits)
    }

    /// Static version of commit_to_stats for use in parallel contexts
    fn commit_to_stats_static(
        repo: &Repository,
        commit: &git2::Commit,
    ) -> Result<Option<CommitStats>> {
        let parent = commit.parents().next();
        let parent_tree = parent.and_then(|p| p.tree().ok());
        let tree = commit.tree()?;

        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

        // Count additions/deletions per file in a single pass
        let mut file_stats: HashMap<PathBuf, (usize, usize)> = HashMap::new();

        // First pass: register all files from deltas
        for i in 0..diff.deltas().count() {
            if let Some(delta) = diff.get_delta(i) {
                if let Some(path) = delta.new_file().path() {
                    file_stats.entry(path.to_path_buf()).or_insert((0, 0));
                }
            }
        }

        // Second pass: count lines using foreach (single diff traversal)
        diff.foreach(
            &mut |_, _| true,
            None,
            None,
            Some(&mut |delta, _hunk, line| {
                if let Some(path) = delta.new_file().path() {
                    let entry = file_stats.entry(path.to_path_buf()).or_insert((0, 0));
                    match line.origin() {
                        '+' => entry.0 += 1,
                        '-' => entry.1 += 1,
                        _ => {}
                    }
                }
                true
            }),
        )?;

        if file_stats.is_empty() {
            return Ok(None);
        }

        let files: Vec<FileStats> = file_stats
            .into_iter()
            .map(|(path, (additions, deletions))| FileStats {
                path,
                additions,
                deletions,
            })
            .collect();

        let time = commit.time();
        let date = Utc
            .timestamp_opt(time.seconds(), 0)
            .single()
            .unwrap_or_else(Utc::now);

        Ok(Some(CommitStats {
            hash: commit.id(),
            date,
            message: commit.message().unwrap_or("").to_string(),
            author_email: commit.author().email().unwrap_or("").to_string(),
            files,
        }))
    }

    /// Get blame information for a file
    ///
    /// # Arguments
    /// * `file_path` - Path to the file, relative to repository root
    ///
    /// # Returns
    /// * BlameData with line-by-line author information
    pub fn blame_file(&self, file_path: &Path) -> Result<BlameData> {
        let repo = self.open_repo()?;
        let relative_path = self.to_relative_path(file_path);
        let relative_str = relative_path.to_string_lossy();

        let mut opts = BlameOptions::new();
        opts.track_copies_same_file(true);

        let blame = repo
            .blame_file(Path::new(relative_str.as_ref()), Some(&mut opts))
            .with_context(|| format!("Failed to blame file {}", file_path.display()))?;

        let mut lines = HashMap::new();
        for hunk in blame.iter() {
            let sig = hunk.final_signature();
            let author = sig.name().unwrap_or("Unknown").to_string();
            let commit_hash = hunk.final_commit_id().to_string();
            let start_line = hunk.final_start_line();
            let num_lines = hunk.lines_in_hunk();

            for i in 0..num_lines {
                let line_num = start_line + i;
                lines.insert(
                    line_num,
                    BlameLineInfo {
                        author: author.clone(),
                        commit_hash: commit_hash.clone(),
                    },
                );
            }
        }

        Ok(BlameData { lines })
    }

    /// Find the commit that introduced a string (pickaxe search)
    ///
    /// Equivalent to `git log -S "pattern" --reverse`
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to search in
    /// * `pattern` - The string pattern to search for
    ///
    /// # Returns
    /// * The first (oldest) commit OID that introduced the pattern, or None
    pub fn find_introduction(
        &self,
        file_path: &Path,
        pattern: &str,
    ) -> Result<Option<(git2::Oid, DateTime<Utc>)>> {
        let repo = self.open_repo()?;
        let relative_path = self.to_relative_path(file_path);

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME | Sort::REVERSE)?; // Oldest first

        for oid in revwalk.filter_map(|r| r.ok()) {
            let commit = repo.find_commit(oid)?;
            if self.commit_introduces_pattern(&repo, &commit, &relative_path, pattern)? {
                let time = commit.time();
                if let Some(date) = Utc.timestamp_opt(time.seconds(), 0).single() {
                    return Ok(Some((oid, date)));
                }
            }
        }

        Ok(None)
    }

    /// Find commits that modified a pattern after a given commit
    ///
    /// Equivalent to `git log <after_commit>..HEAD -G "pattern"`
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to search in
    /// * `pattern` - The regex pattern to search for
    /// * `after_commit` - Start searching after this commit (exclusive)
    ///
    /// # Returns
    /// * Vector of commits that modified lines matching the pattern
    pub fn find_modifications(
        &self,
        file_path: &Path,
        pattern: &str,
        after_commit: git2::Oid,
    ) -> Result<Vec<CommitStats>> {
        let repo = self.open_repo()?;
        let relative_path = self.to_relative_path(file_path);
        let regex = regex::Regex::new(pattern)?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let mut results = Vec::new();

        for oid in revwalk.filter_map(|r| r.ok()) {
            // Stop at the introduction commit
            if oid == after_commit {
                break;
            }

            let commit = repo.find_commit(oid)?;
            if self.commit_modifies_pattern(&repo, &commit, &relative_path, &regex)? {
                if let Some(stats) = self.commit_to_basic_stats(&commit)? {
                    results.push(stats);
                }
            }
        }

        Ok(results)
    }

    /// Count bug fix commits for a file
    ///
    /// # Arguments
    /// * `file_path` - Path to the file, relative to repository root
    ///
    /// # Returns
    /// * Number of commits with bug fix keywords in their message
    pub fn count_bug_fixes(&self, file_path: &Path) -> Result<usize> {
        let repo = self.open_repo()?;
        let relative_path = self.to_relative_path(file_path);

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let count = revwalk
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| repo.find_commit(oid).ok())
            .filter(|commit| self.commit_touches_file(&repo, commit, &relative_path))
            .filter(|commit| commit.message().map(is_bug_fix_message).unwrap_or(false))
            .count();

        Ok(count)
    }

    // =========================================================================
    // Internal Helper Methods
    // =========================================================================

    /// Convert a path to be relative to the repository root
    fn to_relative_path(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.repo_path)
            .unwrap_or(path)
            .to_path_buf()
    }

    /// Check if a commit touches a specific file
    fn commit_touches_file(
        &self,
        repo: &Repository,
        commit: &git2::Commit,
        file_path: &Path,
    ) -> bool {
        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => return false,
        };

        let file_str = file_path.to_string_lossy();

        // Check if file exists in this commit's tree
        if tree.get_path(Path::new(file_str.as_ref())).is_err() {
            return false;
        }

        // Check if the file was changed in this commit
        let parent = commit.parents().next();
        let parent_tree = parent.and_then(|p| p.tree().ok());

        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(file_str.as_ref());

        let diff =
            match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts)) {
                Ok(d) => d,
                Err(_) => return false,
            };

        diff.deltas().count() > 0
    }

    /// Convert commit to basic stats (without file details)
    fn commit_to_basic_stats(&self, commit: &git2::Commit) -> Result<Option<CommitStats>> {
        let time = commit.time();
        let date = Utc
            .timestamp_opt(time.seconds(), 0)
            .single()
            .unwrap_or_else(Utc::now);

        Ok(Some(CommitStats {
            hash: commit.id(),
            date,
            message: commit.message().unwrap_or("").to_string(),
            author_email: commit.author().email().unwrap_or("").to_string(),
            files: Vec::new(),
        }))
    }

    /// Check if a commit introduces a pattern (pickaxe-style search)
    fn commit_introduces_pattern(
        &self,
        repo: &Repository,
        commit: &git2::Commit,
        file_path: &Path,
        pattern: &str,
    ) -> Result<bool> {
        let tree = commit.tree()?;
        let file_str = file_path.to_string_lossy();

        // Check if file exists in this commit
        let entry = match tree.get_path(Path::new(file_str.as_ref())) {
            Ok(e) => e,
            Err(_) => return Ok(false),
        };

        // Get the blob content
        let blob = repo.find_blob(entry.id())?;
        let content = std::str::from_utf8(blob.content()).unwrap_or("");

        // Check if pattern exists in current version
        if !content.contains(pattern) {
            return Ok(false);
        }

        // Check if pattern didn't exist in parent
        let parent = commit.parents().next();
        if let Some(parent_commit) = parent {
            let parent_tree = parent_commit.tree()?;
            if let Ok(parent_entry) = parent_tree.get_path(Path::new(file_str.as_ref())) {
                if let Ok(parent_blob) = repo.find_blob(parent_entry.id()) {
                    let parent_content = std::str::from_utf8(parent_blob.content()).unwrap_or("");
                    if parent_content.contains(pattern) {
                        return Ok(false); // Already existed in parent
                    }
                }
            }
        }

        Ok(true)
    }

    /// Check if a commit modifies lines matching a regex pattern
    fn commit_modifies_pattern(
        &self,
        repo: &Repository,
        commit: &git2::Commit,
        file_path: &Path,
        regex: &regex::Regex,
    ) -> Result<bool> {
        let parent = commit.parents().next();
        let parent_tree = parent.and_then(|p| p.tree().ok());
        let tree = commit.tree()?;

        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(file_path.to_string_lossy().as_ref());

        let diff =
            repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

        // Check if any line in the diff matches the pattern
        let mut found = false;
        diff.foreach(
            &mut |_, _| true,
            None,
            None,
            Some(&mut |_, _, line| {
                if let Ok(content) = std::str::from_utf8(line.content()) {
                    if regex.is_match(content) {
                        found = true;
                    }
                }
                true
            }),
        )?;

        Ok(found)
    }
}

// =============================================================================
// Pure Helper Functions
// =============================================================================

/// Check if a commit message indicates a bug fix
///
/// Pure function - matches the logic from batched.rs
pub fn is_bug_fix_message(message: &str) -> bool {
    if is_excluded_commit(message) {
        return false;
    }

    let lowercase = message.to_lowercase();
    let words: Vec<&str> = lowercase.split(|c: char| !c.is_alphanumeric()).collect();

    words.iter().any(|&word| {
        matches!(
            word,
            "bug" | "fix" | "fixes" | "fixed" | "fixing" | "hotfix"
        )
    })
}

/// Check if a commit should be excluded from bug fix counting
///
/// Pure function - matches the logic from batched.rs
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

        if !has_bug_keyword {
            return true;
        }
    }

    false
}

/// Extract unique authors for a line range from blame data
///
/// Pure function - O(n) iteration over line range
pub fn extract_authors_for_range(
    blame_data: &BlameData,
    start_line: usize,
    end_line: usize,
) -> HashSet<String> {
    (start_line..=end_line)
        .filter_map(|line| blame_data.lines.get(&line))
        .map(|info| info.author.clone())
        .filter(|author| !author.is_empty() && author != "Not Committed Yet")
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().to_path_buf();

        Command::new("git")
            .arg("init")
            .current_dir(&repo_path)
            .output()?;

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

    fn create_and_commit_file(
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

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(repo_path)
            .output()?;

        Ok(())
    }

    #[test]
    fn test_is_bug_fix_message() {
        // Should match bug fixes
        assert!(is_bug_fix_message("fix: resolve login bug"));
        assert!(is_bug_fix_message("Fixed the payment issue"));
        assert!(is_bug_fix_message("Bug fix for issue #123"));
        assert!(is_bug_fix_message("hotfix: urgent fix"));

        // Should NOT match excluded commits
        assert!(!is_bug_fix_message("style: apply formatting fixes"));
        assert!(!is_bug_fix_message("chore: update dependencies"));
        assert!(!is_bug_fix_message("docs: fix typo"));
        assert!(!is_bug_fix_message("refactor: improve prefix handling"));

        // Should NOT match false positives
        assert!(!is_bug_fix_message("Add debugging utilities"));
    }

    #[test]
    fn test_extract_authors_for_range() {
        let mut lines = HashMap::new();
        lines.insert(
            1,
            BlameLineInfo {
                author: "Alice".into(),
                commit_hash: "abc".into(),
            },
        );
        lines.insert(
            2,
            BlameLineInfo {
                author: "Bob".into(),
                commit_hash: "def".into(),
            },
        );
        lines.insert(
            3,
            BlameLineInfo {
                author: "Alice".into(),
                commit_hash: "abc".into(),
            },
        );

        let blame_data = BlameData { lines };
        let authors = extract_authors_for_range(&blame_data, 1, 3);

        assert_eq!(authors.len(), 2);
        assert!(authors.contains("Alice"));
        assert!(authors.contains("Bob"));
    }

    #[test]
    fn test_git2_repository_open() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        // Create initial commit
        create_and_commit_file(&repo_path, "test.rs", "fn main() {}", "Initial commit")?;

        let repo = Git2Repository::open(&repo_path)?;

        // Canonicalize both paths to handle macOS /var -> /private/var symlinks
        let expected = repo_path.canonicalize().unwrap_or(repo_path);
        let actual = repo
            .repo_path()
            .canonicalize()
            .unwrap_or(repo.repo_path().to_path_buf());
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_git2_repository_count_commits() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        create_and_commit_file(&repo_path, "test.rs", "fn main() {}", "Initial commit")?;
        create_and_commit_file(&repo_path, "test.rs", "fn main() { println!(); }", "Second")?;
        create_and_commit_file(&repo_path, "test.rs", "fn main() { dbg!(); }", "Third")?;

        let repo = Git2Repository::open(&repo_path)?;
        let count = repo.count_file_commits(Path::new("test.rs"))?;

        assert_eq!(count, 3);
        Ok(())
    }

    #[test]
    fn test_git2_repository_file_authors() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        create_and_commit_file(&repo_path, "test.rs", "fn main() {}", "Initial commit")?;

        let repo = Git2Repository::open(&repo_path)?;
        let authors = repo.file_authors(Path::new("test.rs"))?;

        assert!(authors.contains("test@example.com"));
        Ok(())
    }

    #[test]
    fn test_git2_repository_blame() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        let content = "line1\nline2\nline3\n";
        create_and_commit_file(&repo_path, "test.txt", content, "Initial commit")?;

        let repo = Git2Repository::open(&repo_path)?;
        let blame = repo.blame_file(Path::new("test.txt"))?;

        assert!(blame.lines.contains_key(&1));
        assert!(blame.lines.contains_key(&2));
        assert!(blame.lines.contains_key(&3));

        Ok(())
    }

    #[test]
    fn test_git2_repository_all_commits_with_stats() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        create_and_commit_file(&repo_path, "test.rs", "fn main() {}", "Initial commit")?;
        create_and_commit_file(&repo_path, "other.rs", "fn other() {}", "Second commit")?;

        let repo = Git2Repository::open(&repo_path)?;
        let commits = repo.all_commits_with_stats()?;

        assert_eq!(commits.len(), 2);
        Ok(())
    }

    #[test]
    fn test_git2_repository_count_bug_fixes() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        create_and_commit_file(&repo_path, "test.rs", "fn main() {}", "Initial commit")?;
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn main() { v2 }",
            "fix: resolve bug",
        )?;
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn main() { v3 }",
            "feat: add feature",
        )?;
        create_and_commit_file(&repo_path, "test.rs", "fn main() { v4 }", "hotfix: urgent")?;

        let repo = Git2Repository::open(&repo_path)?;
        let bug_fixes = repo.count_bug_fixes(Path::new("test.rs"))?;

        assert_eq!(bug_fixes, 2);
        Ok(())
    }

    fn get_head_oid(repo_path: &Path) -> Result<git2::Oid> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()?;
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(git2::Oid::from_str(&hash)?)
    }

    #[test]
    fn test_find_modifications_finds_commits_modifying_pattern() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        // Create file with initial content - we use a zero OID to get ALL commits
        create_and_commit_file(&repo_path, "test.rs", "let marker = 0;", "Initial")?;

        // First modification changes the marker line
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "let marker = 1;",
            "First modification",
        )?;

        // Second modification changes the marker line again
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "let marker = 2;",
            "Second modification",
        )?;

        let repo = Git2Repository::open(&repo_path)?;

        // Use a pattern that matches the changed line in all commits
        // Pass a zero OID that won't match any commit, so we get all modifications
        let zero_oid = git2::Oid::zero();
        let modifications = repo.find_modifications(Path::new("test.rs"), "marker", zero_oid)?;

        // All three commits change a line containing "marker"
        assert_eq!(modifications.len(), 3, "Expected 3 modifications");

        // Verify all commit messages are present
        let messages: Vec<_> = modifications.iter().map(|m| m.message.as_str()).collect();
        assert!(
            messages.iter().any(|m| m.contains("Initial")),
            "Should include Initial"
        );
        assert!(
            messages.iter().any(|m| m.contains("First modification")),
            "Should include First modification"
        );
        assert!(
            messages.iter().any(|m| m.contains("Second modification")),
            "Should include Second modification"
        );
        Ok(())
    }

    #[test]
    fn test_find_modifications_returns_empty_when_pattern_not_in_diff() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        // Create file with stable pattern and a placeholder line
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn stable_pattern() {}\n// placeholder",
            "Initial",
        )?;

        // Change only the placeholder, not the stable_pattern line
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn stable_pattern() {}\n// changed placeholder",
            "Modify placeholder only",
        )?;

        let repo = Git2Repository::open(&repo_path)?;
        let zero_oid = git2::Oid::zero();

        // Search for a pattern that was never in any diff
        let untouched_modifications =
            repo.find_modifications(Path::new("test.rs"), "xyz_never_exists", zero_oid)?;

        assert!(
            untouched_modifications.is_empty(),
            "Expected no modifications for pattern that was never in the diff"
        );
        Ok(())
    }

    #[test]
    fn test_find_modifications_stops_at_specified_commit() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        // Create commits with pattern changes
        create_and_commit_file(&repo_path, "test.rs", "let x = 1;", "v1")?;
        create_and_commit_file(&repo_path, "test.rs", "let x = 2;", "v2")?;
        create_and_commit_file(&repo_path, "test.rs", "let x = 3;", "v3")?;
        create_and_commit_file(&repo_path, "test.rs", "let x = 4;", "v4")?;
        let latest_oid = get_head_oid(&repo_path)?;

        let repo = Git2Repository::open(&repo_path)?;

        // When we pass the latest OID as after_commit, revwalk should immediately
        // hit the break condition and return no results
        let modifications = repo.find_modifications(Path::new("test.rs"), r"let x", latest_oid)?;

        // Should find nothing since we stop at the first commit we encounter
        assert!(
            modifications.is_empty(),
            "Expected no modifications when stopping at latest commit"
        );

        // Verify with zero OID we get all commits
        let all_mods =
            repo.find_modifications(Path::new("test.rs"), r"let x", git2::Oid::zero())?;
        assert_eq!(all_mods.len(), 4, "Should find all 4 commits with zero OID");

        Ok(())
    }

    #[test]
    fn test_find_introduction_finds_first_occurrence() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        // Create file without the pattern
        create_and_commit_file(&repo_path, "test.rs", "fn other() {}", "Initial")?;

        // Add the pattern we're looking for
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn other() {}\nfn special_marker() {}",
            "Add marker",
        )?;
        let expected_oid = get_head_oid(&repo_path)?;

        // Modify after introduction
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn other() {}\nfn special_marker() { updated }",
            "Update",
        )?;

        let repo = Git2Repository::open(&repo_path)?;
        let result = repo.find_introduction(Path::new("test.rs"), "special_marker")?;

        assert!(result.is_some());
        let (oid, _date) = result.unwrap();
        assert_eq!(oid, expected_oid);
        Ok(())
    }

    #[test]
    fn test_find_introduction_returns_none_when_not_found() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        create_and_commit_file(&repo_path, "test.rs", "fn main() {}", "Initial")?;

        let repo = Git2Repository::open(&repo_path)?;
        let result = repo.find_introduction(Path::new("test.rs"), "nonexistent_pattern")?;

        assert!(result.is_none());
        Ok(())
    }
}
