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
        mut progress_callback: F,
    ) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)>
    where
        F: FnMut(CallGraphProgress) + Send + Sync,
    {
        // Phase 1: Discover files
        progress_callback(CallGraphProgress {
            phase: CallGraphPhase::DiscoveringFiles,
            current: 0,
            total: 0,
        });

        // Find all Rust files
        let config = config::get_config();
        let rust_files =
            io::walker::find_project_files_with_config(project_path, vec![Language::Rust], config)
                .context("Failed to find Rust files for call graph")?;

        let total_files = rust_files.len();
        log::info!("Processing {} Rust files in parallel", total_files);

        // Create parallel call graph
        let parallel_graph = Arc::new(ParallelCallGraph::new(total_files));

        // Initialize with base graph
        parallel_graph.merge_concurrent(base_graph);

        // Add minimum visibility pause
        std::thread::sleep(std::time::Duration::from_millis(150));

        // Phase 2: Parse ASTs
        progress_callback(CallGraphProgress {
            phase: CallGraphPhase::ParsingASTs,
            current: 0,
            total: total_files,
        });

        let parsed_files = self.parallel_parse_files_with_progress(
            &rust_files,
            &parallel_graph,
            &mut progress_callback,
        )?;

        // Add minimum visibility pause
        std::thread::sleep(std::time::Duration::from_millis(150));

        // Phase 3: Extract calls
        progress_callback(CallGraphProgress {
            phase: CallGraphPhase::ExtractingCalls,
            current: 0,
            total: total_files,
        });

        self.parallel_multi_file_extraction(&parsed_files, &parallel_graph)?;

        // Add minimum visibility pause
        std::thread::sleep(std::time::Duration::from_millis(150));

        // Phase 4: Link modules
        progress_callback(CallGraphProgress {
            phase: CallGraphPhase::LinkingModules,
            current: 0,
            total: 0,
        });

        let (framework_exclusions, function_pointer_used) =
            self.parallel_enhanced_analysis(&parsed_files, &parallel_graph)?;

        // Convert to regular CallGraph
        let mut final_graph = parallel_graph.to_call_graph();
        final_graph.resolve_cross_file_calls();

        // Report statistics
        let stats = parallel_graph.stats();
        log::info!(
            "Parallel call graph complete: {} nodes, {} edges, {} files processed",
            stats.total_nodes.load(std::sync::atomic::Ordering::Relaxed),
            stats.total_edges.load(std::sync::atomic::Ordering::Relaxed),
            stats
                .files_processed
                .load(std::sync::atomic::Ordering::Relaxed),
        );

        Ok((final_graph, framework_exclusions, function_pointer_used))
    }

    /// Phase 1: Read and parse files with progress tracking
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
        let total_files = file_contents.len();
        let parsed_count = Arc::new(AtomicUsize::new(0));

        let parsed_files: Vec<_> = file_contents
            .iter()
            .enumerate()
            .filter_map(|(idx, (file_path, content))| {
                let parsed = syn::parse_file(content)
                    .map_err(|e| {
                        eprintln!("Warning: Failed to parse {}: {}", file_path.display(), e);
                        e
                    })
                    .ok()?;
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
    let mut config = ParallelConfig::default();

    if let Some(threads) = num_threads {
        config = config.with_threads(threads);
    }

    let builder = ParallelCallGraphBuilder::with_config(config);
    builder.build_parallel(project_path, base_graph, progress_callback)
}
