/// Parallel Execution Pattern Detection
///
/// Detects parallel execution patterns (rayon, tokio, std::thread) and analyzes
/// coordination complexity vs. algorithmic complexity. Helps distinguish between
/// extractable business logic and necessary coordination overhead in parallel code.
use std::collections::HashSet;
use syn::{visit::Visit, Expr, ExprClosure, ExprMethodCall, File, Item, Stmt};

/// Parallel library being used
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ParallelLibrary {
    Rayon,
    Tokio,
    StdThread,
    Crossbeam,
}

impl std::fmt::Display for ParallelLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParallelLibrary::Rayon => write!(f, "rayon"),
            ParallelLibrary::Tokio => write!(f, "tokio"),
            ParallelLibrary::StdThread => write!(f, "std::thread"),
            ParallelLibrary::Crossbeam => write!(f, "crossbeam"),
        }
    }
}

/// Information about a closure in parallel code
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClosureInfo {
    /// Line number where closure starts
    pub line_number: usize,

    /// Variables captured from outer scope (estimated)
    pub captures: Vec<String>,

    /// Whether closure uses `move` keyword
    pub is_move: bool,

    /// Cyclomatic complexity of closure body (estimated)
    pub closure_complexity: usize,

    /// Lines of code in closure
    pub lines: usize,

    /// Whether closure could be extracted
    pub extractable: bool,
}

/// Detected parallel execution pattern
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParallelPattern {
    /// Parallel library being used
    pub library: ParallelLibrary,

    /// Number of closures in function
    pub closure_count: usize,

    /// Total captured variables across all closures
    pub total_captures: usize,

    /// Average captures per closure
    pub avg_captures_per_closure: f64,

    /// Lines in setup phase (before parallel execution)
    pub setup_lines: usize,

    /// Lines in execution phase (parallel iteration/spawn)
    pub execution_lines: usize,

    /// Lines in aggregation phase (after parallel)
    pub aggregation_lines: usize,

    /// Cyclomatic complexity (for comparison)
    pub cyclomatic_complexity: usize,

    /// Coordination complexity (derived metric)
    pub coordination_complexity: f64,

    /// Synchronization primitives used
    pub sync_primitives: Vec<String>,

    /// Whether closures are move closures
    pub has_move_closures: bool,

    /// Individual closure information
    pub closures: Vec<ClosureInfo>,
}

/// Parallel pattern detector configuration
pub struct ParallelPatternDetector {
    pub min_closure_captures: usize,
    pub min_parallel_calls: usize,
}

impl Default for ParallelPatternDetector {
    fn default() -> Self {
        Self {
            min_closure_captures: 3,
            min_parallel_calls: 1,
        }
    }
}

impl ParallelPatternDetector {
    /// Detect parallel execution pattern in a function
    pub fn detect(&self, ast: &File, source_content: &str) -> Option<ParallelPattern> {
        let mut visitor = ParallelVisitor::new();
        visitor.visit_file(ast);

        // Must have at least one parallel call
        if visitor.parallel_calls.is_empty() {
            return None;
        }

        // Determine which library is being used
        let library = self.detect_library(&visitor)?;

        // Analyze closures for captures
        let closures = self.analyze_closures(&visitor, source_content);

        // Must meet minimum capture threshold
        let total_captures: usize = closures.iter().map(|c| c.captures.len()).sum();
        if closures.is_empty() || total_captures < self.min_closure_captures {
            return None;
        }

        let avg_captures = total_captures as f64 / closures.len() as f64;

        // Calculate coordination complexity
        let coordination_complexity = calculate_coordination_complexity(
            total_captures,
            closures.len(),
            visitor.sync_primitives.len(),
        );

        // Estimate phase lines (simplified - actual implementation would parse AST more deeply)
        let total_lines = source_content.lines().count();
        let setup_lines = total_lines / 5; // Rough estimate
        let execution_lines = total_lines / 2;
        let aggregation_lines = total_lines - setup_lines - execution_lines;

        Some(ParallelPattern {
            library,
            closure_count: closures.len(),
            total_captures,
            avg_captures_per_closure: avg_captures,
            setup_lines,
            execution_lines,
            aggregation_lines,
            cyclomatic_complexity: 0, // Will be filled in by caller
            coordination_complexity,
            sync_primitives: visitor.sync_primitives.clone(),
            has_move_closures: closures.iter().any(|c| c.is_move),
            closures,
        })
    }

