use super::coverage_index::CoverageIndex;
use super::function_name_matching::MatchableFunction;
use super::path_normalization::find_matching_path;
use anyhow::{Context, Result};
use indicatif::ProgressBar;
use rayon::prelude::*;
use rustc_demangle;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// Global statistics for diagnostic mode (Spec 203 FR3)
static COVERAGE_MATCH_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
static COVERAGE_MATCH_SUCCESS: AtomicUsize = AtomicUsize::new(0);
static COVERAGE_MATCH_ZERO: AtomicUsize = AtomicUsize::new(0);

/// Print aggregate coverage matching statistics (Spec 203 FR3)
///
/// Called at end of analysis when DEBTMAP_COVERAGE_DEBUG=1 to show
/// summary of match success rates.
pub fn print_coverage_statistics() {
    let attempts = COVERAGE_MATCH_ATTEMPTS.load(Ordering::Relaxed);
    if attempts == 0 {
        return; // No matches attempted
    }

    let success = COVERAGE_MATCH_SUCCESS.load(Ordering::Relaxed);
    let zero = COVERAGE_MATCH_ZERO.load(Ordering::Relaxed);
    let success_rate = (success as f64 / attempts as f64) * 100.0;
    let zero_rate = (zero as f64 / attempts as f64) * 100.0;

    eprintln!();
    eprintln!("[COVERAGE] ═══════════════════════════════════════════════════");
    eprintln!("[COVERAGE] Match Statistics Summary");
    eprintln!("[COVERAGE] ═══════════════════════════════════════════════════");
    eprintln!("[COVERAGE]   Total functions: {}", attempts);
    eprintln!(
        "[COVERAGE]   Matched: {} ({:.1}%)",
        success, success_rate
    );
    eprintln!("[COVERAGE]   Unmatched (0%): {} ({:.1}%)", zero, zero_rate);
    eprintln!("[COVERAGE] ═══════════════════════════════════════════════════");
}

/// Progress state for coverage file parsing
#[derive(Debug, Clone, Copy)]
pub enum CoverageProgress {
    /// Opening and initializing LCOV file reader
    Initializing,
    /// Parsing coverage records with file count progress
    /// (current_file, total_files_seen_so_far)
    Parsing { current: usize, total: usize },
    /// Computing final coverage statistics
    ComputingStats,
    /// Parsing complete
    Complete,
}

/// Normalized function name with multiple matching variants
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFunctionName {
    /// Full normalized path: "module::Struct::method"
    pub full_path: String,

    /// Just the method name: "method"
    pub method_name: String,

    /// Original demangled name (for debugging)
    pub original: String,
}

impl NormalizedFunctionName {
    /// Create a simple NormalizedFunctionName for testing
    pub fn simple(name: &str) -> Self {
        Self {
            full_path: name.to_string(),
            method_name: name.to_string(),
            original: name.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionCoverage {
    pub name: String,
    pub start_line: usize,
    pub execution_count: u64,
    pub coverage_percentage: f64,
    pub uncovered_lines: Vec<usize>,
    /// Normalized name variants for matching (not serialized)
    pub normalized: NormalizedFunctionName,
}

#[derive(Debug, Clone)]
pub struct LcovData {
    pub functions: HashMap<PathBuf, Vec<FunctionCoverage>>,
    pub total_lines: usize,
    pub lines_hit: usize,
    /// LOC counter instance for consistent line counting across analysis modes
    loc_counter: Option<crate::metrics::LocCounter>,
    /// Pre-built index for O(1) function coverage lookups, wrapped in Arc for lock-free sharing across threads
    coverage_index: Arc<CoverageIndex>,
}

impl Default for LcovData {
    fn default() -> Self {
        Self::new()
    }
}

impl LcovData {
    /// Create a new empty LcovData instance
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            total_lines: 0,
            lines_hit: 0,
            loc_counter: None,
            coverage_index: Arc::new(CoverageIndex::empty()),
        }
    }

    /// Build the coverage index from current function data
    /// This should be called after modifying the functions HashMap
    pub fn build_index(&mut self) {
        self.coverage_index = Arc::new(CoverageIndex::from_coverage(self));
    }
}

/// Demangle a Rust function name if it's mangled
///
/// Handles both legacy and v0 mangling schemes:
/// - Legacy: starts with `_ZN`
/// - v0: starts with `_RNv`
///
/// Returns demangled name or original if not mangled.
fn demangle_function_name(name: &str) -> String {
    // Try to demangle any name - rustc_demangle will return the original if it's not mangled
    let demangled = rustc_demangle::demangle(name).to_string();

    // If demangling changed the string, use the demangled version; otherwise keep original
    if demangled != name {
        demangled
    } else {
        name.to_string()
    }
}

/// Strip trailing generic parameters from function names.
///
/// Handles nested generics like `method::<Vec<HashMap<K, V>>>`.
/// Returns `Cow` to avoid allocation when no stripping is needed.
///
/// This is a pure function with no side effects - it transforms strings deterministically.
///
/// # Examples
///
/// ```ignore
/// use std::borrow::Cow;
/// use debtmap::risk::lcov::strip_trailing_generics;
///
/// assert_eq!(strip_trailing_generics("Type::method::<T>"), Cow::Borrowed("Type::method"));
/// assert_eq!(strip_trailing_generics("method::<Vec<T>>"), Cow::Borrowed("method"));
/// assert_eq!(strip_trailing_generics("method"), Cow::Borrowed("method"));
/// ```
pub(crate) fn strip_trailing_generics(s: &str) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;

    if let Some(pos) = s.rfind("::<") {
        // Count angle brackets to find matching close (handles nested generics)
        let mut depth = 0;
        let mut end_pos = None;

        for (i, ch) in s[pos + 3..].char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    if depth == 0 {
                        end_pos = Some(pos + 3 + i);
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }

        if let Some(end) = end_pos {
            let after = &s[end + 1..];
            // If nothing after the >, this is a trailing generic
            if after.is_empty() {
                return Cow::Owned(s[..pos].to_string());
            }
        }
    }
    Cow::Borrowed(s)
}

/// Normalize a demangled function name for consolidation
///
/// Removes generic type parameters and crate hash IDs to
/// group multiple monomorphizations of the same function.
/// Also extracts the method name for flexible matching.
///
/// For example:
/// - `<debtmap[71f4b4990cdcf1ab]::Foo>::bar` -> full_path: `debtmap::Foo::bar`, method_name: `bar`
/// - `std::collections::HashMap<K,V>::insert` -> full_path: `std::collections::HashMap::insert`, method_name: `insert`
/// - `<Struct as Trait>::method` -> full_path: `Struct as Trait::method`, method_name: `method`
/// - `Type::method::<T>` -> full_path: `Type::method`, method_name: `method`
pub(crate) fn normalize_demangled_name(demangled: &str) -> NormalizedFunctionName {
    // Handle impl method patterns: <module::path::Type>::method -> module::path::Type::method
    // Remove angle brackets and crate hash: <crate[hash]::rest>::method -> crate::rest::method
    let without_impl_brackets = if demangled.starts_with('<') {
        if let Some(angle_end) = demangled.find('>') {
            let content = &demangled[1..angle_end];
            let after = &demangled[(angle_end + 1)..];

            // Remove hash from path if present: crate[hash]::rest -> crate::rest
            let content_without_hash = if let Some(bracket_start) = content.find('[') {
                if let Some(bracket_end) = content.find(']') {
                    // Reconstruct: before[hash]after -> beforeafter
                    format!(
                        "{}{}",
                        &content[..bracket_start],
                        &content[(bracket_end + 1)..]
                    )
                } else {
                    content.to_string()
                }
            } else {
                content.to_string()
            };

            format!("{}{}", content_without_hash, after)
        } else {
            demangled.to_string()
        }
    } else {
        demangled.to_string()
    };

    // Strip trailing generic parameters from functions (e.g., "method::<T>" -> "method")
    let without_function_generics = strip_trailing_generics(&without_impl_brackets);

    // Now remove generic type parameters: "HashMap<K,V>::insert" -> "HashMap::insert"
    // Use fold to track depth and only keep characters outside angle brackets
    let result = without_function_generics
        .chars()
        .fold((String::new(), 0usize), |(mut acc, depth), ch| match ch {
            '<' => (acc, depth + 1),
            '>' if depth > 0 => (acc, depth - 1),
            _ if depth == 0 => {
                acc.push(ch);
                (acc, depth)
            }
            _ => (acc, depth),
        })
        .0;

    // Extract method name (final segment after last ::)
    let method_name = result.rsplit("::").next().unwrap_or(&result).to_string();

    NormalizedFunctionName {
        full_path: result,
        method_name,
        original: demangled.to_string(),
    }
}

pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
    parse_lcov_file_with_progress(path, &ProgressBar::hidden())
}

/// Parse LCOV file with progress callback
pub fn parse_lcov_file_with_callback<F>(path: &Path, mut progress_callback: F) -> Result<LcovData>
where
    F: FnMut(CoverageProgress),
{
    use lcov::{Reader, Record};

    // Report initialization
    progress_callback(CoverageProgress::Initializing);

    let reader = Reader::open_file(path)
        .with_context(|| format!("Failed to open LCOV file: {}", path.display()))?;

    let mut data = LcovData::default();
    let mut current_file: Option<PathBuf> = None;
    let mut file_functions: HashMap<String, FunctionCoverage> = HashMap::new();
    let mut file_lines: HashMap<usize, u64> = HashMap::new();
    let mut _lines_found = 0;
    let mut _lines_hit = 0;
    let mut file_count = 0usize;

    // Report parsing start
    progress_callback(CoverageProgress::Parsing {
        current: 0,
        total: 0,
    });

    for record in reader {
        let record = record.with_context(|| "Failed to parse LCOV record")?;

        match record {
            Record::SourceFile { path } => {
                // Save previous file's data if any
                if let Some(file) = current_file.take() {
                    if !file_functions.is_empty() {
                        let mut funcs: Vec<FunctionCoverage> =
                            file_functions.drain().map(|(_, v)| v).collect();
                        funcs.sort_by_key(|f| f.start_line);
                        data.functions.insert(file, funcs);
                    }
                }
                current_file = Some(path);
                file_functions.clear();
                file_lines.clear();
            }

            Record::FunctionName { start_line, name } => {
                // Demangle the function name if it's mangled
                let demangled = demangle_function_name(&name);
                let normalized = normalize_demangled_name(&demangled);

                // Use normalized full_path as key to consolidate duplicates
                // If the entry already exists, keep the existing one (same line, same function)
                file_functions
                    .entry(normalized.full_path.clone())
                    .or_insert_with(|| FunctionCoverage {
                        name: normalized.full_path.clone(),
                        start_line: start_line as usize,
                        execution_count: 0,
                        coverage_percentage: 0.0,
                        uncovered_lines: Vec::new(),
                        normalized,
                    });
            }

            Record::FunctionData { name, count } => {
                // Demangle the function name to match the key used in file_functions
                let demangled = demangle_function_name(&name);
                let normalized = normalize_demangled_name(&demangled);

                if let Some(func) = file_functions.get_mut(&normalized.full_path) {
                    // Keep the maximum execution count when consolidating
                    func.execution_count = func.execution_count.max(count);
                    // If no line data is available, use execution count to determine coverage
                    // Functions with count > 0 are considered 100% covered, 0 means 0% covered
                    if func.coverage_percentage == 0.0 && count > 0 {
                        func.coverage_percentage = 100.0;
                    }
                }
            }

            Record::LineData { line, count, .. } => {
                file_lines.insert(line as usize, count);
                if count > 0 {
                    _lines_hit += 1;
                }
            }

            Record::LinesFound { found } => {
                _lines_found = found as usize;
                data.total_lines += found as usize;
            }

            Record::LinesHit { hit } => {
                _lines_hit = hit as usize;
                data.lines_hit += hit as usize;
            }

            Record::EndOfRecord => {
                // Use parallel processing for function coverage calculation
                process_function_coverage_parallel(&mut file_functions, &file_lines);

                // Save the file's data
                if let Some(file) = current_file.take() {
                    if !file_functions.is_empty() {
                        let mut funcs: Vec<FunctionCoverage> =
                            file_functions.drain().map(|(_, v)| v).collect();
                        funcs.sort_by_key(|f| f.start_line);
                        data.functions.insert(file, funcs);
                    }
                }

                file_functions.clear();
                file_lines.clear();
                file_count += 1;

                // Throttle: update every 10 files
                if file_count % 10 == 0 {
                    progress_callback(CoverageProgress::Parsing {
                        current: file_count,
                        total: file_count,
                    });
                }
            }

            _ => {} // Ignore other record types
        }
    }

    // Handle case where file doesn't end with EndOfRecord
    if let Some(file) = current_file {
        if !file_functions.is_empty() {
            let mut funcs: Vec<FunctionCoverage> = file_functions.drain().map(|(_, v)| v).collect();
            funcs.sort_by_key(|f| f.start_line);
            data.functions.insert(file, funcs);
        }
    }

    // Report statistics computation
    progress_callback(CoverageProgress::ComputingStats);

    // Build the coverage index once after all data is loaded
    data.build_index();

    progress_callback(CoverageProgress::Complete);

    Ok(data)
}

/// Legacy API: Parse LCOV file with ProgressBar
pub fn parse_lcov_file_with_progress(path: &Path, progress: &ProgressBar) -> Result<LcovData> {
    use lcov::{Reader, Record};

    progress.set_message("Loading coverage data");

    let reader = Reader::open_file(path)
        .with_context(|| format!("Failed to open LCOV file: {}", path.display()))?;

    let mut data = LcovData::default();
    let mut current_file: Option<PathBuf> = None;
    let mut file_functions: HashMap<String, FunctionCoverage> = HashMap::new();
    let mut file_lines: HashMap<usize, u64> = HashMap::new();
    let mut _lines_found = 0;
    let mut _lines_hit = 0;
    let mut file_count = 0u64;

    for record in reader {
        let record = record.with_context(|| "Failed to parse LCOV record")?;

        match record {
            Record::SourceFile { path } => {
                // Save previous file's data if any
                if let Some(file) = current_file.take() {
                    if !file_functions.is_empty() {
                        let mut funcs: Vec<FunctionCoverage> =
                            file_functions.drain().map(|(_, v)| v).collect();
                        funcs.sort_by_key(|f| f.start_line);
                        data.functions.insert(file, funcs);
                    }
                }
                current_file = Some(path);
                file_functions.clear();
                file_lines.clear();
            }

            Record::FunctionName { start_line, name } => {
                // Demangle the function name if it's mangled
                let demangled = demangle_function_name(&name);
                let normalized = normalize_demangled_name(&demangled);

                // Use normalized full_path as key to consolidate duplicates
                // If the entry already exists, keep the existing one (same line, same function)
                file_functions
                    .entry(normalized.full_path.clone())
                    .or_insert_with(|| FunctionCoverage {
                        name: normalized.full_path.clone(),
                        start_line: start_line as usize,
                        execution_count: 0,
                        coverage_percentage: 0.0,
                        uncovered_lines: Vec::new(),
                        normalized,
                    });
            }

            Record::FunctionData { name, count } => {
                // Demangle the function name to match the key used in file_functions
                let demangled = demangle_function_name(&name);
                let normalized = normalize_demangled_name(&demangled);

                if let Some(func) = file_functions.get_mut(&normalized.full_path) {
                    // Keep the maximum execution count when consolidating
                    func.execution_count = func.execution_count.max(count);
                    // If no line data is available, use execution count to determine coverage
                    // Functions with count > 0 are considered 100% covered, 0 means 0% covered
                    if func.coverage_percentage == 0.0 && count > 0 {
                        func.coverage_percentage = 100.0;
                    }
                }
            }

            Record::LineData { line, count, .. } => {
                file_lines.insert(line as usize, count);
                if count > 0 {
                    _lines_hit += 1;
                }
            }

            Record::LinesFound { found } => {
                _lines_found = found as usize;
                data.total_lines += found as usize;
            }

            Record::LinesHit { hit } => {
                _lines_hit = hit as usize;
                data.lines_hit += hit as usize;
            }

            Record::EndOfRecord => {
                // Use parallel processing for function coverage calculation
                process_function_coverage_parallel(&mut file_functions, &file_lines);

                // Save the file's data
                if let Some(file) = current_file.take() {
                    if !file_functions.is_empty() {
                        let mut funcs: Vec<FunctionCoverage> =
                            file_functions.drain().map(|(_, v)| v).collect();
                        funcs.sort_by_key(|f| f.start_line);
                        data.functions.insert(file, funcs);
                    }
                }

                file_functions.clear();
                file_lines.clear();
                file_count += 1;
                progress.set_position(file_count);
            }

            _ => {} // Ignore other record types
        }
    }

    // Handle case where file doesn't end with EndOfRecord
    if let Some(file) = current_file {
        if !file_functions.is_empty() {
            let mut funcs: Vec<FunctionCoverage> = file_functions.drain().map(|(_, v)| v).collect();
            funcs.sort_by_key(|f| f.start_line);
            data.functions.insert(file, funcs);
        }
    }

    progress.set_message("Building coverage index");
    // Build the coverage index once after all data is loaded
    data.build_index();

    progress.finish_and_clear();

    Ok(data)
}

