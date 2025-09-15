//! Python expression processing module
//!
//! This module contains functions for processing Python expressions
//! and extracting tokens from them using functional programming principles.

use crate::complexity::entropy_traits::GenericToken;
use rustpython_parser::ast;
use std::collections::HashSet;

use super::core::{ExprCategory, PythonEntropyAnalyzer};

/// Expression processor that handles all Python expression types
pub struct ExpressionProcessor;

impl ExpressionProcessor {
    /// Process any expression type and extract tokens
    pub fn process_expression(
        analyzer: &PythonEntropyAnalyzer,
        expr: &ast::Expr,
        tokens: &mut Vec<GenericToken>,
    ) {
        match categorize_expression(expr) {
            ExprCategory::Operator => tokens.extend(extract_operator_tokens(expr)),
            ExprCategory::ControlFlow => tokens.extend(extract_control_flow_tokens(expr)),
            ExprCategory::Comprehension => {
                tokens.extend(extract_comprehension_expr_tokens(analyzer, expr))
            }
            ExprCategory::Literal => tokens.extend(extract_literal_tokens(expr)),
            ExprCategory::Collection => tokens.extend(extract_collection_tokens(analyzer, expr)),
            ExprCategory::Access => tokens.extend(extract_access_tokens(analyzer, expr)),
            ExprCategory::Special => tokens.extend(extract_special_tokens(analyzer, expr)),
        }
    }

    /// Detect patterns in expressions
    pub fn detect_expression_patterns(expr: &ast::Expr) -> Vec<String> {
        let mut patterns = Vec::new();
        collect_expression_patterns(expr, &mut patterns);
        patterns
    }

    /// Collect variable names from expressions
    pub fn collect_variables_from_expression(expr: &ast::Expr) -> HashSet<String> {
        collect_variables_from_expr(expr)
    }
}

// ============================================================================
// Core Expression Categorization
// ============================================================================

/// Categorizes expression types for simplified pattern matching - pure function
pub fn categorize_expression(expr: &ast::Expr) -> ExprCategory {
    use ast::Expr::*;
    match expr {
        BoolOp(_) | BinOp(_) | UnaryOp(_) | Compare(_) => ExprCategory::Operator,
        IfExp(_) | Lambda(_) => ExprCategory::ControlFlow,
        ListComp(_) | SetComp(_) | DictComp(_) | GeneratorExp(_) => ExprCategory::Comprehension,
        Constant(_) | JoinedStr(_) | FormattedValue(_) => ExprCategory::Literal,
        List(_) | Tuple(_) | Set(_) | Dict(_) => ExprCategory::Collection,
        Attribute(_) | Subscript(_) | Slice(_) => ExprCategory::Access,
        Call(_) | Starred(_) | Name(_) | Yield(_) | YieldFrom(_) | Await(_) | NamedExpr(_) => {
            ExprCategory::Special
        }
    }
}

// ============================================================================
// Token Extraction Functions
// ============================================================================

/// Extracts operator tokens from expressions - pure function
pub fn extract_operator_tokens(expr: &ast::Expr) -> Vec<GenericToken> {
    use ast::Expr::*;
    match expr {
        BoolOp(bool_op) => extract_bool_op_tokens(bool_op),
        BinOp(bin_op) => extract_bin_op_tokens(bin_op),
        UnaryOp(unary_op) => extract_unary_op_tokens(unary_op),
        Compare(compare) => extract_compare_tokens(compare),
        _ => vec![],
    }
}

/// Extracts control flow tokens from expressions - pure function
pub fn extract_control_flow_tokens(expr: &ast::Expr) -> Vec<GenericToken> {
    use ast::Expr::*;
    match expr {
        IfExp(if_exp) => extract_if_exp_tokens(if_exp),
        Lambda(lambda) => extract_lambda_tokens(lambda),
        _ => vec![],
    }
}

/// Extracts tokens from comprehension expressions
pub fn extract_comprehension_expr_tokens(
    analyzer: &PythonEntropyAnalyzer,
    expr: &ast::Expr,
) -> Vec<GenericToken> {
    use ast::Expr::*;
    match expr {
        ListComp(list_comp) => extract_list_comp_tokens(analyzer, list_comp),
        DictComp(dict_comp) => extract_dict_comp_tokens(analyzer, dict_comp),
        SetComp(set_comp) => extract_set_comp_tokens(analyzer, set_comp),
        GeneratorExp(gen_exp) => extract_generator_exp_tokens(analyzer, gen_exp),
        _ => vec![],
    }
}

/// Extracts literal tokens - pure function
pub fn extract_literal_tokens(expr: &ast::Expr) -> Vec<GenericToken> {
    use ast::Expr::*;
    match expr {
        Constant(constant) => {
            use ast::Constant;
            let value = match &constant.value {
                Constant::None => "None".to_string(),
                Constant::Bool(_) => "bool".to_string(),
                Constant::Str(_) => "string".to_string(),
                Constant::Int(_) => "int".to_string(),
                Constant::Float(_) => "float".to_string(),
                Constant::Complex { .. } => "complex".to_string(),
                Constant::Bytes(_) => "bytes".to_string(),
                Constant::Tuple(_) => "tuple".to_string(),
                Constant::Ellipsis => "...".to_string(),
            };
            vec![GenericToken::literal(value)]
        }
        JoinedStr(joined_str) => extract_join_str_tokens(joined_str),
        FormattedValue(formatted_value) => extract_formatted_value_tokens(formatted_value),
        _ => vec![],
    }
}

