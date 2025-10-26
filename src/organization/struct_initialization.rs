/// Struct Initialization Pattern Detection
///
/// Detects struct initialization/conversion functions where high cyclomatic complexity
/// comes from conditional field assignment rather than algorithmic logic. These functions
/// are flagged incorrectly by traditional complexity metrics.
use syn::{
    spanned::Spanned, visit::Visit, Expr, ExprBlock, ExprStruct, File, ImplItem, ImplItemFn, Item,
    ItemImpl, ReturnType, Stmt, Type,
};

/// Field dependency information
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldDependency {
    /// Name of field being initialized
    pub field_name: String,

    /// Other fields or parameters this field references
    pub depends_on: Vec<String>,

    /// Complexity of field initialization (lines)
    pub initialization_complexity: usize,
}

/// Detected struct initialization pattern
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StructInitPattern {
    /// Name of struct being initialized
    pub struct_name: String,

    /// Number of fields in struct literal
    pub field_count: usize,

    /// Total lines in function
    pub function_lines: usize,

    /// Lines dedicated to field initialization
    pub initialization_lines: usize,

    /// Ratio of initialization to total lines (0.0 - 1.0)
    pub initialization_ratio: f64,

    /// Average nesting depth across initialization
    pub avg_nesting_depth: f64,

    /// Maximum nesting depth encountered
    pub max_nesting_depth: usize,

    /// Field dependencies (which fields reference others)
    pub field_dependencies: Vec<FieldDependency>,

    /// Fields requiring >10 lines of logic
    pub complex_fields: Vec<String>,

    /// Cyclomatic complexity (for comparison/context)
    pub cyclomatic_complexity: usize,

    /// Whether function wraps result in Result<T>
    pub is_result_wrapped: bool,

    /// Whether initialization calls other constructors
    pub calls_constructors: bool,
}

/// Return statement analysis
#[derive(Debug, Clone)]
pub struct ReturnAnalysis {
    pub returns_struct: bool,
    pub struct_name: Option<String>,
    pub field_count: usize,
    pub field_names: Vec<String>,
    pub is_result_wrapped: bool,
}

/// Struct initialization pattern detector configuration
pub struct StructInitPatternDetector {
    pub min_field_count: usize,
    pub min_init_ratio: f64,
    pub max_nesting_depth: usize,
}

impl Default for StructInitPatternDetector {
    fn default() -> Self {
        Self {
            min_field_count: 15,
            min_init_ratio: 0.70,
            max_nesting_depth: 4,
        }
    }
}

impl StructInitPatternDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect struct initialization pattern in a Rust file
    pub fn detect(&self, file: &File, file_content: &str) -> Option<StructInitPattern> {
        let mut detector = StructInitVisitor::new(file_content);
        detector.visit_file(file);

        // Find the function with the strongest initialization pattern
        detector
            .patterns
            .into_iter()
            .filter(|p| {
                p.field_count >= self.min_field_count
                    && p.initialization_ratio >= self.min_init_ratio
                    && p.max_nesting_depth <= self.max_nesting_depth
            })
            .max_by(|a, b| a.field_count.cmp(&b.field_count))
    }

    /// Calculate field-based complexity score (alternative to cyclomatic)
    pub fn calculate_init_complexity_score(&self, pattern: &StructInitPattern) -> f64 {
        let field_score = match pattern.field_count {
            0..=20 => 1.0,
            21..=40 => 2.0,
            41..=60 => 3.5,
            _ => 5.0,
        };

        let nesting_penalty = pattern.max_nesting_depth as f64 * 0.5;
        let complex_field_penalty = pattern.complex_fields.len() as f64 * 1.0;

        field_score + nesting_penalty + complex_field_penalty
    }

    /// Generate recommendation based on pattern
    pub fn generate_recommendation(&self, pattern: &StructInitPattern) -> String {
        if pattern.field_count > 50 {
            "Consider builder pattern to reduce initialization complexity".to_string()
        } else if pattern.complex_fields.len() > 5 {
            "Extract complex field initializations into helper functions".to_string()
        } else if pattern.max_nesting_depth > 3 {
            "Reduce nesting depth in field initialization".to_string()
        } else {
            "Initialization is appropriately complex for field count".to_string()
        }
    }

    /// Calculate confidence score (0.0 to 1.0)
    pub fn confidence(&self, pattern: &StructInitPattern) -> f64 {
        let mut confidence = 0.0;

        // Base confidence from initialization ratio
        if pattern.initialization_ratio > 0.85 {
            confidence += 0.35;
        } else if pattern.initialization_ratio > 0.75 {
            confidence += 0.25;
        } else if pattern.initialization_ratio > 0.65 {
            confidence += 0.15;
        } else {
            confidence += 0.05;
        }

        // Boost from field count
        confidence += (pattern.field_count as f64 / 50.0).min(0.25);

        // Boost from low nesting (characteristic of initialization)
        if pattern.max_nesting_depth <= 2 {
            confidence += 0.20;
        } else if pattern.max_nesting_depth <= 3 {
            confidence += 0.10;
        }

        // Boost from struct name patterns
        if pattern.struct_name.contains("Args")
            || pattern.struct_name.contains("Config")
            || pattern.struct_name.contains("Options")
        {
            confidence += 0.10;
        }

        // Penalty for complex fields (might be business logic)
        if pattern.complex_fields.len() > pattern.field_count / 3 {
            confidence *= 0.7;
        }

        confidence.min(1.0)
    }
}