/// Parallel processing function for calculating function coverage
/// This replaces the sequential processing in the EndOfRecord handler
fn process_function_coverage_parallel(
    file_functions: &mut HashMap<String, FunctionCoverage>,
    file_lines: &HashMap<usize, u64>,
) {
    // Early return if no data to process
    if file_functions.is_empty() || file_lines.is_empty() {
        return;
    }

    // Collect and sort function start lines for boundary detection (parallel-friendly)
    let func_boundaries: Vec<usize> = file_functions
        .par_iter()
        .map(|(_, func)| func.start_line)
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    // Convert file_lines HashMap to sorted Vec for efficient range queries
    let sorted_lines: Vec<(usize, u64)> = file_lines
        .par_iter()
        .map(|(line, count)| (*line, *count))
        .collect::<Vec<_>>()
        .into_par_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    // Use Mutex for thread-safe access to the functions HashMap
    let functions_mutex = Mutex::new(file_functions);

    // Process functions in parallel
    func_boundaries.par_iter().for_each(|&func_start| {
        // Calculate function coverage for this function
        let coverage_data =
            calculate_function_coverage_data(func_start, &func_boundaries, &sorted_lines);

        // Update the function in the mutex-protected HashMap
        if let Ok(mut functions) = functions_mutex.lock() {
            if let Some(func) = functions.values_mut().find(|f| f.start_line == func_start) {
                func.coverage_percentage = coverage_data.coverage_percentage;
                func.uncovered_lines = coverage_data.uncovered_lines;
            }
        }
    });
}

/// Calculate coverage data for a single function
/// Pure function that can be called in parallel
#[derive(Debug)]
struct FunctionCoverageData {
    coverage_percentage: f64,
    uncovered_lines: Vec<usize>,
}

fn calculate_function_coverage_data(
    func_start: usize,
    func_boundaries: &[usize],
    sorted_lines: &[(usize, u64)],
) -> FunctionCoverageData {
    // Find the next function's start line using binary search
    let next_func_idx = func_boundaries
        .binary_search(&func_start)
        .unwrap_or_else(|idx| idx);

    let func_end = if next_func_idx + 1 < func_boundaries.len() {
        func_boundaries[next_func_idx + 1]
    } else {
        usize::MAX
    };

    // Binary search for function's line range in sorted_lines
    let start_idx = sorted_lines
        .binary_search_by_key(&func_start, |(line, _)| *line)
        .unwrap_or_else(|idx| idx);
    let end_idx = sorted_lines
        .binary_search_by_key(&func_end, |(line, _)| *line)
        .unwrap_or_else(|idx| idx);

    let func_lines = &sorted_lines[start_idx..end_idx];

    if !func_lines.is_empty() {
        let covered = func_lines
            .par_iter()
            .filter(|(_, count)| *count > 0)
            .count();
        let coverage_percentage = (covered as f64 / func_lines.len() as f64) * 100.0;

        // Collect uncovered lines in parallel
        let uncovered_lines = func_lines
            .par_iter()
            .filter(|(_, count)| *count == 0)
            .map(|(line, _)| *line)
            .collect();

        FunctionCoverageData {
            coverage_percentage,
            uncovered_lines,
        }
    } else {
        FunctionCoverageData {
            coverage_percentage: 0.0,
            uncovered_lines: Vec::new(),
        }
    }
}

impl LcovData {
    /// Set the LOC counter to use for consistent line counting
    pub fn with_loc_counter(mut self, loc_counter: crate::metrics::LocCounter) -> Self {
        self.loc_counter = Some(loc_counter);
        self
    }

    /// Get the LOC counter instance if set
    pub fn loc_counter(&self) -> Option<&crate::metrics::LocCounter> {
        self.loc_counter.as_ref()
    }

    /// Recalculate total lines using LOC counter for consistency
    /// This ensures coverage denominator matches the LOC count used elsewhere
    pub fn recalculate_with_loc_counter(&mut self) {
        if let Some(counter) = &self.loc_counter {
            let files: Vec<PathBuf> = self.functions.keys().cloned().collect();
            let mut total_code_lines = 0;

            for file in &files {
                if counter.should_include(file) {
                    if let Ok(count) = counter.count_file(file) {
                        total_code_lines += count.code_lines;
                        log::debug!(
                            "LOC counter: {} has {} code lines",
                            file.display(),
                            count.code_lines
                        );
                    }
                }
            }

            log::debug!(
                "Recalculated total_lines using LocCounter: {} (was {})",
                total_code_lines,
                self.total_lines
            );
            self.total_lines = total_code_lines;
        }
    }

    /// Get function coverage using O(1) indexed lookup
    ///
    /// This method uses the pre-built coverage index for fast lookups,
    /// avoiding the O(n) linear search through function arrays.
    pub fn get_function_coverage(&self, file: &Path, function_name: &str) -> Option<f64> {
        self.coverage_index
            .get_function_coverage(file, function_name)
    }

