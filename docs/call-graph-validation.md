# Call Graph Validation

Call graph validation analyzes the structural integrity and health of your codebase's function call relationships. It identifies potential issues like orphaned functions, dangling references, and suspicious patterns.

## Overview

The call graph validator performs comprehensive checks to ensure your codebase's call graph is well-structured and maintainable. It categorizes issues by severity and provides an overall health score.

## Validation Categories

### Structural Issues

These are serious problems that indicate potential bugs or broken references:

#### 1. Dangling Edges
**What it is**: References to functions that don't exist in the call graph.

**Example**:
```rust
fn caller() {
    non_existent_function(); // Dangling edge
}
```

**Impact**: 10 points per occurrence on health score

**How to fix**:
- Ensure all called functions are properly analyzed
- Check for missing imports or module boundaries
- Verify macro expansions are captured

#### 2. Duplicate Nodes
**What it is**: The same function appears multiple times in the call graph.

**Example**: A function registered twice due to analysis errors.

**Impact**: 5 points per occurrence on health score

**How to fix**:
- Check for parsing errors in the source code
- Ensure consistent file path handling
- Report as a bug if reproducible

#### 3. Unreachable Functions
**What it is**: Functions with no callers but that call other functions (dead code with dependencies).

**Categories**:
- **Not entry points**: Regular functions that should be called but aren't
- **Orphaned subsystems**: Entire chains of functions with no external callers

**Example**:
```rust
// This function calls helpers but is never called itself
fn dead_feature() {
    helper_a();
    helper_b();
}

fn helper_a() { /* ... */ }
fn helper_b() { /* ... */ }
```

**Impact**: 1 point per occurrence on health score

**How to fix**:
- Remove dead code if truly unused
- Add the function to the orphan whitelist if it's work-in-progress
- Mark as an entry point if it's called externally (FFI, CLI commands, etc.)

#### 4. Isolated Functions
**What it is**: Functions with no callers and no callees (complete orphans).

**Example**:
```rust
// This function is completely isolated
fn unused_utility() {
    println!("Never called");
}
```

**Impact**: 0.5 points per occurrence on health score

**How to fix**:
- Remove if truly unused
- Add to orphan whitelist if it's work-in-progress or future-use
- Document why it exists if it's intentionally kept

### Warnings

Heuristic-based warnings about suspicious patterns:

#### 1. Too Many Callers (High Fan-In)
**Threshold**: > 50 callers

**What it means**: Function is called from many places, indicating potential coupling issues.

**Impact**: 2 points per occurrence on health score

**Consider**:
- Is this a utility function that should be in a shared module?
- Could callers be refactored to reduce coupling?
- Is this a god object or utility belt anti-pattern?

#### 2. Too Many Callees (High Fan-Out)
**Threshold**: > 50 callees

**What it means**: Function calls many other functions, indicating high complexity.

**Impact**: 2 points per occurrence on health score

**Consider**:
- Break down into smaller, focused functions
- Extract logical groupings into helper functions
- Check for violation of single responsibility principle

#### 3. File With No Calls
**What it means**: All functions in a file have no callers.

**Impact**: 2 points per occurrence on health score

**Indicates**:
- Work-in-progress module
- Dead code that should be removed
- Missing integration points

#### 4. Unused Public Function
**What it means**: A public-looking function has no callers.

**Impact**: 2 points per occurrence on health score

**Consider**:
- Is this part of a public API?
- Should visibility be reduced to private?
- Is this actually called externally (outside analyzed codebase)?

### Informational

Observations that are not issues but provide useful insights:

#### 1. Leaf Functions
**What it is**: Functions that have callers but don't call anything else.

**Examples**: Utility functions, calculations, data transformations

**Not a problem**: This is normal and expected.

#### 2. Self-Referential Functions (Recursive)
**What it is**: Functions that call themselves.

**Examples**:
```rust
fn factorial(n: u32) -> u32 {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}
```

**Not a problem**: Recursion is a valid pattern when appropriate.

## Orphan Categories

The validator distinguishes between different types of orphaned nodes:

### True Orphans vs. Expected Orphans

**True Orphans** (flagged as issues):
- Regular functions with no callers
- Not entry points
- Not in configured whitelist

**Expected Orphans** (NOT flagged):
1. **Entry Points**:
   - `main()` functions
   - Test functions (`test_*`, `*::test_*`)
   - Benchmark functions (`bench_*`)
   - Functions in `examples/` or `benches/` directories
   - Trait implementations (Default, Clone, From, Into, Display)
   - Constructor patterns (`new()`, `builder()`, `with_*()`)

