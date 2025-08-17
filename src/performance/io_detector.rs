use super::{IOPattern, PerformanceAntiPattern, PerformanceDetector, PerformanceImpact};
use std::path::Path;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprForLoop, ExprLoop, ExprWhile, File};

pub struct IOPerformanceDetector {}

impl IOPerformanceDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for IOPerformanceDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceDetector for IOPerformanceDetector {
    fn detect_anti_patterns(&self, file: &File, _path: &Path) -> Vec<PerformanceAntiPattern> {
        let mut visitor = IOVisitor {
            patterns: Vec::new(),
            in_loop: false,
            loop_depth: 0,
        };

        visitor.visit_file(file);
        visitor.patterns
    }

    fn detector_name(&self) -> &'static str {
        "IOPerformanceDetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::InefficientIO { io_pattern, .. } => match io_pattern {
                IOPattern::SyncInLoop => PerformanceImpact::High,
                IOPattern::UnbatchedQueries => PerformanceImpact::Critical,
                IOPattern::UnbufferedIO => PerformanceImpact::Medium,
                IOPattern::ExcessiveConnections => PerformanceImpact::High,
            },
            _ => PerformanceImpact::Low,
        }
    }
}

struct IOVisitor {
    patterns: Vec<PerformanceAntiPattern>,
    in_loop: bool,
    loop_depth: usize,
}

impl IOVisitor {
    fn check_io_operation(&mut self, expr: &Expr) {
        if !self.in_loop {
            return;
        }

        // Check for file I/O operations
        if let Expr::Call(call) = expr {
            if let Expr::Path(path) = &*call.func {
                let path_str = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");

                if self.is_file_io(&path_str) {
                    self.patterns.push(PerformanceAntiPattern::InefficientIO {
                        io_pattern: IOPattern::SyncInLoop,
                        batching_opportunity: true,
                        async_opportunity: true,
                    });
                } else if self.is_database_operation(&path_str) {
                    self.patterns.push(PerformanceAntiPattern::InefficientIO {
                        io_pattern: IOPattern::UnbatchedQueries,
                        batching_opportunity: true,
                        async_opportunity: true,
                    });
                } else if self.is_network_operation(&path_str) {
                    self.patterns.push(PerformanceAntiPattern::InefficientIO {
                        io_pattern: IOPattern::SyncInLoop,
                        batching_opportunity: false,
                        async_opportunity: true,
                    });
                }
            }
        }

        // Check for method calls that might be I/O
        if let Expr::MethodCall(method_call) = expr {
            let method_name = method_call.method.to_string();

            if self.is_io_method(&method_name) {
                let (io_pattern, can_batch) =
                    if method_name.contains("query") || method_name.contains("execute") {
                        (IOPattern::UnbatchedQueries, true)
                    } else {
                        (IOPattern::SyncInLoop, false)
                    };

                self.patterns.push(PerformanceAntiPattern::InefficientIO {
                    io_pattern,
                    batching_opportunity: can_batch,
                    async_opportunity: true,
                });
            }
        }
    }

    fn is_file_io(&self, path: &str) -> bool {
        path.contains("fs::")
            || path.contains("File::")
            || path.contains("read_to_string")
            || path.contains("write")
            || path.contains("OpenOptions")
    }

    fn is_database_operation(&self, path: &str) -> bool {
        path.contains("query")
            || path.contains("execute")
            || path.contains("sqlx")
            || path.contains("diesel")
            || path.contains("postgres")
            || path.contains("mysql")
            || path.contains("sqlite")
    }

    fn is_network_operation(&self, path: &str) -> bool {
        path.contains("TcpStream")
            || path.contains("reqwest")
            || path.contains("hyper")
            || path.contains("http")
            || path.contains("Request")
            || path.contains("Response")
    }

    fn is_io_method(&self, method: &str) -> bool {
        method == "read"
            || method == "write"
            || method == "read_to_string"
            || method == "read_to_end"
            || method == "write_all"
            || method == "flush"
            || method == "query"
            || method == "execute"
            || method == "fetch"
            || method == "fetch_one"
            || method == "fetch_all"
            || method == "send"
            || method == "recv"
    }

