use debtmap::core::{DebtType, Language, Priority};

#[test]
fn test_debt_type_display_todo() {
    assert_eq!(format!("{}", DebtType::Todo { reason: None }), "TODO");
}

#[test]
fn test_debt_type_display_fixme() {
    assert_eq!(format!("{}", DebtType::Fixme { reason: None }), "FIXME");
}

#[test]
fn test_debt_type_display_code_smell() {
    assert_eq!(
        format!("{}", DebtType::CodeSmell { smell_type: None }),
        "Code Smell"
    );
}

#[test]
fn test_debt_type_display_duplication() {
    assert_eq!(
        format!(
            "{}",
            DebtType::Duplication {
                instances: 2,
                total_lines: 20
            }
        ),
        "Duplication"
    );
}

#[test]
fn test_debt_type_display_complexity() {
    assert_eq!(
        format!(
            "{}",
            DebtType::Complexity {
                cyclomatic: 10,
                cognitive: 8
            }
        ),
        "Complexity"
    );
}

#[test]
fn test_debt_type_display_dependency() {
    assert_eq!(
        format!(
            "{}",
            DebtType::Dependency {
                dependency_type: None
            }
        ),
        "Dependency"
    );
}

#[test]
fn test_debt_type_display_all_variants() {
    let types = vec![
        (DebtType::Todo { reason: None }, "TODO"),
        (DebtType::Fixme { reason: None }, "FIXME"),
        (DebtType::CodeSmell { smell_type: None }, "Code Smell"),
        (
            DebtType::Duplication {
                instances: 2,
                total_lines: 20,
            },
            "Duplication",
        ),
        (
            DebtType::Complexity {
                cyclomatic: 10,
                cognitive: 8,
            },
            "Complexity",
        ),
        (
            DebtType::Dependency {
                dependency_type: None,
            },
            "Dependency",
        ),
    ];

    for (debt_type, expected) in types {
        assert_eq!(format!("{debt_type}"), expected);
    }
}

#[test]
fn test_priority_display_low() {
    assert_eq!(format!("{}", Priority::Low), "Low");
}

#[test]
fn test_priority_display_medium() {
    assert_eq!(format!("{}", Priority::Medium), "Medium");
}

#[test]
fn test_priority_display_high() {
    assert_eq!(format!("{}", Priority::High), "High");
}

#[test]
fn test_priority_display_critical() {
    assert_eq!(format!("{}", Priority::Critical), "Critical");
}

#[test]
fn test_priority_display_all_variants() {
    let priorities = vec![
        (Priority::Low, "Low"),
        (Priority::Medium, "Medium"),
        (Priority::High, "High"),
        (Priority::Critical, "Critical"),
    ];

    for (priority, expected) in priorities {
        assert_eq!(format!("{priority}"), expected);
    }
}

#[test]
fn test_language_display_rust() {
    assert_eq!(format!("{}", Language::Rust), "Rust");
}

#[test]
fn test_language_display_python() {
    assert_eq!(format!("{}", Language::Python), "Python");
}

#[test]
fn test_language_display_unknown() {
    assert_eq!(format!("{}", Language::Unknown), "Unknown");
}

#[test]
fn test_language_display_all_variants() {
    let languages = vec![
        (Language::Rust, "Rust"),
        (Language::Python, "Python"),
        (Language::Unknown, "Unknown"),
    ];

    for (language, expected) in languages {
        assert_eq!(format!("{language}"), expected);
    }
}

#[test]
fn test_language_from_extension_rust() {
    assert_eq!(Language::from_extension("rs"), Language::Rust);
}

#[test]
fn test_language_from_extension_python() {
    // Python support removed in spec 191
    assert_eq!(Language::from_extension("py"), Language::Unknown);
}

#[test]
fn test_language_from_extension_javascript() {
    // JavaScript support removed in spec 191
    assert_eq!(Language::from_extension("js"), Language::Unknown);
    assert_eq!(Language::from_extension("jsx"), Language::Unknown);
    assert_eq!(Language::from_extension("mjs"), Language::Unknown);
    assert_eq!(Language::from_extension("cjs"), Language::Unknown);
}

#[test]
fn test_language_from_extension_typescript() {
    // TypeScript support removed in spec 191
    assert_eq!(Language::from_extension("ts"), Language::Unknown);
    assert_eq!(Language::from_extension("tsx"), Language::Unknown);
    assert_eq!(Language::from_extension("mts"), Language::Unknown);
    assert_eq!(Language::from_extension("cts"), Language::Unknown);
}

#[test]
fn test_language_from_extension_unknown() {
    assert_eq!(Language::from_extension("txt"), Language::Unknown);
    assert_eq!(Language::from_extension("md"), Language::Unknown);
    assert_eq!(Language::from_extension("unknown"), Language::Unknown);
}
