use super::{
    AllocationFrequency, AllocationType, PerformanceAntiPattern, PerformanceDetector,
    PerformanceImpact,
};
use std::path::Path;
use syn::visit::{self, Visit};
use syn::{BinOp, Expr, ExprBinary, ExprForLoop, ExprLoop, ExprMethodCall, ExprWhile, File};

pub struct AllocationDetector {}

impl AllocationDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AllocationDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceDetector for AllocationDetector {
    fn detect_anti_patterns(&self, file: &File, _path: &Path) -> Vec<PerformanceAntiPattern> {
        let mut visitor = AllocationVisitor {
            patterns: Vec::new(),
            in_loop: false,
            loop_depth: 0,
            in_recursive_fn: false,
        };

        visitor.visit_file(file);
        visitor.patterns
    }

    fn detector_name(&self) -> &'static str {
        "AllocationDetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::ExcessiveAllocation {
                allocation_type,
                frequency,
                ..
            } => match (allocation_type, frequency) {
                (_, AllocationFrequency::InLoop) => {
                    if matches!(allocation_type, AllocationType::StringConcatenation) {
                        PerformanceImpact::High
                    } else {
                        PerformanceImpact::Medium
                    }
                }
                (AllocationType::Clone, AllocationFrequency::InHotPath) => {
                    PerformanceImpact::Medium
                }
                (AllocationType::LargeStackAllocation, _) => PerformanceImpact::High,
                (_, AllocationFrequency::Recursive) => PerformanceImpact::High,
                _ => PerformanceImpact::Low,
            },
            _ => PerformanceImpact::Low,
        }
    }
}

struct AllocationVisitor {
    patterns: Vec<PerformanceAntiPattern>,
    in_loop: bool,
    loop_depth: usize,
    in_recursive_fn: bool,
}

impl AllocationVisitor {
    fn check_clone(&mut self, method_call: &ExprMethodCall) {
        if method_call.method == "clone" || method_call.method == "to_owned" {
            let frequency = if self.in_loop {
                AllocationFrequency::InLoop
            } else if self.in_recursive_fn {
                AllocationFrequency::Recursive
            } else {
                AllocationFrequency::Occasional
            };

            if !matches!(frequency, AllocationFrequency::Occasional) {
                let suggestion = if self.in_loop {
                    "Move clone outside loop or use references/Cow<>"
                } else if self.in_recursive_fn {
                    "Consider using references or Arc for shared data"
                } else {
                    "Consider borrowing instead of cloning"
                };

                self.patterns
                    .push(PerformanceAntiPattern::ExcessiveAllocation {
                        allocation_type: AllocationType::Clone,
                        frequency,
                        suggested_optimization: suggestion.to_string(),
                    });
            }
        }

        // Check for collect() which allocates
        if method_call.method == "collect" && self.in_loop {
            self.patterns
                .push(PerformanceAntiPattern::ExcessiveAllocation {
                    allocation_type: AllocationType::TemporaryCollection,
                    frequency: AllocationFrequency::InLoop,
                    suggested_optimization:
                        "Consider using iterators directly or pre-allocating collections"
                            .to_string(),
                });
        }

        // Check for to_string() in loops
        if (method_call.method == "to_string" || method_call.method == "to_owned") && self.in_loop {
            self.patterns
                .push(PerformanceAntiPattern::ExcessiveAllocation {
                    allocation_type: AllocationType::StringConcatenation,
                    frequency: AllocationFrequency::InLoop,
                    suggested_optimization: "Consider using &str or String::with_capacity()"
                        .to_string(),
                });
        }
    }

    fn check_string_concatenation(&mut self, binary: &ExprBinary) {
        if matches!(binary.op, BinOp::Add(_)) && self.in_loop {
            // Check if either operand looks like a string
            if self.looks_like_string(&binary.left) || self.looks_like_string(&binary.right) {
                self.patterns
                    .push(PerformanceAntiPattern::ExcessiveAllocation {
                        allocation_type: AllocationType::StringConcatenation,
                        frequency: AllocationFrequency::InLoop,
                        suggested_optimization:
                            "Use String::with_capacity() and push_str() instead of + operator"
                                .to_string(),
                    });
            }
        }
    }

