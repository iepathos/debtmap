use crate::analyzers::nix::debt::detect_complexity_debt;
use crate::analyzers::nix::dependencies::extract_dependencies;
use crate::analyzers::nix::generated::is_generated_nix;
use crate::analyzers::nix::metrics::{build_file_metrics, to_function_metrics};
use crate::analyzers::nix::visitor::analyze_ast;
use crate::config::GeneratedCodeMode;
use crate::core::FileMetrics;
use crate::core::ast::NixAst;

pub fn analyze_nix_file(ast: &NixAst, threshold: u32) -> FileMetrics {
    analyze_nix_file_with_mode(ast, threshold, GeneratedCodeMode::SuppressDebt)
}

pub fn analyze_nix_file_with_mode(
    ast: &NixAst,
    threshold: u32,
    generated_mode: GeneratedCodeMode,
) -> FileMetrics {
    let analysis = analyze_ast(ast);
    let functions: Vec<_> = analysis.functions.iter().map(to_function_metrics).collect();
    let debt_items = if suppress_generated_debt(ast, generated_mode) {
        Vec::new()
    } else {
        detect_complexity_debt(&ast.path, threshold, &functions)
    };

    build_file_metrics(
        ast.path.clone(),
        functions,
        debt_items,
        extract_dependencies(ast),
        ast.source.lines().count(),
    )
}

fn suppress_generated_debt(ast: &NixAst, generated_mode: GeneratedCodeMode) -> bool {
    generated_mode == GeneratedCodeMode::SuppressDebt && is_generated_nix(&ast.path, &ast.source)
}