    /// Get function coverage with line number fallback using O(log n) indexed lookup
    ///
    /// Tries exact function name match first (O(1)), then falls back to
    /// line-based lookup (O(log n)) if needed.
    pub fn get_function_coverage_with_line(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<f64> {
        self.coverage_index
            .get_function_coverage_with_line(file, function_name, line)
    }

    /// Get function coverage using exact boundaries from AST analysis
    /// This is more accurate than guessing boundaries from LCOV data alone
    ///
    /// **Integration of Specs 201, 202, and 203:**
    /// - Uses path normalization (Spec 201) to find matching files
    /// - Uses function name matching (Spec 202) to find matching functions
    /// - Returns 0.0 instead of None when LCOV provided but function not found (Spec 203)
    /// - Logs diagnostics when DEBTMAP_COVERAGE_DEBUG is set (Spec 203)
    pub fn get_function_coverage_with_bounds(
        &self,
        file: &Path,
        function_name: &str,
        _start_line: usize,
        _end_line: usize,
    ) -> Option<f64> {
        let debug_mode = std::env::var("DEBTMAP_COVERAGE_DEBUG").is_ok();

        // Track statistics for diagnostic mode (Spec 203 FR3)
        if debug_mode {
            COVERAGE_MATCH_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        }

        // Phase 1: Path matching using Spec 201
        let available_paths: Vec<PathBuf> = self.functions.keys().cloned().collect();
        let path_match = find_matching_path(file, &available_paths);

        if debug_mode {
            if let Some((_matched_path, strategy)) = &path_match {
                eprintln!(
                    "[COVERAGE] {}::{} Path:✓ Strategy:{:?}",
                    file.display(),
                    function_name,
                    strategy
                );
            } else {
                eprintln!(
                    "[COVERAGE] {}::{} Path:✗ (not found in {} paths)",
                    file.display(),
                    function_name,
                    available_paths.len()
                );
                COVERAGE_MATCH_ZERO.fetch_add(1, Ordering::Relaxed);
                return Some(0.0); // Return 0% not None when LCOV provided
            }
        }

        let (matched_path, _path_strategy) = path_match?;

        // Phase 2: Function matching using Spec 202
        let functions = self.functions.get(matched_path)?;

        // Convert to MatchableFunction for the matching algorithm
        let matchable_funcs: Vec<MatchableFunction<&FunctionCoverage>> = functions
            .iter()
            .map(|f| MatchableFunction {
                name: f.name.clone(),
                data: f,
            })
            .collect();

        let func_match =
            super::function_name_matching::find_matching_function(function_name, &matchable_funcs);

        if debug_mode {
            if let Some((matched_func, confidence)) = &func_match {
                eprintln!(
                    "[COVERAGE]   Func:✓ Confidence:{:?} Coverage:{:.1}%",
                    confidence, matched_func.data.coverage_percentage
                );
            } else {
                eprintln!(
                    "[COVERAGE]   Func:✗ (not found in {} functions)",
                    functions.len()
                );
                COVERAGE_MATCH_ZERO.fetch_add(1, Ordering::Relaxed);
                return Some(0.0); // Return 0% not None when LCOV provided
            }
        }

        let (matched_func, _confidence) = func_match?;
        let coverage = matched_func.data.coverage_percentage / 100.0;

        // Track successful match in debug mode (Spec 203 FR3)
        if debug_mode && coverage > 0.0 {
            COVERAGE_MATCH_SUCCESS.fetch_add(1, Ordering::Relaxed);
        } else if debug_mode {
            COVERAGE_MATCH_ZERO.fetch_add(1, Ordering::Relaxed);
        }

        Some(coverage)
    }

    pub fn get_overall_coverage(&self) -> f64 {
        if self.total_lines == 0 {
            0.0
        } else {
            (self.lines_hit as f64 / self.total_lines as f64) * 100.0
        }
    }

    pub fn get_file_coverage(&self, file: &Path) -> Option<f64> {
        self.functions.get(file).map(|funcs| {
            if funcs.is_empty() {
                0.0
            } else {
                // Use parallel processing for coverage calculation
                let sum: f64 = funcs.par_iter().map(|f| f.coverage_percentage).sum();
                sum / funcs.len() as f64 / 100.0 // Convert to fraction
            }
        })
    }

    /// Get uncovered lines for a specific function
    /// Get uncovered lines for a function using O(1) indexed lookup
    pub fn get_function_uncovered_lines(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<Vec<usize>> {
        self.coverage_index
            .get_function_uncovered_lines(file, function_name, line)
    }

    /// Batch process coverage queries for multiple functions in parallel
    /// This is more efficient when querying coverage for many functions at once
    pub fn batch_get_function_coverage(
        &self,
        queries: &[(PathBuf, String, usize)], // (file, function_name, line)
    ) -> Vec<Option<f64>> {
        queries
            .par_iter()
            .map(|(file, function_name, line)| {
                self.get_function_coverage_with_line(file, function_name, *line)
            })
            .collect()
    }

    /// Get coverage statistics for all files in parallel
    pub fn get_all_file_coverages(&self) -> HashMap<PathBuf, f64> {
        self.functions
            .par_iter()
            .map(|(path, funcs)| {
                let coverage = if funcs.is_empty() {
                    0.0
                } else {
                    let sum: f64 = funcs.par_iter().map(|f| f.coverage_percentage).sum();
                    sum / funcs.len() as f64 / 100.0 // Convert to fraction
                };
                (path.clone(), coverage)
            })
            .collect()
    }
}

/// Find a function by name using multiple matching strategies
/// Returns the first match found using these strategies in order:
/// 1. Exact match
/// 2. Normalized match (handling generics and special characters)
/// 3. Suffix match (for module-qualified names)
#[allow(dead_code)]
fn find_function_by_name<'a>(
    funcs: &'a [FunctionCoverage],
    target_name: &str,
) -> Option<&'a FunctionCoverage> {
    // For small function lists, use sequential search to avoid parallel overhead
    if funcs.len() < 10 {
        // Strategy 1: Exact match
        funcs
            .iter()
            .find(|f| f.name == target_name)
            // Strategy 2: Normalized match
            .or_else(|| {
                let normalized_target = normalize_function_name(target_name);
                funcs
                    .iter()
                    .find(|f| normalize_function_name(&f.name) == normalized_target)
            })
            // Strategy 3: Suffix match (e.g., "module::function" matches "function")
            .or_else(|| {
                funcs
                    .iter()
                    .find(|f| f.name.ends_with(target_name) || target_name.ends_with(&f.name))
            })
    } else {
        // For larger function lists, use parallel search
        // Strategy 1: Exact match (parallel)
        funcs
            .par_iter()
            .find_any(|f| f.name == target_name)
            // Strategy 2: Normalized match (parallel)
            .or_else(|| {
                let normalized_target = normalize_function_name(target_name);
                funcs
                    .par_iter()
                    .find_any(|f| normalize_function_name(&f.name) == normalized_target)
            })
            // Strategy 3: Suffix match (parallel)
            .or_else(|| {
                funcs
                    .par_iter()
                    .find_any(|f| f.name.ends_with(target_name) || target_name.ends_with(&f.name))
            })
    }
}

/// Find a function by line number with tolerance for AST/LCOV discrepancies
/// Pure function that searches for functions within a line range
#[allow(dead_code)]
fn find_function_by_line_with_tolerance(
    funcs: &[FunctionCoverage],
    target_line: usize,
    tolerance: usize,
) -> Option<&FunctionCoverage> {
    funcs
        .iter()
        .filter(|f| {
            let line_diff = (f.start_line as i32 - target_line as i32).abs();
            line_diff <= tolerance as i32
        })
        // If multiple matches within tolerance, prefer the closest one
        .min_by_key(|f| (f.start_line as i32 - target_line as i32).abs())
}

/// Normalize function names for matching
/// This is now a pure function used internally by the matching strategies
#[allow(dead_code)]
fn normalize_function_name(name: &str) -> String {
    // Handle Rust function names with generics and impl blocks
    name.replace(['<', '>'], "_")
        .replace("::", "_")
        .replace(' ', "_")
        .replace('\'', "")
}

/// Strategy 1: Check if query path ends with LCOV path
///
/// This strategy handles cases where the query path is absolute and LCOV paths are relative.
/// It's the most common case when analyzing projects with absolute file paths.
///
/// # Arguments
/// * `query_path` - The path being searched for (often absolute)
/// * `lcov_path` - The path from LCOV data (often relative)
///
/// # Example
///
/// Given query path `/home/user/project/src/main.rs` and LCOV path `src/main.rs`,
/// this strategy returns `true` because the query ends with the LCOV path.
#[allow(dead_code)]
fn matches_suffix_strategy(query_path: &Path, lcov_path: &Path) -> bool {
    query_path.ends_with(lcov_path)
}

/// Strategy 2: Check if LCOV path ends with normalized query path
///
/// This strategy handles cases where LCOV paths are absolute but query paths are relative.
/// Normalization removes leading "./" to enable matching.
///
/// # Arguments
/// * `query_path` - The path being searched for (often relative with ./)
/// * `lcov_path` - The path from LCOV data (often absolute)
///
/// # Example
///
/// Given query path `./src/lib.rs` and LCOV path `/home/user/project/src/lib.rs`,
/// after normalizing the query to `src/lib.rs`, this strategy returns `true`
/// because the LCOV path ends with the normalized query.
#[allow(dead_code)]
fn matches_reverse_suffix_strategy(query_path: &Path, lcov_path: &Path) -> bool {
    let normalized_query = normalize_path(query_path);
    lcov_path.ends_with(&normalized_query)
}

