# Entropy Analysis

Entropy analysis is Debtmap's unique approach to distinguishing genuinely complex code from repetitive pattern-based code. This reduces false positives by 60-75% compared to traditional cyclomatic complexity metrics.

## Overview

Traditional static analysis tools flag code as "complex" based purely on cyclomatic complexity or lines of code. However, not all complexity is equal:

- **Repetitive patterns** (validation functions, dispatchers) have high cyclomatic complexity but low cognitive load
- **Diverse logic** (state machines, business rules) may have moderate cyclomatic complexity but high cognitive load

Entropy analysis uses information theory to distinguish between these cases.

## How It Works

### Shannon Entropy

Shannon entropy measures the variety and unpredictability of code patterns:

```
H(X) = -Σ p(x) × log₂(p(x))
```

Where:
- `p(x)` = probability of each token type
- High entropy (0.8-1.0) = many different patterns
- Low entropy (0.0-0.3) = repetitive patterns

### Token Classification

Debtmap can classify tokens by importance to give more weight to semantically significant tokens in entropy calculations. This is controlled by the `use_classification` configuration option.

**When enabled (default)**, tokens are weighted by importance:

**High importance (weight: 1.0):**
- Control flow keywords (`if`, `match`, `for`, `while`)
- Error handling (`try`, `catch`, `?`, `unwrap`)
- Async keywords (`async`, `await`)

**Medium importance (weight: 0.7):**
- Function calls
- Method invocations
- Operators

**Low importance (weight: 0.3):**
- Identifiers (variable names)
- Literals (strings, numbers)
- Punctuation

**When disabled** (`use_classification = false`), all tokens are treated equally, which may be useful for debugging or when you want unweighted entropy scores.

### Pattern Repetition Detection

Detects repetitive structures in the AST:

```rust
// Low pattern repetition (0.2) - all branches identical
if a.is_none() { return Err(...) }
if b.is_none() { return Err(...) }
if c.is_none() { return Err(...) }

// High pattern repetition (0.9) - diverse branches
match state {
    Active => transition_to_standby(),
    Standby => transition_to_active(),
    Maintenance => schedule_restart(),
}
```

### Branch Similarity Analysis

Analyzes similarity between conditional branches:

```rust
// High branch similarity (0.9) - branches are nearly identical
if condition_a {
    log("A happened");
    process_a();
}
if condition_b {
    log("B happened");
    process_b();
}

// Low branch similarity (0.2) - branches are very different
if needs_auth {
    authenticate_user()?;
    load_profile()?;
} else {
    show_guest_ui();
}
```

### Effective Complexity Adjustment

Debtmap applies complexity dampening based on **Spec 68: Graduated Entropy Dampening** only when code exhibits very low entropy (< 0.2), indicating highly repetitive patterns.

#### When Dampening Applies

**Threshold**: Entropy < 0.2 (very low entropy only)

- **Entropy >= 0.2**: No dampening applied (preserves 100% of complexity)
- **Entropy < 0.2**: Graduated dampening applied (preserves 50-100% of complexity)

#### Dampening Formula (Spec 68)

For very low entropy cases (< 0.2):

```
dampening_factor = 0.5 + 0.5 × (entropy / 0.2)
effective_complexity = raw_complexity × dampening_factor
```

This ensures:
- **Maximum reduction**: 50% (when entropy = 0.0)
- **Minimum reduction**: 0% (when entropy = 0.2)
- **Graduated scaling**: Linear interpolation between these extremes

#### Example: Very Low Entropy (Dampening Applies)

```
Raw Complexity: 20
Token Entropy: 0.1 (very low - highly repetitive)

dampening_factor = 0.5 + 0.5 × (0.1 / 0.2)
                 = 0.5 + 0.5 × 0.5
                 = 0.5 + 0.25
                 = 0.75

Effective Complexity = 20 × 0.75 = 15

Result: 25% reduction (preserves 75% of original complexity)
```

#### Example: Normal Entropy (No Dampening)

```
Raw Complexity: 20
Token Entropy: 0.5 (normal variety)

dampening_factor = 1.0 (no dampening for entropy >= 0.2)

Effective Complexity = 20 × 1.0 = 20

Result: 0% reduction (preserves 100% of original complexity)
```

## Real-World Examples

### Example 1: Validation Function

```rust
fn validate_config(config: &Config) -> Result<()> {
    if config.output_dir.is_none() {
        return Err(anyhow!("output_dir required"));
    }
    if config.max_workers.is_none() {
        return Err(anyhow!("max_workers required"));
    }
    if config.timeout_secs.is_none() {
        return Err(anyhow!("timeout_secs required"));
    }
    // ... 17 more similar checks
    Ok(())
}
```

**Traditional analysis:**
- Cyclomatic Complexity: 20
- Assessment: CRITICAL

**Entropy analysis:**
- Shannon Entropy: 0.3 (low variety)
- Pattern Repetition: 0.9 (highly repetitive)
- Branch Similarity: 0.95 (nearly identical)
- Effective Complexity: 5
- Assessment: LOW PRIORITY

### Example 2: State Machine Logic