/// AST visitor for struct initialization detection
struct StructInitVisitor<'a> {
    patterns: Vec<StructInitPattern>,
    file_content: &'a str,
}

impl<'a> StructInitVisitor<'a> {
    fn new(file_content: &'a str) -> Self {
        Self {
            patterns: Vec::new(),
            file_content,
        }
    }

    fn analyze_function(&mut self, function: &ImplItemFn, _impl_block: &ItemImpl) {
        // Analyze return statement
        let return_analysis = analyze_return_statement(function);

        if !return_analysis.returns_struct || return_analysis.field_count == 0 {
            return;
        }

        // Calculate function metrics
        let span = function.span();
        let start_line = span.start().line;
        let end_line = span.end().line;
        let function_lines = count_lines_in_span(self.file_content, start_line, end_line);

        // Estimate initialization lines (simplified - count lines with field assignments)
        let initialization_lines =
            estimate_initialization_lines(self.file_content, start_line, end_line);

        let initialization_ratio = initialization_lines as f64 / function_lines as f64;

        // Measure nesting depth
        let (avg_nesting, max_nesting) = measure_nesting_depth(&function.block);

        // Simple cyclomatic complexity estimate (count branches)
        let cyclomatic = estimate_cyclomatic_complexity(&function.block);

        // Detect constructor calls
        let calls_constructors = detect_constructor_calls(&function.block);

        // Analyze field dependencies and complexity
        let (field_dependencies, complex_fields) = analyze_field_dependencies_and_complexity(
            &function.block,
            &return_analysis.field_names,
            self.file_content,
        );

        // Create pattern
        let pattern = StructInitPattern {
            struct_name: return_analysis.struct_name.unwrap_or_default(),
            field_count: return_analysis.field_count,
            function_lines,
            initialization_lines,
            initialization_ratio,
            avg_nesting_depth: avg_nesting,
            max_nesting_depth: max_nesting,
            field_dependencies,
            complex_fields,
            cyclomatic_complexity: cyclomatic,
            is_result_wrapped: return_analysis.is_result_wrapped,
            calls_constructors,
        };

        self.patterns.push(pattern);
    }
}

impl<'a, 'ast> Visit<'ast> for StructInitVisitor<'a> {
    fn visit_item(&mut self, item: &'ast Item) {
        if let Item::Impl(item_impl) = item {
            for impl_item in &item_impl.items {
                if let ImplItem::Fn(method) = impl_item {
                    self.analyze_function(method, item_impl);
                }
            }
        }
        syn::visit::visit_item(self, item);
    }
}

/// Analyze return statement to detect struct literal
fn analyze_return_statement(function: &ImplItemFn) -> ReturnAnalysis {
    let mut visitor = ReturnStructVisitor {
        struct_name: None,
        field_count: 0,
        field_names: Vec::new(),
        is_result_wrapped: false,
    };

    // Check return type
    if let ReturnType::Type(_, ty) = &function.sig.output {
        visitor.is_result_wrapped = is_result_type(ty);
    }

    // Visit function body to find struct literal
    visitor.visit_block(&function.block);

    ReturnAnalysis {
        returns_struct: visitor.struct_name.is_some(),
        struct_name: visitor.struct_name,
        field_count: visitor.field_count,
        field_names: visitor.field_names,
        is_result_wrapped: visitor.is_result_wrapped,
    }
}

/// Visitor to find struct literals in return statements
struct ReturnStructVisitor {
    struct_name: Option<String>,
    field_count: usize,
    field_names: Vec<String>,
    is_result_wrapped: bool,
}

