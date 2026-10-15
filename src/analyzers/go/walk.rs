//! One traversal of a Go function, shared by independent per-node visitors.

use crate::analyzers::syntax_tree;
pub use crate::analyzers::syntax_tree::{Scope, Visitor};
use tree_sitter::Node;

/// Walks every node below `root` once, calling all visitors on each.
pub fn walk<'t>(root: Node<'t>, body: Option<Node<'t>>, visitors: &mut [&mut dyn Visitor<'t>]) {
    syntax_tree::walk(root, body, is_nested_callable, visitors);
}

pub fn is_nested_callable(node: Node) -> bool {
    matches!(
        node.kind(),
        "func_literal" | "function_declaration" | "method_declaration"
    )
}
