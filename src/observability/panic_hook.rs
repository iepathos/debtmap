//! Custom panic hook for structured crash reports.
//!
//! Following the Stillwater principle: "Errors Should Tell Stories".
//! When debtmap crashes, we want to provide actionable information:
//!
//! - What was being analyzed (file, phase, function)
//! - How far through analysis we got
//! - The actual error message and location
//! - A stack trace for debugging
//!
//! ## TUI Compatibility
//!
//! The panic hook automatically exits TUI mode (alternate screen, raw mode)
//! before printing the crash report, ensuring it's visible to users.

use super::context::{get_current_context, get_progress, AnalysisContext};
use super::tracing::set_tui_active;
use std::panic::PanicHookInfo;
use tracing::Span;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ISSUE_URL: &str = "https://github.com/iepathos/debtmap/issues/new";

/// Install the custom panic hook.
///
/// This should be called early in main() before any analysis begins.
/// The hook captures context from the observability module and produces
/// a structured crash report.
///
/// # Example
///
/// ```ignore
/// use debtmap::observability::install_panic_hook;
///
/// fn main() {
///     install_panic_hook();
///     // ... rest of application
/// }
/// ```
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        print_crash_report(info);
    }));
}

fn print_crash_report(info: &PanicHookInfo<'_>) {
    // Exit TUI mode first so crash report is visible
    exit_tui_mode();

    let context = get_current_context();
    let (processed, total) = get_progress();

    eprintln!();
    print_header();
    print_panic_details(info);
    print_context_section(&context, processed, total);
    print_backtrace_section();
    print_footer(&context);
}

fn exit_tui_mode() {
    // Mark TUI as inactive so subsequent logging works
    set_tui_active(false);

    // Ignore errors - we're already panicking
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen);
}

fn print_header() {
    let platform = std::env::consts::OS;
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    eprintln!("╔══════════════════════════════════════════════════════════════════════════════╗");
    eprintln!("║                           DEBTMAP CRASH REPORT                               ║");
    eprintln!("╠══════════════════════════════════════════════════════════════════════════════╣");
    eprintln!("║  Version: {:<67} ║", VERSION);
    eprintln!("║  Platform: {:<66} ║", platform);
    eprintln!("║  Time: {:<70} ║", timestamp);
    eprintln!("╠══════════════════════════════════════════════════════════════════════════════╣");
}

fn print_panic_details(info: &PanicHookInfo<'_>) {
    let message = extract_panic_message(info);
    eprintln!("║  PANIC: {:<68} ║", truncate(&message, 68));

    if let Some(location) = info.location() {
        let loc_str = format!(
            "{}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
        eprintln!("║  Location: {:<66} ║", truncate(&loc_str, 66));
    }
}

fn print_context_section(context: &AnalysisContext, processed: usize, total: usize) {
    eprintln!("╠══════════════════════════════════════════════════════════════════════════════╣");
    eprintln!("║  OPERATION CONTEXT:                                                          ║");

    match &context.phase {
        Some(phase) => {
            eprintln!("║    Phase: {:<66} ║", phase);
        }
        None => {
            eprintln!(
                "║    Phase: (not set - crash occurred before analysis started)                 ║"
            );
        }
    }

    // Print current tracing span context (spec 208)
    let current_span = Span::current();
    if let Some(metadata) = current_span.metadata() {
        eprintln!("║    Span: {:<67} ║", truncate(metadata.name(), 67));
    }

    if let Some(file) = &context.current_file {
        let file_str = file.display().to_string();
        eprintln!("║    File: {:<67} ║", truncate(&file_str, 67));
    }

    if let Some(func) = &context.current_function {
        eprintln!("║    Function: {:<63} ║", truncate(func, 63));
    }

    if total > 0 {
        let pct = (processed as f64 / total as f64 * 100.0) as usize;
        let progress_str = format!("{} / {} files ({}%)", processed, total, pct);
        eprintln!("║    Progress: {:<63} ║", progress_str);
    }
}

fn print_backtrace_section() {
    eprintln!("╠══════════════════════════════════════════════════════════════════════════════╣");

    if std::env::var("RUST_BACKTRACE").is_ok() {
        eprintln!(
            "║  STACK TRACE:                                                                ║"
        );
        eprintln!(
            "╚══════════════════════════════════════════════════════════════════════════════╝"
        );
        eprintln!();
        eprintln!("{}", std::backtrace::Backtrace::capture());
    } else {
        eprintln!(
            "║  Run with RUST_BACKTRACE=1 for stack trace                                   ║"
        );
        eprintln!(
            "╚══════════════════════════════════════════════════════════════════════════════╝"
        );
    }
}

fn print_footer(context: &AnalysisContext) {
    eprintln!();
    eprintln!("════════════════════════════════════════════════════════════════════════════════");
    eprintln!("To report this issue: {}", ISSUE_URL);
    if let Some(file) = &context.current_file {
        eprintln!("Include this crash report and the file: {}", file.display());
    }
    eprintln!("════════════════════════════════════════════════════════════════════════════════");
}

fn extract_panic_message(info: &PanicHookInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("short", 10), "short");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("exactly10!", 10), "exactly10!");
    }

    #[test]
    fn test_truncate_long_string() {
        let result = truncate("this is a long string that needs truncation", 20);
        assert_eq!(result.len(), 20);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_very_short_max() {
        let result = truncate("hello", 3);
        assert_eq!(result, "...");
    }
}
