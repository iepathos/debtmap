use syn::{Block, Expr, Stmt};

/// Core trait for semantic normalization of AST structures
pub trait SemanticNormalizer {
    type Input;
    type Output;

    fn normalize(&self, input: Self::Input) -> Self::Output;
}

/// Normalized representation of a code block
#[derive(Debug, Clone)]
pub struct NormalizedBlock {
    pub statements: Vec<NormalizedStatement>,
    pub logical_structure: LogicalStructure,
    pub formatting_metadata: FormattingMetadata,
}

/// Normalized representation of a statement
#[derive(Debug, Clone)]
pub enum NormalizedStatement {
    Expression(NormalizedExpression),
    Declaration(NormalizedDeclaration),
    Control(NormalizedControl),
    Local(NormalizedLocal),
}

/// Normalized representation of an expression
#[derive(Debug, Clone)]
pub struct NormalizedExpression {
    pub expr_type: ExprType,
    pub logical_components: Vec<LogicalComponent>,
    pub is_multiline_artifact: bool,
    pub original_expr: Expr,
}

/// Normalized representation of a declaration
#[derive(Debug, Clone)]
pub struct NormalizedDeclaration {
    pub name: String,
    pub is_multiline_signature: bool,
}

/// Normalized representation of control flow
#[derive(Debug, Clone)]
pub struct NormalizedControl {
    pub control_type: ControlType,
    pub condition: Option<Box<NormalizedExpression>>,
    pub body: Box<NormalizedBlock>,
}

/// Normalized representation of local variables
#[derive(Debug, Clone)]
pub struct NormalizedLocal {
    pub pattern: String,
    pub init: Option<NormalizedExpression>,
    pub is_multiline_pattern: bool,
}

/// Logical structure information
#[derive(Debug, Clone)]
pub struct LogicalStructure {
    pub depth: usize,
    pub branching_factor: usize,
    pub has_early_return: bool,
}

/// Formatting metadata for tracking normalization changes
#[derive(Debug, Clone)]
pub struct FormattingMetadata {
    pub original_lines: usize,
    pub normalized_lines: usize,
    pub whitespace_changes: u32,
    pub multiline_expressions_normalized: u32,
}

/// Expression type classification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExprType {
    ControlFlow,
    Match { arm_count: usize },
    LogicalOp,
    Closure { is_async: bool },
    Await,
    Unsafe,
    MethodCall,
    Binary,
    Unary,
    Literal,
    Path,
    Field,
    FunctionCall,
    Tuple,
    Array,
    Other,
}

/// Control flow type
#[derive(Debug, Clone, PartialEq)]
pub enum ControlType {
    If,
    While,
    For,
    Loop,
    Match,
    Try,
}

/// Logical component within an expression
#[derive(Debug, Clone)]
pub struct LogicalComponent {
    pub component_type: ComponentType,
    pub complexity_contribution: u32,
}

/// Component type classification
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentType {
    Condition,
    Branch,
    Iteration,
    Exception,
    Operator,
    FunctionCall,
    FieldAccess,
}

/// Helper functions for normalization
impl NormalizedBlock {
    pub fn from_syn_block(block: &Block) -> Self {
        let mut statements = Vec::new();
        let mut formatting_metadata = FormattingMetadata {
            original_lines: 0,
            normalized_lines: 0,
            whitespace_changes: 0,
            multiline_expressions_normalized: 0,
        };

        for stmt in &block.stmts {
            statements.push(normalize_statement(stmt, &mut formatting_metadata));
        }

        let logical_structure = LogicalStructure {
            depth: calculate_logical_depth(&statements),
            branching_factor: calculate_branching_factor(&statements),
            has_early_return: has_early_return(&statements),
        };

        Self {
            statements,
            logical_structure,
            formatting_metadata,
        }
    }
}

