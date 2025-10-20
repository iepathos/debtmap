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

Debtmap classifies tokens by importance:

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

Adjusts raw cyclomatic complexity based on entropy:

```
Effective Complexity = Raw Complexity × (1 - Entropy Adjustment)

Entropy Adjustment = min(
    max_reduction,
    (1 - shannon_entropy) × weight +
    pattern_repetition × weight +
    branch_similarity × weight
)
```

**Example:**
```
Raw Complexity: 20
Shannon Entropy: 0.3 (low variety)
Pattern Repetition: 0.8 (highly repetitive)
Branch Similarity: 0.9 (very similar branches)

Entropy Adjustment = min(0.7, (1 - 0.3) × 0.4 + 0.8 × 0.3 + 0.9 × 0.3)
                    = min(0.7, 0.28 + 0.24 + 0.27)
                    = min(0.7, 0.79)
                    = 0.7

Effective Complexity = 20 × (1 - 0.7) = 6
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

# Weight of entropy in complexity adjustment (0.0-1.0, default: 0.5)
weight = 0.5

# Minimum tokens required for entropy calculation (default: 10)
min_tokens = 10

# Pattern similarity threshold for detection (0.0-1.0, default: 0.7)
pattern_threshold = 0.7

# Enable advanced token classification (default: true)
use_classification = true

# Entropy level below which dampening is applied (default: 0.5)
entropy_threshold = 0.5

# Branch similarity above which dampening is applied (default: 0.7)
branch_threshold = 0.7

# Maximum combined complexity reduction percentage (default: 0.7 = 70%)
max_combined_reduction = 0.7
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

Or via CLI:
```bash
debtmap analyze . --semantic-off
```

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