2. **Whitelisted Functions**:
   - Work-in-progress features
   - Future APIs
   - FFI entry points
   - Plugin systems
   - CLI command handlers

3. **Leaf Functions**:
   - Have callers (not orphaned)
   - Informational only

4. **Self-Referential Functions**:
   - Call themselves (recursive)
   - Not isolated

## Health Score Calculation

The health score ranges from 0 (poor) to 100 (excellent) and is calculated as:

```
Starting Score: 100

Deductions:
- Dangling edges: -10 points each (critical)
- Duplicate nodes: -5 points each (serious)
- Unreachable functions: -1 point each (moderate)
- Isolated functions: -0.5 points each (low)
- Warnings: -2 points each (minor)
- Info items: 0 points (informational only)

Final Score: max(0, Starting Score - Total Deductions)
```

### Score Interpretation

- **90-100**: Excellent - Well-maintained codebase with minimal issues
- **70-89**: Good - Some minor issues but generally healthy
- **50-69**: Fair - Moderate issues that should be addressed
- **30-49**: Poor - Significant structural problems
- **0-29**: Critical - Major issues requiring immediate attention

## Configuration

### Orphan Whitelist

Use the orphan whitelist to exclude expected orphaned functions from being flagged as issues:

```rust
use debtmap::analyzers::call_graph::validation::{
    CallGraphValidator, CallGraphValidationConfig
};

let mut config = CallGraphValidationConfig::new();

// Add work-in-progress functions
config.add_orphan_whitelist("experimental_feature".to_string());
config.add_orphan_whitelist("future_api".to_string());

// Add FFI entry points
config.add_orphan_whitelist("ffi_initialize".to_string());

// Validate with config
let report = CallGraphValidator::validate_with_config(&call_graph, &config);
```

**When to whitelist**:
- Work-in-progress features not yet integrated
- Future APIs reserved for backward compatibility
- FFI/C API entry points called from external code
- Plugin system hooks
- CLI command handlers registered dynamically

### Additional Entry Points

Mark functions as entry points that wouldn't be detected automatically:

```rust
let mut config = CallGraphValidationConfig::new();

// Mark custom entry points
config.add_entry_point("plugin_init".to_string());
config.add_entry_point("custom_main".to_string());

let report = CallGraphValidator::validate_with_config(&call_graph, &config);
```

**When to use**:
- Dynamic dispatch systems
- Plugin architectures
- Custom runtime entry points
- Callback systems
- Event handlers

### Builder Pattern

Configuration supports method chaining:

```rust
let mut config = CallGraphValidationConfig::new();
config
    .add_orphan_whitelist("wip_feature".to_string())
    .add_orphan_whitelist("future_api".to_string())
    .add_entry_point("plugin_main".to_string())
    .add_entry_point("cli_handler".to_string());
```

## Common Patterns

### Entry Point Detection

The validator automatically recognizes these entry point patterns:

1. **Main functions**: `fn main()`
2. **Test functions**: `fn test_*()`, `mod::test_*()`
3. **Benchmarks**: `fn bench_*()`, in `benches/` directory
4. **Examples**: Functions in `examples/` directory
5. **Trait implementations**:
   - `Default::default`
   - `Clone::clone`, `Clone::clone_box`
   - `From::from`, `Into::into`
   - `Display::fmt`, `Debug::fmt`
   - `Type::new`, `Type::builder`, `Type::create`

### Trait Implementations

Trait implementations are recognized as entry points because they're called through trait dispatch:

```rust
impl Default for Config {
    fn default() -> Self {  // Recognized as entry point
        Config { /* ... */ }
    }
}

impl Clone for MyType {
    fn clone(&self) -> Self {  // Recognized as entry point
        MyType { /* ... */ }
    }
}
```

**Note**: This recognition is heuristic-based. For non-standard traits or complex patterns, use the configuration to mark functions explicitly.

## Limitations and Future Work

### Current Limitations

1. **Indirect Call Tracking**: Function pointers, closures, and callbacks are not fully tracked
   - Requires data flow analysis
   - Conservative approach: May flag some legitimately-used functions as orphaned

2. **Macro-Generated Code**: Some macro-expanded functions may not be detected
   - Depends on macro expansion visibility
   - May require explicit whitelisting

3. **Dynamic Dispatch**: Trait object calls (`dyn Trait`) have limited resolution
   - Implementations marked conservatively as potentially callable
   - Spec 152 addresses trait method resolution improvements

