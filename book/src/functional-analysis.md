# Functional Composition Analysis

Debtmap provides deep AST-based analysis to detect and evaluate functional programming patterns in Rust code. This feature helps you understand how effectively your codebase uses functional composition patterns like iterator pipelines, identify opportunities for refactoring imperative code to functional style, and rewards pure, side-effect-free functions in complexity scoring.

## Overview

Functional analysis examines your code at the AST level to detect:

- **Iterator pipelines** - Chains like `.iter().map().filter().collect()`
- **Purity analysis** - Functions with no mutable state or side effects
- **Composition quality metrics** - Overall functional programming quality scores
- **Side effect classification** - Categorization of Pure, Benign, and Impure side effects

This analysis integrates with debtmap's scoring system, providing score bonuses for high-quality functional code and reducing god object warnings for codebases with many small pure helper functions.

**Implementation**: This feature is implemented in [src/analysis/functional_composition.rs](../../src/analysis/functional_composition.rs) with accuracy targets of precision ≥90%, recall ≥85%, F1 ≥0.87, and performance overhead <10%.

## Configuration Profiles

Debtmap provides three pre-configured analysis profiles to match different codebases:

| Profile | Use Case | Min Pipeline Depth | Max Closure Complexity | Purity Threshold | Quality Threshold |
|---------|----------|-------------------|------------------------|------------------|-------------------|
| **Strict** | Functional-first codebases | 3 | 3 | 0.9 | 0.7 |
| **Balanced** (default) | Typical Rust projects | 2 | 5 | 0.8 | 0.6 |
| **Lenient** | Imperative-heavy legacy code | 2 | 10 | 0.5 | 0.4 |

### Choosing a Profile

**Use Strict** when:
- Your codebase emphasizes functional programming patterns
- You want to enforce high purity standards
- You're building a new project with functional-first principles
- You want to detect even simple pipelines (3+ stages)

**Use Balanced** (default) when:
- You have a typical Rust codebase mixing functional and imperative styles
- You want reasonable detection without being overly strict
- You're working on a mature project with mixed patterns
- You want to reward functional patterns without penalizing pragmatic imperative code

**Use Lenient** when:
- You're analyzing legacy code with heavy imperative patterns
- You want to identify only the most obviously functional code
- You're migrating from an imperative codebase and want gradual improvement
- You have complex closures that are still fundamentally functional

### CLI Usage

Enable functional analysis with the `--ast-functional-analysis` flag and select a profile with `--functional-analysis-profile`:

```bash
# Enable with balanced profile (default)
debtmap analyze . --ast-functional-analysis --functional-analysis-profile balanced

# Use strict profile for functional-first codebases
debtmap analyze . --ast-functional-analysis --functional-analysis-profile strict

# Use lenient profile for legacy code
debtmap analyze . --ast-functional-analysis --functional-analysis-profile lenient
```

**Note:** The `--ast-functional-analysis` flag enables the feature, while `--functional-analysis-profile` selects the configuration profile (strict/balanced/lenient).

## Pure Function Detection

A function is considered pure when it:
1. Returns same output for same input (deterministic)
2. Has no observable side effects
3. Doesn't mutate external state
4. Doesn't perform I/O

### Examples

```rust
// Pure function
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Pure function with internal iteration
fn factorial(n: u32) -> u32 {
    (1..=n).product()  // Pure despite internal iteration
}

// Not pure: I/O side effect
fn log_and_add(a: i32, b: i32) -> i32 {
    println!("Adding {} and {}", a, b);  // Side effect!
    a + b
}

// Not pure: mutates external state
fn increment_counter(counter: &mut i32) -> i32 {
    *counter += 1;  // Side effect!
    *counter
}
```

## Pipeline Detection

Debtmap detects functional pipelines through deep AST analysis, identifying iterator chains and their transformations.

### Pipeline Stages

The analyzer recognizes these pipeline stage types:

#### 1. Iterator Initialization
Methods that start an iterator chain:
- `.iter()` - Immutable iteration
- `.into_iter()` - Consuming iteration
- `.iter_mut()` - Mutable iteration

```rust
// Detected iterator initialization
let results = collection.iter()
    .map(|x| x * 2)
    .collect();
```

#### 2. Map Transformations
Applies a transformation function to each element:

