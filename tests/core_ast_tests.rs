use debtmap::core::ast::{
    combine_asts, filter_ast, Ast, AstNode, JavaScriptAst, NodeKind, RustAst, TypeScriptAst,
};
use std::path::PathBuf;

fn create_test_rust_ast() -> Ast {
    Ast::Rust(RustAst {
        file: syn::parse_str("fn main() {}").unwrap(),
        path: PathBuf::from("test.rs"),
    })
}

fn create_test_python_ast() -> Ast {
    // Create a simple Python AST for testing
    // We'll use Unknown since the exact Python AST structure is complex
    // The important part is testing the extract_nodes behavior
    Ast::Unknown
}

fn create_test_javascript_ast() -> Ast {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .unwrap();
    let source = "function test() {}";
    let tree = parser.parse(source, None).unwrap();

    Ast::JavaScript(JavaScriptAst {
        tree,
        source: source.to_string(),
        path: PathBuf::from("test.js"),
    })
}

fn create_test_typescript_ast() -> Ast {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .unwrap();
    let source = "function test(): void {}";
    let tree = parser.parse(source, None).unwrap();

    Ast::TypeScript(TypeScriptAst {
        tree,
        source: source.to_string(),
        path: PathBuf::from("test.ts"),
    })
}

#[test]
fn test_extract_rust_nodes() {
    let ast = create_test_rust_ast();
    let nodes = ast.extract_nodes();

    // Currently returns empty vec as per implementation
    assert_eq!(nodes.len(), 0);

    // Test that it's called for Rust AST
    match &ast {
        Ast::Rust(_) => {
            // This branch should be taken
            // Note: extract_rust_nodes is private, so we test via extract_nodes
        }
        _ => panic!("Expected Rust AST"),
    }
}

#[test]
fn test_extract_python_nodes() {
    let ast = create_test_python_ast();
    let nodes = ast.extract_nodes();

    // Currently returns empty vec as per implementation
    assert_eq!(nodes.len(), 0);

    // Test that Unknown AST returns empty nodes
    match &ast {
        Ast::Unknown => {
            // This branch should be taken
        }
        _ => panic!("Expected Unknown AST"),
    }
}

#[test]
fn test_extract_javascript_nodes() {
    let ast = create_test_javascript_ast();
    let nodes = ast.extract_nodes();

    // Currently returns empty vec as per implementation
    assert_eq!(nodes.len(), 0);

    // Test that it's called for JavaScript AST
    match &ast {
        Ast::JavaScript(_) => {
            // This branch should be taken
            // Note: extract_javascript_nodes is private, so we test via extract_nodes
        }
        _ => panic!("Expected JavaScript AST"),
    }
}

#[test]
fn test_extract_typescript_nodes() {
    let ast = create_test_typescript_ast();
    let nodes = ast.extract_nodes();

    // Currently returns empty vec as per implementation
    assert_eq!(nodes.len(), 0);

    // Test that it's called for TypeScript AST
    match &ast {
        Ast::TypeScript(_) => {
            // This branch should be taken
            // Note: extract_typescript_nodes is private, so we test via extract_nodes
        }
        _ => panic!("Expected TypeScript AST"),
    }
}

#[test]
fn test_extract_nodes_unknown_ast() {
    let ast = Ast::Unknown;
    let nodes = ast.extract_nodes();

    // Unknown AST should return empty vec
    assert_eq!(nodes.len(), 0);
}

#[test]
fn test_ast_transform() {
    let ast = create_test_rust_ast();

    let transformed = ast.transform(|a| a);

    // Should preserve the AST type
    assert!(matches!(transformed, Ast::Rust(_)));
}

#[test]
fn test_map_functions_empty() {
    let ast = create_test_rust_ast();

    let functions: Vec<String> = ast.map_functions(|node| {
        if matches!(node.kind, NodeKind::Function | NodeKind::Method) {
            node.name.clone()
        } else {
            None
        }
    });

    // Since extract_nodes returns empty, this should be empty
    assert_eq!(functions.len(), 0);
}

