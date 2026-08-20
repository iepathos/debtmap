//! Conservative generated and vendor source classification.

use crate::core::Language;
use std::path::{Component, Path};

pub fn is_generated_or_vendor(path: &Path, language: Language, source: &str) -> bool {
    match language {
        Language::Go => crate::analyzers::go::generated::is_generated_go(path, source),
        Language::Solidity => {
            crate::analyzers::solidity::generated::is_vendor_or_generated_solidity(path, source)
        }
        Language::Rust => common_generated(path, source) || file_ends_with(path, ".generated.rs"),
        Language::Python => common_generated(path, source) || file_ends_with(path, "_pb2.py"),
        Language::JavaScript | Language::TypeScript => {
            common_generated(path, source)
                || file_ends_with(path, ".min.js")
                || file_ends_with(path, ".min.mjs")
        }
        Language::Unknown => false,
    }
}

fn common_generated(path: &Path, source: &str) -> bool {
    has_generated_header(source) || has_generated_component(path) || has_vendor_component(path)
}

fn has_generated_header(source: &str) -> bool {
    source.lines().take(10).any(|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("automatically generated")
            || (lower.contains("generated") && lower.contains("do not edit"))
    })
}

fn has_generated_component(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::Normal(name) if matches!(name.to_str(), Some("generated" | "autogen"))
        )
    })
}

fn has_vendor_component(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::Normal(name)
                if matches!(name.to_str(), Some("vendor" | "node_modules" | "dist"))
        )
    })
}

fn file_ends_with(path: &Path, suffix: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_language_generated_fixtures_without_broad_substrings() {
        let fixtures = [
            ("src/model.generated.rs", Language::Rust, ""),
            ("pkg/schema_pb2.py", Language::Python, ""),
            ("public/app.min.js", Language::JavaScript, ""),
            ("api/service.pb.go", Language::Go, "package api"),
            (
                "contracts/Generated.sol",
                Language::Solidity,
                "// Generated. DO NOT EDIT\npragma solidity 0.8.20;",
            ),
        ];
        for (path, language, source) in fixtures {
            assert!(is_generated_or_vendor(Path::new(path), language, source));
        }
        assert!(!is_generated_or_vendor(
            Path::new("src/generator.rs"),
            Language::Rust,
            "fn generate() {}"
        ));
    }
}
