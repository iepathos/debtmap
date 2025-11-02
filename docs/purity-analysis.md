# Purity Analysis in Debtmap

## Overview

Debtmap includes advanced purity analysis to identify functions that are free of side effects. This helps prioritize refactoring efforts and understand code complexity more accurately.

## What is Function Purity?

A **pure function** is one that:
1. Returns the same output for the same inputs (deterministic)
2. Has no observable side effects (no I/O, mutations, or state changes)

Examples of pure functions:
```rust
// Pure: deterministic, no side effects
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Pure: only performs computation
fn calculate_discount(price: f64, rate: f64) -> f64 {
    price * (1.0 - rate)
}
```

Examples of impure functions:
```rust
// Impure: performs I/O
fn log_error(msg: &str) {
    eprintln!("Error: {}", msg);
}

// Impure: mutates external state
fn increment_counter(counter: &mut i32) {
    *counter += 1;
}
```

## Inter-Procedural Purity Analysis (Spec 156)

Debtmap uses a two-phase approach to analyze purity across your entire codebase:

### Phase 1: Intrinsic Analysis
Each function is analyzed in isolation to detect:
- I/O operations (file, network, console)
- Mutable references and state mutations
- Unsafe blocks and raw pointer operations
- System calls and FFI
- Global variable access

### Phase 2: Whole-Program Propagation
Purity information flows through the call graph:
- A function calling only pure functions can be classified as pure
- Recursive functions are analyzed for structural purity
- Cross-file dependencies are tracked
- Confidence decreases with call chain depth

## Benefits of Purity Analysis

### 1. Reduced False Negatives
**Before inter-procedural analysis**: 40-60% of pure functions missed
**After inter-procedural analysis**: <15% false negative rate

Example:
```rust
// Without propagation: Might be missed as pure
fn process_data(items: &[i32]) -> Vec<i32> {
    items.iter().map(|x| calculate(x)).collect()
}

// Helper is intrinsically pure
fn calculate(x: &i32) -> i32 {
    x * 2 + 1
}

// With propagation: Both correctly identified as pure
```

### 2. Better Refactoring Priorities

Pure functions with high complexity are excellent refactoring candidates because:
- They're easier to test (no mocks needed)
- They're safer to parallelize
- They're simpler to reason about
- They can be extracted without side effect concerns

### 3. Scoring Adjustments

Debtmap adjusts complexity scores based on purity:

| Purity Level | Confidence | Score Multiplier | Impact |
|-------------|------------|------------------|---------|
| Pure | High (>80%) | 0.70x | 30% reduction |
| Pure | Medium (60-80%) | 0.80x | 20% reduction |
| Pure | Low (<60%) | 0.90x | 10% reduction |
| Impure | N/A | 1.00x | No change |

This reflects that pure functions, even if complex, are inherently easier to maintain than impure ones.

## Understanding Purity Results

When you run debtmap, purity information appears in the analysis output:

```
Function: calculate_total
  Purity: Pure (confidence: 0.92)
  Reason: PropagatedFromDeps(depth: 2)
  Complexity: 15 (adjusted: 10.5 due to purity)
  Priority: Medium
```

### Purity Reasons

- **Intrinsic**: Function has no side effects or calls
- **PropagatedFromDeps**: All called functions are pure
- **RecursivePure**: Pure structural recursion (e.g., tree traversal)
- **RecursiveWithSideEffects**: Recursive with I/O or mutations
- **SideEffects**: Contains I/O, mutations, or other side effects
- **UnknownDeps**: Unable to analyze all dependencies

## Recursive Function Handling

Recursive functions receive special handling:

### Pure Recursion (Factorial Example)
```rust
fn factorial(n: u32) -> u32 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}
// Classification: Pure (confidence reduced by 30% due to recursion)
```

### Impure Recursion (Directory Traversal)
```rust
fn traverse_dir(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.path().is_dir() {
            files.extend(traverse_dir(&entry.path())?);
        } else {
            files.push(entry.path());
        }
    }
    Ok(files)
}
// Classification: Impure (I/O operations)
```

## Confidence Interpretation

Confidence scores indicate analysis certainty:

- **> 0.90**: Very High - Direct analysis, clear purity/impurity
- **0.75 - 0.90**: High - Propagated through 1-2 levels
- **0.60 - 0.75**: Medium - Deeper call chains or complex patterns
- **< 0.60**: Low - Many unknowns or very deep propagation

Lower confidence suggests:
- Review the function manually
- May benefit from explicit purity annotations
- Could have hidden dependencies

## Best Practices

### 1. Extract Pure Functions
When refactoring complex code, prioritize extracting pure subfunctions:

```rust
// Before: Complex impure function
fn process_and_save(data: &[i32]) -> io::Result<()> {
    let result: Vec<_> = data.iter()
        .map(|x| x * 2)
        .filter(|x| x > &10)
        .collect();
    save_to_file(&result)?;
    Ok(())
}

// After: Separated pure logic from I/O
fn transform_data(data: &[i32]) -> Vec<i32> {  // Pure!
    data.iter()
        .map(|x| x * 2)
        .filter(|x| x > &10)
        .collect()
}

fn process_and_save(data: &[i32]) -> io::Result<()> {
    let result = transform_data(data);
    save_to_file(&result)?;
    Ok(())
}
```

### 2. Test Pure Functions First
Pure functions are the easiest to test:
- No setup/teardown needed
- No mocking required
- Fast execution
- Easy to property-test

### 3. Parallelize Pure Functions
Pure functions are safe to run in parallel:
```rust
// Safe parallelization
use rayon::prelude::*;

let results: Vec<_> = data.par_iter()
    .map(|x| pure_calculation(x))
    .collect();
```

## Integration with Other Analyses

Purity analysis works together with:

- **Complexity metrics**: Pure complex functions are better refactoring targets
- **Test coverage**: Pure functions with low coverage are easy to test
- **Dead code detection**: Unused pure functions are safe to remove
- **Call graph analysis**: Understand purity propagation paths

## Technical Details

### Algorithm
1. **Parse phase**: Extract AST and detect local side effects
2. **Call graph phase**: Build function dependency graph
3. **Propagation phase**: Topological traversal, bottom-up purity inference
4. **Scoring phase**: Apply confidence-based multipliers to complexity

### Caching
Purity analysis results are cached to improve performance on large codebases. The cache is invalidated when:
- Source files change
- Dependencies change
- Analysis configuration changes

### Limitations
Current limitations include:
- Dynamic dispatch may reduce confidence
- Macro-generated code needs special handling
- Foreign function interfaces (FFI) assumed impure
- Trait methods require whole-program analysis for best results

## Future Enhancements

Planned improvements:
- User-provided purity annotations
- Effect system integration
- Better handling of trait methods
- IDE integration for real-time purity feedback

## See Also

- [Complexity Metrics](complexity-metrics.md)
- [Call Graph Analysis](call-graph.md)
- [Refactoring Guide](refactoring-guide.md)
