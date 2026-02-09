use crate::{
    analysis::call_graph::RustCallGraphBuilder,
    analyzers::rust_call_graph::extract_call_graph_multi_file,
    builders::parallel_call_graph::{CallGraphPhase, CallGraphProgress},
    config,
    core::FunctionMetrics,
    core::Language,
    io, priority,
};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Parsed workspace files ready for call graph analysis
type ParsedFile = (PathBuf, syn::File);
type ExpandedFile = (syn::File, PathBuf);

/// Result of call graph finalization containing exclusions and used functions
pub struct CallGraphResult {
    pub framework_exclusions: HashSet<priority::call_graph::FunctionId>,
    pub function_pointer_used: HashSet<priority::call_graph::FunctionId>,
}

pub fn build_initial_call_graph(metrics: &[FunctionMetrics]) -> priority::CallGraph {
    let mut call_graph = priority::CallGraph::new();

    for metric in metrics {
        let func_id = priority::call_graph::FunctionId::new(
            metric.file.clone(),
            metric.name.clone(),
            metric.line,
        );

        call_graph.add_function(
            func_id,
            is_entry_point(&metric.name),
            is_test_function(&metric.name, &metric.file, metric.is_test),
            metric.cyclomatic,
            metric.length,
        );
    }

    call_graph
}

fn is_entry_point(function_name: &str) -> bool {
    match function_name {
        "main" => true,
        name if name.starts_with("handle_") => true,
        name if name.starts_with("run_") => true,
        _ => false,
    }
}

fn is_test_function(function_name: &str, file_path: &Path, is_test_attr: bool) -> bool {
    is_test_attr
        || function_name.starts_with("test_")
        || file_path.to_string_lossy().contains("test")
}

pub fn process_rust_files_for_call_graph<F>(
    project_path: &Path,
    call_graph: &mut priority::CallGraph,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    progress_callback: F,
) -> Result<(
    HashSet<priority::call_graph::FunctionId>,
    HashSet<priority::call_graph::FunctionId>,
)>
where
    F: FnMut(CallGraphProgress),
{
    process_rust_files_for_call_graph_with_files(
        project_path,
        call_graph,
        verbose_macro_warnings,
        show_macro_stats,
        None,
        progress_callback,
    )
}

/// Process Rust files for call graph with optional pre-discovered files
///
/// Orchestrates the call graph building pipeline:
/// 1. Discover files (if not pre-provided)
/// 2. Parse ASTs
/// 3. Extract and analyze calls
/// 4. Finalize and merge results
pub fn process_rust_files_for_call_graph_with_files<F>(
    project_path: &Path,
    call_graph: &mut priority::CallGraph,
    _verbose_macro_warnings: bool,
    _show_macro_stats: bool,
    rust_files: Option<&[PathBuf]>,
    mut progress_callback: F,
) -> Result<(
    HashSet<priority::call_graph::FunctionId>,
    HashSet<priority::call_graph::FunctionId>,
)>
where
    F: FnMut(CallGraphProgress),
{
    // Phase 1: Discover or use pre-discovered files
    let discovered_files = discover_rust_files(project_path, rust_files, &mut progress_callback)?;
    let rust_files = rust_files.unwrap_or(&discovered_files);
    let total_files = rust_files.len();

    // Phase 2: Parse ASTs
    let (workspace_files, expanded_files) =
        parse_rust_files(rust_files, total_files, &mut progress_callback);

    // Phase 3: Extract and analyze calls
    let enhanced_builder =
        analyze_workspace_calls(call_graph, &workspace_files, &expanded_files, &mut progress_callback)?;

    // Phase 4: Finalize and merge
    let result = finalize_call_graph(call_graph, enhanced_builder, &mut progress_callback)?;

    // Reset SourceMap after all call graph extraction is complete
    crate::core::parsing::reset_span_locations();

    Ok((result.framework_exclusions, result.function_pointer_used))
}

/// Phase 1: Discover Rust files in the project
///
/// If files are pre-provided, logs and returns empty (caller uses pre-provided).
/// Otherwise, walks the filesystem to find all Rust files.
fn discover_rust_files<F>(
    project_path: &Path,
    pre_discovered: Option<&[PathBuf]>,
    progress_callback: &mut F,
) -> Result<Vec<PathBuf>>
where
    F: FnMut(CallGraphProgress),
{
    if let Some(files) = pre_discovered {
        log::info!("Using {} pre-discovered Rust files", files.len());
        return Ok(Vec::new());
    }

    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::DiscoveringFiles,
        current: 0,
        total: 0,
    });

    let config = config::get_config();
    let discovered_files = io::walker::find_project_files_with_config(
        project_path,
        vec![Language::Rust],
        config,
    )
    .context("Failed to find Rust files for call graph")?;

    log::info!("Discovered {} Rust files", discovered_files.len());

    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::DiscoveringFiles,
        current: discovered_files.len(),
        total: discovered_files.len(),
    });

    Ok(discovered_files)
}

