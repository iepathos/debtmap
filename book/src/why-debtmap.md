# Why Debtmap?

Debtmap is a **code complexity sensor** designed for AI-assisted development workflows. It identifies technical debt hotspots and provides the structured data AI coding tools need to understand and fix them.

## The AI Development Paradox

AI coding assistants like Claude Code, GitHub Copilot, and Cursor are transforming software development. They can write code faster than ever before. But this creates a paradox:

**AI creates technical debt faster than humans can manage it.**

When an AI generates hundreds of lines of code per hour, traditional code review and refactoring processes break down. Teams accumulate debt faster than they can pay it down.

At the same time, AI assistants struggle to fix the debt they create:

- **Limited context window** - They can't see the entire codebase at once
- **No test awareness** - They don't know which code is tested vs untested
- **No prioritization** - They can't identify what matters most
- **Wasted tokens** - They read irrelevant code while missing critical context

## What AI Coding Tools Need

For an AI assistant to effectively fix technical debt, it needs:

### 1. Prioritized Targets

Not "here are 500 complex functions," but "here are the 10 functions that matter most, ranked by severity."

Debtmap provides a severity score (0-10) that combines:
- Complexity metrics (cyclomatic, cognitive, nesting)
- Test coverage gaps
- Coupling and dependency impact
- Pattern-based false positive reduction

### 2. Context Suggestions

Not "this function is complex," but "read lines 38-85 of parser.rs, plus lines 100-120 of handler.rs where it's called, and lines 50-75 of the test file."

Debtmap's context suggestions tell the AI exactly which code to read:

```
CONTEXT:
├─ Primary: src/parser.rs:38-85 (the debt item)
├─ Caller: src/handler.rs:100-120 (usage context)
└─ Tests: tests/parser_test.rs:50-75 (expected behavior)
```

### 3. Quantified Signals

Not "this code is bad," but "cyclomatic complexity: 12, cognitive complexity: 18, test coverage: 0%, called by 8 functions."

These signals let the AI make informed decisions about the best approach:
- High complexity + good coverage = risky to refactor
- Low complexity + no coverage = easy test target
- High coupling + high complexity = incremental approach needed

### 4. Structured Output

Not free-form text, but JSON and markdown optimized for LLM consumption:
- Consistent structure across all debt items
- Minimal tokens for maximum information
- Deterministic output for reproducible workflows
- Stable IDs for referencing items across runs

## What Debtmap Provides

### Complexity Signals

| Signal | What It Measures | Why It Matters |
|--------|------------------|----------------|
| Cyclomatic | Decision points (if, match, loop) | Number of execution paths |
| Cognitive | Readability difficulty | How hard code is to understand |
| Nesting | Indentation depth | Compound complexity |
| Lines | Function length | Scope of changes needed |

### Coverage Signals

| Signal | What It Measures | Why It Matters |
|--------|------------------|----------------|
| Line coverage | % of lines executed by tests | Basic test coverage |
| Branch coverage | % of branches taken | Edge case coverage |
| Function coverage | Whether function is tested at all | Critical gap detection |

### Coupling Signals

| Signal | What It Measures | Why It Matters |
|--------|------------------|----------------|
| Fan-in | Functions that call this function | Impact of changes |
| Fan-out | Functions this function calls | Dependency risk |
| Call depth | Distance from entry points | Integration complexity |

### Quality Signals

| Signal | What It Measures | Why It Matters |
|--------|------------------|----------------|
| Entropy | Pattern variety in code | False positive filtering |
| Purity | Side effect presence | Testability indicator |
| Dead code | Unused functions | Cleanup candidates |

## What Debtmap Doesn't Do

Debtmap is a **sensor**, not a prescriber. It measures and reports; it doesn't tell you what to do.

### No Fix Suggestions

Debtmap doesn't say "split this function into 5 modules" or "add 8 unit tests." Those decisions require understanding the business context, architectural constraints, and team preferences that only humans (or AI with proper context) can evaluate.

### No "Should" Statements

Debtmap doesn't say "you should refactor this" or "consider extracting a helper function." It reports facts: "complexity: 18, coverage: 0%, called by 12 functions." The AI or developer decides what to do with that information.

### No Impact Predictions

Debtmap doesn't claim "refactoring this will reduce bugs by 40%." Such predictions are speculative. Debtmap reports what it can measure accurately and leaves interpretation to the consumer.

## Comparison with Alternatives

### vs Static Analysis Tools (SonarQube, CodeClimate)

| Aspect | Traditional Tools | Debtmap |
|--------|-------------------|---------|
| Output | Recommendations | Signals |
| Audience | Humans | AI + Humans |
| Format | Dashboards | JSON/Markdown |
| Speed | Minutes | Seconds |
| Focus | "Fix this" | "Here's what exists" |

Traditional tools are designed for human code review workflows. Debtmap is designed for AI-assisted development.

### vs Linters (Clippy, ESLint)

| Aspect | Linters | Debtmap |
|--------|---------|---------|
| Focus | Style/idioms | Complexity/debt |
| Scope | Line-level | Function-level |
| Output | Warnings | Prioritized signals |
| Coverage | Not integrated | Core feature |

Linters catch code style issues. Debtmap identifies complexity hotspots. Use both.

### vs Coverage Tools (Tarpaulin, pytest-cov)

| Aspect | Coverage Tools | Debtmap |
|--------|----------------|---------|
| Output | Coverage % | Risk-prioritized gaps |
| Complexity | Not considered | Core metric |
| Context | None | File ranges for AI |

Coverage tools tell you what's tested. Debtmap tells you what untested code is most risky.

## How Debtmap Fits in Your Workflow

### AI-Assisted Development

```bash
# Generate debt signals
debtmap analyze . --format llm-markdown --lcov coverage.lcov

# Pipe to AI
cat debt.md | claude "Fix the top item, read the suggested context first"
```

### CI/CD Integration

```bash
# Fail build if debt exceeds thresholds
debtmap validate . --max-debt-density 10.0

# Generate report for PR review
debtmap analyze . --format json --output debt-report.json
```

### Exploratory Analysis

```bash
# Quick overview
debtmap analyze . --top 10

# Deep dive with coverage
debtmap analyze . --lcov coverage.lcov --format terminal -vv
```

## Key Insights

1. **Debtmap is a sensor** - It measures, it doesn't prescribe
2. **AI does the thinking** - Debtmap provides data, AI decides action
3. **Context is key** - Knowing what to read is as valuable as what to fix
4. **Signals over interpretations** - Raw metrics, not template advice
5. **Speed matters** - Fast enough for local development loops

## Next Steps

Ready to try it? Head to [Getting Started](getting-started.md) to install debtmap and run your first analysis.

Want to integrate with your AI workflow? See [LLM Integration](llm-integration.md) for detailed guidance.

Want to understand how it works under the hood? See [Architecture](architecture.md) for the analysis pipeline.
