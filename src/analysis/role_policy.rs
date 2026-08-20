//! Shared evidence and policy for orthogonal code roles.

use crate::core::{FunctionMetrics, Language};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleSignal {
    TestSyntax,
    TestPath,
    EntrySyntax,
    EntryConvention,
    Framework { name: String, kind: String },
    PublicExport,
    Configured { role: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleEvidence {
    pub signals: Vec<RoleSignal>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeRoles {
    pub is_test: bool,
    pub is_entry_point: bool,
    pub is_framework_managed: bool,
    pub is_public_api: bool,
}

impl CodeRoles {
    pub fn is_production(self) -> bool {
        !self.is_test
    }
}

pub fn evidence_for_metric(metric: &FunctionMetrics) -> RoleEvidence {
    let language = Language::from_path(&metric.file);
    let mut signals = Vec::new();
    if metric.is_test || metric.in_test_module {
        signals.push(RoleSignal::TestSyntax);
    }
    if is_test_path(&metric.file, language) {
        signals.push(RoleSignal::TestPath);
    }
    if is_entry_name(&metric.name, language) {
        signals.push(RoleSignal::EntryConvention);
    }
    if is_public_visibility(metric.visibility.as_deref()) {
        signals.push(RoleSignal::PublicExport);
    }
    RoleEvidence { signals }
}

pub fn classify_roles(evidence: &RoleEvidence) -> CodeRoles {
    CodeRoles {
        is_test: evidence
            .signals
            .iter()
            .any(|signal| matches!(signal, RoleSignal::TestSyntax | RoleSignal::TestPath)),
        is_entry_point: evidence.signals.iter().any(|signal| {
            matches!(
                signal,
                RoleSignal::EntrySyntax | RoleSignal::EntryConvention
            )
        }),
        is_framework_managed: evidence
            .signals
            .iter()
            .any(|signal| matches!(signal, RoleSignal::Framework { .. })),
        is_public_api: evidence
            .signals
            .iter()
            .any(|signal| matches!(signal, RoleSignal::PublicExport)),
    }
}

pub fn roles_for_metric(metric: &FunctionMetrics) -> CodeRoles {
    classify_roles(&evidence_for_metric(metric))
}

pub fn estimate_test_lines(
    path: &Path,
    language: Language,
    functions: &[FunctionMetrics],
    total_lines: usize,
) -> usize {
    if is_test_path(path, language) {
        return total_lines;
    }
    functions
        .iter()
        .filter(|metric| roles_for_metric(metric).is_test)
        .map(|metric| metric.length)
        .sum::<usize>()
        .min(total_lines)
}

pub fn is_entry_name(name: &str, language: Language) -> bool {
    let unqualified = name.rsplit("::").next().unwrap_or(name);
    match language {
        Language::Python => matches!(unqualified, "__main__" | "main"),
        Language::Solidity => matches!(unqualified, "constructor" | "fallback" | "receive"),
        Language::Rust
        | Language::JavaScript
        | Language::TypeScript
        | Language::Go
        | Language::Unknown => {
            unqualified == "main"
                || ["handle_", "run_"]
                    .iter()
                    .any(|prefix| unqualified.starts_with(prefix))
        }
    }
}

pub fn is_test_path(path: &Path, language: Language) -> bool {
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let test_component = path.components().any(|component| match component {
        Component::Normal(value) => matches!(value.to_str(), Some("test" | "tests" | "__tests__")),
        _ => false,
    });
    test_component || test_file_name(file, language)
}

fn test_file_name(file: &str, language: Language) -> bool {
    match language {
        Language::Rust => file.ends_with("_test.rs"),
        Language::Python => {
            (file.starts_with("test_") || file.ends_with("_test.py"))
                && (file.ends_with(".py") || file.ends_with(".pyw"))
        }
        Language::JavaScript | Language::TypeScript => [".test.", ".spec."]
            .iter()
            .any(|marker| file.contains(marker)),
        Language::Go => file.ends_with("_test.go"),
        Language::Solidity => file.ends_with(".t.sol") || file.ends_with(".test.sol"),
        Language::Unknown => false,
    }
}

fn is_public_visibility(visibility: Option<&str>) -> bool {
    matches!(
        visibility,
        Some("pub" | "public" | "export" | "external" | "pub(crate)" | "pub(super)")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_signals_are_orthogonal() {
        let evidence = RoleEvidence {
            signals: vec![
                RoleSignal::EntrySyntax,
                RoleSignal::Framework {
                    name: "axum".into(),
                    kind: "handler".into(),
                },
                RoleSignal::PublicExport,
            ],
        };

        assert_eq!(
            classify_roles(&evidence),
            CodeRoles {
                is_test: false,
                is_entry_point: true,
                is_framework_managed: true,
                is_public_api: true,
            }
        );
    }

    #[test]
    fn path_policy_avoids_test_substring_false_positives() {
        for path in ["src/contest.py", "latest/worker.ts", "src/testimonial.rs"] {
            assert!(!is_test_path(
                Path::new(path),
                Language::from_path(Path::new(path))
            ));
        }
        for path in [
            "tests/unit.rs",
            "src/test_worker.py",
            "src/worker.spec.ts",
            "pkg/worker_test.go",
            "test/Worker.t.sol",
        ] {
            assert!(is_test_path(
                Path::new(path),
                Language::from_path(Path::new(path))
            ));
        }
    }
}
