use crate::analysis::multi_signal_aggregation::ResponsibilityCategory;
use crate::analysis::rust_patterns::context::RustFunctionContext;
use serde::{Deserialize, Serialize};
use syn::{visit::Visit, Expr, ExprAwait, ExprCall, ExprPath};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AsyncPatternType {
    AsyncFunction,
    TaskSpawning,
    ChannelCommunication,
    MutexUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AsyncPattern {
    pub pattern_type: AsyncPatternType,
    pub confidence: f64,
    pub evidence: String,
}

/// AST visitor for detecting concurrency patterns
#[derive(Default)]
pub struct ConcurrencyPatternVisitor {
    pub has_mutex: bool,
    pub has_rwlock: bool,
    pub has_channel_send: bool,
    pub has_channel_recv: bool,
    pub spawn_calls: Vec<String>,
    pub await_points: usize,
}

impl ConcurrencyPatternVisitor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'ast> Visit<'ast> for ConcurrencyPatternVisitor {
    fn visit_expr_await(&mut self, await_expr: &'ast ExprAwait) {
        self.await_points += 1;
        syn::visit::visit_expr_await(self, await_expr);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        // Build path string for analysis
        let path_segments: Vec<_> = path.segments.iter().map(|s| s.ident.to_string()).collect();

        // Detect synchronization primitives
        if path_segments.iter().any(|s| s == "Mutex") {
            self.has_mutex = true;
        }
        if path_segments.iter().any(|s| s == "RwLock") {
            self.has_rwlock = true;
        }

        syn::visit::visit_path(self, path);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        // Detect spawn calls (tokio::spawn, async_std::spawn, etc.)
        if let Expr::Path(ExprPath { path, .. }) = &*call.func {
            let path_str = path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            if path_str.contains("spawn") {
                self.spawn_calls.push(path_str);
            }
        }

        syn::visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, method: &'ast syn::ExprMethodCall) {
        let method_name = method.method.to_string();

        // Detect channel operations
        if method_name == "send" || method_name == "try_send" {
            self.has_channel_send = true;
        }
        if method_name == "recv" || method_name == "try_recv" {
            self.has_channel_recv = true;
        }

        syn::visit::visit_expr_method_call(self, method);
    }
}

pub struct RustAsyncDetector;

impl RustAsyncDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_async_patterns(&self, context: &RustFunctionContext) -> Vec<AsyncPattern> {
        let mut patterns = Vec::new();

        // Check if function is async (using verified capability)
        if context.is_async() {
            patterns.push(AsyncPattern {
                pattern_type: AsyncPatternType::AsyncFunction,
                confidence: 1.0,
                evidence: "Function is declared as async".into(),
            });
        }

        // Traverse AST to find concurrency patterns
        let mut visitor = ConcurrencyPatternVisitor::new();
        visitor.visit_block(context.body());

        // Task spawning detected
        if !visitor.spawn_calls.is_empty() {
            patterns.push(AsyncPattern {
                pattern_type: AsyncPatternType::TaskSpawning,
                confidence: 0.9,
                evidence: format!("Spawns async tasks: {}", visitor.spawn_calls.join(", ")),
            });
        }

        // Channel communication
        if visitor.has_channel_send || visitor.has_channel_recv {
            patterns.push(AsyncPattern {
                pattern_type: AsyncPatternType::ChannelCommunication,
                confidence: 0.85,
                evidence: "Uses channel communication".into(),
            });
        }

        // Mutex usage
        if visitor.has_mutex || visitor.has_rwlock {
            patterns.push(AsyncPattern {
                pattern_type: AsyncPatternType::MutexUsage,
                confidence: 0.85,
                evidence: format!(
                    "Uses synchronization: Mutex={}, RwLock={}",
                    visitor.has_mutex, visitor.has_rwlock
                ),
            });
        }

        patterns
    }

    pub fn classify_from_async_patterns(
        &self,
        patterns: &[AsyncPattern],
    ) -> Option<ResponsibilityCategory> {
        if patterns.is_empty() {
            return None;
        }

        // Task spawning = Orchestration
        if patterns
            .iter()
            .any(|p| p.pattern_type == AsyncPatternType::TaskSpawning)
        {
            return Some(ResponsibilityCategory::Orchestration);
        }

        // Async function with channel = Coordination
        if patterns
            .iter()
            .any(|p| p.pattern_type == AsyncPatternType::ChannelCommunication)
        {
            return Some(ResponsibilityCategory::Coordination);
        }

        // Just async = Orchestration
        if patterns
            .iter()
            .any(|p| p.pattern_type == AsyncPatternType::AsyncFunction)
        {
            return Some(ResponsibilityCategory::Orchestration);
        }

        None
    }
}

