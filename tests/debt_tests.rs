use debtmap::*;
use std::path::PathBuf;

#[test]
fn test_todo_fixme_detection() {
    // debtmap:ignore-start -- Test fixture data
    let content = r#"
    // TODO: Implement this feature
    // FIXME: This is broken
    // HACK: Temporary workaround
    // XXX: This needs review
    // BUG: Known issue here
    // OPTIMIZE: Can be improved
    // REFACTOR: Needs cleanup
    "#;
    // debtmap:ignore-end

    let path = PathBuf::from("test.rs");
    let items = find_todos_and_fixmes(content, &path);

    assert_eq!(items.len(), 7, "Should find all debt markers");

    // Check that different markers have appropriate priorities
    let has_high_priority = items.iter().any(|item| item.priority == Priority::High);
    let has_medium_priority = items.iter().any(|item| item.priority == Priority::Medium);

    assert!(
        has_high_priority,
        "Should have high priority items (FIXME, BUG)"
    );
    assert!(
        has_medium_priority,
        "Should have medium priority items (TODO)"
    );

    // Verify specific items
    let todo_item = items.iter().find(|i| i.message.contains("TODO")).unwrap();
    assert_eq!(todo_item.priority, Priority::Medium);
    assert!(matches!(todo_item.debt_type, DebtType::Todo { .. }));

    let fixme_item = items.iter().find(|i| i.message.contains("FIXME")).unwrap();
    assert_eq!(fixme_item.priority, Priority::High);
    assert!(matches!(fixme_item.debt_type, DebtType::Fixme { .. }));
}

#[test]
fn test_debt_item_creation() {
    let item = DebtItem {
        id: "test-debt-1".to_string(),
        debt_type: DebtType::Complexity {
            cyclomatic: 10,
            cognitive: 8,
        },
        priority: Priority::High,
        file: PathBuf::from("complex.rs"),
        line: 42,
        column: None,
        message: "Function has high complexity".to_string(),
        context: Some("fn complex_function() { ... }".to_string()),
    };

    assert!(matches!(item.debt_type, DebtType::Complexity { .. }));
    assert_eq!(item.priority, Priority::High);
    assert_eq!(item.line, 42);
    assert!(item.context.is_some());
}

#[test]
fn test_priority_ordering() {
    let low = Priority::Low;
    let medium = Priority::Medium;
    let high = Priority::High;
    let critical = Priority::Critical;

    assert!(low < medium);
    assert!(medium < high);
    assert!(high < critical);
    assert!(critical > low);
}

#[test]
fn test_language_detection() {
    assert_eq!(Language::from_extension("rs"), Language::Rust);
    // Python support removed in spec 191
    assert_eq!(Language::from_extension("py"), Language::Unknown);
    assert_eq!(Language::from_extension("unknown"), Language::Unknown);
}
