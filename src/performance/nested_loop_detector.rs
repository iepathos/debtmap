use super::{
    ComplexityClass, LocationConfidence, LoopOperation, PerformanceAntiPattern,
    PerformanceDetector, PerformanceImpact, SourceLocation,
};
use std::path::Path;
use syn::visit::{self, Visit};
use syn::{Block, Expr, ExprForLoop, ExprLoop, ExprWhile, File};

pub struct NestedLoopDetector {
    max_acceptable_nesting: u32,
}

impl NestedLoopDetector {
    pub fn new() -> Self {
        Self {
            max_acceptable_nesting: 2,
        }
    }

    pub fn with_max_nesting(max_nesting: u32) -> Self {
        Self {
            max_acceptable_nesting: max_nesting,
        }
    }

    pub fn with_source_content(_source_content: &str) -> Self {
        Self {
            max_acceptable_nesting: 2,
        }
    }
}

impl Default for NestedLoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceDetector for NestedLoopDetector {
    fn detect_anti_patterns(&self, file: &File, _path: &Path) -> Vec<PerformanceAntiPattern> {
        let mut visitor = NestedLoopVisitor {
            patterns: Vec::new(),
            current_nesting: 0,
            max_nesting_seen: 0,
            loop_stack: Vec::new(),
            max_acceptable_nesting: self.max_acceptable_nesting,
            deepest_violation_depth: None,
            violation_operations: Vec::new(),
        };

        visitor.visit_file(file);
        visitor.patterns
    }

    fn detector_name(&self) -> &'static str {
        "NestedLoopDetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::NestedLoop {
                estimated_complexity,
                inner_operations,
                ..
            } => {
                let has_expensive_ops = inner_operations.iter().any(|op| {
                    matches!(
                        op,
                        LoopOperation::DatabaseQuery
                            | LoopOperation::NetworkRequest
                            | LoopOperation::FileIO
                    )
                });

                match estimated_complexity {
                    ComplexityClass::Exponential => PerformanceImpact::Critical,
                    ComplexityClass::Cubic => {
                        if has_expensive_ops {
                            PerformanceImpact::Critical
                        } else {
                            PerformanceImpact::High
                        }
                    }
                    ComplexityClass::Quadratic => {
                        if has_expensive_ops {
                            PerformanceImpact::High
                        } else {
                            PerformanceImpact::Medium
                        }
                    }
                    _ => PerformanceImpact::Low,
                }
            }
            _ => PerformanceImpact::Low,
        }
    }
}

struct NestedLoopVisitor {
    patterns: Vec<PerformanceAntiPattern>,
    current_nesting: u32,
    max_nesting_seen: u32,
    loop_stack: Vec<LoopInfo>,
    max_acceptable_nesting: u32,
    deepest_violation_depth: Option<u32>,
    violation_operations: Vec<LoopOperation>,
}

#[derive(Debug, Clone)]
struct LoopInfo {
    operations: Vec<LoopOperation>,
    #[allow(dead_code)]
    has_mutable_state: bool,
    #[allow(dead_code)]
    has_dependencies: bool,
}

impl NestedLoopVisitor {
    fn analyze_loop_operations(&self, block: &Block) -> Vec<LoopOperation> {
        let mut operations = Vec::new();
        let mut op_visitor = OperationVisitor {
            operations: Vec::new(),
        };

        for stmt in &block.stmts {
            op_visitor.visit_stmt(stmt);
        }

        operations.extend(op_visitor.operations);
        operations
    }

    fn estimate_complexity(
        &self,
        nesting_level: u32,
        operations: &[LoopOperation],
    ) -> ComplexityClass {
        let base_complexity = match nesting_level {
            1 => ComplexityClass::Linear,
            2 => ComplexityClass::Quadratic,
            3 => ComplexityClass::Cubic,
            _ => ComplexityClass::Exponential,
        };

        // Adjust based on expensive operations
        let has_expensive_operations = operations.iter().any(|op| {
            matches!(
                op,
                LoopOperation::DatabaseQuery
                    | LoopOperation::NetworkRequest
                    | LoopOperation::FileIO
            )
        });

        if has_expensive_operations && nesting_level > 1 {
            match base_complexity {
                ComplexityClass::Linear => ComplexityClass::Quadratic,
                ComplexityClass::Quadratic => ComplexityClass::Cubic,
                _ => ComplexityClass::Exponential,
            }
        } else {
            base_complexity
        }
    }

    #[allow(dead_code)]
    fn analyze_parallelization_potential(&self, block: &Block) -> bool {
        let mut dep_visitor = DependencyVisitor {
            has_mutable_state: false,
            has_dependencies: false,
            has_io: false,
        };

        for stmt in &block.stmts {
            dep_visitor.visit_stmt(stmt);
        }

        !dep_visitor.has_dependencies && !dep_visitor.has_mutable_state && !dep_visitor.has_io
    }

    fn enter_loop(&mut self, block: &Block) {
        self.current_nesting += 1;
        if self.current_nesting > self.max_nesting_seen {
            self.max_nesting_seen = self.current_nesting;
        }

        let operations = self.analyze_loop_operations(block);
        let has_mutable_state = false; // Simplified for now
        let has_dependencies = false; // Simplified for now

        self.loop_stack.push(LoopInfo {
            operations: operations.clone(),
            has_mutable_state,
            has_dependencies,
        });

        // Check if we've exceeded acceptable nesting
        if self.current_nesting >= self.max_acceptable_nesting {
            // Track that we've found a violation and potentially update the depth
            if self.deepest_violation_depth.is_none()
                || self.current_nesting > self.deepest_violation_depth.unwrap()
            {
                self.deepest_violation_depth = Some(self.current_nesting);

                // Collect all operations up to this point
                self.violation_operations.clear();
                for loop_info in &self.loop_stack {
                    self.violation_operations
                        .extend(loop_info.operations.clone());
                }
            }
        }
    }