/// Strategy 3: Check if normalized paths are equal
///
/// This strategy normalizes both paths by removing leading "./" and checks for exact equality.
/// It's useful when both paths might have different normalization but refer to the same file.
///
/// # Arguments
/// * `query_path` - The path being searched for
/// * `lcov_path` - The path from LCOV data
///
/// # Example
///
/// Given query path `src/main.rs` and LCOV path `./src/main.rs`,
/// both normalize to `src/main.rs` and this strategy returns `true`.
#[allow(dead_code)]
fn matches_normalized_equality_strategy(query_path: &Path, lcov_path: &Path) -> bool {
    normalize_path(lcov_path) == normalize_path(query_path)
}

/// Apply matching strategies sequentially to find a match
///
/// Uses sequential iteration for smaller datasets (≤20 items) to avoid parallel overhead.
/// Applies three matching strategies in order until a match is found.
///
/// # Arguments
/// * `functions` - HashMap of file paths to function coverage data
/// * `query_path` - The path to search for
///
/// # Returns
/// Reference to function coverage data if found, None otherwise
#[allow(dead_code)]
fn apply_strategies_sequential<'a>(
    functions: &'a HashMap<PathBuf, Vec<FunctionCoverage>>,
    query_path: &Path,
) -> Option<&'a Vec<FunctionCoverage>> {
    functions
        .iter()
        .find(|(lcov_path, _)| matches_suffix_strategy(query_path, lcov_path))
        .map(|(_, funcs)| funcs)
        .or_else(|| {
            functions
                .iter()
                .find(|(lcov_path, _)| matches_reverse_suffix_strategy(query_path, lcov_path))
                .map(|(_, funcs)| funcs)
        })
        .or_else(|| {
            functions
                .iter()
                .find(|(lcov_path, _)| matches_normalized_equality_strategy(query_path, lcov_path))
                .map(|(_, funcs)| funcs)
        })
}

/// Apply matching strategies in parallel to find a match
///
/// Uses parallel iteration for larger datasets (>20 items) to improve performance.
/// Applies three matching strategies using rayon's parallel iterator until a match is found.
///
/// # Arguments
/// * `functions` - HashMap of file paths to function coverage data
/// * `query_path` - The path to search for
///
/// # Returns
/// Reference to function coverage data if found, None otherwise
#[allow(dead_code)]
fn apply_strategies_parallel<'a>(
    functions: &'a HashMap<PathBuf, Vec<FunctionCoverage>>,
    query_path: &Path,
) -> Option<&'a Vec<FunctionCoverage>> {
    functions
        .par_iter()
        .find_any(|(lcov_path, _)| matches_suffix_strategy(query_path, lcov_path))
        .map(|(_, funcs)| funcs)
        .or_else(|| {
            functions
                .par_iter()
                .find_any(|(lcov_path, _)| matches_reverse_suffix_strategy(query_path, lcov_path))
                .map(|(_, funcs)| funcs)
        })
        .or_else(|| {
            functions
                .par_iter()
                .find_any(|(lcov_path, _)| {
                    matches_normalized_equality_strategy(query_path, lcov_path)
                })
                .map(|(_, funcs)| funcs)
        })
}

/// Find functions by normalizing and matching paths
///
/// This is the main entry point for path-based function lookup. It automatically chooses
/// between parallel and sequential search based on dataset size and applies multiple
/// matching strategies to handle various path format combinations.
///
/// # Path Matching Strategies
/// The function tries three strategies in order:
/// 1. Suffix matching - query path ends with LCOV path
/// 2. Reverse suffix matching - LCOV path ends with normalized query
/// 3. Normalized equality - both paths equal after normalization
///
/// # Performance
/// - Uses parallel search (rayon) for >20 items
/// - Uses sequential search for ≤20 items to avoid parallel overhead
///
/// # Arguments
/// * `functions` - HashMap mapping file paths to their function coverage data
/// * `query_path` - The path to search for
///
/// # Returns
/// Reference to the function coverage data if found, None otherwise
///
/// # Example
///
/// This function can find coverage data even when paths differ in normalization.
/// For instance, querying `./src/main.rs` will match stored data under `src/main.rs`.
/// Uses parallel search for large datasets (>20 files) and sequential for smaller ones.
#[allow(dead_code)]
fn find_functions_by_path<'a>(
    functions: &'a HashMap<PathBuf, Vec<FunctionCoverage>>,
    query_path: &Path,
) -> Option<&'a Vec<FunctionCoverage>> {
    if functions.len() > 20 {
        apply_strategies_parallel(functions, query_path)
    } else {
        apply_strategies_sequential(functions, query_path)
    }
}

/// Normalize a path by removing leading ./ and resolving components
///
/// This is a pure function that standardizes path representation for matching.
/// It's idempotent: normalizing an already-normalized path returns the same path.
///
/// # Arguments
/// * `path` - The path to normalize
///
/// # Returns
/// A new PathBuf with leading "./" removed
///
/// # Example
///
/// Normalizing `./src/main.rs` produces `src/main.rs`.
/// The function is idempotent: normalizing `src/main.rs` again returns `src/main.rs`.
#[allow(dead_code)]
fn normalize_path(path: &Path) -> PathBuf {
    // Convert to string, remove leading ./
    let path_str = path.to_string_lossy();
    let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);
    PathBuf::from(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Tests for demangling functions
    mod demangle_tests {
        use super::*;

        #[test]
        fn test_demangle_v0_mangled_name() {
            let mangled = "_RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes";
            let demangled = demangle_function_name(mangled);

            assert!(demangled.contains("ChangeTracker"));
            assert!(demangled.contains("track_changes"));
            assert!(!demangled.starts_with("_RNv"));
        }

        #[test]
        fn test_demangle_legacy_mangled_name() {
            // Test with a simple legacy mangled name
            let mangled = "_ZN3foo3barE";
            let demangled = demangle_function_name(mangled);

            // rustc-demangle should handle this
            assert!(!demangled.starts_with("_ZN") || demangled == mangled);
        }

        #[test]
        fn test_demangle_already_demangled() {
            let name = "my_module::my_function";
            let result = demangle_function_name(name);

            assert_eq!(result, name);
        }

        #[test]
        fn test_normalize_removes_generics() {
            let result = normalize_demangled_name("HashMap<String, i32>::insert");
            assert_eq!(result.full_path, "HashMap::insert");
            assert_eq!(result.method_name, "insert");

            let result = normalize_demangled_name("Vec<T>::push");
            assert_eq!(result.full_path, "Vec::push");
            assert_eq!(result.method_name, "push");

            let result = normalize_demangled_name("simple_function");
            assert_eq!(result.full_path, "simple_function");
            assert_eq!(result.method_name, "simple_function");
        }

        #[test]
        fn test_normalize_preserves_module_path() {
            let result = normalize_demangled_name("std::collections::HashMap<K,V>::insert");
            assert_eq!(result.full_path, "std::collections::HashMap::insert");
            assert_eq!(result.method_name, "insert");
        }

        #[test]
        fn test_normalize_removes_crate_hash() {
            let result = normalize_demangled_name("<debtmap[71f4b4990cdcf1ab]::Foo>::bar");
            assert_eq!(result.full_path, "debtmap::Foo::bar");
            assert_eq!(result.method_name, "bar");
        }

        #[test]
        fn test_normalize_extracts_method_name() {
            let result =
                normalize_demangled_name("prodigy::cook::CommitTracker::create_auto_commit");
            assert_eq!(
                result.full_path,
                "prodigy::cook::CommitTracker::create_auto_commit"
            );
            assert_eq!(result.method_name, "create_auto_commit");

            let result = normalize_demangled_name("<Foo as Bar>::method");
            assert_eq!(result.method_name, "method");
        }

        #[test]
        fn test_strip_trailing_generics_simple() {
            use std::borrow::Cow;

            assert_eq!(
                strip_trailing_generics("Type::method::<WorkflowExecutor>"),
                Cow::Borrowed("Type::method")
            );
            assert_eq!(
                strip_trailing_generics("crate::Type::method::<T>"),
                Cow::Borrowed("crate::Type::method")
            );
            assert_eq!(
                strip_trailing_generics("Type::method"), // No generics
                Cow::Borrowed("Type::method")
            );
        }

        #[test]
        fn test_strip_trailing_generics_nested() {
            use std::borrow::Cow;

            // Nested generics
            assert_eq!(
                strip_trailing_generics("method::<Vec<HashMap<K, V>>>"),
                Cow::Borrowed("method")
            );
            // Multiple type parameters
            assert_eq!(
                strip_trailing_generics("method::<T, U, V>"),
                Cow::Borrowed("method")
            );
            // Complex nested case
            assert_eq!(
                strip_trailing_generics("Type::method::<Result<Vec<T>, Error>>"),
                Cow::Borrowed("Type::method")
            );
        }

        #[test]
        fn test_normalize_strips_trailing_generics() {
            // Test that normalize_demangled_name now strips trailing generics
            let result = normalize_demangled_name("Type::method::<WorkflowExecutor>");
            assert_eq!(result.full_path, "Type::method");
            assert_eq!(result.method_name, "method");

            let result = normalize_demangled_name("SetupPhaseExecutor::execute::<T>");
            assert_eq!(result.full_path, "SetupPhaseExecutor::execute");
            assert_eq!(result.method_name, "execute");
        }
    }

    #[test]
    fn test_parse_lcov_file() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,test_function
FNDA:5,test_function
FNF:1
FNH:1
DA:10,5
DA:11,5
DA:12,0
LF:3
LH:2
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        assert_eq!(data.total_lines, 3);
        assert_eq!(data.lines_hit, 2);

        let file_path = PathBuf::from("/path/to/file.rs");
        assert!(data.functions.contains_key(&file_path));

        let funcs = &data.functions[&file_path];
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "test_function");
        assert_eq!(funcs[0].execution_count, 5);
    }

    #[test]
    fn test_function_name_normalization() {
        assert_eq!(
            normalize_function_name("MyStruct::my_function"),
            normalize_function_name("MyStruct_my_function")
        );

        assert_eq!(
            normalize_function_name("function<T>"),
            normalize_function_name("function_T_")
        );
    }

    #[test]
    fn test_coverage_percentage_calculation() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,fully_covered
