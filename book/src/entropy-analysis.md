# Entropy Analysis

## Overview

Entropy analysis is an advanced feature that helps debtmap distinguish between genuinely complex code and pattern-based code that appears complex but is actually simple and repetitive. By using information theory principles, entropy analysis significantly reduces false positives in complexity detection.

This is especially valuable for:
- **Validation functions** with many similar checks
- **Dispatchers** with uniform case handling
- **Configuration parsers** with repetitive validation
- **Error handling** with similar error returns

## How It Works

Entropy analysis examines three key aspects of your code to determine if high complexity scores are justified:

### 1. Shannon Entropy Calculation

Shannon entropy measures the randomness or variety in code patterns. The formula is:

```
H(X) = -Σ p(xi) * log2(p(xi))
```

Where `p(xi)` is the probability of each token type appearing in the code.

**What it means:**
- **High entropy (>0.7)**: Code has varied, unpredictable patterns - genuinely complex
- **Low entropy (<0.4)**: Code has repetitive, predictable patterns - pattern-based simplicity

**Example of low entropy code:**
```rust
fn validate_input(value: i32) -> Result<(), String> {
    if value < 0 {
        return Err("Value must be non-negative".to_string());
    }
    if value > 100 {
        return Err("Value must be <= 100".to_string());
    }
    if value % 2 != 0 {
        return Err("Value must be even".to_string());
    }
    // Token entropy: 0.3 - very repetitive pattern
    Ok(())
}
```

### 2. Pattern Repetition Detection

The analyzer extracts patterns from your code's abstract syntax tree (AST) and measures how often they repeat:

1. **Pattern Extraction**: Each AST node becomes a simplified pattern
   - `if-stmt` for if statements
   - `match-5` for match/switch with 5 arms
   - `call-validate` for function calls
   - `return` for return statements

2. **Frequency Analysis**: Counts how many times each pattern appears

3. **Repetition Score**: Calculates the ratio of repeated patterns to total patterns

**High repetition (>0.6)** indicates structural repetition, common in:
- Validation chains with similar structure
- Switch statements with uniform case handling
- Dispatchers with consistent patterns

### 3. Branch Similarity Analysis

For conditional statements (if/else, switch/match), the analyzer measures how similar the branches are:

1. **Token Sequence Extraction**: Extracts simplified token sequences from each branch
2. **Pairwise Comparison**: Compares all pairs of branches
3. **Similarity Score**: Calculates average similarity across all branch pairs

**High similarity (>0.7)** indicates pattern-based code like:
```rust
fn process_command(cmd: &str) -> String {
    match cmd {
        "start" => execute_start(),   // All branches have
        "stop" => execute_stop(),      // the same structure:
        "pause" => execute_pause(),    // one function call
        "status" => execute_status(),
        _ => execute_unknown(),
    }
}
```

## Token Classification

Tokens are categorized and weighted for entropy calculation:

| Category | Weight | Examples |
|----------|--------|----------|
| ControlFlow | 1.2 | `if`, `match`, `for`, `while` |
| Keyword | 1.0 | `fn`, `let`, `return`, `struct` |
| Operator | 1.0 | `+`, `==`, `&&`, `->` |
| FunctionCall | 0.8 | Function invocations |
| Identifier | 0.5 | Variable names |
| Literal | 0.3 | Numbers, strings |

Control flow tokens have higher weights because they indicate potential complexity, while literals have lower weights since repetitive literals don't add complexity.

## Effective Complexity Adjustment

The three metrics combine to produce an **effective complexity multiplier**:

```
simplicity_factor = (1.0 - token_entropy) * pattern_repetition * similarity_weight
effective_complexity = 1.0 - (simplicity_factor * 0.9)
```

This produces a multiplier between **0.1** (very simple patterns) and **1.0** (genuinely complex).

**Step-by-step example:**

Given a validation function with:
- Token entropy: 0.3
- Pattern repetition: 0.8
- Branch similarity: 0.9
- Traditional complexity: 5

Calculation:
```
simplicity_factor = (1.0 - 0.3) * 0.8 * 0.9 = 0.504
effective_complexity = 1.0 - (0.504 * 0.9) = 0.546
adjusted_score = 5 * 0.546 = 2.73 ≈ 3

Result: Complexity reduced from 5 to 3 (40% reduction)
```

## Configuration

Enable and configure entropy analysis in `.debtmap.toml`:

```toml
[entropy]
# Enable entropy-based complexity adjustment
enabled = true

# Weight of entropy in final score (0.0-1.0)
# 0.0 = no effect, 0.5 = balanced, 1.0 = maximum effect
weight = 0.5

# Minimum tokens required for analysis
# Functions with fewer tokens skip entropy calculation
min_tokens = 20

# Pattern similarity threshold (0.0-1.0)
# Higher values require more similarity to trigger dampening
pattern_threshold = 0.7

# Entropy threshold for low entropy detection (0.0-1.0)
# Values below this are considered low entropy
entropy_threshold = 0.4

# Branch similarity threshold (0.0-1.0)
# Values above this indicate similar branches
branch_threshold = 0.7

# Maximum complexity reduction limits (0.0-1.0)
max_entropy_reduction = 0.15      # For low token entropy
max_repetition_reduction = 0.25    # For high pattern repetition
max_branch_reduction = 0.20        # For similar branches
max_combined_reduction = 0.40      # Overall limit
```

