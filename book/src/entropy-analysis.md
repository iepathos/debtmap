# Entropy Analysis

Entropy analysis is Debtmap's unique approach to distinguishing genuinely complex code from repetitive pattern-based code. This reduces false positives by 60-75% compared to traditional cyclomatic complexity metrics.

## Overview

Traditional static analysis tools flag code as "complex" based purely on cyclomatic complexity or lines of code. However, not all complexity is equal:

- **Repetitive patterns** (validation functions, dispatchers) have high cyclomatic complexity but low cognitive load
- **Diverse logic** (state machines, business rules) may have moderate cyclomatic complexity but high cognitive load

Entropy analysis uses information theory to distinguish between these cases.

## How It Works

Debtmap's entropy analysis is **language-agnostic**, working across Rust, Python, JavaScript, and TypeScript codebases using a universal token classification approach. This ensures consistent complexity assessment regardless of the programming language used.

### Language-Agnostic Analysis

The same entropy concepts apply consistently across all supported languages. Here's how a validation function would be analyzed in different languages:

**Rust:**
```rust
fn validate_config(config: &Config) -> Result<()> {
    if config.output_dir.is_none() { return Err(anyhow!("output_dir required")); }
    if config.max_workers.is_none() { return Err(anyhow!("max_workers required")); }
    if config.timeout_secs.is_none() { return Err(anyhow!("timeout_secs required")); }
    Ok(())
}
// Entropy: ~0.3, Pattern Repetition: 0.9, Effective Complexity: ~5
```

**Python:**
```python
def validate_config(config: Config) -> None:
    if config.output_dir is None: raise ValueError("output_dir required")
    if config.max_workers is None: raise ValueError("max_workers required")
    if config.timeout_secs is None: raise ValueError("timeout_secs required")
# Entropy: ~0.3, Pattern Repetition: 0.9, Effective Complexity: ~5
```

**JavaScript/TypeScript:**
```typescript
function validateConfig(config: Config): void {
    if (!config.outputDir) throw new Error("outputDir required");
    if (!config.maxWorkers) throw new Error("maxWorkers required");
    if (!config.timeoutSecs) throw new Error("timeoutSecs required");
}
// Entropy: ~0.3, Pattern Repetition: 0.9, Effective Complexity: ~5
```

All three receive similar entropy scores because they share the same repetitive validation pattern, demonstrating how Debtmap's analysis transcends language syntax to identify underlying code structure patterns.

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

**When enabled** (`use_classification = false` by default for backward compatibility), tokens are weighted by importance:

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

**How token classification affects Shannon entropy** (`entropy.rs:134-142`, `entropy_core.rs:166-198`):

When `use_classification` is enabled, the Shannon entropy calculation uses weighted token frequencies instead of equal weights. The algorithm:
1. Extracts tokens and classifies them by importance (control flow × 1.0, operators × 0.7, identifiers × 0.3)
2. Calculates weighted frequency: `frequency_map[category] += token.weight()`
3. Computes probability: `probability = weighted_frequency / total_weight`
4. Applies Shannon formula: `entropy = -Σ probability × log₂(probability)`

This gives more importance to structural complexity (control flow patterns) over identifier variety. For example, a function with many different variable names but simple control flow will have lower entropy than a function with complex branching logic, which better reflects cognitive complexity.

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

Debtmap uses a multi-factor dampening approach that analyzes three dimensions of code repetitiveness:

1. **Pattern Repetition** - Detects repetitive AST structures
2. **Token Entropy** - Measures variety in token usage
3. **Branch Similarity** - Compares similarity between conditional branches

These factors are combined multiplicatively with a minimum floor of 0.7 (preserving at least 70% of original complexity):

```
dampening_factor = (repetition_factor × entropy_factor × branch_factor).max(0.7)
effective_complexity = raw_complexity × dampening_factor
```

#### Historical Note: Spec 68

**Spec 68: Graduated Entropy Dampening** was the original simple algorithm that only considered entropy < 0.2:

```
dampening_factor = 0.5 + 0.5 × (entropy / 0.2)  [when entropy < 0.2]
```

The current implementation uses a more sophisticated **graduated dampening** approach that considers all three factors (repetition, entropy, branch similarity) with separate thresholds and ranges for each. The test suite references Spec 68 to verify backward compatibility with the original behavior.

#### When Dampening Applies

Dampening is applied based on multiple thresholds:

- **Pattern Repetition**: Values approaching 1.0 trigger dampening (high repetition detected)
- **Token Entropy**: Values below 0.4 trigger graduated dampening (low variety)
- **Branch Similarity**: Values above 0.8 trigger dampening (similar branches)

