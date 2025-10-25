# AST-Based Functional Pattern Detection (Spec 111)

## Overview

Debtmap includes AST-based functional composition analysis that detects and evaluates functional programming patterns in Rust code. This feature helps identify well-composed functional code, measure code purity, and assess the quality of functional transformations.

## Features

- **Pipeline Detection**: Identifies iterator chains (`.map()`, `.filter()`, `.fold()`, etc.)
- **Purity Analysis**: Evaluates functions for side effects and referential transparency
- **Composition Quality Scoring**: Measures how well code uses functional patterns
- **Configurable Profiles**: Three analysis profiles to match your codebase's functional programming style

## Usage

### Command Line

Enable functional analysis using the `--ast-functional-analysis` flag:

```bash
# Enable with default (balanced) profile
debtmap analyze src/ --ast-functional-analysis

# Use a specific profile
debtmap analyze src/ --ast-functional-analysis --functional-analysis-profile strict
debtmap analyze src/ --ast-functional-analysis --functional-analysis-profile balanced
debtmap analyze src/ --ast-functional-analysis --functional-analysis-profile lenient
```

### Configuration File

Add functional analysis settings to `.debtmap.toml`:

```toml
[functional_analysis]
min_pipeline_depth = 2
max_closure_complexity = 5
min_purity_score = 0.8
composition_quality_threshold = 0.6
min_function_complexity = 3
```

## Analysis Profiles

### Strict Profile

Best for codebases emphasizing functional purity:

- `min_pipeline_depth = 3` - Requires longer iterator chains
- `max_closure_complexity = 3` - Enforces simple closures
- `min_purity_score = 0.9` - High purity threshold
- `composition_quality_threshold = 0.75` - Strict quality requirements

**Use when**: Your codebase follows strict functional programming principles

### Balanced Profile (Default)

Suitable for typical Rust codebases:

- `min_pipeline_depth = 2` - Moderate pipeline requirements
- `max_closure_complexity = 5` - Reasonable closure complexity
- `min_purity_score = 0.8` - Balanced purity expectations
- `composition_quality_threshold = 0.6` - Standard quality threshold

**Use when**: You want a balance between functional and imperative styles

### Lenient Profile

For imperative-heavy or legacy codebases:

- `min_pipeline_depth = 1` - Accepts short chains
- `max_closure_complexity = 8` - Allows complex closures
- `min_purity_score = 0.6` - Lower purity requirements
- `composition_quality_threshold = 0.4` - Relaxed quality standards

**Use when**: You're gradually adopting functional patterns or working with legacy code

## Detected Patterns

### Iterator Pipelines

```rust
// Detected as high-quality functional composition
fn process_data(items: Vec<i32>) -> Vec<i32> {
    items.iter()
        .filter(|&x| *x > 0)
        .map(|x| x * 2)
        .collect()
}
```

### Pure Functions

```rust
// High purity score - no side effects, referentially transparent
fn calculate_discount(price: f64, rate: f64) -> f64 {
    price * (1.0 - rate)
}
```

### Impure Functions

```rust
// Low purity score - contains I/O side effects
fn log_and_calculate(x: i32) -> i32 {
    println!("Calculating for: {}", x);  // Side effect
    x * 2
}
```

## Metrics Provided

### Composition Metrics

Each function analyzed with functional analysis receives:

- **pipelines**: List of detected iterator pipelines
- **purity_score**: Score from 0.0 to 1.0 indicating function purity
- **composition_quality_score**: Overall quality of functional composition
- **has_side_effects**: Boolean indicating if side effects were detected
- **side_effect_kind**: Type of side effect (I/O, mutation, etc.)

### Pipeline Information

For each pipeline:

- **depth**: Number of chained operations
- **stages**: Individual operations in the pipeline
- **terminal_op**: Final operation (collect, fold, etc.)
- **is_parallel**: Whether the pipeline uses parallel iterators

## Performance

The AST-based functional analysis is designed to have minimal performance impact:

- **Overhead**: < 10% additional analysis time (validated in tests)
- **Lazy Evaluation**: Only analyzes functions when enabled
- **Cached Results**: Metrics are cached for repeated analyses

## Accuracy

Validation testing ensures high accuracy:

- **Precision**: ≥ 90% - When it reports a functional pattern, it's correct
- **Recall**: ≥ 85% - It finds most functional patterns present
- **F1 Score**: ≥ 0.87 - Balanced overall accuracy

## Integration with Debt Scoring

Functional analysis integrates with debtmap's debt scoring system:

- Well-composed functional code may receive lower debt scores
- Impure functions with side effects are flagged for review
- Complex closures contribute to cognitive complexity

## Examples

### Before and After

**Before functional analysis:**
```bash
debtmap analyze src/
# Only sees cyclomatic and cognitive complexity
```

**After functional analysis:**
```bash
debtmap analyze src/ --ast-functional-analysis
# Additionally sees:
# - Iterator pipeline usage
# - Function purity levels
# - Composition quality
# - Functional pattern detection
```

### Output Differences

With functional analysis enabled, function metrics include:

```json
{
  "name": "process_items",
  "cyclomatic_complexity": 3,
  "cognitive_complexity": 4,
  "composition_metrics": {
    "pipelines": [
      {
        "depth": 4,
        "stages": ["filter", "map", "flat_map", "collect"],
        "terminal_op": "collect",
        "is_parallel": false
      }
    ],
    "purity_score": 0.95,
    "composition_quality_score": 0.82,
    "has_side_effects": false,
    "side_effect_kind": null
  }
}
```

## Troubleshooting

### Analysis Not Running

If functional analysis doesn't seem to be working:

1. Ensure you're using the `--ast-functional-analysis` flag
2. Check that you're analyzing Rust files (`.rs` extension)
3. Verify the function complexity meets minimum thresholds

### Performance Issues

If analysis is too slow:

1. Use the lenient profile for faster analysis
2. Increase `min_function_complexity` to skip trivial functions
3. Consider disabling for very large codebases (>100k LOC)

### False Positives/Negatives

To tune detection accuracy:

1. Adjust profile settings in `.debtmap.toml`
2. Modify `min_pipeline_depth` to change sensitivity
3. Change `composition_quality_threshold` for scoring

## Further Reading

- [Functional Programming in Rust](https://doc.rust-lang.org/book/ch13-00-functional-features.html)
- [Iterator Documentation](https://doc.rust-lang.org/std/iter/trait.Iterator.html)
- [Debtmap Configuration Guide](./CONFIGURATION.md)

## Implementation Details

The functional analysis:

1. Parses Rust code into an Abstract Syntax Tree (AST)
2. Traverses the AST to identify functional patterns
3. Analyzes iterator method chains (map, filter, fold, etc.)
4. Evaluates function purity by detecting side effects
5. Calculates composition quality based on pattern usage
6. Integrates metrics into the standard analysis output

The analysis is **opt-in** via CLI flag or config file and adds minimal overhead to ensure it doesn't impact normal debtmap usage.
