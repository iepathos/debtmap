//! File-level git blame cache for efficient author lookups
//!
//! This module implements a per-file blame cache that dramatically reduces
//! git subprocess calls. Instead of calling `git blame -L start,end` for
//! each function in a file, we call `git blame --porcelain` once per file
//! and cache the results.
//!
//! # Architecture (Stillwater Philosophy)
//!
//! Following the "pure core, imperative shell" pattern:
//!
//! - **Pure functions**: `parse_full_blame_output`, `extract_authors_for_range`
//!   - Easily testable without git
//!   - No side effects
//!   - Deterministic output for given input
//!
//! - **I/O boundary**: `FileBlameCache::get_or_fetch`, `fetch_file_blame`
//!   - Single git blame call per file
//!   - Thread-safe caching with DashMap
//!
//! # Performance
//!
//! For a file with N functions:
//! - **Before**: N subprocess calls to `git blame -L`
//! - **After**: 1 subprocess call to `git blame --porcelain`
//!
//! This provides a 10x+ reduction in blame-related overhead for typical files.

use anyhow::{Context as _, Result};
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Information extracted from a single blame line
///
/// Contains the author name and commit hash for a specific line of code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameLineInfo {
    /// Author name who last modified this line
    pub author: String,
    /// Commit hash of the last modification
    pub commit_hash: String,
}

/// Cache for file-level blame data
///
/// Provides thread-safe, lock-free caching of git blame output per file.
/// Uses DashMap for concurrent access from parallel analysis with rayon.
pub struct FileBlameCache {
    /// Maps file_path -> (line_number -> blame info)
    /// Line numbers are 1-indexed (matching git blame output)
    cache: DashMap<PathBuf, HashMap<usize, BlameLineInfo>>,
    /// Root path of the git repository
    repo_root: PathBuf,
}

// =============================================================================
// Pure Functions (Testable Without Git)
// =============================================================================

/// Parse git blame --porcelain output into line-indexed map
///
/// Pure function - parses string input, returns HashMap.
///
/// # Git Blame Porcelain Format
///
/// ```text
/// <40-char-hash> <orig-line> <final-line> [<num-lines>]
/// author <author-name>
/// author-mail <author-email>
/// author-time <unix-timestamp>
/// author-tz <timezone>
/// committer <committer-name>
/// committer-mail <committer-email>
/// committer-time <unix-timestamp>
/// committer-tz <timezone>
/// summary <first-line-of-commit-message>
/// [previous <hash> <filename>]  # if line was copied
/// filename <filename>
/// \t<actual-line-content>
/// ```
///
/// For subsequent lines from the same commit, only the hash line and content are repeated.
///
/// # Arguments
/// * `blame_output` - Raw output from `git blame --porcelain <file>`
///
/// # Returns
/// HashMap mapping line numbers (1-indexed) to BlameLineInfo
pub fn parse_full_blame_output(blame_output: &str) -> HashMap<usize, BlameLineInfo> {
    let mut result = HashMap::new();
    let mut current_commit: Option<String> = None;
    let mut current_author: Option<String> = None;
    let mut current_line: Option<usize> = None;

    for line in blame_output.lines() {
        // Check for commit header line: "<hash> <orig_line> <final_line> [<num_lines>]"
        if line.len() >= 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit()) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let new_commit = parts[0].to_string();
                // parts[2] is the final line number in the output
                if let Ok(line_num) = parts[2].parse::<usize>() {
                    current_line = Some(line_num);
                }
                // Reset author for new commit block (will be set if header follows)
                // Don't reset if this is a continuation line from the same commit
                if parts.len() >= 4 {
                    // This is a new commit block (has num_lines), author header will follow
                    current_author = None;
                } else if current_commit.as_ref() != Some(&new_commit) {
                    // Different commit but no full header - look up author from cache
                    current_author = result
                        .values()
                        .find(|info: &&BlameLineInfo| info.commit_hash == new_commit)
                        .map(|info| Some(info.author.clone()))
                        .unwrap_or(None);
                }
                // Only update current_commit after we've done the comparison
                current_commit = Some(new_commit);
            }
        } else if let Some(author_name) = line.strip_prefix("author ") {
            current_author = Some(author_name.to_string());
        } else if line.starts_with('\t') {
            // Content line - commit the blame info for this line
            if let (Some(ref commit), Some(line_num)) = (&current_commit, current_line) {
                // Use the author from the current commit block, or look up from previous entry
                let author = current_author.clone().unwrap_or_else(|| {
                    // For continuation lines, find the author from a previous line with same commit
                    result
                        .values()
                        .find(|info: &&BlameLineInfo| info.commit_hash == *commit)
                        .map(|info| info.author.clone())
                        .unwrap_or_default()
                });

                result.insert(
                    line_num,
                    BlameLineInfo {
                        author,
                        commit_hash: commit.clone(),
                    },
                );
            }
        }
    }

    result
}