#### Graduated Dampening Formula

Each factor is dampened individually using a graduated calculation:

```rust
// Conceptual pseudocode showing the three-factor approach
// Actual implementation in src/complexity/entropy.rs:185-195 and :429-439
fn calculate_dampening_factor(
    repetition: f64,     // 0.0-1.0
    entropy: f64,        // 0.0-1.0
    branch_similarity: f64  // 0.0-1.0
) -> f64 {
    // Each factor uses calculate_graduated_dampening with its own threshold/range
    let repetition_factor = graduated_dampening(repetition, threshold=1.0, max_reduction=0.20);
    let entropy_factor = graduated_dampening(entropy, threshold=0.4, max_reduction=0.15);
    let branch_factor = graduated_dampening(branch_similarity, threshold=0.8, max_reduction=0.25);

    (repetition_factor * entropy_factor * branch_factor).max(0.7)  // Never reduce below 70%
}
```

**Key Parameters:**
- **Repetition**: Threshold 1.0, max 20% reduction (configurable via `max_repetition_reduction`)
- **Entropy**: Threshold 0.4 (hardcoded), max 15% reduction (configurable via `max_entropy_reduction`)
- **Branch Similarity**: Threshold 0.8 (configurable via `branch_threshold`), max 25% reduction (configurable via `max_branch_reduction`)
- **Combined Floor**: Minimum 70% of original complexity preserved (configurable via `max_combined_reduction`)

#### Example: Repetitive Validation Function

```
Raw Complexity: 20
Pattern Repetition: 0.95 (very high)
Token Entropy: 0.3 (low variety)
Branch Similarity: 0.9 (very similar branches)

repetition_factor ≈ 0.85 (15% reduction)
entropy_factor ≈ 0.90 (10% reduction)
branch_factor ≈ 0.80 (20% reduction)

dampening_factor = (0.85 × 0.90 × 0.80) = 0.612
dampening_factor = max(0.612, 0.7) = 0.7  // Floor applied

Effective Complexity = 20 × 0.7 = 14

Result: 30% reduction (maximum allowed)
```

#### Example: Diverse State Machine

```
Raw Complexity: 20
Pattern Repetition: 0.2 (low - not repetitive)
Token Entropy: 0.8 (high variety)
Branch Similarity: 0.3 (diverse branches)

repetition_factor ≈ 1.0 (no reduction)
entropy_factor ≈ 1.0 (no reduction)
branch_factor ≈ 1.0 (no reduction)

dampening_factor = (1.0 × 1.0 × 1.0) = 1.0

Effective Complexity = 20 × 1.0 = 20

Result: 0% reduction (complexity preserved)
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

Configure entropy analysis in `.debtmap.toml` or disable via the `--semantic-off` CLI flag.

```toml
[entropy]
# Enable entropy analysis (default: true)
enabled = true

# Weight of entropy in overall complexity scoring (0.0-1.0, default: 1.0)
# Note: This affects scoring, not dampening thresholds
weight = 1.0

# Minimum tokens required for entropy calculation (default: 20)
min_tokens = 20

# Pattern similarity threshold for repetition detection (0.0-1.0, default: 0.7)
pattern_threshold = 0.7

# Entropy threshold for low entropy detection (0.0-1.0, default: 0.4)
# Used for detecting low entropy patterns in code
entropy_threshold = 0.4

# Enable advanced token classification (default: false for backward compatibility)
# When true, weights tokens by semantic importance (control flow > operators > identifiers)
use_classification = false

# Branch similarity threshold (0.0-1.0, default: 0.8)
# Branches with similarity above this threshold contribute to dampening
branch_threshold = 0.8

