use debtmap::core::Language;
use debtmap::utils::language_parser::{default_languages, parse_languages, parse_single_language};

#[test]
fn test_parse_single_language_rust() {
    assert_eq!(parse_single_language("rust"), Some(Language::Rust));
    assert_eq!(parse_single_language("rs"), Some(Language::Rust));
    assert_eq!(parse_single_language("RUST"), Some(Language::Rust));
    assert_eq!(parse_single_language("Rs"), Some(Language::Rust));
}

#[test]
fn test_parse_single_language_python() {
    assert_eq!(parse_single_language("python"), Some(Language::Python));
    assert_eq!(parse_single_language("py"), Some(Language::Python));
    assert_eq!(parse_single_language("PYTHON"), Some(Language::Python));
    assert_eq!(parse_single_language("Py"), Some(Language::Python));
}

#[test]
fn test_parse_single_language_javascript() {
    assert_eq!(
        parse_single_language("javascript"),
        Some(Language::JavaScript)
    );
    assert_eq!(parse_single_language("js"), Some(Language::JavaScript));
    assert_eq!(
        parse_single_language("JAVASCRIPT"),
        Some(Language::JavaScript)
    );
    assert_eq!(parse_single_language("JS"), Some(Language::JavaScript));
}

#[test]
fn test_parse_single_language_typescript() {
    assert_eq!(
        parse_single_language("typescript"),
        Some(Language::TypeScript)
    );
    assert_eq!(parse_single_language("ts"), Some(Language::TypeScript));
    assert_eq!(
        parse_single_language("TYPESCRIPT"),
        Some(Language::TypeScript)
    );
    assert_eq!(parse_single_language("TS"), Some(Language::TypeScript));
}

#[test]
fn test_parse_single_language_unknown() {
    assert_eq!(parse_single_language("java"), None);
    assert_eq!(parse_single_language("c++"), None);
    assert_eq!(parse_single_language("go"), None);
    assert_eq!(parse_single_language(""), None);
}

#[test]
fn test_parse_languages_with_valid_input() {
    let input = Some(vec!["rust".to_string(), "python".to_string()]);
    let result = parse_languages(input);
    assert_eq!(result, vec![Language::Rust, Language::Python]);
}

#[test]
fn test_parse_languages_with_mixed_valid_invalid() {
    let input = Some(vec![
        "rust".to_string(),
        "java".to_string(),
        "python".to_string(),
    ]);
    let result = parse_languages(input);
    assert_eq!(result, vec![Language::Rust, Language::Python]);
}

#[test]
fn test_parse_languages_with_none_uses_default() {
    let result = parse_languages(None);
    assert_eq!(result, default_languages());
}

#[test]
fn test_parse_languages_empty_vec_returns_empty() {
    let input = Some(vec![]);
    let result = parse_languages(input);
    assert_eq!(result, vec![]);
}

#[test]
fn test_default_languages() {
    let defaults = default_languages();
    assert_eq!(defaults.len(), 4);
    assert!(defaults.contains(&Language::Rust));
    assert!(defaults.contains(&Language::Python));
    assert!(defaults.contains(&Language::JavaScript));
    assert!(defaults.contains(&Language::TypeScript));
}
