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
use std::sync::OnceLock;
use tracing::Span;

/// Captured panic information for retrieval after thread join.
///
/// When a thread panics, the panic hook stores the details here so that
/// `get_last_panic_info()` can retrieve a meaningful error message including
/// the source location, not just the panic message.
#[derive(Clone, Debug)]
pub struct CapturedPanicInfo {
    pub message: String,
    pub location: Option<String>,
    pub file_being_analyzed: Option<String>,
    pub phase: Option<String>,
}

impl std::fmt::Display for CapturedPanicInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(loc) = &self.location {
            write!(f, " at {}", loc)?;
        }
        if let Some(file) = &self.file_being_analyzed {
            write!(f, " (analyzing: {})", file)?;
        }
        if let Some(phase) = &self.phase {
            write!(f, " [phase: {}]", phase)?;
        }
        Ok(())
    }
}

/// Global storage for the last panic info.
/// Using OnceLock for thread-safe one-time initialization, then Mutex for updates.
static LAST_PANIC: OnceLock<std::sync::Mutex<Option<CapturedPanicInfo>>> = OnceLock::new();

fn get_panic_storage() -> &'static std::sync::Mutex<Option<CapturedPanicInfo>> {
    LAST_PANIC.get_or_init(|| std::sync::Mutex::new(None))
}

/// Store panic info for later retrieval.
fn store_panic_info(info: CapturedPanicInfo) {
    if let Ok(mut guard) = get_panic_storage().lock() {
        *guard = Some(info);
    }
}

/// Retrieve the last captured panic info, if any.
///
/// This is useful when handling thread join errors, as it provides
/// the full context captured by the panic hook.
pub fn get_last_panic_info() -> Option<CapturedPanicInfo> {
    get_panic_storage().lock().ok().and_then(|g| g.clone())
}

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

    // Capture panic info for later retrieval (spec 210: informative panic messages)
    let message = extract_panic_message(info);
    let location = info
        .location()
        .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()));
    let file_being_analyzed = context
        .current_file
        .as_ref()
        .map(|p| p.display().to_string());
    let phase = context.phase.as_ref().map(|p| p.to_string());

    store_panic_info(CapturedPanicInfo {
        message: message.clone(),
        location: location.clone(),
        file_being_analyzed,
        phase,
    });

    eprintln!();
    print_header();
    print_panic_details_with_message(&message, location.as_deref());
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

fn print_panic_details_with_message(message: &str, location: Option<&str>) {
    eprintln!("║  PANIC: {:<68} ║", truncate(message, 68));

    if let Some(loc_str) = location {
        eprintln!("║  Location: {:<66} ║", truncate(loc_str, 66));
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

/// Extract a meaningful message from a thread panic payload.
///
/// When `thread.join()` returns `Err(Box<dyn Any + Send>)`, the `Any`
/// type doesn't have a useful Debug implementation. This function:
/// 1. Checks if the panic hook captured detailed info (including location)
/// 2. Falls back to downcasting the payload for the message
///
/// The result includes the panic location and context if available.
///
/// # Example
///
/// ```ignore
/// use debtmap::observability::extract_thread_panic_message;
///
/// let handle = std::thread::spawn(|| {
///     panic!("something went wrong");
/// });
///
/// match handle.join() {
///     Ok(_) => println!("Thread completed"),
///     Err(payload) => {
///         let message = extract_thread_panic_message(&payload);
///         // Message includes location: "something went wrong at src/foo.rs:42:5"
///         eprintln!("Thread panic: {}", message);
///     }
/// }
/// ```
pub fn extract_thread_panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    // First, check if the panic hook captured detailed info with location
    if let Some(captured) = get_last_panic_info() {
        return captured.to_string();
    }

    // Fallback: extract message from payload (no location available)
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic (payload is not a string)".to_string()
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

    #[test]
    fn test_extract_thread_panic_message_from_str() {
        // Simulate a panic with &str payload
        let handle = std::thread::spawn(|| {
            panic!("test panic message");
        });

        let err = handle.join().expect_err("Should have panicked");
        let message = extract_thread_panic_message(&err);
        assert_eq!(message, "test panic message");
    }

    #[test]
    fn test_extract_thread_panic_message_from_string() {
        // Simulate a panic with String payload (from format!)
        let handle = std::thread::spawn(|| {
            panic!("{}", format!("formatted panic: {}", 42));
        });

        let err = handle.join().expect_err("Should have panicked");
        let message = extract_thread_panic_message(&err);
        assert_eq!(message, "formatted panic: 42");
    }

    #[test]
    fn test_extract_thread_panic_message_from_expect() {
        // Simulate a panic from .expect()
        let handle = std::thread::spawn(|| {
            #[allow(clippy::unnecessary_literal_unwrap)]
            None::<i32>.expect("expected value was missing");
        });

        let err = handle.join().expect_err("Should have panicked");
        let message = extract_thread_panic_message(&err);
        assert!(
            message.contains("expected value was missing"),
            "Expected message to contain 'expected value was missing', got: {}",
            message
        );
    }
}
