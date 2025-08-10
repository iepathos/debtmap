use debtmap::core::{DebtType, Language, Priority};

#[test]
fn test_debt_type_display_todo() {
    assert_eq!(format!("{}", DebtType::Todo), "TODO");
}

#[test]
fn test_debt_type_display_fixme() {
    assert_eq!(format!("{}", DebtType::Fixme), "FIXME");
}

#[test]
fn test_debt_type_display_code_smell() {
    assert_eq!(format!("{}", DebtType::CodeSmell), "Code Smell");
}

#[test]
fn test_debt_type_display_duplication() {
    assert_eq!(format!("{}", DebtType::Duplication), "Duplication");
}

#[test]
fn test_debt_type_display_complexity() {
    assert_eq!(format!("{}", DebtType::Complexity), "Complexity");
}

#[test]
fn test_debt_type_display_dependency() {
    assert_eq!(format!("{}", DebtType::Dependency), "Dependency");
}

#[test]
fn test_debt_type_display_all_variants() {
    let types = vec![
        (DebtType::Todo, "TODO"),
        (DebtType::Fixme, "FIXME"),
        (DebtType::CodeSmell, "Code Smell"),
        (DebtType::Duplication, "Duplication"),
        (DebtType::Complexity, "Complexity"),
        (DebtType::Dependency, "Dependency"),
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
fn test_language_display_javascript() {
    assert_eq!(format!("{}", Language::JavaScript), "JavaScript");
}

#[test]
fn test_language_display_typescript() {
    assert_eq!(format!("{}", Language::TypeScript), "TypeScript");
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
        (Language::JavaScript, "JavaScript"),
        (Language::TypeScript, "TypeScript"),
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
    assert_eq!(Language::from_extension("py"), Language::Python);
}

#[test]
fn test_language_from_extension_javascript() {
    assert_eq!(Language::from_extension("js"), Language::JavaScript);
    assert_eq!(Language::from_extension("jsx"), Language::JavaScript);
    assert_eq!(Language::from_extension("mjs"), Language::JavaScript);
    assert_eq!(Language::from_extension("cjs"), Language::JavaScript);
}

#[test]
fn test_language_from_extension_typescript() {
    assert_eq!(Language::from_extension("ts"), Language::TypeScript);
    assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
    assert_eq!(Language::from_extension("mts"), Language::TypeScript);
    assert_eq!(Language::from_extension("cts"), Language::TypeScript);
}

#[test]
fn test_language_from_extension_unknown() {
    assert_eq!(Language::from_extension("txt"), Language::Unknown);
    assert_eq!(Language::from_extension("md"), Language::Unknown);
    assert_eq!(Language::from_extension("unknown"), Language::Unknown);
}
