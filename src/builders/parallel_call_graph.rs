use crate::{
    analysis::call_graph::RustCallGraphBuilder,
    analyzers::rust_call_graph::extract_call_graph_multi_file,
    config,
    core::Language,
    io,
    priority::{
        call_graph::{CallGraph, FunctionId},
        parallel_call_graph::{ParallelCallGraph, ParallelConfig},
    },
};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Call graph construction phases for progress tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallGraphPhase {
    DiscoveringFiles,
    ParsingASTs,
    ExtractingCalls,
    LinkingModules,
}

/// Progress information for call graph construction
#[derive(Debug, Clone)]
pub struct CallGraphProgress {
    pub phase: CallGraphPhase,
    pub current: usize,
    pub total: usize,
}

/// Parallel call graph builder for Rust projects
pub struct ParallelCallGraphBuilder;

impl Default for ParallelCallGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelCallGraphBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn with_config(_config: ParallelConfig) -> Self {
        // Config is no longer used as thread pool is configured globally
        Self
    }

    /// Build call graph with parallel processing
    pub fn build_parallel<F>(
        &self,
        project_path: &Path,
        base_graph: CallGraph,
        progress_callback: F,
    ) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)>
    where
        F: FnMut(CallGraphProgress) + Send + Sync,
    {
        self.build_parallel_with_files(project_path, base_graph, None, progress_callback)
    }

    /// Build call graph with parallel processing, using optional pre-discovered files
    ///
    /// If `rust_files` is provided, skips file discovery and uses the given files.
    /// This avoids redundant filesystem walking when files were already discovered.
    ///
    /// Spec 210: Uses batched processing to prevent proc-macro2 SourceMap overflow.
    /// Files are processed in batches of ~200 files, with SourceMap reset between batches.
    pub fn build_parallel_with_files<F>(
        &self,
        project_path: &Path,
        base_graph: CallGraph,
        rust_files: Option<&[PathBuf]>,
        mut progress_callback: F,
    ) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)>
    where
        F: FnMut(CallGraphProgress) + Send + Sync,
    {
        let discovered_files: Vec<PathBuf>;
        let rust_files = match rust_files {
            Some(files) => {
                // Skip discover phase - files already known from stage 0
                log::info!("Using {} pre-discovered Rust files", files.len());
                files
            }
            None => {
                // Phase 1: Discover files (only when not pre-discovered)
                progress_callback(CallGraphProgress {
                    phase: CallGraphPhase::DiscoveringFiles,
                    current: 0,
                    total: 0,
                });

                let config = config::get_config();
                discovered_files = io::walker::find_project_files_with_config(
                    project_path,
                    vec![Language::Rust],
                    config,
                )
                .context("Failed to find Rust files for call graph")?;
                log::info!("Discovered {} Rust files", discovered_files.len());

                // Mark discover phase complete
                progress_callback(CallGraphProgress {
                    phase: CallGraphPhase::DiscoveringFiles,
                    current: discovered_files.len(),
                    total: discovered_files.len(),
                });

                &discovered_files
            }
        };

        let total_files = rust_files.len();
        log::info!("Processing {} Rust files in parallel", total_files);

        // Create parallel call graph
        let parallel_graph = Arc::new(ParallelCallGraph::new(total_files));

        // Initialize with base graph
        parallel_graph.merge_concurrent(base_graph);

        // Spec 210: Batch size to prevent SourceMap overflow
        // 200 files * ~50KB avg = ~10MB per batch, well under the 4GB limit
        const BATCH_SIZE: usize = 200;

        let mut all_framework_exclusions = HashSet::new();
        let mut all_function_pointer_used = HashSet::new();
        let mut files_processed = 0;

        // Add minimum visibility pause
        std::thread::sleep(std::time::Duration::from_millis(150));

        // Process files in batches to prevent SourceMap overflow
        for batch in rust_files.chunks(BATCH_SIZE) {
            let batch_start = files_processed;
            let batch_end = batch_start + batch.len();

            // Phase 2: Parse ASTs for this batch
            progress_callback(CallGraphProgress {
                phase: CallGraphPhase::ParsingASTs,
                current: batch_start,
                total: total_files,
            });

            let parsed_files = self.parallel_parse_files_batch(batch, &parallel_graph)?;

            // Phase 3: Extract calls for this batch
            progress_callback(CallGraphProgress {
                phase: CallGraphPhase::ExtractingCalls,
                current: batch_start,
                total: total_files,
            });

            self.parallel_multi_file_extraction(&parsed_files, &parallel_graph)?;

            // Phase 4: Enhanced analysis for this batch
            let (batch_framework_exclusions, batch_function_pointer_used) =
                self.parallel_enhanced_analysis(&parsed_files, &parallel_graph)?;

            all_framework_exclusions.extend(batch_framework_exclusions);
            all_function_pointer_used.extend(batch_function_pointer_used);

            files_processed = batch_end;

            // Reset SourceMap after each batch to prevent overflow
            // The parsed ASTs are no longer needed after extraction
            crate::core::parsing::reset_span_locations();

            log::debug!(
                "Processed batch {}/{} ({} files)",
                batch_end,
                total_files,
                batch.len()
            );
        }

        // Final progress update
        progress_callback(CallGraphProgress {
            phase: CallGraphPhase::LinkingModules,
            current: 0,
            total: 0,
        });

        // Convert to regular CallGraph
        let mut final_graph = parallel_graph.to_call_graph();
        final_graph.resolve_cross_file_calls();

        // Report statistics
        let stats = parallel_graph.stats();
        log::info!(
            "Parallel call graph complete: {} nodes, {} edges, {} files processed in {} batches",
            stats.total_nodes.load(std::sync::atomic::Ordering::Relaxed),
            stats.total_edges.load(std::sync::atomic::Ordering::Relaxed),
            stats
                .files_processed
                .load(std::sync::atomic::Ordering::Relaxed),
            total_files.div_ceil(BATCH_SIZE),
        );

        Ok((
            final_graph,
            all_framework_exclusions,
            all_function_pointer_used,
        ))
    }

    /// Parse a batch of files without progress tracking (used in batched processing)
    ///
    /// Note: Uses sequential iteration because syn::File doesn't implement Send
    /// when compiled with proc-macro feature (spans contain non-Send types).
    fn parallel_parse_files_batch(
        &self,
        batch: &[PathBuf],
        parallel_graph: &Arc<ParallelCallGraph>,
    ) -> Result<Vec<(PathBuf, syn::File)>> {
        // Read file contents in parallel (I/O bound, content is Send)
        let file_contents: Vec<_> = batch
            .par_iter()
            .filter_map(|file_path| {
                io::read_file(file_path)
                    .map_err(|e| {
                        log::warn!("Failed to read file {}: {}", file_path.display(), e);
                        e
                    })
                    .ok()
                    .map(|content| (file_path.clone(), content))
            })
            .collect();

        // Parse sequentially (syn::File is not Send)
        // Note: DO NOT reset SourceMap here - ASTs are held and used later
        // for call graph analysis. Span references must remain valid.
        let parsed_files: Vec<_> = file_contents
            .iter()
            .filter_map(|(file_path, content)| {
                let parsed = syn::parse_file(content).ok()?;
                parallel_graph.stats().increment_files();
                Some((file_path.clone(), parsed))
            })
            .collect();

        Ok(parsed_files)
    }

    /// Phase 1: Read and parse files with progress tracking
    #[allow(dead_code)]
    fn parallel_parse_files_with_progress<F>(
        &self,
        rust_files: &[PathBuf],
        parallel_graph: &Arc<ParallelCallGraph>,
        progress_callback: &mut F,
    ) -> Result<Vec<(PathBuf, syn::File)>>
    where
        F: FnMut(CallGraphProgress) + Send + Sync,
    {
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Step 1: Read file contents in parallel (I/O bound)
        let file_contents: Vec<_> = rust_files
            .par_iter()
            .filter_map(|file_path| {
                let content = io::read_file(file_path)
                    .map_err(|e| {
                        eprintln!(
                            "Warning: Failed to read file {}: {}",
                            file_path.display(),
                            e
                        );
                        e
                    })
                    .ok()?;
                Some((file_path.clone(), content))
            })
            .collect();

        // Step 2: Parse files to AST with progress tracking
        // Note: DO NOT reset SourceMap here - ASTs are held and used later
        // for call graph analysis. Span references must remain valid.
        let total_files = file_contents.len();
        let parsed_count = Arc::new(AtomicUsize::new(0));

        let parsed_files: Vec<_> = file_contents
            .iter()
            .enumerate()
            .filter_map(|(idx, (file_path, content))| {
                let parsed = syn::parse_file(content).ok()?;
                parallel_graph.stats().increment_files();

                let count = parsed_count.fetch_add(1, Ordering::Relaxed) + 1;

                // Throttled progress updates (every 10 files or at completion)
                if count % 10 == 0 || count == total_files {
                    progress_callback(CallGraphProgress {
                        phase: CallGraphPhase::ParsingASTs,
                        current: count,
                        total: total_files,
                    });
                }

                // Update unified progress
                crate::io::progress::AnalysisProgress::with_global(|p| {
                    p.update_progress(crate::io::progress::PhaseProgress::Progress {
                        current: idx + 1,
                        total: total_files,
                    });
                });

                Some((file_path.clone(), parsed))
            })
            .collect();

        Ok(parsed_files)
    }

    /// Phase 2: Extract multi-file call graph from pre-parsed ASTs
    ///
    /// Uses pre-parsed ASTs to avoid redundant parsing operations.
    /// Processes all files at once for optimal cross-file call resolution.
    fn parallel_multi_file_extraction(
        &self,
        parsed_files: &[(PathBuf, syn::File)],
        parallel_graph: &Arc<ParallelCallGraph>,
    ) -> Result<()> {
        // Process ALL files at once (no chunking)
        // This enables optimal cross-file call resolution with a single PathResolver
        // and complete visibility of all functions across the entire codebase.
        // Progress tracking is handled inside extract_call_graph_multi_file()
        let files_for_extraction: Vec<_> = parsed_files
            .iter()
            .map(|(path, parsed)| (parsed.clone(), path.clone()))
            .collect();

        // Extract call graph for all files with full cross-file resolution
        // This will show progress for:
        // - Phase 1: Analyzing functions and imports (X/Y files)
        // - Phase 2: Resolving function calls (X/Y calls)
        // - Phase 3: Final cross-file resolution (X/Y calls)
        let graph = extract_call_graph_multi_file(&files_for_extraction);

        // Merge into main graph
        parallel_graph.merge_concurrent(graph);

        Ok(())
    }

    /// Phase 3: Enhanced analysis using pre-parsed ASTs
    ///
    /// Uses pre-parsed ASTs to avoid redundant parsing operations.
    fn parallel_enhanced_analysis(
        &self,
        parsed_files: &[(PathBuf, syn::File)],
        parallel_graph: &Arc<ParallelCallGraph>,
    ) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
        // Use already-parsed files directly - no re-parsing needed!
        let workspace_files: Vec<(PathBuf, syn::File)> = parsed_files
            .iter()
            .map(|(path, parsed)| (path.clone(), parsed.clone()))
            .collect();

        // Create thread-safe enhanced builder
        let base_graph = parallel_graph.to_call_graph();
        let mut enhanced_builder = RustCallGraphBuilder::from_base_graph(base_graph);

        // Suppress old progress bars - unified system already shows "3/4 Building call graph"
        // Process files sequentially for enhanced analysis
        // (This is complex to parallelize due to shared state)
        for (file_path, parsed) in &workspace_files {
            enhanced_builder
                .analyze_basic_calls(file_path, parsed)?
                .analyze_trait_dispatch(file_path, parsed)?
                .analyze_function_pointers(file_path, parsed)?
                .analyze_framework_patterns(file_path, parsed)?;
        }

        // Cross-module analysis (no progress bar needed)
        enhanced_builder.analyze_cross_module(&workspace_files)?;

        // Finalize trait analysis - detect patterns ONCE after all files processed
        enhanced_builder.finalize_trait_analysis()?;

        // Extract results
        let enhanced_graph = enhanced_builder.build();

        let framework_exclusions: HashSet<FunctionId> = enhanced_graph
            .framework_patterns
            .get_exclusions()
            .into_iter()
            .collect();

        let function_pointer_used: HashSet<FunctionId> = enhanced_graph
            .function_pointer_tracker
            .get_definitely_used_functions()
            .into_iter()
            .collect();

        // Merge enhanced graph into parallel graph
        parallel_graph.merge_concurrent(enhanced_graph.base_graph);

        Ok((framework_exclusions, function_pointer_used))
    }
}

