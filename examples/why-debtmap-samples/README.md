# Debtmap Sample Code

This directory contains sample code used in the blog post "Why Debtmap? Rethinking Code Quality Analysis".

## Purpose

These samples demonstrate the difference between **cyclomatic complexity** and **actual cognitive complexity**, showing how debtmap's entropy-based analysis provides more accurate assessments.

## Sample Files

### `src/validation.rs` - High Cyclomatic, Low Cognitive Complexity

Contains `validate_config()` function with:
- **20 branches** (high cyclomatic complexity)
- **Low entropy** (0.33) - repetitive pattern
- **Low cognitive load** - easy to understand despite many branches
- Demonstrates debtmap's **complexity dampening** for repetitive code

### `src/state_reconciliation.rs` - Moderate Cyclomatic, High Cognitive Complexity

Contains `reconcile_state()` function with:
- **9 branches** (moderate cyclomatic complexity)
- **Higher entropy** (0.28) - diverse logic paths
- **High nesting depth** (4 levels)
- **High cognitive complexity** (16)
- Demonstrates complex interdependencies and state transitions

## Running the Comparison

### Install Required Tools

**Debtmap:**
```bash
cargo install debtmap
```

**Codacy CLI v2 (optional, for comparison):**
```bash
brew install codacy/codacy-cli-v2/codacy-cli-v2
```

### Run Analysis

**Debtmap:**
```bash
# Basic analysis
debtmap analyze .

# With top N recommendations
debtmap analyze . --top 10

# With test coverage (when available)
cargo tarpaulin --out lcov
debtmap analyze . --lcov lcov.info
```

**Codacy (for comparison):**
```bash
# Initialize and configure
codacy-cli init
codacy-cli config discover .
codacy-cli install

# Run analysis
codacy-cli analyze
```

### Compare the Results

Look at how Codacy (using Lizard) vs Debtmap prioritize the functions:

- **Codacy/Lizard**: Flags `validate_config()` (CCN=21) as WARNING, `reconcile_state()` (CCN=8) as OK
- **Debtmap**: Prioritizes `reconcile_state()` 3× higher due to cognitive complexity and nesting

See [COMPARISON.md](./COMPARISON.md) for detailed analysis.

## Expected Output

Debtmap should identify `reconcile_state()` as higher priority (4.15 vs 3.10) than `validate_config()`, despite the latter having higher cyclomatic complexity (21 vs 9), because:

1. **Pattern Detection**:
   - Detects "Coordinator pattern" in `reconcile_state()` with 4 actions and 2 state comparisons
   - Identifies "Repetitive validation pattern" in `validate_config()` with entropy 0.33

2. **Entropy Analysis**:
   - Dampens validation complexity from 21 → 10 (50% reduction)
   - Recognizes it as "boilerplate, not complexity"

3. **Cognitive Complexity**:
   - `reconcile_state()`: Cognitive=16 (high mental load from nested conditionals)
   - `validate_config()`: Cognitive=6 (easy to understand despite 21 branches)

4. **Architectural Guidance**:
   - Recommends "Extract state reconciliation logic into transition functions" (specific)
   - Suggests "Replace 20 repetitive validation checks with declarative pattern" (actionable)

5. **Impact Quantification**:
   - Shows exact complexity reduction: 9/16 → 4/11 for coordinator pattern
   - Quantifies risk: -1.5 for state transitions vs -1.0 for validation

## Blog Post

See the full analysis at: `/blog/why-debtmap/`
