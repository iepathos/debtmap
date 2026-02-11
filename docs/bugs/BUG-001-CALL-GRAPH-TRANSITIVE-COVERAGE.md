# BUG-001: Call Graph Detection and Transitive Coverage Failures

## Status
- **Status**: Open
- **Severity**: High
- **Created**: 2026-02-10
- **Component**: Rust Analyzer / Call Graph / Coverage

## Summary

Debtmap fails to detect function calls within the same file, resulting in broken transitive coverage calculation and incorrect role classification. This causes I/O orchestration functions to be misclassified as "PureLogic" and report 0% transitive coverage even when their callees are fully tested.

## Reproduction Case

**File**: `hosaka/src/agent/ops/claude_runner.rs`

**Function**: `invoke` (lines 25-87)

### Actual Debtmap Output

```yaml
Function: invoke
Downstream Callees: 0
Transitive Coverage: 0%
Role Multiplier: 1.30 (PureLogic)
```

### Expected Output

```yaml
Function: invoke
Downstream Callees: 4+
  - take_child_stdio (line 48)
  - parse_stream_json (line 56)
  - interpret_process_result (line 86)
  - tokio::spawn (lines 52, 65)
Transitive Coverage: >50% (parse_stream_json has 6 test hits)
Role Multiplier: ~0.5 (I/O Orchestration)
```

## Evidence

### 1. Function Clearly Calls Other Functions

```rust
async fn invoke(...) -> Result<(), BoxError> {
    // ...
    let (stdout, stderr) = take_child_stdio(&mut child)?;  // LINE 48
    // ...
    let stdout_handle = tokio::spawn(async move {
        // ...
        if let Some(content) = parse_stream_json(&line) {  // LINE 56
            // ...
        }
    });
    // ...
    interpret_process_result(result, "Claude")  // LINE 86
}
```

### 2. Callees Have Test Coverage

From `lcov.info`:
```
FNDA:6,_RNvNtNtNtCsgtFxOZeymfN_6hosaka5agent3ops13claude_runner17parse_stream_json
FNDA:1,_RNvNtNtNtCsgtFxOZeymfN_6hosaka5agent3ops13claude_runner18extract_text_block
FNDA:1,_RNvNtNtNtCsgtFxOZeymfN_6hosaka5agent3ops13claude_runner22extract_assistant_text
```

- `parse_stream_json`: 6 test executions
- `extract_text_block`: 1 test execution
- `extract_assistant_text`: 1 test execution

### 3. Role Classification Is Inverted

The function contains obvious I/O markers:
- `Command::new("claude")...spawn()?` - Process spawning
- `tokio::spawn(async move { ... })` - Async task spawning
- `BufReader::new(stdout)` - I/O buffering
- `tokio::time::timeout(...)` - Async timing
- `child.wait()` - Process waiting

Yet it's classified as `PureLogic` with a 1.30 multiplier (increases priority), when it should be classified as I/O Orchestration with ~0.5 multiplier (decreases priority).

## Root Cause Analysis

### Hypothesis 1: Same-File Call Detection

The call graph analyzer may only be detecting cross-file dependencies, missing intra-file function calls. This would explain:
- `Downstream Callees: 0` when there are clearly 4+ calls
- `Upstream Callers: 0` (no other function in codebase calls `invoke` directly?)

**Investigation**: Check `/src/analyzers/rust/call_graph.rs` or equivalent for how function calls are resolved.

### Hypothesis 2: Async Closure Boundary

Calls inside `tokio::spawn(async move { ... })` closures may not be attributed to the parent function. The call to `parse_stream_json` happens inside a spawned async block.

**Investigation**: Check if the AST visitor descends into closure bodies.

### Hypothesis 3: Transitive Coverage Dependency

Transitive coverage calculation depends on call graph accuracy. If callees aren't detected, transitive coverage can't be computed.

```
invoke -> [no callees detected] -> transitive coverage = direct coverage = 0%
```