impl Default for RustAsyncDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn create_test_context(code: &str) -> RustFunctionContext<'static> {
        let item_fn: &'static syn::ItemFn = Box::leak(Box::new(syn::parse_str(code).unwrap()));
        let file_path: &'static Path = Path::new("test.rs");

        RustFunctionContext {
            item_fn,
            metrics: None,
            impl_context: None,
            file_path,
        }
    }

    #[test]
    fn test_detect_async_function() {
        let detector = RustAsyncDetector::new();
        let code = r#"
            async fn async_function() {
                println!("test");
            }
        "#;
        let context = create_test_context(code);

        let patterns = detector.detect_async_patterns(&context);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, AsyncPatternType::AsyncFunction);
        assert_eq!(patterns[0].confidence, 1.0);
    }

    #[test]
    fn test_detect_tokio_spawn() {
        let detector = RustAsyncDetector::new();
        let code = r#"
            async fn spawn_task() {
                tokio::spawn(async {
                    println!("task");
                });
            }
        "#;
        let context = create_test_context(code);

        let patterns = detector.detect_async_patterns(&context);
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == AsyncPatternType::TaskSpawning));
    }

    #[test]
    fn test_detect_channel_communication() {
        let detector = RustAsyncDetector::new();
        let code = r#"
            fn use_channel(sender: Sender<i32>) {
                sender.send(42);
            }
        "#;
        let context = create_test_context(code);

        let patterns = detector.detect_async_patterns(&context);
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == AsyncPatternType::ChannelCommunication));
    }

    #[test]
    fn test_detect_mutex_usage() {
        let detector = RustAsyncDetector::new();
        let code = r#"
            fn use_mutex() {
                let mutex = std::sync::Mutex::new(42);
                let guard = mutex.lock();
            }
        "#;
        let context = create_test_context(code);

        let patterns = detector.detect_async_patterns(&context);
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == AsyncPatternType::MutexUsage));
    }

    #[test]
    fn test_classify_task_spawning() {
        let detector = RustAsyncDetector::new();
        let patterns = vec![AsyncPattern {
            pattern_type: AsyncPatternType::TaskSpawning,
            confidence: 0.9,
            evidence: "Spawns tasks".into(),
        }];

        let category = detector.classify_from_async_patterns(&patterns);
        assert_eq!(category, Some(ResponsibilityCategory::Orchestration));
    }

    #[test]
    fn test_classify_channel_communication() {
        let detector = RustAsyncDetector::new();
        let patterns = vec![AsyncPattern {
            pattern_type: AsyncPatternType::ChannelCommunication,
            confidence: 0.85,
            evidence: "Uses channels".into(),
        }];

        let category = detector.classify_from_async_patterns(&patterns);
        assert_eq!(category, Some(ResponsibilityCategory::Coordination));
    }

    #[test]
    fn test_classify_empty_patterns() {
        let detector = RustAsyncDetector::new();
        let patterns = vec![];

        let category = detector.classify_from_async_patterns(&patterns);
        assert_eq!(category, None);
    }
}
