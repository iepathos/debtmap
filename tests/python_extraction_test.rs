use debtmap::extraction::UnifiedFileExtractor;
use std::path::Path;

#[test]
fn test_python_extraction_basic() {
    let source = r#"
def add(a, b):
    return a + b

class Math:
    def multiply(self, a, b):
        return a * b
"#;
    let path = Path::new("test.py");
    let data = UnifiedFileExtractor::extract(path, source).expect("Failed to extract");

    assert_eq!(data.total_lines, 7);
    assert_eq!(data.functions.len(), 2);

    let add_func = data.functions.iter().find(|f| f.name == "add").unwrap();
    assert_eq!(add_func.parameter_names, vec!["a", "b"]);
    assert_eq!(add_func.cyclomatic, 1);

    let multiply_func = data
        .functions
        .iter()
        .find(|f| f.name == "multiply")
        .unwrap();
    assert_eq!(multiply_func.qualified_name, "Math.multiply");
    assert_eq!(multiply_func.parameter_names, vec!["self", "a", "b"]);
}

#[test]
fn test_python_complexity() {
    let source = r#"
def complex(x):
    if x > 0:
        if x > 10:
            return 1
        else:
            return 2
    elif x < 0:
        for i in range(10):
            print(i)
    return 0
"#;
    let path = Path::new("complex.py");
    let data = UnifiedFileExtractor::extract(path, source).expect("Failed to extract");

    let func = &data.functions[0];
    assert!(func.cyclomatic >= 5); // 1 (base) + if + if + elif + for = 5
    assert!(func.nesting >= 2);
}

#[test]
fn test_python_imports() {
    let source = r#"
import os
import sys as system
from collections import HashMap, deque
from math import *
"#;
    let path = Path::new("imports.py");
    let data = UnifiedFileExtractor::extract(path, source).expect("Failed to extract");

    let paths: Vec<_> = data.imports.iter().map(|i| &i.path).collect();
    assert!(paths.contains(&&"os".to_string()));
    assert!(paths.contains(&&"sys".to_string()));
    assert!(paths.contains(&&"collections.HashMap".to_string()));
    assert!(paths.contains(&&"collections.deque".to_string()));
    assert!(paths.contains(&&"math.*".to_string()));

    let sys_import = data.imports.iter().find(|i| i.path == "sys").unwrap();
    assert_eq!(sys_import.alias, Some("system".to_string()));
}

#[test]
fn test_python_io_detection() {
    let source = r#"
def io_func():
    print("hello")
    with open("test.txt", "r") as f:
        content = f.read()
    import socket
    s = socket.socket()
"#;
    let path = Path::new("io.py");
    let data = UnifiedFileExtractor::extract(path, source).expect("Failed to extract");

    let func = &data.functions[0];
    assert!(func
        .io_operations
        .iter()
        .any(|op| op.io_type == debtmap::extraction::IoType::Console));
    assert!(func
        .io_operations
        .iter()
        .any(|op| op.io_type == debtmap::extraction::IoType::File));
}

#[test]
fn test_python_test_lines() {
    let source = r#"
def regular_func():
    return 42

def test_func():
    assert regular_func() == 42
"#;
    let path = Path::new("test_lines.py");
    let data = UnifiedFileExtractor::extract(path, source).expect("Failed to extract");

    // test_func is 2 lines
    assert!(data.test_lines >= 2);
    assert!(data.production_lines() >= 2);
}

#[test]
fn test_python_extraction_handles_typed_default_parameters() {
    let source = r#"
def configure(
    host: str = "localhost",
    port: int = 5432,
    *args,
    **kwargs,
):
    return host, port, args, kwargs
"#;
    let path = Path::new("typed_default_params.py");
    let data = UnifiedFileExtractor::extract(path, source).expect("Failed to extract");

    let func = data
        .functions
        .iter()
        .find(|f| f.name == "configure")
        .expect("configure function should be extracted");

    assert_eq!(func.parameter_names, vec!["host", "port", "args", "kwargs"]);
}
