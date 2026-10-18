//! Helpers shared by the tree-sitter based analyzers.

use tree_sitter::{Node, Tree, TreeCursor};

/// Whether the parse tree contains an error node.
pub fn has_parse_errors(tree: &Tree) -> bool {
    tree.root_node().has_error()
}

/// Source text covered by `node`.
pub fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

/// 1-indexed line where `node` starts.
pub fn node_line(node: &Node) -> usize {
    node.start_position().row + 1
}

/// 1-indexed column where `node` starts.
pub fn node_column(node: &Node) -> usize {
    node.start_position().column + 1
}

/// Number of lines `node` spans.
pub fn function_length(node: Node) -> usize {
    node.end_position()
        .row
        .saturating_sub(node.start_position().row)
        + 1
}

pub fn children(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

/// Calls `visit` on each direct child of `node`.
pub fn walk_children<'t>(node: Node<'t>, mut visit: impl FnMut(Node<'t>)) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child);
    }
}

/// Calls `visit` on `node` and every descendant in pre-order.
pub fn walk_nodes<'t>(node: Node<'t>, visit: &mut impl FnMut(Node<'t>)) {
    visit(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_nodes(child, visit);
    }
}

/// Where a node sits within the walked function.
#[derive(Clone, Copy)]
pub struct Scope {
    pub in_body: bool,
    pub nested: bool,
}

impl Scope {
    /// Inside the function body and outside any nested callable.
    pub fn own_body(self) -> bool {
        self.in_body && !self.nested
    }
}

/// Observes every node below the root in pre-order.
pub trait Visitor<'t> {
    fn enter(&mut self, node: Node<'t>, scope: Scope);
    fn exit(&mut self, _node: Node<'t>, _scope: Scope) {}
}

/// How the walk recognizes the function body and nested callables.
#[derive(Clone, Copy)]
struct Bounds {
    body: Option<usize>,
    is_nested: fn(Node) -> bool,
}

/// Walks every node below `root` once, calling all visitors on each.
/// `is_nested` marks callables whose subtrees belong to another function.
pub fn walk<'t>(
    root: Node<'t>,
    body: Option<Node<'t>>,
    is_nested: fn(Node) -> bool,
    visitors: &mut [&mut dyn Visitor<'t>],
) {
    let scope = Scope {
        in_body: body == Some(root),
        nested: false,
    };
    let bounds = Bounds {
        body: body.map(|body| body.id()),
        is_nested,
    };
    visit_children(&mut root.walk(), bounds, scope, visitors);
}

fn visit_children<'t>(
    cursor: &mut TreeCursor<'t>,
    bounds: Bounds,
    scope: Scope,
    visitors: &mut [&mut dyn Visitor<'t>],
) {
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        visit(cursor, bounds, scope, visitors);
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    cursor.goto_parent();
}

fn visit<'t>(
    cursor: &mut TreeCursor<'t>,
    bounds: Bounds,
    parent: Scope,
    visitors: &mut [&mut dyn Visitor<'t>],
) {
    let node = cursor.node();
    let scope = Scope {
        in_body: parent.in_body || Some(node.id()) == bounds.body,
        nested: parent.nested || (bounds.is_nested)(node),
    };
    for visitor in visitors.iter_mut() {
        visitor.enter(node, scope);
    }
    visit_children(cursor, bounds, scope, visitors);
    for visitor in visitors.iter_mut() {
        visitor.exit(node, scope);
    }
}
