use debtmap::analyzers::python::PythonAnalyzer;
use debtmap::analyzers::Analyzer;
use debtmap::core::DebtType;
use std::path::PathBuf;

#[test]
fn test_python_mutable_default_detection() {
    let code = r#"
def process_items(items=[]):  # Mutable default
    items.append(1)
    return items

def safe_process(items=None):  # Good pattern
    if items is None:
        items = []
    return items
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Should detect mutable default argument
    let mutable_defaults: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| {
            item.debt_type == DebtType::CodeOrganization && item.message.contains("Mutable default")
        })
        .collect();

    assert_eq!(
        mutable_defaults.len(),
        1,
        "Should detect one mutable default argument"
    );
}

#[test]
fn test_python_god_class_detection() {
    let mut methods = String::new();
    for i in 0..25 {
        methods.push_str(&format!("    def method_{}(self): pass\n", i));
    }

    let code = format!(
        r#"
class GodClass:
{}
"#,
        methods
    );

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(&code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Should detect God Object pattern
    let god_classes: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| {
            item.debt_type == DebtType::CodeOrganization && item.message.contains("God Object")
        })
        .collect();

    assert_eq!(god_classes.len(), 1, "Should detect one God Object");
}

#[test]
fn test_python_test_without_assertions() {
    let code = r#"
def test_something():
    # Setup
    x = 1
    y = 2
    # Missing assertion!
    
def test_with_assertion():
    x = 1
    assert x == 1
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Should detect test without assertions
    let no_assert_tests: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| {
            item.debt_type == DebtType::TestQuality && item.message.contains("no assertions")
        })
        .collect();

    assert_eq!(
        no_assert_tests.len(),
        1,
        "Should detect one test without assertions"
    );
}

#[test]
#[ignore = "Nested loop detection needs more work"]
fn test_python_nested_loop_detection() {
    let code = r#"
def nested_loops(data):
    for i in data:
        for j in i:  # Nested loop
            print(j)
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Should detect performance issue (nested loop)
    let perf_issues: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| item.debt_type == DebtType::Performance)
        .collect();

    assert!(
        perf_issues.len() >= 1,
        "Should detect at least one performance issue in nested loops"
    );
}