### Configuration Guidelines

**`enabled`**: Set to `true` to activate entropy-based complexity dampening. Keep `false` if you prefer traditional complexity metrics only.

**`weight`**: Controls how much entropy affects the final complexity score:
- `0.0` = No effect (traditional complexity only)
- `0.5` = Balanced (recommended starting point)
- `1.0` = Maximum entropy influence

**`min_tokens`**: Functions smaller than this skip entropy calculation. Default of 20 is recommended since very small functions don't have enough data for meaningful entropy analysis.

**`pattern_threshold`**: Higher values require more similarity to trigger dampening. Increase if you're getting too much dampening; decrease if pattern-based code isn't being caught.

**`max_*_reduction`**: These limits prevent over-dampening. Even highly repetitive code maintains a minimum complexity score.

## Examples

### Example 1: Pattern-Based Validation (Low Entropy)

```rust
fn validate_input(value: i32) -> Result<(), String> {
    if value < 0 {
        return Err("Value must be non-negative".to_string());
    }
    if value > 100 {
        return Err("Value must be <= 100".to_string());
    }
    if value % 2 != 0 {
        return Err("Value must be even".to_string());
    }
    if value % 5 != 0 {
        return Err("Value must be divisible by 5".to_string());
    }
    Ok(())
}
```

**Analysis:**
- **Traditional Complexity**: 5 (high - 4 if statements)
- **Token Entropy**: 0.3 (low variety, repetitive patterns)
- **Pattern Repetition**: 0.8 (high repetition of if-return pattern)
- **Branch Similarity**: 0.9 (very similar error returns)
- **Effective Complexity**: 1.5
- **Reduction**: 70% (5 → 1.5)

**Why the reduction?** This is clearly pattern-based validation code. Each branch does essentially the same thing: check a condition and return an error. The high pattern repetition and branch similarity indicate this isn't genuinely complex.

### Example 2: Genuine Business Logic (High Entropy)

```rust
fn calculate_discount(customer_type: &str, purchase_amount: f64, loyalty_years: u32) -> f64 {
    let base_discount = match customer_type {
        "premium" => 0.15,
        "regular" => 0.05,
        _ => 0.0,
    };

    let loyalty_bonus = if loyalty_years > 5 {
        0.10
    } else if loyalty_years > 2 {
        0.05
    } else {
        0.0
    };

    let volume_discount = if purchase_amount > 1000.0 {
        0.08
    } else if purchase_amount > 500.0 {
        0.04
    } else {
        0.0
    };

    let total_discount = base_discount + loyalty_bonus + volume_discount;
    total_discount.min(0.25)
}
```

**Analysis:**
- **Traditional Complexity**: 8 (high - multiple branches, calculations)
- **Token Entropy**: 0.8 (high variety, different calculations)
- **Pattern Repetition**: 0.2 (low repetition)
- **Branch Similarity**: 0.3 (branches do different things)
- **Effective Complexity**: 7.2
- **Reduction**: 10% (8 → 7.2)

**Why minimal reduction?** This is genuinely complex business logic. Each conditional represents a different business rule with different calculations. The high token entropy and low branch similarity indicate real complexity.

### Example 3: Dispatcher Pattern (Medium Entropy)

```rust
fn process_command(cmd: &str) -> String {
    match cmd {
        "start" => execute_start(),
        "stop" => execute_stop(),
        "pause" => execute_pause(),
        "resume" => execute_resume(),
        "restart" => execute_restart(),
        "status" => execute_status(),
        _ => execute_unknown(),
    }
}
```

**Analysis:**
- **Traditional Complexity**: 7 (high - 7 match arms)
- **Token Entropy**: 0.4 (moderate variety)
- **Pattern Repetition**: 0.7 (repetitive dispatch pattern)
- **Branch Similarity**: 0.8 (similar structure in each arm)
- **Effective Complexity**: 2.8
- **Reduction**: 60% (7 → 2.8)

**Why the reduction?** This is a straightforward dispatcher. All branches have identical structure (one function call), and the pattern is highly repetitive. While traditional metrics count 7 branches as high complexity, the entropy analysis recognizes this as a simple routing pattern.

## Use Cases

### When to Enable Entropy Analysis

**✅ Enable when:**
- Your codebase has many validation functions
- You use dispatcher or command patterns
- You have configuration parsers with many similar checks
- Traditional complexity metrics produce too many false positives
- You want to focus on genuinely complex code