```rust
// Detected Map stage
items.iter()
    .map(|x| x * 2)          // Simple closure (low complexity)
    .map(|x| {                // Complex closure (higher complexity)
        let doubled = x * 2;
        doubled + 1
    })
    .collect()
```

The analyzer tracks **closure complexity** for each map operation. Complex closures may indicate code smells and affect quality scoring based on your `max_closure_complexity` threshold.

#### 3. Filter Predicates
Selects elements based on a predicate:

```rust
// Detected Filter stage
items.iter()
    .filter(|x| *x > 0)      // Simple predicate
    .filter(|x| {             // Complex predicate
        x.is_positive() && x < 100
    })
    .collect()
```

#### 4. Fold/Reduce Aggregation
Combines elements into a single value:

```rust
// Detected Fold stage
items.iter()
    .fold(0, |acc, x| acc + x)

// Or using reduce
items.iter()
    .reduce(|a, b| a + b)
```

#### 5. FlatMap Transformations
Maps and flattens nested structures:

```rust
// Detected FlatMap stage
items.iter()
    .flat_map(|x| vec![x, x * 2])
    .collect()
```

#### 6. Inspect (Side-Effect Aware)
Performs side effects while passing through values:

```rust
// Detected Inspect stage (affects purity scoring)
items.iter()
    .inspect(|x| println!("Processing: {}", x))
    .map(|x| x * 2)
    .collect()
```

#### 7. Result/Option Chaining
Specialized stages for error handling:

```rust
// Detected AndThen stage
results.iter()
    .and_then(|x| try_process(x))
    .collect()

// Detected MapErr stage
results.iter()
    .map_err(|e| format!("Error: {}", e))
    .collect()
```

### Terminal Operations

Pipelines typically end with a terminal operation that consumes the iterator:

- **`collect()`** - Gather elements into a collection
- **`sum()`** - Sum numeric values
- **`count()`** - Count elements
- **`any()`** - Check if any element matches
- **`all()`** - Check if all elements match
- **`find()`** - Find first matching element
- **`reduce()`** - Reduce to single value
- **`for_each()`** - Execute side effects for each element

```rust
// Complete pipeline with terminal operation
let total: i32 = items.iter()
    .filter(|x| **x > 0)
    .map(|x| x * 2)
    .sum();  // Terminal operation: sum
```

### Nested Pipelines

Debtmap detects pipelines nested within closures, indicating highly functional code patterns:

```rust
// Nested pipeline detected
let results = outer_items.iter()
    .map(|item| {
        // Inner pipeline (nesting_level = 1)
        item.values.iter()
            .filter(|v| **v > 0)
            .collect()
    })
    .collect();
```

**Nesting level** tracking helps identify sophisticated functional composition patterns.

### Parallel Pipelines

Parallel iteration using Rayon is automatically detected:

```rust
use rayon::prelude::*;

// Detected as parallel pipeline (is_parallel = true)
let results: Vec<_> = items.par_iter()
    .filter(|x| **x > 0)
    .map(|x| x * 2)
    .collect();
```

Parallel pipelines indicate high-performance functional patterns and receive positive quality scoring.

### Builder Pattern Filtering

To avoid false positives, debtmap distinguishes builder patterns from functional pipelines:

```rust
// This is a builder pattern, NOT counted as a functional pipeline
let config = ConfigBuilder::new()
    .with_host("localhost")
    .with_port(8080)
    .build();

// This IS a functional pipeline
let values = items.iter()
    .map(|x| x * 2)
    .collect();
```

Builder patterns are filtered out to ensure accurate functional composition metrics.

## Purity Analysis

Debtmap analyzes functions to determine their purity level - whether they have side effects and mutable state.

### Purity Levels

Functions are classified into three purity levels for god object weighting (defined in `src/organization/purity_analyzer.rs`):

> **Note:** Debtmap has two purity analysis systems serving different purposes:
> 1. **PurityLevel** (three levels) - Used for god object scoring with weight multipliers (this section)
> 2. **PurityLevel** (four levels) - Used in `src/analysis/purity_analysis.rs` for detailed responsibility classification (Strictly Pure, Locally Pure, Read-Only, Impure)
>
> This chapter focuses on the three-level system for god object integration.

#### Pure (Weight 0.3)
Guaranteed no side effects:
- No mutable parameters (`&mut`, `mut self`)
- No I/O operations
- No global mutations
- No `unsafe` blocks
- Only immutable bindings

