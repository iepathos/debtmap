use super::{AnalysisTarget, Context, ContextDetails, ContextProvider};
use crate::core::FunctionMetrics;
use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

mod batched;
pub mod batched_function;
mod blame_cache;
mod function_level;
pub mod git2_provider;
mod stability;

#[cfg(test)]
mod test_helpers;
#[cfg(test)]
mod tests;

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
    batched_functions: Option<batched_function::BatchedFunctionGitHistory>,
    /// Cache for file-level git blame data (uses git2 library)
    blame_cache: blame_cache::FileBlameCache,
    /// Git2 repository wrapper for reliable git operations
    git2_repo: Option<git2_provider::Git2Repository>,
}

impl GitHistoryProvider {
    pub fn new(repo_root: PathBuf) -> Result<Self> {
        // Use git2 to verify this is a git repository (more reliable path handling)
        let git2_repo = match git2_provider::Git2Repository::open(&repo_root) {
            Ok(repo) => {
                log::debug!(
                    "git2 repository opened successfully at {}",
                    repo_root.display()
                );
                Some(repo)
            }
            Err(e) => {
                anyhow::bail!("Not a git repository: {} ({})", repo_root.display(), e);
            }
        };

        // Use the git2 repository's workdir as the canonical repo_root
        // This ensures path lookups match what git stores (relative to .git parent)
        let canonical_repo_root = git2_repo
            .as_ref()
            .map(|r| r.repo_path().to_path_buf())
            .unwrap_or(repo_root);

        // File-level batched history is built during function preload (one commit walk).
        let batched_history = None;

        // Create blame cache for efficient per-file blame lookups (now uses git2)
        let blame_cache =
            blame_cache::FileBlameCache::new(canonical_repo_root.clone(), git2_repo.as_ref());

        Ok(Self {
            repo_root: canonical_repo_root,
            cache: Arc::new(DashMap::new()),
            batched_history,
            batched_functions: None,
            blame_cache,
            git2_repo,
        })
    }

    /// Preload per-function git histories for all metrics (parallel, single commit walk).
    pub fn preload_function_histories(&mut self, metrics: &[FunctionMetrics]) -> Result<()> {
        self.preload_function_histories_with_progress(metrics, None)
    }

    /// Like `preload_function_histories`, but reports `(processed, total)` commit
    /// progress via the callback (best-effort, ~every 50 commits).
    pub fn preload_function_histories_with_progress(
        &mut self,
        metrics: &[FunctionMetrics],
        progress_cb: Option<batched_function::ProgressCallback<'_>>,
    ) -> Result<()> {
        let Some(ref repo) = self.git2_repo else {
            return Ok(());
        };

        let targets: Vec<batched_function::FunctionPreloadTarget> = metrics
            .iter()
            .filter(|m| !m.name.is_empty())
            .map(|m| batched_function::FunctionPreloadTarget {
                file: self.to_relative_path(&m.file).into_owned(),
                name: m.name.clone(),
                line_range: (m.line, m.line.saturating_add(m.length.max(1))),
            })
            .collect();

        let start = Instant::now();
        let scan = batched_function::BatchedFunctionGitHistory::build(
            repo,
            &self.blame_cache,
            &targets,
            progress_cb,
        )?;
        self.batched_functions = Some(scan.functions);
        self.batched_history = Some(scan.file_history);
        log::info!(
            "Function git history preload: {} functions in {:?}",
            self.batched_functions
                .as_ref()
                .map(batched_function::BatchedFunctionGitHistory::len)
                .unwrap_or(0),
            start.elapsed()
        );
        Ok(())
    }

