use crate::analyzers::go::debt::detect_complexity_debt;
use crate::analyzers::go::dependencies::extract_dependencies;
use crate::analyzers::go::generated::is_generated_go;
use crate::analyzers::go::metrics::{build_file_metrics, to_function_metrics};
use crate::analyzers::go::visitor::analyze_ast;
use crate::core::FileMetrics;
use crate::core::ast::GoAst;

pub fn analyze_go_file(ast: &GoAst, threshold: u32) -> FileMetrics {
    let analysis = analyze_ast(ast);
    let functions: Vec<_> = analysis.functions.iter().map(to_function_metrics).collect();
    let debt_items = if is_generated_go(&ast.path, &ast.source) {
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
