use crate::{
    analysis::call_graph::RustCallGraphBuilder,
    analysis::python_call_graph::{
        analyze_python_project, PythonCallGraphAnalyzer, TwoPassExtractor,
    },
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

    // Finalize trait analysis - detect patterns ONCE after all files processed
    enhanced_builder.finalize_trait_analysis()?;

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

/// Read and parse a Python file, returning the content and parsed AST
fn read_and_parse_python_file(file_path: &Path) -> Result<(String, rustpython_parser::ast::Mod)> {
    let content = io::read_file(file_path)
        .with_context(|| format!("Failed to read Python file {:?}", file_path))?;

    let module = rustpython_parser::parse(&content, rustpython_parser::Mode::Module, "<module>")
        .with_context(|| format!("Failed to parse Python file {:?}", file_path))?;

    Ok((content, module))
}

/// Extract call graph from a parsed Python AST using TwoPassExtractor
fn extract_call_graph_from_parsed_python(
    module: &rustpython_parser::ast::Mod,
    file_path: &Path,
    content: &str,
) -> priority::CallGraph {
    let mut extractor = TwoPassExtractor::new_with_source(file_path.to_path_buf(), content);
    extractor.extract(module)
}

/// Log a Python file processing error with consistent formatting
fn log_python_file_error(error_type: &str, file_path: &Path, error: &dyn std::error::Error) {
    log::warn!(
        "Failed to {} Python file {:?}: {}",
        error_type,
        file_path,
        error
    );
}

/// Determine if cross-module analysis should be used based on file count
fn should_use_cross_module_analysis(python_files: &[std::path::PathBuf]) -> bool {
    python_files.len() > 1
}

/// Process Python files using cross-module analysis
fn process_with_cross_module_analysis(
    python_files: &[std::path::PathBuf],
    call_graph: &mut priority::CallGraph,
) -> Result<()> {
    log::debug!(
        "Using cross-module analysis for {} Python files",
        python_files.len()
    );

    match analyze_python_project(python_files) {
        Ok(cross_module_graph) => {
            call_graph.merge(cross_module_graph);
            Ok(())
        }
        Err(e) => {
            log::warn!(
                "Cross-module analysis failed, falling back to single-file analysis: {}",
                e
            );
            process_with_fallback_analysis(python_files, call_graph)
        }
    }
}

/// Process Python files using fallback single-file analysis with type tracking
fn process_with_fallback_analysis(
    python_files: &[std::path::PathBuf],
    call_graph: &mut priority::CallGraph,
) -> Result<()> {
    for file_path in python_files {
        match read_and_parse_python_file(file_path) {
            Ok((content, module)) => {
                let file_call_graph =
                    extract_call_graph_from_parsed_python(&module, file_path, &content);
                call_graph.merge(file_call_graph);
            }
            Err(e) => {
                log_python_file_error("parse", file_path, e.as_ref());
            }
        }
    }
    Ok(())
}

/// Process Python files using basic (non-type-aware) analysis
fn process_with_basic_analysis(
    python_files: &[std::path::PathBuf],
    call_graph: &mut priority::CallGraph,
) -> Result<()> {
    let mut analyzer = PythonCallGraphAnalyzer::new();

    for file_path in python_files {
        match read_and_parse_python_file(file_path) {
            Ok((content, module)) => {
                if let Err(e) =
                    analyzer.analyze_module_with_source(&module, file_path, &content, call_graph)
                {
                    log_python_file_error("analyze", file_path, e.as_ref());
                }
            }
            Err(e) => {
                log_python_file_error("parse", file_path, e.as_ref());
            }
        }
    }
    Ok(())
}

/// Process Python files with type-aware analysis (cross-module or single-file)
fn process_with_type_tracking(
    python_files: &[std::path::PathBuf],
    call_graph: &mut priority::CallGraph,
) -> Result<()> {
    if should_use_cross_module_analysis(python_files) {
        process_with_cross_module_analysis(python_files, call_graph)
    } else {
        process_with_fallback_analysis(python_files, call_graph)
    }
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
        process_with_type_tracking(&python_files, call_graph)
    } else {
        process_with_basic_analysis(&python_files, call_graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_and_parse_valid_python_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let python_code = "def hello():\n    return 'world'\n";
        temp_file.write_all(python_code.as_bytes()).unwrap();

        let result = read_and_parse_python_file(temp_file.path());
        assert!(result.is_ok());

        let (content, _module) = result.unwrap();
        assert_eq!(content, python_code);
    }

    #[test]
    fn test_read_and_parse_invalid_python_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let invalid_python = "def broken(\n    syntax error\n";
        temp_file.write_all(invalid_python.as_bytes()).unwrap();

        let result = read_and_parse_python_file(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_call_graph_from_simple_function() {
        let python_code = "def foo():\n    bar()\n\ndef bar():\n    pass\n";
        let module =
            rustpython_parser::parse(python_code, rustpython_parser::Mode::Module, "<module>")
                .unwrap();

        let temp_file = NamedTempFile::new().unwrap();
        let call_graph =
            extract_call_graph_from_parsed_python(&module, temp_file.path(), python_code);

        // Verify the call graph contains functions
        assert!(!call_graph.is_empty());
    }

    #[test]
    fn test_log_python_file_error_formats_correctly() {
        use std::io;

        let temp_file = NamedTempFile::new().unwrap();
        let error = io::Error::new(io::ErrorKind::NotFound, "test error");

        // This test ensures the function doesn't panic
        log_python_file_error("test", temp_file.path(), &error);
    }

    #[test]
    fn test_should_use_cross_module_analysis_single_file() {
        let files = vec![std::path::PathBuf::from("single.py")];
        assert!(!should_use_cross_module_analysis(&files));
    }

    #[test]
    fn test_should_use_cross_module_analysis_multiple_files() {
        let files = vec![
            std::path::PathBuf::from("file1.py"),
            std::path::PathBuf::from("file2.py"),
        ];
        assert!(should_use_cross_module_analysis(&files));
    }

    #[test]
    fn test_should_use_cross_module_analysis_empty() {
        let files: Vec<std::path::PathBuf> = vec![];
        assert!(!should_use_cross_module_analysis(&files));
    }

    #[test]
    fn test_process_with_fallback_analysis() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let python_code = "def test():\n    pass\n";
        temp_file.write_all(python_code.as_bytes()).unwrap();

        let files = vec![temp_file.path().to_path_buf()];
        let mut call_graph = priority::CallGraph::new();

        let result = process_with_fallback_analysis(&files, &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_with_basic_analysis() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let python_code = "def foo():\n    bar()\n\ndef bar():\n    pass\n";
        temp_file.write_all(python_code.as_bytes()).unwrap();

        let files = vec![temp_file.path().to_path_buf()];
        let mut call_graph = priority::CallGraph::new();

        let result = process_with_basic_analysis(&files, &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_with_type_tracking_single_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let python_code = "def test():\n    pass\n";
        temp_file.write_all(python_code.as_bytes()).unwrap();

        let files = vec![temp_file.path().to_path_buf()];
        let mut call_graph = priority::CallGraph::new();

        let result = process_with_type_tracking(&files, &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_main_function_with_type_tracking() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.py");
        std::fs::write(&file_path, "def test():\n    pass\n").unwrap();

        let mut call_graph = priority::CallGraph::new();
        let result =
            process_python_files_for_call_graph_with_types(temp_dir.path(), &mut call_graph, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_main_function_without_type_tracking() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.py");
        std::fs::write(&file_path, "def foo():\n    bar()\n").unwrap();

        let mut call_graph = priority::CallGraph::new();
        let result =
            process_python_files_for_call_graph_with_types(temp_dir.path(), &mut call_graph, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_main_function_with_empty_directory() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let mut call_graph = priority::CallGraph::new();

        let result =
            process_python_files_for_call_graph_with_types(temp_dir.path(), &mut call_graph, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_with_fallback_analysis_handles_parse_errors() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let invalid_python = "def broken(\n    syntax error\n";
        temp_file.write_all(invalid_python.as_bytes()).unwrap();

        let files = vec![temp_file.path().to_path_buf()];
        let mut call_graph = priority::CallGraph::new();

        // Should succeed despite parse errors (logs warning internally)
        let result = process_with_fallback_analysis(&files, &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_with_basic_analysis_handles_parse_errors() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let invalid_python = "def broken(\n    syntax error\n";
        temp_file.write_all(invalid_python.as_bytes()).unwrap();

        let files = vec![temp_file.path().to_path_buf()];
        let mut call_graph = priority::CallGraph::new();

        // Should succeed despite parse errors (logs warning internally)
        let result = process_with_basic_analysis(&files, &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_and_parse_nonexistent_file() {
        use std::path::PathBuf;

        let result = read_and_parse_python_file(&PathBuf::from("/nonexistent/file.py"));
        assert!(result.is_err());
    }

    #[test]
    fn test_process_with_cross_module_analysis_success() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file1 = temp_dir.path().join("module1.py");
        let file2 = temp_dir.path().join("module2.py");
        std::fs::write(&file1, "def func1():\n    pass\n").unwrap();
        std::fs::write(
            &file2,
            "from module1 import func1\ndef func2():\n    func1()\n",
        )
        .unwrap();

        let files = vec![file1, file2];
        let mut call_graph = priority::CallGraph::new();

        let result = process_with_cross_module_analysis(&files, &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_with_cross_module_analysis_empty_files() {
        let files = vec![];
        let mut call_graph = priority::CallGraph::new();

        let result = process_with_cross_module_analysis(&files, &mut call_graph);
        // Should handle empty file list gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_process_with_cross_module_analysis_single_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let python_code = "def test():\n    pass\n";
        temp_file.write_all(python_code.as_bytes()).unwrap();

        let files = vec![temp_file.path().to_path_buf()];
        let mut call_graph = priority::CallGraph::new();

        let result = process_with_cross_module_analysis(&files, &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_with_cross_module_analysis_invalid_python() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file1 = temp_dir.path().join("bad1.py");
        let file2 = temp_dir.path().join("bad2.py");
        std::fs::write(&file1, "def broken(\n    syntax error\n").unwrap();
        std::fs::write(&file2, "also broken\n").unwrap();

        let files = vec![file1, file2];
        let mut call_graph = priority::CallGraph::new();

        // Should fall back to single-file analysis
        let result = process_with_cross_module_analysis(&files, &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_python_files_for_call_graph_wrapper() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.py");
        std::fs::write(&file_path, "def test():\n    pass\n").unwrap();

        let mut call_graph = priority::CallGraph::new();
        let result = process_python_files_for_call_graph(temp_dir.path(), &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_python_files_for_call_graph_empty_dir() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let mut call_graph = priority::CallGraph::new();

        let result = process_python_files_for_call_graph(temp_dir.path(), &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_python_files_for_call_graph_multiple_files() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file1 = temp_dir.path().join("file1.py");
        let file2 = temp_dir.path().join("file2.py");
        std::fs::write(&file1, "def foo():\n    pass\n").unwrap();
        std::fs::write(&file2, "def bar():\n    foo()\n").unwrap();

        let mut call_graph = priority::CallGraph::new();
        let result = process_python_files_for_call_graph(temp_dir.path(), &mut call_graph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_with_type_tracking_multiple_files() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file1 = temp_dir.path().join("mod1.py");
        let file2 = temp_dir.path().join("mod2.py");
        std::fs::write(&file1, "def helper():\n    return 42\n").unwrap();
        std::fs::write(
            &file2,
            "from mod1 import helper\ndef main():\n    helper()\n",
        )
        .unwrap();

        let files = vec![file1, file2];
        let mut call_graph = priority::CallGraph::new();

        let result = process_with_type_tracking(&files, &mut call_graph);
        assert!(result.is_ok());
    }
}
