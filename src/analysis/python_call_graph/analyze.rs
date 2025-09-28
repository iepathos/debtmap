use super::cross_module::{CrossModuleAnalyzer, CrossModuleContext};
use super::import_tracker::ImportTracker;
use crate::analysis::python_type_tracker::TwoPassExtractor;
use crate::priority::call_graph::CallGraph;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Analyze a Python project with cross-module call graph resolution
pub fn analyze_python_project(files: &[PathBuf]) -> Result<CallGraph> {
    // Phase 1: Build cross-module context
    let context = build_cross_module_context(files)?;

    // Phase 2: Analyze each file with the context
    let call_graphs: Vec<CallGraph> = files
        .par_iter()
        .map(|file| analyze_file_with_context(file, &context))
        .collect::<Result<Vec<_>>>()?;

    // Phase 3: Merge all call graphs
    Ok(context.merge_call_graphs(call_graphs))
}

/// Build cross-module context from Python files
pub fn build_cross_module_context(files: &[PathBuf]) -> Result<CrossModuleContext> {
    let mut analyzer = CrossModuleAnalyzer::new();

    for file in files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        analyzer
            .analyze_file(file, &content)
            .with_context(|| format!("Failed to analyze file: {}", file.display()))?;
    }

    Ok(analyzer.take_context())
}

/// Analyze a single file with cross-module context
fn analyze_file_with_context(file: &Path, context: &CrossModuleContext) -> Result<CallGraph> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let module = rustpython_parser::parse(
        &content,
        rustpython_parser::Mode::Module,
        file.to_str().unwrap_or("unknown"),
    )
    .with_context(|| format!("Failed to parse Python file: {}", file.display()))?;

    // Use TwoPassExtractor with cross-module context
    let mut extractor =
        TwoPassExtractor::new_with_context(file.to_path_buf(), &content, context.clone());

    Ok(extractor.extract(&module))
}

/// Analyze Python imports in a file
pub fn analyze_imports(file: &Path) -> Result<ImportTracker> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let module = rustpython_parser::parse(
        &content,
        rustpython_parser::Mode::Module,
        file.to_str().unwrap_or("unknown"),
    )
    .with_context(|| format!("Failed to parse Python file: {}", file.display()))?;

    let mut tracker = ImportTracker::new(file.to_path_buf());

    if let rustpython_parser::ast::Mod::Module(module) = module {
        for stmt in &module.body {
            match stmt {
                rustpython_parser::ast::Stmt::Import(import) => {
                    tracker.track_import(import);
                }
                rustpython_parser::ast::Stmt::ImportFrom(import_from) => {
                    tracker.track_import_from(import_from);
                }
                _ => {}
            }
        }
    }

    Ok(tracker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_cross_module_analysis() -> Result<()> {
        // Create temporary files for testing
        let mut file1 = NamedTempFile::new()?;
        let mut file2 = NamedTempFile::new()?;

        // Module 1: defines a class
        writeln!(
            file1,
            r#"
class Manager:
    def process(self, item):
        return item * 2
"#
        )?;

        // Module 2: uses the class from module 1
        writeln!(
            file2,
            r#"
from manager import Manager

def main():
    mgr = Manager()
    result = mgr.process(5)
    print(result)
"#
        )?;

        let files = vec![file1.path().to_path_buf(), file2.path().to_path_buf()];

        let call_graph = analyze_python_project(&files)?;

        // Verify that cross-module calls are detected
        assert!(call_graph.get_all_functions().count() > 0);

        Ok(())
    }
}