    fn exit_loop(&mut self) {
        // If we're exiting from a violation depth back to acceptable levels, report it
        if let Some(violation_depth) = self.deepest_violation_depth {
            if self.current_nesting == self.max_acceptable_nesting
                && violation_depth >= self.max_acceptable_nesting
            {
                let complexity =
                    self.estimate_complexity(violation_depth, &self.violation_operations);
                let can_parallelize = false; // Simplified

                self.patterns.push(PerformanceAntiPattern::NestedLoop {
                    nesting_level: violation_depth,
                    estimated_complexity: complexity,
                    inner_operations: self.violation_operations.clone(),
                    can_parallelize,
                    location: SourceLocation {
                        line: 1,
                        column: None,
                        end_line: None,
                        end_column: None,
                        confidence: LocationConfidence::Unavailable,
                    },
                });

                // Reset for next potential violation
                self.deepest_violation_depth = None;
                self.violation_operations.clear();
            }
        }

        self.current_nesting -= 1;
        self.loop_stack.pop();
    }
}

impl<'ast> Visit<'ast> for NestedLoopVisitor {
    fn visit_expr_for_loop(&mut self, node: &'ast ExprForLoop) {
        self.enter_loop(&node.body);
        visit::visit_expr_for_loop(self, node);
        self.exit_loop();
    }

    fn visit_expr_while(&mut self, node: &'ast ExprWhile) {
        self.enter_loop(&node.body);
        visit::visit_expr_while(self, node);
        self.exit_loop();
    }

    fn visit_expr_loop(&mut self, node: &'ast ExprLoop) {
        self.enter_loop(&node.body);
        visit::visit_expr_loop(self, node);
        self.exit_loop();
    }
}

struct OperationVisitor {
    operations: Vec<LoopOperation>,
}

impl<'ast> Visit<'ast> for OperationVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let method_name = node.method.to_string();

        // Check for collection operations
        if matches!(
            method_name.as_str(),
            "contains" | "iter" | "into_iter" | "drain" | "retain"
        ) {
            self.operations.push(LoopOperation::CollectionIteration);
        }

        // Check for I/O operations
        if method_name.contains("read") || method_name.contains("write") {
            self.operations.push(LoopOperation::FileIO);
        }

        // Check for string operations
        if method_name == "push_str" || method_name == "push" {
            self.operations.push(LoopOperation::StringOperation);
        }

        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Expr::Path(path) = &*node.func {
            let path_str = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            // Check for file I/O
            if path_str.contains("fs::") || path_str.contains("File::") {
                self.operations.push(LoopOperation::FileIO);
            }

            // Check for network operations
            if path_str.contains("TcpStream") || path_str.contains("reqwest") {
                self.operations.push(LoopOperation::NetworkRequest);
            }

            // Check for database operations
            if path_str.contains("query") || path_str.contains("execute") {
                self.operations.push(LoopOperation::DatabaseQuery);
            }
        }

        visit::visit_expr_call(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        use syn::BinOp;

        if matches!(node.op, BinOp::Add(_)) {
            // Check if this is string concatenation
            self.operations.push(LoopOperation::StringOperation);
        }

        visit::visit_expr_binary(self, node);
    }
}

#[allow(dead_code)]
struct DependencyVisitor {
    has_mutable_state: bool,
    has_dependencies: bool,
    has_io: bool,
}

impl<'ast> Visit<'ast> for DependencyVisitor {
    fn visit_expr_assign(&mut self, _node: &'ast syn::ExprAssign) {
        self.has_mutable_state = true;
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Expr::Path(path) = &*node.func {
            let path_str = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            if path_str.contains("fs::") || path_str.contains("File::") {
                self.has_io = true;
            }
        }

        visit::visit_expr_call(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nested_loop_detection() {
        let source = r#"
            fn inefficient_search(items: &[Vec<i32>], target: i32) -> bool {
                for outer_vec in items {
                    for &item in outer_vec {
                        if item == target {
                            return true;
                        }
                    }
                }
                false
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = NestedLoopDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        assert_eq!(patterns.len(), 1);
        if let PerformanceAntiPattern::NestedLoop {
            nesting_level,
            estimated_complexity,
            ..
        } = &patterns[0]
        {
            assert_eq!(*nesting_level, 2);
            assert_eq!(*estimated_complexity, ComplexityClass::Quadratic);
        } else {
            panic!("Expected nested loop pattern");
        }
    }

    #[test]
    fn test_triple_nested_loop() {
        let source = r#"
            fn triple_nested(a: &[Vec<Vec<i32>>]) {
                for outer in a {
                    for middle in outer {
                        for &inner in middle {
                            println!("{}", inner);
                        }
                    }
                }
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = NestedLoopDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        assert!(!patterns.is_empty());
        if let Some(PerformanceAntiPattern::NestedLoop {
            nesting_level,
            estimated_complexity,
            ..
        }) = patterns.first()
        {
            assert_eq!(*nesting_level, 3);
            assert_eq!(*estimated_complexity, ComplexityClass::Cubic);
        }
    }
}
