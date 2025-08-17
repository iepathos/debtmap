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
    let timing_patterns = [
        "thread::sleep",
        "sleep",
        "delay",
        "timeout",
        "Duration::from",
        "Instant::now",
        "SystemTime::now",
        "time::sleep",
        "tokio::time::sleep",
        "async_std::task::sleep",
    ];

    timing_patterns.iter().any(|pattern| path.contains(pattern))
}

fn is_timing_method(method: &str) -> bool {
    let timing_methods = [
        "elapsed",
        "duration_since",
        "checked_duration_since",
        "timeout",
        "wait",
        "wait_timeout",
    ];

    timing_methods.contains(&method)
}

fn is_random_function(path: &str) -> bool {
    let random_patterns = [
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

    random_patterns.iter().any(|pattern| path.contains(pattern))
}

fn is_external_service_call(path: &str) -> bool {
    let external_patterns = [
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

    external_patterns
        .iter()
        .any(|pattern| path.contains(pattern))
}

fn is_filesystem_call(path: &str) -> bool {
    let fs_patterns = [
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

    fs_patterns.iter().any(|pattern| path.contains(pattern))
}

fn is_network_call(path: &str) -> bool {
    let network_patterns = [
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

    network_patterns
        .iter()
        .any(|pattern| path.contains(pattern))
}