    fn check_unbuffered_io(&mut self, call: &ExprCall) {
        if let Expr::Path(path) = &*call.func {
            let path_str = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            // Check for direct file operations without buffering
            if path_str.contains("File::open") || path_str.contains("File::create") {
                // Check if it's being wrapped in a BufReader/BufWriter
                // This is simplified - real implementation would track usage
                self.patterns.push(PerformanceAntiPattern::InefficientIO {
                    io_pattern: IOPattern::UnbufferedIO,
                    batching_opportunity: false,
                    async_opportunity: false,
                });
            }
        }
    }
}

impl<'ast> Visit<'ast> for IOVisitor {
    fn visit_expr_for_loop(&mut self, node: &'ast ExprForLoop) {
        let was_in_loop = self.in_loop;
        self.in_loop = true;
        self.loop_depth += 1;

        visit::visit_expr_for_loop(self, node);

        self.loop_depth -= 1;
        self.in_loop = was_in_loop || self.loop_depth > 0;
    }

    fn visit_expr_while(&mut self, node: &'ast ExprWhile) {
        let was_in_loop = self.in_loop;
        self.in_loop = true;
        self.loop_depth += 1;

        visit::visit_expr_while(self, node);

        self.loop_depth -= 1;
        self.in_loop = was_in_loop || self.loop_depth > 0;
    }

    fn visit_expr_loop(&mut self, node: &'ast ExprLoop) {
        let was_in_loop = self.in_loop;
        self.in_loop = true;
        self.loop_depth += 1;

        visit::visit_expr_loop(self, node);

        self.loop_depth -= 1;
        self.in_loop = was_in_loop || self.loop_depth > 0;
    }

    fn visit_expr(&mut self, node: &'ast Expr) {
        self.check_io_operation(node);

        if let Expr::Call(call) = node {
            self.check_unbuffered_io(call);
        }

        visit::visit_expr(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_io_in_loop() {
        let source = r#"
            fn process_files(filenames: &[String]) -> Vec<String> {
                let mut contents = Vec::new();
                for filename in filenames {
                    let content = std::fs::read_to_string(filename).unwrap();
                    contents.push(content);
                }
                contents
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = IOPerformanceDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        assert!(!patterns.is_empty());
        let io_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::InefficientIO {
                    io_pattern: IOPattern::SyncInLoop,
                    ..
                }
            )
        });
        assert!(io_pattern.is_some());
    }

    #[test]
    fn test_database_query_in_loop() {
        let source = r#"
            async fn fetch_users(ids: &[i32]) {
                for id in ids {
                    let user = sqlx::query("SELECT * FROM users WHERE id = ?")
                        .bind(id)
                        .fetch_one(&pool)
                        .await
                        .unwrap();
                    process_user(user);
                }
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = IOPerformanceDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        let query_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::InefficientIO {
                    io_pattern: IOPattern::UnbatchedQueries,
                    ..
                }
            )
        });
        assert!(query_pattern.is_some());
    }

    #[test]
    fn test_unbuffered_file_io() {
        let source = r#"
            use std::fs::File;
            use std::io::Read;
            
            fn read_file(path: &str) -> String {
                let mut file = File::open(path).unwrap();
                let mut contents = String::new();
                file.read_to_string(&mut contents).unwrap();
                contents
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = IOPerformanceDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        let unbuffered_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::InefficientIO {
                    io_pattern: IOPattern::UnbufferedIO,
                    ..
                }
            )
        });
        assert!(unbuffered_pattern.is_some());
    }

    #[test]
    fn test_no_false_positives_on_imports() {
        let source = r#"
            use debtmap::core::{
                cache::AnalysisCache, ComplexityMetrics, FileMetrics, FunctionMetrics, Language,
            };
            use std::path::PathBuf;
            use tempfile::TempDir;

            fn simple_function() {
                println!("No I/O operations here");
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = IOPerformanceDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        // Should find no patterns since there are no actual I/O operations
        assert_eq!(
            patterns.len(),
            0,
            "Should not detect I/O patterns in import statements"
        );
    }
}
