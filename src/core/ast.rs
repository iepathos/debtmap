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
        match self {
            Ast::Rust(_) => self.extract_rust_nodes(),
            Ast::Python(_) => self.extract_python_nodes(),
            Ast::JavaScript(_) => self.extract_javascript_nodes(),
            Ast::TypeScript(_) => self.extract_typescript_nodes(),
            Ast::Unknown => vec![],
        }
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
