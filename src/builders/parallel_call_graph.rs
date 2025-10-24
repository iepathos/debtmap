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

/// Parallel call graph builder for Rust projects
pub struct ParallelCallGraphBuilder {
    config: ParallelConfig,
}

impl Default for ParallelCallGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelCallGraphBuilder {
    pub fn new() -> Self {
        Self {
            config: ParallelConfig::default(),
        }
    }

    pub fn with_config(config: ParallelConfig) -> Self {
        Self { config }
    }

    /// Build call graph with parallel processing
    pub fn build_parallel(
        &self,
        project_path: &Path,
        base_graph: CallGraph,
    ) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)> {
        // Configure Rayon thread pool if specified
        if self.config.num_threads > 0 {
            rayon::ThreadPoolBuilder::new()
                .num_threads(self.config.num_threads)
                .build_global()
                .ok(); // Ignore if already configured
        }

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

        // Phase 1: Parallel file parsing and initial extraction
        let parsed_files = self.parallel_parse_files(&rust_files, &parallel_graph)?;

        // Phase 2: Parallel multi-file call graph extraction
        self.parallel_multi_file_extraction(&parsed_files, &parallel_graph)?;

        // Phase 3: Parallel enhanced analysis
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

    /// Phase 1: Parse files in parallel
    fn parallel_parse_files(
        &self,
        rust_files: &[PathBuf],
        parallel_graph: &Arc<ParallelCallGraph>,
    ) -> Result<Vec<(PathBuf, String)>> {
        // Parse files in parallel, but only store the content strings
        let parsed_files: Vec<_> = rust_files
            .par_iter()
            .filter_map(|file_path| {
                let content = io::read_file(file_path).ok()?;

                // Update progress
                parallel_graph.stats().increment_files();
                if let Some(ref callback) = self.config.progress_callback {
                    let processed = parallel_graph
                        .stats()
                        .files_processed
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let total = parallel_graph
                        .stats()
                        .total_files
                        .load(std::sync::atomic::Ordering::Relaxed);
                    callback(processed, total);
                }

                Some((file_path.clone(), content))
            })
            .collect();

        Ok(parsed_files)
    }

    /// Phase 2: Extract multi-file call graph in parallel
    fn parallel_multi_file_extraction(
        &self,
        parsed_files: &[(PathBuf, String)],
        parallel_graph: &Arc<ParallelCallGraph>,
    ) -> Result<()> {
        // Group files into chunks for better parallelization
        let chunk_size = std::cmp::max(10, parsed_files.len() / rayon::current_num_threads());

        // Process chunks in parallel
        parsed_files.par_chunks(chunk_size).for_each(|chunk| {
            // Parse syn files within each chunk
            let parsed_chunk: Vec<_> = chunk
                .iter()
                .filter_map(|(path, content)| {
                    syn::parse_file(content)
                        .ok()
                        .map(|parsed| (parsed, path.clone()))
                })
                .collect();

            if !parsed_chunk.is_empty() {
                // Extract call graph for this chunk
                let chunk_graph = extract_call_graph_multi_file(&parsed_chunk);

                // Merge into main graph
                parallel_graph.merge_concurrent(chunk_graph);
            }
        });

        Ok(())
    }

    /// Phase 3: Enhanced analysis in parallel
    fn parallel_enhanced_analysis(
        &self,
        parsed_files: &[(PathBuf, String)],
        parallel_graph: &Arc<ParallelCallGraph>,
    ) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
        // Parse all files first (sequential but fast)
        let workspace_files: Vec<(PathBuf, syn::File)> = parsed_files
            .iter()
            .filter_map(|(path, content)| {
                syn::parse_file(content)
                    .ok()
                    .map(|parsed| (path.clone(), parsed))
            })
            .collect();

        // Create thread-safe enhanced builder
        let base_graph = parallel_graph.to_call_graph();
        let mut enhanced_builder = RustCallGraphBuilder::from_base_graph(base_graph);

        // Process files sequentially for enhanced analysis
        // (This is complex to parallelize due to shared state)
        for (file_path, parsed) in &workspace_files {
            enhanced_builder
                .analyze_basic_calls(file_path, parsed)?
                .analyze_trait_dispatch(file_path, parsed)?
                .analyze_function_pointers(file_path, parsed)?
                .analyze_framework_patterns(file_path, parsed)?;
        }

        // Cross-module analysis
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
pub fn build_call_graph_parallel(
    project_path: &Path,
    base_graph: CallGraph,
    num_threads: Option<usize>,
    show_progress: bool,
) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)> {
    let mut config = ParallelConfig::default();

    if let Some(threads) = num_threads {
        config = config.with_threads(threads);
    }

    if show_progress {
        config = config.with_progress(|processed, total| {
            let percentage = (processed as f64 / total as f64 * 100.0) as u32;
            let remaining = total - processed;

            // Update every 5% or at completion
            if processed % (total / 20).max(1) == 0 || processed == total {
                eprint!(
                    "\r🔗 Building call graph... {}/{} files ({:3}%) - {} remaining",
                    processed, total, percentage, remaining
                );
                std::io::Write::flush(&mut std::io::stderr()).ok();
            }

            // Final newline when complete
            if processed == total {
                eprintln!();
            }
        });
    }

    let builder = ParallelCallGraphBuilder::with_config(config);
    builder.build_parallel(project_path, base_graph)
}