    /// Convert a path to be relative to the repo root.
    ///
    /// Git stores paths relative to the repo root, so we need to strip the repo_root
    /// prefix from absolute paths for lookups in batched history.
    ///
    /// Handles:
    /// - Absolute paths: strips repo_root prefix
    /// - Symlinks (e.g., macOS /var -> /private/var): canonicalizes before comparison
    /// - Current directory prefix (./): strips ./ prefix for relative paths
    fn to_relative_path<'a>(&self, path: &'a Path) -> std::borrow::Cow<'a, Path> {
        // Fast path: try direct strip_prefix first
        if let Ok(rel) = path.strip_prefix(&self.repo_root) {
            return std::borrow::Cow::Borrowed(rel);
        }

        // Handle ./ prefix for relative paths (e.g., ./src/file.ts -> src/file.ts)
        if let Ok(rel) = path.strip_prefix("./") {
            return std::borrow::Cow::Borrowed(rel);
        }

        // Also handle . prefix (single dot component)
        if let Ok(rel) = path.strip_prefix(".") {
            // strip_prefix(".") returns empty path if path is exactly "."
            if !rel.as_os_str().is_empty() {
                return std::borrow::Cow::Borrowed(rel);
            }
        }

        // Slow path: canonicalize both paths to resolve symlinks (e.g., /var -> /private/var)
        if path.is_absolute() {
            if let (Ok(canonical_path), Ok(canonical_root)) =
                (path.canonicalize(), self.repo_root.canonicalize())
            {
                if let Ok(rel) = canonical_path.strip_prefix(&canonical_root) {
                    return std::borrow::Cow::Owned(rel.to_path_buf());
                }
            }
        }

        // Return original path if no stripping was possible
        std::borrow::Cow::Borrowed(path)
    }

    /// Get file history from cache or fetch it (immutable, thread-safe)
    fn get_or_fetch_history(&self, path: &Path, now: DateTime<Utc>) -> Result<FileHistory> {
        // Convert to relative path for git lookups (git stores relative paths)
        let relative_path = self.to_relative_path(path);

        // Try cache first (lock-free read) - use relative path for consistency
        if let Some(cached) = self.cache.get(relative_path.as_ref()) {
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
            )) = batched.calculate_metrics(relative_path.as_ref(), now)
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
                self.cache
                    .insert(relative_path.into_owned(), history.clone());
                return Ok(history);
            }
        }

        // Fallback to direct git queries (slow path)
        let history = self.fetch_history_direct(relative_path.as_ref(), now)?;
        self.cache
            .insert(relative_path.into_owned(), history.clone());
        Ok(history)
    }

    /// Fetch file history directly via git2 (fallback when batched fails)
    fn fetch_history_direct(&self, path: &Path, now: DateTime<Utc>) -> Result<FileHistory> {
        if let Some(ref repo) = self.git2_repo {
            // Use git2 for reliable path handling
            let total_commits = repo.count_file_commits(path)?;
            let age_days = repo.file_age_days(path, now)?;
            let bug_fix_count = repo.count_bug_fixes(path)?;
            let author_count = repo.file_authors(path)?.len();
            let last_modified = repo.file_last_modified(path)?;

            let change_frequency = if age_days > 0 {
                (total_commits as f64) / (age_days as f64) * 30.0
            } else {
                0.0
            };

            let stability_score =
                self.calculate_stability_from_values(age_days, total_commits, bug_fix_count);

            Ok(FileHistory {
                change_frequency,
                bug_fix_count,
                last_modified,
                author_count,
                stability_score,
                total_commits,
                age_days,
            })
        } else {
            // Fallback to default values if git2 is not available
            log::warn!("git2 not available, returning default history");
            Ok(FileHistory {
                change_frequency: 0.0,
                bug_fix_count: 0,
                last_modified: None,
                author_count: 0,
                stability_score: 1.0,
                total_commits: 0,
                age_days: 0,
            })
        }
    }

    /// Calculate stability score from pre-computed values
    fn calculate_stability_from_values(
        &self,
        age_days: u32,
        commits: usize,
        bug_fixes: usize,
    ) -> f64 {
        if commits == 0 {
            return 1.0; // New file, assume stable
        }

        let churn_factor = if age_days > 0 {
            let monthly_churn = (commits as f64) / (age_days as f64) * 30.0;
            1.0 / (1.0 + monthly_churn)
        } else {
            0.5
        };

        let bug_factor = 1.0 - (bug_fixes as f64 / commits as f64).min(1.0);
        let age_factor = (age_days as f64 / 365.0).min(1.0);

        (churn_factor * 0.4 + bug_factor * 0.4 + age_factor * 0.2).min(1.0)
    }

    /// Legacy mutable API - kept for backward compatibility
    pub fn analyze_file(&mut self, path: &Path) -> Result<FileHistory> {
        self.get_or_fetch_history(path, Utc::now())
    }

    /// Get file history with explicit reference time
    pub fn analyze_file_with_time(&self, path: &Path, now: DateTime<Utc>) -> Result<FileHistory> {
        self.get_or_fetch_history(path, now)
    }

    /// Get all paths stored in batched history (for debugging/testing)
    #[cfg(test)]
    pub fn batched_paths(&self) -> Vec<std::path::PathBuf> {
        self.batched_history
            .as_ref()
            .map(|b| b.all_paths().into_iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Check if a path exists in batched history (for debugging/testing)
    #[cfg(test)]
    pub fn batched_has_path(&self, path: &Path) -> bool {
        self.batched_history
            .as_ref()
            .map(|b| b.has_path(path))
            .unwrap_or(false)
    }

    /// Get the repo root path (for debugging/testing)
    #[cfg(test)]
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
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
                ..
            } => stability::explain_historical_context(
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
    /// Uses cached `git blame` on current lines to identify contributors.
    fn gather_for_function(&self, target: &AnalysisTarget) -> Result<Context> {
        let relative_path = self.to_relative_path(&target.file_path);
        let history = self
            .lookup_function_history(relative_path.as_ref(), target)?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No function history for '{}' in {}",
                    target.function_name,
                    relative_path.display()
                )
            })?;

        Ok(Self::context_from_function_history(
            self.name(),
            self.weight(),
            &history,
            target.reference_time,
        ))
    }

    fn context_from_function_history(
        provider_name: &str,
        weight: f64,
        history: &function_level::FunctionHistory,
        reference_time: DateTime<Utc>,
    ) -> Context {
        let contribution = stability::classify_risk_contribution(
            history.change_frequency(reference_time),
            history.bug_density(),
        );

        Context {
            provider: provider_name.to_string(),
            weight,
            contribution,
            details: ContextDetails::Historical {
                change_frequency: history.change_frequency(reference_time),
                bug_density: history.bug_density(),
                age_days: history.age_days(reference_time),
                author_count: history.authors.len(),
                total_commits: history.total_commits_including_introduction() as u32,
                bug_fix_count: history.bug_fix_count as u32,
            },
        }
    }

    fn lookup_function_history(
        &self,
        relative_path: &Path,
        target: &AnalysisTarget,
    ) -> Result<Option<function_level::FunctionHistory>> {
        if let Some(ref batched) = self.batched_functions {
            if let Some(history) = batched.get(relative_path, &target.function_name) {
                return Ok(Some(history));
            }
        }

        if let Some(ref repo) = self.git2_repo {
            return function_level::get_function_history_git2(
                repo,
                relative_path,
                &target.function_name,
                target.line_range,
                &self.blame_cache,
            )
            .map(Some);
        }

        function_level::get_function_history(
            &self.repo_root,
            relative_path,
            &target.function_name,
            target.line_range,
            &self.blame_cache,
            target.reference_time,
        )
        .map(Some)
    }

    /// Gather context using file-level git history analysis (fallback)
    fn gather_for_file(&self, target: &AnalysisTarget) -> Result<Context> {
        // Use cached/batched history (O(1) lookup, no git subprocess calls)
        let history = self.get_or_fetch_history(&target.file_path, target.reference_time)?;

        // Calculate contribution based on instability
        let bug_density =
            stability::calculate_bug_density(history.bug_fix_count, history.total_commits);

        let contribution =
            stability::classify_risk_contribution(history.change_frequency, bug_density);

        Ok(Context {
            provider: self.name().to_string(),
            weight: self.weight(),
            contribution,
            details: ContextDetails::Historical {
                change_frequency: history.change_frequency,
                bug_density,
                age_days: history.age_days,
                author_count: history.author_count,
                total_commits: history.total_commits as u32,
                bug_fix_count: history.bug_fix_count as u32,
            },
        })
    }
}