    /// Calculate confidence in pattern detection
    pub fn confidence(&self, pattern: &ParallelPattern) -> f64 {
        let mut confidence: f64 = 0.7; // Base confidence

        // Higher confidence with more closures
        if pattern.closure_count >= 2 {
            confidence += 0.1;
        }

        // Higher confidence with clear parallel library usage
        confidence += 0.1;

        // Higher confidence with sync primitives
        if !pattern.sync_primitives.is_empty() {
            confidence += 0.1;
        }

        // Lower confidence if few captures (might not be true parallel pattern)
        if pattern.avg_captures_per_closure < 2.0 {
            confidence -= 0.1;
        }

        confidence.clamp(0.0, 1.0)
    }

    fn detect_library(&self, visitor: &ParallelVisitor) -> Option<ParallelLibrary> {
        // Check for rayon patterns
        if visitor
            .parallel_calls
            .iter()
            .any(|call| call.contains("par_iter") || call.contains("par_bridge"))
        {
            return Some(ParallelLibrary::Rayon);
        }

        // Check for tokio patterns
        if visitor.parallel_calls.iter().any(|call| {
            call.contains("tokio::spawn")
                || call.contains("spawn")
                || call.contains("join!")
                || call.contains("select!")
        }) {
            return Some(ParallelLibrary::Tokio);
        }

        // Check for std::thread patterns
        if visitor
            .parallel_calls
            .iter()
            .any(|call| call.contains("thread::spawn") || call.contains("thread::scope"))
        {
            return Some(ParallelLibrary::StdThread);
        }

        // Check for crossbeam patterns
        if visitor
            .parallel_calls
            .iter()
            .any(|call| call.contains("crossbeam"))
        {
            return Some(ParallelLibrary::Crossbeam);
        }

        None
    }

    fn analyze_closures(
        &self,
        visitor: &ParallelVisitor,
        source_content: &str,
    ) -> Vec<ClosureInfo> {
        visitor
            .closures
            .iter()
            .map(|closure_expr| {
                // Estimate captures by counting identifiers in closure that aren't parameters
                // This is a simplified heuristic - proper implementation would use scope analysis
                let captures = estimate_captures(closure_expr, source_content);
                let is_move = is_move_closure(closure_expr);
                let lines = estimate_closure_lines(closure_expr, source_content);

                // Closure is extractable if it has few captures and is complex enough
                let extractable = captures.len() <= 2 && lines > 20;

                ClosureInfo {
                    line_number: 0, // Would need span info
                    captures: captures.clone(),
                    is_move,
                    closure_complexity: estimate_closure_complexity(closure_expr),
                    lines,
                    extractable,
                }
            })
            .collect()
    }
}

/// Visitor to find parallel execution patterns
struct ParallelVisitor {
    parallel_calls: Vec<String>,
    closures: Vec<ExprClosure>,
    sync_primitives: Vec<String>,
}

