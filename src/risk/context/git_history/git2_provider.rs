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

    /// Open a fresh Repository instance (test-visible helper)
    pub(super) fn open_repo(&self) -> Result<Repository> {
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
    /// * `now` - Reference time for age calculation
    ///
    /// # Returns
    /// * Age in days, or 0 if file has no git history
    pub fn file_age_days(&self, file_path: &Path, now: DateTime<Utc>) -> Result<u32> {
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
                        let age = now.signed_duration_since(date);
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
        let regex = regex::Regex::new(pattern)?;
        self.find_modifications_with_regex(file_path, &regex, after_commit)
    }

    /// Find commits modifying a file where the diff matches `regex` after `after_commit`.
    ///
    /// Equivalent to `git log <after_commit>..HEAD -G "<pattern>" -- <file>`.
    pub fn find_modifications_with_regex(
        &self,
        file_path: &Path,
        regex: &regex::Regex,
        after_commit: git2::Oid,
    ) -> Result<Vec<CommitStats>> {
        let repo = self.open_repo()?;
        let relative_path = self.to_relative_path(file_path);

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        // Equivalent to `git log <after_commit>..HEAD` (exclude intro and its ancestors).
        if after_commit != Oid::zero() {
            revwalk.hide(after_commit)?;
        }
        revwalk.set_sorting(Sort::TIME)?;

        let mut results = Vec::new();

        for oid in revwalk.filter_map(|r| r.ok()) {
            let commit = repo.find_commit(oid)?;
            if self.commit_modifies_pattern(&repo, &commit, &relative_path, regex)? {
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

    /// Check if a commit introduces a pattern (pickaxe `-S` semantics).
    fn commit_introduces_pattern(
        &self,
        repo: &Repository,
        commit: &git2::Commit,
        file_path: &Path,
        pattern: &str,
    ) -> Result<bool> {
        Ok(commit_pickaxe_changes_pattern(
            repo, commit, file_path, pattern,
        )?)
    }

    /// Check if a commit modifies lines matching a regex (`git log -G` semantics).
    fn commit_modifies_pattern(
        &self,
        repo: &Repository,
        commit: &git2::Commit,
        file_path: &Path,
        regex: &regex::Regex,
    ) -> Result<bool> {
        commit_diff_matches_regex(repo, commit, file_path, regex)
    }
}

/// Count non-overlapping occurrences of `pattern` in `content` (pickaxe `-S` unit).
pub fn count_pattern_occurrences(content: &str, pattern: &str) -> usize {
    if pattern.is_empty() {
        return 0;
    }
    content.matches(pattern).count()
}

/// Occurrences of `pattern` in `file_path` at `commit` (0 if file missing).
pub fn pattern_occurrences_in_commit(
    repo: &Repository,
    commit: &git2::Commit,
    file_path: &Path,
    pattern: &str,
) -> Result<usize> {
    let tree = commit.tree()?;
    let file_str = file_path.to_string_lossy();
    let entry = match tree.get_path(Path::new(file_str.as_ref())) {
        Ok(e) => e,
        Err(_) => return Ok(0),
    };
    let blob = repo.find_blob(entry.id())?;
    let content = std::str::from_utf8(blob.content()).unwrap_or("");
    Ok(count_pattern_occurrences(content, pattern))
}

/// True when pickaxe `-S` would report a change for this file in `commit`.
///
/// Matches `git log -S`: occurrence count of `pattern` in the file changed vs parent.
pub fn commit_pickaxe_changes_pattern(
    repo: &Repository,
    commit: &git2::Commit,
    file_path: &Path,
    pattern: &str,
) -> Result<bool> {
    let new_count = pattern_occurrences_in_commit(repo, commit, file_path, pattern)?;
    let old_count = match commit.parents().next() {
        Some(parent) => pattern_occurrences_in_commit(repo, &parent, file_path, pattern)?,
        None => 0,
    };
    Ok(new_count != old_count)
}

/// Commit history aggregates for a single function within one file.
#[derive(Debug, Clone, Default)]
pub struct FileFunctionRecord {
    pub introduction_oid: Option<git2::Oid>,
    pub introduction_date: Option<DateTime<Utc>>,
    pub modifications: Vec<CommitStats>,
}

/// Per-commit accounting for `compute_repo_function_histories`.
#[derive(Debug, Clone)]
struct CommitFunctionData {
    oid: Oid,
    date: DateTime<Utc>,
    message: String,
    author_email: String,
    /// Churn (additions + deletions) per file touched in this commit.
    file_churn: HashMap<PathBuf, usize>,
    /// (file, function_name) → (added_count, removed_count, regex_matched)
    updates: HashMap<(PathBuf, String), (usize, usize, bool)>,
}

/// File-level commit scan exported for building `BatchedGitHistory`.
#[derive(Debug, Clone)]
pub struct FileCommitScan {
    pub date: DateTime<Utc>,
    pub message: String,
    pub author_email: String,
    pub file_churn: HashMap<PathBuf, usize>,
}

/// Combined function- and file-level history from one repository walk.
pub struct RepoHistoryScan {
    pub functions: HashMap<(PathBuf, String), FileFunctionRecord>,
    pub file_scans: Vec<FileCommitScan>,
}

/// Compute per-function histories for many files via one repository-wide commit walk.
///
/// Phase 1: collect OIDs sequentially.
/// Phase 2: process each commit in parallel via rayon, accumulating per-commit
/// per-(file,function) `-S` add/remove counts and `-G` regex hits.
/// Phase 3: walk results in commit order to determine the introduction commit
/// (first non-zero `-S` delta) and modification commits (`-G` matches after
/// introduction).
///
/// `progress_cb` receives `(processed_commits, total_commits)` at most once
/// per ~50 commits processed. It is called from worker threads; keep it cheap.
pub fn compute_repo_function_histories(
    repo_path: &Path,
    file_targets: &HashMap<PathBuf, Vec<String>>,
    progress_cb: Option<super::batched_function::ProgressCallback<'_>>,
) -> Result<RepoHistoryScan> {
    use super::batched_function::GitPreloadPhase;

    if file_targets.is_empty() {
        return Ok(RepoHistoryScan {
            functions: HashMap::new(),
            file_scans: Vec::new(),
        });
    }

    let (intro_patterns, mod_regexes) = build_function_pattern_tables(file_targets);
    let oids = collect_repo_oids(repo_path)?;
    let total = oids.len();

    if let Some(cb) = progress_cb {
        cb(GitPreloadPhase::Commits, 0, total);
    }

    let processed = std::sync::atomic::AtomicUsize::new(0);
    let mut commit_data: Vec<CommitFunctionData> = oids
        .par_iter()
        .filter_map(|&oid| {
            let repo = Repository::open(repo_path).ok()?;
            let data = process_commit_for_function_history(
                &repo,
                oid,
                file_targets,
                &intro_patterns,
                &mod_regexes,
            )
            .ok()
            .flatten();
            let done = processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if let Some(cb) = progress_cb {
                if done % 50 == 0 || done == total {
                    cb(GitPreloadPhase::Commits, done, total);
                }
            }
            data
        })
        .collect();

    commit_data.sort_by_key(|d| d.date);
    let file_scans = commit_data
        .iter()
        .map(|d| FileCommitScan {
            date: d.date,
            message: d.message.clone(),
            author_email: d.author_email.clone(),
            file_churn: d.file_churn.clone(),
        })
        .collect();
    Ok(RepoHistoryScan {
        functions: reduce_commit_data_to_records(file_targets, &commit_data),
        file_scans,
    })
}

fn build_function_pattern_tables(
    file_targets: &HashMap<PathBuf, Vec<String>>,
) -> (
    HashMap<PathBuf, Vec<(String, String)>>,
    HashMap<PathBuf, Vec<(String, regex::Regex)>>,
) {
    let intro = file_targets
        .iter()
        .map(|(file, names)| {
            let v = names
                .iter()
                .map(|n| (n.clone(), format!("fn {n}")))
                .collect();
            (file.clone(), v)
        })
        .collect();

    let mods = file_targets
        .iter()
        .map(|(file, names)| {
            let v = names
                .iter()
                .filter_map(|n| {
                    regex::Regex::new(n)
                        .or_else(|_| regex::Regex::new(&regex::escape(n)))
                        .ok()
                        .map(|r| (n.clone(), r))
                })
                .collect();
            (file.clone(), v)
        })
        .collect();

    (intro, mods)
}

fn collect_repo_oids(repo_path: &Path) -> Result<Vec<Oid>> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME | Sort::REVERSE)?;
    Ok(revwalk.filter_map(|r| r.ok()).collect())
}

fn process_commit_for_function_history(
    repo: &Repository,
    oid: Oid,
    file_targets: &HashMap<PathBuf, Vec<String>>,
    intro_patterns: &HashMap<PathBuf, Vec<(String, String)>>,
    mod_regexes: &HashMap<PathBuf, Vec<(String, regex::Regex)>>,
) -> Result<Option<CommitFunctionData>> {
    let commit = repo.find_commit(oid)?;
    let parent = commit.parents().next();
    let parent_tree = parent.as_ref().and_then(|p| p.tree().ok());
    let tree = commit.tree()?;
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

    let mut per_file_added: HashMap<PathBuf, Vec<String>> = HashMap::new();
    let mut per_file_removed: HashMap<PathBuf, Vec<String>> = HashMap::new();
    let mut file_churn: HashMap<PathBuf, usize> = HashMap::new();

    diff.foreach(
        &mut |_, _| true,
        None,
        None,
        Some(&mut |delta, _, line| {
            let Some(path) = delta.new_file().path().or_else(|| delta.old_file().path()) else {
                return true;
            };
            let path_buf = path.to_path_buf();
            match line.origin() {
                '+' | '-' => {
                    *file_churn.entry(path_buf.clone()).or_default() += 1;
                }
                _ => return true,
            }
            if !file_targets.contains_key(&path_buf) {
                return true;
            }
            let Ok(text) = std::str::from_utf8(line.content()) else {
                return true;
            };
            match line.origin() {
                '+' => per_file_added
                    .entry(path_buf)
                    .or_default()
                    .push(text.to_string()),
                '-' => per_file_removed
                    .entry(path_buf)
                    .or_default()
                    .push(text.to_string()),
                _ => {}
            }
            true
        }),
    )?;

    if file_churn.is_empty() {
        return Ok(None);
    }

    let touched: HashSet<&PathBuf> = per_file_added
        .keys()
        .chain(per_file_removed.keys())
        .collect();

    let mut updates: HashMap<(PathBuf, String), (usize, usize, bool)> = HashMap::new();
    let empty: Vec<String> = Vec::new();
    for file in touched {
        let added = per_file_added.get(file).unwrap_or(&empty);
        let removed = per_file_removed.get(file).unwrap_or(&empty);
        if let Some(patterns) = intro_patterns.get(file) {
            for (name, intro_pat) in patterns {
                let added_count: usize = added
                    .iter()
                    .map(|l| count_pattern_occurrences(l, intro_pat))
                    .sum();
                let removed_count: usize = removed
                    .iter()
                    .map(|l| count_pattern_occurrences(l, intro_pat))
                    .sum();
                let entry = updates.entry((file.clone(), name.clone())).or_default();
                entry.0 = added_count;
                entry.1 = removed_count;
            }
        }
        if let Some(regexes) = mod_regexes.get(file) {
            for (name, regex) in regexes {
                let matched = added.iter().any(|l| regex.is_match(l))
                    || removed.iter().any(|l| regex.is_match(l));
                let entry = updates.entry((file.clone(), name.clone())).or_default();
                entry.2 = matched;
            }
        }
    }

    let date = Utc
        .timestamp_opt(commit.time().seconds(), 0)
        .single()
        .unwrap_or_else(Utc::now);
    let message = commit.message().unwrap_or("").to_string();
    let author_email = commit.author().email().unwrap_or("").to_string();
    Ok(Some(CommitFunctionData {
        oid,
        date,
        message,
        author_email,
        file_churn,
        updates,
    }))
}

fn reduce_commit_data_to_records(
    file_targets: &HashMap<PathBuf, Vec<String>>,
    commit_data: &[CommitFunctionData],
) -> HashMap<(PathBuf, String), FileFunctionRecord> {
    let mut records: HashMap<(PathBuf, String), FileFunctionRecord> = file_targets
        .iter()
        .flat_map(|(file, names)| {
            names
                .iter()
                .map(move |n| ((file.clone(), n.clone()), FileFunctionRecord::default()))
        })
        .collect();

    for data in commit_data {
        for (key, (added, removed, matched)) in &data.updates {
            let Some(record) = records.get_mut(key) else {
                continue;
            };
            if record.introduction_oid.is_none() {
                if added != removed {
                    record.introduction_oid = Some(data.oid);
                    record.introduction_date = Some(data.date);
                }
            } else if *matched && record.introduction_oid != Some(data.oid) {
                record.modifications.push(CommitStats {
                    hash: data.oid,
                    date: data.date,
                    message: data.message.clone(),
                    author_email: data.author_email.clone(),
                    files: Vec::new(),
                });
            }
        }
    }

    records
}

/// Compute `-S` introductions and `-G` modifications for many functions
/// while walking each file's commit history exactly once.
///
/// Equivalent to running `git log -S "fn <name>" --reverse -- <file>` and
/// `git log <intro>..HEAD -G "<name>" -- <file>` for each function, but
/// shares the commit walk, blob reads, and diff computation across all
/// functions in the same file.
pub fn compute_file_function_histories(
    repo: &Repository,
    file_path: &Path,
    function_names: &[String],
) -> Result<HashMap<String, FileFunctionRecord>> {
    let mut records: HashMap<String, FileFunctionRecord> = function_names
        .iter()
        .map(|n| (n.clone(), FileFunctionRecord::default()))
        .collect();
    if function_names.is_empty() {
        return Ok(records);
    }

    let intro_patterns: Vec<(String, String)> = function_names
        .iter()
        .map(|n| (n.clone(), format!("fn {n}")))
        .collect();
    let mod_regexes: Vec<(String, regex::Regex)> = function_names
        .iter()
        .filter_map(|n| {
            regex::Regex::new(n)
                .or_else(|_| regex::Regex::new(&regex::escape(n)))
                .ok()
                .map(|r| (n.clone(), r))
        })
        .collect();

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME | Sort::REVERSE)?;
    let path_str = file_path.to_string_lossy().to_string();

    for oid in revwalk.filter_map(|r| r.ok()) {
        let commit = repo.find_commit(oid)?;
        let parent = commit.parents().next();
        let parent_tree = parent.as_ref().and_then(|p| p.tree().ok());
        let tree = commit.tree()?;

        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(path_str.as_str());
        let diff =
            repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;
        if diff.deltas().count() == 0 {
            continue;
        }

        let new_content = read_file_at_tree(repo, &tree, file_path).unwrap_or_default();
        let old_content = parent_tree
            .as_ref()
            .and_then(|t| read_file_at_tree(repo, t, file_path))
            .unwrap_or_default();

        let commit_time = commit.time();
        let commit_date = Utc.timestamp_opt(commit_time.seconds(), 0).single();

        for (name, pattern) in &intro_patterns {
            let record = records.get_mut(name).expect("inserted above");
            if record.introduction_oid.is_some() {
                continue;
            }
            let new_count = count_pattern_occurrences(&new_content, pattern);
            let old_count = count_pattern_occurrences(&old_content, pattern);
            if new_count != old_count {
                record.introduction_oid = Some(oid);
                record.introduction_date = commit_date;
            }
        }

        let mut added_or_removed: Vec<String> = Vec::new();
        diff.foreach(
            &mut |_, _| true,
            None,
            None,
            Some(&mut |_, _, line| {
                if matches!(line.origin(), '+' | '-') {
                    if let Ok(text) = std::str::from_utf8(line.content()) {
                        added_or_removed.push(text.to_string());
                    }
                }
                true
            }),
        )?;

        let stats = CommitStats {
            hash: oid,
            date: commit_date.unwrap_or_else(Utc::now),
            message: commit.message().unwrap_or("").to_string(),
            author_email: commit.author().email().unwrap_or("").to_string(),
            files: Vec::new(),
        };

        for (name, regex) in &mod_regexes {
            let record = records.get_mut(name).expect("inserted above");
            let Some(intro_oid) = record.introduction_oid else {
                continue;
            };
            if intro_oid == oid {
                continue;
            }
            if added_or_removed.iter().any(|l| regex.is_match(l)) {
                record.modifications.push(stats.clone());
            }
        }
    }

    Ok(records)
}

