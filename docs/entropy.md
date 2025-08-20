# Entropy-Based Complexity Scoring

## Overview

The entropy-based complexity scoring system uses information theory principles to distinguish between genuinely complex code and pattern-based code that appears complex but is actually simple and repetitive. This significantly reduces false positives in complexity detection, particularly for validation functions, dispatchers, and configuration parsers.

## Methodology

### Shannon Entropy

Shannon entropy measures the randomness or variety in code patterns. It quantifies the information content of the code:

```
H(X) = -Σ p(xi) * log2(p(xi))
```

Where:
- `H(X)` is the entropy
- `p(xi)` is the probability of token type i appearing
- The sum is over all unique token types

**High entropy** indicates varied, unpredictable code patterns (genuinely complex)
**Low entropy** indicates repetitive, predictable patterns (pattern-based simplicity)

### Pattern Repetition Detection

The system analyzes AST patterns to identify repetitive structures:

1. **Pattern Extraction**: Converts each AST node into a simplified pattern representation
2. **Frequency Analysis**: Counts how often each pattern appears
3. **Repetition Score**: Calculates the ratio of repeated patterns to total patterns

Example patterns detected:
- `if-stmt` for if statements
- `match-N` for match/switch expressions with N arms
- `call-functionName` for function calls
- `return` for return statements

### Branch Similarity Analysis

For conditional statements (if/else, switch/match), the system measures how similar the branches are:

1. **Token Sequence Extraction**: Extracts simplified token sequences from each branch
2. **Pairwise Comparison**: Compares all pairs of branches
3. **Similarity Score**: Calculates average similarity across all branch pairs

High similarity indicates pattern-based code like:
- Validation chains with similar error returns
- Switch statements with similar case handlers
- Dispatchers with uniform handling

### Effective Complexity Calculation

The final effective complexity combines all three metrics:

```rust
effective_complexity = 1.0 - (simplicity_factor * 0.9)

where:
simplicity_factor = (1.0 - token_entropy) * pattern_repetition * similarity_weight
```

This produces a multiplier between 0.1 (very simple patterns) and 1.0 (genuinely complex).

## Configuration

Enable and configure entropy scoring in `.debtmap.toml`:

```toml
[entropy]
enabled = true          # Enable entropy-based scoring (default: false)
weight = 0.5           # Weight of entropy in complexity adjustment (0.0-1.0)
min_tokens = 20        # Minimum tokens required for calculation
pattern_threshold = 0.7 # Pattern similarity threshold for detection
```

### Configuration Guidelines

- **`enabled`**: Set to `true` to activate entropy-based complexity dampening
- **`weight`**: Controls how much entropy affects the final complexity score
  - `0.0` = No effect (traditional complexity only)
  - `0.5` = Balanced (default)
  - `1.0` = Maximum entropy influence
- **`min_tokens`**: Functions with fewer tokens skip entropy calculation
- **`pattern_threshold`**: Higher values require more similarity to trigger dampening

## Examples

### Pattern-Based Validation (Low Entropy)

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

**Traditional Complexity**: 5 (high)
**Entropy Analysis**:
- Token Entropy: 0.3 (low variety, repetitive patterns)
- Pattern Repetition: 0.8 (high repetition of if-return pattern)
- Branch Similarity: 0.9 (very similar error returns)
- **Effective Complexity**: 1.5 (reduced by 70%)

### Genuine Business Logic (High Entropy)

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

**Traditional Complexity**: 8 (high)
**Entropy Analysis**:
- Token Entropy: 0.8 (high variety, different calculations)
- Pattern Repetition: 0.2 (low repetition)
- Branch Similarity: 0.3 (branches do different things)
- **Effective Complexity**: 7.2 (minimal reduction, genuinely complex)

### Dispatcher Pattern (Medium Entropy)

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

**Traditional Complexity**: 7 (high)
**Entropy Analysis**:
- Token Entropy: 0.4 (moderate variety)
- Pattern Repetition: 0.7 (repetitive dispatch pattern)
- Branch Similarity: 0.8 (similar structure in each arm)
- **Effective Complexity**: 2.8 (reduced by 60%)