fn normalize_statement(stmt: &Stmt, metadata: &mut FormattingMetadata) -> NormalizedStatement {
    match stmt {
        Stmt::Local(local) => {
            let is_multiline = detect_multiline_pattern(&local.pat);
            if is_multiline {
                metadata.multiline_expressions_normalized += 1;
            }

            NormalizedStatement::Local(NormalizedLocal {
                pattern: format!("{}", quote::quote! { #local.pat }),
                init: local
                    .init
                    .as_ref()
                    .map(|init| normalize_expression(&init.expr, metadata)),
                is_multiline_pattern: is_multiline,
            })
        }
        Stmt::Expr(expr, _) => {
            NormalizedStatement::Expression(normalize_expression(expr, metadata))
        }
        Stmt::Item(item) => NormalizedStatement::Declaration(NormalizedDeclaration {
            name: extract_item_name(item),
            is_multiline_signature: detect_multiline_signature(item),
        }),
        Stmt::Macro(_) => NormalizedStatement::Expression(NormalizedExpression {
            expr_type: ExprType::Other,
            logical_components: vec![],
            is_multiline_artifact: false,
            original_expr: syn::parse_quote! { () },
        }),
    }
}

fn normalize_expression(expr: &Expr, metadata: &mut FormattingMetadata) -> NormalizedExpression {
    let expr_type = classify_expression(expr);
    let is_multiline = detect_multiline_expression(expr);

    if is_multiline {
        metadata.multiline_expressions_normalized += 1;
    }

    let logical_components = extract_logical_components(expr);

    NormalizedExpression {
        expr_type,
        logical_components,
        is_multiline_artifact: is_multiline && !requires_multiline(&expr_type),
        original_expr: expr.clone(),
    }
}

fn classify_expression(expr: &Expr) -> ExprType {
    match expr {
        Expr::If(_) | Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) | Expr::Try(_) => {
            ExprType::ControlFlow
        }
        Expr::Match(expr_match) => ExprType::Match {
            arm_count: expr_match.arms.len(),
        },
        Expr::Binary(binary) if is_logical_operator(&binary.op) => ExprType::LogicalOp,
        Expr::Closure(closure) => ExprType::Closure {
            is_async: closure.asyncness.is_some(),
        },
        Expr::Await(_) => ExprType::Await,
        Expr::Unsafe(_) => ExprType::Unsafe,
        Expr::MethodCall(_) => ExprType::MethodCall,
        Expr::Binary(_) => ExprType::Binary,
        Expr::Unary(_) => ExprType::Unary,
        Expr::Lit(_) => ExprType::Literal,
        Expr::Path(_) => ExprType::Path,
        Expr::Field(_) => ExprType::Field,
        Expr::Call(_) => ExprType::FunctionCall,
        Expr::Tuple(_) => ExprType::Tuple,
        Expr::Array(_) | Expr::Repeat(_) => ExprType::Array,
        _ => ExprType::Other,
    }
}

fn is_logical_operator(op: &syn::BinOp) -> bool {
    matches!(op, syn::BinOp::And(_) | syn::BinOp::Or(_))
}

fn detect_multiline_expression(expr: &Expr) -> bool {
    // This is a simplified check - in a real implementation,
    // we would use span information to detect actual multiline expressions
    match expr {
        Expr::Closure(closure) => {
            // Check if closure body spans multiple lines
            matches!(&*closure.body, Expr::Block(_))
        }
        Expr::Match(expr_match) => {
            // Match expressions are typically multiline
            expr_match.arms.len() > 1
        }
        Expr::Block(_) | Expr::If(_) | Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) => true,
        Expr::Call(call) => {
            // Function calls with many arguments might be multiline
            call.args.len() > 3
        }
        Expr::MethodCall(method) => {
            // Method chains might be multiline
            method.args.len() > 2 || matches!(&*method.receiver, Expr::MethodCall(_))
        }
        Expr::Tuple(tuple) => {
            // Large tuples might be multiline
            tuple.elems.len() > 3
        }
        _ => false,
    }
}

fn detect_multiline_pattern(pat: &syn::Pat) -> bool {
    // Detect if a pattern is likely formatted across multiple lines
    match pat {
        syn::Pat::Struct(pat_struct) => {
            // Struct patterns with many fields are often multiline
            pat_struct.fields.len() > 2
        }
        syn::Pat::Tuple(pat_tuple) => {
            // Tuple patterns with many elements might be multiline
            pat_tuple.elems.len() > 3
        }
        syn::Pat::TupleStruct(pat_tuple) => pat_tuple.elems.len() > 3,
        _ => false,
    }
}

fn detect_multiline_signature(item: &syn::Item) -> bool {
    match item {
        syn::Item::Fn(item_fn) => {
            // Functions with many parameters might have multiline signatures
            item_fn.sig.inputs.len() > 3
        }
        _ => false,
    }
}

fn extract_item_name(item: &syn::Item) -> String {
    match item {
        syn::Item::Fn(f) => f.sig.ident.to_string(),
        syn::Item::Struct(s) => s.ident.to_string(),
        syn::Item::Enum(e) => e.ident.to_string(),
        syn::Item::Trait(t) => t.ident.to_string(),
        syn::Item::Type(t) => t.ident.to_string(),
        syn::Item::Const(c) => c.ident.to_string(),
        syn::Item::Static(s) => s.ident.to_string(),
        _ => String::from("<item>"),
    }
}