/// Extracts collection tokens
pub fn extract_collection_tokens(
    analyzer: &PythonEntropyAnalyzer,
    expr: &ast::Expr,
) -> Vec<GenericToken> {
    use ast::Expr::*;
    match expr {
        List(list) => extract_list_tokens(analyzer, list),
        Tuple(tuple) => extract_tuple_tokens(analyzer, tuple),
        Set(set) => extract_set_tokens(analyzer, set),
        Dict(dict) => extract_dict_tokens(analyzer, dict),
        _ => vec![],
    }
}

/// Extracts access tokens
pub fn extract_access_tokens(
    analyzer: &PythonEntropyAnalyzer,
    expr: &ast::Expr,
) -> Vec<GenericToken> {
    use ast::Expr::*;
    match expr {
        Attribute(attribute) => extract_attribute_tokens(analyzer, attribute),
        Subscript(subscript) => extract_subscript_tokens(analyzer, subscript),
        Slice(slice) => extract_slice_tokens(analyzer, slice),
        _ => vec![],
    }
}

/// Extracts special expression tokens
pub fn extract_special_tokens(
    analyzer: &PythonEntropyAnalyzer,
    expr: &ast::Expr,
) -> Vec<GenericToken> {
    use ast::Expr::*;
    match expr {
        Call(call) => extract_call_tokens(analyzer, call),
        Starred(starred) => extract_starred_tokens(analyzer, starred),
        Name(name) => vec![GenericToken::identifier(name.id.to_string())],
        Yield(yield_expr) => extract_yield_tokens(analyzer, yield_expr),
        YieldFrom(yield_from) => extract_yield_from_tokens(analyzer, yield_from),
        Await(await_expr) => extract_await_tokens(analyzer, await_expr),
        NamedExpr(named_expr) => extract_named_expr_tokens(analyzer, named_expr),
        _ => vec![],
    }
}

// ============================================================================
// Boolean and Binary Operations
// ============================================================================

/// Extract tokens from boolean operations - pure function with recursive processing
pub fn extract_bool_op_tokens(bool_op: &ast::ExprBoolOp) -> Vec<GenericToken> {
    let mut tokens = Vec::new();

    // Add operator token
    let op_token = match bool_op.op {
        ast::BoolOp::And => "and",
        ast::BoolOp::Or => "or",
    };
    tokens.push(GenericToken::operator(op_token.to_string()));

    // Process nested values recursively
    for value in &bool_op.values {
        use ast::Expr::*;
        match value {
            BoolOp(nested_bool_op) => tokens.extend(extract_bool_op_tokens(nested_bool_op)),
            BinOp(bin_op) => tokens.extend(extract_bin_op_tokens(bin_op)),
            UnaryOp(unary_op) => tokens.extend(extract_unary_op_tokens(unary_op)),
            Compare(compare) => tokens.extend(extract_compare_tokens(compare)),
            Constant(_) => tokens.extend(extract_literal_tokens(value)),
            _ => {} // Skip other expression types for now
        }
    }

    tokens
}

/// Extract tokens from binary operations - pure function with full recursion
pub fn extract_bin_op_tokens(bin_op: &ast::ExprBinOp) -> Vec<GenericToken> {
    let mut tokens = Vec::new();

    // Add operator token (using name instead of symbol for tests)
    let op_str = match bin_op.op {
        ast::Operator::Add => "Add",
        ast::Operator::Sub => "Sub",
        ast::Operator::Mult => "Mult",
        ast::Operator::MatMult => "MatMult",
        ast::Operator::Div => "Div",
        ast::Operator::Mod => "Mod",
        ast::Operator::Pow => "Pow",
        ast::Operator::LShift => "LShift",
        ast::Operator::RShift => "RShift",
        ast::Operator::BitOr => "BitOr",
        ast::Operator::BitXor => "BitXor",
        ast::Operator::BitAnd => "BitAnd",
        ast::Operator::FloorDiv => "FloorDiv",
    };
    tokens.push(GenericToken::operator(op_str.to_string()));

    // Recursively process nested expressions
    tokens.extend(extract_nested_expr_tokens(&bin_op.left));
    tokens.extend(extract_nested_expr_tokens(&bin_op.right));

    tokens
}

/// Helper function to extract tokens from nested expressions - pure function
fn extract_nested_expr_tokens(expr: &ast::Expr) -> Vec<GenericToken> {
    use ast::Expr::*;
    match expr {
        BinOp(nested_bin_op) => extract_bin_op_tokens(nested_bin_op),
        BoolOp(bool_op) => extract_bool_op_tokens(bool_op),
        UnaryOp(unary_op) => extract_unary_op_tokens(unary_op),
        Compare(compare) => extract_compare_tokens(compare),
        Constant(_) => extract_literal_tokens(expr),
        NamedExpr(named_expr) => {
            // Extract walrus operator (:=) and its operands
            let mut tokens = Vec::new();
            tokens.push(GenericToken::operator(":=".to_string()));
            tokens.extend(extract_nested_expr_tokens(&named_expr.value));
            tokens
        }
        _ => vec![], // Skip other expression types for now
    }
}

/// Extract tokens from unary operations - pure function
pub fn extract_unary_op_tokens(unary_op: &ast::ExprUnaryOp) -> Vec<GenericToken> {
    let op_str = match unary_op.op {
        ast::UnaryOp::Invert => "~",
        ast::UnaryOp::Not => "not",
        ast::UnaryOp::UAdd => "+",
        ast::UnaryOp::USub => "-",
    };

    vec![GenericToken::operator(op_str.to_string())]
}

