---
name: false-positive-check
description: Analyze debtmap's self-analysis for false positives and suggest improvements
---

# False Positive Analysis for Debtmap

Run debtmap's self-analysis and evaluate the results for false positives, then suggest improvements to prevent them across different codebases.

## Step 1: Run Self-Analysis

```bash
just analyze-self
```

## Step 2: Review Output

Examine the debtmap output for items that may be false positives:
- Intentional patterns in test fixtures
- Valid architectural decisions marked as debt
- Framework-specific patterns that are actually best practices
- Language idioms incorrectly flagged

## Step 3: Categorize False Positives

Group false positives by type:
1. **Test Fixture False Positives** - Intentional technical debt in test data
2. **Architecture Pattern False Positives** - Valid design patterns incorrectly flagged
3. **Language Idiom False Positives** - Standard language practices marked as debt
4. **Context-Specific False Positives** - Code that makes sense in its specific context

## Step 4: Apply Fixes

For test fixture false positives, use debtmap ignore syntax:

```rust
// debtmap:ignore-start -- Test fixture with intentional complexity
// Complex test setup code here
// debtmap:ignore-end
```

Or use configuration in `.debtmap.toml`:
```toml
[ignore]
patterns = ["test/fixtures/**", "*.test.rs", "*.spec.js"]
```

## Step 5: Suggest General Improvements

Based on false positives found, recommend:
1. Additional heuristics for debtmap to better identify intentional patterns
2. Context-aware analysis improvements
3. Language-specific rule adjustments
4. Configuration defaults that reduce false positives

## Step 6: Document Findings

Create a report with:
- List of false positives found
- Category of each false positive
- Suggested fix (ignore block, config change, or debtmap improvement)
- Priority for addressing each type

## Example Output Format

```markdown
# False Positive Analysis Report

## False Positives Found

### 1. Test Fixture Complexity
- **File**: `tests/fixtures/complex_data.rs`
- **Type**: Intentional test complexity
- **Solution**: Add debtmap:ignore-start block
- **General Fix**: Auto-detect test fixtures by path pattern

### 2. Builder Pattern
- **File**: `src/config/builder.rs`
- **Type**: Valid design pattern
- **Solution**: Reduce god object threshold for builders
- **General Fix**: Recognize builder pattern by method chaining

## Recommended Improvements

1. **Test Detection**: Improve test file detection heuristics
2. **Pattern Recognition**: Add common design pattern recognition
3. **Context Analysis**: Consider file location and naming conventions
```