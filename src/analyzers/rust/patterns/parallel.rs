//! Parallel execution pattern detection
//!
//! Detects patterns like rayon, tokio parallel execution.

use crate::organization::parallel_execution_pattern::{
    adjust_parallel_score, ParallelPatternDetector,
};

/// Detect parallel execution patterns and update complexity (spec 127)
pub fn detect_parallel_patterns(
    file_ast: Option<&syn::File>,
    source_content: &str,
    cyclomatic: u32,
    current_adjusted: Option<f64>,
) -> (Vec<String>, Option<f64>) {
    let Some(ast) = file_ast else {
        return (Vec::new(), current_adjusted);
    };

    let detector = ParallelPatternDetector::default();
    let Some(mut pattern) = detector.detect(ast, source_content) else {
        return (Vec::new(), current_adjusted);
    };

    pattern.cyclomatic_complexity = cyclomatic as usize;
    let confidence = detector.confidence(&pattern);
    let parallel_adjusted = adjust_parallel_score(cyclomatic as f64, &pattern);

    let best_adjusted = match current_adjusted {
        Some(curr) if curr <= parallel_adjusted => Some(curr),
        _ => Some(parallel_adjusted),
    };

    let pattern_desc = format!(
        "ParallelExecution({}, {:.0}% confidence, {} closures, {} captures)",
        pattern.library,
        confidence * 100.0,
        pattern.closure_count,
        pattern.total_captures
    );

    (vec![pattern_desc], best_adjusted)
}