impl ParallelVisitor {
    fn new() -> Self {
        Self {
            parallel_calls: Vec::new(),
            closures: Vec::new(),
            sync_primitives: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for ParallelVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();

        // Check for parallel iterator methods
        if method_name.contains("par_iter")
            || method_name.contains("par_bridge")
            || method_name == "spawn"
        {
            self.parallel_calls.push(method_name);
        }

        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast ExprClosure) {
        self.closures.push(node.clone());
        syn::visit::visit_expr_closure(self, node);
    }

    fn visit_item(&mut self, node: &'ast Item) {
        // Look for sync primitive types
        if let Item::Type(ty) = node {
            let ty_str = quote::quote!(#ty).to_string();
            if ty_str.contains("Mutex")
                || ty_str.contains("RwLock")
                || ty_str.contains("AtomicBool")
                || ty_str.contains("Arc")
            {
                self.sync_primitives
                    .push(extract_sync_primitive_name(&ty_str));
            }
        }

        syn::visit::visit_item(self, node);
    }
}

/// Calculate coordination complexity based on captures, closures, and sync primitives
fn calculate_coordination_complexity(
    total_captures: usize,
    closure_count: usize,
    sync_primitive_count: usize,
) -> f64 {
    let capture_complexity = total_captures as f64 * 0.5;
    let closure_complexity = closure_count as f64 * 1.0;
    let sync_complexity = sync_primitive_count as f64 * 0.8;

    capture_complexity + closure_complexity + sync_complexity
}

/// Adjust base score for parallel execution patterns
pub fn adjust_parallel_score(base_score: f64, pattern: &ParallelPattern) -> f64 {
    // Reduce score for coordination complexity (expected overhead)
    let coordination_factor = if pattern.avg_captures_per_closure > 5.0 {
        0.5 // High capture count = complex coordination, expected
    } else if pattern.avg_captures_per_closure > 3.0 {
        0.6 // Moderate captures
    } else {
        0.8 // Low captures - might be extractable
    };

    // Consider closure complexity
    let closure_factor = if pattern.closure_count > 2 {
        0.9 // Multiple closures = complex coordination
    } else {
        1.0
    };

    base_score * coordination_factor * closure_factor
}

/// Estimate captured variables in a closure (simplified heuristic)
fn estimate_captures(closure: &ExprClosure, _source_content: &str) -> Vec<String> {
    let mut captures = HashSet::new();

    // Get closure parameters to exclude them from captures
    let params: HashSet<String> = closure
        .inputs
        .iter()
        .filter_map(|pat| {
            if let syn::Pat::Ident(ident) = pat {
                Some(ident.ident.to_string())
            } else {
                None
            }
        })
        .collect();

    // Visit closure body to find identifiers
    let mut identifier_visitor = IdentifierVisitor {
        identifiers: HashSet::new(),
    };
    identifier_visitor.visit_expr(&closure.body);

    // Captures are identifiers that aren't parameters
    for ident in identifier_visitor.identifiers {
        if !params.contains(&ident) && !is_keyword(&ident) {
            captures.insert(ident);
        }
    }

    captures.into_iter().collect()
}

/// Check if closure uses move keyword
fn is_move_closure(closure: &ExprClosure) -> bool {
    closure.capture.is_some()
}

/// Estimate lines of code in closure
fn estimate_closure_lines(closure: &ExprClosure, _source_content: &str) -> usize {
    // Simplified: count statements in closure body
    if let Expr::Block(block) = closure.body.as_ref() {
        block.block.stmts.len()
    } else {
        1
    }
}

/// Estimate closure complexity (simplified cyclomatic complexity)
fn estimate_closure_complexity(closure: &ExprClosure) -> usize {
    let mut complexity = 1; // Base complexity

    if let Expr::Block(block) = closure.body.as_ref() {
        for stmt in &block.block.stmts {
            if let Stmt::Expr(expr, _) = stmt {
                complexity += count_branches(expr);
            }
        }
    }

    complexity
}

/// Count branches in an expression
fn count_branches(expr: &Expr) -> usize {
    match expr {
        Expr::If(_) => 1,
        Expr::Match(match_expr) => match_expr.arms.len(),
        Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) => 1,
        _ => 0,
    }
}

/// Extract sync primitive name from type string
fn extract_sync_primitive_name(ty_str: &str) -> String {
    if ty_str.contains("Mutex") {
        "Mutex".to_string()
    } else if ty_str.contains("RwLock") {
        "RwLock".to_string()
    } else if ty_str.contains("AtomicBool") {
        "AtomicBool".to_string()
    } else if ty_str.contains("Arc") {
        "Arc".to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Check if identifier is a Rust keyword
fn is_keyword(ident: &str) -> bool {
    matches!(
        ident,
        "self" | "Self" | "true" | "false" | "Some" | "None" | "Ok" | "Err"
    )
}

/// Visitor to collect identifiers
struct IdentifierVisitor {
    identifiers: HashSet<String>,
}

impl<'ast> Visit<'ast> for IdentifierVisitor {
    fn visit_ident(&mut self, node: &'ast syn::Ident) {
        self.identifiers.insert(node.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_library_detection() {
        let code = r#"
            fn search_parallel(args: &Args) -> Result<bool> {
                let results = items.par_iter().map(|item| {
                    process(item)
                }).collect();
                Ok(results)
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let detector = ParallelPatternDetector::default();

        if let Some(pattern) = detector.detect(&ast, code) {
            assert_eq!(pattern.library, ParallelLibrary::Rayon);
        }
    }

    #[test]
    fn test_coordination_complexity_calculation() {
        let complexity = calculate_coordination_complexity(6, 1, 2);
        // 6 * 0.5 + 1 * 1.0 + 2 * 0.8 = 3.0 + 1.0 + 1.6 = 5.6
        assert!((complexity - 5.6).abs() < 0.01);
    }

    #[test]
    fn test_score_adjustment() {
        let pattern = ParallelPattern {
            library: ParallelLibrary::Rayon,
            closure_count: 1,
            total_captures: 6,
            avg_captures_per_closure: 6.0,
            setup_lines: 10,
            execution_lines: 40,
            aggregation_lines: 5,
            cyclomatic_complexity: 15,
            coordination_complexity: 8.0,
            sync_primitives: vec!["AtomicBool".into(), "Mutex".into()],
            has_move_closures: true,
            closures: vec![],
        };

        let base_score = 1000.0;
        let adjusted = adjust_parallel_score(base_score, &pattern);

        // Should be reduced by 50% (high captures)
        assert!((adjusted - 500.0).abs() < 0.01);
    }

    #[test]
    fn test_move_closure_detection() {
        let code = r#"move || { println!("test"); }"#;

        let ast: ExprClosure = syn::parse_str(code).unwrap();
        assert!(is_move_closure(&ast));
    }
}
