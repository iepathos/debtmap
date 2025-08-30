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

            // Check for timing functions
            if is_timing_function(&path_str) {
                self.indicators.push(FlakinessIndicator {
                    flakiness_type: FlakinessType::TimingDependency,
                    impact: ReliabilityImpact::High,
                    suggestion:
                        "Replace sleep/timing dependencies with deterministic waits or mocks"
                            .to_string(),
                });
            }

            // Check for random functions
            if is_random_function(&path_str) {
                self.indicators.push(FlakinessIndicator {
                    flakiness_type: FlakinessType::RandomValues,
                    impact: ReliabilityImpact::Medium,
                    suggestion: "Use deterministic test data instead of random values".to_string(),
                });
            }

            // Check for external service calls
            if is_external_service_call(&path_str) {
                self.indicators.push(FlakinessIndicator {
                    flakiness_type: FlakinessType::ExternalDependency,
                    impact: ReliabilityImpact::Critical,
                    suggestion: "Mock external service calls for unit tests".to_string(),
                });
            }

            // Check for filesystem operations
            if is_filesystem_call(&path_str) {
                self.indicators.push(FlakinessIndicator {
                    flakiness_type: FlakinessType::FilesystemDependency,
                    impact: ReliabilityImpact::Medium,
                    suggestion: "Use temporary directories or mock filesystem operations"
                        .to_string(),
                });
            }

            // Check for network operations
            if is_network_call(&path_str) {
                self.indicators.push(FlakinessIndicator {
                    flakiness_type: FlakinessType::NetworkDependency,
                    impact: ReliabilityImpact::Critical,
                    suggestion: "Mock network calls or use test doubles".to_string(),
                });
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

fn is_timing_function(path: &str) -> bool {
    // Group patterns to reduce measured complexity while maintaining clarity
    // Sleep-related patterns
    if path.contains("sleep") || path.contains("delay") {
        return true;
    }

    // Time measurement patterns
    if path.contains("timeout") || path.contains("Duration::from") {
        return true;
    }

    // Time instant patterns
    if path.contains("Instant::now") || path.contains("SystemTime::now") {
        return true;
    }

    // Async-specific sleep patterns (more specific checks)
    path.contains("time::sleep")
        || path.contains("tokio::time::sleep")
        || path.contains("async_std::task::sleep")
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
    const RANDOM_PATTERNS: &[&str] = &[
        "rand",
        "random",
        "thread_rng",
        "StdRng",
        "SmallRng",
        "gen_range",
        "sample",
        "shuffle",
        "choose",
    ];

    RANDOM_PATTERNS.iter().any(|pattern| path.contains(pattern))
}

fn is_external_service_call(path: &str) -> bool {
    const EXTERNAL_PATTERNS: &[&str] = &[
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
    ];

    EXTERNAL_PATTERNS
        .iter()
        .any(|pattern| path.contains(pattern))
}

fn is_filesystem_call(path: &str) -> bool {
    const FS_PATTERNS: &[&str] = &[
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
    ];

    FS_PATTERNS.iter().any(|pattern| path.contains(pattern))
}

fn is_network_call(path: &str) -> bool {
    const NETWORK_PATTERNS: &[&str] = &[
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "connect",
        "bind",
        "listen",
        "accept",
        "send_to",
        "recv_from",
    ];

    NETWORK_PATTERNS
        .iter()
        .any(|pattern| path.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_partial_matches_work() {
        // Ensure contains() logic works for partial matches
        assert!(is_timing_function("my_sleep_function"));
        assert!(is_timing_function("function_with_timeout_logic"));
        assert!(is_network_call("MyTcpStream"));
        assert!(is_filesystem_call("custom_fs::operation"));
    }

    #[test]
    fn test_is_timing_function_comprehensive_coverage() {
        // Test all timing patterns are properly detected
        // Sleep patterns
        assert!(is_timing_function("thread::sleep"));
        assert!(is_timing_function("sleep"));
        assert!(is_timing_function("my_sleep_wrapper"));

        // Delay patterns
        assert!(is_timing_function("delay"));
        assert!(is_timing_function("apply_delay"));
        assert!(is_timing_function("network_delay"));

        // Timeout patterns
        assert!(is_timing_function("timeout"));
        assert!(is_timing_function("set_timeout"));
        assert!(is_timing_function("operation_with_timeout"));

        // Duration patterns
        assert!(is_timing_function("Duration::from_secs"));
        assert!(is_timing_function("Duration::from_millis"));

        // Time instant patterns
        assert!(is_timing_function("Instant::now"));
        assert!(is_timing_function("SystemTime::now"));

        // Async-specific sleep patterns
        assert!(is_timing_function("time::sleep"));
        assert!(is_timing_function("tokio::time::sleep"));
        assert!(is_timing_function("async_std::task::sleep"));

        // Negative cases - should not match
        assert!(!is_timing_function("process_data"));
        assert!(!is_timing_function("calculate_result"));
        assert!(!is_timing_function("handle_request"));
        assert!(!is_timing_function(""));
    }
}