/// Phase 2: Parse Rust files into ASTs
///
/// Returns two collections:
/// - workspace_files: (path, ast) pairs for enhanced analysis
/// - expanded_files: (ast, path) pairs for multi-file extraction
fn parse_rust_files<F>(
    rust_files: &[PathBuf],
    total_files: usize,
    progress_callback: &mut F,
) -> (Vec<ParsedFile>, Vec<ExpandedFile>)
where
    F: FnMut(CallGraphProgress),
{
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::ParsingASTs,
        current: 0,
        total: total_files,
    });

    let mut workspace_files = Vec::with_capacity(rust_files.len());
    let mut expanded_files = Vec::with_capacity(rust_files.len());

    for (idx, file_path) in rust_files.iter().enumerate() {
        if let Some((parsed, expanded)) = parse_single_file(file_path) {
            workspace_files.push(parsed);
            expanded_files.push(expanded);
        }

        report_progress_throttled(idx + 1, total_files, CallGraphPhase::ParsingASTs, progress_callback);
    }

    (workspace_files, expanded_files)
}

/// Parse a single Rust file into AST
///
/// Returns None if file cannot be read or parsed.
fn parse_single_file(file_path: &Path) -> Option<(ParsedFile, ExpandedFile)> {
    let content = io::read_file(file_path).ok()?;
    let parsed = syn::parse_file(&content).ok()?;

    let workspace = (file_path.to_path_buf(), parsed.clone());
    let expanded = (parsed, file_path.to_path_buf());

    Some((workspace, expanded))
}

/// Phase 3: Extract and analyze calls from parsed files
fn analyze_workspace_calls<F>(
    call_graph: &mut priority::CallGraph,
    workspace_files: &[ParsedFile],
    expanded_files: &[ExpandedFile],
    progress_callback: &mut F,
) -> Result<RustCallGraphBuilder>
where
    F: FnMut(CallGraphProgress),
{
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::ExtractingCalls,
        current: 0,
        total: workspace_files.len(),
    });

    // Extract basic call graph from expanded files
    if !expanded_files.is_empty() {
        let multi_file_call_graph = extract_call_graph_multi_file(expanded_files);
        call_graph.merge(multi_file_call_graph);
    }

    // Enhanced analysis with trait dispatch, function pointers, and framework patterns
    let mut enhanced_builder = RustCallGraphBuilder::from_base_graph(call_graph.clone());

    for (file_path, parsed) in workspace_files {
        enhanced_builder
            .analyze_basic_calls(file_path, parsed)?
            .analyze_trait_dispatch(file_path, parsed)?
            .analyze_function_pointers(file_path, parsed)?
            .analyze_framework_patterns(file_path, parsed)?;
    }

    enhanced_builder.analyze_cross_module(workspace_files)?;

    Ok(enhanced_builder)
}

/// Phase 4: Finalize trait analysis and merge results
fn finalize_call_graph<F>(
    call_graph: &mut priority::CallGraph,
    mut enhanced_builder: RustCallGraphBuilder,
    progress_callback: &mut F,
) -> Result<CallGraphResult>
where
    F: FnMut(CallGraphProgress),
{
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::LinkingModules,
        current: 0,
        total: 0,
    });

    log_status("Resolving trait patterns and method calls...");
    enhanced_builder.finalize_trait_analysis()?;
    log_status_done();

    let enhanced_graph = enhanced_builder.build();

    let framework_exclusions: HashSet<priority::call_graph::FunctionId> = enhanced_graph
        .framework_patterns
        .get_exclusions()
        .into_iter()
        .collect();

    let function_pointer_used: HashSet<priority::call_graph::FunctionId> = enhanced_graph
        .function_pointer_tracker
        .get_definitely_used_functions()
        .into_iter()
        .collect();

    call_graph.merge(enhanced_graph.base_graph);
    call_graph.resolve_cross_file_calls();

    Ok(CallGraphResult {
        framework_exclusions,
        function_pointer_used,
    })
}

/// Report progress with throttling (every 10 items or at completion)
fn report_progress_throttled<F>(
    current: usize,
    total: usize,
    phase: CallGraphPhase,
    progress_callback: &mut F,
)
where
    F: FnMut(CallGraphProgress),
{
    if current % 10 == 0 || current == total {
        progress_callback(CallGraphProgress {
            phase,
            current,
            total,
        });
    }
}

/// Log status message (respects DEBTMAP_QUIET)
fn log_status(message: &str) {
    if !is_quiet_mode() {
        eprint!("{}", message);
        std::io::Write::flush(&mut std::io::stderr()).ok();
    }
}

/// Log status completion (respects DEBTMAP_QUIET)
fn log_status_done() {
    if !is_quiet_mode() {
        eprintln!(" done");
    }
}

/// Check if quiet mode is enabled via environment variable
fn is_quiet_mode() -> bool {
    std::env::var("DEBTMAP_QUIET").is_ok()
}