/// Extract unique authors for a line range from cached data
///
/// Pure function - O(n) iteration over line range, O(1) lookups.
///
/// # Arguments
/// * `blame_data` - Cached blame data for a file
/// * `start_line` - Start line number (1-indexed, inclusive)
/// * `end_line` - End line number (1-indexed, inclusive)
///
/// # Returns
/// HashSet of unique author names (excluding "Not Committed Yet")
pub fn extract_authors_for_range(
    blame_data: &HashMap<usize, BlameLineInfo>,
    start_line: usize,
    end_line: usize,
) -> HashSet<String> {
    (start_line..=end_line)
        .filter_map(|line| blame_data.get(&line))
        .map(|info| info.author.clone())
        .filter(|author| !author.is_empty() && author != "Not Committed Yet")
        .collect()
}

// =============================================================================
// I/O Boundary (Imperative Shell)
// =============================================================================

impl FileBlameCache {
    /// Create a new FileBlameCache for a git repository
    ///
    /// # Arguments
    /// * `repo_root` - Path to the git repository root
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            cache: DashMap::new(),
            repo_root,
        }
    }

    /// Get or fetch blame data for entire file (I/O boundary)
    ///
    /// Uses lock-free read for cache hits. On cache miss, fetches the
    /// complete blame output and parses it into the cache.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file (relative to repo root)
    ///
    /// # Returns
    /// Reference to the cached blame data, or error if fetch fails
    pub fn get_or_fetch(&self, file_path: &Path) -> Result<HashMap<usize, BlameLineInfo>> {
        // Try cache first (lock-free read)
        if let Some(cached) = self.cache.get(file_path) {
            return Ok(cached.clone());
        }

        // I/O: Single git blame call for entire file
        let blame_output = self.fetch_file_blame(file_path)?;

        // Pure: Parse the output
        let blame_data = parse_full_blame_output(&blame_output);

        // Cache for future lookups
        self.cache
            .insert(file_path.to_path_buf(), blame_data.clone());

        Ok(blame_data)
    }

    /// Fetch complete blame for a file (I/O)
    ///
    /// Runs `git blame --porcelain <file>` to get the full blame output.
    /// Returns empty string for untracked or binary files.
    fn fetch_file_blame(&self, file_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(["blame", "--porcelain", &file_path.to_string_lossy()])
            .current_dir(&self.repo_root)
            .output()
            .context("Failed to run git blame")?;

        if !output.status.success() {
            // Return empty for untracked/binary files
            return Ok(String::new());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get authors for a function's line range (public API)
    ///
    /// Fetches blame data if not cached, then extracts unique authors
    /// for the specified line range.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file (relative to repo root)
    /// * `start_line` - Start line number (1-indexed, inclusive)
    /// * `end_line` - End line number (1-indexed, inclusive)
    ///
    /// # Returns
    /// HashSet of unique author names for the line range
    pub fn get_authors(
        &self,
        file_path: &Path,
        start_line: usize,
        end_line: usize,
    ) -> Result<HashSet<String>> {
        let blame_data = self.get_or_fetch(file_path)?;
        Ok(extract_authors_for_range(&blame_data, start_line, end_line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Pure Function Tests (No Git Required)
    // =========================================================================

    #[test]
    fn test_parse_full_blame_output_single_author() {
        // Build the test data with explicit tab characters for content lines
        let blame_output = [
            "abc123def456789012345678901234567890abcd 1 1 3",
            "author John Doe",
            "author-mail <john@example.com>",
            "author-time 1234567890",
            "author-tz +0000",
            "committer Jane Smith",
            "committer-mail <jane@example.com>",
            "committer-time 1234567890",
            "committer-tz +0000",
            "summary Initial commit",
            "filename src/test.rs",
            "\tfn foo() {",
            "abc123def456789012345678901234567890abcd 2 2",
            "\t    bar();",
            "abc123def456789012345678901234567890abcd 3 3",
            "\t}",
        ]
        .join("\n");

        let blame_data = parse_full_blame_output(&blame_output);

        assert_eq!(blame_data.len(), 3);
        assert_eq!(blame_data.get(&1).unwrap().author, "John Doe");
        assert_eq!(
            blame_data.get(&1).unwrap().commit_hash,
            "abc123def456789012345678901234567890abcd"
        );
        assert_eq!(blame_data.get(&2).unwrap().author, "John Doe");
        assert_eq!(blame_data.get(&3).unwrap().author, "John Doe");
    }

    #[test]
    fn test_parse_full_blame_output_multiple_authors() {
        // Build the test data with explicit tab characters for content lines
        let blame_output = [
            "abc123def456789012345678901234567890abcd 1 1 2",
            "author Alice",
            "author-mail <alice@example.com>",
            "author-time 1234567890",
            "author-tz +0000",
            "committer Alice",
            "committer-mail <alice@example.com>",
            "committer-time 1234567890",
            "committer-tz +0000",
            "summary First commit",
            "filename test.rs",
            "\tfn foo() {",
            "abc123def456789012345678901234567890abcd 2 2",
            "\t    // Alice's code",
            "def456abc78901234567890123456789012bcdef 3 3 2",
            "author Bob",
            "author-mail <bob@example.com>",
            "author-time 1234567891",
            "author-tz +0000",
            "committer Bob",
            "committer-mail <bob@example.com>",
            "committer-time 1234567891",
            "committer-tz +0000",
            "summary Second commit",
            "filename test.rs",
            "\t    bar();",
            "def456abc78901234567890123456789012bcdef 4 4",
            "\t}",
        ]
        .join("\n");

        let blame_data = parse_full_blame_output(&blame_output);

        assert_eq!(blame_data.len(), 4);
        assert_eq!(blame_data.get(&1).unwrap().author, "Alice");
        assert_eq!(blame_data.get(&2).unwrap().author, "Alice");
        assert_eq!(blame_data.get(&3).unwrap().author, "Bob");
        assert_eq!(blame_data.get(&4).unwrap().author, "Bob");
    }

    #[test]
    fn test_parse_full_blame_output_empty() {
        let blame_output = "";
        let blame_data = parse_full_blame_output(blame_output);
        assert!(blame_data.is_empty());
    }

    #[test]
    fn test_extract_authors_for_range_basic() {
        let mut blame_data = HashMap::new();
        blame_data.insert(
            1,
            BlameLineInfo {
                author: "Alice".into(),
                commit_hash: "abc".into(),
            },
        );
        blame_data.insert(
            2,
            BlameLineInfo {
                author: "Bob".into(),
                commit_hash: "def".into(),
            },
        );
        blame_data.insert(
            3,
            BlameLineInfo {
                author: "Alice".into(),
                commit_hash: "abc".into(),
            },
        );
        blame_data.insert(
            4,
            BlameLineInfo {
                author: "Charlie".into(),
                commit_hash: "ghi".into(),
            },
        );

        let authors = extract_authors_for_range(&blame_data, 1, 3);

        assert_eq!(authors.len(), 2);
        assert!(authors.contains("Alice"));
        assert!(authors.contains("Bob"));
        assert!(!authors.contains("Charlie"));
    }

    #[test]
    fn test_extract_authors_for_range_single_line() {
        let mut blame_data = HashMap::new();
        blame_data.insert(
            5,
            BlameLineInfo {
                author: "Dave".into(),
                commit_hash: "xyz".into(),
            },
        );

        let authors = extract_authors_for_range(&blame_data, 5, 5);

        assert_eq!(authors.len(), 1);
        assert!(authors.contains("Dave"));
    }

    #[test]
    fn test_extract_authors_filters_not_committed() {
        let mut blame_data = HashMap::new();
        blame_data.insert(
            1,
            BlameLineInfo {
                author: "Alice".into(),
                commit_hash: "abc".into(),
            },
        );
        blame_data.insert(
            2,
            BlameLineInfo {
                author: "Not Committed Yet".into(),
                commit_hash: "0000000000000000000000000000000000000000".into(),
            },
        );

        let authors = extract_authors_for_range(&blame_data, 1, 2);

        assert_eq!(authors.len(), 1);
        assert!(authors.contains("Alice"));
        assert!(!authors.contains("Not Committed Yet"));
    }

    #[test]
    fn test_extract_authors_empty_range() {
        let blame_data = HashMap::new();
        let authors = extract_authors_for_range(&blame_data, 100, 200);
        assert!(authors.is_empty());
    }

    #[test]
    fn test_extract_authors_partial_range() {
        let mut blame_data = HashMap::new();
        blame_data.insert(
            5,
            BlameLineInfo {
                author: "Eve".into(),
                commit_hash: "abc".into(),
            },
        );
        blame_data.insert(
            7,
            BlameLineInfo {
                author: "Frank".into(),
                commit_hash: "def".into(),
            },
        );
        // Missing lines 6, 8, 9, 10

        let authors = extract_authors_for_range(&blame_data, 5, 10);

        assert_eq!(authors.len(), 2);
        assert!(authors.contains("Eve"));
        assert!(authors.contains("Frank"));
    }

    #[test]
    fn test_extract_authors_filters_empty_names() {
        let mut blame_data = HashMap::new();
        blame_data.insert(
            1,
            BlameLineInfo {
                author: "Alice".into(),
                commit_hash: "abc".into(),
            },
        );
        blame_data.insert(
            2,
            BlameLineInfo {
                author: "".into(),
                commit_hash: "def".into(),
            },
        );

        let authors = extract_authors_for_range(&blame_data, 1, 2);

        assert_eq!(authors.len(), 1);
        assert!(authors.contains("Alice"));
    }

    #[test]
    fn test_blame_line_info_equality() {
        let info1 = BlameLineInfo {
            author: "Alice".into(),
            commit_hash: "abc123".into(),
        };
        let info2 = BlameLineInfo {
            author: "Alice".into(),
            commit_hash: "abc123".into(),
        };
        let info3 = BlameLineInfo {
            author: "Bob".into(),
            commit_hash: "abc123".into(),
        };

        assert_eq!(info1, info2);
        assert_ne!(info1, info3);
    }
}