# Maximum reduction limits (these are configurable)
max_repetition_reduction = 0.20  # Max 20% reduction from pattern repetition
max_entropy_reduction = 0.15     # Max 15% reduction from low token entropy
max_branch_reduction = 0.25      # Max 25% reduction from branch similarity
max_combined_reduction = 0.30    # Overall cap at 30% reduction (minimum 70% preserved)
```

**Important Notes:**

1. **Dampening thresholds** - Some are configurable, some are hardcoded (`src/complexity/entropy.rs:185-195`):
   - **Entropy dampening threshold: 0.4** - Hardcoded in the graduated_dampening call (`entropy.rs:192`). This threshold controls when entropy-based dampening is applied during the dampening factor calculation.
   - **Configurable `entropy_threshold` field: 0.4** - Separate config field (`config/languages.rs:82-84`) used for low entropy detection during analysis. This serves a different purpose than the hardcoded dampening threshold.
   - **Branch threshold: 0.8** - Configurable via `branch_threshold` in config file
   - **Pattern threshold: 0.7/1.0** - Configurable via `pattern_threshold` in config file

2. **The `weight` parameter** controls how entropy-adjusted complexity contributes to the final priority score calculation in the scoring pipeline (`priority/scoring/computation.rs`). A weight of 1.0 means full contribution, 0.5 means half contribution. This parameter does not affect the dampening factor calculation itself—it only influences how the entropy-adjusted complexity is weighted when computing the final priority score.

3. **Token classification** defaults to `false` (disabled) for backward compatibility, even though it provides more accurate entropy analysis when enabled.

### Tuning for Your Project

**Enable token classification for better accuracy:**
```toml
[entropy]
enabled = true
use_classification = true  # Weight control flow keywords more heavily
```

**Strict mode (fewer reductions, flag more code):**
```toml
[entropy]
enabled = true
max_repetition_reduction = 0.10  # Reduce from default 0.20
max_entropy_reduction = 0.08     # Reduce from default 0.15
max_branch_reduction = 0.12      # Reduce from default 0.25
max_combined_reduction = 0.20    # Reduce from default 0.30 (preserve 80%)
```

**Lenient mode (more aggressive reduction):**
```toml
[entropy]
enabled = true
max_repetition_reduction = 0.30  # Increase from default 0.20
max_entropy_reduction = 0.25     # Increase from default 0.15
max_branch_reduction = 0.35      # Increase from default 0.25
max_combined_reduction = 0.50    # Increase from default 0.30 (preserve 50%)
```

**Disable entropy dampening entirely:**
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

## Interpreting Entropy-Adjusted Output

When entropy analysis detects repetitive patterns, debtmap displays both the original and adjusted complexity values to help you understand the adjustment. This transparency allows you to verify the analysis and understand why certain code receives lower priority.

### Output Format

When viewing detailed output (verbosity level 2 with `-vv`), entropy-adjusted complexity is shown in the **COMPLEXITY** section:

```
COMPLEXITY: cyclomatic=20 (dampened: 14, factor: 0.70), est_branches=40, cognitive=25, nesting=3, entropy=0.30
```

And in the **Entropy Impact** scoring section:

```
  - Entropy Impact: 30% dampening (entropy: 0.30, repetition: 95%)
```

### Understanding the Values

**cyclomatic=20**: Original cyclomatic complexity before adjustment
**dampened: 14**: Adjusted complexity after entropy analysis (20 × 0.70 = 14)
**factor: 0.70**: The dampening factor applied (0.70 = 30% reduction)
**entropy=0.30**: Shannon entropy score (0.0-1.0, lower = more repetitive)
**repetition: 95%**: Pattern repetition score (higher = more repetitive)

### Reconstructing the Calculation

You can verify the adjustment by multiplying:
```
original_complexity × dampening_factor = adjusted_complexity
20 × 0.70 = 14
```

The dampening percentage shown in the Entropy Impact section is:
```
dampening_percentage = (1.0 - dampening_factor) × 100%
(1.0 - 0.70) × 100% = 30%
```

### When Entropy Data is Unavailable

If a function is too small for entropy analysis (< 20 tokens) or entropy is disabled, the output shows complexity without dampening:

```
COMPLEXITY: cyclomatic=5, est_branches=10, cognitive=8, nesting=2
```

No "dampened" or "factor" values are shown, indicating the raw complexity is used for scoring.

### Example Output Comparison

**Before entropy-adjustment:**
```
#1 SCORE: 95.5 [CRITICAL]
├─ COMPLEXITY: cyclomatic=20, est_branches=40, cognitive=25, nesting=3
```

**After entropy-adjustment:**
```
#15 SCORE: 68.2 [HIGH]
├─ COMPLEXITY: cyclomatic=20 (dampened: 14, factor: 0.70), est_branches=40, cognitive=25, nesting=3, entropy=0.30
  - Entropy Impact: 30% dampening (entropy: 0.30, repetition: 95%)
```

The item dropped from rank #1 to #15 because entropy analysis detected the high complexity was primarily due to repetitive validation patterns rather than genuine cognitive complexity.

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
max_combined_reduction = 0.20  # Reduce from default 0.30 (preserve 80%)
max_repetition_reduction = 0.10  # Reduce individual factors
max_entropy_reduction = 0.08
max_branch_reduction = 0.12
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
