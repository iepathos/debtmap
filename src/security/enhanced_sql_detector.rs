use crate::security::types::{
    SecurityDetector, SecurityVulnerability, Severity, SqlInjectionType, TaintSource,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, File, Local, Pat, PatIdent};

pub struct EnhancedSqlInjectionDetector {
    sql_keywords: HashSet<String>,
    dangerous_functions: HashSet<String>,
    safe_functions: HashSet<String>,
}

impl Default for EnhancedSqlInjectionDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl EnhancedSqlInjectionDetector {
    pub fn new() -> Self {
        let sql_keywords: HashSet<String> = vec![
            "SELECT", "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER", "FROM", "WHERE",
            "JOIN", "UNION", "ORDER BY", "GROUP BY", "HAVING", "LIMIT", "OFFSET", "SET", "VALUES",
            "INTO",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let dangerous_functions: HashSet<String> =
            ["format", "push_str", "concat", "join", "replace"]
                .iter()
                .map(|s| s.to_string())
                .collect();

        let safe_functions: HashSet<String> = [
            "bind",
            "prepare",
            "query_as",
            "query_scalar",
            "fetch_one",
            "fetch_all",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        Self {
            sql_keywords,
            dangerous_functions,
            safe_functions,
        }
    }

    fn contains_sql_keywords(&self, text: &str) -> bool {
        let text_upper = text.to_uppercase();
        self.sql_keywords
            .iter()
            .any(|keyword| text_upper.contains(keyword))
    }

    fn analyze_taint_source(&self, expr: &Expr) -> Option<TaintSource> {
        let expr_str = quote::quote!(#expr).to_string();
        Self::classify_taint_source(&expr_str)
    }

    /// Pure function to classify taint sources from expression strings
    fn classify_taint_source(expr_str: &str) -> Option<TaintSource> {
        // Pattern-based classification using functional approach
        match () {
            _ if expr_str.contains("args()") || expr_str.contains("env::args") => {
                Some(TaintSource::CliArgument)
            }
            _ if expr_str.contains("env::var") || expr_str.contains("std::env") => {
                Some(TaintSource::Environment)
            }
            _ if expr_str.contains("request") || expr_str.contains("req.") => {
                Some(TaintSource::HttpRequest)
            }
            _ if expr_str.contains("File::") || expr_str.contains("read_to_string") => {
                Some(TaintSource::FileInput)
            }
            _ if expr_str.contains("user_input") || expr_str.contains("input") => {
                Some(TaintSource::UserControlled)
            }
            _ => None,
        }
    }

    fn get_injection_type(&self, expr_str: &str) -> SqlInjectionType {
        if expr_str.contains("format!") || expr_str.contains("format(") {
            SqlInjectionType::FormatString
        } else if expr_str.contains("push_str") || expr_str.contains("+ &") {
            SqlInjectionType::StringConcatenation
        } else if expr_str.contains("query_dsl") || expr_str.contains("sql_query") {
            SqlInjectionType::DynamicQuery
        } else {
            SqlInjectionType::TemplateInjection
        }
    }

    fn calculate_severity(&self, has_taint: bool, injection_type: &SqlInjectionType) -> Severity {
        match (has_taint, injection_type) {
            (true, SqlInjectionType::FormatString) => Severity::Critical,
            (true, SqlInjectionType::StringConcatenation) => Severity::Critical,
            (true, _) => Severity::High,
            (false, SqlInjectionType::FormatString) => Severity::High,
            (false, SqlInjectionType::StringConcatenation) => Severity::High,
            (false, _) => Severity::Medium,
        }
    }
}

impl SecurityDetector for EnhancedSqlInjectionDetector {
    fn detect_vulnerabilities(&self, file: &File, path: &Path) -> Vec<SecurityVulnerability> {
        let mut visitor = SqlInjectionVisitor {
            vulnerabilities: Vec::new(),
            path: path.to_path_buf(),
            detector: self,
            tainted_variables: HashMap::new(),
        };
        visitor.visit_file(file);
        visitor.vulnerabilities
    }

    fn detector_name(&self) -> &'static str {
        "EnhancedSqlInjectionDetector"
    }
}

struct SqlInjectionVisitor<'a> {
    vulnerabilities: Vec<SecurityVulnerability>,
    path: std::path::PathBuf,
    detector: &'a EnhancedSqlInjectionDetector,
    tainted_variables: HashMap<String, TaintSource>,
}

impl<'a, 'ast> Visit<'ast> for SqlInjectionVisitor<'a> {
    fn visit_local(&mut self, local: &'ast Local) {
        // Track variable assignments that might be tainted
        if let Pat::Ident(PatIdent { ident, .. }) = &local.pat {
            if let Some(init) = &local.init {
                if let Some(taint_source) = self.detector.analyze_taint_source(&init.expr) {
                    self.tainted_variables
                        .insert(ident.to_string(), taint_source);
                }
            }
        }
        syn::visit::visit_local(self, local);
    }