```rust
// Pure function
fn calculate_total(items: &[i32]) -> i32 {
    items.iter().sum()
}

// Pure function with immutable bindings
fn process_value(x: i32) -> i32 {
    let doubled = x * 2;  // Immutable binding
    let result = doubled + 10;
    result
}
```

#### Probably Pure (Weight 0.5)
Likely no side effects:
- Static functions (`fn` items, not methods)
- Associated functions (no `self`)
- No obvious side effects detected

```rust
// Probably pure - static function
fn transform(value: i32) -> i32 {
    value * 2
}

// Probably pure - associated function
impl MyType {
    fn create_default() -> Self {
        MyType { value: 0 }
    }
}
```

#### Impure (Weight 1.0)
Has side effects:
- Uses mutable references (`&mut`, `mut self`)
- Performs I/O operations (`println!`, file I/O, network)
- Uses `async` (potential side effects)
- Mutates global state
- Uses `unsafe`

```rust
// Impure - mutable reference
fn increment(value: &mut i32) {
    *value += 1;
}

// Impure - I/O operation
fn log_value(value: i32) {
    println!("Value: {}", value);
}

// Impure - mutation
fn process_items(items: &mut Vec<i32>) {
    items.push(42);
}
```

### Purity Weight Multipliers

Purity levels affect god object detection through weight multipliers (implemented in `src/organization/purity_analyzer.rs:29-39`). Pure functions contribute **less** to god object scores, rewarding codebases with many small pure helper functions:

- **Pure (0.3)**: A pure function counts as 30% of a regular function in god object method count calculations
- **Probably Pure (0.5)**: Counts as 50%
- **Impure (1.0)**: Full weight

The `purity_score` dampens god object scores via the `weight_multiplier` calculation. For example, pure functions with weight 0.3 count as only 30% of a regular function when calculating method counts for god object detection.

**Example**: A module with 20 pure helper functions (20 × 0.3 = 6.0 effective) is less likely to trigger god object warnings than a module with 10 impure functions (10 × 1.0 = 10.0 effective).

## Side Effect Detection

### Detected Side Effects

**I/O Operations:**
- File reading/writing
- Network calls
- Console output
- Database queries

**State Mutation:**
- Mutable global variables
- Shared mutable state
- Reference mutations

**Randomness:**
- Random number generation
- Time-dependent behavior

**System Interaction:**
- Environment variable access
- System calls
- Thread spawning

### Rust-Specific Detection

```rust
// Interior mutability detection
use std::cell::RefCell;

fn has_side_effect() {
    let data = RefCell::new(vec![]);
    data.borrow_mut().push(1);  // Detected as mutation
}

// Unsafe code detection
fn unsafe_side_effect() {
    unsafe {
        // Automatically flagged as potentially impure
    }
}
```

### Side Effect Classification

Side effects are categorized by severity:

#### Pure - No Side Effects
No mutations, I/O, or global state changes:

```rust
// Pure - only computation
fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

#### Benign - Small Penalty
Only logging, tracing, or metrics:

```rust
use tracing::debug;

// Benign - logging side effect
fn process(value: i32) -> i32 {
    debug!("Processing value: {}", value);
    value * 2
}
```

Benign side effects receive a **small penalty** in purity scoring. Logging and observability are recognized as practical necessities.

#### Impure - Large Penalty
I/O, mutations, network operations:

```rust
// Impure - file I/O
fn save_to_file(data: &str) -> std::io::Result<()> {
    std::fs::write("output.txt", data)
}

