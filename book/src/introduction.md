# Introduction

Debtmap is a **code complexity sensor** for AI-assisted development. It identifies technical debt hotspots and provides the structured data AI coding tools need to understand and fix them.

## What is Debtmap?

Debtmap is different from traditional static analysis tools. Instead of telling you what to fix, it provides **signals** that AI assistants can use to make informed decisions:

1. **Where to look** - Prioritized list of debt items with exact file locations
2. **What to read** - Context suggestions (callers, callees, test files)
3. **What signals matter** - Complexity, coverage, coupling metrics

The key insight: AI coding assistants are great at fixing code, but they need guidance on *where* to focus and *what* to read. Debtmap provides that guidance.

## The AI Sensor Model

Debtmap is a **sensor**, not a prescriber. It measures and reports; it doesn't tell you what to do.

**What Debtmap provides:**
- Quantified complexity signals (cyclomatic, cognitive, nesting)
- Test coverage gaps with risk prioritization
- Context suggestions for AI consumption
- Structured output (JSON, LLM-markdown) for machine consumption

**What Debtmap doesn't provide:**
- "Fix this by doing X" recommendations
- "You should consider Y" advice
- Template-based refactoring suggestions

This design is intentional. AI assistants can consider business context, team preferences, and constraints that Debtmap can't know. The AI decides what to do; Debtmap tells it where to look.

## Quick Start

```bash
# Install
cargo install debtmap

# Analyze and pipe to Claude Code
debtmap analyze . --format llm-markdown --top 3 | claude "Fix the top item"

# Get structured signals for your AI workflow
debtmap analyze . --format json --top 10 > debt.json

# With coverage data for accurate risk assessment
cargo llvm-cov --lcov --output-path coverage.lcov
debtmap analyze . --lcov coverage.lcov --format llm-markdown
```

## Key Features

### Signal Generation
- **Complexity signals** - Cyclomatic, cognitive, nesting depth, lines of code
- **Coverage signals** - Line coverage, branch coverage, function coverage
- **Coupling signals** - Fan-in, fan-out, call graph depth
- **Quality signals** - Entropy (false positive reduction), purity (testability)

### AI-Optimized Output
- **LLM markdown format** - Minimal tokens, maximum information
- **Context suggestions** - File ranges the AI should read
- **Structured JSON** - Stable schema for programmatic access
- **Deterministic output** - Same input = same output

### Analysis Capabilities
- **Rust-first analysis** - Full AST parsing, macro expansion, trait resolution
- **Coverage integration** - Native LCOV support for risk assessment
- **Entropy analysis** - Reduces false positives from repetitive code
- **Parallel processing** - Fast analysis (10-100x faster than Java/Python tools)

### Workflow Integration
- **Direct piping** - Pipe output to Claude, Cursor, or custom agents
- **CI/CD gates** - Validate debt thresholds in pipelines
- **Progress tracking** - Compare debt across commits

## Current Status

Debtmap focuses exclusively on Rust. This focused approach allows us to:

- Build deep Rust-specific analysis (macros, traits, lifetimes)
- Perfect core algorithms before expanding
- Deliver the best possible AI sensor for Rust

Multi-language support (Python, JavaScript/TypeScript, Go) is planned for future releases.

## Target Audience

Debtmap is designed for:

- **AI-assisted developers** - Get signals that help AI assistants make better decisions
- **Development teams** - Prioritize debt remediation with quantified metrics
- **CI/CD engineers** - Enforce quality gates with coverage-aware thresholds
- **Legacy codebase maintainers** - Identify where AI can help most

## Getting Started

Ready to start? Check out:
- [Getting Started](./getting-started.md) - Installation and first analysis
- [LLM Integration](./llm-integration.md) - AI workflow patterns
- [Why Debtmap?](./why-debtmap.md) - The AI sensor model explained

**Quick tip:** Start with `debtmap analyze . --format llm-markdown --top 5` to see the top priority items with context suggestions.