    fn looks_like_string(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Lit(lit) => matches!(lit.lit, syn::Lit::Str(_)),
            Expr::Path(path) => {
                let path_str = path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                path_str.contains("string")
                    || path_str.contains("String")
                    || path_str.contains("str")
            }
            Expr::MethodCall(call) => {
                call.method == "to_string" || call.method == "as_str" || call.method == "to_owned"
            }
            _ => false,
        }
    }

    fn check_vec_allocation(&mut self, expr: &Expr) {
        if self.in_loop {
            // Check for Vec::new() in loops
            if let Expr::Call(call) = expr {
                if let Expr::Path(path) = &*call.func {
                    let path_str = path
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");

                    if path_str.contains("Vec::new") || path_str.contains("HashMap::new") {
                        self.patterns
                            .push(PerformanceAntiPattern::ExcessiveAllocation {
                            allocation_type: AllocationType::TemporaryCollection,
                            frequency: AllocationFrequency::InLoop,
                            suggested_optimization:
                                "Pre-allocate collections outside the loop or use with_capacity()"
                                    .to_string(),
                        });
                    }
                }
            }

            // Check for vec![] macro in loops
            if let Expr::Macro(mac) = expr {
                if mac.mac.path.segments.last().map(|s| s.ident.to_string())
                    == Some("vec".to_string())
                {
                    self.patterns
                        .push(PerformanceAntiPattern::ExcessiveAllocation {
                            allocation_type: AllocationType::TemporaryCollection,
                            frequency: AllocationFrequency::InLoop,
                            suggested_optimization:
                                "Pre-allocate Vec outside the loop and clear/reuse it".to_string(),
                        });
                }
            }
        }
    }

    fn check_box_allocation(&mut self, expr: &Expr) {
        if let Expr::Call(call) = expr {
            if let Expr::Path(path) = &*call.func {
                let path_str = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");

                if path_str.contains("Box::new") && self.in_loop {
                    self.patterns
                        .push(PerformanceAntiPattern::ExcessiveAllocation {
                            allocation_type: AllocationType::RepeatedBoxing,
                            frequency: AllocationFrequency::InLoop,
                            suggested_optimization:
                                "Consider object pooling or pre-allocation strategies".to_string(),
                        });
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for AllocationVisitor {
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
        self.check_clone(node);
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        self.check_string_concatenation(node);
        visit::visit_expr_binary(self, node);
    }

    fn visit_expr(&mut self, node: &'ast Expr) {
        self.check_vec_allocation(node);
        self.check_box_allocation(node);
        visit::visit_expr(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone_in_loop() {
        let source = r#"
            fn clone_in_loop(items: &[String]) -> Vec<String> {
                let mut result = Vec::new();
                for item in items {
                    result.push(item.clone());
                }
                result
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = AllocationDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        let clone_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::ExcessiveAllocation {
                    allocation_type: AllocationType::Clone,
                    frequency: AllocationFrequency::InLoop,
                    ..
                }
            )
        });
        assert!(clone_pattern.is_some());
    }

    #[test]
    fn test_string_concatenation_in_loop() {
        let source = r#"
            fn build_string(items: &[&str]) -> String {
                let mut result = String::new();
                for item in items {
                    result = result + item + ",";
                }
                result
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = AllocationDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        let string_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::ExcessiveAllocation {
                    allocation_type: AllocationType::StringConcatenation,
                    ..
                }
            )
        });
        assert!(string_pattern.is_some());
    }

    #[test]
    fn test_vec_new_in_loop() {
        let source = r#"
            fn vec_in_loop(n: usize) {
                for i in 0..n {
                    let temp = Vec::new();
                    // do something with temp
                }
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = AllocationDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        let vec_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::ExcessiveAllocation {
                    allocation_type: AllocationType::TemporaryCollection,
                    ..
                }
            )
        });
        assert!(vec_pattern.is_some());
    }
}