**❌ Don't enable when:**
- You want all branches counted equally regardless of similarity
- Your codebase is mostly algorithmic/mathematical code
- You prefer conservative complexity scoring
- You're analyzing generated code that should be flagged

### Tuning Recommendations

1. **Start with defaults**: Enable with `weight = 0.5`
2. **Monitor results**: Check if genuine complexity is still detected
3. **Adjust weight**:
   - Increase to 0.7-1.0 for more dampening
   - Decrease to 0.3-0.4 for less dampening
4. **Adjust thresholds**:
   - Increase `pattern_threshold` if too much dampening
   - Decrease `entropy_threshold` to catch more low-entropy code

### Interpreting Results

When using verbose output (`--verbose`), entropy details are displayed:

```json
{
  "entropy_details": {
    "token_entropy": 0.35,
    "pattern_repetition": 0.75,
    "branch_similarity": 0.80,
    "effective_complexity": 0.28,
    "dampening_applied": true,
    "dampening_factor": 0.72
  }
}
```

**Interpretation guidelines:**
- **token_entropy < 0.4**: Simple, repetitive token patterns
- **pattern_repetition > 0.6**: Significant structural repetition
- **branch_similarity > 0.7**: Conditional branches are very similar
- **effective_complexity < 0.5**: Pattern-based code, not genuinely complex

## Performance and Caching

### Token Caching

The entropy analyzer includes an efficient LRU cache to avoid recalculating entropy for identical code patterns:

**Cache features:**
- LRU eviction when cache is full
- Configurable maximum cache size (default: 1000 entries)
- Hit/miss tracking for performance monitoring
- Memory-efficient storage (~128 bytes per entry)

### Performance Characteristics

- **First Analysis**: ~10% overhead for entropy calculation
- **Cached Analysis**: <1% overhead (50%+ speedup with cache hits)
- **Memory Usage**: Linear with cache size (default 1000 entries ≈ 128KB)

The performance impact is minimal, and caching makes repeated analysis very efficient.

## Language Support

| Language | Support Level | Notes |
|----------|---------------|-------|
| Rust | ✅ Full | Complete support with syn-based AST parsing |
| JavaScript | ✅ Full | ES6+ syntax, async/await, JSX/TSX |
| TypeScript | ✅ Full | All TypeScript-specific syntax supported |
| Python | 🚧 Partial | Basic support, ongoing improvements |

All languages support the core entropy features: token analysis, pattern repetition, and branch similarity.

## Troubleshooting

### "Entropy calculation skipped for function"

**Cause**: Function has fewer tokens than `min_tokens` threshold.

**Solution**: This is normal for small functions. Reduce `min_tokens` if needed, but small functions rarely need entropy analysis.

### "Too much dampening - genuine complexity reduced"

**Cause**: Weight or thresholds are too aggressive.

**Solution**:
1. Reduce `weight` from 0.5 to 0.3
2. Increase `pattern_threshold` from 0.7 to 0.8
3. Increase `entropy_threshold` from 0.4 to 0.5

### "Not enough dampening - pattern-based code still flagged"

**Cause**: Weight or thresholds are too conservative.

**Solution**:
1. Increase `weight` from 0.5 to 0.7
2. Decrease `pattern_threshold` from 0.7 to 0.6
3. Decrease `entropy_threshold` from 0.4 to 0.3

### "Cache hit rate is low"

**Cause**: Code varies significantly between analyses.

**Solution**: This is normal for diverse codebases. The cache helps most with repeated analysis of similar code patterns.

## Best Practices

### Do's

✅ Start with default settings and adjust based on results
✅ Use verbose output to understand how entropy affects scoring
✅ Monitor both high and low entropy cases in your codebase
✅ Combine with traditional complexity metrics for full picture
✅ Review entropy-adjusted scores to validate accuracy

### Don'ts

❌ Don't set `weight` to 1.0 without thorough testing
❌ Don't disable entropy for specific code patterns - use thresholds instead
❌ Don't ignore genuine complexity that happens to have patterns
❌ Don't set `min_tokens` too low (<10) - entropy needs sufficient data
❌ Don't rely solely on entropy - it's a complement to traditional metrics

## Related Topics

- [Configuration](configuration.md) - Complete configuration reference
- [Analysis Guide](analysis-guide.md) - Understanding complexity metrics
- [Scoring Strategies](scoring-strategies.md) - How entropy integrates with scoring
- [CLI Reference](cli-reference.md) - Command-line options for entropy analysis

## Summary

Entropy analysis helps you focus on genuinely complex code by identifying and dampening pattern-based complexity. By analyzing token variety, pattern repetition, and branch similarity, it reduces false positives from validation functions, dispatchers, and other repetitive code patterns.

**Key takeaways:**
- Enable with default settings first
- Monitor results and tune as needed
- Use verbose output to understand scoring
- Combine with traditional metrics for best results
- Great for validation-heavy and dispatcher-pattern codebases