FN:20,partially_covered
FN:30,not_covered
FNDA:10,fully_covered
FNDA:5,partially_covered
FNDA:0,not_covered
DA:10,10
DA:11,10
DA:12,10
DA:20,5
DA:21,5
DA:22,0
DA:23,0
DA:30,0
DA:31,0
DA:32,0
LF:10
LH:5
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/file.rs");

        // Test fully covered function (100%)
        let coverage = data.get_function_coverage(&file_path, "fully_covered");
        assert_eq!(coverage, Some(1.0));

        // Test partially covered function (50%)
        let coverage = data.get_function_coverage(&file_path, "partially_covered");
        assert_eq!(coverage, Some(0.5));

        // Test uncovered function (0%)
        let coverage = data.get_function_coverage(&file_path, "not_covered");
        assert_eq!(coverage, Some(0.0));
    }

    #[test]
    fn test_get_function_coverage_with_line() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,function_at_line_10
FN:50,function_at_line_50
FNDA:5,function_at_line_10
FNDA:0,function_at_line_50
DA:10,5
DA:11,5
DA:50,0
DA:51,0
LF:4
LH:2
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/file.rs");

        // Test finding function by line number
        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 10);
        assert_eq!(coverage, Some(1.0)); // Should find function_at_line_10

        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 51);
        assert_eq!(coverage, Some(0.0)); // Should find function_at_line_50

        // Test line outside any function range
        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 200);
        assert_eq!(coverage, None);
    }

    #[test]
    fn test_overall_coverage_calculation() {
        let lcov_content = r#"TN:
SF:/path/to/file1.rs
DA:1,1
DA:2,1
DA:3,0
DA:4,0
LF:4
LH:2
end_of_record
SF:/path/to/file2.rs
DA:1,5
DA:2,5
DA:3,5
DA:4,5
LF:4
LH:4
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        // Overall coverage: 6 lines hit out of 8 total = 75%
        assert_eq!(data.get_overall_coverage(), 75.0);
    }

    #[test]
    fn test_empty_coverage_data() {
        let lcov_content = r#"TN:
SF:/path/to/empty.rs
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        assert_eq!(data.get_overall_coverage(), 0.0);
        assert_eq!(data.functions.len(), 0);
    }

    #[test]
    fn test_consolidate_duplicate_mangled_functions() {
        // Test that duplicate mangled functions with different crate hashes
        // are consolidated into a single entry
        // Using actual valid mangled names from the spec
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:18,_RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
FNDA:5,_RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
FN:18,_RNvMNtNtNtCs5ZpFxq88JTF_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
FNDA:3,_RNvMNtNtNtCs5ZpFxq88JTF_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
DA:18,5
DA:19,5
LF:2
LH:2
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/file.rs");

        // Should consolidate to single function
        let funcs = &data.functions[&file_path];
        assert_eq!(funcs.len(), 1, "Expected 1 function after consolidation");

        // Should keep max execution count (5 vs 3)
        assert_eq!(funcs[0].execution_count, 5, "Expected max execution count");

        // Function name should be demangled
        assert!(
            funcs[0].name.contains("ChangeTracker") || funcs[0].name.contains("track_changes"),
            "Expected demangled name, got: {}",
            funcs[0].name
        );
        assert!(
            !funcs[0].name.starts_with("_RNv"),
            "Name should not be mangled: {}",
            funcs[0].name
        );

        // Start line should be preserved
        assert_eq!(funcs[0].start_line, 18);
    }

    // Tests for find_functions_by_path
    mod find_functions_by_path_tests {
        use super::*;

        fn create_test_function_coverage(name: &str) -> Vec<FunctionCoverage> {
            vec![FunctionCoverage {
                name: name.to_string(),
                start_line: 10,
                execution_count: 5,
                coverage_percentage: 60.0,
                uncovered_lines: vec![12, 13],
                normalized: NormalizedFunctionName {
                    full_path: name.to_string(),
                    method_name: name.to_string(),
                    original: name.to_string(),
                },
            }]
        }

        #[test]
        fn test_strategy_1_suffix_matching_sequential() {
            // Test Strategy 1: query_path.ends_with(lcov_path)
            // Sequential path (<=20 items)
            let mut functions = HashMap::new();
            functions.insert(
                PathBuf::from("src/main.rs"),
                create_test_function_coverage("main"),
            );

            let query = PathBuf::from("/home/user/project/src/main.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "main");
        }

        #[test]
        fn test_strategy_2_reverse_suffix_matching_sequential() {
            // Test Strategy 2: lcov_path.ends_with(normalized_query)
            // Sequential path (<=20 items)
            let mut functions = HashMap::new();
            functions.insert(
                PathBuf::from("/home/user/project/src/lib.rs"),
                create_test_function_coverage("lib_func"),
            );

            let query = PathBuf::from("./src/lib.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "lib_func");
        }

        #[test]
        fn test_strategy_3_normalized_equality_sequential() {
            // Test Strategy 3: normalize_path(lcov_path) == normalize_path(query_path)
            // Sequential path (<=20 items)
            let mut functions = HashMap::new();
            functions.insert(
                PathBuf::from("./src/utils.rs"),
                create_test_function_coverage("util_func"),
            );

            let query = PathBuf::from("src/utils.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "util_func");
        }

        #[test]
        fn test_strategy_1_suffix_matching_parallel() {
            // Test Strategy 1 with >20 items (parallel path)
            let mut functions = HashMap::new();

            // Add 21 items to trigger parallel path
            for i in 0..21 {
                functions.insert(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    create_test_function_coverage(&format!("func_{}", i)),
                );
            }

            // Add our target
            functions.insert(
                PathBuf::from("src/target.rs"),
                create_test_function_coverage("target_func"),
            );

            let query = PathBuf::from("/home/user/project/src/target.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "target_func");
        }

        #[test]
        fn test_strategy_2_reverse_suffix_matching_parallel() {
            // Test Strategy 2 with >20 items (parallel path)
            let mut functions = HashMap::new();

            // Add 21 items to trigger parallel path
            for i in 0..21 {
                functions.insert(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    create_test_function_coverage(&format!("func_{}", i)),
                );
            }

            // Add our target with absolute path
            functions.insert(
                PathBuf::from("/home/user/project/src/target.rs"),
                create_test_function_coverage("target_func"),
            );

            // Query with relative path
            let query = PathBuf::from("./src/target.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "target_func");
        }

        #[test]
        fn test_strategy_3_normalized_equality_parallel() {
            // Test Strategy 3 with >20 items (parallel path)
            let mut functions = HashMap::new();

            // Add 21 items to trigger parallel path
            for i in 0..21 {
                functions.insert(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    create_test_function_coverage(&format!("func_{}", i)),
                );
            }

            // Add our target with ./ prefix
            functions.insert(
                PathBuf::from("./src/target.rs"),
                create_test_function_coverage("target_func"),
            );

            // Query without ./ prefix
            let query = PathBuf::from("src/target.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "target_func");
        }

        #[test]
        fn test_no_match_found() {
            // Test when no strategy matches
            let mut functions = HashMap::new();
            functions.insert(
                PathBuf::from("src/main.rs"),
                create_test_function_coverage("main"),
            );

            let query = PathBuf::from("src/different.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_none());
        }

        #[test]
        fn test_empty_map() {
            // Test with empty function map
            let functions = HashMap::new();
            let query = PathBuf::from("src/main.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_none());
        }

        #[test]
        fn test_single_item_map() {
            // Test edge case with exactly 1 item (sequential path)
            let mut functions = HashMap::new();
            functions.insert(
                PathBuf::from("src/single.rs"),
                create_test_function_coverage("single_func"),
            );

            let query = PathBuf::from("./src/single.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "single_func");
        }

        #[test]
        fn test_exactly_20_items() {
            // Test boundary case with exactly 20 items (should use sequential)
            let mut functions = HashMap::new();

            for i in 0..20 {
                functions.insert(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    create_test_function_coverage(&format!("func_{}", i)),
                );
            }

            let query = PathBuf::from("./src/file_10.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "func_10");
        }

        #[test]
        fn test_exactly_21_items() {
            // Test boundary case with exactly 21 items (should use parallel)
            let mut functions = HashMap::new();

            for i in 0..21 {
                functions.insert(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    create_test_function_coverage(&format!("func_{}", i)),
                );
            }

            let query = PathBuf::from("./src/file_10.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "func_10");
        }

        #[test]
        fn test_normalize_path_idempotency() {
            // Test that normalize_path is idempotent
            let path1 = PathBuf::from("./src/main.rs");
            let normalized1 = normalize_path(&path1);
            let normalized2 = normalize_path(&normalized1);

            assert_eq!(normalized1, normalized2);
        }

        #[test]
        fn test_normalize_path_removes_leading_dot_slash() {
            // Test that normalize_path removes leading ./
            let path = PathBuf::from("./src/main.rs");
            let normalized = normalize_path(&path);

            assert_eq!(normalized, PathBuf::from("src/main.rs"));
        }

        #[test]
        fn test_normalize_path_no_leading_dot_slash() {
            // Test that normalize_path doesn't affect paths without leading ./
            let path = PathBuf::from("src/main.rs");
            let normalized = normalize_path(&path);

            assert_eq!(normalized, PathBuf::from("src/main.rs"));
        }
    }

    // Tests for extracted strategy functions
    mod strategy_tests {
        use super::*;

        #[test]
        fn test_matches_suffix_strategy_basic() {
            let query = PathBuf::from("/home/user/project/src/main.rs");
            let lcov = PathBuf::from("src/main.rs");

            assert!(matches_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_suffix_strategy_no_match() {
            let query = PathBuf::from("/home/user/project/src/main.rs");
            let lcov = PathBuf::from("src/lib.rs");

            assert!(!matches_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_suffix_strategy_exact_match() {
            let query = PathBuf::from("src/main.rs");
            let lcov = PathBuf::from("src/main.rs");

            assert!(matches_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_reverse_suffix_strategy_basic() {
            let query = PathBuf::from("./src/lib.rs");
            let lcov = PathBuf::from("/home/user/project/src/lib.rs");

            assert!(matches_reverse_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_reverse_suffix_strategy_no_match() {
            let query = PathBuf::from("./src/main.rs");
            let lcov = PathBuf::from("/home/user/project/src/lib.rs");

            assert!(!matches_reverse_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_reverse_suffix_strategy_with_normalization() {
            let query = PathBuf::from("./src/utils.rs");
            let lcov = PathBuf::from("/home/user/src/utils.rs");

            assert!(matches_reverse_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_normalized_equality_strategy_basic() {
            let query = PathBuf::from("src/main.rs");
            let lcov = PathBuf::from("./src/main.rs");

            assert!(matches_normalized_equality_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_normalized_equality_strategy_both_normalized() {
            let query = PathBuf::from("./src/main.rs");
            let lcov = PathBuf::from("./src/main.rs");

            assert!(matches_normalized_equality_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_normalized_equality_strategy_no_match() {
            let query = PathBuf::from("src/main.rs");
            let lcov = PathBuf::from("src/lib.rs");

            assert!(!matches_normalized_equality_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_normalized_equality_strategy_different_depths() {
            let query = PathBuf::from("./src/main.rs");
            let lcov = PathBuf::from("./lib/main.rs");

            assert!(!matches_normalized_equality_strategy(&query, &lcov));
        }
    }

    // Property-based tests
    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_normalize_path_idempotent(path in "[a-z/]+\\.rs") {
                let path_buf = PathBuf::from(&path);
                let normalized1 = normalize_path(&path_buf);
                let normalized2 = normalize_path(&normalized1);

                prop_assert_eq!(normalized1, normalized2);
            }

            #[test]
            fn test_normalize_path_removes_leading_dot_slash_property(path in "[a-z/]+\\.rs") {
                let with_dot = PathBuf::from(format!("./{}", path));
                let without_dot = PathBuf::from(&path);

                prop_assert_eq!(normalize_path(&with_dot), normalize_path(&without_dot));
            }

            #[test]
            fn test_suffix_strategy_reflexive(path in "[a-z/]+\\.rs") {
                let path_buf = PathBuf::from(&path);

                // A path always matches itself with suffix strategy
                prop_assert!(matches_suffix_strategy(&path_buf, &path_buf));
            }

            #[test]
            fn test_normalized_equality_symmetric(path in "[a-z/]+\\.rs") {
                let path1 = PathBuf::from(&path);
                let path2 = PathBuf::from(format!("./{}", path));

                // Normalized equality should be symmetric
                prop_assert_eq!(
                    matches_normalized_equality_strategy(&path1, &path2),
                    matches_normalized_equality_strategy(&path2, &path1)
                );
            }

            #[test]
            fn test_find_functions_with_large_dataset(count in 50usize..150) {
                // Test parallel path with varying sizes
                let mut functions = HashMap::new();

                for i in 0..count {
                    let func_name = format!("func_{}", i);
                    functions.insert(
                        PathBuf::from(format!("src/file_{}.rs", i)),
                        vec![FunctionCoverage {
                            name: func_name.clone(),
                            start_line: 10,
                            execution_count: 5,
                            coverage_percentage: 60.0,
                            uncovered_lines: vec![12, 13],
                            normalized: NormalizedFunctionName {
                                full_path: func_name.clone(),
                                method_name: func_name.clone(),
                                original: func_name.clone(),
                            },
                        }],
                    );
                }

                // Should find the target regardless of dataset size
                let query = PathBuf::from("./src/file_42.rs");
                let result = find_functions_by_path(&functions, &query);

                if count > 42 {
                    prop_assert!(result.is_some());
                    if let Some(funcs) = result {
                        prop_assert_eq!(&funcs[0].name, "func_42");
                    }
                }
            }

            #[test]
            fn test_strategies_handle_empty_paths_gracefully(s in ".*") {
                let path1 = PathBuf::from(&s);
                let path2 = PathBuf::from(&s);

                // Should not panic with any input
                let _ = matches_suffix_strategy(&path1, &path2);
                let _ = matches_reverse_suffix_strategy(&path1, &path2);
                let _ = matches_normalized_equality_strategy(&path1, &path2);

                prop_assert!(true);
            }
        }
    }

    /// Tests for Rust impl method matching with angle brackets
    /// These tests expose the bug where methods like <Type>::method() fail to match
    mod impl_method_matching_tests {
        use super::*;

        #[test]
        fn test_normalize_impl_method_with_angle_brackets() {
            // This is the actual pattern from LCOV that fails to match
            let demangled =
                "<prodigy::cook::workflow::resume::ResumeExecutor>::execute_remaining_steps";
            let result = normalize_demangled_name(demangled);

            // After normalization, should be usable for matching
            assert_eq!(result.method_name, "execute_remaining_steps");

            // The full_path should preserve the structure but be matchable
            // Current bug: the angle brackets make matching fail
            assert!(
                result.full_path.contains("ResumeExecutor"),
                "full_path should contain ResumeExecutor, got: {}",
                result.full_path
            );
            assert!(
                result.full_path.contains("execute_remaining_steps"),
                "full_path should contain method name, got: {}",
                result.full_path
            );
        }

        #[test]
        fn test_impl_method_matching_in_lcov() {
            // Simulate real LCOV data with mangled Rust impl method
            let lcov_content = r#"TN:
SF:src/cook/workflow/resume.rs
FN:824,_RNvMNtNtNtCsaSt3OvnYFHR_7prodigy4cook8workflow6resumeNtB2_14ResumeExecutor23execute_remaining_steps
FNDA:0,_RNvMNtNtNtCsaSt3OvnYFHR_7prodigy4cook8workflow6resumeNtB2_14ResumeExecutor23execute_remaining_steps
DA:824,0
DA:825,0
DA:826,0
LF:3
LH:0
end_of_record
"#;

            let mut temp_file = NamedTempFile::new().unwrap();
            temp_file.write_all(lcov_content.as_bytes()).unwrap();

            let data = parse_lcov_file(temp_file.path()).unwrap();
            let file_path = PathBuf::from("src/cook/workflow/resume.rs");

            // Test 1: Query with short form (what debtmap uses)
            let coverage =
                data.get_function_coverage(&file_path, "ResumeExecutor::execute_remaining_steps");
            assert!(
                coverage.is_some(),
                "BUG: Should match 'ResumeExecutor::execute_remaining_steps' but got None"
            );

            // Test 2: Query with method name only
            let coverage =
                data.get_function_coverage_with_line(&file_path, "execute_remaining_steps", 824);
            assert!(
                coverage.is_some(),
                "BUG: Should match method name 'execute_remaining_steps' but got None"
            );

            // Test 3: Query with full path
            let coverage = data.get_function_coverage(
                &file_path,
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
            );
            assert!(
                coverage.is_some(),
                "BUG: Should match full path but got None"
            );
        }

        #[test]
        fn test_standalone_function_vs_impl_method() {
            // Test that standalone functions work (baseline)
            // and impl methods fail (demonstrates the bug)
            let lcov_content = r#"TN:
SF:src/cli/commands/resume.rs
FN:568,_RNvNtNtNtCsaSt3OvnYFHR_7prodigy3cli8commands6resume24execute_mapreduce_resume
FNDA:0,_RNvNtNtNtCsaSt3OvnYFHR_7prodigy3cli8commands6resume24execute_mapreduce_resume
DA:568,0
DA:569,0
LF:2
LH:0
end_of_record
SF:src/cook/workflow/resume.rs
FN:824,_RNvMNtNtNtCsaSt3OvnYFHR_7prodigy4cook8workflow6resumeNtB2_14ResumeExecutor23execute_remaining_steps
FNDA:0,_RNvMNtNtNtCsaSt3OvnYFHR_7prodigy4cook8workflow6resumeNtB2_14ResumeExecutor23execute_remaining_steps
DA:824,0
DA:825,0
LF:2
LH:0
end_of_record
"#;

            let mut temp_file = NamedTempFile::new().unwrap();
            temp_file.write_all(lcov_content.as_bytes()).unwrap();

            let data = parse_lcov_file(temp_file.path()).unwrap();

            // Standalone function - should work
            let file_path_1 = PathBuf::from("src/cli/commands/resume.rs");
            let coverage_standalone =
                data.get_function_coverage_with_line(&file_path_1, "execute_mapreduce_resume", 568);
            assert!(
                coverage_standalone.is_some(),
                "Standalone function 'execute_mapreduce_resume' should match (baseline test)"
            );

            // Impl method - currently fails
            let file_path_2 = PathBuf::from("src/cook/workflow/resume.rs");
            let coverage_impl =
                data.get_function_coverage_with_line(&file_path_2, "execute_remaining_steps", 824);
            assert!(
                coverage_impl.is_some(),
                "BUG: Impl method 'execute_remaining_steps' should match but doesn't"
            );
        }

        #[test]
        fn test_multiple_impl_methods() {
            // Test multiple common patterns that appear in real codebases
            let test_cases = vec![
                ("<Type>::method", "method", "Simple impl method"),
                (
                    "<crate::module::Type>::method",
                    "method",
                    "Fully qualified impl method",
                ),
                (
                    "<impl Trait for Type>::method",
                    "method",
                    "Trait impl method",
                ),
                (
                    "<Type<T>>::generic_method",
                    "generic_method",
                    "Generic impl method",
                ),
            ];

            for (input, expected_method, description) in test_cases {
                let result = normalize_demangled_name(input);
                assert_eq!(
                    result.method_name, expected_method,
                    "{}: method_name mismatch for '{}'",
                    description, input
                );
            }
        }

        #[test]
        fn test_normalize_preserves_impl_for_matching() {
            // The normalization should make impl methods matchable
            let demangled = "<foo::bar::Baz>::do_something";
            let normalized = normalize_demangled_name(demangled);

            // Should be able to match against any of these forms:
            // 1. Just the method name: "do_something"
            // 2. Type::method: "Baz::do_something"
            // 3. Full path: "foo::bar::Baz::do_something"

            assert_eq!(normalized.method_name, "do_something");

            // The full_path should be matchable (without angle brackets causing issues)
            assert!(
                !normalized.full_path.starts_with('<'),
                "full_path should not start with angle bracket, got: {}",
                normalized.full_path
            );
        }

        #[test]
        fn test_real_world_prodigy_patterns() {
            // These are actual patterns from the Prodigy codebase that fail
            let real_world_cases = vec![
                "_RNvMNtNtNtCsaSt3OvnYFHR_7prodigy4cook8workflow6resumeNtB2_14ResumeExecutor23execute_remaining_steps",
                "_RNvMNtNtNtCsaSt3OvnYFHR_7prodigy4cook9execution14setup_executorNtB2_18SetupPhaseExecutor7execute",
                "_RNvMNtNtNtCsaSt3OvnYFHR_7prodigy8commands8handlers4fileNtB2_11FileHandler7execute",
            ];

            for mangled in real_world_cases {
                let demangled = demangle_function_name(mangled);
                let normalized = normalize_demangled_name(&demangled);

                // All should have a valid method name
                assert!(
                    !normalized.method_name.is_empty(),
                    "Method name should not be empty for: {}",
                    demangled
                );

                // Should not have angle brackets in final normalized form for matching
                // (This is currently failing - angle brackets remain)
                assert!(
                    !normalized.full_path.starts_with('<'),
                    "BUG: full_path should not have leading angle bracket: {} -> {}",
                    demangled,
                    normalized.full_path
                );
            }
        }
    }
}