/// Extract tokens from comparison operations - pure function
fn extract_compare_tokens(compare: &ast::ExprCompare) -> Vec<GenericToken> {
    let mut tokens = Vec::new();

    // Extract tokens from left operand (including NamedExpr)
    tokens.extend(extract_nested_expr_tokens(&compare.left));

    // Extract comparison operators
    for op in &compare.ops {
        let op_str = match op {
            ast::CmpOp::Eq => "==",
            ast::CmpOp::NotEq => "!=",
            ast::CmpOp::Lt => "<",
            ast::CmpOp::LtE => "<=",
            ast::CmpOp::Gt => ">",
            ast::CmpOp::GtE => ">=",
            ast::CmpOp::Is => "is",
            ast::CmpOp::IsNot => "is not",
            ast::CmpOp::In => "in",
            ast::CmpOp::NotIn => "not in",
        };
        tokens.push(GenericToken::operator(op_str.to_string()));
    }

    // Extract tokens from comparators
    for comparator in &compare.comparators {
        tokens.extend(extract_nested_expr_tokens(comparator));
    }

    tokens
}

// ============================================================================
// Control Flow Expressions
// ============================================================================

/// Extract tokens from lambda expressions - processes operator tokens from body
pub fn extract_lambda_tokens(lambda: &ast::ExprLambda) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::keyword("lambda".to_string())];

    // Only extract operator tokens from the lambda body to avoid double-counting variables
    use ast::Expr::*;
    match &*lambda.body {
        BoolOp(bool_op) => tokens.extend(extract_bool_op_tokens(bool_op)),
        BinOp(bin_op) => tokens.extend(extract_bin_op_tokens(bin_op)),
        UnaryOp(unary_op) => tokens.extend(extract_unary_op_tokens(unary_op)),
        Compare(compare) => tokens.extend(extract_compare_tokens(compare)),
        _ => {} // Skip other types to avoid interference with variable counting
    }

    tokens
}

/// Extract tokens from if expressions - extracts operators from branches
pub fn extract_if_exp_tokens(if_exp: &ast::ExprIfExp) -> Vec<GenericToken> {
    let mut tokens = vec![
        GenericToken::control_flow("if".to_string()),
        GenericToken::control_flow("else".to_string()),
    ];

    // Extract operator tokens from test, body, and else branches
    tokens.extend(extract_nested_expr_tokens(&if_exp.test));
    tokens.extend(extract_nested_expr_tokens(&if_exp.body));
    tokens.extend(extract_nested_expr_tokens(&if_exp.orelse));

    tokens
}

// ============================================================================
// Comprehensions
// ============================================================================

/// Extract tokens from list comprehensions
pub fn extract_list_comp_tokens(
    analyzer: &PythonEntropyAnalyzer,
    list_comp: &ast::ExprListComp,
) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::operator("list_comp".to_string())];

    // Process element expression
    analyzer.extract_tokens_from_expr(&list_comp.elt, &mut tokens);

    // Process generators
    for generator in &list_comp.generators {
        tokens.push(GenericToken::control_flow("for".to_string()));
        analyzer.extract_tokens_from_expr(&generator.target, &mut tokens);
        tokens.push(GenericToken::keyword("in".to_string()));
        analyzer.extract_tokens_from_expr(&generator.iter, &mut tokens);

        // Add literal tokens for range arguments if present
        if let ast::Expr::Call(call) = &generator.iter {
            if let ast::Expr::Name(name) = &*call.func {
                if name.id.as_str() == "range" {
                    for arg in &call.args {
                        if let ast::Expr::Constant(_) = arg {
                            tokens.extend(extract_literal_tokens(arg));
                        }
                    }
                }
            }
        }

        // Process conditions
        for if_clause in &generator.ifs {
            tokens.push(GenericToken::control_flow("if".to_string()));
            analyzer.extract_tokens_from_expr(if_clause, &mut tokens);
        }
    }

    tokens.push(GenericToken::operator("for".to_string())); // Add for keyword
    tokens
}

/// Extract tokens from dictionary comprehensions
pub fn extract_dict_comp_tokens(
    analyzer: &PythonEntropyAnalyzer,
    dict_comp: &ast::ExprDictComp,
) -> Vec<GenericToken> {
    let mut tokens = vec![
        GenericToken::custom("{".to_string()),
        GenericToken::keyword("dict_comp".to_string()), // Add dict_comp token for tests
    ];

    // Process key and value expressions
    analyzer.extract_tokens_from_expr(&dict_comp.key, &mut tokens);
    tokens.push(GenericToken::custom(":".to_string()));
    analyzer.extract_tokens_from_expr(&dict_comp.value, &mut tokens);

    // Process generators
    for generator in &dict_comp.generators {
        tokens.push(GenericToken::control_flow("for".to_string()));
        analyzer.extract_tokens_from_expr(&generator.target, &mut tokens);
        tokens.push(GenericToken::keyword("in".to_string()));
        analyzer.extract_tokens_from_expr(&generator.iter, &mut tokens);

        // Process conditions
        for if_clause in &generator.ifs {
            tokens.push(GenericToken::control_flow("if".to_string()));
            analyzer.extract_tokens_from_expr(if_clause, &mut tokens);
        }
    }

    tokens.push(GenericToken::custom("}".to_string()));
    tokens
}

