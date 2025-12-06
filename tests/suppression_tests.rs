use debtmap::*;
use std::path::PathBuf;

#[test]
fn test_suppression_block_comments() {
    let content = r#"
// debtmap:ignore-start
// TODO: This should be suppressed
// FIXME: This too
// debtmap:ignore-end
// TODO: This should not be suppressed
"#;

    let path = PathBuf::from("test.rs");
    let items = find_todos_and_fixmes_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(content, Language::Rust, &path)),
    );

    assert_eq!(items.len(), 1, "Should only find one non-suppressed TODO");
    assert!(items[0].message.contains("This should not be suppressed"));
}

#[test]
fn test_suppression_line_comments() {
    // debtmap:ignore-start -- Test fixture data
    let content = r#"
// TODO: Not suppressed
// TODO: Suppressed // debtmap:ignore
// FIXME: Also not suppressed
"#;
    // debtmap:ignore-end

    let path = PathBuf::from("test.rs");
    let items = find_todos_and_fixmes_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(content, Language::Rust, &path)),
    );

    assert_eq!(items.len(), 2, "Should find two non-suppressed items");
    assert!(!items.iter().any(|i| i.message.contains("Suppressed")));
}

#[test]
fn test_suppression_next_line() {
    let content = r#"
// debtmap:ignore-next-line
// TODO: This should be suppressed
// TODO: This should not be suppressed
"#;

    let path = PathBuf::from("test.rs");
    let items = find_todos_and_fixmes_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(content, Language::Rust, &path)),
    );

    assert_eq!(items.len(), 1, "Should only find one non-suppressed TODO");
    assert!(items[0].message.contains("This should not be suppressed"));
}

#[test]
fn test_type_specific_suppression() {
    // debtmap:ignore-start -- Test fixture data
    let content = r#"
// debtmap:ignore-start[todo]
// TODO: Suppressed
// FIXME: Not suppressed  
// debtmap:ignore-end
"#;
    // debtmap:ignore-end

    let path = PathBuf::from("test.rs");
    let items = find_todos_and_fixmes_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(content, Language::Rust, &path)),
    );

    assert_eq!(items.len(), 1, "Should only find FIXME");
    assert!(matches!(items[0].debt_type, DebtType::Fixme { .. }));
}

#[test]
fn test_suppression_with_reason() {
    let content = r#"
// debtmap:ignore-start -- Test fixture data
// TODO: Test TODO
// FIXME: Test FIXME
// debtmap:ignore-end
"#;

    let path = PathBuf::from("test.rs");
    let suppression = parse_suppression_comments(content, Language::Rust, &path);
    let items = find_todos_and_fixmes_with_suppression(content, &path, Some(&suppression));

    assert_eq!(items.len(), 0, "All items should be suppressed");
    assert_eq!(
        suppression.active_blocks[0].reason,
        Some("Test fixture data".to_string())
    );
}

#[test]
fn test_python_suppression() {
    // debtmap:ignore-start -- Test fixture data
    let content = r#"
# debtmap:ignore-start
# TODO: Python TODO
# FIXME: Python FIXME
# debtmap:ignore-end
# TODO: Not suppressed
"#;
    // debtmap:ignore-end

    let path = PathBuf::from("test.py");
    let items = find_todos_and_fixmes_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(
            content,
            Language::Python,
            &path,
        )),
    );

    assert_eq!(items.len(), 1, "Should only find one non-suppressed TODO");
    assert!(items[0].message.contains("Not suppressed"));
}

#[test]
fn test_wildcard_suppression() {
    // debtmap:ignore-start -- Test fixture data
    let content = r#"
// debtmap:ignore[*]
// TODO: Suppressed
// FIXME: Also suppressed
"#;
    // debtmap:ignore-end

    let path = PathBuf::from("test.rs");
    let suppression = parse_suppression_comments(content, Language::Rust, &path);

    assert!(suppression.is_suppressed(2, &DebtType::Todo { reason: None }));
    assert!(suppression.is_suppressed(2, &DebtType::Fixme { reason: None }));
    assert!(suppression.is_suppressed(2, &DebtType::CodeSmell { smell_type: None }));
}
