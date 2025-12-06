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
use std::path::Path;

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
    _verbose_macro_warnings: bool,
    _show_macro_stats: bool,
    mut progress_callback: F,
) -> Result<(
    HashSet<priority::call_graph::FunctionId>,
    HashSet<priority::call_graph::FunctionId>,
)>
where
    F: FnMut(CallGraphProgress),
{
    // Phase 1: Discover files
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::DiscoveringFiles,
        current: 0,
        total: 0,
    });

    let config = config::get_config();
    let rust_files =
        io::walker::find_project_files_with_config(project_path, vec![Language::Rust], config)
            .context("Failed to find Rust files for call graph")?;

    let total_files = rust_files.len();

    // Add minimum visibility pause
    std::thread::sleep(std::time::Duration::from_millis(150));

    // Phase 2: Parse ASTs
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::ParsingASTs,
        current: 0,
        total: total_files,
    });

    let mut enhanced_builder = RustCallGraphBuilder::from_base_graph(call_graph.clone());
    let mut workspace_files = Vec::new();
    let mut expanded_files = Vec::new();

    for (idx, file_path) in rust_files.iter().enumerate() {
        if let Ok(content) = io::read_file(file_path) {
            if let Ok(parsed) = syn::parse_file(&content) {
                expanded_files.push((parsed.clone(), file_path.clone()));
                workspace_files.push((file_path.clone(), parsed));

                // Throttled progress updates (every 10 files or at completion)
                let count = idx + 1;
                if count % 10 == 0 || count == total_files {
                    progress_callback(CallGraphProgress {
                        phase: CallGraphPhase::ParsingASTs,
                        current: count,
                        total: total_files,
                    });
                }
            }
        }
    }

    // Add minimum visibility pause
    std::thread::sleep(std::time::Duration::from_millis(150));

    // Phase 3: Extract calls
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::ExtractingCalls,
        current: 0,
        total: total_files,
    });

    if !expanded_files.is_empty() {
        let multi_file_call_graph = extract_call_graph_multi_file(&expanded_files);
        call_graph.merge(multi_file_call_graph);
    }

    for (file_path, parsed) in &workspace_files {
        enhanced_builder
            .analyze_basic_calls(file_path, parsed)?
            .analyze_trait_dispatch(file_path, parsed)?
            .analyze_function_pointers(file_path, parsed)?
            .analyze_framework_patterns(file_path, parsed)?;
    }

    enhanced_builder.analyze_cross_module(&workspace_files)?;

    // Add minimum visibility pause
    std::thread::sleep(std::time::Duration::from_millis(150));

    // Phase 4: Link modules
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::LinkingModules,
        current: 0,
        total: 0,
    });

    // Finalize trait analysis - detect patterns ONCE after all files processed
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();
    if !quiet_mode {
        eprint!("Resolving trait patterns and method calls...");
        std::io::Write::flush(&mut std::io::stderr()).ok();
    }
    enhanced_builder.finalize_trait_analysis()?;
    if !quiet_mode {
        eprintln!(" done");
    }

    let enhanced_graph = enhanced_builder.build();
    let framework_exclusions = enhanced_graph.framework_patterns.get_exclusions();
    let framework_exclusions_std: HashSet<priority::call_graph::FunctionId> =
        framework_exclusions.into_iter().collect();

    let function_pointer_used_functions = enhanced_graph
        .function_pointer_tracker
        .get_definitely_used_functions();
    let function_pointer_used_std: HashSet<priority::call_graph::FunctionId> =
        function_pointer_used_functions.into_iter().collect();

    call_graph.merge(enhanced_graph.base_graph);
    call_graph.resolve_cross_file_calls();

    Ok((framework_exclusions_std, function_pointer_used_std))
}