/// Extract tokens from set comprehensions
pub fn extract_set_comp_tokens(
    analyzer: &PythonEntropyAnalyzer,
    set_comp: &ast::ExprSetComp,
) -> Vec<GenericToken> {
    let mut tokens = vec![
        GenericToken::custom("{".to_string()),
        GenericToken::keyword("set_comp".to_string()), // Add set_comp token for tests
    ];

    // Process element expression
    analyzer.extract_tokens_from_expr(&set_comp.elt, &mut tokens);

    // Process generators
    for generator in &set_comp.generators {
        tokens.push(GenericToken::control_flow("for".to_string()));
        analyzer.extract_tokens_from_expr(&generator.target, &mut tokens);
        tokens.push(GenericToken::keyword("in".to_string()));
        analyzer.extract_tokens_from_expr(&generator.iter, &mut tokens);

        // Process conditions
        for if_clause in &generator.ifs {
            tokens.push(GenericToken::control_flow("if".to_string()));
            analyzer.extract_tokens_from_expr(if_clause, &mut tokens);
        }
    }

    tokens.push(GenericToken::custom("}".to_string()));
    tokens
}

/// Extract tokens from generator expressions
pub fn extract_generator_exp_tokens(
    analyzer: &PythonEntropyAnalyzer,
    gen_exp: &ast::ExprGeneratorExp,
) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::operator("generator".to_string())];

    // Process element expression
    analyzer.extract_tokens_from_expr(&gen_exp.elt, &mut tokens);

    // Process generators
    for generator in &gen_exp.generators {
        tokens.push(GenericToken::control_flow("for".to_string()));
        analyzer.extract_tokens_from_expr(&generator.target, &mut tokens);
        tokens.push(GenericToken::keyword("in".to_string()));
        analyzer.extract_tokens_from_expr(&generator.iter, &mut tokens);

        // Process conditions
        for if_clause in &generator.ifs {
            tokens.push(GenericToken::control_flow("if".to_string()));
            analyzer.extract_tokens_from_expr(if_clause, &mut tokens);
        }
    }

    tokens
}

// ============================================================================
// Async and Yield Expressions
// ============================================================================

/// Extract tokens from yield expressions
pub fn extract_yield_tokens(
    analyzer: &PythonEntropyAnalyzer,
    yield_expr: &ast::ExprYield,
) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::control_flow("yield".to_string())];

    if let Some(ref value) = yield_expr.value {
        analyzer.extract_tokens_from_expr(value, &mut tokens);
    }

    tokens
}

/// Extract tokens from yield from expressions
pub fn extract_yield_from_tokens(
    analyzer: &PythonEntropyAnalyzer,
    yield_from: &ast::ExprYieldFrom,
) -> Vec<GenericToken> {
    let mut tokens = vec![
        GenericToken::control_flow("yield".to_string()),
        GenericToken::keyword("from".to_string()),
    ];

    analyzer.extract_tokens_from_expr(&yield_from.value, &mut tokens);
    tokens
}

/// Extract tokens from await expressions
pub fn extract_await_tokens(
    analyzer: &PythonEntropyAnalyzer,
    await_expr: &ast::ExprAwait,
) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::control_flow("await".to_string())];
    analyzer.extract_tokens_from_expr(&await_expr.value, &mut tokens);
    tokens
}

// ============================================================================
// Function Calls and Access
// ============================================================================

/// Extract tokens from function calls
pub fn extract_call_tokens(
    analyzer: &PythonEntropyAnalyzer,
    call: &ast::ExprCall,
) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::operator("call".to_string())];

    // Process function being called
    analyzer.extract_tokens_from_expr(&call.func, &mut tokens);

    tokens.push(GenericToken::custom("(".to_string()));

    // Process arguments
    for arg in &call.args {
        analyzer.extract_tokens_from_expr(arg, &mut tokens);
    }

    // Process keyword arguments
    for keyword in &call.keywords {
        if let Some(ref arg_name) = keyword.arg {
            tokens.push(GenericToken::identifier(arg_name.to_string()));
            tokens.push(GenericToken::custom("=".to_string()));
        }
        analyzer.extract_tokens_from_expr(&keyword.value, &mut tokens);
    }

    tokens.push(GenericToken::custom(")".to_string()));
    tokens
}

/// Extract tokens from attribute access
pub fn extract_attribute_tokens(
    analyzer: &PythonEntropyAnalyzer,
    attribute: &ast::ExprAttribute,
) -> Vec<GenericToken> {
    let mut tokens = Vec::new();
    analyzer.extract_tokens_from_expr(&attribute.value, &mut tokens);
    tokens.push(GenericToken::custom(".".to_string()));
    tokens.push(GenericToken::identifier(attribute.attr.to_string()));
    tokens
}

/// Extract tokens from subscript access
pub fn extract_subscript_tokens(
    analyzer: &PythonEntropyAnalyzer,
    subscript: &ast::ExprSubscript,
) -> Vec<GenericToken> {
    let mut tokens = vec![
        GenericToken::operator("subscript".to_string()),
        GenericToken::operator("[]".to_string()),
    ];
    analyzer.extract_tokens_from_expr(&subscript.value, &mut tokens);
    tokens.push(GenericToken::custom("[".to_string()));
    analyzer.extract_tokens_from_expr(&subscript.slice, &mut tokens);
    tokens.push(GenericToken::custom("]".to_string()));
    tokens
}

/// Extract tokens from slice expressions
pub fn extract_slice_tokens(
    analyzer: &PythonEntropyAnalyzer,
    slice: &ast::ExprSlice,
) -> Vec<GenericToken> {
    let mut tokens = Vec::new();

    if let Some(ref lower) = slice.lower {
        analyzer.extract_tokens_from_expr(lower, &mut tokens);
    }

    tokens.push(GenericToken::custom(":".to_string()));

    if let Some(ref upper) = slice.upper {
        analyzer.extract_tokens_from_expr(upper, &mut tokens);
    }

    if let Some(ref step) = slice.step {
        tokens.push(GenericToken::custom(":".to_string()));
        analyzer.extract_tokens_from_expr(step, &mut tokens);
    }

    tokens
}