impl<'ast> Visit<'ast> for ReturnStructVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Struct(struct_expr) => {
                self.extract_struct_info(struct_expr);
            }
            Expr::Call(call_expr) => {
                // Check for Ok(StructName { ... })
                if let Expr::Path(path) = &*call_expr.func {
                    if path
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident == "Ok")
                        .unwrap_or(false)
                    {
                        if let Some(first_arg) = call_expr.args.first() {
                            if let Expr::Struct(struct_expr) = first_arg {
                                self.extract_struct_info(struct_expr);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }
}

impl ReturnStructVisitor {
    fn extract_struct_info(&mut self, struct_expr: &ExprStruct) {
        // Extract struct name
        if let Some(segment) = struct_expr.path.segments.last() {
            self.struct_name = Some(segment.ident.to_string());
        }

        // Count fields
        self.field_count = struct_expr.fields.len();

        // Extract field names
        self.field_names = struct_expr
            .fields
            .iter()
            .filter_map(|f| match &f.member {
                syn::Member::Named(ident) => Some(ident.to_string()),
                _ => None,
            })
            .collect();
    }
}

/// Check if type is Result<T>
fn is_result_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        type_path
            .path
            .segments
            .first()
            .map(|s| s.ident == "Result")
            .unwrap_or(false)
    } else {
        false
    }
}

/// Count non-empty, non-comment lines in a span
fn count_lines_in_span(content: &str, start_line: usize, end_line: usize) -> usize {
    content
        .lines()
        .enumerate()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line) + 1)
        .filter(|(_, line)| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .count()
}

/// Estimate lines dedicated to field initialization
fn estimate_initialization_lines(content: &str, start_line: usize, end_line: usize) -> usize {
    content
        .lines()
        .enumerate()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line) + 1)
        .filter(|(_, line)| {
            let trimmed = line.trim();
            // Look for field assignment patterns
            trimmed.contains("let ")
                || trimmed.contains(":")
                || trimmed.contains("unwrap_or")
                || trimmed.contains("match")
        })
        .count()
}

/// Measure nesting depth in a block
fn measure_nesting_depth(block: &syn::Block) -> (f64, usize) {
    let mut max_depth = 0;
    let mut depth_sum = 0;
    let mut node_count = 0;

    measure_depth_recursive(
        &block.stmts,
        1,
        &mut max_depth,
        &mut depth_sum,
        &mut node_count,
    );

    let avg_depth = if node_count > 0 {
        depth_sum as f64 / node_count as f64
    } else {
        0.0
    };

    (avg_depth, max_depth)
}

