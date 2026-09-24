use crate::{
    builders::parallel_call_graph::{CallGraphPhase, CallGraphProgress},
    config,
    core::FunctionMetrics,
    core::Language,
    io, priority,
};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn build_initial_call_graph(metrics: &[FunctionMetrics]) -> priority::CallGraph {
    let mut call_graph = priority::CallGraph::new();

    for metric in metrics {
        let evidence = crate::analysis::role_policy::evidence_for_metric(metric);
        let func_id = priority::call_graph::FunctionId::new(
            metric.file.clone(),
            metric.name.clone(),
            metric.line,
        )
        .with_column(metric.column);

        call_graph.add_function_with_evidence(func_id, evidence, metric.cyclomatic, metric.length);
    }

    call_graph
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
    let (graph, exclusions, used) = super::rust_workspace::build(
        rust_files,
        std::mem::take(call_graph),
        false,
        progress_callback,
    )?;
    *call_graph = graph;
    Ok((exclusions, used))
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
    let discovered_files =
        io::walker::find_project_files_with_config(project_path, vec![Language::Rust], config)
            .context("Failed to find Rust files for call graph")?;

    log::info!("Discovered {} Rust files", discovered_files.len());

    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::DiscoveringFiles,
        current: discovered_files.len(),
        total: discovered_files.len(),
    });

    Ok(discovered_files)
}

/// Process TypeScript/JavaScript files for call graph
///
/// This function parses JS/TS files and extracts function call relationships,
/// merging them into the provided call graph.
///
/// # Arguments
///
/// * `project_path` - Root path of the project
/// * `call_graph` - The call graph to merge extracted calls into
/// * `js_ts_files` - Optional list of JS/TS files to process (if None, discovers files)
///
/// # Returns
///
/// Ok(()) on success, Error on failure
pub fn process_typescript_files_for_call_graph(
    project_path: &Path,
    call_graph: &mut priority::CallGraph,
    js_ts_files: Option<&[PathBuf]>,
) -> Result<()> {
    use crate::analyzers::typescript::parser::parse_source;
    use crate::analyzers::typescript::project_call_graph::extract_project_call_graph;
    use crate::core::ast::JsLanguageVariant;

    // Discover or use provided files
    let files = if let Some(files) = js_ts_files {
        files.to_vec()
    } else {
        let config = config::get_config();
        io::walker::find_project_files_with_config(
            project_path,
            vec![Language::JavaScript, Language::TypeScript],
            config,
        )
        .context("Failed to find JS/TS files for call graph")?
    };

    if files.is_empty() {
        return Ok(());
    }

    log::info!("Processing {} JS/TS files for call graph", files.len());

    let mut asts = Vec::new();
    for file_path in &files {
        // Read file content
        let content = match io::read_file(file_path) {
            Ok(c) => c,
            Err(e) => {
                log::debug!("Failed to read file {:?}: {}", file_path, e);
                continue;
            }
        };

        // Determine language variant from extension
        let variant = match file_path.extension().and_then(|e| e.to_str()) {
            Some("ts" | "tsx" | "mts" | "cts") => JsLanguageVariant::TypeScript,
            Some("jsx") => JsLanguageVariant::Jsx,
            _ => JsLanguageVariant::JavaScript,
        };

        // Parse the file
        let ast = match parse_source(&content, file_path, variant) {
            Ok(ast) => ast,
            Err(e) => {
                log::debug!("Failed to parse {:?}: {}", file_path, e);
                continue;
            }
        };

        asts.push(ast);
    }
    call_graph.merge(extract_project_call_graph(&asts));

    log::info!(
        "Merged JS/TS call graph: {} total functions",
        call_graph.node_count()
    );

    Ok(())
}
