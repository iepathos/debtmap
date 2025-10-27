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
    progress::ProgressManager,
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

    /// Phase 1: Read and parse files, returning parsed ASTs
    ///
    /// This function reads files in parallel and parses them to syn::File objects.
    /// Each file is parsed exactly once, eliminating redundant parsing
    /// across multiple phases.
    ///
    /// # Performance
    ///
    /// Files are read in parallel, then parsed. The parsed ASTs are stored
    /// in memory and reused in subsequent phases without re-parsing.
    fn parallel_parse_files(
        &self,
        rust_files: &[PathBuf],
        parallel_graph: &Arc<ParallelCallGraph>,
    ) -> Result<Vec<(PathBuf, syn::File)>> {
        // Create progress bar using global progress manager
        let progress = ProgressManager::global().map(|pm| {
            let pb = pm.create_bar(
                rust_files.len() as u64,
                crate::progress::TEMPLATE_CALL_GRAPH,
            );
            pb.set_message("Building call graph");
            pb
        });

        // Step 1: Read file contents in parallel (I/O bound)
        let file_contents: Vec<_> = rust_files
            .par_iter()
            .filter_map(|file_path| {
                let content = io::read_file(file_path).ok()?;
                Some((file_path.clone(), content))
            })
            .collect();

        // Step 2: Parse files to AST (cannot be parallelized due to syn::File not being Send)
        let parsed_files: Vec<_> = file_contents
            .iter()
            .filter_map(|(file_path, content)| {
                let parsed = syn::parse_file(content).ok()?;
                parallel_graph.stats().increment_files();

                if let Some(ref pb) = progress {
                    pb.inc(1);
                }

                Some((file_path.clone(), parsed))
            })
            .collect();

        // Finish progress bar with completion message
        if let Some(pb) = progress {
            pb.finish_with_message("Call graph complete");
        }

        Ok(parsed_files)
    }

    /// Phase 2: Extract multi-file call graph from pre-parsed ASTs
    ///
    /// Uses pre-parsed ASTs to avoid redundant parsing operations.
    /// Processes files sequentially due to syn::File not being Send+Sync.
    fn parallel_multi_file_extraction(
        &self,
        parsed_files: &[(PathBuf, syn::File)],
        parallel_graph: &Arc<ParallelCallGraph>,
    ) -> Result<()> {
        // Create progress bar for multi-file extraction
        let total_chunks = parsed_files.len().div_ceil(10); // chunks of size ~10
        let progress = crate::progress::ProgressManager::global()
            .map(|pm| {
                let pb = pm.create_bar(
                    total_chunks as u64,
                    "🔗 {msg} {pos}/{len} chunks ({percent}%) - {eta}",
                );
                pb.set_message("Extracting cross-file call relationships");
                pb
            })
            .unwrap_or_else(indicatif::ProgressBar::hidden);

        // Group files into chunks for better parallelization
        let chunk_size = std::cmp::max(10, parsed_files.len() / rayon::current_num_threads());

        // Process files in chunks (sequentially due to syn::File limitations)
        for chunk in parsed_files.chunks(chunk_size) {
            if !chunk.is_empty() {
                // Convert to format expected by extract_call_graph_multi_file
                // No re-parsing needed!
                let chunk_for_extraction: Vec<_> = chunk
                    .iter()
                    .map(|(path, parsed)| (parsed.clone(), path.clone()))
                    .collect();

                // Extract call graph for this chunk
                let chunk_graph = extract_call_graph_multi_file(&chunk_for_extraction);

                // Merge into main graph
                parallel_graph.merge_concurrent(chunk_graph);

                progress.inc(1);
            }
        }

        progress.finish_with_message("Cross-file analysis complete");

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

        // Create progress bar for enhanced analysis
        let progress = crate::progress::ProgressManager::global()
            .map(|pm| {
                let pb = pm.create_bar(
                    workspace_files.len() as u64,
                    "🔧 {msg} {pos}/{len} files ({percent}%) - {eta}",
                );
                pb.set_message("Enhanced call graph analysis");
                pb
            })
            .unwrap_or_else(indicatif::ProgressBar::hidden);

        // Process files sequentially for enhanced analysis
        // (This is complex to parallelize due to shared state)
        for (file_path, parsed) in &workspace_files {
            enhanced_builder
                .analyze_basic_calls(file_path, parsed)?
                .analyze_trait_dispatch(file_path, parsed)?
                .analyze_function_pointers(file_path, parsed)?
                .analyze_framework_patterns(file_path, parsed)?;
            progress.inc(1);
        }

        progress.finish_with_message("Enhanced analysis complete");

        // Cross-module analysis
        let cross_module_progress = crate::progress::ProgressManager::global()
            .map(|pm| pm.create_spinner("Analyzing cross-module calls"))
            .unwrap_or_else(indicatif::ProgressBar::hidden);

        enhanced_builder.analyze_cross_module(&workspace_files)?;
        cross_module_progress.finish_with_message("Cross-module analysis complete");

        // Finalize trait analysis - detect patterns ONCE after all files processed
        let trait_progress = crate::progress::ProgressManager::global()
            .map(|pm| pm.create_spinner("Resolving trait patterns and method calls"))
            .unwrap_or_else(indicatif::ProgressBar::hidden);

        enhanced_builder.finalize_trait_analysis()?;
        trait_progress.finish_with_message("Trait resolution complete");

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
) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)> {
    let mut config = ParallelConfig::default();

    if let Some(threads) = num_threads {
        config = config.with_threads(threads);
    }

    let builder = ParallelCallGraphBuilder::with_config(config);
    builder.build_parallel(project_path, base_graph)
}
