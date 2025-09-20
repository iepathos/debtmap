use crate::{
    analysis::call_graph::RustCallGraphBuilder,
    analysis::python_call_graph::{PythonCallGraphAnalyzer, TwoPassExtractor},
    analyzers::rust_call_graph::extract_call_graph_multi_file,
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
        let func_id = priority::call_graph::FunctionId {
            file: metric.file.clone(),
            name: metric.name.clone(),
            line: metric.line,
        };

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

pub fn process_rust_files_for_call_graph(
    project_path: &Path,
    call_graph: &mut priority::CallGraph,
    _verbose_macro_warnings: bool,
    _show_macro_stats: bool,
) -> Result<(
    HashSet<priority::call_graph::FunctionId>,
    HashSet<priority::call_graph::FunctionId>,
)> {
    let config = config::get_config();
    let rust_files =
        io::walker::find_project_files_with_config(project_path, vec![Language::Rust], config)
            .context("Failed to find Rust files for call graph")?;

    let mut enhanced_builder = RustCallGraphBuilder::from_base_graph(call_graph.clone());
    let mut workspace_files = Vec::new();
    let mut expanded_files = Vec::new();

    for file_path in rust_files {
        if let Ok(content) = io::read_file(&file_path) {
            if let Ok(parsed) = syn::parse_file(&content) {
                expanded_files.push((parsed.clone(), file_path.clone()));
                workspace_files.push((file_path.clone(), parsed));
            }
        }
    }

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

pub fn process_python_files_for_call_graph(
    project_path: &Path,
    call_graph: &mut priority::CallGraph,
) -> Result<()> {
    process_python_files_for_call_graph_with_types(project_path, call_graph, true)
}

/// Process Python files with optional two-pass type-aware extraction
pub fn process_python_files_for_call_graph_with_types(
    project_path: &Path,
    call_graph: &mut priority::CallGraph,
    use_type_tracking: bool,
) -> Result<()> {
    let config = config::get_config();
    let python_files =
        io::walker::find_project_files_with_config(project_path, vec![Language::Python], config)
            .context("Failed to find Python files for call graph")?;

    if use_type_tracking {
        // Use two-pass type-aware extraction for better accuracy
        for file_path in &python_files {
            match io::read_file(file_path) {
                Ok(content) => {
                    match rustpython_parser::parse(
                        &content,
                        rustpython_parser::Mode::Module,
                        "<module>",
                    ) {
                        Ok(module) => {
                            let mut extractor = TwoPassExtractor::new(file_path.to_path_buf());
                            let file_call_graph = extractor.extract(&module);
                            call_graph.merge(file_call_graph);
                        }
                        Err(e) => {
                            log::warn!("Failed to parse Python file {:?}: {}", file_path, e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read Python file {:?}: {}", file_path, e);
                }
            }
        }
    } else {
        // Fall back to original implementation
        let mut analyzer = PythonCallGraphAnalyzer::new();

        for file_path in &python_files {
            match io::read_file(file_path) {
                Ok(content) => {
                    match rustpython_parser::parse(
                        &content,
                        rustpython_parser::Mode::Module,
                        "<module>",
                    ) {
                        Ok(module) => {
                            if let Err(e) = analyzer.analyze_module_with_source(
                                &module, file_path, &content, call_graph,
                            ) {
                                log::warn!("Failed to analyze Python file {:?}: {}", file_path, e);
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to parse Python file {:?}: {}", file_path, e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read Python file {:?}: {}", file_path, e);
                }
            }
        }
    }

    Ok(())
}