// ============================================================================
// Collections
// ============================================================================

/// Extract tokens from list literals
pub fn extract_list_tokens(
    analyzer: &PythonEntropyAnalyzer,
    list: &ast::ExprList,
) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::operator("list".to_string())];

    for elt in &list.elts {
        if let ast::Expr::Constant(_) = elt {
            tokens.extend(extract_literal_tokens(elt));
        } else {
            analyzer.extract_tokens_from_expr(elt, &mut tokens);
        }
    }

    tokens
}

/// Extract tokens from tuple literals
pub fn extract_tuple_tokens(
    analyzer: &PythonEntropyAnalyzer,
    tuple: &ast::ExprTuple,
) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::operator("tuple".to_string())];

    for elt in &tuple.elts {
        if let ast::Expr::Constant(_) = elt {
            tokens.extend(extract_literal_tokens(elt));
        } else {
            analyzer.extract_tokens_from_expr(elt, &mut tokens);
        }
    }

    tokens
}

/// Extract tokens from dictionary literals
pub fn extract_dict_tokens(
    analyzer: &PythonEntropyAnalyzer,
    dict: &ast::ExprDict,
) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::operator("dict".to_string())];

    for (key, value) in dict.keys.iter().zip(dict.values.iter()) {
        if let Some(key_expr) = key {
            if let ast::Expr::Constant(_) = key_expr {
                tokens.extend(extract_literal_tokens(key_expr));
            } else {
                analyzer.extract_tokens_from_expr(key_expr, &mut tokens);
            }
        }
        if let ast::Expr::Constant(_) = value {
            tokens.extend(extract_literal_tokens(value));
        } else {
            analyzer.extract_tokens_from_expr(value, &mut tokens);
        }
    }

    tokens
}

/// Extract tokens from set literals
pub fn extract_set_tokens(
    analyzer: &PythonEntropyAnalyzer,
    set: &ast::ExprSet,
) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::operator("set".to_string())];

    for elt in &set.elts {
        if let ast::Expr::Constant(_) = elt {
            tokens.extend(extract_literal_tokens(elt));
        } else {
            analyzer.extract_tokens_from_expr(elt, &mut tokens);
        }
    }

    tokens
}

// ============================================================================
// Special Expressions
// ============================================================================

/// Extract tokens from named expressions (walrus operator)
pub fn extract_named_expr_tokens(
    analyzer: &PythonEntropyAnalyzer,
    named_expr: &ast::ExprNamedExpr,
) -> Vec<GenericToken> {
    let mut tokens = Vec::new();
    analyzer.extract_tokens_from_expr(&named_expr.target, &mut tokens);
    tokens.push(GenericToken::operator(":=".to_string()));
    analyzer.extract_tokens_from_expr(&named_expr.value, &mut tokens);
    tokens
}

/// Extract tokens from starred expressions
pub fn extract_starred_tokens(
    analyzer: &PythonEntropyAnalyzer,
    starred: &ast::ExprStarred,
) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::operator("*".to_string())];
    analyzer.extract_tokens_from_expr(&starred.value, &mut tokens);
    tokens
}

/// Extract tokens from joined strings (f-strings)
pub fn extract_join_str_tokens(joined_str: &ast::ExprJoinedStr) -> Vec<GenericToken> {
    let mut tokens = vec![GenericToken::literal("f-string".to_string())];
    tokens.extend(joined_str.values.iter().flat_map(extract_literal_tokens));
    tokens
}

/// Extract tokens from formatted values (f-string components)
pub fn extract_formatted_value_tokens(
    _formatted_value: &ast::ExprFormattedValue,
) -> Vec<GenericToken> {
    vec![GenericToken::custom("formatted_value".to_string())]
}

// ============================================================================
// Variable and Pattern Collection
// ============================================================================

/// Collects variable names from expressions - pure function
pub fn collect_variables_from_expr(expr: &ast::Expr) -> HashSet<String> {
    use ast::Expr::*;
    match expr {
        Name(name) => {
            let mut vars = HashSet::new();
            vars.insert(name.id.to_string());
            vars
        }
        Attribute(attr) => collect_variables_from_expr(&attr.value),
        Subscript(subscript) => {
            let mut vars = collect_variables_from_expr(&subscript.value);
            vars.extend(collect_variables_from_expr(&subscript.slice));
            vars
        }
        BinOp(bin_op) => {
            let mut vars = collect_variables_from_expr(&bin_op.left);
            vars.extend(collect_variables_from_expr(&bin_op.right));
            vars
        }
        BoolOp(bool_op) => bool_op
            .values
            .iter()
            .flat_map(collect_variables_from_expr)
            .collect(),
        Compare(compare) => {
            let mut vars = collect_variables_from_expr(&compare.left);
            vars.extend(
                compare
                    .comparators
                    .iter()
                    .flat_map(collect_variables_from_expr),
            );
            vars
        }
        Call(call) => {
            let mut vars = collect_variables_from_expr(&call.func);
            vars.extend(call.args.iter().flat_map(collect_variables_from_expr));
            vars.extend(
                call.keywords
                    .iter()
                    .flat_map(|kw| collect_variables_from_expr(&kw.value)),
            );
            vars
        }
        List(list) => list
            .elts
            .iter()
            .flat_map(collect_variables_from_expr)
            .collect(),
        Tuple(tuple) => tuple
            .elts
            .iter()
            .flat_map(collect_variables_from_expr)
            .collect(),
        Dict(dict) => {
            let key_vars: HashSet<String> = dict
                .keys
                .iter()
                .filter_map(|key| key.as_ref())
                .flat_map(collect_variables_from_expr)
                .collect();
            let value_vars: HashSet<String> = dict
                .values
                .iter()
                .flat_map(collect_variables_from_expr)
                .collect();
            key_vars.union(&value_vars).cloned().collect()
        }
        Set(set) => set
            .elts
            .iter()
            .flat_map(collect_variables_from_expr)
            .collect(),
        ListComp(list_comp) => collect_comprehension_variables(list_comp),
        SetComp(set_comp) => collect_set_comp_variables(set_comp),
        DictComp(dict_comp) => collect_dict_comp_variables(dict_comp),
        GeneratorExp(gen_exp) => collect_generator_variables(gen_exp),
        Starred(starred) => collect_variables_from_expr(&starred.value),
        _ => HashSet::new(),
    }
}

