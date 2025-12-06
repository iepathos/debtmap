//! Completion reporting for batch analysis operations.

use super::collection::AnalysisResults;
use super::summary::ErrorSummary;

/// Reports completion summary for batch analysis.
pub fn report_completion_summary<T>(results: &AnalysisResults<T>) {
    eprintln!("\nAnalysis Summary:");
    eprintln!("  Total files processed: {}", results.total_count());
    eprintln!("  Successfully analyzed: {}", results.success_count());
    eprintln!("  Failed to analyze: {}", results.failure_count());
    eprintln!("  Success rate: {:.1}%", results.success_rate() * 100.0);

    if !results.failures.is_empty() {
        let summary = ErrorSummary::from_failures(&results.failures);
        eprintln!("{}", summary.report());
    }
}

/// Reports brief summary (just counts).
pub fn report_brief_summary<T>(results: &AnalysisResults<T>) {
    if results.is_complete_success() {
        eprintln!("✓ Successfully analyzed {} files", results.success_count());
    } else {
        eprintln!(
            "⚠ Analyzed {} files ({} failed)",
            results.success_count(),
            results.failure_count()
        );
    }
}