// Impure - network operation
async fn fetch_data(url: &str) -> Result<String, reqwest::Error> {
    reqwest::get(url).await?.text().await
}
```

Impure side effects receive a **large penalty** in purity scoring.

### Purity Metrics

For each function, debtmap calculates purity metrics through the functional composition analysis (`src/analysis/functional_composition.rs`). These metrics are computed by `analyze_composition()` and returned in `CompositionMetrics` and `PurityMetrics`:

- **`has_mutable_state`** - Whether the function uses mutable bindings
- **`has_side_effects`** - Whether I/O or global mutations are detected
- **`immutability_ratio`** - Ratio of immutable to total bindings (0.0-1.0)
- **`is_const_fn`** - Whether declared as `const fn`
- **`side_effect_kind`** - Classification: Pure, Benign, or Impure
- **`purity_score`** - Overall purity score (0.0 impure to 1.0 pure)

#### Immutability Ratio

The immutability ratio measures how much of a function's local state is immutable:

```rust
fn example() {
    let x = 10;         // Immutable
    let y = 20;         // Immutable
    let mut z = 30;     // Mutable
    z += 1;
    // immutability_ratio = 2/3 = 0.67
}
```

Higher immutability ratios contribute to better purity scores.

## Composition Pattern Recognition

### Function Composition

```rust
// Detected composition pattern
fn process_data(input: String) -> Result<Output> {
    input
        .parse()
        .map(validate)
        .and_then(transform)
        .map(normalize)
}
```

### Higher-Order Functions

```rust
// Detected HOF pattern
fn apply_twice<F>(f: F, x: i32) -> i32
where
    F: Fn(i32) -> i32,
{
    f(f(x))
}
```

### Map/Filter/Fold Chains

```rust
// Detected functional pipeline
let result = items
    .iter()
    .filter(|x| x.is_valid())
    .map(|x| x.transform())
    .fold(0, |acc, x| acc + x);
```

## Composition Quality Scoring

Debtmap combines pipeline metrics and purity analysis into an overall **composition quality score** (0.0-1.0).

### Scoring Factors

The composition quality score considers:

1. **Pipeline depth** - Longer pipelines indicate more functional composition
2. **Purity score** - Higher purity means better functional programming
3. **Immutability ratio** - More immutable bindings improve the score
4. **Closure complexity** - Simpler closures score better
5. **Parallel execution** - Parallel pipelines receive bonuses
6. **Nested pipelines** - Sophisticated composition patterns score higher

### Quality Thresholds

Based on your configuration profile, functions with composition quality above the threshold receive **score boosts** in debtmap's overall analysis:

- **Strict**: Quality ≥ 0.7 required for boost
- **Balanced**: Quality ≥ 0.6 required for boost
- **Lenient**: Quality ≥ 0.4 required for boost

High-quality functional code can offset complexity in other areas of your codebase.

### Purity Scoring

#### Distribution Analysis

Debtmap calculates purity distribution:
- **Pure functions**: 0 side effects detected
- **Mostly pure**: Minor side effects (e.g., logging)
- **Impure**: Multiple side effects
- **Highly impure**: Extensive state mutation and I/O

#### Scoring Formula

```
Purity Score = (pure_functions / total_functions) × 100
Side Effect Density = total_side_effects / total_functions
```

#### Codebase Health Metrics

```
Target Purity Levels:
- Core business logic: 80%+ pure
- Utilities: 70%+ pure
- I/O layer: 20-30% pure (expected)
- Overall: 50%+ pure
```

### Integration with Risk Scoring

Functional composition quality integrates with debtmap's risk scoring system and multi-signal aggregation framework:

- **High composition quality** → Lower risk scores (functions with quality above threshold receive score boosts)
- **Pure functions** → Reduced god object penalties (via weight multipliers in `purity_analyzer.rs`)
- **Deep pipelines** → Bonus for functional patterns
- **Impure side effects** → Risk penalties applied

**Multi-Signal Integration**: Functional composition analysis is one of several signals aggregated in the unified analysis system (`src/builders/unified_analysis.rs` and `src/analysis/multi_signal_aggregation.rs`) alongside complexity metrics, god object detection, and risk assessment. This ensures that functional programming quality contributes to the comprehensive technical debt assessment across multiple dimensions.

This integration ensures that well-written functional code is properly rewarded in the overall technical debt assessment.

## Practical Examples

### Example 1: Detecting Imperative vs Functional Code

**Imperative style** (lower composition quality):

```rust
fn process_items_imperative(items: Vec<i32>) -> Vec<i32> {
    let mut results = Vec::new();
    for item in items {
        if item > 0 {
            results.push(item * 2);
        }
    }
    results
}
// Detected: No pipelines, mutable state, lower purity score
```

**Functional style** (higher composition quality):

```rust
fn process_items_functional(items: Vec<i32>) -> Vec<i32> {
    items.iter()
        .filter(|x| **x > 0)
        .map(|x| x * 2)
        .collect()
}
// Detected: Pipeline depth 3, pure function, high composition quality
```

### Example 2: Identifying Refactoring Opportunities

When debtmap detects low composition quality, it suggests refactoring:

```rust
// Original: Imperative with mutations
fn calculate_statistics(data: &[f64]) -> (f64, f64, f64) {
    let mut sum = 0.0;
    let mut min = f64::MAX;
    let mut max = f64::MIN;

    for &value in data {
        sum += value;
        if value < min { min = value; }
        if value > max { max = value; }
    }

    (sum / data.len() as f64, min, max)
}

