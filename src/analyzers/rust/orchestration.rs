//! Analysis orchestration
//!
//! Top-level functions for orchestrating Rust file analysis.

use super::debt::create_debt_items;
use super::dependencies::extract_dependencies;
use super::metrics::{build_file_metrics, calculate_total_complexity};
use super::visitor::analyze_ast_with_content;
use crate::complexity::threshold_manager::ComplexityThresholds;
use crate::core::ast::RustAst;
use crate::core::FileMetrics;

/// Analyze a Rust file and return file metrics
pub fn analyze_rust_file(
    ast: &RustAst,
    threshold: u32,
    enhanced_thresholds: &ComplexityThresholds,
    _use_enhanced: bool,
    enable_functional_analysis: bool,
    enable_rust_patterns: bool,
) -> FileMetrics {
    let start = std::time::Instant::now();

    // Capture line count from source during initial analysis to avoid redundant I/O later
    let total_lines = ast.source.lines().count();

    let analysis_start = std::time::Instant::now();
    let analysis_result = analyze_ast_with_content(
        ast,
        &ast.source,
        enhanced_thresholds,
        enable_functional_analysis,
        enable_rust_patterns,
    );
    let analysis_time = analysis_start.elapsed();

    let debt_start = std::time::Instant::now();
    let debt_items = create_debt_items(
        &ast.file,
        &ast.path,
        threshold,
        &analysis_result.functions,
        &ast.source,
        &analysis_result.enhanced_analysis,
    );
    let debt_time = debt_start.elapsed();

    let deps_start = std::time::Instant::now();
    let dependencies = extract_dependencies(&ast.file);
    let deps_time = deps_start.elapsed();

    let complexity_metrics = calculate_total_complexity(&analysis_result.functions);

    let total_time = start.elapsed();

    if std::env::var("DEBTMAP_TIMING").is_ok() {
        eprintln!(
            "[TIMING] analyze_rust_file {}: total={:.2}s (analysis={:.2}s, debt={:.2}s, deps={:.2}s)",
            ast.path.display(),
            total_time.as_secs_f64(),
            analysis_time.as_secs_f64(),
            debt_time.as_secs_f64(),
            deps_time.as_secs_f64()
        );
    }

    build_file_metrics(
        ast.path.clone(),
        analysis_result.functions,
        complexity_metrics,
        debt_items,
        dependencies,
        total_lines,
    )
}
