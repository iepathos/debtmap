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

pub struct RoleFacts<'a> {
    pub path: &'a Path,
    pub language: Language,
    pub name: &'a str,
    pub is_test: bool,
    pub in_test_module: bool,
    pub visibility: Option<&'a str>,
}

impl CodeRoles {
    pub fn is_production(self) -> bool {
        !self.is_test
    }
}

pub fn evidence_for_metric(metric: &FunctionMetrics) -> RoleEvidence {
    let language = Language::from_path(&metric.file);
    evidence_for_facts(RoleFacts {
        path: &metric.file,
        language,
        name: &metric.name,
        is_test: metric.is_test,
        in_test_module: metric.in_test_module,
        visibility: metric.visibility.as_deref(),
    })
}

pub fn evidence_for_facts(facts: RoleFacts<'_>) -> RoleEvidence {
    let mut signals = Vec::new();
    if facts.is_test || facts.in_test_module {
        signals.push(RoleSignal::TestSyntax);
    }
    if is_test_path(facts.path, facts.language) {
        signals.push(RoleSignal::TestPath);
    }
    if is_entry_name(facts.name, facts.language) {
        signals.push(RoleSignal::EntryConvention);
    }
    if is_public_visibility(facts.visibility) {
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

pub fn evidence_from_roles(roles: CodeRoles) -> RoleEvidence {
    let mut signals = Vec::new();
    if roles.is_test {
        signals.push(RoleSignal::TestSyntax);
    }
    if roles.is_entry_point {
        signals.push(RoleSignal::EntrySyntax);
    }
    if roles.is_framework_managed {
        signals.push(RoleSignal::Framework {
            name: "configured".into(),
            kind: "managed".into(),
        });
    }
    if roles.is_public_api {
        signals.push(RoleSignal::PublicExport);
    }
    RoleEvidence { signals }
}

pub fn merge_evidence(left: &RoleEvidence, right: &RoleEvidence) -> RoleEvidence {
    let mut signals = left
        .signals
        .iter()
        .chain(&right.signals)
        .cloned()
        .collect::<Vec<_>>();
    signals.sort_by_key(role_signal_key);
    signals.dedup();
    RoleEvidence { signals }
}

fn role_signal_key(signal: &RoleSignal) -> String {
    match signal {
        RoleSignal::TestSyntax => "test:syntax".into(),
        RoleSignal::TestPath => "test:path".into(),
        RoleSignal::EntrySyntax => "entry:syntax".into(),
        RoleSignal::EntryConvention => "entry:convention".into(),
        RoleSignal::Framework { name, kind } => format!("framework:{name}:{kind}"),
        RoleSignal::PublicExport => "public:export".into(),
        RoleSignal::Configured { role } => format!("configured:{role}"),
    }
}

pub fn roles_for_metric(metric: &FunctionMetrics) -> CodeRoles {
    classify_roles(&evidence_for_metric(metric))
}

pub fn framework_evidence_for_source(
    language: Language,
    source: &str,
    function_name: &str,
) -> RoleEvidence {
    let framework = match language {
        Language::JavaScript | Language::TypeScript => {
            js_framework_registration(source, function_name)
        }
        Language::Go => go_framework_registration(source, function_name),
        Language::Solidity => solidity_framework_registration(source, function_name),
        _ => None,
    };
    framework
        .map(|(name, kind)| RoleEvidence {
            signals: vec![RoleSignal::Framework { name, kind }],
        })
        .unwrap_or_default()
}

fn js_framework_registration(source: &str, name: &str) -> Option<(String, String)> {
    source.lines().find_map(|line| {
        let is_route = [".get(", ".post(", ".put(", ".delete(", ".use("]
            .iter()
            .any(|pattern| line.contains(pattern));
        (is_route && line.contains(name)).then(|| ("js-router".into(), "registration".into()))
    })
}

fn go_framework_registration(source: &str, name: &str) -> Option<(String, String)> {
    source.lines().find_map(|line| {
        let is_handler = line.contains("HandleFunc(") || line.contains(".Handle(");
        (is_handler && line.contains(name)).then(|| ("net/http".into(), "handler".into()))
    })
}

fn solidity_framework_registration(source: &str, name: &str) -> Option<(String, String)> {
    let method = name.rsplit('.').next().unwrap_or(name);
    let is_hook =
        method == "setUp" || method.starts_with("test") || method.starts_with("invariant");
    (is_hook && (source.contains("forge-std/Test.sol") || source.contains("hardhat/console.sol")))
        .then(|| ("solidity-test-framework".into(), "test-hook".into()))
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
    fn extracted_facts_preserve_overlapping_roles() {
        let evidence = evidence_for_facts(RoleFacts {
            path: Path::new("tests/runner.py"),
            language: Language::Python,
            name: "main",
            is_test: false,
            in_test_module: false,
            visibility: Some("public"),
        });

        assert_eq!(
            classify_roles(&evidence),
            CodeRoles {
                is_test: true,
                is_entry_point: true,
                is_framework_managed: false,
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

    #[test]
    fn merging_evidence_is_deterministic_and_lossless() {
        let left = RoleEvidence {
            signals: vec![RoleSignal::PublicExport, RoleSignal::TestPath],
        };
        let right = RoleEvidence {
            signals: vec![RoleSignal::TestPath, RoleSignal::EntrySyntax],
        };

        let merged = merge_evidence(&left, &right);

        assert_eq!(merged.signals.len(), 3);
        assert_eq!(
            classify_roles(&merged),
            CodeRoles {
                is_test: true,
                is_entry_point: true,
                is_framework_managed: false,
                is_public_api: true,
            }
        );
    }

    #[test]
    fn language_framework_registration_is_explicit() {
        let fixtures = [
            (
                Language::TypeScript,
                "router.get('/users', listUsers);",
                "listUsers",
            ),
            (
                Language::Go,
                "http.HandleFunc(\"/health\", health);",
                "health",
            ),
            (
                Language::Solidity,
                "import 'forge-std/Test.sol'; function setUp() public {}",
                "Suite.setUp",
            ),
        ];
        for (language, source, name) in fixtures {
            assert!(
                classify_roles(&framework_evidence_for_source(language, source, name))
                    .is_framework_managed
            );
        }
        assert!(
            !classify_roles(&framework_evidence_for_source(
                Language::Go,
                "func health() {}",
                "health"
            ))
            .is_framework_managed
        );
    }
}