// Refactored: Functional style
fn calculate_statistics_functional(data: &[f64]) -> (f64, f64, f64) {
    let sum: f64 = data.iter().sum();
    let min = data.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max = data.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

    (sum / data.len() as f64, *min, *max)
}
// Higher purity score, multiple pipelines detected
```

### Example 3: Using Profiles for Different Codebases

**Strict profile** - Catches subtle functional patterns:

```bash
$ debtmap analyze --ast-functional-analysis --functional-analysis-profile strict src/
# Detects pipelines with 3+ stages
# Requires purity ≥ 0.9 for "pure" classification
# Flags closures with complexity > 3
```

**Balanced profile** - Default for most projects:

```bash
$ debtmap analyze --ast-functional-analysis --functional-analysis-profile balanced src/
# Detects pipelines with 2+ stages
# Requires purity ≥ 0.8 for "pure" classification
# Flags closures with complexity > 5
```

**Lenient profile** - For legacy code:

```bash
$ debtmap analyze --ast-functional-analysis --functional-analysis-profile lenient src/
# Detects pipelines with 2+ stages
# Requires purity ≥ 0.5 for "pure" classification
# Flags closures with complexity > 10
```

### Example 4: Interpreting Purity Scores

**Pure function** (score: 1.0):
```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}
// Purity: 1.0 (perfect)
// Immutability ratio: 1.0 (no bindings)
// Side effects: None
```

**Mostly pure** (score: 0.8):
```rust
fn process(values: &[i32]) -> i32 {
    let doubled: Vec<_> = values.iter().map(|x| x * 2).collect();
    let sum: i32 = doubled.iter().sum();
    sum
}
// Purity: 0.8 (high)
// Immutability ratio: 1.0 (both bindings immutable)
// Side effects: None
// Pipelines: 2 detected
```

**Impure function** (score: 0.2):
```rust
fn log_and_process(values: &mut Vec<i32>) {
    println!("Processing {} items", values.len());
    values.iter_mut().for_each(|x| *x *= 2);
}
// Purity: 0.2 (low)
// Immutability ratio: 0.0 (mutable parameter)
// Side effects: I/O (println), mutation
```

## Best Practices

### Writing Functional Rust Code

To achieve high composition quality scores:

1. **Prefer iterator chains over manual loops**
   ```rust
   // Good
   let evens: Vec<_> = items.iter().filter(|x| *x % 2 == 0).collect();

   // Avoid
   let mut evens = Vec::new();
   for item in &items {
       if item % 2 == 0 { evens.push(item); }
   }
   ```

2. **Minimize mutable state**
   ```rust
   // Good
   let result = calculate(input);

   // Avoid
   let mut result = 0;
   result = calculate(input);
   ```

3. **Separate pure logic from side effects**
   ```rust
   // Good - pure computation
   fn calculate_price(quantity: u32, unit_price: f64) -> f64 {
       quantity as f64 * unit_price
   }

   // Good - I/O at the boundary
   fn display_price(price: f64) {
       println!("Total: ${:.2}", price);
   }
   ```

4. **Keep closures simple**
   ```rust
   // Good - simple closure
   items.map(|x| x * 2)

   // Consider extracting - complex closure
   items.map(|x| {
       let temp = expensive_operation(x);
       transform(temp)
   })

   // Better
   fn transform_item(x: i32) -> i32 {
       let temp = expensive_operation(x);
       transform(temp)
   }
   items.map(transform_item)
   ```

5. **Use parallel iteration for CPU-intensive work**
   ```rust
   use rayon::prelude::*;

   let results: Vec<_> = large_dataset.par_iter()
       .map(|item| expensive_computation(item))
       .collect();
   ```

### Code Organization

**Separate pure from impure:**
- Keep pure logic in core modules
- Isolate I/O at boundaries
- Use dependency injection for testability

**Maximize purity in:**
- Business logic
- Calculations and transformations
- Validation functions
- Data structure operations

**Accept impurity in:**
- I/O layers
- Logging and monitoring
- External system integration
- Application boundaries

**Refactoring strategy:**
1. Identify impure functions
2. Extract pure logic
3. Push side effects to boundaries
4. Test pure functions exhaustively

### Migration Guide

To enable functional analysis on existing projects:

1. **Start with lenient profile** to understand current state:
   ```bash
   debtmap analyze --ast-functional-analysis --functional-analysis-profile lenient .
   ```

2. **Identify quick wins** - functions that are almost functional:
   - Look for loops that can become iterator chains
   - Find mutable variables that can be immutable
   - Spot side effects that can be extracted

3. **Gradually refactor** to functional patterns:
   - Convert one function at a time
   - Run tests after each change
   - Measure improvements with debtmap

4. **Tighten profile** as codebase improves:
   ```bash
   # After refactoring
   debtmap analyze --ast-functional-analysis --functional-analysis-profile balanced .

   # For new modules
   debtmap analyze --ast-functional-analysis --functional-analysis-profile strict src/new_module/
   ```

5. **Monitor composition quality trends** over time

## Use Cases

### Code Quality Audit

```bash
# Assess functional purity
debtmap analyze . --ast-functional-analysis --functional-analysis-profile balanced --format markdown
```

### Refactoring Targets

```bash
# Find impure functions in core logic
debtmap analyze src/core/ --ast-functional-analysis --functional-analysis-profile strict
```

### Onboarding Guide

```bash
# Show functional patterns in codebase
debtmap analyze . --ast-functional-analysis --functional-analysis-profile balanced --summary
```

## Troubleshooting

### "No pipelines detected" but I have iterator chains

- **Check pipeline depth**: Your chains may be too short for the profile
  - Strict requires 3+ stages
  - Balanced/Lenient require 2+ stages
- **Check for builder patterns**: Method chaining for construction is filtered out
- **Verify terminal operation**: Ensure the chain ends with `collect()`, `sum()`, etc.

### "Low purity score" for seemingly pure functions

- **Check for hidden side effects**:
  - `println!` or logging statements
  - Calls to impure helper functions
  - `unsafe` blocks
- **Review immutability ratio**: Unnecessary `mut` bindings lower the score
- **Verify no I/O operations**: File access, network calls affect purity

### "High complexity closures flagged"

- **Extract complex closures** into named functions:
  ```rust
  // Instead of
  items.map(|x| { /* 10 lines */ })

  // Use
  fn process_item(x: Item) -> Result { /* 10 lines */ }
  items.map(process_item)
  ```
- **Adjust `max_closure_complexity`**: Consider lenient profile if needed
- **Refactor closure logic**: Break down complex operations

### Too Many False Positives

**Issue:** Pure functions flagged as impure

**Solution:**
- Use lenient profile
- Suppress known patterns
- Review detection criteria
- Report false positives

### Missing Side Effects

**Issue:** Known impure functions not detected

**Solution:**
- Use strict profile
- Check for exotic side effect patterns
- Enable comprehensive analysis

### Performance impact concerns

- **Spec 111 targets <10% overhead**: Performance impact should be minimal
- **Disable for hot paths**: Analyze functional patterns in separate runs if needed
- **Use caching**: Debtmap caches analysis results between runs

## Related Chapters

- [Analysis Guide](analysis-guide.md) - Understanding analysis types
- [Metrics Reference](./metrics-reference.md) - How functional patterns affect complexity metrics
- [Scoring Strategies](./scoring-strategies.md) - Integration with overall technical debt scoring
- [God Object Detection](./god-object-detection.md) - How purity weights reduce false positives
- [Configuration](./configuration.md) - Advanced functional analysis configuration options

## Summary

Functional composition analysis helps you:

- **Identify functional patterns** in your Rust codebase through AST-based pipeline detection
- **Measure purity** with side effect detection and immutability analysis
- **Improve code quality** by refactoring imperative code to functional style
- **Get scoring benefits** for high-quality functional programming patterns
- **Choose appropriate profiles** (strict/balanced/lenient) for different codebases

Enable it with `--functional-analysis-profile` to start benefiting from functional programming insights in your technical debt analysis.
