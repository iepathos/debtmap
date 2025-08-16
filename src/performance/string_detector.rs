use super::{PerformanceAntiPattern, PerformanceDetector, PerformanceImpact, StringAntiPattern};
use std::path::Path;
use syn::visit::{self, Visit};
use syn::{
    BinOp, Expr, ExprBinary, ExprCall, ExprForLoop, ExprLoop, ExprMethodCall, ExprWhile, File,
};

pub struct StringPerformanceDetector {}

impl StringPerformanceDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for StringPerformanceDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceDetector for StringPerformanceDetector {
    fn detect_anti_patterns(&self, file: &File, _path: &Path) -> Vec<PerformanceAntiPattern> {
        let mut visitor = StringVisitor {
            patterns: Vec::new(),
            in_loop: false,
            loop_depth: 0,
        };

        visitor.visit_file(file);
        visitor.patterns
    }

    fn detector_name(&self) -> &'static str {
        "StringPerformanceDetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::StringProcessingAntiPattern {
                pattern_type,
                performance_impact,
                ..
            } => match pattern_type {
                StringAntiPattern::ConcatenationInLoop => *performance_impact,
                StringAntiPattern::RegexInLoop => PerformanceImpact::High,
                StringAntiPattern::RepeatedFormatting => PerformanceImpact::Medium,
                StringAntiPattern::InefficientParsing => PerformanceImpact::Medium,
            },
            _ => PerformanceImpact::Low,
        }
    }
}

struct StringVisitor {
    patterns: Vec<PerformanceAntiPattern>,
    in_loop: bool,
    loop_depth: usize,
}

impl StringVisitor {
    fn check_string_concatenation(&mut self, binary: &ExprBinary) {
        if !self.in_loop {
            return;
        }

        if matches!(binary.op, BinOp::Add(_)) {
            // Check if this looks like string concatenation
            if self.is_string_type(&binary.left) || self.is_string_type(&binary.right) {
                self.patterns
                    .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                        pattern_type: StringAntiPattern::ConcatenationInLoop,
                        performance_impact: if self.loop_depth > 1 {
                            PerformanceImpact::Critical
                        } else {
                            PerformanceImpact::High
                        },
                        recommended_approach:
                            "Use String::with_capacity() and push_str() for better performance"
                                .to_string(),
                    });
            }
        }
    }

    fn check_format_in_loop(&mut self, mac: &syn::ExprMacro) {
        if !self.in_loop {
            return;
        }

        let macro_name = mac
            .mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();

        if macro_name == "format" || macro_name == "write" || macro_name == "writeln" {
            self.patterns
                .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                    pattern_type: StringAntiPattern::RepeatedFormatting,
                    performance_impact: PerformanceImpact::Medium,
                    recommended_approach:
                        "Pre-allocate String with capacity and use write! macro or push_str"
                            .to_string(),
                });
        }
    }

    fn check_regex_compilation(&mut self, call: &ExprCall) {
        if !self.in_loop {
            return;
        }

        if let Expr::Path(path) = &*call.func {
            let path_str = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            if path_str.contains("Regex::new") || path_str.contains("RegexBuilder::new") {
                self.patterns
                    .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                        pattern_type: StringAntiPattern::RegexInLoop,
                        performance_impact: PerformanceImpact::High,
                        recommended_approach:
                            "Compile regex once outside the loop using lazy_static or OnceCell"
                                .to_string(),
                    });
            }
        }
    }

    fn check_string_methods(&mut self, method_call: &ExprMethodCall) {
        if !self.in_loop {
            return;
        }

        let method_name = method_call.method.to_string();

        // Check for repeated parsing
        if method_name == "parse" {
            self.patterns
                .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                    pattern_type: StringAntiPattern::InefficientParsing,
                    performance_impact: PerformanceImpact::Medium,
                    recommended_approach:
                        "Consider caching parsed values or using more efficient parsing methods"
                            .to_string(),
                });
        }

        // Check for inefficient string building
        if method_name == "push_str" || method_name == "push" {
            // Check if receiver is a mutable string being built inefficiently
            if self.is_inefficient_string_building(&method_call.receiver) {
                self.patterns
                    .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                        pattern_type: StringAntiPattern::ConcatenationInLoop,
                        performance_impact: PerformanceImpact::Medium,
                        recommended_approach:
                            "Ensure String is pre-allocated with sufficient capacity".to_string(),
                    });
            }
        }

        // Check for repeated to_string() calls
        if method_name == "to_string" || method_name == "to_owned" {
            self.patterns
                .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                    pattern_type: StringAntiPattern::RepeatedFormatting,
                    performance_impact: PerformanceImpact::Low,
                    recommended_approach:
                        "Consider using string slices (&str) or caching converted strings"
                            .to_string(),
                });
        }
    }

    fn is_string_type(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Lit(lit) => matches!(lit.lit, syn::Lit::Str(_)),
            Expr::Path(path) => {
                let ident = path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                ident.contains("String") || ident.contains("string") || ident.contains("str")
            }
            Expr::MethodCall(call) => {
                matches!(
                    call.method.to_string().as_str(),
                    "to_string" | "to_owned" | "as_str" | "clone"
                )
            }
            Expr::Reference(r) => self.is_string_type(&r.expr),
            _ => false,
        }
    }

    fn is_inefficient_string_building(&self, _expr: &Expr) -> bool {
        // Simplified check - in real implementation would track if String::with_capacity was used
        false
    }
}

impl<'ast> Visit<'ast> for StringVisitor {
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

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        self.check_string_concatenation(node);
        visit::visit_expr_binary(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        self.check_format_in_loop(node);
        visit::visit_expr_macro(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        self.check_regex_compilation(node);
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        self.check_string_methods(node);
        visit::visit_expr_method_call(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let detector = StringPerformanceDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        let concat_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::StringProcessingAntiPattern {
                    pattern_type: StringAntiPattern::ConcatenationInLoop,
                    ..
                }
            )
        });
        assert!(concat_pattern.is_some());
    }

    #[test]
    fn test_format_in_loop() {
        let source = r#"
            fn format_items(items: &[i32]) -> Vec<String> {
                let mut results = Vec::new();
                for item in items {
                    let s = format!("Item: {}", item);
                    results.push(s);
                }
                results
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = StringPerformanceDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        let format_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::StringProcessingAntiPattern {
                    pattern_type: StringAntiPattern::RepeatedFormatting,
                    ..
                }
            )
        });
        assert!(format_pattern.is_some());
    }

    #[test]
    fn test_regex_in_loop() {
        let source = r#"
            use regex::Regex;
            
            fn validate_items(items: &[String]) -> Vec<bool> {
                let mut results = Vec::new();
                for item in items {
                    let re = Regex::new(r"^\d{3}-\d{4}$").unwrap();
                    results.push(re.is_match(item));
                }
                results
            }
        "#;

        let file = syn::parse_str::<File>(source).unwrap();
        let detector = StringPerformanceDetector::new();
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        let regex_pattern = patterns.iter().find(|p| {
            matches!(
                p,
                PerformanceAntiPattern::StringProcessingAntiPattern {
                    pattern_type: StringAntiPattern::RegexInLoop,
                    ..
                }
            )
        });
        assert!(regex_pattern.is_some());
    }
}