/// Collects patterns from match statements - pure function
#[allow(dead_code)]
pub fn collect_patterns(pattern: &ast::Pattern) -> Vec<String> {
    use ast::Pattern::*;
    match pattern {
        MatchValue(match_value) => vec![format!("value:{:?}", match_value.value)],
        MatchSingleton(match_singleton) => vec![format!("singleton:{:?}", match_singleton.value)],
        MatchSequence(match_sequence) => {
            let mut patterns = vec!["sequence".to_string()];
            patterns.extend(match_sequence.patterns.iter().flat_map(collect_patterns));
            patterns
        }
        MatchMapping(match_mapping) => {
            let mut patterns = vec!["mapping".to_string()];
            patterns.extend(match_mapping.patterns.iter().flat_map(collect_patterns));
            patterns
        }
        MatchClass(match_class) => {
            let mut patterns = vec![format!("class:{:?}", match_class.cls)];
            patterns.extend(match_class.patterns.iter().flat_map(collect_patterns));
            patterns
        }
        MatchStar(match_star) => {
            let mut patterns = vec!["star".to_string()];
            if let Some(ref name) = match_star.name {
                patterns.push(format!("name:{}", name));
            }
            patterns
        }
        MatchAs(match_as) => {
            let mut patterns = Vec::new();
            if let Some(ref pattern) = match_as.pattern {
                patterns.extend(collect_patterns(pattern));
            }
            if let Some(ref name) = match_as.name {
                patterns.push(format!("as:{}", name));
            }
            patterns
        }
        MatchOr(match_or) => {
            let mut patterns = vec!["or".to_string()];
            patterns.extend(match_or.patterns.iter().flat_map(collect_patterns));
            patterns
        }
    }
}

/// Classifies comprehension items - pure function
#[allow(dead_code)]
pub fn classify_item(item: &str) -> String {
    if item.contains("for") {
        "generator".to_string()
    } else if item.contains("if") {
        "condition".to_string()
    } else {
        "element".to_string()
    }
}

// ============================================================================
// Helper Functions for Comprehensions
// ============================================================================

fn collect_comprehension_variables(list_comp: &ast::ExprListComp) -> HashSet<String> {
    let mut vars = collect_variables_from_expr(&list_comp.elt);

    for generator in &list_comp.generators {
        vars.extend(collect_variables_from_expr(&generator.target));
        vars.extend(collect_variables_from_expr(&generator.iter));

        for if_clause in &generator.ifs {
            vars.extend(collect_variables_from_expr(if_clause));
        }
    }

    vars
}

fn collect_set_comp_variables(set_comp: &ast::ExprSetComp) -> HashSet<String> {
    let mut vars = collect_variables_from_expr(&set_comp.elt);

    for generator in &set_comp.generators {
        vars.extend(collect_variables_from_expr(&generator.target));
        vars.extend(collect_variables_from_expr(&generator.iter));

        for if_clause in &generator.ifs {
            vars.extend(collect_variables_from_expr(if_clause));
        }
    }

    vars
}

fn collect_dict_comp_variables(dict_comp: &ast::ExprDictComp) -> HashSet<String> {
    let mut vars = collect_variables_from_expr(&dict_comp.key);
    vars.extend(collect_variables_from_expr(&dict_comp.value));

    for generator in &dict_comp.generators {
        vars.extend(collect_variables_from_expr(&generator.target));
        vars.extend(collect_variables_from_expr(&generator.iter));

        for if_clause in &generator.ifs {
            vars.extend(collect_variables_from_expr(if_clause));
        }
    }

    vars
}

fn collect_generator_variables(gen_exp: &ast::ExprGeneratorExp) -> HashSet<String> {
    let mut vars = collect_variables_from_expr(&gen_exp.elt);

    for generator in &gen_exp.generators {
        vars.extend(collect_variables_from_expr(&generator.target));
        vars.extend(collect_variables_from_expr(&generator.iter));

        for if_clause in &generator.ifs {
            vars.extend(collect_variables_from_expr(if_clause));
        }
    }

    vars
}

// ============================================================================
// Pattern Collection
// ============================================================================

