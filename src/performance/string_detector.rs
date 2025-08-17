use super::{LocationConfidence, LocationExtractor, PerformanceAntiPattern, PerformanceDetector, PerformanceImpact, SourceLocation, StringAntiPattern};
use std::path::Path;
use syn::visit::{self, Visit};
use syn::{
    BinOp, Expr, ExprBinary, ExprCall, ExprForLoop, ExprLoop, ExprMethodCall, ExprWhile, File,
};

pub struct StringPerformanceDetector {
    location_extractor: Option<LocationExtractor>,
}

impl StringPerformanceDetector {
    pub fn new() -> Self {
        Self {
            location_extractor: None,
        }
    }
    
    pub fn with_source_content(source_content: &str) -> Self {
        Self {
            location_extractor: Some(LocationExtractor::new(source_content)),
        }
    }
}

impl Default for StringPerformanceDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceDetector for StringPerformanceDetector {
    fn detect_anti_patterns(&self, file: &File, path: &Path) -> Vec<PerformanceAntiPattern> {
        // If no location extractor, try to read source file for location extraction
        let temp_extractor;
        let location_extractor = if let Some(ref extractor) = self.location_extractor {
            Some(extractor)
        } else {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    temp_extractor = LocationExtractor::new(&content);
                    Some(&temp_extractor)
                }
                Err(_) => None,
            }
        };
            
        let mut visitor = StringVisitor {
            patterns: Vec::new(),
            in_loop: false,
            loop_depth: 0,
            location_extractor,
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

struct StringVisitor<'a> {
    patterns: Vec<PerformanceAntiPattern>,
    in_loop: bool,
    loop_depth: usize,
    location_extractor: Option<&'a LocationExtractor>,
}

impl<'a> StringVisitor<'a> {
    fn extract_location(&self, expr: &Expr) -> SourceLocation {
        if let Some(extractor) = self.location_extractor {
            extractor.extract_expr_location(expr)
        } else {
            // Fallback when no source content available
            SourceLocation {
                line: 1,
                column: None,
                end_line: None,
                end_column: None,
                confidence: LocationConfidence::Unavailable,
            }
        }
    }

    fn check_string_concatenation(&mut self, binary: &ExprBinary) {
        if !self.in_loop {
            return;
        }

        if matches!(binary.op, BinOp::Add(_)) {
            // Check if this looks like string concatenation
            if self.is_string_type(&binary.left) || self.is_string_type(&binary.right) {
                let location = self.extract_location(&Expr::Binary(binary.clone()));
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
                        location,
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
            let location = self.extract_location(&Expr::Macro(mac.clone()));
            self.patterns
                .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                    pattern_type: StringAntiPattern::RepeatedFormatting,
                    performance_impact: PerformanceImpact::Medium,
                    recommended_approach:
                        "Pre-allocate String with capacity and use write! macro or push_str"
                            .to_string(),
                    location,
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
                let location = self.extract_location(&Expr::Call(call.clone()));
                self.patterns
                    .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                        pattern_type: StringAntiPattern::RegexInLoop,
                        performance_impact: PerformanceImpact::High,
                        recommended_approach:
                            "Compile regex once outside the loop using lazy_static or OnceCell"
                                .to_string(),
                        location,
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
            let location = self.extract_location(&Expr::MethodCall(method_call.clone()));
            self.patterns
                .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                    pattern_type: StringAntiPattern::InefficientParsing,
                    performance_impact: PerformanceImpact::Medium,
                    recommended_approach:
                        "Consider caching parsed values or using more efficient parsing methods"
                            .to_string(),
                    location,
                });
        }

        // Check for inefficient string building
        if method_name == "push_str" || method_name == "push" {
            // Check if receiver is a mutable string being built inefficiently
            if self.is_inefficient_string_building(&method_call.receiver) {
                let location = self.extract_location(&Expr::MethodCall(method_call.clone()));
                self.patterns
                    .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                        pattern_type: StringAntiPattern::ConcatenationInLoop,
                        performance_impact: PerformanceImpact::Medium,
                        recommended_approach:
                            "Ensure String is pre-allocated with sufficient capacity".to_string(),
                        location,
                    });
            }
        }

        // Check for repeated to_string() calls
        if method_name == "to_string" || method_name == "to_owned" {
            let location = self.extract_location(&Expr::MethodCall(method_call.clone()));
            self.patterns
                .push(PerformanceAntiPattern::StringProcessingAntiPattern {
                    pattern_type: StringAntiPattern::RepeatedFormatting,
                    performance_impact: PerformanceImpact::Low,
                    recommended_approach:
                        "Consider using string slices (&str) or caching converted strings"
                            .to_string(),
                    location,
                });
        }
    }

    #[allow(clippy::only_used_in_recursion)]
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

impl<'ast, 'a> Visit<'ast> for StringVisitor<'a> {
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
