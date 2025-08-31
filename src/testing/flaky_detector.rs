use super::{
    is_test_function, FlakinessType, ReliabilityImpact, TestQualityImpact, TestingAntiPattern,
    TestingDetector,
};
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, File, Item, ItemFn};

pub struct FlakyTestDetector {}

impl Default for FlakyTestDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FlakyTestDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl TestingDetector for FlakyTestDetector {
    fn detect_anti_patterns(&self, file: &File, path: &Path) -> Vec<TestingAntiPattern> {
        let mut patterns = Vec::new();

        for item in &file.items {
            if let Item::Fn(function) = item {
                if is_test_function(function) {
                    let flakiness_indicators = analyze_flakiness(function);

                    for indicator in flakiness_indicators {
                        let line = function.sig.ident.span().start().line;

                        patterns.push(TestingAntiPattern::FlakyTestPattern {
                            test_name: function.sig.ident.to_string(),
                            file: path.to_path_buf(),
                            line,
                            flakiness_type: indicator.flakiness_type,
                            reliability_impact: indicator.impact,
                            stabilization_suggestion: indicator.suggestion,
                        });
                    }
                }
            }

            // Also check test modules
            if let Item::Mod(module) = item {
                if let Some((_, items)) = &module.content {
                    for mod_item in items {
                        if let Item::Fn(function) = mod_item {
                            if is_test_function(function) {
                                let flakiness_indicators = analyze_flakiness(function);

                                for indicator in flakiness_indicators {
                                    let line = function.sig.ident.span().start().line;

                                    patterns.push(TestingAntiPattern::FlakyTestPattern {
                                        test_name: function.sig.ident.to_string(),
                                        file: path.to_path_buf(),
                                        line,
                                        flakiness_type: indicator.flakiness_type,
                                        reliability_impact: indicator.impact,
                                        stabilization_suggestion: indicator.suggestion,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "FlakyTestDetector"
    }

    fn assess_test_quality_impact(&self, pattern: &TestingAntiPattern) -> TestQualityImpact {
        match pattern {
            TestingAntiPattern::FlakyTestPattern {
                reliability_impact, ..
            } => match reliability_impact {
                ReliabilityImpact::Critical => TestQualityImpact::Critical,
                ReliabilityImpact::High => TestQualityImpact::High,
                ReliabilityImpact::Medium => TestQualityImpact::Medium,
                ReliabilityImpact::Low => TestQualityImpact::Low,
            },
            _ => TestQualityImpact::Medium,
        }
    }
}

#[derive(Debug)]
struct FlakinessIndicator {
    flakiness_type: FlakinessType,
    impact: ReliabilityImpact,
    suggestion: String,
}

struct FlakinessAnalyzer {
    indicators: Vec<FlakinessIndicator>,
}

impl FlakinessAnalyzer {
    fn new() -> Self {
        Self {
            indicators: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for FlakinessAnalyzer {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = &*node.func {
            let path_str = path
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            // Consolidated pattern matching for flakiness detection
            if let Some(indicator) = detect_flakiness_pattern(&path_str) {
                self.indicators.push(indicator);
            }
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();

        // Check for timing-related methods
        if is_timing_method(&method_name) {
            self.indicators.push(FlakinessIndicator {
                flakiness_type: FlakinessType::TimingDependency,
                impact: ReliabilityImpact::High,
                suggestion: "Avoid time-dependent assertions, use deterministic checks".to_string(),
            });
        }

        // Check for thread spawning
        if method_name == "spawn" || method_name == "join" {
            self.indicators.push(FlakinessIndicator {
                flakiness_type: FlakinessType::ThreadingIssue,
                impact: ReliabilityImpact::High,
                suggestion: "Consider using deterministic concurrency testing tools".to_string(),
            });
        }

        syn::visit::visit_expr_method_call(self, node);
    }
}

fn analyze_flakiness(function: &ItemFn) -> Vec<FlakinessIndicator> {
    let mut analyzer = FlakinessAnalyzer::new();
    analyzer.visit_item_fn(function);
    analyzer.indicators
}

// Pure function for pattern-based flakiness detection
fn detect_flakiness_pattern(path_str: &str) -> Option<FlakinessIndicator> {
    // Pattern matching using a declarative approach
    match () {
        _ if is_timing_function(path_str) => Some(FlakinessIndicator {
            flakiness_type: FlakinessType::TimingDependency,
            impact: ReliabilityImpact::High,
            suggestion: "Replace sleep/timing dependencies with deterministic waits or mocks"
                .to_string(),
        }),
        _ if is_random_function(path_str) => Some(FlakinessIndicator {
            flakiness_type: FlakinessType::RandomValues,
            impact: ReliabilityImpact::Medium,
            suggestion: "Use deterministic test data instead of random values".to_string(),
        }),
        _ if is_external_service_call(path_str) => Some(FlakinessIndicator {
            flakiness_type: FlakinessType::ExternalDependency,
            impact: ReliabilityImpact::Critical,
            suggestion: "Mock external service calls for unit tests".to_string(),
        }),
        _ if is_filesystem_call(path_str) => Some(FlakinessIndicator {
            flakiness_type: FlakinessType::FilesystemDependency,
            impact: ReliabilityImpact::Medium,
            suggestion: "Use temporary directories or mock filesystem operations".to_string(),
        }),
        _ if is_network_call(path_str) => Some(FlakinessIndicator {
            flakiness_type: FlakinessType::NetworkDependency,
            impact: ReliabilityImpact::Critical,
            suggestion: "Mock network calls or use test doubles".to_string(),
        }),
        _ => None,
    }
}

// Pattern-based classification for flakiness detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatternCategory {
    Timing,
    Random,
    ExternalService,
    Filesystem,
    Network,
}

impl PatternCategory {
    const fn patterns(&self) -> &'static [&'static str] {
        match self {
            Self::Timing => &[
                "sleep",
                "Instant::now",
                "SystemTime::now",
                "Duration::from",
                "delay",
                "timeout",
                "wait_for",
                "park_timeout",
                "recv_timeout",
            ],
            Self::Random => &[
                "rand",
                "random",
                "thread_rng",
                "StdRng",
                "SmallRng",
                "gen_range",
                "sample",
                "shuffle",
                "choose",
            ],
            Self::ExternalService => &[
                "reqwest",
                "hyper",
                "http",
                "Client::new",
                "HttpClient",
                "ApiClient",
                "database",
                "db",
                "postgres",
                "mysql",
                "redis",
                "mongodb",
                "sqlx",
                "diesel",
            ],
            Self::Filesystem => &[
                "fs::",
                "File::",
                "std::fs",
                "tokio::fs",
                "async_std::fs",
                "read_to_string",
                "write",
                "create",
                "remove_file",
                "remove_dir",
                "rename",
                "copy",
                "metadata",
            ],
            Self::Network => &[
                "TcpStream",
                "TcpListener",
                "UdpSocket",
                "connect",
                "bind",
                "listen",
                "accept",
                "send_to",
                "recv_from",
            ],
        }
    }

    fn matches(&self, text: &str) -> bool {
        self.patterns().iter().any(|pattern| text.contains(pattern))
    }
}

// Specific helper functions using the consolidated pattern matching
fn is_timing_function(path: &str) -> bool {
    PatternCategory::Timing.matches(path)
}

fn is_timing_method(method: &str) -> bool {
    const TIMING_METHODS: &[&str] = &[
        "elapsed",
        "duration_since",
        "checked_duration_since",
        "timeout",
        "wait",
        "wait_timeout",
    ];

    TIMING_METHODS.contains(&method)
}

fn is_random_function(path: &str) -> bool {
    PatternCategory::Random.matches(path)
}

fn is_external_service_call(path: &str) -> bool {
    PatternCategory::ExternalService.matches(path)
}

fn is_filesystem_call(path: &str) -> bool {
    PatternCategory::Filesystem.matches(path)
}

fn is_network_call(path: &str) -> bool {
    PatternCategory::Network.matches(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for PatternCategory enum
    #[test]
    fn test_pattern_category_timing_matches() {
        let category = PatternCategory::Timing;
        assert!(category.matches("thread::sleep"));
        assert!(category.matches("Instant::now()"));
        assert!(category.matches("SystemTime::now()"));
        assert!(category.matches("Duration::from_secs"));
        assert!(category.matches("delay_ms"));
        assert!(category.matches("timeout_handler"));
        assert!(!category.matches("random_value"));
        assert!(!category.matches("file_read"));
    }

    #[test]
    fn test_pattern_category_random_matches() {
        let category = PatternCategory::Random;
        assert!(category.matches("rand::thread_rng"));
        assert!(category.matches("random_number"));
        assert!(category.matches("StdRng::new"));
        assert!(category.matches("SmallRng::from_entropy"));
        assert!(category.matches("gen_range(0, 100)"));
        assert!(category.matches("vec.shuffle()"));
        assert!(category.matches("items.choose()"));
        assert!(!category.matches("sleep_ms"));
        assert!(!category.matches("file_write"));
    }

    #[test]
    fn test_pattern_category_external_service_matches() {
        let category = PatternCategory::ExternalService;
        assert!(category.matches("reqwest::Client"));
        assert!(category.matches("hyper::server"));
        assert!(category.matches("http::request"));
        assert!(category.matches("Client::new()"));
        assert!(category.matches("database_connection"));
        assert!(category.matches("postgres::connect"));
        assert!(category.matches("redis::get"));
        assert!(category.matches("sqlx::query"));
        assert!(!category.matches("local_computation"));
    }

    #[test]
    fn test_pattern_category_filesystem_matches() {
        let category = PatternCategory::Filesystem;
        assert!(category.matches("fs::read_to_string"));
        assert!(category.matches("File::open"));
        assert!(category.matches("std::fs::create_dir"));
        assert!(category.matches("tokio::fs::write"));
        assert!(category.matches("async_std::fs::remove"));
        assert!(category.matches("metadata()"));
        assert!(!category.matches("network_send"));
    }

    #[test]
    fn test_pattern_category_network_matches() {
        let category = PatternCategory::Network;
        assert!(category.matches("TcpStream::connect"));
        assert!(category.matches("TcpListener::bind"));
        assert!(category.matches("UdpSocket::send_to"));
        assert!(category.matches("socket.accept()"));
        assert!(category.matches("recv_from_addr"));
        assert!(!category.matches("file_read"));
    }

    /// Pure function to check if a string contains any of the given patterns
    #[inline]
    fn matches_any_pattern(text: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|pattern| text.contains(pattern))
    }

    #[test]
    fn test_matches_any_pattern() {
        // Test the pure helper function
        assert!(matches_any_pattern("test_sleep", &["sleep", "delay"]));
        assert!(matches_any_pattern("delay_function", &["sleep", "delay"]));
        assert!(!matches_any_pattern("normal_function", &["sleep", "delay"]));
        assert!(!matches_any_pattern("", &["sleep", "delay"]));
        assert!(!matches_any_pattern("test", &[]));
    }

    #[test]
    fn test_is_timing_function_detects_sleep() {
        assert!(is_timing_function("thread::sleep"));
        assert!(is_timing_function("some_function_with_sleep_in_name"));
        assert!(is_timing_function("tokio::time::sleep"));
        assert!(is_timing_function("async_std::task::sleep"));
    }

    #[test]
    fn test_is_timing_function_detects_time_operations() {
        assert!(is_timing_function("Instant::now"));
        assert!(is_timing_function("SystemTime::now"));
        assert!(is_timing_function("Duration::from_secs"));
        assert!(is_timing_function("timeout_handler"));
    }

    #[test]
    fn test_is_timing_function_ignores_non_timing() {
        assert!(!is_timing_function("process_data"));
        assert!(!is_timing_function("calculate_result"));
        assert!(!is_timing_function("handle_request"));
    }

    #[test]
    fn test_is_timing_method_detects_timing_methods() {
        assert!(is_timing_method("elapsed"));
        assert!(is_timing_method("duration_since"));
        assert!(is_timing_method("wait"));
        assert!(is_timing_method("wait_timeout"));
    }

    #[test]
    fn test_is_timing_method_ignores_non_timing() {
        assert!(!is_timing_method("process"));
        assert!(!is_timing_method("calculate"));
        assert!(!is_timing_method("handle"));
    }

    #[test]
    fn test_is_random_function_detects_random_operations() {
        assert!(is_random_function("rand::thread_rng"));
        assert!(is_random_function("random_number"));
        assert!(is_random_function("StdRng::new"));
        assert!(is_random_function("gen_range"));
        assert!(is_random_function("shuffle_items"));
    }

    #[test]
    fn test_is_random_function_ignores_non_random() {
        assert!(!is_random_function("deterministic_function"));
        assert!(!is_random_function("calculate_sum"));
        assert!(!is_random_function("process_data"));
    }

    #[test]
    fn test_is_external_service_call_detects_http_clients() {
        assert!(is_external_service_call("reqwest::Client"));
        assert!(is_external_service_call("hyper::Client"));
        assert!(is_external_service_call("http::request"));
        assert!(is_external_service_call("ApiClient::new"));
    }

    // Edge case tests for pattern matching
    #[test]
    fn test_pattern_case_sensitivity() {
        // All patterns are case-sensitive
        assert!(is_timing_function("sleep"));
        assert!(!is_timing_function("SLEEP"));
        assert!(is_random_function("random"));
        assert!(!is_random_function("RANDOM"));
    }

    #[test]
    fn test_partial_matches_work() {
        // Patterns match as substrings
        assert!(is_timing_function("my_sleep_function"));
        assert!(is_timing_function("function_with_timeout_handler"));
        assert!(is_random_function("get_random_value"));
        assert!(is_filesystem_call("my_fs::operations"));
    }

    #[test]
    fn test_empty_string_handling() {
        assert!(!is_timing_function(""));
        assert!(!is_timing_method(""));
        assert!(!is_random_function(""));
        assert!(!is_external_service_call(""));
        assert!(!is_filesystem_call(""));
        assert!(!is_network_call(""));
    }

    #[test]
    fn test_pattern_category_equality() {
        assert_eq!(PatternCategory::Timing, PatternCategory::Timing);
        assert_ne!(PatternCategory::Timing, PatternCategory::Random);
        assert_ne!(PatternCategory::Filesystem, PatternCategory::Network);
    }

    #[test]
    fn test_is_external_service_call_detects_databases() {
        assert!(is_external_service_call("postgres::connect"));
        assert!(is_external_service_call("mysql::query"));
        assert!(is_external_service_call("redis::get"));
        assert!(is_external_service_call("mongodb::find"));
        assert!(is_external_service_call("sqlx::query"));
        assert!(is_external_service_call("diesel::connection"));
    }

    #[test]
    fn test_is_external_service_call_ignores_internal_calls() {
        assert!(!is_external_service_call("internal_function"));
        assert!(!is_external_service_call("process_locally"));
        assert!(!is_external_service_call("calculate_value"));
    }

    #[test]
    fn test_is_filesystem_call_detects_fs_operations() {
        assert!(is_filesystem_call("std::fs::read_to_string"));
        assert!(is_filesystem_call("File::open"));
        assert!(is_filesystem_call("tokio::fs::write"));
        assert!(is_filesystem_call("remove_file"));
        assert!(is_filesystem_call("create_dir"));
    }

    #[test]
    fn test_is_filesystem_call_ignores_non_fs() {
        assert!(!is_filesystem_call("calculate"));
        assert!(!is_filesystem_call("process"));
        assert!(!is_filesystem_call("transform"));
    }

    #[test]
    fn test_is_network_call_detects_network_operations() {
        assert!(is_network_call("TcpStream::connect"));
        assert!(is_network_call("TcpListener::bind"));
        assert!(is_network_call("UdpSocket::bind"));
        assert!(is_network_call("socket.send_to"));
        assert!(is_network_call("listener.accept"));
    }

    #[test]
    fn test_is_network_call_ignores_non_network() {
        assert!(!is_network_call("process_data"));
        assert!(!is_network_call("calculate_result"));
        assert!(!is_network_call("transform_input"));
    }

    #[test]
    fn test_pattern_matching_is_case_sensitive() {
        // These tests ensure our pattern matching is working as expected
        assert!(!is_timing_function("SLEEP")); // case sensitive
        assert!(is_timing_function("sleep")); // lowercase matches

        assert!(!is_random_function("RAND")); // case sensitive
        assert!(is_random_function("rand")); // lowercase matches
    }

    #[test]
    fn test_consolidated_patterns_still_work() {
        // Test all the specific patterns that were consolidated
        assert!(is_timing_function("thread::sleep"));
        assert!(is_timing_function("time::sleep"));
        assert!(is_timing_function("tokio::time::sleep"));
        assert!(is_timing_function("async_std::task::sleep"));
        assert!(is_timing_function("delay_ms"));
        assert!(is_timing_function("set_timeout"));
        assert!(is_timing_function("Duration::from_millis"));
        assert!(is_timing_function("Instant::now"));
        assert!(is_timing_function("SystemTime::now"));
    }

    #[test]
    fn test_is_timing_function_edge_cases() {
        // Test empty string
        assert!(!is_timing_function(""));

        // Test single character strings
        assert!(!is_timing_function("a"));

        // Test exact matches
        assert!(is_timing_function("sleep"));
        assert!(is_timing_function("delay"));
        assert!(is_timing_function("timeout"));

        // Test case sensitivity - function is case-sensitive
        assert!(!is_timing_function("SLEEP")); // Should not match uppercase
        assert!(!is_timing_function("Sleep")); // Should not match mixed case

        // Test with special characters in path
        assert!(is_timing_function("module::sleep_function"));
        assert!(is_timing_function("sleep.function"));
        assert!(is_timing_function("sleep-function"));
    }

    #[test]
    fn test_is_timing_function_boundary_patterns() {
        // Test Duration patterns with various formats
        assert!(is_timing_function("Duration::from_secs"));
        assert!(is_timing_function("Duration::from_millis"));
        assert!(is_timing_function("Duration::from_nanos"));
        assert!(is_timing_function("Duration::from_micros"));

        // Test delay variations
        assert!(is_timing_function("delay_for"));
        assert!(is_timing_function("delay_until"));
        assert!(is_timing_function("with_delay"));

        // Test timeout variations
        assert!(is_timing_function("timeout_after"));
        assert!(is_timing_function("with_timeout"));
        assert!(is_timing_function("set_read_timeout"));

        // Ensure non-timing "Duration" patterns don't match
        assert!(!is_timing_function("calculate_duration")); // "duration" alone without "Duration::from" context
    }

    #[test]
    fn test_is_timing_function_new_patterns() {
        // Test new patterns added during refactoring
        assert!(is_timing_function("wait_for_completion"));
        assert!(is_timing_function("park_timeout_ms"));
        assert!(is_timing_function("recv_timeout"));

        // Test that these are detected as timing functions
        assert!(is_timing_function("thread::park_timeout"));
        assert!(is_timing_function("channel.recv_timeout"));
        assert!(is_timing_function("future.wait_for"));
    }

    #[test]
    fn test_detect_flakiness_pattern_timing() {
        let indicator = detect_flakiness_pattern("thread::sleep").unwrap();
        assert_eq!(indicator.flakiness_type, FlakinessType::TimingDependency);
        assert_eq!(indicator.impact, ReliabilityImpact::High);
        assert!(indicator.suggestion.contains("deterministic"));

        let indicator = detect_flakiness_pattern("Instant::now").unwrap();
        assert_eq!(indicator.flakiness_type, FlakinessType::TimingDependency);
    }

    #[test]
    fn test_detect_flakiness_pattern_random() {
        let indicator = detect_flakiness_pattern("rand::thread_rng").unwrap();
        assert_eq!(indicator.flakiness_type, FlakinessType::RandomValues);
        assert_eq!(indicator.impact, ReliabilityImpact::Medium);
        assert!(indicator.suggestion.contains("deterministic test data"));
    }

    #[test]
    fn test_detect_flakiness_pattern_external_service() {
        let indicator = detect_flakiness_pattern("reqwest::Client").unwrap();
        assert_eq!(indicator.flakiness_type, FlakinessType::ExternalDependency);
        assert_eq!(indicator.impact, ReliabilityImpact::Critical);
        assert!(indicator.suggestion.contains("Mock external"));

        let indicator = detect_flakiness_pattern("postgres::connect").unwrap();
        assert_eq!(indicator.flakiness_type, FlakinessType::ExternalDependency);
    }

    #[test]
    fn test_detect_flakiness_pattern_filesystem() {
        let indicator = detect_flakiness_pattern("std::fs::read_to_string").unwrap();
        assert_eq!(
            indicator.flakiness_type,
            FlakinessType::FilesystemDependency
        );
        assert_eq!(indicator.impact, ReliabilityImpact::Medium);
        assert!(indicator.suggestion.contains("temporary directories"));

        let indicator = detect_flakiness_pattern("File::create").unwrap();
        assert_eq!(
            indicator.flakiness_type,
            FlakinessType::FilesystemDependency
        );
    }

    #[test]
    fn test_detect_flakiness_pattern_network() {
        let indicator = detect_flakiness_pattern("TcpStream::connect").unwrap();
        assert_eq!(indicator.flakiness_type, FlakinessType::NetworkDependency);
        assert_eq!(indicator.impact, ReliabilityImpact::Critical);
        assert!(indicator.suggestion.contains("test doubles"));

        let indicator = detect_flakiness_pattern("UdpSocket::bind").unwrap();
        assert_eq!(indicator.flakiness_type, FlakinessType::NetworkDependency);
    }

    #[test]
    fn test_detect_flakiness_pattern_no_match() {
        assert!(detect_flakiness_pattern("regular_function").is_none());
        assert!(detect_flakiness_pattern("process_data").is_none());
        assert!(detect_flakiness_pattern("calculate_result").is_none());
        assert!(detect_flakiness_pattern("").is_none());
    }

    #[test]
    fn test_detect_flakiness_pattern_priority_order() {
        // Test that timing patterns take precedence (first in match)
        let indicator = detect_flakiness_pattern("sleep_and_random").unwrap();
        assert_eq!(indicator.flakiness_type, FlakinessType::TimingDependency);

        // Test edge cases
        let indicator = detect_flakiness_pattern("timeout_with_delay").unwrap();
        assert_eq!(indicator.flakiness_type, FlakinessType::TimingDependency);
    }
}