### Hypothesis 4: Role Classification Logic

The `PureLogic` role is being assigned despite clear I/O markers. Possible issues:
- Purity analysis says `Is Pure: false` but role multiplier ignores this
- Role classification uses different heuristics than purity analysis
- The 1.30 multiplier for "PureLogic" seems backwards (pure functions are easier to test, should have lower priority)

## Impact

1. **False Positives**: I/O orchestration functions with tested pure helpers appear as high-priority debt
2. **Wasted Effort**: Janitor agent spends time evaluating functions that are correctly structured
3. **Wrong Priorities**: Functions that genuinely need tests are deprioritized relative to well-structured I/O wrappers
4. **Incorrect Metrics**: Transitive coverage is useless if call graph is broken

## Affected Components

| Component | File (Probable) | Issue |
|-----------|-----------------|-------|
| Call Graph Detection | `src/analyzers/rust/call_graph.rs` | Not detecting same-file calls |
| Transitive Coverage | `src/coverage/transitive.rs` | Depends on broken call graph |
| Role Classification | `src/priority/roles.rs` | Ignoring I/O markers, misclassifying as PureLogic |
| Purity Analysis | `src/analyzers/rust/purity.rs` | Correctly identifies impure, but not used in role |

## Suggested Fixes

### Fix 1: Same-File Call Detection

Ensure the call graph visitor:
1. Resolves function calls to definitions in the same file
2. Descends into closure/async block bodies
3. Handles method calls on `self` and associated functions

### Fix 2: Role Classification from Purity

```rust
fn classify_role(func: &FunctionAnalysis) -> Role {
    // If purity analysis says impure with I/O side effects, classify as I/O
    if !func.is_pure && func.has_io_side_effects() {
        return Role::IoOrchestration;
    }
    // ... existing logic
}
```

### Fix 3: Invert PureLogic Multiplier

Pure functions are easier to test and refactor. The multiplier should decrease priority, not increase it:

```rust
// Current (wrong)
Role::PureLogic => 1.30  // Increases priority

// Correct
Role::PureLogic => 0.70  // Decreases priority (pure = easy to test)
Role::IoOrchestration => 1.20  // Increases priority (harder to test)
```

Or better: use purity to adjust the coverage factor, not as a blanket multiplier.

## Test Cases

```rust
#[test]
fn test_same_file_call_detection() {
    let source = r#"
        fn helper() -> i32 { 42 }
        fn caller() -> i32 { helper() }
    "#;
    let call_graph = analyze_call_graph(source);
    assert!(call_graph.callees_of("caller").contains("helper"));
}

#[test]
fn test_async_closure_call_detection() {
    let source = r#"
        fn helper() {}
        async fn caller() {
            tokio::spawn(async { helper() });
        }
    "#;
    let call_graph = analyze_call_graph(source);
    assert!(call_graph.callees_of("caller").contains("helper"));
}

#[test]
fn test_io_function_role_classification() {
    let source = r#"
        async fn io_heavy() {
            Command::new("ls").spawn().unwrap();
        }
    "#;
    let analysis = analyze_function(source, "io_heavy");
    assert_eq!(analysis.role, Role::IoOrchestration);
    assert!(analysis.role_multiplier < 1.0); // Should decrease priority
}

#[test]
fn test_transitive_coverage_propagation() {
    // invoke calls parse_stream_json which has 100% coverage
    // invoke's transitive coverage should reflect this
    let coverage = calculate_transitive_coverage("invoke", &call_graph, &lcov);
    assert!(coverage > 0.0);
}
```

## Priority

**High** - This bug causes systematic misclassification of well-structured code, leading to:
- Noise in debt reports
- Wasted janitor cycles
- Erosion of trust in debtmap output

## Related

- SPEC-215: Function-Level Debt Suppression (workaround for this bug)
- Janitor agent instructions (had to add strict criteria due to this bug)
