use super::RustFlakinessType;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ItemFn, PathSegment, Stmt};

/// Detects patterns that can cause test flakiness in Rust tests
pub struct FlakyDetector {
    indicators: Vec<FlakyIndicator>,
}

#[derive(Debug, Clone)]
pub struct FlakyIndicator {
    pub flakiness_type: RustFlakinessType,
    pub line: usize,
    pub explanation: String,
}

impl FlakyDetector {
    pub fn new() -> Self {
        Self {
            indicators: Vec::new(),
        }
    }

    /// Detect flaky patterns in a test function
    pub fn detect_flaky_patterns(&mut self, func: &ItemFn) -> Vec<FlakyIndicator> {
        self.indicators.clear();
        self.visit_block(&func.block);
        self.indicators.clone()
    }

    /// Check if any flaky patterns were detected
    pub fn has_flaky_patterns(&self) -> bool {
        !self.indicators.is_empty()
    }

    /// Get count of flaky patterns
    pub fn flaky_pattern_count(&self) -> usize {
        self.indicators.len()
    }

    /// Check if path represents a timing-related function
    fn is_timing_related(&self, segments: &[&PathSegment]) -> bool {
        let path_str = segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        // Timing dependencies
        path_str.contains("thread::sleep")
            || path_str.contains("sleep")
            || path_str.contains("Instant::now")
            || path_str.contains("SystemTime::now")
            || path_str.contains("Duration")
    }

    /// Check if path represents random value generation
    fn is_random_related(&self, segments: &[&PathSegment]) -> bool {
        let path_str = segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        path_str.contains("rand")
            || path_str.contains("random")
            || path_str.contains("Uuid::new")
            || path_str.contains("uuid")
    }

    /// Check if path represents external dependencies
    fn is_external_dependency(&self, segments: &[&PathSegment]) -> bool {
        let path_str = segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        path_str.contains("reqwest")
            || path_str.contains("hyper")
            || path_str.contains("http")
            || path_str.contains("Client")
            || path_str.contains("Request")
    }

    /// Check if path represents filesystem operations
    fn is_filesystem_related(&self, segments: &[&PathSegment]) -> bool {
        let path_str = segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        path_str.contains("File::")
            || path_str.contains("fs::")
            || path_str.contains("read")
            || path_str.contains("write")
            || path_str.contains("OpenOptions")
    }

    /// Check if path represents network operations
    fn is_network_related(&self, segments: &[&PathSegment]) -> bool {
        let path_str = segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        path_str.contains("TcpStream")
            || path_str.contains("UdpSocket")
            || path_str.contains("bind")
            || path_str.contains("connect")
            || path_str.contains("listen")
    }

    /// Check if path represents threading operations
    fn is_threading_related(&self, segments: &[&PathSegment]) -> bool {
        let path_str = segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        path_str.contains("thread::spawn")
            || path_str.contains("spawn")
            || path_str.contains("Arc")
            || path_str.contains("Mutex")
            || path_str.contains("RwLock")
            || path_str.contains("Channel")
    }

    /// Check if path represents HashMap iteration (non-deterministic ordering)
    fn is_hash_ordering_issue(&self, segments: &[&PathSegment]) -> bool {
        let path_str = segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        path_str.contains("HashMap::iter") || path_str.contains("HashSet::iter")
    }

    /// Extract path segments from an expression
    fn extract_path_segments(expr: &Expr) -> Vec<&PathSegment> {
        match expr {
            Expr::Path(expr_path) => expr_path.path.segments.iter().collect(),
            Expr::Call(call) => Self::extract_path_segments(&call.func),
            Expr::MethodCall(_method) => vec![],
            _ => vec![],
        }
    }

