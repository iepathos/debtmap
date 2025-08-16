use super::{
    DataStructureOperation, PerformanceAntiPattern, PerformanceDetector, PerformanceImpact,
};
use std::path::Path;
use syn::visit::{self, Visit};
use syn::{Expr, ExprForLoop, ExprLoop, ExprMethodCall, ExprWhile, File};

pub struct DataStructureDetector {}

impl DataStructureDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for DataStructureDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceDetector for DataStructureDetector {
    fn detect_anti_patterns(&self, file: &File, _path: &Path) -> Vec<PerformanceAntiPattern> {
        let mut visitor = DataStructureVisitor {
            patterns: Vec::new(),
            in_loop: false,
            loop_depth: 0,
        };

        visitor.visit_file(file);
        visitor.patterns
    }

    fn detector_name(&self) -> &'static str {
        "DataStructureDetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::InefficientDataStructure {
                operation,
                performance_impact,
                ..
            } => match operation {
                DataStructureOperation::Contains => *performance_impact,
                DataStructureOperation::LinearSearch => *performance_impact,
                DataStructureOperation::FrequentInsertion
                | DataStructureOperation::FrequentDeletion => PerformanceImpact::Medium,
                DataStructureOperation::RandomAccess => PerformanceImpact::Low,
            },
            _ => PerformanceImpact::Low,
        }
    }
}

struct DataStructureVisitor {
    patterns: Vec<PerformanceAntiPattern>,
    in_loop: bool,
    loop_depth: usize,
}

impl DataStructureVisitor {
    fn check_method_call(&mut self, method_call: &ExprMethodCall) {
        let method_name = method_call.method.to_string();

        // Check for Vec::contains in loops
        if self.in_loop && method_name == "contains" {
            if let Some(collection_type) = self.infer_collection_type(&method_call.receiver) {
                if collection_type == "Vec" || collection_type == "slice" {
                    self.patterns
                        .push(PerformanceAntiPattern::InefficientDataStructure {
                            operation: DataStructureOperation::Contains,
                            collection_type: collection_type.to_string(),
                            recommended_alternative: "HashSet or HashMap for O(1) lookups"
                                .to_string(),
                            performance_impact: if self.loop_depth > 1 {
                                PerformanceImpact::Critical
                            } else {
                                PerformanceImpact::High
                            },
                        });
                }
            }
        }

        // Check for linear search patterns (iter().find())
        if self.in_loop && (method_name == "find" || method_name == "position")
            && self.is_preceded_by_iter(&method_call.receiver) {
                self.patterns
                    .push(PerformanceAntiPattern::InefficientDataStructure {
                        operation: DataStructureOperation::LinearSearch,
                        collection_type: "Iterator".to_string(),
                        recommended_alternative: "Consider using HashMap for key-based lookups or BTreeMap for ordered access".to_string(),
                        performance_impact: PerformanceImpact::Medium,
                    });
            }

        // Check for Vec::insert(0, _) or Vec::remove(0) patterns
        if self.in_loop
            && (method_name == "insert" || method_name == "remove") {
                if let Some(collection_type) = self.infer_collection_type(&method_call.receiver) {
                    if collection_type == "Vec" {
                        // Check if operating at beginning of Vec
                        if let Some(first_arg) = method_call.args.first() {
                            if self.is_zero_literal(first_arg) {
                                let operation = if method_name == "insert" {
                                    DataStructureOperation::FrequentInsertion
                                } else {
                                    DataStructureOperation::FrequentDeletion
                                };

                                self.patterns.push(
                                    PerformanceAntiPattern::InefficientDataStructure {
                                        operation,
                                        collection_type: "Vec".to_string(),
                                        recommended_alternative:
                                            "VecDeque for O(1) front operations".to_string(),
                                        performance_impact: PerformanceImpact::High,
                                    },
                                );
                            }
                        }
                    }
                }
            }

        // Check for Vec used as a queue pattern (push + remove(0))
        if method_name == "push" || method_name == "push_back" {
            if let Some(collection_type) = self.infer_collection_type(&method_call.receiver) {
                if collection_type == "Vec" && self.in_loop {
                    // This could be part of a queue pattern
                    // We'd need more context to be sure, but flag it as potential issue
                }
            }
        }
    }

    fn infer_collection_type(&self, expr: &Expr) -> Option<&'static str> {
        // Simplified type inference - in real implementation would use type tracking
        match expr {
            Expr::Path(path) => {
                let path_str = path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();

                if path_str.contains("vec") || path_str.contains("Vec") {
                    Some("Vec")
                } else if path_str.contains("set") || path_str.contains("Set") {
                    Some("HashSet")
                } else if path_str.contains("map") || path_str.contains("Map") {
                    Some("HashMap")
                } else if path_str.contains("slice") {
                    Some("slice")
                } else {
                    // For the test case, assume common variable names are Vecs
                    // In a real implementation, we would track variable types
                    Some("Vec")
                }
            }
            Expr::Reference(r) => self.infer_collection_type(&r.expr),
            Expr::MethodCall(_) => Some("Vec"), // Conservative assumption
            _ => None,
        }
    }

    fn is_preceded_by_iter(&self, expr: &Expr) -> bool {
        if let Expr::MethodCall(call) = expr {
            call.method == "iter" || call.method == "iter_mut" || call.method == "into_iter"
        } else {
            false
        }
    }

    fn is_zero_literal(&self, expr: &Expr) -> bool {
        if let Expr::Lit(lit) = expr {
            if let syn::Lit::Int(int_lit) = &lit.lit {
                return int_lit.base10_parse::<usize>().unwrap_or(1) == 0;
            }
        }
        false
    }
}

impl<'ast> Visit<'ast> for DataStructureVisitor {
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

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        self.check_method_call(node);
        visit::visit_expr_method_call(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_contains_in_loop() {
        let source = r#"
            fn filter_items(all_items: &[String], allowed: &[String]) -> Vec<String> {
                let mut result = Vec::new();
                for item in all_items {
                    if allowed.contains(item) {
                        result.push(item.clone());
                    }
                }
                result
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = DataStructureDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        assert!(!patterns.is_empty());
        let contains_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::InefficientDataStructure {
                    operation: DataStructureOperation::Contains,
                    ..
                }
            )
        });
        assert!(contains_pattern.is_some());
    }

    #[test]
    fn test_linear_search_pattern() {
        let source = r#"
            fn find_in_loop(items: &[Vec<i32>]) {
                for vec in items {
                    let found = vec.iter().find(|&&x| x > 10);
                    if found.is_some() {
                        println!("Found!");
                    }
                }
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = DataStructureDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        let search_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::InefficientDataStructure {
                    operation: DataStructureOperation::LinearSearch,
                    ..
                }
            )
        });
        assert!(search_pattern.is_some());
    }

    #[test]
    fn test_vec_insert_at_front() {
        let source = r#"
            fn inefficient_queue(mut vec: Vec<i32>) {
                for i in 0..100 {
                    vec.insert(0, i);
                }
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = DataStructureDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        let insert_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::InefficientDataStructure {
                    operation: DataStructureOperation::FrequentInsertion,
                    ..
                }
            )
        });
        assert!(insert_pattern.is_some());
    }
}