    fn visit_expr_method_call(&mut self, method_call: &'ast ExprMethodCall) {
        let method_name = method_call.method.to_string();

        // Check for database query methods
        if method_name == "query" || method_name == "execute" || method_name == "raw_sql" {
            self.analyze_query_call(method_call);
        }

        // Check for string manipulation in SQL context
        if self.detector.dangerous_functions.contains(&method_name) {
            let expr_str = quote::quote!(#method_call).to_string();
            if self.detector.contains_sql_keywords(&expr_str) {
                self.analyze_string_manipulation(method_call);
            }
        }

        syn::visit::visit_expr_method_call(self, method_call);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        let expr_str = quote::quote!(#call).to_string();

        // Check for format! macro with SQL
        if expr_str.contains("format!") && self.detector.contains_sql_keywords(&expr_str) {
            self.analyze_format_macro(call);
        }

        // Check for sql! or query! macros
        if expr_str.contains("sql!") || expr_str.contains("query!") {
            self.analyze_sql_macro(call);
        }

        syn::visit::visit_expr_call(self, call);
    }
}

impl<'a> SqlInjectionVisitor<'a> {
    fn analyze_query_call(&mut self, method_call: &ExprMethodCall) {
        let expr_str = quote::quote!(#method_call).to_string();

        // Skip if using safe parameterized queries
        if self
            .detector
            .safe_functions
            .iter()
            .any(|safe| expr_str.contains(safe))
        {
            return;
        }

        // Check for dynamic query construction
        let has_concatenation =
            expr_str.contains('+') || expr_str.contains("push_str") || expr_str.contains("format!");
        if !has_concatenation {
            return;
        }

        // Check for tainted data
        let taint_source = self.find_taint_in_expr(&expr_str);
        let injection_type = self.detector.get_injection_type(&expr_str);
        let severity = self
            .detector
            .calculate_severity(taint_source.is_some(), &injection_type);

        self.vulnerabilities
            .push(SecurityVulnerability::SqlInjection {
                injection_type,
                taint_source,
                confidence: if taint_source.is_some() { 0.9 } else { 0.7 },
                severity,
                line: 0, // Would need span info
                file: self.path.clone(),
            });
    }

    fn analyze_format_macro(&mut self, call: &ExprCall) {
        let expr_str = quote::quote!(#call).to_string();

        // Check if format string contains SQL and variables
        if expr_str.contains("{}") || expr_str.contains("{:") {
            let taint_source = self.find_taint_in_expr(&expr_str);
            let severity = self
                .detector
                .calculate_severity(taint_source.is_some(), &SqlInjectionType::FormatString);

            self.vulnerabilities
                .push(SecurityVulnerability::SqlInjection {
                    injection_type: SqlInjectionType::FormatString,
                    taint_source,
                    confidence: 0.85,
                    severity,
                    line: 0,
                    file: self.path.clone(),
                });
        }
    }

    fn analyze_string_manipulation(&mut self, method_call: &ExprMethodCall) {
        let expr_str = quote::quote!(#method_call).to_string();
        let taint_source = self.find_taint_in_expr(&expr_str);
        let severity = self.detector.calculate_severity(
            taint_source.is_some(),
            &SqlInjectionType::StringConcatenation,
        );

        self.vulnerabilities
            .push(SecurityVulnerability::SqlInjection {
                injection_type: SqlInjectionType::StringConcatenation,
                taint_source,
                confidence: 0.75,
                severity,
                line: 0,
                file: self.path.clone(),
            });
    }

    fn analyze_sql_macro(&mut self, call: &ExprCall) {
        // Check if macro contains dynamic parts
        let expr_str = quote::quote!(#call).to_string();

        if expr_str.contains("$") || expr_str.contains("{}") {
            let taint_source = self.find_taint_in_expr(&expr_str);

            if taint_source.is_some() {
                self.vulnerabilities
                    .push(SecurityVulnerability::SqlInjection {
                        injection_type: SqlInjectionType::TemplateInjection,
                        taint_source,
                        confidence: 0.8,
                        severity: Severity::High,
                        line: 0,
                        file: self.path.clone(),
                    });
            }
        }
    }

    fn find_taint_in_expr(&self, expr_str: &str) -> Option<TaintSource> {
        // Check if any tainted variables are used
        for (var_name, taint_source) in &self.tainted_variables {
            if expr_str.contains(var_name) {
                return Some(*taint_source);
            }
        }

        // Check for direct taint sources
        self.detector
            .analyze_taint_source(&syn::parse_str::<Expr>(expr_str).ok()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_keyword_detection() {
        let detector = EnhancedSqlInjectionDetector::new();

        assert!(detector.contains_sql_keywords("SELECT * FROM users"));
        assert!(detector.contains_sql_keywords("insert into table"));
        assert!(!detector.contains_sql_keywords("let result = 5"));
    }

    #[test]
    fn test_severity_calculation() {
        let detector = EnhancedSqlInjectionDetector::new();

        // Critical: tainted format string
        assert_eq!(
            detector.calculate_severity(true, &SqlInjectionType::FormatString),
            Severity::Critical
        );

        // High: untainted format string
        assert_eq!(
            detector.calculate_severity(false, &SqlInjectionType::FormatString),
            Severity::High
        );

        // Medium: untainted template
        assert_eq!(
            detector.calculate_severity(false, &SqlInjectionType::TemplateInjection),
            Severity::Medium
        );
    }

    #[test]
    fn test_classify_taint_source_cli_arguments() {
        // Test CLI argument detection
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("env::args()"),
            Some(TaintSource::CliArgument)
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("std::env::args"),
            Some(TaintSource::CliArgument)
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("let args = args()"),
            Some(TaintSource::CliArgument)
        );
    }

    #[test]
    fn test_classify_taint_source_environment() {
        // Test environment variable detection
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("env::var(\"HOME\")"),
            Some(TaintSource::Environment)
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("std::env::var"),
            Some(TaintSource::Environment)
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("use std::env"),
            Some(TaintSource::Environment)
        );
    }

    #[test]
    fn test_classify_taint_source_http_request() {
        // Test HTTP request detection
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("request.body()"),
            Some(TaintSource::HttpRequest)
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("req.params"),
            Some(TaintSource::HttpRequest)
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("http_request"),
            Some(TaintSource::HttpRequest)
        );
    }

    #[test]
    fn test_classify_taint_source_file_input() {
        // Test file input detection
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("File::open(path)"),
            Some(TaintSource::FileInput)
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("fs::read_to_string"),
            Some(TaintSource::FileInput)
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("std::fs::File::"),
            Some(TaintSource::FileInput)
        );
    }

    #[test]
    fn test_classify_taint_source_user_controlled() {
        // Test user-controlled input detection
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("user_input.trim()"),
            Some(TaintSource::UserControlled)
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("get_input()"),
            Some(TaintSource::UserControlled)
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("stdin_input"),
            Some(TaintSource::UserControlled)
        );
    }

    #[test]
    fn test_classify_taint_source_none() {
        // Test cases that should return None
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("let x = 5"),
            None
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("println!(\"hello\")"),
            None
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("safe_function()"),
            None
        );
    }

    #[test]
    fn test_classify_taint_source_edge_cases() {
        // Test edge cases and boundary conditions
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source(""),
            None
        );
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("   "),
            None
        );
        // Test priority - CLI args should take precedence over input
        assert_eq!(
            EnhancedSqlInjectionDetector::classify_taint_source("args() with input"),
            Some(TaintSource::CliArgument)
        );
    }

    #[test]
    fn test_get_injection_type() {
        let detector = EnhancedSqlInjectionDetector::new();

        // Test format string detection
        assert_eq!(
            detector.get_injection_type("format!(\"SELECT {}\", id)"),
            SqlInjectionType::FormatString
        );
        assert_eq!(
            detector.get_injection_type("query.format()"),
            SqlInjectionType::FormatString
        );

        // Test string concatenation detection
        assert_eq!(
            detector.get_injection_type("query.push_str(&user_input)"),
            SqlInjectionType::StringConcatenation
        );
        assert_eq!(
            detector.get_injection_type("sql + &table_name"),
            SqlInjectionType::StringConcatenation
        );

        // Test dynamic query detection
        assert_eq!(
            detector.get_injection_type("diesel::query_dsl::RunQueryDsl"),
            SqlInjectionType::DynamicQuery
        );
        assert_eq!(
            detector.get_injection_type("diesel::sql_query(raw_sql)"),
            SqlInjectionType::DynamicQuery
        );

        // Test template injection (default)
        assert_eq!(
            detector.get_injection_type("render_template(sql)"),
            SqlInjectionType::TemplateInjection
        );
    }
}