4. **Cross-Crate Calls**: External crate usage not fully tracked
   - Public API functions may appear orphaned
   - Use entry point configuration to mark public APIs

### Future Enhancements

These features are planned for future versions:

1. **Data Flow Analysis**: Track indirect calls through function pointers and closures (requires interprocedural analysis)

2. **Trait Object Resolution**: Better handling of dynamic trait dispatch (Spec 152)

3. **Public API Detection**: Automatically recognize public module exports

4. **Call Graph Visualization**: Interactive visualization of orphaned nodes and their relationships

5. **Confidence Scoring**: Assign confidence levels to orphan detection ("definitely orphaned", "likely orphaned", "uncertain")

## Examples

### Example 1: Basic Validation

```rust
use debtmap::priority::call_graph::CallGraph;
use debtmap::analyzers::call_graph::validation::CallGraphValidator;

let call_graph = CallGraph::new();
// ... build call graph from analysis ...

let report = CallGraphValidator::validate(&call_graph);

println!("Health Score: {}/100", report.health_score);
println!("Structural Issues: {}", report.structural_issues.len());
println!("Warnings: {}", report.warnings.len());
println!("Total Functions: {}", report.statistics.total_functions);
println!("Entry Points: {}", report.statistics.entry_points);
println!("Orphaned: {}", report.statistics.isolated_functions);
```

### Example 2: Validation with Configuration

```rust
use debtmap::analyzers::call_graph::validation::{
    CallGraphValidator, CallGraphValidationConfig
};

let mut config = CallGraphValidationConfig::new();

// Whitelist work-in-progress features
config
    .add_orphan_whitelist("experimental_parser".to_string())
    .add_orphan_whitelist("future_optimization".to_string());

// Mark plugin entry points
config
    .add_entry_point("plugin_initialize".to_string())
    .add_entry_point("plugin_execute".to_string());

let report = CallGraphValidator::validate_with_config(&call_graph, &config);

// Whitelisted functions won't be flagged as orphaned
assert!(report.health_score > 80);
```

### Example 3: Handling Validation Results

```rust
let report = CallGraphValidator::validate(&call_graph);

// Check for critical issues
for issue in &report.structural_issues {
    match issue {
        StructuralIssue::DanglingEdge { caller, callee } => {
            eprintln!("ERROR: {} calls non-existent function {}",
                      caller.name, callee.name);
        }
        StructuralIssue::UnreachableFunction { function, .. } => {
            println!("WARNING: Unreachable function: {}", function.name);
        }
        StructuralIssue::IsolatedFunction { function } => {
            println!("INFO: Isolated function: {} (consider removing)",
                     function.name);
        }
        StructuralIssue::DuplicateNode { function, count } => {
            eprintln!("ERROR: Function {} appears {} times",
                      function.name, count);
        }
    }
}

// Review warnings
for warning in &report.warnings {
    match warning {
        ValidationWarning::TooManyCallers { function, count } => {
            println!("WARNING: {} has {} callers (high coupling)",
                     function.name, count);
        }
        ValidationWarning::TooManyCallees { function, count } => {
            println!("WARNING: {} calls {} functions (high complexity)",
                     function.name, count);
        }
        // ... other warning types
    }
}
```

## Best Practices

1. **Regular Validation**: Run call graph validation as part of CI/CD pipeline

2. **Incremental Cleanup**: Address high-severity issues first (dangling edges, duplicates)

3. **Whitelist Judiciously**: Only whitelist functions you're certain should be orphaned

4. **Document Whitelists**: Add comments explaining why functions are whitelisted

5. **Track Health Score**: Monitor health score trends over time

6. **Review Periodically**: Revisit whitelisted functions to check if they can be removed

7. **Use with Dead Code Detection**: Combine with dead code analysis for comprehensive cleanup

## Integration with debtmap

Call graph validation is integrated into debtmap's analysis pipeline and works alongside:

- **Dead Code Detection**: Identifies unused functions based on call graph
- **Complexity Analysis**: High fan-out functions often correlate with high complexity
- **Dependency Scoring**: Orphaned nodes affect dependency priority
- **Risk Assessment**: Unreachable code may hide bugs or security issues

## References

- **Spec 151**: Improve Call Graph Orphaned Node Detection
- **Spec 152**: Improve Trait Method Call Graph Resolution
- **Source**: `src/analyzers/call_graph/validation.rs`
- **Tests**: `src/analyzers/call_graph/validation.rs` (tests module)