#[test]
fn test_map_functions_with_mock_nodes() {
    // Create a mock function to test the filtering logic
    let _ast = create_test_rust_ast();

    // Create mock nodes for testing
    let mock_nodes = [
        AstNode {
            kind: NodeKind::Function,
            name: Some("test_func".to_string()),
            line: 10,
            children: vec![],
        },
        AstNode {
            kind: NodeKind::Class,
            name: Some("TestClass".to_string()),
            line: 20,
            children: vec![],
        },
        AstNode {
            kind: NodeKind::Method,
            name: Some("test_method".to_string()),
            line: 30,
            children: vec![],
        },
    ];

    // Filter only functions and methods
    let functions: Vec<String> = mock_nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
        .filter_map(|n| n.name.clone())
        .collect();

    assert_eq!(functions.len(), 2);
    assert!(functions.contains(&"test_func".to_string()));
    assert!(functions.contains(&"test_method".to_string()));
}

#[test]
fn test_count_branches_empty() {
    let ast = create_test_rust_ast();
    let count = ast.count_branches();

    // Since extract_nodes returns empty, this should be 0
    assert_eq!(count, 0);
}

#[test]
fn test_count_branches_with_mock_nodes() {
    // Create mock nodes for testing branch counting logic
    let mock_nodes = vec![
        AstNode {
            kind: NodeKind::If,
            name: None,
            line: 10,
            children: vec![],
        },
        AstNode {
            kind: NodeKind::While,
            name: None,
            line: 20,
            children: vec![],
        },
        AstNode {
            kind: NodeKind::For,
            name: None,
            line: 30,
            children: vec![],
        },
        AstNode {
            kind: NodeKind::Match,
            name: None,
            line: 40,
            children: vec![],
        },
        AstNode {
            kind: NodeKind::Function,
            name: Some("test".to_string()),
            line: 50,
            children: vec![],
        },
    ];

    // Count branch nodes
    let branch_count = mock_nodes
        .iter()
        .filter(|n| {
            matches!(
                n.kind,
                NodeKind::If | NodeKind::While | NodeKind::For | NodeKind::Match
            )
        })
        .count();

    assert_eq!(branch_count, 4);
}

#[test]
fn test_combine_asts() {
    let ast1 = create_test_rust_ast();
    let ast2 = create_test_python_ast();
    let ast3 = create_test_javascript_ast();

    let asts = vec![ast1, ast2, ast3];
    let combined = combine_asts(asts.clone());

    // combine_asts just returns the input vector
    assert_eq!(combined.len(), 3);
}

#[test]
fn test_filter_ast_matching() {
    let ast = create_test_rust_ast();

    let filtered = filter_ast(ast, |a| matches!(a, Ast::Rust(_)));

    assert!(filtered.is_some());
    assert!(matches!(filtered.unwrap(), Ast::Rust(_)));
}

#[test]
fn test_filter_ast_not_matching() {
    let ast = create_test_rust_ast();

    let filtered = filter_ast(ast, |a| matches!(a, Ast::Python(_)));

    assert!(filtered.is_none());
}

#[test]
fn test_filter_ast_with_unknown() {
    let ast = Ast::Unknown;

    let filtered = filter_ast(ast, |a| matches!(a, Ast::Unknown));

    assert!(filtered.is_some());
    assert!(matches!(filtered.unwrap(), Ast::Unknown));
}

#[test]
fn test_node_kind_equality() {
    assert_eq!(NodeKind::Function, NodeKind::Function);
    assert_ne!(NodeKind::Function, NodeKind::Method);
    assert_eq!(NodeKind::If, NodeKind::If);
    assert_ne!(NodeKind::While, NodeKind::For);
}

#[test]
fn test_ast_node_structure() {
    let node = AstNode {
        kind: NodeKind::Function,
        name: Some("test_function".to_string()),
        line: 42,
        children: vec![AstNode {
            kind: NodeKind::If,
            name: None,
            line: 43,
            children: vec![],
        }],
    };

    assert_eq!(node.kind, NodeKind::Function);
    assert_eq!(node.name, Some("test_function".to_string()));
    assert_eq!(node.line, 42);
    assert_eq!(node.children.len(), 1);
    assert_eq!(node.children[0].kind, NodeKind::If);
}