fn read_file_at_tree(repo: &Repository, tree: &git2::Tree, file_path: &Path) -> Option<String> {
    let file_str = file_path.to_string_lossy();
    let entry = tree.get_path(Path::new(file_str.as_ref())).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    std::str::from_utf8(blob.content()).ok().map(String::from)
}

/// True when `git log -G` would include this commit for `file_path`.
///
/// Only added/removed diff lines are considered (context lines are ignored).
pub fn commit_diff_matches_regex(
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

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

    let mut found = false;
    diff.foreach(
        &mut |_, _| true,
        None,
        None,
        Some(&mut |_, _, line| {
            if matches!(line.origin(), '+' | '-') {
                if let Ok(content) = std::str::from_utf8(line.content()) {
                    if regex.is_match(content) {
                        found = true;
                    }
                }
            }
            true
        }),
    )?;

    Ok(found)
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

        // Hiding HEAD excludes HEAD and ancestors — no commits in `latest..HEAD`
        let modifications = repo.find_modifications(Path::new("test.rs"), r"let x", latest_oid)?;

        assert!(
            modifications.is_empty(),
            "Expected no modifications when range excludes HEAD"
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
    fn test_find_introduction_matches_subprocess_pickaxe() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        create_and_commit_file(&repo_path, "test.rs", "fn my_func() {}", "Initial")?;
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn my_func() { println!(\"v2\"); }",
            "fix",
        )?;
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn my_func() { println!(\"v3\"); }",
            "feat",
        )?;

        let intro_output = std::process::Command::new("git")
            .args([
                "log",
                "-S",
                "fn my_func",
                "--format=%H",
                "--reverse",
                "--",
                "test.rs",
            ])
            .current_dir(&repo_path)
            .output()?;
        let cli_intro = String::from_utf8_lossy(&intro_output.stdout)
            .lines()
            .next()
            .unwrap()
            .trim()
            .to_string();

        let repo = Git2Repository::open(&repo_path)?;
        let git2_intro = repo
            .find_introduction(Path::new("test.rs"), "fn my_func")?
            .map(|(oid, _)| oid.to_string());

        assert_eq!(
            Some(cli_intro),
            git2_intro,
            "git2 find_introduction must match git log -S --reverse"
        );

        Ok(())
    }

    #[test]
    fn test_find_modifications_matches_subprocess_two_commit_case() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        create_and_commit_file(&repo_path, "test.rs", "fn my_func() {}", "Initial")?;
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn my_func() { println!(\"v2\"); }",
            "fix",
        )?;
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn my_func() { println!(\"v3\"); }",
            "feat",
        )?;

        let intro_output = std::process::Command::new("git")
            .args([
                "log",
                "-S",
                "fn my_func",
                "--format=%H",
                "--reverse",
                "--",
                "test.rs",
            ])
            .current_dir(&repo_path)
            .output()?;
        let intro_hash = String::from_utf8_lossy(&intro_output.stdout)
            .lines()
            .next()
            .unwrap()
            .trim()
            .to_string();

        let mods_output = std::process::Command::new("git")
            .args([
                "log",
                &format!("{intro_hash}..HEAD"),
                "-G",
                "my_func",
                "--format=%H",
                "--",
                "test.rs",
            ])
            .current_dir(&repo_path)
            .output()?;
        let cli_count = String::from_utf8_lossy(&mods_output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .count();

        let repo = Git2Repository::open(&repo_path)?;
        let (intro_oid, _) = repo
            .find_introduction(Path::new("test.rs"), "fn my_func")?
            .expect("intro");
        assert_eq!(
            intro_oid.to_string(),
            intro_hash,
            "intro oid must match git log -S"
        );
        let intro_from_cli = git2::Oid::from_str(&intro_hash)?;
        let regex = regex::Regex::new("my_func")?;
        let git2_mods =
            repo.find_modifications_with_regex(Path::new("test.rs"), &regex, intro_oid)?;
        let git2_mods_cli =
            repo.find_modifications_with_regex(Path::new("test.rs"), &regex, intro_from_cli)?;
        assert_eq!(
            git2_mods.len(),
            git2_mods_cli.len(),
            "intro oid source should not matter"
        );

        let git_repo = repo.open_repo()?;
        let regex = regex::Regex::new("my_func")?;
        for hash in String::from_utf8_lossy(&mods_output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
        {
            let oid = git2::Oid::from_str(hash.trim())?;
            let commit = git_repo.find_commit(oid)?;
            let matches =
                commit_diff_matches_regex(&git_repo, &commit, Path::new("test.rs"), &regex)?;
            assert!(matches, "commit {hash} should match -G per git CLI");
        }

        assert_eq!(
            cli_count,
            git2_mods.len(),
            "cli={cli_count} git2={} hashes={:?}",
            git2_mods.len(),
            git2_mods
                .iter()
                .map(|c| c.hash.to_string())
                .collect::<Vec<_>>()
        );

        Ok(())
    }

    #[test]
    fn test_pickaxe_ignores_context_only_diff_for_regex() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        let content = "fn my_func() {}\n\nfn other_func() {}\n";
        create_and_commit_file(&repo_path, "test.rs", content, "Initial")?;

        let content_v2 = "fn my_func() {}\n\nfn other_func() {\nprintln!(\"modified\");\n}\n";
        create_and_commit_file(&repo_path, "test.rs", content_v2, "fix other")?;

        let repo = Git2Repository::open(&repo_path)?;
        let git_repo = repo.open_repo()?;
        let head = git_repo.find_commit(get_head_oid(&repo_path)?)?;

        let regex = regex::Regex::new("my_func")?;
        assert!(
            !commit_diff_matches_regex(&git_repo, &head, Path::new("test.rs"), &regex)?,
            "context-only changes must not match -G"
        );

        Ok(())
    }

    #[test]
    fn test_pickaxe_counts_occurrence_changes_not_substring_presence() -> Result<()> {
        let (_temp, repo_path) = setup_test_repo()?;

        create_and_commit_file(&repo_path, "test.rs", "fn my_func() {}\n", "Initial")?;
        create_and_commit_file(
            &repo_path,
            "test.rs",
            "fn my_func() { println!(\"v2\"); }\n",
            "fix",
        )?;

        let repo = Git2Repository::open(&repo_path)?;
        let git_repo = repo.open_repo()?;
        let head = git_repo.find_commit(get_head_oid(&repo_path)?)?;

        assert!(
            !commit_pickaxe_changes_pattern(&git_repo, &head, Path::new("test.rs"), "fn my_func")?,
            "body-only edits keep the same pickaxe count"
        );

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
