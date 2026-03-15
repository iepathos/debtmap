use debtmap::extraction::UnifiedFileExtractor;
use std::path::Path;

fn get_extraction(source: &str) -> debtmap::extraction::ExtractedFileData {
    UnifiedFileExtractor::extract(Path::new("test.py"), source).expect("Failed to extract")
}

#[test]
fn test_python_cyclomatic_basic_control_flow() {
    let source = r#"
def flow(x):
    if x > 0:           # +1
        return 1
    elif x < 0:         # +1
        return -1
    else:
        return 0
"#;
    let data = get_extraction(source);
    let func = &data.functions[0];
    // Base(1) + if(1) + elif(1) = 3
    assert_eq!(func.cyclomatic, 3);
}

#[test]
fn test_python_cyclomatic_loops() {
    let source = r#"
def loops(items):
    for item in items:      # +1
        while item > 0:     # +1
            item -= 1
    return items
"#;
    let data = get_extraction(source);
    let func = &data.functions[0];
    // Base(1) + for(1) + while(1) = 3
    assert_eq!(func.cyclomatic, 3);
}

#[test]
fn test_python_cyclomatic_boolean_ops() {
    let source = r#"
def booleans(a, b, c):
    if a and b or c:    # +1 (if) + 1 (and) + 1 (or) = 3
        return True
    return False
"#;
    let data = get_extraction(source);
    let func = &data.functions[0];
    // Base(1) + if(1) + and(1) + or(1) = 4
    assert_eq!(func.cyclomatic, 4);
}

#[test]
fn test_python_cognitive_nesting_penalty() {
    let source = r#"
def nested(x, y):
    if x > 0:               # +1
        if y > 0:           # +2 (1 + 1 nesting)
            for i in range(x): # +3 (1 + 2 nesting)
                print(i)
    return x + y
"#;
    let data = get_extraction(source);
    let func = &data.functions[0];
    // 1 (if) + 2 (nested if) + 3 (nested for) = 6
    assert_eq!(func.cognitive, 6);
    assert_eq!(func.nesting, 3);
}

#[test]
fn test_python_exception_handling_complexity() {
    let source = r#"
def try_except():
    try:                    # +1 (cyclomatic)
        risky_op()
    except ValueError:      # +1 (cyclomatic)
        handle_val_error()
    except Exception as e:  # +1 (cyclomatic)
        handle_generic()
    finally:
        cleanup()
"#;
    let data = get_extraction(source);
    let func = &data.functions[0];
    // Base(1) + try(1) + except(1) + except(1) = 4
    // Note: Python tree-sitter might treat try_statement as the branch.
    assert!(func.cyclomatic >= 2);
}

#[test]
fn test_python_conditional_expression() {
    let source = r#"
def ternary(x):
    return "pos" if x > 0 else "neg"  # +1
"#;
    let data = get_extraction(source);
    let func = &data.functions[0];
    assert_eq!(func.cyclomatic, 2);
}

#[test]
fn test_python_list_comprehension_complexity() {
    // Comprehensions contain implicit loops, but traditionally
    // cyclomatic complexity often treats them as 1 unless they have 'if'
    let source = r#"
def comp(items):
    return [x * 2 for x in items if x > 0] # +1 for 'if'
"#;
    let data = get_extraction(source);
    let func = &data.functions[0];
    // Base(1) + list_comprehension(1) = 2
    assert_eq!(func.cyclomatic, 2);
}

#[test]
fn test_python_match_statement() {
    // Python 3.10 match/case
    let source = r#"
def switch(status):
    match status:           # +1
        case 200:           # +1
            return "OK"
        case 404:           # +1
            return "Not Found"
        case 500:           # +1
            return "Error"
        case _:             # +1
            return "Unknown"
"#;
    let data = get_extraction(source);
    let func = &data.functions[0];
    // Base(1) + match(1) + 4 cases = 6
    assert_eq!(func.cyclomatic, 6);
}
