//! Call Graph Detection for Async and Same-File Calls
//!
//! This test validates that same-file function calls are correctly detected,
//! including calls within async closures (tokio::spawn).

use debtmap::analyzers::rust_call_graph::extract_call_graph;
use std::path::PathBuf;

/// Test same-file function calls from an orchestration function.
/// This mimics the `invoke` function in `hosaka/src/agent/ops/claude_runner.rs`.
#[test]
fn test_same_file_orchestration_calls() {
    let code = r#"
fn take_child_stdio(child: &mut Child) -> Result<(Stdout, Stderr), Error> {
    Ok((child.stdout.take().unwrap(), child.stderr.take().unwrap()))
}

fn parse_stream_json(line: &str) -> Option<String> {
    serde_json::from_str(line).ok()
}

fn interpret_process_result(result: Result<ExitStatus, Error>, name: &str) -> Result<(), Error> {
    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(Error::new(format!("{} exited with {}", name, status))),
        Err(e) => Err(e),
    }
}

async fn invoke(child: &mut Child) -> Result<(), Error> {
    let (stdout, stderr) = take_child_stdio(child)?;

    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        if let Some(content) = parse_stream_json(&line?) {
            println!("{}", content);
        }
    }

    let result = child.wait();
    interpret_process_result(result, "Claude")
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("claude_runner.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Check that all functions are detected
    let functions = call_graph.find_all_functions();
    assert!(
        functions.len() >= 4,
        "Should find at least 4 functions (invoke, take_child_stdio, parse_stream_json, interpret_process_result)"
    );

    // Find the invoke function
    let invoke_func = functions
        .iter()
        .find(|f| f.name == "invoke")
        .expect("Should find invoke function");

    // Check invoke's callees - this is the core BUG-001 assertion
    let invoke_callees = call_graph.get_callees(invoke_func);

    // invoke should call take_child_stdio
    assert!(
        invoke_callees.iter().any(|f| f.name == "take_child_stdio"),
        "invoke should call take_child_stdio. Found callees: {:?}",
        invoke_callees.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    // invoke should call parse_stream_json
    assert!(
        invoke_callees.iter().any(|f| f.name == "parse_stream_json"),
        "invoke should call parse_stream_json. Found callees: {:?}",
        invoke_callees.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    // invoke should call interpret_process_result
    assert!(
        invoke_callees
            .iter()
            .any(|f| f.name == "interpret_process_result"),
        "invoke should call interpret_process_result. Found callees: {:?}",
        invoke_callees.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    // Should have at least 3 callees (the 3 same-file functions)
    assert!(
        invoke_callees.len() >= 3,
        "invoke should have at least 3 callees, found {}",
        invoke_callees.len()
    );
}

/// Test calls inside tokio::spawn async blocks.
/// Validates that calls inside `tokio::spawn(async move { ... })` are attributed to parent.
#[test]
fn test_calls_inside_tokio_spawn() {
    let code = r#"
fn parse_stream_json(line: &str) -> Option<String> {
    serde_json::from_str(line).ok()
}

fn extract_text_block(content: &str) -> String {
    content.to_string()
}

async fn invoke() {
    let stdout_handle = tokio::spawn(async move {
        let line = "test";
        if let Some(content) = parse_stream_json(&line) {
            let text = extract_text_block(&content);
            println!("{}", text);
        }
    });

    stdout_handle.await.unwrap();
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    let functions = call_graph.find_all_functions();
    let invoke_func = functions
        .iter()
        .find(|f| f.name == "invoke")
        .expect("Should find invoke function");

    let invoke_callees = call_graph.get_callees(invoke_func);

    // Calls inside tokio::spawn should be attributed to parent function
    assert!(
        invoke_callees.iter().any(|f| f.name == "parse_stream_json"),
        "invoke should detect parse_stream_json inside tokio::spawn. Found: {:?}",
        invoke_callees.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    assert!(
        invoke_callees
            .iter()
            .any(|f| f.name == "extract_text_block"),
        "invoke should detect extract_text_block inside tokio::spawn. Found: {:?}",
        invoke_callees.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

/// Test that call graph correctly tracks bidirectional relationships.
/// If invoke calls helper, then helper's callers should include invoke.
#[test]
fn test_bidirectional_caller_callee_tracking() {
    let code = r#"
fn helper() -> i32 {
    42
}

fn caller() -> i32 {
    helper()
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    let functions = call_graph.find_all_functions();

    // Check caller's callees
    let caller_func = functions
        .iter()
        .find(|f| f.name == "caller")
        .expect("Should find caller function");
    let caller_callees = call_graph.get_callees(caller_func);
    assert!(
        caller_callees.iter().any(|f| f.name == "helper"),
        "caller should have helper as callee"
    );

    // Check helper's callers (reverse relationship)
    let helper_func = functions
        .iter()
        .find(|f| f.name == "helper")
        .expect("Should find helper function");
    let helper_callers = call_graph.get_callers(helper_func);
    assert!(
        helper_callers.iter().any(|f| f.name == "caller"),
        "helper should have caller as caller"
    );
}

/// Test nested async closures and multiple spawn calls.
#[test]
fn test_multiple_spawn_with_nested_calls() {
    let code = r#"
fn process_stdout(line: &str) -> Option<String> {
    Some(line.to_string())
}

fn process_stderr(line: &str) -> Option<String> {
    Some(line.to_string())
}

fn finalize() {
    // cleanup
}

async fn invoke() {
    let stdout_handle = tokio::spawn(async move {
        process_stdout("test");
    });

    let stderr_handle = tokio::spawn(async move {
        process_stderr("error");
    });

    stdout_handle.await.unwrap();
    stderr_handle.await.unwrap();
    finalize();
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    let functions = call_graph.find_all_functions();
    let invoke_func = functions
        .iter()
        .find(|f| f.name == "invoke")
        .expect("Should find invoke function");

    let invoke_callees = call_graph.get_callees(invoke_func);

    // Should detect all three same-file calls
    assert!(
        invoke_callees.iter().any(|f| f.name == "process_stdout"),
        "Should detect process_stdout in first spawn"
    );
    assert!(
        invoke_callees.iter().any(|f| f.name == "process_stderr"),
        "Should detect process_stderr in second spawn"
    );
    assert!(
        invoke_callees.iter().any(|f| f.name == "finalize"),
        "Should detect finalize direct call"
    );
}

/// Test that struct method calls within async blocks are detected.
#[test]
fn test_method_calls_in_async_blocks() {
    let code = r#"
struct Parser;

impl Parser {
    fn parse_json(&self, line: &str) -> Option<String> {
        Some(line.to_string())
    }

    fn validate(&self, content: &str) -> bool {
        !content.is_empty()
    }

    async fn process(&self) {
        tokio::spawn(async move {
            let parser = Parser;
            if let Some(content) = parser.parse_json("test") {
                parser.validate(&content);
            }
        });
    }
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    let functions = call_graph.find_all_functions();
    let process_func = functions
        .iter()
        .find(|f| f.name.contains("process"))
        .expect("Should find process method");

    let process_callees = call_graph.get_callees(process_func);

    // Should detect method calls inside the spawn block
    assert!(
        process_callees
            .iter()
            .any(|f| f.name.contains("parse_json")),
        "Should detect parse_json method call. Found: {:?}",
        process_callees.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}