## Performance Optimization

### Token Caching

The entropy analyzer includes an efficient caching system:

```rust
// Using cached entropy calculation
let mut analyzer = EntropyAnalyzer::with_cache_size(1000);
let score = analyzer.calculate_entropy_cached(&block, &function_hash);

// Get cache statistics
let stats = analyzer.get_cache_stats();
println!("Cache hit rate: {:.2}%", stats.hit_rate * 100.0);
```

Cache features:
- LRU eviction when cache is full
- Configurable maximum cache size
- Hit/miss tracking for performance monitoring
- Memory-efficient storage (~128 bytes per entry)

### Performance Characteristics

- **First Analysis**: ~10% overhead for entropy calculation
- **Cached Analysis**: <1% overhead (50%+ speedup with caching)
- **Memory Usage**: Linear with cache size (default 1000 entries ≈ 128KB)

## Language Support

### Rust
Full entropy analysis support with syn-based AST parsing:
- All control flow patterns
- Match expressions with arm similarity
- Closure and nested function handling

### JavaScript/TypeScript
Full support via tree-sitter parsing:
- ES6+ syntax including async/await
- Switch statement analysis
- Arrow functions and method definitions
- JSX/TSX pattern detection

### Python
Planned support in future releases

## Integration with Unified Scoring

Entropy scores are integrated into the unified debt prioritization system:

1. **Complexity Factor Adjustment**: Base complexity is multiplied by effective complexity
2. **Priority Score Impact**: Lower effective complexity reduces overall priority
3. **Threshold Application**: Configurable thresholds for different entropy levels

## Interpreting Results

### Entropy Score Components

When using `--verbose` output, entropy details are displayed:

```json
{
  "entropy_details": {
    "token_entropy": 0.35,
    "pattern_repetition": 0.75,
    "branch_similarity": 0.80,
    "effective_complexity": 0.28,
    "dampening_applied": true,
    "dampening_factor": 0.72,
    "reasoning": [
      "High pattern repetition detected (75%)",
      "Similar branch structures found (80% similarity)",
      "Complexity reduced by 72% due to pattern-based code"
    ]
  }
}
```

### Guidelines for Interpretation

1. **Token Entropy < 0.4**: Indicates simple, repetitive patterns
2. **Pattern Repetition > 0.6**: Significant structural repetition
3. **Branch Similarity > 0.7**: Conditional branches are very similar
4. **Effective Complexity < 0.5**: Code is pattern-based, not genuinely complex

## Best Practices

### When to Enable Entropy Scoring

Enable entropy scoring when:
- Your codebase has many validation functions
- You use dispatcher or command patterns
- You have configuration parsers with many similar checks
- Traditional complexity metrics produce too many false positives

### Tuning Recommendations

1. **Start with defaults**: Enable with default weight (0.5)
2. **Monitor results**: Check if genuine complexity is still detected
3. **Adjust weight**: Increase for more dampening, decrease for less
4. **Set thresholds**: Adjust pattern_threshold based on your code style

### Limitations

Entropy scoring may not be suitable for:
- Very small functions (< 20 tokens)
- Highly mathematical or algorithmic code
- Code with intentional repetition for clarity
- Generated code that should be flagged despite patterns

## Future Enhancements

Planned improvements include:
- Machine learning for pattern recognition
- Cross-project entropy baselines
- IDE integration with real-time feedback
- Entropy-guided refactoring suggestions
- Historical entropy trend analysis

## Technical Details

### Algorithm Complexity

- Token Extraction: O(n) where n = number of AST nodes
- Entropy Calculation: O(m) where m = number of unique tokens
- Pattern Detection: O(n) for AST traversal
- Branch Similarity: O(b²) where b = number of branches
- Overall: O(n) for typical functions

### Memory Requirements

- Token Storage: ~50 bytes per unique token type
- Pattern Map: ~100 bytes per unique pattern
- Cache Entry: ~128 bytes per cached function
- Total: Typically < 1MB for large codebases with caching