```rust
fn reconcile_state(current: &State, desired: &State) -> Vec<Action> {
    let mut actions = vec![];

    match (current.mode, desired.mode) {
        (Mode::Active, Mode::Standby) => {
            if current.has_active_connections() {
                actions.push(Action::DrainConnections);
                actions.push(Action::WaitForDrain);
            }
            actions.push(Action::TransitionToStandby);
        }
        (Mode::Standby, Mode::Active) => {
            if desired.requires_warmup() {
                actions.push(Action::Warmup);
            }
            actions.push(Action::TransitionToActive);
        }
        // ... more diverse state transitions
        _ => {}
    }

    actions
}
```

**Traditional analysis:**
- Cyclomatic Complexity: 8
- Assessment: MODERATE

**Entropy analysis:**
- Shannon Entropy: 0.85 (high variety)
- Pattern Repetition: 0.2 (not repetitive)
- Branch Similarity: 0.3 (diverse branches)
- Effective Complexity: 9
- Assessment: HIGH PRIORITY

## Configuration

Configure entropy analysis in `.debtmap.toml`:

```toml
[entropy]
# Enable entropy analysis (default: true)
enabled = true

# Weight of entropy in complexity adjustment (0.0-1.0, default: 1.0)
weight = 1.0

# Minimum tokens required for entropy calculation (default: 20)
min_tokens = 20

# Pattern similarity threshold for detection (0.0-1.0, default: 0.7)
pattern_threshold = 0.7

# Enable advanced token classification (default: true)
use_classification = true

# Entropy level below which dampening is applied (default: 0.4)
entropy_threshold = 0.4

# Branch similarity above which dampening is applied (default: 0.8)
branch_threshold = 0.8

# Maximum reduction from repetition patterns (default: 0.20 = 20%)
max_repetition_reduction = 0.20

# Maximum reduction from entropy analysis (default: 0.15 = 15%)
max_entropy_reduction = 0.15

# Maximum reduction from branch similarity (default: 0.25 = 25%)
max_branch_reduction = 0.25

# Maximum combined complexity reduction percentage (default: 0.30 = 30%)
max_combined_reduction = 0.30
```

### Tuning for Your Project

**Strict mode (fewer false positive reductions):**
```toml
[entropy]
enabled = true
weight = 0.3
max_combined_reduction = 0.5
```

**Lenient mode (more aggressive false positive reduction):**
```toml
[entropy]
enabled = true
weight = 0.7
max_combined_reduction = 0.9
```

**Disable entropy analysis:**
```toml
[entropy]
enabled = false
```

Or via CLI (disables entropy-based complexity adjustments):
```bash
# Disables semantic analysis features including entropy dampening
debtmap analyze . --semantic-off
```

**Note**: The `--semantic-off` flag disables all semantic analysis features, including entropy-based complexity adjustments. This is useful when you want raw cyclomatic complexity without any dampening.

## Understanding the Impact

### Measuring False Positive Reduction

Run analysis with and without entropy:

```bash
# Without entropy
debtmap analyze . --semantic-off --top 20 > without_entropy.txt

# With entropy (default)
debtmap analyze . --top 20 > with_entropy.txt

# Compare
diff without_entropy.txt with_entropy.txt
```

**Expected results:**
- 60-75% reduction in flagged validation functions
- 40-50% reduction in flagged dispatcher functions
- 20-30% reduction in flagged configuration parsers
- No reduction in genuinely complex state machines or business logic

### Verifying Correctness

Entropy analysis should:
- **Reduce** flags on repetitive code (validators, dispatchers)
- **Preserve** flags on genuinely complex code (state machines, business logic)

If entropy analysis incorrectly reduces flags on genuinely complex code, adjust configuration:

```toml
[entropy]
weight = 0.3  # Reduce impact
max_combined_reduction = 0.5  # Limit maximum reduction
```

## Best Practices

1. **Use default settings** - They work well for most projects
2. **Verify results** - Spot-check top-priority items to ensure correctness
3. **Tune conservatively** - Start with default settings, adjust if needed
4. **Disable for debugging** - Use `--semantic-off` if entropy seems incorrect
5. **Report issues** - If entropy incorrectly flags code, report it

## Limitations

Entropy analysis works best for:
- Functions with cyclomatic complexity 10-50
- Code with clear repetitive patterns
- Validation, dispatch, and configuration functions

Entropy analysis is less effective for:
- Very simple functions (complexity < 5)
- Very complex functions (complexity > 100)
- Obfuscated or generated code

## Comparison with Other Approaches

| Approach | False Positive Rate | Complexity | Speed |
|----------|---------------------|------------|-------|
| Raw Cyclomatic Complexity | High (many false positives) | Low | Fast |
| Cognitive Complexity | Medium | Medium | Medium |
| Entropy Analysis (Debtmap) | Low | High | Fast |
| Manual Code Review | Very Low | Very High | Very Slow |

Debtmap's entropy analysis provides the best balance of accuracy and speed.

## See Also

- [Why Debtmap?](why-debtmap.md) - Real-world examples of entropy analysis
- [Analysis Guide](analysis-guide.md) - General analysis concepts
- [Configuration](configuration.md) - Complete configuration reference
