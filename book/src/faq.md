# Frequently Asked Questions

Common questions about debtmap's features, usage, and AI integration.

## AI Integration

### How does debtmap work with AI coding assistants?

Debtmap is designed as a **sensor** that provides structured data for AI consumption. Instead of telling you what to fix, it tells AI assistants:

1. **Where to look** - Prioritized list of debt items with file locations
2. **What to read** - Context suggestions (callers, callees, tests)
3. **What signals matter** - Complexity, coverage, coupling metrics

**Example workflow:**
```bash
# Pipe directly to Claude Code
debtmap analyze . --format llm-markdown --top 3 | claude "Fix the top item"
```

### What output format is best for AI?

Use `--format llm-markdown` for AI workflows. This format:

- Minimizes tokens while maximizing information
- Includes context suggestions inline
- Uses consistent structure for reliable parsing
- Avoids verbose descriptions that waste context window

```bash
debtmap analyze . --format llm-markdown --top 5
```

### Does debtmap provide fix suggestions?

No. Debtmap is a **sensor**, not a prescriber. It provides signals (metrics, coverage, coupling) and lets the AI decide how to fix issues.

This design is intentional:
- AI can consider business context you provide
- Different situations require different approaches
- Template recommendations are often wrong

### How do I use context suggestions?

Each debt item includes file ranges the AI should read:

```
Context:
├─ Primary: src/parser.rs:38-85 (the debt item)
├─ Caller: src/handler.rs:100-120 (usage context)
└─ Test: tests/parser_test.rs:50-75 (expected behavior)
```

Tell your AI to read these files before making changes:

```bash
debtmap analyze . --format llm-markdown --top 1 | \
  claude "Read the context files first, then fix the top item"
```

### Can I integrate debtmap with Cursor?

Yes. Generate a report file and reference it in Cursor:

```bash
# Generate report
debtmap analyze . --format llm-markdown --top 10 > debt-report.md

# In Cursor, use: @debt-report.md Fix the top critical item
```

## Features & Capabilities

### What's the difference between measured and estimated metrics?

**Measured Metrics** - Precise values from AST analysis:
- `cyclomatic_complexity`: Exact count of decision points
- `cognitive_complexity`: Weighted readability measure
- `nesting_depth`: Maximum nesting levels
- `loc`: Lines of code

**Estimated Metrics** - Heuristic approximations:
- `est_branches`: Estimated execution paths (formula-based)

Use measured metrics for thresholds and gates. Use estimated metrics for prioritization.

### What is entropy-based complexity analysis?

Entropy analysis uses information theory to distinguish between genuinely complex code and repetitive pattern-based code.

A function with 20 identical if/return validation checks has the same cyclomatic complexity as a function with 20 diverse conditional branches. Entropy analysis gives the validation function a much lower effective complexity score because it follows a simple, repetitive pattern.

**Result:** 60-75% reduction in false positives compared to traditional complexity metrics.

### What languages are supported?

**Currently supported:**
- Rust - Full support with AST parsing, macro expansion, and trait resolution

**Planned:**
- Python, JavaScript/TypeScript, Go (after Rust analysis is mature)

### Why is debtmap Rust-only right now?

We're taking a focused approach to deliver the best possible Rust code analyzer before expanding. This allows us to:

1. Perfect core algorithms with one language
2. Build Rust-specific features (macros, traits, lifetimes)
3. Establish trust in the Rust community
4. Apply learnings to future languages

### How does coverage integration work?

Debtmap reads LCOV format coverage data and maps it to functions:

```bash
# Generate coverage
cargo llvm-cov --lcov --output-path coverage.lcov

# Analyze with coverage
debtmap analyze . --lcov coverage.lcov
```

Coverage affects prioritization:
- Complex function with good coverage = lower priority
- Simple function with no coverage = higher priority
- High complexity + zero coverage = critical priority

## Usage & Configuration

### How do I exclude test files from analysis?

By default, debtmap excludes common test directories. To customize:

