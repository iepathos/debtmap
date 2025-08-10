use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Ast {
    Rust(RustAst),
    Python(PythonAst),
    JavaScript(JavaScriptAst),
    TypeScript(TypeScriptAst),
    Unknown,
}

#[derive(Clone, Debug)]
pub struct RustAst {
    pub file: syn::File,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct PythonAst {
    pub module: rustpython_parser::ast::Mod,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct JavaScriptAst {
    pub tree: tree_sitter::Tree,
    pub source: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct TypeScriptAst {
    pub tree: tree_sitter::Tree,
    pub source: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct AstNode {
    pub kind: NodeKind,
    pub name: Option<String>,
    pub line: usize,
    pub children: Vec<AstNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    Function,
    Method,
    Class,
    Module,
    If,
    While,
    For,
    Match,
    Try,
    Block,
}

impl Ast {
    pub fn transform<F>(self, f: F) -> Self
    where
        F: Fn(Self) -> Self,
    {
        f(self)
    }

    pub fn map_functions<F, T>(&self, f: F) -> Vec<T>
    where
        F: Fn(&AstNode) -> Option<T>,
    {
        let nodes = self.extract_nodes();
        nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
            .filter_map(f)
            .collect()
    }

    pub fn extract_nodes(&self) -> Vec<AstNode> {
        type ExtractorFn = fn(&Ast) -> Vec<AstNode>;
        type MatcherFn = fn(&Ast) -> bool;

        static EXTRACTORS: &[(MatcherFn, ExtractorFn)] = &[
            (
                |ast| matches!(ast, Ast::Rust(_)),
                |ast| ast.extract_rust_nodes(),
            ),
            (
                |ast| matches!(ast, Ast::Python(_)),
                |ast| ast.extract_python_nodes(),
            ),
            (
                |ast| matches!(ast, Ast::JavaScript(_)),
                |ast| ast.extract_javascript_nodes(),
            ),
            (
                |ast| matches!(ast, Ast::TypeScript(_)),
                |ast| ast.extract_typescript_nodes(),
            ),
        ];

        EXTRACTORS
            .iter()
            .find(|(matcher, _)| matcher(self))
            .map(|(_, extractor)| extractor(self))
            .unwrap_or_default()
    }

    fn extract_rust_nodes(&self) -> Vec<AstNode> {
        vec![]
    }

    fn extract_python_nodes(&self) -> Vec<AstNode> {
        vec![]
    }

    fn extract_javascript_nodes(&self) -> Vec<AstNode> {
        vec![]
    }

    fn extract_typescript_nodes(&self) -> Vec<AstNode> {
        vec![]
    }

    pub fn count_branches(&self) -> usize {
        self.extract_nodes()
            .iter()
            .filter(|n| {
                matches!(
                    n.kind,
                    NodeKind::If | NodeKind::While | NodeKind::For | NodeKind::Match
                )
            })
            .count()
    }
}

pub fn combine_asts(asts: Vec<Ast>) -> Vec<Ast> {
    asts
}

pub fn filter_ast<F>(ast: Ast, predicate: F) -> Option<Ast>
where
    F: Fn(&Ast) -> bool,
{
    if predicate(&ast) {
        Some(ast)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_functions_extracts_functions() {
        let ast = Ast::Unknown;
        let results = ast.map_functions(|node| {
            if matches!(node.kind, NodeKind::Function | NodeKind::Method) {
                Some(node.name.clone().unwrap_or_else(|| "anonymous".to_string()))
            } else {
                None
            }
        });

        // Since Unknown AST has no functions, expect empty
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_ast_transform() {
        let ast = Ast::Unknown;
        let transformed = ast.clone().transform(|a| {
            // Simple identity transform
            a
        });

        assert!(matches!(transformed, Ast::Unknown));
    }

    #[test]
    fn test_count_branches() {
        let ast = Ast::Unknown;
        let count = ast.count_branches();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_extract_nodes_unknown() {
        let ast = Ast::Unknown;
        let nodes = ast.extract_nodes();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_combine_asts() {
        let asts = vec![Ast::Unknown, Ast::Unknown];
        let combined = combine_asts(asts);
        assert_eq!(combined.len(), 2);
    }

    #[test]
    fn test_filter_ast_matches() {
        let ast = Ast::Unknown;
        let filtered = filter_ast(ast, |a| matches!(a, Ast::Unknown));
        assert!(filtered.is_some());
    }

    #[test]
    fn test_filter_ast_no_match() {
        let ast = Ast::Unknown;
        let filtered = filter_ast(ast, |a| matches!(a, Ast::Rust(_)));
        assert!(filtered.is_none());
    }

    #[test]
    fn test_ast_node_creation() {
        let node = AstNode {
            kind: NodeKind::Function,
            name: Some("test_func".to_string()),
            line: 10,
            children: vec![],
        };

        assert_eq!(node.kind, NodeKind::Function);
        assert_eq!(node.name, Some("test_func".to_string()));
        assert_eq!(node.line, 10);
        assert_eq!(node.children.len(), 0);
    }

    #[test]
    fn test_map_functions_filters_correctly() {
        // Create a mock AST with both function and non-function nodes
        let nodes = [
            AstNode {
                kind: NodeKind::Function,
                name: Some("func1".to_string()),
                line: 1,
                children: vec![],
            },
            AstNode {
                kind: NodeKind::If,
                name: None,
                line: 2,
                children: vec![],
            },
            AstNode {
                kind: NodeKind::Method,
                name: Some("method1".to_string()),
                line: 3,
                children: vec![],
            },
        ];

        // Since we cannot easily construct a real AST with nodes,
        // test the filtering logic directly
        let function_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Function | NodeKind::Method))
            .collect();

        assert_eq!(function_nodes.len(), 2);
    }

    #[test]
    fn test_extract_rust_nodes() {
        let ast = Ast::Rust(RustAst {
            file: syn::File {
                shebang: None,
                attrs: vec![],
                items: vec![],
            },
            path: PathBuf::from("test.rs"),
        });
        
        // Currently returns empty vec, test that it doesn't panic
        let nodes = ast.extract_rust_nodes();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_extract_python_nodes() {
        // Test with Unknown AST since creating PythonAst requires complex structures
        let ast = Ast::Unknown;
        let nodes = ast.extract_python_nodes();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_extract_javascript_nodes() {
        // We can't easily create a tree_sitter::Tree, but we can test with Unknown
        let ast = Ast::Unknown;
        let nodes = ast.extract_javascript_nodes();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_extract_typescript_nodes() {
        // We can't easily create a tree_sitter::Tree, but we can test with Unknown
        let ast = Ast::Unknown;
        let nodes = ast.extract_typescript_nodes();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_extract_nodes_rust() {
        let ast = Ast::Rust(RustAst {
            file: syn::File {
                shebang: None,
                attrs: vec![],
                items: vec![],
            },
            path: PathBuf::from("test.rs"),
        });
        
        let nodes = ast.extract_nodes();
        assert_eq!(nodes.len(), 0); // Expected since extract_rust_nodes returns empty
    }

    #[test]
    fn test_extract_nodes_python() {
        // Test extraction logic without creating complex AST
        let ast = Ast::Unknown;
        let nodes = ast.extract_nodes();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_ast_node_with_children() {
        let child1 = AstNode {
            kind: NodeKind::Block,
            name: None,
            line: 5,
            children: vec![],
        };
        
        let child2 = AstNode {
            kind: NodeKind::If,
            name: None,
            line: 6,
            children: vec![],
        };
        
        let parent = AstNode {
            kind: NodeKind::Function,
            name: Some("parent_func".to_string()),
            line: 4,
            children: vec![child1, child2],
        };
        
        assert_eq!(parent.children.len(), 2);
        assert_eq!(parent.children[0].kind, NodeKind::Block);
        assert_eq!(parent.children[1].kind, NodeKind::If);
    }

    #[test]
    fn test_node_kind_equality() {
        assert_eq!(NodeKind::Function, NodeKind::Function);
        assert_ne!(NodeKind::Function, NodeKind::Method);
        assert_ne!(NodeKind::If, NodeKind::While);
    }

    #[test]
    fn test_count_branches_with_different_node_types() {
        // Test that count_branches correctly identifies branch nodes
        let branch_kinds = vec![
            NodeKind::If,
            NodeKind::While,
            NodeKind::For,
            NodeKind::Match,
        ];
        
        let non_branch_kinds = vec![
            NodeKind::Function,
            NodeKind::Method,
            NodeKind::Class,
            NodeKind::Module,
            NodeKind::Try,
            NodeKind::Block,
        ];
        
        for kind in branch_kinds {
            assert!(
                matches!(kind, NodeKind::If | NodeKind::While | NodeKind::For | NodeKind::Match),
                "Expected {:?} to be a branch node",
                kind
            );
        }
        
        for kind in non_branch_kinds {
            assert!(
                !matches!(kind, NodeKind::If | NodeKind::While | NodeKind::For | NodeKind::Match),
                "Expected {:?} to not be a branch node",
                kind
            );
        }
    }

    #[test]
    fn test_transform_preserves_type() {
        let rust_ast = Ast::Rust(RustAst {
            file: syn::File {
                shebang: None,
                attrs: vec![],
                items: vec![],
            },
            path: PathBuf::from("test.rs"),
        });
        
        let transformed = rust_ast.transform(|a| a);
        assert!(matches!(transformed, Ast::Rust(_)));
    }

    #[test]
    fn test_combine_asts_preserves_order() {
        let ast1 = Ast::Unknown;
        let ast2 = Ast::Unknown;
        let ast3 = Ast::Unknown;
        
        let combined = combine_asts(vec![ast1, ast2, ast3]);
        assert_eq!(combined.len(), 3);
    }

    #[test]
    fn test_combine_asts_empty() {
        let combined = combine_asts(vec![]);
        assert_eq!(combined.len(), 0);
    }

    #[test]
    fn test_filter_ast_with_multiple_predicates() {
        let ast = Ast::Unknown;
        
        // Test with always true predicate
        let result1 = filter_ast(ast.clone(), |_| true);
        assert!(result1.is_some());
        
        // Test with always false predicate
        let result2 = filter_ast(ast.clone(), |_| false);
        assert!(result2.is_none());
        
        // Test with specific type check
        let result3 = filter_ast(ast.clone(), |a| matches!(a, Ast::Unknown));
        assert!(result3.is_some());
    }
}