/// Parallel processing entry point for call graph construction
pub fn build_call_graph_parallel<F>(
    project_path: &Path,
    base_graph: CallGraph,
    num_threads: Option<usize>,
    progress_callback: F,
) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)>
where
    F: FnMut(CallGraphProgress) + Send + Sync,
{
    build_call_graph_parallel_with_files(
        project_path,
        base_graph,
        num_threads,
        None,
        progress_callback,
    )
}

/// Parallel processing entry point with optional pre-discovered files
///
/// If `rust_files` is provided, skips file discovery and uses the given files.
/// This avoids redundant filesystem walking when files were already discovered.
pub fn build_call_graph_parallel_with_files<F>(
    project_path: &Path,
    base_graph: CallGraph,
    num_threads: Option<usize>,
    rust_files: Option<&[PathBuf]>,
    progress_callback: F,
) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)>
where
    F: FnMut(CallGraphProgress) + Send + Sync,
{
    let mut config = ParallelConfig::default();

    if let Some(threads) = num_threads {
        config = config.with_threads(threads);
    }

    let builder = ParallelCallGraphBuilder::with_config(config);
    builder.build_parallel_with_files(project_path, base_graph, rust_files, progress_callback)
}

// ============================================================================
// Spec 213: Call Graph Building from Extracted Data
// ============================================================================

