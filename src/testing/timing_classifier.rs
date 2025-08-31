// Demonstration of a function that needs refactoring based on debt analysis
// This function has cyclomatic complexity of 11, matching the debt item

/// Classifies timing-related function calls using pattern consolidation
/// Refactored to reduce complexity from 11 to <10 using match with guards
pub fn classify_timing_operation(path: &str, method: &str) -> TimingCategory {
    // Use pattern matching with guards for cleaner classification
    match () {
        _ if path.contains("Instant") && method == "now" => TimingCategory::CurrentTime,
        _ if path.contains("SystemTime") && method == "now" => TimingCategory::SystemTime,
        _ if path.contains("Duration") && method.starts_with("from_") => {
            TimingCategory::DurationCreation
        }
        _ if matches!(method, "elapsed" | "duration_since") => TimingCategory::ElapsedTime,
        _ if contains_pattern(path, method, "sleep") => TimingCategory::Sleep,
        _ if matches!(method, "park_timeout" | "recv_timeout") => TimingCategory::ThreadTimeout,
        _ if contains_pattern(path, method, "timeout") => TimingCategory::Timeout,
        _ if method.contains("wait") && !method.contains("await") => TimingCategory::Wait,
        _ if contains_pattern(path, method, "delay") => TimingCategory::Delay,
        _ if contains_pattern(path, method, "timer") => TimingCategory::Timer,
        _ => TimingCategory::Unknown,
    }
}

/// Helper function to check if pattern exists in either path or method
#[inline]
fn contains_pattern(path: &str, method: &str, pattern: &str) -> bool {
    path.contains(pattern) || method.contains(pattern)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingCategory {
    CurrentTime,
    SystemTime,
    DurationCreation,
    ElapsedTime,
    Sleep,
    Timeout,
    Wait,
    ThreadTimeout,
    Delay,
    Timer,
    Unknown,
}

impl TimingCategory {
    pub fn is_flaky(&self) -> bool {
        !matches!(
            self,
            TimingCategory::DurationCreation | TimingCategory::Unknown
        )
    }

    pub fn description(&self) -> &'static str {
        match self {
            TimingCategory::CurrentTime => "Gets current instant time",
            TimingCategory::SystemTime => "Gets system clock time",
            TimingCategory::DurationCreation => "Creates duration value",
            TimingCategory::ElapsedTime => "Measures elapsed time",
            TimingCategory::Sleep => "Thread sleep operation",
            TimingCategory::Timeout => "Operation with timeout",
            TimingCategory::Wait => "Waiting operation",
            TimingCategory::ThreadTimeout => "Thread parking with timeout",
            TimingCategory::Delay => "Delay operation",
            TimingCategory::Timer => "Timer-based operation",
            TimingCategory::Unknown => "Unknown timing operation",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instant_now_classification() {
        assert_eq!(
            classify_timing_operation("std::time::Instant", "now"),
            TimingCategory::CurrentTime
        );
    }

    #[test]
    fn test_system_time_classification() {
        assert_eq!(
            classify_timing_operation("SystemTime", "now"),
            TimingCategory::SystemTime
        );
    }

    #[test]
    fn test_duration_from_classification() {
        assert_eq!(
            classify_timing_operation("Duration", "from_secs"),
            TimingCategory::DurationCreation
        );
        assert_eq!(
            classify_timing_operation("std::time::Duration", "from_millis"),
            TimingCategory::DurationCreation
        );
    }

    #[test]
    fn test_elapsed_time_classification() {
        assert_eq!(
            classify_timing_operation("Instant", "elapsed"),
            TimingCategory::ElapsedTime
        );
        assert_eq!(
            classify_timing_operation("SystemTime", "duration_since"),
            TimingCategory::ElapsedTime
        );
    }

    #[test]
    fn test_sleep_classification() {
        assert_eq!(
            classify_timing_operation("thread", "sleep"),
            TimingCategory::Sleep
        );
        assert_eq!(
            classify_timing_operation("sleep_for", "call"),
            TimingCategory::Sleep
        );
    }

    #[test]
    fn test_timeout_classification() {
        assert_eq!(
            classify_timing_operation("", "timeout"),
            TimingCategory::Timeout
        );
        assert_eq!(
            classify_timing_operation("with_timeout", "run"),
            TimingCategory::Timeout
        );
    }

    #[test]
    fn test_wait_classification() {
        assert_eq!(classify_timing_operation("", "wait"), TimingCategory::Wait);
        // Should not match 'await'
        assert_ne!(classify_timing_operation("", "await"), TimingCategory::Wait);
    }

    #[test]
    fn test_thread_timeout_classification() {
        assert_eq!(
            classify_timing_operation("", "park_timeout"),
            TimingCategory::ThreadTimeout
        );
        assert_eq!(
            classify_timing_operation("channel", "recv_timeout"),
            TimingCategory::ThreadTimeout
        );
    }

    #[test]
    fn test_delay_classification() {
        assert_eq!(
            classify_timing_operation("delay_for", "run"),
            TimingCategory::Delay
        );
        assert_eq!(
            classify_timing_operation("", "delay"),
            TimingCategory::Delay
        );
    }

    #[test]
    fn test_timer_classification() {
        assert_eq!(
            classify_timing_operation("timer", "start"),
            TimingCategory::Timer
        );
        assert_eq!(
            classify_timing_operation("", "set_timer"),
            TimingCategory::Timer
        );
    }

    #[test]
    fn test_unknown_classification() {
        assert_eq!(
            classify_timing_operation("something", "else"),
            TimingCategory::Unknown
        );
    }

    #[test]
    fn test_flaky_detection() {
        assert!(TimingCategory::CurrentTime.is_flaky());
        assert!(TimingCategory::Sleep.is_flaky());
        assert!(!TimingCategory::DurationCreation.is_flaky());
        assert!(!TimingCategory::Unknown.is_flaky());
    }
}
