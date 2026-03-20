# Introduction

Debtmap is a **code complexity sensor** for modern development workflows. It identifies technical debt hotspots and gives developers clear, prioritized insight directly in the terminal, TUI, dashboard, or machine-readable outputs.

## What is Debtmap?

Debtmap is different from traditional static analysis tools. Instead of burying you in warnings, it provides **signals** that developers and automation can use to make informed decisions:

1. **Where to look** - Prioritized list of debt items with exact file locations
2. **What to read** - Context suggestions (callers, callees, test files)
3. **What signals matter** - Multi-strategy analysis across complexity, coverage, coupling, git history, purity, and entropy

Good prioritization matters whether a developer is reviewing the results directly or an AI assistant is acting on them. Debtmap provides that guidance.

What makes Debtmap distinctive is that it does not rely on a single complexity score. It cross-checks multiple strategies:

- **Entropy-adjusted complexity** - Reduces false positives when repetitive boilerplate or patterned code inflates raw complexity
- **Coverage + complexity prioritization** - Treats untested complex code as materially riskier than equally complex code with strong coverage
- **Git history hotspot analysis** - Uses churn, bug-fix patterns, and author activity to identify code that keeps causing trouble
- **Coupling and dependency analysis** - Estimates impact radius by looking at fan-in, fan-out, and call-graph relationships
- **Purity analysis** - Distinguishes testable, low-side-effect code from code with heavier mutation and I/O risk

The result is a ranked view of technical debt that is designed to surface genuinely risky hotspots instead of overwhelming developers with generic complexity warnings.

## The AI Sensor Model

Debtmap is a **sensor**, not a prescriber. It measures and reports; it doesn't tell you what to do.

**What Debtmap provides:**
- Quantified complexity signals (cyclomatic, cognitive, nesting)
- Entropy-adjusted scoring to reduce false positives
- Test coverage gaps combined with complexity for cross-prioritization
- Git history context to surface churn and bug hotspots
- Coupling and dependency signals to estimate impact radius
- Purity analysis to reflect side effects and testability
- Direct, browsable results for developers in terminal and TUI workflows
- Context suggestions for AI consumption
- Structured output (JSON, LLM-markdown) for machine consumption

**What Debtmap doesn't provide:**
- "Fix this by doing X" recommendations
- "You should consider Y" advice
- Template-based refactoring suggestions

This design is intentional. Developers and AI assistants can consider business context, team preferences, and constraints that Debtmap can't know. Debtmap tells you where to look and why it matters.

## Quick Start

```bash
# Install
cargo install debtmap

# Analyze directly in the terminal/TUI
debtmap analyze .

# Get structured signals for dashboards and automation
debtmap analyze . --format json --top 10 --output debt.json

# Optional: pipe the top item to Claude Code
debtmap analyze . --format markdown --top 3 | claude "Fix the top item"

# With coverage data for accurate risk assessment
cargo llvm-cov --lcov --output-path coverage.lcov
debtmap analyze . --lcov coverage.lcov --format markdown
```

## Key Features

### Signal Generation
- **Complexity signals** - Cyclomatic, cognitive, nesting depth, lines of code
- **Coverage signals** - Line coverage, branch coverage, function coverage, combined with complexity for risk ranking
- **Git context signals** - Churn, bug-fix frequency, and author/change history
- **Coupling signals** - Fan-in, fan-out, call graph depth, and impact radius
- **Quality signals** - Entropy (false positive reduction) and purity (testability and side-effect profile)

For a complete list of metrics and their formulas, see the [Metrics Reference](./metrics-reference.md).

### AI-Optimized Output
- **LLM markdown format** - Minimal tokens, maximum information
- **Context suggestions** - File ranges the AI should read
- **Structured JSON** - Stable schema for programmatic access
- **Deterministic output** - Same input = same output

### Analysis Capabilities
- **Rust, Python, and JS/TS analysis** - Native Rust AST parsing plus tree-sitter analysis for Python, JavaScript, and TypeScript
- **Multi-strategy prioritization** - Cross-ranks debt using complexity, coverage, git history, coupling, purity, and entropy
- **Coverage integration** - Native LCOV support for risk assessment, especially for complex code
- **Debt pattern detection** - God objects, boilerplate code, error handling anti-patterns
- **Entropy analysis** - Reduces false positives from repetitive or formulaic code
- **Git and dependency context** - Highlights bug hotspots and likely impact radius before refactoring
- **Parallel processing** - Fast analysis (10-100x faster than Java/Python tools)

### Workflow Integration
- **Developer-first exploration** - Review prioritized debt directly in terminal, TUI, and dashboard flows
- **Direct piping** - Pipe output to Claude, Cursor, or custom agents
- **CI/CD gates** - Validate debt thresholds with the `validate` command
- **Progress tracking** - Compare debt across commits with `compare` and `validate-improvement` commands

## Current Status

Debtmap currently supports Rust, Python, JavaScript, and TypeScript analysis. This lets debtmap:

- Build deep Rust-specific analysis (macros, traits, lifetimes)
- Analyze Python projects with language-aware parsing and complexity detection
- Analyze JavaScript and TypeScript projects with tree-sitter-based parsing and frontend/backend-aware patterns
- Keep the core AI workflow focused on high-signal, coverage-aware prioritization

Additional language expansion remains possible in future releases, but `0.16.0` ships with Rust, Python, JavaScript, and TypeScript as the documented supported set.

## Target Audience

Debtmap is designed for:

- **Individual developers** - Inspect complexity hotspots and debt rankings directly while coding
- **AI-assisted developers** - Feed the same signals into coding assistants when useful
- **Development teams** - Prioritize debt remediation with quantified metrics
- **CI/CD engineers** - Enforce quality gates with coverage-aware thresholds
- **Legacy codebase maintainers** - Identify where AI can help most

## Getting Started

Ready to start? Check out:
- [Getting Started](./getting-started.md) - Installation and first analysis
- [LLM Integration](./llm-integration.md) - AI workflow patterns
- [Why Debtmap?](./why-debtmap.md) - Positioning, tradeoffs, and workflow model
- [TUI Guide](./tui-guide.md) - Interactive exploration with the terminal UI

**Quick tip:** Start with `debtmap analyze . --format markdown --top 5` to see the top priority items with context suggestions.