use crate::extraction::{ExtractedFileData, ExtractedFunctionData};
use std::collections::HashMap;

/// Build call graph from pre-extracted file data (spec 213).
///
/// Uses extracted call information to build the call graph without re-parsing files.
/// This prevents proc-macro2 SourceMap overflow on large codebases.
///
/// # Arguments
///
/// * `base_graph` - Base call graph from function metrics
/// * `extracted` - Pre-extracted file data from unified extraction phase
///
/// # Returns
///
/// Tuple of (CallGraph, framework_exclusions, function_pointer_used)
pub fn build_call_graph_from_extracted(
    base_graph: CallGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> (CallGraph, HashSet<FunctionId>, HashSet<FunctionId>) {
    use crate::priority::call_graph::CallType as GraphCallType;

    let sorted_extracted = extracted_files_sorted(extracted);
    let callee_index = CalleeResolutionIndex::from_sorted_extracted(&sorted_extracted);
    let mut final_graph = base_graph;

    for (path, file_data) in &sorted_extracted {
        for func in &file_data.functions {
            let function_id = extracted_function_id(path, func);
            if final_graph.get_function_info(&function_id).is_some() {
                continue;
            }
            let roles = crate::analysis::role_policy::classify_roles(
                &crate::analysis::role_policy::evidence_for_facts(
                    crate::analysis::role_policy::RoleFacts {
                        path,
                        language: crate::core::Language::from_path(path),
                        name: &func.qualified_name,
                        is_test: func.is_test,
                        in_test_module: func.in_test_module,
                        visibility: func.visibility.as_deref(),
                    },
                ),
            );
            final_graph.add_function_with_roles(function_id, roles, func.cyclomatic, func.length);
        }
    }

    for (path, file_data) in sorted_extracted {
        for func in &file_data.functions {
            let caller = extracted_function_id(path, func);
            for call in &func.calls {
                let outcome = resolve_callee_from_extracted(call, path, &callee_index);
                final_graph.add_resolution(caller.clone(), GraphCallType::Direct, outcome);
            }
        }
    }

    // For now, no framework exclusions or function pointer detection from extracted data
    // These require deeper AST analysis that isn't captured in extraction
    let framework_exclusions = HashSet::new();
    let function_pointer_used = HashSet::new();

    log::info!(
        "Call graph from extracted data: {} nodes in {} files",
        final_graph.node_count(),
        extracted.len()
    );

    (final_graph, framework_exclusions, function_pointer_used)
}

struct CalleeResolutionIndex {
    same_file_functions: HashMap<PathBuf, HashMap<String, Vec<FunctionId>>>,
    qualified_functions: HashMap<String, Vec<FunctionId>>,
    method_functions: HashMap<String, Vec<FunctionId>>,
}

fn extracted_files_sorted(
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> Vec<(&PathBuf, &ExtractedFileData)> {
    let mut files: Vec<_> = extracted.iter().collect();
    files.sort_by(|left, right| left.0.cmp(right.0));
    files
}

impl CalleeResolutionIndex {
    fn from_sorted_extracted(sorted_extracted: &[(&PathBuf, &ExtractedFileData)]) -> Self {
        sorted_extracted
            .iter()
            .fold(Self::empty(), |mut index, item| {
                index.add_file_functions(item.0, &item.1.functions);
                index
            })
    }

    fn empty() -> Self {
        Self {
            same_file_functions: HashMap::new(),
            qualified_functions: HashMap::new(),
            method_functions: HashMap::new(),
        }
    }

    fn add_file_functions(&mut self, path: &Path, functions: &[ExtractedFunctionData]) {
        let file_functions = self
            .same_file_functions
            .entry(path.to_path_buf())
            .or_default();
        for func in functions {
            let function_id = extracted_function_id(path, func);
            add_same_file_function(file_functions, func, &function_id);
            add_candidate(
                &mut self.qualified_functions,
                &func.qualified_name,
                &function_id,
            );
            add_candidate(&mut self.method_functions, &func.name, &function_id);
        }
    }
}

fn extracted_function_id(path: &Path, func: &ExtractedFunctionData) -> FunctionId {
    FunctionId::new(path.to_path_buf(), func.qualified_name.clone(), func.line)
}

fn add_same_file_function(
    file_functions: &mut HashMap<String, Vec<FunctionId>>,
    func: &ExtractedFunctionData,
    function_id: &FunctionId,
) {
    add_candidate(file_functions, &func.qualified_name, function_id);
    add_candidate(file_functions, &func.name, function_id);
}

fn add_candidate(
    functions: &mut HashMap<String, Vec<FunctionId>>,
    key: &str,
    function_id: &FunctionId,
) {
    let candidates = functions.entry(key.to_string()).or_default();
    if !candidates.contains(function_id) {
        candidates.push(function_id.clone());
        candidates.sort();
    }
}

/// Resolve a callee name to a FunctionId using extracted data.
fn resolve_callee_from_extracted(
    call: &crate::extraction::CallSite,
    caller_file: &Path,
    index: &CalleeResolutionIndex,
) -> crate::priority::call_graph::ResolutionOutcome {
    use crate::extraction::CallType;
    use crate::priority::call_graph::ResolutionOutcome;

    match call.call_type {
        CallType::Direct | CallType::StaticMethod | CallType::TraitMethod => {
            resolve_direct_callee(call, caller_file, index)
        }
        CallType::Method => {
            if crate::analyzers::call_graph::CallResolver::is_common_library_method(
                &call.callee_name,
            ) {
                return ResolutionOutcome::Ignored {
                    reason: format!("common library method {}", call.callee_name),
                };
            }
            let candidates = index
                .method_functions
                .get(&call.callee_name)
                .cloned()
                .unwrap_or_default();
            outcome_for_candidates(
                candidates,
                call,
                caller_file,
                crate::priority::call_graph::CallEdgeProvenance::NameHeuristic,
                60,
            )
        }
        CallType::Closure | CallType::FunctionPointer => ResolutionOutcome::Ignored {
            reason: "dynamic callable".to_string(),
        },
    }
}

fn resolve_direct_callee(
    call: &crate::extraction::CallSite,
    caller_file: &Path,
    index: &CalleeResolutionIndex,
) -> crate::priority::call_graph::ResolutionOutcome {
    let local = index
        .same_file_functions
        .get(caller_file)
        .and_then(|functions| functions.get(&call.callee_name))
        .cloned()
        .unwrap_or_default();
    if !local.is_empty() {
        return outcome_for_candidates(
            local,
            call,
            caller_file,
            crate::priority::call_graph::CallEdgeProvenance::AstDirect,
            100,
        );
    }
    let global = index
        .qualified_functions
        .get(&call.callee_name)
        .cloned()
        .unwrap_or_default();
    outcome_for_candidates(
        global,
        call,
        caller_file,
        crate::priority::call_graph::CallEdgeProvenance::ImportResolution,
        85,
    )
}

fn outcome_for_candidates(
    mut candidates: Vec<FunctionId>,
    call: &crate::extraction::CallSite,
    caller_file: &Path,
    provenance: crate::priority::call_graph::CallEdgeProvenance,
    confidence: u8,
) -> crate::priority::call_graph::ResolutionOutcome {
    use crate::priority::call_graph::{CallSite, ResolutionOutcome};
    candidates.sort();
    match candidates.as_slice() {
        [target] => ResolutionOutcome::Resolved {
            target: target.clone(),
            provenance,
            confidence,
            call_site: Some(CallSite {
                file: caller_file.to_path_buf(),
                line: call.line,
                column: None,
            }),
        },
        [] => ResolutionOutcome::Unresolved {
            query: call.callee_name.clone(),
        },
        _ => ResolutionOutcome::Ambiguous { candidates },
    }
}

#[cfg(test)]
mod extracted_call_resolution_tests {
    use super::*;
    use crate::extraction::{CallType, ExtractedFileData, ExtractedFunctionData};

    #[test]
    fn direct_calls_prefer_same_file_before_qualified_index() {
        let caller = PathBuf::from("src/caller.rs");
        let other = PathBuf::from("src/other.rs");
        let extracted = extracted_files(vec![
            (
                caller.clone(),
                vec![function("helper", "local::helper", 10)],
            ),
            (other, vec![function("helper", "helper", 20)]),
        ]);
        let index = CalleeResolutionIndex::from_sorted_extracted(&sorted(&extracted));

        let outcome =
            resolve_callee_from_extracted(&call("helper", CallType::Direct, 6), &caller, &index);
        let crate::priority::call_graph::ResolutionOutcome::Resolved {
            target: resolved, ..
        } = outcome
        else {
            panic!("expected resolved call, got {outcome:?}");
        };

        assert_eq!(resolved.file, caller);
        assert_eq!(resolved.name, "local::helper");
        assert_eq!(resolved.line, 10);
    }

    #[test]
    fn ambiguous_method_names_produce_no_resolved_target() {
        let first = PathBuf::from("src/a.rs");
        let second = PathBuf::from("src/b.rs");
        let mut entry = function("entry", "entry", 1);
        entry.calls = vec![call("run", CallType::Method, 2)];
        let extracted = extracted_files(vec![
            (second, vec![function("run", "Second::run", 20)]),
            (
                first.clone(),
                vec![entry, function("run", "First::run", 10)],
            ),
        ]);
        let index = CalleeResolutionIndex::from_sorted_extracted(&sorted(&extracted));

        let outcome =
            resolve_callee_from_extracted(&call("run", CallType::Method, 6), &first, &index);

        assert!(matches!(
            outcome,
            crate::priority::call_graph::ResolutionOutcome::Ambiguous { .. }
        ));
        let (graph, _, _) = build_call_graph_from_extracted(CallGraph::new(), &extracted);
        let entry_id = FunctionId::new(first, "entry".to_string(), 1);
        assert!(graph.get_callees_exact(&entry_id).is_empty());
        assert_eq!(graph.edge_evidence().count(), 0);
    }

    #[test]
    fn common_library_methods_do_not_resolve_by_simple_method_name() {
        let caller = PathBuf::from("src/builders/parallel_unified_analysis.rs");
        let support = PathBuf::from("src/support.rs");
        let extracted = extracted_files(vec![
            (caller.clone(), vec![function("entry", "entry", 5)]),
            (
                support,
                vec![
                    function("filter", "LazyPipeline::filter", 10),
                    function("map", "LazyPipeline::map", 20),
                    function("take", "LazyPipeline::take", 30),
                    function("get", "PurityCache::get", 40),
                ],
            ),
        ]);
        let index = CalleeResolutionIndex::from_sorted_extracted(&sorted(&extracted));

        for method in ["filter", "map", "take", "get"] {
            let outcome =
                resolve_callee_from_extracted(&call(method, CallType::Method, 6), &caller, &index);

            assert!(
                matches!(
                    outcome,
                    crate::priority::call_graph::ResolutionOutcome::Ignored { .. }
                ),
                "common library method {method} should be ignored, got {outcome:?}"
            );
        }
    }

    #[test]
    fn build_call_graph_from_extracted_preserves_direct_and_method_edges() {
        let caller = PathBuf::from("src/caller.rs");
        let helper = PathBuf::from("src/helper.rs");
        let mut entry = function("entry", "entry", 5);
        entry.calls = vec![
            crate::extraction::CallSite {
                callee_name: "local_helper".to_string(),
                call_type: CallType::Direct,
                line: 6,
            },
            crate::extraction::CallSite {
                callee_name: "Helper::remote".to_string(),
                call_type: CallType::StaticMethod,
                line: 7,
            },
            crate::extraction::CallSite {
                callee_name: "run".to_string(),
                call_type: CallType::Method,
                line: 8,
            },
        ];

        let extracted = extracted_files(vec![
            (
                caller.clone(),
                vec![entry, function("local_helper", "local_helper", 20)],
            ),
            (
                helper.clone(),
                vec![
                    function("remote", "Helper::remote", 10),
                    function("run", "Helper::run", 30),
                ],
            ),
        ]);

        let (graph, _, _) = build_call_graph_from_extracted(CallGraph::new(), &extracted);
        let entry_id = FunctionId::new(caller.clone(), "entry".to_string(), 5);
        let callees = graph.get_callees_exact(&entry_id);
        let callee_names: Vec<_> = callees.iter().map(|id| id.name.as_str()).collect();

        assert_eq!(callees.len(), 3);
        assert!(callee_names.contains(&"local_helper"));
        assert!(callee_names.contains(&"Helper::remote"));
        assert!(callee_names.contains(&"Helper::run"));
        let evidence: Vec<_> = graph.edge_evidence().collect();
        assert_eq!(evidence.len(), 3);
        assert!(evidence.iter().all(|edge| edge.confidence > 0));
        assert!(evidence.iter().all(|edge| edge.call_site.is_some()));
    }

    #[test]
    fn extracted_nodes_do_not_overwrite_base_roles() {
        let path = PathBuf::from("src/entry.py");
        let function_id = FunctionId::new(path.clone(), "main".to_string(), 1);
        let roles = crate::analysis::role_policy::CodeRoles {
            is_test: false,
            is_entry_point: true,
            is_framework_managed: true,
            is_public_api: true,
        };
        let mut base_graph = CallGraph::new();
        base_graph.add_function_with_roles(function_id.clone(), roles, 1, 2);
        let extracted = extracted_files(vec![(path, vec![function("main", "main", 1)])]);

        let (graph, _, _) = build_call_graph_from_extracted(base_graph, &extracted);

        assert_eq!(graph.nodes[&function_id].roles, roles);
    }

    fn extracted_files(
        files: Vec<(PathBuf, Vec<ExtractedFunctionData>)>,
    ) -> HashMap<PathBuf, ExtractedFileData> {
        files
            .into_iter()
            .map(|(path, functions)| {
                let mut file_data = ExtractedFileData::empty(path.clone());
                file_data.functions = functions;
                (path, file_data)
            })
            .collect()
    }

    fn function(name: &str, qualified_name: &str, line: usize) -> ExtractedFunctionData {
        let mut function = ExtractedFunctionData::minimal(name, line);
        function.qualified_name = qualified_name.to_string();
        function
    }

    fn call(name: &str, call_type: CallType, line: usize) -> crate::extraction::CallSite {
        crate::extraction::CallSite {
            callee_name: name.to_string(),
            call_type,
            line,
        }
    }

    fn sorted(
        extracted: &HashMap<PathBuf, ExtractedFileData>,
    ) -> Vec<(&PathBuf, &ExtractedFileData)> {
        let mut sorted: Vec<_> = extracted.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        sorted
    }
}
