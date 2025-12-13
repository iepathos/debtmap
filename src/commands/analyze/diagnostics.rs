//! Diagnostics module for the analyze command.
//!
//! This module handles debug output, validation display, and statistics.
//! It follows the "Shell" pattern - handling I/O for diagnostic output.

use super::config::AnalyzeConfig;
use crate::analyzers::call_graph::debug::{CallGraphDebugger, DebugConfig, DebugFormat};
use crate::analyzers::call_graph::validation::{
    CallGraphValidator, ValidationReport, ValidationStatistics, ValidationWarning,
};
use crate::cli::DebugFormatArg;
use crate::priority::{CallGraph, UnifiedAnalysis};
use anyhow::Result;

/// Handle call graph debug and validation diagnostics (I/O).
pub fn handle_call_graph(analysis: &UnifiedAnalysis, config: &AnalyzeConfig) -> Result<()> {
    let call_graph = &analysis.call_graph;

    if config.validate_call_graph {
        let report = CallGraphValidator::validate(call_graph);
        print_validation_report(&report, config.verbosity);
    }

    if config.debug_call_graph {
        print_debug_report(config)?;
    }

    if config.call_graph_stats_only {
        print_statistics(call_graph);
    }

    print_coverage_diagnostics_if_enabled();
    Ok(())
}

/// Print validation report to stderr (I/O).
fn print_validation_report(report: &ValidationReport, verbosity: u8) {
    eprintln!("\n=== Call Graph Validation Report ===");
    eprintln!("Health Score: {}/100", report.health_score);
    print_validation_statistics(&report.statistics);
    print_validation_issues(report, verbosity);
}

/// Print validation statistics section.
fn print_validation_statistics(stats: &ValidationStatistics) {
    eprintln!("\nStatistics:");
    eprintln!("  Total Functions: {}", stats.total_functions);
    eprintln!("  Entry Points: {}", stats.entry_points);
    eprintln!(
        "  Leaf Functions: {} (has callers, no callees)",
        stats.leaf_functions
    );
    eprintln!(
        "  Unreachable: {} (no callers, has callees)",
        stats.unreachable_functions
    );
    eprintln!(
        "  Isolated: {} (no callers, no callees)",
        stats.isolated_functions
    );
    if stats.recursive_functions > 0 {
        eprintln!("  Recursive: {}", stats.recursive_functions);
    }
}

/// Print validation issues and warnings.
fn print_validation_issues(report: &ValidationReport, verbosity: u8) {
    eprintln!("\nStructural Issues: {}", report.structural_issues.len());
    eprintln!("Warnings: {}", report.warnings.len());

    if !report.structural_issues.is_empty() {
        eprintln!("\nStructural Issues:");
        for issue in &report.structural_issues {
            eprintln!("  - {:?}", issue);
        }
    }

    if !report.warnings.is_empty() && verbosity > 0 {
        print_warnings(&report.warnings);
    }
}

/// Print warnings with limit.
fn print_warnings(warnings: &[ValidationWarning]) {
    const MAX_WARNINGS: usize = 10;
    eprintln!("\nWarnings:");
    for warning in warnings.iter().take(MAX_WARNINGS) {
        eprintln!("  - {:?}", warning);
    }
    if warnings.len() > MAX_WARNINGS {
        eprintln!("  ... and {} more warnings", warnings.len() - MAX_WARNINGS);
    }
}

/// Print debug report (I/O).
fn print_debug_report(config: &AnalyzeConfig) -> Result<()> {
    let format = match config.debug_format {
        DebugFormatArg::Text => DebugFormat::Text,
        DebugFormatArg::Json => DebugFormat::Json,
    };

    let debug_config = build_debug_config(config, format);
    let mut debugger = CallGraphDebugger::new(debug_config);

    add_trace_functions(&mut debugger, &config.trace_functions);
    debugger.finalize_statistics();

    eprintln!("\n=== Call Graph Debug Report ===");
    let mut stdout = std::io::stdout();
    debugger.write_report(&mut stdout)?;
    Ok(())
}

/// Build debug configuration.
fn build_debug_config(config: &AnalyzeConfig, format: DebugFormat) -> DebugConfig {
    DebugConfig {
        show_successes: config.verbosity > 1,
        show_timing: true,
        max_candidates_shown: 5,
        format,
        filter_functions: config
            .trace_functions
            .as_ref()
            .map(|funcs| funcs.iter().cloned().collect()),
    }
}

/// Add trace functions to debugger.
fn add_trace_functions(debugger: &mut CallGraphDebugger, trace_functions: &Option<Vec<String>>) {
    if let Some(ref funcs) = trace_functions {
        for func in funcs {
            debugger.add_trace_function(func.clone());
        }
    }
}

/// Print call graph statistics (I/O).
fn print_statistics(call_graph: &CallGraph) {
    eprintln!("\n=== Call Graph Statistics ===");
    eprintln!("Total Functions: {}", call_graph.node_count());

    let total_calls = calculate_total_calls(call_graph);
    eprintln!("Total Calls: {}", total_calls);

    let avg_calls = calculate_avg_calls(call_graph.node_count(), total_calls);
    eprintln!("Average Calls per Function: {:.2}", avg_calls);
}

/// Calculate total number of calls in call graph (pure).
fn calculate_total_calls(call_graph: &CallGraph) -> usize {
    call_graph
        .get_all_functions()
        .map(|func| call_graph.get_callees(func).len())
        .sum()
}

/// Calculate average calls per function (pure).
fn calculate_avg_calls(node_count: usize, total_calls: usize) -> f64 {
    if node_count > 0 {
        total_calls as f64 / node_count as f64
    } else {
        0.0
    }
}

/// Print coverage matching statistics if diagnostic mode enabled.
fn print_coverage_diagnostics_if_enabled() {
    if std::env::var("DEBTMAP_COVERAGE_DEBUG").is_ok() {
        crate::risk::lcov::print_coverage_statistics();
    }
}