fn extract_logical_components(expr: &Expr) -> Vec<LogicalComponent> {
    let mut components = Vec::new();

    match expr {
        Expr::If(_) => {
            components.push(LogicalComponent {
                component_type: ComponentType::Condition,
                complexity_contribution: 1,
            });
            components.push(LogicalComponent {
                component_type: ComponentType::Branch,
                complexity_contribution: 1,
            });
        }
        Expr::Match(expr_match) => {
            for _ in &expr_match.arms {
                components.push(LogicalComponent {
                    component_type: ComponentType::Branch,
                    complexity_contribution: 1,
                });
            }
        }
        Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) => {
            components.push(LogicalComponent {
                component_type: ComponentType::Iteration,
                complexity_contribution: 1,
            });
        }
        Expr::Try(_) => {
            components.push(LogicalComponent {
                component_type: ComponentType::Exception,
                complexity_contribution: 1,
            });
        }
        Expr::Binary(binary) if is_logical_operator(&binary.op) => {
            components.push(LogicalComponent {
                component_type: ComponentType::Operator,
                complexity_contribution: 1,
            });
        }
        Expr::Call(_) | Expr::MethodCall(_) => {
            components.push(LogicalComponent {
                component_type: ComponentType::FunctionCall,
                complexity_contribution: 0,
            });
        }
        Expr::Field(_) => {
            components.push(LogicalComponent {
                component_type: ComponentType::FieldAccess,
                complexity_contribution: 0,
            });
        }
        _ => {}
    }

    components
}

fn requires_multiline(expr_type: &ExprType) -> bool {
    // Some expressions naturally require multiple lines
    matches!(
        expr_type,
        ExprType::ControlFlow | ExprType::Match { .. } | ExprType::Unsafe
    )
}

fn calculate_logical_depth(statements: &[NormalizedStatement]) -> usize {
    let mut max_depth = 0;
    let mut current_depth = 0;

    for stmt in statements {
        if let NormalizedStatement::Control(control) = stmt {
            current_depth += 1;
            max_depth = max_depth.max(current_depth);
            // Recursively check the control body
            let body_depth = control.body.logical_structure.depth;
            max_depth = max_depth.max(current_depth + body_depth);
            current_depth -= 1;
        }
    }

    max_depth
}

fn calculate_branching_factor(statements: &[NormalizedStatement]) -> usize {
    let mut branching = 0;

    for stmt in statements {
        match stmt {
            NormalizedStatement::Control(control) => match control.control_type {
                ControlType::If | ControlType::Match => branching += 1,
                _ => {}
            },
            NormalizedStatement::Expression(expr) => {
                if expr.expr_type == ExprType::LogicalOp {
                    branching += 1;
                }
            }
            _ => {}
        }
    }

    branching
}

fn has_early_return(statements: &[NormalizedStatement]) -> bool {
    for (i, stmt) in statements.iter().enumerate() {
        if i < statements.len() - 1 {
            // Check if this is a return that's not the last statement
            if let NormalizedStatement::Expression(expr) = stmt {
                if let Expr::Return(_) = &expr.original_expr {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_normalize_simple_block() {
        let block: Block = parse_quote! {
            {
                let x = 5;
                println!("{}", x);
            }
        };

        let normalized = NormalizedBlock::from_syn_block(&block);
        assert_eq!(normalized.statements.len(), 2);
        assert_eq!(normalized.logical_structure.depth, 0);
    }

    #[test]
    fn test_detect_multiline_tuple() {
        let expr: Expr = parse_quote! {
            (a, b, c, d, e)
        };

        assert!(detect_multiline_expression(&expr));
    }

    #[test]
    fn test_classify_control_flow() {
        let if_expr: Expr = parse_quote! { if x > 0 { 1 } else { 0 } };
        assert_eq!(classify_expression(&if_expr), ExprType::ControlFlow);

        let match_expr: Expr = parse_quote! {
            match x {
                0 => "zero",
                _ => "other",
            }
        };
        assert_eq!(
            classify_expression(&match_expr),
            ExprType::Match { arm_count: 2 }
        );
    }

    #[test]
    fn test_multiline_detection_not_required() {
        let expr: Expr = parse_quote! { x + y };
        let normalized = normalize_expression(
            &expr,
            &mut FormattingMetadata {
                original_lines: 0,
                normalized_lines: 0,
                whitespace_changes: 0,
                multiline_expressions_normalized: 0,
            },
        );

        assert!(!normalized.is_multiline_artifact);
    }
}
