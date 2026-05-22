//! Batched function-level git history (preload at context load time).
//!
//! One repository-wide revwalk drives per-file accounting of `-S`
//! introductions and `-G` modifications, matching `git log` semantics
//! without per-function subprocess fan-out.

use super::blame_cache::{extract_authors_for_range, FileBlameCache};
use super::function_level::{calculate_function_history_with_authors, CommitInfo, FunctionHistory};
use super::git2_provider::{self, Git2Repository};
use crate::time_span;
use anyhow::Result;
use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// One function to preload history for (paths relative to repo root).
#[derive(Debug, Clone)]
pub struct FunctionPreloadTarget {
    pub file: PathBuf,
    pub name: String,
    pub line_range: (usize, usize),
}

/// Cache key for function history lookups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionHistoryKey {
    pub file: PathBuf,
    pub name: String,
}

/// Phase of function git history preload (for TUI progress).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitPreloadPhase {
    /// Scanning repository commits for `-S`/`-G` history.
    Commits,
    /// Running per-file blame to attribute authors.
    BlameFiles,
}

/// Progress callback: `(phase, processed, total)`.
pub type ProgressCallback<'a> = &'a (dyn Fn(GitPreloadPhase, usize, usize) + Send + Sync);

/// Preloaded function histories for O(1) lookup during scoring.
pub struct BatchedFunctionGitHistory {
    histories: DashMap<FunctionHistoryKey, FunctionHistory>,
}

/// Result of batched function git history preload.
pub struct FunctionPreloadResult {
    pub functions: BatchedFunctionGitHistory,
    pub file_history: super::batched::BatchedGitHistory,
}

impl BatchedFunctionGitHistory {
    pub fn build(
        repo: &Git2Repository,
        blame_cache: &FileBlameCache,
        targets: &[FunctionPreloadTarget],
        progress_cb: Option<ProgressCallback<'_>>,
    ) -> Result<FunctionPreloadResult> {
        time_span!("git_function_history_preload");
        let start = Instant::now();

        let by_file = group_targets_by_file(targets);
        let file_count = by_file.len();
        let file_names: HashMap<PathBuf, Vec<String>> = by_file
            .iter()
            .map(|(file, fns)| (file.clone(), fns.iter().map(|(n, _)| n.clone()).collect()))
            .collect();

        let scan = git2_provider::compute_repo_function_histories(
            repo.repo_path(),
            &file_names,
            progress_cb,
        )?;
        let records = scan.functions;
        let file_history = super::batched::BatchedGitHistory::from_commit_scans(&scan.file_scans);

        let histories: DashMap<FunctionHistoryKey, FunctionHistory> = DashMap::new();
        let processed_files = AtomicUsize::new(0);
        if let Some(cb) = progress_cb {
            cb(GitPreloadPhase::BlameFiles, 0, file_count);
        }

        by_file.par_iter().for_each(|(file, functions)| {
            let blame_data = blame_cache.get_or_fetch(file).ok();
            for (name, line_range) in functions {
                let Some(record) = records.get(&(file.clone(), name.clone())) else {
                    continue;
                };
                if record.introduction_oid.is_none() {
                    continue;
                }
                let (start_line, end_line) = *line_range;
                let blame_authors = blame_data
                    .as_ref()
                    .map(|data| extract_authors_for_range(data, start_line, end_line))
                    .unwrap_or_default();
                let modification_commits: Vec<CommitInfo> = record
                    .modifications
                    .iter()
                    .map(|s| CommitInfo {
                        hash: s.hash.to_string(),
                        date: Some(s.date),
                        message: s.message.clone(),
                        author: s.author_email.clone(),
                    })
                    .collect();
                let history = calculate_function_history_with_authors(
                    record.introduction_oid.map(|o| o.to_string()),
                    record.introduction_date,
                    &modification_commits,
                    blame_authors,
                );
                histories.insert(
                    FunctionHistoryKey {
                        file: file.clone(),
                        name: name.clone(),
                    },
                    history,
                );
            }

            let done = processed_files.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(cb) = progress_cb {
                if done % 10 == 0 || done == file_count {
                    cb(GitPreloadPhase::BlameFiles, done, file_count);
                }
            }
        });

        log::info!(
            "Preloaded {} function histories from {} files in {:?}",
            histories.len(),
            file_count,
            start.elapsed()
        );

        Ok(FunctionPreloadResult {
            functions: Self { histories },
            file_history,
        })
    }

    pub fn get(&self, file: &Path, function_name: &str) -> Option<FunctionHistory> {
        self.histories
            .get(&FunctionHistoryKey {
                file: file.to_path_buf(),
                name: function_name.to_string(),
            })
            .map(|entry| entry.clone())
    }

    pub fn len(&self) -> usize {
        self.histories.len()
    }
}

type FileTargetMap = HashMap<PathBuf, Vec<(String, (usize, usize))>>;

fn group_targets_by_file(targets: &[FunctionPreloadTarget]) -> FileTargetMap {
    let mut by_file: FileTargetMap = HashMap::new();
    for target in targets {
        if target.name.is_empty() {
            continue;
        }
        let entries = by_file.entry(target.file.clone()).or_default();
        if entries.iter().any(|(name, _)| name == &target.name) {
            continue;
        }
        entries.push((target.name.clone(), target.line_range));
    }
    by_file
}
