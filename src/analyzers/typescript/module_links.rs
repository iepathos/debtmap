//! Named import and re-export evidence from JavaScript and TypeScript modules.

use super::parser::node_text;
use crate::core::ast::TypeScriptAst;
use tree_sitter::Node;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportBinding {
    pub local: String,
    pub imported: ImportName,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ImportName {
    Named(String),
    Namespace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReExport {
    pub exported: String,
    pub imported: String,
    pub source: String,
}

pub(crate) fn extract_imports(ast: &TypeScriptAst) -> Vec<ImportBinding> {
    descendants(ast.tree.root_node(), "import_statement")
        .into_iter()
        .flat_map(|statement| import_bindings(statement, ast))
        .collect()
}

pub(crate) fn extract_reexports(ast: &TypeScriptAst) -> Vec<ReExport> {
    descendants(ast.tree.root_node(), "export_statement")
        .into_iter()
        .filter_map(|statement| {
            statement
                .child_by_field_name("source")
                .map(|source| (statement, string_value(source, ast)))
        })
        .flat_map(|(statement, source)| {
            descendants(statement, "export_specifier")
                .into_iter()
                .filter_map(move |specifier| reexport(specifier, ast, &source))
        })
        .collect()
}

fn import_bindings(statement: Node<'_>, ast: &TypeScriptAst) -> Vec<ImportBinding> {
    let Some(source) = statement.child_by_field_name("source") else {
        return Vec::new();
    };
    let source = string_value(source, ast);
    let mut bindings = descendants(statement, "import_specifier")
        .into_iter()
        .filter_map(|specifier| named_import(specifier, ast, &source))
        .collect::<Vec<_>>();
    bindings.extend(
        descendants(statement, "namespace_import")
            .into_iter()
            .filter_map(|namespace| namespace_import(namespace, ast, &source)),
    );
    bindings
}

fn named_import(node: Node<'_>, ast: &TypeScriptAst, source: &str) -> Option<ImportBinding> {
    let imported = node.child_by_field_name("name")?;
    let local = node.child_by_field_name("alias").unwrap_or(imported);
    Some(ImportBinding {
        local: node_text(&local, &ast.source).to_string(),
        imported: ImportName::Named(node_text(&imported, &ast.source).to_string()),
        source: source.to_string(),
    })
}

fn namespace_import(node: Node<'_>, ast: &TypeScriptAst, source: &str) -> Option<ImportBinding> {
    let local = node
        .named_children(&mut node.walk())
        .find(|child| child.kind() == "identifier")?;
    Some(ImportBinding {
        local: node_text(&local, &ast.source).to_string(),
        imported: ImportName::Namespace,
        source: source.to_string(),
    })
}

fn reexport(node: Node<'_>, ast: &TypeScriptAst, source: &str) -> Option<ReExport> {
    let imported = node.child_by_field_name("name")?;
    let exported = node.child_by_field_name("alias").unwrap_or(imported);
    Some(ReExport {
        exported: node_text(&exported, &ast.source).to_string(),
        imported: node_text(&imported, &ast.source).to_string(),
        source: source.to_string(),
    })
}

fn descendants<'a>(node: Node<'a>, kind: &str) -> Vec<Node<'a>> {
    let mut found = Vec::new();
    collect_descendants(node, kind, &mut found);
    found
}

fn collect_descendants<'a>(node: Node<'a>, kind: &str, found: &mut Vec<Node<'a>>) {
    if node.kind() == kind {
        found.push(node);
    }
    for child in node.children(&mut node.walk()) {
        collect_descendants(child, kind, found);
    }
}

fn string_value(node: Node<'_>, ast: &TypeScriptAst) -> String {
    node_text(&node, &ast.source)
        .trim_matches(['"', '\'', '`'])
        .to_string()
}
