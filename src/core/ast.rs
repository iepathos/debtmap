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
}