```toml
# .debtmap.toml
[analysis]
exclude_patterns = [
    "**/tests/**",
    "**/*_test.rs",
    "**/target/**",
]
```

### Can I customize the complexity thresholds?

Yes. Configure in `.debtmap.toml`:

```toml
[thresholds]
cyclomatic_complexity = 10
nesting_depth = 3
loc = 200

[tiers]
critical = 8.0
high = 5.0
moderate = 2.0
```

### Does debtmap integrate with CI/CD?

Yes. Use the `validate` command:

```bash
debtmap validate . --max-debt-density 10.0
```

**Exit codes:**
- `0` = validation passed
- `1` = validation failed (debt exceeds thresholds)
- `2` = analysis error

**GitHub Actions example:**
```yaml
- name: Check technical debt
  run: |
    cargo llvm-cov --lcov --output-path coverage.lcov
    debtmap validate . --lcov coverage.lcov --max-debt-density 10.0
```

### What if debtmap reports false positives?

1. **Verify entropy analysis is enabled** (default):
   ```toml
   [analysis]
   enable_entropy_analysis = true
   ```

2. **Adjust thresholds** for your project:
   ```toml
   [thresholds]
   cyclomatic_complexity = 15
   ```

3. **Use ignore comments** for specific functions:
   ```rust
   // debtmap:ignore - acceptable validation pattern
   fn validate_many_fields() { ... }
   ```

4. **Report issues** - If you believe analysis is incorrect, [open an issue](https://github.com/iepathos/debtmap/issues) with a code example.

### How accurate is the risk scoring?

Risk scores are **relative prioritization metrics**, not absolute measures. They help you answer "which code should I focus on first?" rather than "exactly how risky is this code?"

Use risk scores for prioritization, but apply your domain knowledge when deciding what to fix.

## Comparison with Other Tools

### How is debtmap different from SonarQube?

| Aspect | Debtmap | SonarQube |
|--------|---------|-----------|
| **Output** | Signals for AI | Recommendations |
| **Speed** | Seconds | Minutes |
| **Coverage** | Built-in | Enterprise only |
| **Entropy** | Yes | No |
| **Setup** | Single binary | Server required |

**When to use SonarQube:** Multi-language enterprise dashboards.
**When to use debtmap:** AI-assisted Rust development.

### Should I replace clippy with debtmap?

**No—use both.** They serve different purposes:

**clippy:**
- Rust idioms and patterns
- Common mistakes
- Runs in milliseconds

**debtmap:**
- Technical debt prioritization
- Coverage-based risk
- Context for AI

```bash
cargo clippy -- -D warnings
debtmap analyze . --lcov coverage.lcov --top 10
```

### How does debtmap compare to coverage tools?

Coverage tools (tarpaulin, llvm-cov) tell you what's tested. Debtmap tells you which untested code is most risky.

**Coverage tools:** "You have 75% coverage"
**Debtmap:** "Function X has 0% coverage and complexity 12—fix this first"

## Troubleshooting

### Analysis is slow on my large codebase

**Optimization strategies:**

1. **Exclude unnecessary files:**
   ```toml
   [analysis]
   exclude_patterns = ["**/target/**", "**/vendor/**"]
   ```

2. **Analyze specific directories:**
   ```bash
   debtmap analyze src/
   ```

3. **Reduce parallelism:**
   ```bash
   debtmap analyze . --jobs 4
   ```

### Coverage data isn't being applied

Check:
1. LCOV file path is correct
2. LCOV file contains data: `grep -c "^SF:" coverage.lcov`
3. Source paths match between LCOV and project

### Debtmap reports "No functions found"

Check:
1. Project contains Rust files (`.rs`)
2. Files aren't excluded by ignore patterns
3. No syntax errors: `debtmap analyze . -vv`

## Getting Help

- **Documentation:** [debtmap.dev](https://iepathos.github.io/debtmap/)
- **GitHub Issues:** [Report bugs](https://github.com/iepathos/debtmap/issues)
- **LLM Integration:** See [LLM Integration Guide](llm-integration.md)
- **Examples:** See [Examples](examples.md)
