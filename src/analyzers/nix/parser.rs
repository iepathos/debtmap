//! Tree-sitter parser integration for Nix.

use crate::core::ast::NixAst;
use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::{Language as TsLanguage, Parser};

fn get_language() -> TsLanguage {
    tree_sitter_nix::LANGUAGE.into()
}

pub fn parse_source(content: &str, path: &Path) -> Result<NixAst> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .context("Failed to set tree-sitter Nix language")?;

    let tree = parser
        .parse(content, None)
        .context("Failed to parse Nix source")?;

    if has_parse_errors(&tree) {
        anyhow::bail!("Nix parse tree contains syntax errors");
    }

    Ok(NixAst {
        tree,
        path: path.to_path_buf(),
        source: content.to_string(),
    })
}

pub use crate::analyzers::syntax_tree::has_parse_errors;

pub use crate::analyzers::syntax_tree::{node_column, node_line, node_text};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_flake() {
        let source = r#"{
  description = "example";
  outputs = { self, nixpkgs }: {
    packages.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.hello;
  };
}
"#;
        let ast = parse_source(source, &PathBuf::from("flake.nix")).unwrap();

        assert!(!has_parse_errors(&ast.tree));
        assert_eq!(ast.tree.root_node().kind(), "source_code");
    }

    #[test]
    fn test_malformed_nix_fails() {
        let result = parse_source("{ a = ; ", &PathBuf::from("broken.nix"));
        assert!(result.is_err());
    }

    #[test]
    fn test_node_helpers() {
        let source = "let a = 1; in a\n";
        let ast = parse_source(source, &PathBuf::from("a.nix")).unwrap();
        let root = ast.tree.root_node();

        assert_eq!(node_text(&root, &ast.source), source);
        assert_eq!(node_line(&root), 1);
    }
}