    /// Analyze expression for flaky patterns
    fn analyze_expr(&mut self, expr: &Expr, line: usize) {
        let segments = Self::extract_path_segments(expr);

        if self.is_timing_related(&segments) {
            self.indicators.push(FlakyIndicator {
                flakiness_type: RustFlakinessType::TimingDependency,
                line,
                explanation: "Test uses timing-dependent code (sleep, Instant::now) which can cause flakiness".to_string(),
            });
        }

        if self.is_random_related(&segments) {
            self.indicators.push(FlakyIndicator {
                flakiness_type: RustFlakinessType::RandomValue,
                line,
                explanation: "Test uses random values which can cause non-deterministic behavior"
                    .to_string(),
            });
        }

        if self.is_external_dependency(&segments) {
            self.indicators.push(FlakyIndicator {
                flakiness_type: RustFlakinessType::ExternalDependency,
                line,
                explanation: "Test depends on external services which can be unreliable"
                    .to_string(),
            });
        }

        if self.is_filesystem_related(&segments) {
            self.indicators.push(FlakyIndicator {
                flakiness_type: RustFlakinessType::FileSystemDependency,
                line,
                explanation:
                    "Test performs filesystem operations which can fail in different environments"
                        .to_string(),
            });
        }

        if self.is_network_related(&segments) {
            self.indicators.push(FlakyIndicator {
                flakiness_type: RustFlakinessType::NetworkDependency,
                line,
                explanation: "Test uses network operations which can be unreliable".to_string(),
            });
        }

        if self.is_threading_related(&segments) {
            self.indicators.push(FlakyIndicator {
                flakiness_type: RustFlakinessType::ThreadingIssue,
                line,
                explanation: "Test uses threading which can cause race conditions and flakiness"
                    .to_string(),
            });
        }

        if self.is_hash_ordering_issue(&segments) {
            self.indicators.push(FlakyIndicator {
                flakiness_type: RustFlakinessType::HashOrdering,
                line,
                explanation: "Test iterates HashMap/HashSet which has non-deterministic ordering"
                    .to_string(),
            });
        }
    }
}

impl Default for FlakyDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Visit<'ast> for FlakyDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        let line = expr.span().start().line;
        self.analyze_expr(expr, line);
        syn::visit::visit_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if let Stmt::Expr(expr, _) = stmt {
            let line = expr.span().start().line;
            self.analyze_expr(expr, line);
        }
        syn::visit::visit_stmt(self, stmt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_detect_sleep() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_timing() {
                std::thread::sleep(std::time::Duration::from_millis(100));
                assert!(true);
            }
        };

        let mut detector = FlakyDetector::new();
        let indicators = detector.detect_flaky_patterns(&func);
        assert!(!indicators.is_empty());
        assert!(indicators
            .iter()
            .any(|i| matches!(i.flakiness_type, RustFlakinessType::TimingDependency)));
    }

    #[test]
    fn test_detect_random() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_random() {
                use rand::Rng;
                let value = rand::thread_rng().gen_range(0..100);
                assert!(value < 100);
            }
        };

        let mut detector = FlakyDetector::new();
        let indicators = detector.detect_flaky_patterns(&func);
        assert!(indicators
            .iter()
            .any(|i| matches!(i.flakiness_type, RustFlakinessType::RandomValue)));
    }

    #[test]
    fn test_detect_network() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_network() {
                let client = reqwest::Client::new();
                assert!(true);
            }
        };

        let mut detector = FlakyDetector::new();
        let indicators = detector.detect_flaky_patterns(&func);
        assert!(indicators
            .iter()
            .any(|i| matches!(i.flakiness_type, RustFlakinessType::ExternalDependency)));
    }

    #[test]
    fn test_detect_filesystem() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_file() {
                std::fs::write("/tmp/test.txt", "data").unwrap();
                assert!(true);
            }
        };

        let mut detector = FlakyDetector::new();
        let indicators = detector.detect_flaky_patterns(&func);
        assert!(indicators
            .iter()
            .any(|i| matches!(i.flakiness_type, RustFlakinessType::FileSystemDependency)));
    }

    #[test]
    fn test_detect_threading() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_thread() {
                std::thread::spawn(|| {
                    // do work
                });
                assert!(true);
            }
        };

        let mut detector = FlakyDetector::new();
        let indicators = detector.detect_flaky_patterns(&func);
        assert!(indicators
            .iter()
            .any(|i| matches!(i.flakiness_type, RustFlakinessType::ThreadingIssue)));
    }

    #[test]
    fn test_no_flaky_patterns() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_clean() {
                let x = 42;
                assert_eq!(x, 42);
            }
        };

        let mut detector = FlakyDetector::new();
        let indicators = detector.detect_flaky_patterns(&func);
        assert!(indicators.is_empty());
    }

    #[test]
    fn test_multiple_flaky_patterns() {
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_multiple() {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let value = rand::thread_rng().gen_range(0..100);
                assert!(value < 100);
            }
        };

        let mut detector = FlakyDetector::new();
        let indicators = detector.detect_flaky_patterns(&func);
        assert!(indicators.len() >= 2);
    }
}
