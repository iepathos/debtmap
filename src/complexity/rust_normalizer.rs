use super::semantic_normalizer::{
    ComponentType, ControlType, ExprType, FormattingMetadata, LogicalComponent, LogicalStructure,
    NormalizedBlock, NormalizedExpression, NormalizedStatement, SemanticNormalizer,
};
use syn::{Block, Expr, Stmt};

/// Rust-specific semantic normalizer
pub struct RustSemanticNormalizer;

impl Default for RustSemanticNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl RustSemanticNormalizer {
    pub fn new() -> Self {
        Self
    }

    /// Normalize a Rust function signature to remove formatting artifacts
    pub fn normalize_signature(&self, sig: &syn::Signature) -> NormalizedSignature {
        let is_multiline = sig.inputs.len() > 3 || self.has_complex_return_type(&sig.output);

        NormalizedSignature {
            name: sig.ident.to_string(),
            param_count: sig.inputs.len(),
            is_async: sig.asyncness.is_some(),
            is_unsafe: sig.unsafety.is_some(),
            is_multiline_artifact: is_multiline,
        }
    }

    /// Check if return type is complex enough to warrant multiline formatting
    fn has_complex_return_type(&self, output: &syn::ReturnType) -> bool {
        match output {
            syn::ReturnType::Default => false,
            syn::ReturnType::Type(_, ty) => Self::is_complex_type(ty),
        }
    }

