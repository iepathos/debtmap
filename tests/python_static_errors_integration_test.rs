use debtmap::analyzers::python::PythonAnalyzer;
use debtmap::analyzers::Analyzer;
use debtmap::core::DebtType;
use std::path::PathBuf;

#[test]
fn test_undefined_variable_appears_in_debt_items() {
    let code = r#"
def on_message_added(self, message, index):
    if message is messages[index].message:
        return True
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Should detect undefined variable 'messages'
    let undefined_errors: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| {
            item.debt_type == DebtType::CodeSmell
                && item.message.contains("Undefined variable 'messages'")
        })
        .collect();

    assert_eq!(
        undefined_errors.len(),
        1,
        "Should detect one undefined variable 'messages'"
    );

    let error = undefined_errors[0];
    assert!(
        error.message.contains("on_message_added"),
        "Error should reference the function name"
    );
}

#[test]
fn test_missing_import_appears_in_debt_items() {
    let code = r#"
def test(param):
    wx.CallAfter(param)
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Should detect missing import for 'wx'
    let missing_imports: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| {
            item.debt_type == DebtType::CodeSmell && item.message.contains("Missing import: wx")
        })
        .collect();

    assert_eq!(
        missing_imports.len(),
        1,
        "Should detect missing import 'wx'"
    );

    let error = missing_imports[0];
    assert!(
        error.message.contains("wx"),
        "Error should reference module 'wx'"
    );
}

#[test]
fn test_no_false_positives_for_builtins() {
    let code = r#"
def process():
    return len([1, 2, 3]) + sum([1, 2, 3])
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Should NOT flag 'len' and 'sum' as undefined (they are builtins)
    let undefined_errors: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| {
            item.debt_type == DebtType::CodeSmell
                && (item.message.contains("Undefined variable 'len'")
                    || item.message.contains("Undefined variable 'sum'"))
        })
        .collect();

    assert_eq!(
        undefined_errors.len(),
        0,
        "Should not flag builtins as undefined"
    );
}

#[test]
fn test_no_false_positives_for_parameters() {
    let code = r#"
def add(a, b):
    return a + b
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Should NOT flag parameters 'a' and 'b' as undefined
    let undefined_errors: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| {
            item.debt_type == DebtType::CodeSmell
                && (item.message.contains("Undefined variable 'a'")
                    || item.message.contains("Undefined variable 'b'"))
        })
        .collect();

    assert_eq!(
        undefined_errors.len(),
        0,
        "Should not flag parameters as undefined"
    );
}

#[test]
fn test_no_false_positives_for_local_variables() {
    let code = r#"
def process():
    x = 10
    return x + 5
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Should NOT flag 'x' as undefined (it's assigned before use)
    let undefined_errors: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| {
            item.debt_type == DebtType::CodeSmell && item.message.contains("Undefined variable 'x'")
        })
        .collect();

    assert_eq!(
        undefined_errors.len(),
        0,
        "Should not flag local variables as undefined"
    );
}

#[test]
fn test_multiple_errors_in_same_file() {
    let code = r#"
def func1():
    return undefined_var1

def func2():
    os.path.join("a", "b")
    return undefined_var2
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Should detect both undefined variables and missing import
    let static_errors: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| {
            item.debt_type == DebtType::CodeSmell
                && (item.message.contains("Undefined variable")
                    || item.message.contains("Missing import"))
        })
        .collect();

    assert!(
        static_errors.len() >= 3,
        "Should detect at least 3 errors (2 undefined vars + 1 missing import)"
    );
}