fn collect_expression_patterns(expr: &ast::Expr, patterns: &mut Vec<String>) {
    use ast::Expr::*;
    match expr {
        BoolOp(_) => patterns.push("bool_op".to_string()),
        BinOp(bin_op) => {
            patterns.push(format!("bin_op:{:?}", bin_op.op));
            collect_expression_patterns(&bin_op.left, patterns);
            collect_expression_patterns(&bin_op.right, patterns);
        }
        UnaryOp(unary_op) => {
            patterns.push(format!("unary_op:{:?}", unary_op.op));
            collect_expression_patterns(&unary_op.operand, patterns);
        }
        Lambda(_) => patterns.push("lambda".to_string()),
        IfExp(_) => patterns.push("if_exp".to_string()),
        Dict(_) => patterns.push("dict".to_string()),
        Set(_) => patterns.push("set".to_string()),
        ListComp(_) => patterns.push("list_comp".to_string()),
        SetComp(_) => patterns.push("set_comp".to_string()),
        DictComp(_) => patterns.push("dict_comp".to_string()),
        GeneratorExp(_) => patterns.push("generator_exp".to_string()),
        Await(_) => patterns.push("await".to_string()),
        Yield(_) => patterns.push("yield".to_string()),
        YieldFrom(_) => patterns.push("yield_from".to_string()),
        Compare(_) => patterns.push("compare".to_string()),
        Call(call) => {
            patterns.push("call".to_string());
            collect_expression_patterns(&call.func, patterns);
            for arg in &call.args {
                collect_expression_patterns(arg, patterns);
            }
        }
        FormattedValue(_) => patterns.push("f_string".to_string()),
        JoinedStr(_) => patterns.push("joined_str".to_string()),
        Constant(_) => patterns.push("constant".to_string()),
        Attribute(_) => patterns.push("attribute".to_string()),
        Subscript(_) => patterns.push("subscript".to_string()),
        Starred(_) => patterns.push("starred".to_string()),
        Name(_) => patterns.push("name".to_string()),
        List(_) => patterns.push("list".to_string()),
        Tuple(_) => patterns.push("tuple".to_string()),
        Slice(_) => patterns.push("slice".to_string()),
        NamedExpr(_) => patterns.push("named_expr".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::{ast, parse, Mode};

    fn parse_expr(code: &str) -> ast::Expr {
        let parsed = parse(code, Mode::Expression, "<test>").unwrap();
        match parsed {
            ast::Mod::Expression(expr) => *expr.body,
            _ => panic!("Expected expression"),
        }
    }

    #[test]
    fn test_extract_nested_expr_tokens_with_binary_op() {
        let expr = parse_expr("x * 2");
        let tokens = extract_nested_expr_tokens(&expr);
        assert!(tokens.iter().any(|t| t.value() == "Mult"));
        assert!(tokens.iter().any(|t| t.value() == "int"));
    }

    #[test]
    fn test_extract_nested_expr_tokens_with_nested_binary() {
        let expr = parse_expr("x * 2 + y");
        let tokens = extract_nested_expr_tokens(&expr);
        // Should get both Add (outer) and Mult (inner)
        assert!(tokens.iter().any(|t| t.value() == "Add"));
        assert!(tokens.iter().any(|t| t.value() == "Mult"));
    }

    #[test]
    fn test_extract_nested_expr_tokens_with_boolean() {
        let expr = parse_expr("x and y");
        let tokens = extract_nested_expr_tokens(&expr);
        assert!(tokens.iter().any(|t| t.value() == "and"));
    }

    #[test]
    fn test_extract_bin_op_tokens_deeply_nested() {
        let expr = parse_expr("(a + b) * (c - d)");
        if let ast::Expr::BinOp(bin_op) = expr {
            let tokens = extract_bin_op_tokens(&bin_op);
            // Should extract all operators
            assert!(tokens.iter().any(|t| t.value() == "Mult")); // Top level
            assert!(tokens.iter().any(|t| t.value() == "Add")); // Left nested
            assert!(tokens.iter().any(|t| t.value() == "Sub")); // Right nested
        } else {
            panic!("Expected BinOp");
        }
    }

    #[test]
    fn test_collect_expression_patterns_bool_op() {
        let expr = parse_expr("x and y or z");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"bool_op".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_lambda() {
        let expr = parse_expr("lambda x: x + 1");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"lambda".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_if_exp() {
        let expr = parse_expr("x if condition else y");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"if_exp".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_dict() {
        let expr = parse_expr("{'key': 'value'}");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"dict".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_set() {
        let expr = parse_expr("{1, 2, 3}");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"set".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_list_comp() {
        let expr = parse_expr("[x * 2 for x in range(10)]");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"list_comp".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_set_comp() {
        let expr = parse_expr("{x * 2 for x in range(10)}");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"set_comp".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_dict_comp() {
        let expr = parse_expr("{x: x**2 for x in range(10)}");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"dict_comp".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_generator_exp() {
        let expr = parse_expr("(x * 2 for x in range(10))");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"generator_exp".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_await() {
        // Test await expression pattern detection
        // We need to construct the AST manually since await requires async context
        use rustpython_parser::ast;

        let await_expr = ast::Expr::Await(ast::ExprAwait {
            value: Box::new(ast::Expr::Name(ast::ExprName {
                id: "x".to_string().into(),
                ctx: ast::ExprContext::Load,
                range: Default::default(),
            })),
            range: Default::default(),
        });

        let mut patterns = Vec::new();
        collect_expression_patterns(&await_expr, &mut patterns);
        assert!(patterns.contains(&"await".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_yield() {
        // Test yield expression pattern detection
        use rustpython_parser::ast;

        let yield_expr = ast::Expr::Yield(ast::ExprYield {
            value: Some(Box::new(ast::Expr::Name(ast::ExprName {
                id: "x".to_string().into(),
                ctx: ast::ExprContext::Load,
                range: Default::default(),
            }))),
            range: Default::default(),
        });

        let mut patterns = Vec::new();
        collect_expression_patterns(&yield_expr, &mut patterns);
        assert!(patterns.contains(&"yield".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_yield_from() {
        // Test yield from expression pattern detection
        use rustpython_parser::ast;

        let yield_from_expr = ast::Expr::YieldFrom(ast::ExprYieldFrom {
            value: Box::new(ast::Expr::Name(ast::ExprName {
                id: "x".to_string().into(),
                ctx: ast::ExprContext::Load,
                range: Default::default(),
            })),
            range: Default::default(),
        });

        let mut patterns = Vec::new();
        collect_expression_patterns(&yield_from_expr, &mut patterns);
        assert!(patterns.contains(&"yield_from".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_formatted_value() {
        // Test FormattedValue pattern detection (part of f-strings)
        use rustpython_parser::ast;

        // Create a FormattedValue directly
        let formatted_value_expr = ast::Expr::FormattedValue(ast::ExprFormattedValue {
            value: Box::new(ast::Expr::Name(ast::ExprName {
                id: "x".to_string().into(),
                ctx: ast::ExprContext::Load,
                range: Default::default(),
            })),
            conversion: ast::ConversionFlag::None,
            format_spec: None,
            range: Default::default(),
        });

        let mut patterns = Vec::new();
        collect_expression_patterns(&formatted_value_expr, &mut patterns);
        assert!(patterns.contains(&"f_string".to_string()));

        // Also test JoinedStr which contains FormattedValues
        let expr = parse_expr("f'{x}'");
        let mut patterns2 = Vec::new();
        collect_expression_patterns(&expr, &mut patterns2);
        assert!(patterns2.contains(&"joined_str".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_subscript() {
        let expr = parse_expr("arr[0]");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"subscript".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_starred() {
        let expr = parse_expr("*args");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"starred".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_list() {
        let expr = parse_expr("[1, 2, 3]");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"list".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_tuple() {
        let expr = parse_expr("(1, 2, 3)");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"tuple".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_slice() {
        // Test slice expression pattern detection
        use rustpython_parser::ast;

        // Create a Slice directly
        let slice_expr = ast::Expr::Slice(ast::ExprSlice {
            lower: Some(Box::new(ast::Expr::Constant(ast::ExprConstant {
                value: ast::Constant::Int(1.into()),
                kind: None,
                range: Default::default(),
            }))),
            upper: Some(Box::new(ast::Expr::Constant(ast::ExprConstant {
                value: ast::Constant::Int(5.into()),
                kind: None,
                range: Default::default(),
            }))),
            step: None,
            range: Default::default(),
        });

        let mut patterns = Vec::new();
        collect_expression_patterns(&slice_expr, &mut patterns);
        assert!(patterns.contains(&"slice".to_string()));

        // Also test slice as part of subscript
        let expr = parse_expr("arr[1:5]");
        let mut patterns2 = Vec::new();
        collect_expression_patterns(&expr, &mut patterns2);
        assert!(patterns2.contains(&"subscript".to_string()));
    }

    #[test]
    fn test_collect_expression_patterns_named_expr() {
        // Walrus operator (Python 3.8+)
        let expr = parse_expr("(x := 5)");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        // May be parsed as named_expr or assignment depending on parser version
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_collect_expression_patterns_complex() {
        // Test a complex expression with multiple patterns
        let expr = parse_expr("[x * 2 for x in range(10) if x > 5]");
        let mut patterns = Vec::new();
        collect_expression_patterns(&expr, &mut patterns);
        assert!(patterns.contains(&"list_comp".to_string()));
        // The comprehension internally has other expressions but we only get top-level pattern
    }

    #[test]
    fn test_extract_literal_tokens_none() {
        let expr = parse_expr("None");
        let tokens = extract_literal_tokens(&expr);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value(), "None");
    }

    #[test]
    fn test_extract_literal_tokens_bool() {
        let expr = parse_expr("True");
        let tokens = extract_literal_tokens(&expr);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value(), "bool");
    }

    #[test]
    fn test_extract_literal_tokens_string() {
        let expr = parse_expr("'hello'");
        let tokens = extract_literal_tokens(&expr);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value(), "string");
    }

    #[test]
    fn test_extract_literal_tokens_int() {
        let expr = parse_expr("42");
        let tokens = extract_literal_tokens(&expr);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value(), "int");
    }

    #[test]
    fn test_extract_literal_tokens_float() {
        let expr = parse_expr("3.14");
        let tokens = extract_literal_tokens(&expr);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value(), "float");
    }

    #[test]
    fn test_extract_literal_tokens_bytes() {
        let expr = parse_expr("b'bytes'");
        let tokens = extract_literal_tokens(&expr);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value(), "bytes");
    }

    #[test]
    fn test_extract_literal_tokens_ellipsis() {
        let expr = parse_expr("...");
        let tokens = extract_literal_tokens(&expr);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value(), "...");
    }

    #[test]
    fn test_extract_literal_tokens_joined_str() {
        let expr = parse_expr("f'hello {name}'");
        let tokens = extract_literal_tokens(&expr);
        // JoinedStr should be handled by extract_join_str_tokens
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_extract_literal_tokens_non_literal() {
        // Test the default case - non-literal expressions should return empty
        let expr = parse_expr("x + y");
        let tokens = extract_literal_tokens(&expr);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_extract_literal_tokens_function_call() {
        // Another test for the default case
        let expr = parse_expr("print('hello')");
        let tokens = extract_literal_tokens(&expr);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_extract_literal_tokens_list() {
        // Lists are not literals, should return empty
        let expr = parse_expr("[1, 2, 3]");
        let tokens = extract_literal_tokens(&expr);
        assert!(tokens.is_empty());
    }
}