    /// Determine if a type is complex (likely to be formatted across multiple lines)
    fn is_complex_type(ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Tuple(tuple) => tuple.elems.len() > 2,
            syn::Type::Path(path) => {
                // Check for complex generic arguments
                path.path.segments.iter().any(|seg| {
                    matches!(&seg.arguments, syn::PathArguments::AngleBracketed(args) if args.args.len() > 2)
                })
            }
            syn::Type::Reference(reference) => Self::is_complex_type(&reference.elem),
            syn::Type::Ptr(ptr) => Self::is_complex_type(&ptr.elem),
            syn::Type::Slice(slice) => Self::is_complex_type(&slice.elem),
            syn::Type::Array(array) => Self::is_complex_type(&array.elem),
            _ => false,
        }
    }

    /// Normalize match expressions to handle formatting variations
    pub fn normalize_match(&self, expr_match: &syn::ExprMatch) -> NormalizedMatch {
        let mut patterns = Vec::new();
        let mut has_guard = false;
        let mut has_multiline_patterns = false;

        for arm in &expr_match.arms {
            // Check for guard conditions
            if arm.guard.is_some() {
                has_guard = true;
            }

            // Check for multiline patterns
            if self.is_multiline_pattern(&arm.pat) {
                has_multiline_patterns = true;
            }

            patterns.push(NormalizedPattern {
                is_wildcard: matches!(&arm.pat, syn::Pat::Wild(_)),
                has_guard: arm.guard.is_some(),
                is_multiline: self.is_multiline_pattern(&arm.pat),
            });
        }

        NormalizedMatch {
            arm_count: expr_match.arms.len(),
            patterns,
            has_guard,
            has_multiline_patterns,
        }
    }

    /// Check if a pattern is likely formatted across multiple lines
    fn is_multiline_pattern(&self, pat: &syn::Pat) -> bool {
        match pat {
            syn::Pat::Struct(pat_struct) => pat_struct.fields.len() > 2,
            syn::Pat::Tuple(pat_tuple) => pat_tuple.elems.len() > 3,
            syn::Pat::TupleStruct(pat_tuple) => pat_tuple.elems.len() > 3,
            syn::Pat::Or(pat_or) => pat_or.cases.len() > 2,
            _ => false,
        }
    }

    /// Normalize method chains to handle different formatting styles
    pub fn normalize_method_chain(&self, expr: &Expr) -> Option<NormalizedMethodChain> {
        let mut chain_length = 0;
        let mut current = expr;

        loop {
            match current {
                Expr::MethodCall(method) => {
                    chain_length += 1;
                    current = &method.receiver;
                }
                Expr::Await(await_expr) => {
                    chain_length += 1;
                    current = &await_expr.base;
                }
                Expr::Field(field) => {
                    current = &field.base;
                }
                _ => break,
            }
        }

        if chain_length > 1 {
            Some(NormalizedMethodChain {
                chain_length,
                is_multiline_artifact: chain_length > 2,
            })
        } else {
            None
        }
    }

    /// Normalize string literals and format macros
    pub fn normalize_string_literal(&self, expr: &Expr) -> Option<NormalizedStringLiteral> {
        match expr {
            Expr::Lit(lit) => match &lit.lit {
                syn::Lit::Str(str_lit) => {
                    let value = str_lit.value();
                    let is_multiline = value.contains('\n');
                    Some(NormalizedStringLiteral {
                        is_multiline,
                        line_count: value.lines().count(),
                    })
                }
                _ => None,
            },
            Expr::Macro(macro_expr) => {
                // Check for format macros
                if let Some(ident) = macro_expr.mac.path.get_ident() {
                    let name = ident.to_string();
                    if name == "format"
                        || name == "println"
                        || name == "eprintln"
                        || name == "write"
                        || name == "writeln"
                    {
                        // Format macros often span multiple lines
                        return Some(NormalizedStringLiteral {
                            is_multiline: true,
                            line_count: 1,
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Normalize tuple destructuring patterns
    pub fn normalize_tuple_destructure(
        &self,
        pat: &syn::Pat,
    ) -> Option<NormalizedTupleDestructure> {
        match pat {
            syn::Pat::Tuple(tuple) => {
                let element_count = tuple.elems.len();
                Some(NormalizedTupleDestructure {
                    element_count,
                    is_multiline_artifact: element_count > 3,
                })
            }
            syn::Pat::TupleStruct(tuple_struct) => {
                let element_count = tuple_struct.elems.len();
                Some(NormalizedTupleDestructure {
                    element_count,
                    is_multiline_artifact: element_count > 3,
                })
            }
            _ => None,
        }
    }
}

impl SemanticNormalizer for RustSemanticNormalizer {
    type Input = Block;
    type Output = NormalizedBlock;

    fn normalize(&self, block: Self::Input) -> Self::Output {
        let mut statements = Vec::new();
        let mut metadata = FormattingMetadata {
            original_lines: 0,
            normalized_lines: 0,
            whitespace_changes: 0,
            multiline_expressions_normalized: 0,
        };

        for stmt in &block.stmts {
            statements.push(self.normalize_statement(stmt, &mut metadata));
        }

        let logical_structure = self.calculate_logical_structure(&statements);

        NormalizedBlock {
            statements,
            logical_structure,
            formatting_metadata: metadata,
        }
    }
}

impl RustSemanticNormalizer {
    fn normalize_statement(
        &self,
        stmt: &Stmt,
        metadata: &mut FormattingMetadata,
    ) -> NormalizedStatement {
        match stmt {
            Stmt::Local(local) => {
                // Check for multiline patterns in destructuring
                if let Some(tuple_destructure) = self.normalize_tuple_destructure(&local.pat) {
                    if tuple_destructure.is_multiline_artifact {
                        metadata.multiline_expressions_normalized += 1;
                    }
                }

                NormalizedStatement::Local(super::semantic_normalizer::NormalizedLocal {
                    pattern: format!("{}", quote::quote! { #local.pat }),
                    init: local
                        .init
                        .as_ref()
                        .map(|init| self.normalize_expression(&init.expr, metadata)),
                    is_multiline_pattern: self.is_multiline_pattern(&local.pat),
                })
            }
            Stmt::Expr(expr, _) => {
                NormalizedStatement::Expression(self.normalize_expression(expr, metadata))
            }
            Stmt::Item(item) => {
                let (name, is_multiline) = self.extract_item_info(item);
                if is_multiline {
                    metadata.multiline_expressions_normalized += 1;
                }

                NormalizedStatement::Declaration(
                    super::semantic_normalizer::NormalizedDeclaration {
                        name,
                        is_multiline_signature: is_multiline,
                    },
                )
            }
            Stmt::Macro(_) => NormalizedStatement::Expression(NormalizedExpression {
                expr_type: ExprType::Other,
                logical_components: vec![],
                is_multiline_artifact: false,
                original_expr: syn::parse_quote! { () },
            }),
        }
    }

    fn normalize_expression(
        &self,
        expr: &Expr,
        metadata: &mut FormattingMetadata,
    ) -> NormalizedExpression {
        let expr_type = self.classify_expression(expr);

        // Check for specific formatting artifacts
        let is_multiline_artifact = match expr {
            // Method chains formatted across lines
            Expr::MethodCall(_) => {
                if let Some(chain) = self.normalize_method_chain(expr) {
                    chain.is_multiline_artifact
                } else {
                    false
                }
            }
            // String literals or format macros
            Expr::Lit(_) | Expr::Macro(_) => {
                if let Some(string_lit) = self.normalize_string_literal(expr) {
                    string_lit.is_multiline
                } else {
                    false
                }
            }
            // Function calls with many arguments
            Expr::Call(call) => call.args.len() > 3,
            // Tuple expressions
            Expr::Tuple(tuple) => tuple.elems.len() > 3,
            // Closures with block bodies
            Expr::Closure(closure) => matches!(&*closure.body, Expr::Block(_)),
            _ => false,
        };

        if is_multiline_artifact {
            metadata.multiline_expressions_normalized += 1;
        }

        let logical_components = self.extract_logical_components(expr);

        NormalizedExpression {
            expr_type,
            logical_components,
            is_multiline_artifact,
            original_expr: expr.clone(),
        }
    }

    fn classify_expression(&self, expr: &Expr) -> ExprType {
        match expr {
            Expr::If(_) | Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) | Expr::Try(_) => {
                ExprType::ControlFlow
            }
            Expr::Match(expr_match) => ExprType::Match {
                arm_count: expr_match.arms.len(),
            },
            Expr::Binary(binary) if self.is_logical_operator(&binary.op) => ExprType::LogicalOp,
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

    fn is_logical_operator(&self, op: &syn::BinOp) -> bool {
        matches!(op, syn::BinOp::And(_) | syn::BinOp::Or(_))
    }

    fn extract_logical_components(&self, expr: &Expr) -> Vec<LogicalComponent> {
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
                // Don't count formatting of match arms as complexity
                let normalized_match = self.normalize_match(expr_match);
                if !normalized_match.has_multiline_patterns {
                    for _ in &expr_match.arms {
                        components.push(LogicalComponent {
                            component_type: ComponentType::Branch,
                            complexity_contribution: 1,
                        });
                    }
                } else {
                    // Reduce complexity contribution for formatted match expressions
                    components.push(LogicalComponent {
                        component_type: ComponentType::Branch,
                        complexity_contribution: expr_match.arms.len() as u32 / 2,
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
            Expr::Binary(binary) if self.is_logical_operator(&binary.op) => {
                components.push(LogicalComponent {
                    component_type: ComponentType::Operator,
                    complexity_contribution: 1,
                });
            }
            _ => {}
        }

        components
    }

    fn extract_item_info(&self, item: &syn::Item) -> (String, bool) {
        match item {
            syn::Item::Fn(f) => {
                let sig = self.normalize_signature(&f.sig);
                (f.sig.ident.to_string(), sig.is_multiline_artifact)
            }
            syn::Item::Struct(s) => (s.ident.to_string(), false),
            syn::Item::Enum(e) => (e.ident.to_string(), false),
            syn::Item::Trait(t) => (t.ident.to_string(), false),
            syn::Item::Type(t) => (t.ident.to_string(), false),
            syn::Item::Const(c) => (c.ident.to_string(), false),
            syn::Item::Static(s) => (s.ident.to_string(), false),
            _ => (String::from("<item>"), false),
        }
    }

    fn calculate_logical_structure(&self, statements: &[NormalizedStatement]) -> LogicalStructure {
        let depth = Self::calculate_max_depth(statements);
        let branching_factor = Self::calculate_branching_factor(statements);
        let has_early_return = Self::has_early_return(statements);

        LogicalStructure {
            depth,
            branching_factor,
            has_early_return,
        }
    }

    fn calculate_max_depth(statements: &[NormalizedStatement]) -> usize {
        statements.iter().fold((0, 0), |(max_depth, current_depth), stmt| {
            match stmt {
                NormalizedStatement::Control(_) => {
                    let new_depth = current_depth + 1;
                    (max_depth.max(new_depth), current_depth)
                }
                _ => (max_depth, current_depth)
            }
        }).0
    }

    fn calculate_branching_factor(statements: &[NormalizedStatement]) -> usize {
        statements.iter().map(|stmt| {
            match stmt {
                NormalizedStatement::Control(control) => {
                    Self::is_branching_control(&control.control_type) as usize
                }
                NormalizedStatement::Expression(expr) if expr.expr_type == ExprType::LogicalOp => 1,
                _ => 0
            }
        }).sum()
    }

    fn is_branching_control(control_type: &ControlType) -> bool {
        matches!(control_type, ControlType::If | ControlType::Match)
    }

    fn has_early_return(statements: &[NormalizedStatement]) -> bool {
        statements.iter().enumerate().any(|(i, stmt)| {
            match stmt {
                NormalizedStatement::Control(control) => {
                    control.body.logical_structure.has_early_return
                }
                NormalizedStatement::Expression(expr) if i < statements.len() - 1 => {
                    matches!(&expr.original_expr, Expr::Return(_))
                }
                _ => false
            }
        })
    }
}

/// Normalized representation of a function signature
#[derive(Debug, Clone)]
pub struct NormalizedSignature {
    pub name: String,
    pub param_count: usize,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_multiline_artifact: bool,
}

/// Normalized representation of a match expression
#[derive(Debug, Clone)]
pub struct NormalizedMatch {
    pub arm_count: usize,
    pub patterns: Vec<NormalizedPattern>,
    pub has_guard: bool,
    pub has_multiline_patterns: bool,
}

/// Normalized representation of a match pattern
#[derive(Debug, Clone)]
pub struct NormalizedPattern {
    pub is_wildcard: bool,
    pub has_guard: bool,
    pub is_multiline: bool,
}

/// Normalized representation of a method chain
#[derive(Debug, Clone)]
pub struct NormalizedMethodChain {
    pub chain_length: usize,
    pub is_multiline_artifact: bool,
}

/// Normalized representation of a string literal
#[derive(Debug, Clone)]
pub struct NormalizedStringLiteral {
    pub is_multiline: bool,
    pub line_count: usize,
}

/// Normalized representation of tuple destructuring
#[derive(Debug, Clone)]
pub struct NormalizedTupleDestructure {
    pub element_count: usize,
    pub is_multiline_artifact: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_normalize_multiline_signature() {
        let sig: syn::Signature = parse_quote! {
            fn complex_function(
                param1: String,
                param2: Vec<u32>,
                param3: HashMap<String, Value>,
                param4: Option<Box<dyn Future>>
            ) -> Result<ComplexType, Error>
        };

        let normalizer = RustSemanticNormalizer::new();
        let normalized = normalizer.normalize_signature(&sig);

        assert_eq!(normalized.param_count, 4);
        assert!(normalized.is_multiline_artifact);
    }

    #[test]
    fn test_normalize_method_chain() {
        let expr: Expr = parse_quote! {
            foo.bar()
                .baz()
                .qux()
                .await
        };

        let normalizer = RustSemanticNormalizer::new();
        let chain = normalizer.normalize_method_chain(&expr);

        assert!(chain.is_some());
        let chain = chain.unwrap();
        assert_eq!(chain.chain_length, 4);
        assert!(chain.is_multiline_artifact);
    }

    #[test]
    fn test_normalize_match_with_guards() {
        let expr: Expr = parse_quote! {
            match value {
                Some(x) if x > 0 => "positive",
                Some(x) if x < 0 => "negative",
                Some(_) => "zero",
                None => "none",
            }
        };

        if let Expr::Match(expr_match) = expr {
            let normalizer = RustSemanticNormalizer::new();
            let normalized = normalizer.normalize_match(&expr_match);

            assert_eq!(normalized.arm_count, 4);
            assert!(normalized.has_guard);
        }
    }

    #[test]
    fn test_is_branching_control() {
        assert!(RustSemanticNormalizer::is_branching_control(&ControlType::If));
        assert!(RustSemanticNormalizer::is_branching_control(&ControlType::Match));
        assert!(!RustSemanticNormalizer::is_branching_control(&ControlType::Loop));
        assert!(!RustSemanticNormalizer::is_branching_control(&ControlType::While));
        assert!(!RustSemanticNormalizer::is_branching_control(&ControlType::For));
    }
}