fn measure_depth_recursive(
    stmts: &[Stmt],
    current_depth: usize,
    max_depth: &mut usize,
    depth_sum: &mut usize,
    node_count: &mut usize,
) {
    *max_depth = (*max_depth).max(current_depth);
    *depth_sum += current_depth * stmts.len();
    *node_count += stmts.len();

    for stmt in stmts {
        match stmt {
            Stmt::Expr(Expr::If(expr_if), _) => {
                measure_depth_recursive(
                    &expr_if.then_branch.stmts,
                    current_depth + 1,
                    max_depth,
                    depth_sum,
                    node_count,
                );
            }
            Stmt::Expr(Expr::Match(expr_match), _) => {
                for arm in &expr_match.arms {
                    if let Expr::Block(ExprBlock { block, .. }) = &*arm.body {
                        measure_depth_recursive(
                            &block.stmts,
                            current_depth + 1,
                            max_depth,
                            depth_sum,
                            node_count,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

/// Estimate cyclomatic complexity (count decision points)
fn estimate_cyclomatic_complexity(block: &syn::Block) -> usize {
    let mut complexity = 1; // Start at 1
    count_decision_points(&block.stmts, &mut complexity);
    complexity
}

fn count_decision_points(stmts: &[Stmt], complexity: &mut usize) {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(Expr::If(_), _) => {
                *complexity += 1;
            }
            Stmt::Expr(Expr::Match(expr_match), _) => {
                *complexity += expr_match.arms.len().saturating_sub(1);
            }
            Stmt::Expr(Expr::While(_), _) | Stmt::Expr(Expr::ForLoop(_), _) => {
                *complexity += 1;
            }
            _ => {}
        }
    }
}

/// Detect if function calls other constructors (new, from, etc.)
fn detect_constructor_calls(block: &syn::Block) -> bool {
    let mut visitor = ConstructorCallVisitor {
        calls_constructor: false,
    };
    visitor.visit_block(block);
    visitor.calls_constructor
}

struct ConstructorCallVisitor {
    calls_constructor: bool,
}

impl<'ast> Visit<'ast> for ConstructorCallVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Expr::Call(call_expr) = expr {
            if let Expr::Path(path) = &*call_expr.func {
                if let Some(segment) = path.path.segments.last() {
                    let name = segment.ident.to_string();
                    if name == "new" || name.starts_with("from_") || name.starts_with("with_") {
                        self.calls_constructor = true;
                    }
                }
            }
        }
        syn::visit::visit_expr(self, expr);
    }
}

/// Analyze field dependencies and identify complex fields
fn analyze_field_dependencies_and_complexity(
    block: &syn::Block,
    field_names: &[String],
    file_content: &str,
) -> (Vec<FieldDependency>, Vec<String>) {
    let mut field_dependencies = Vec::new();
    let mut complex_fields = Vec::new();

    // Extract local variable bindings and their initializations
    let local_bindings = extract_local_bindings(block);

    // For each field, analyze its initialization
    for field_name in field_names {
        if let Some(binding) = local_bindings.iter().find(|(name, _)| name == field_name) {
            let (_name, expr) = binding;

            // Count lines in field initialization
            let span = expr.span();
            let start_line = span.start().line;
            let end_line = span.end().line;
            let init_lines = count_lines_in_span(file_content, start_line, end_line);

            // Identify complex fields (>10 lines)
            if init_lines > 10 {
                complex_fields.push(field_name.clone());
            }

            // Find dependencies (variables referenced in initialization)
            let depends_on = find_variable_references(expr, &local_bindings);

            // Only add dependency info if there are actual dependencies
            if !depends_on.is_empty() || init_lines > 5 {
                field_dependencies.push(FieldDependency {
                    field_name: field_name.clone(),
                    depends_on,
                    initialization_complexity: init_lines,
                });
            }
        }
    }

    (field_dependencies, complex_fields)
}

/// Extract local variable bindings from a block
fn extract_local_bindings(block: &syn::Block) -> Vec<(String, Expr)> {
    let mut bindings = Vec::new();

    for stmt in &block.stmts {
        if let Stmt::Local(local) = stmt {
            if let syn::Pat::Ident(pat_ident) = &local.pat {
                let var_name = pat_ident.ident.to_string();
                if let Some(init) = &local.init {
                    bindings.push((var_name, (*init.expr).clone()));
                }
            }
        }
    }

    bindings
}

/// Find variable references in an expression
fn find_variable_references(expr: &Expr, local_bindings: &[(String, Expr)]) -> Vec<String> {
    let mut visitor = VariableRefVisitor {
        references: Vec::new(),
        local_vars: local_bindings.iter().map(|(name, _)| name.clone()).collect(),
    };
    visitor.visit_expr(expr);
    visitor.references
}

/// Visitor to find variable references
struct VariableRefVisitor {
    references: Vec<String>,
    local_vars: Vec<String>,
}

impl<'ast> Visit<'ast> for VariableRefVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Path(expr_path) => {
                if let Some(ident) = expr_path.path.get_ident() {
                    let var_name = ident.to_string();
                    // Only track references to local variables
                    if self.local_vars.contains(&var_name) && !self.references.contains(&var_name) {
                        self.references.push(var_name);
                    }
                }
            }
            Expr::Field(expr_field) => {
                // Handle field access like low.column
                if let Expr::Path(base_path) = &*expr_field.base {
                    if let Some(ident) = base_path.path.get_ident() {
                        let var_name = ident.to_string();
                        if self.local_vars.contains(&var_name) && !self.references.contains(&var_name) {
                            self.references.push(var_name);
                        }
                    }
                }
            }
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_rust_code(code: &str) -> File {
        syn::parse_str(code).expect("Failed to parse Rust code")
    }

    #[test]
    fn test_detect_struct_init_basic() {
        let code = r#"
            pub struct HiArgs {
                patterns: String,
                paths: String,
                column: bool,
                heading: bool,
                timeout: u32,
                retries: u32,
                max_wait: u32,
                backoff: u32,
                host: String,
                port: u16,
                path: String,
                headers: Vec<String>,
                buffer_size: usize,
                enable_logging: bool,
                enable_metrics: bool,
            }

            impl HiArgs {
                pub fn from_low_args(low: LowArgs) -> Result<HiArgs> {
                    let column = low.column.unwrap_or(low.vimgrep);
                    let heading = match low.heading {
                        None => !low.vimgrep && true,
                        Some(false) => false,
                        Some(true) => !low.vimgrep,
                    };
                    let timeout = low.timeout.unwrap_or(30);
                    let retries = low.retries.unwrap_or(3);
                    let max_wait = timeout * retries;
                    let backoff = timeout / retries;
                    let host = low.host.unwrap_or_default();
                    let port = low.port.unwrap_or(8080);
                    let path = low.path.unwrap_or_else(|| "/".to_string());
                    let headers = low.headers.unwrap_or_default();
                    let buffer_size = low.buffer_size.unwrap_or(8192);
                    let enable_logging = low.enable_logging;
                    let enable_metrics = low.enable_metrics;

                    Ok(HiArgs {
                        patterns: "test".into(),
                        paths: "test".into(),
                        column,
                        heading,
                        timeout,
                        retries,
                        max_wait,
                        backoff,
                        host,
                        port,
                        path,
                        headers,
                        buffer_size,
                        enable_logging,
                        enable_metrics,
                    })
                }
            }

            pub struct LowArgs {
                pub column: Option<bool>,
                pub vimgrep: bool,
                pub heading: Option<bool>,
                pub timeout: Option<u32>,
                pub retries: Option<u32>,
                pub host: Option<String>,
                pub port: Option<u16>,
                pub path: Option<String>,
                pub headers: Option<Vec<String>>,
                pub buffer_size: Option<usize>,
                pub enable_logging: bool,
                pub enable_metrics: bool,
            }

            pub struct Result<T> {
                value: T,
            }
        "#;

        let file = parse_rust_code(code);
        // Use lower thresholds for this test
        let detector = StructInitPatternDetector {
            min_field_count: 10,
            min_init_ratio: 0.40, // Lower ratio since test code has extra whitespace
            max_nesting_depth: 5,
        };

        let pattern = detector.detect(&file, code);
        assert!(
            pattern.is_some(),
            "Should detect struct initialization pattern"
        );

        let pattern = pattern.unwrap();
        assert_eq!(pattern.struct_name, "HiArgs");
        assert!(pattern.field_count >= 15, "Should detect 15 fields");
        assert!(
            pattern.initialization_ratio > 0.40,
            "Initialization ratio should be > 0.40, got {:.2}",
            pattern.initialization_ratio
        );
    }

    #[test]
    fn test_field_based_complexity_lower_than_cyclomatic() {
        let pattern = StructInitPattern {
            struct_name: "HiArgs".into(),
            field_count: 40,
            function_lines: 214,
            initialization_lines: 180,
            initialization_ratio: 0.84,
            avg_nesting_depth: 1.8,
            max_nesting_depth: 3,
            field_dependencies: vec![],
            complex_fields: vec![],
            cyclomatic_complexity: 42,
            is_result_wrapped: true,
            calls_constructors: true,
        };

        let detector = StructInitPatternDetector::default();
        let field_score = detector.calculate_init_complexity_score(&pattern);

        // Field-based score should be much lower than cyclomatic 42
        assert!(
            field_score < 10.0,
            "Field score {} should be < 10.0",
            field_score
        );
        assert!(
            field_score < pattern.cyclomatic_complexity as f64 / 4.0,
            "Field score {} should be < cyclomatic/4",
            field_score
        );
    }

    #[test]
    fn test_not_initialization_business_logic() {
        let code = r#"
            impl Calculator {
                pub fn calculate_scores(data: &[Item]) -> Vec<Score> {
                    data.iter()
                        .filter(|item| item.is_valid())
                        .map(|item| {
                            let base = item.value * 2;
                            let bonus = if item.premium { 10 } else { 0 };
                            Score { value: base + bonus }
                        })
                        .collect()
                }
            }

            pub struct Score {
                value: i32,
            }
        "#;

        let file = parse_rust_code(code);
        let detector = StructInitPatternDetector::default();

        let pattern = detector.detect(&file, code);
        // Small struct inside business logic - should not match thresholds
        assert!(
            pattern.is_none(),
            "Business logic should not be detected as initialization"
        );
    }

    #[test]
    fn test_confidence_calculation() {
        let detector = StructInitPatternDetector::default();

        let high_confidence = StructInitPattern {
            struct_name: "HttpClientArgs".into(),
            field_count: 35,
            function_lines: 150,
            initialization_lines: 130,
            initialization_ratio: 0.87,
            avg_nesting_depth: 1.5,
            max_nesting_depth: 2,
            field_dependencies: vec![],
            complex_fields: vec![],
            cyclomatic_complexity: 38,
            is_result_wrapped: true,
            calls_constructors: false,
        };

        let confidence = detector.confidence(&high_confidence);
        assert!(
            confidence > 0.70,
            "High confidence pattern should score > 0.70, got {}",
            confidence
        );
    }
}
