# Overview

Debtmap analyzes code through multiple lenses to provide a comprehensive view of technical health. The goal is to move beyond simple problem identification to **evidence-based prioritization** - showing what to fix first based on risk scores, test coverage gaps, and ROI calculations, with actionable recommendations backed by impact metrics.

## Supported Languages

- **Rust** - Full analysis support with AST-based parsing via `syn`
- **Python** - Partial support for basic metrics

Source: `src/organization/language.rs:4-7`

## Analysis Capabilities

### Complexity Metrics

Calculates multiple dimensions of code complexity:

- **Cyclomatic Complexity** - Measures linearly independent paths through code (control flow branching)
- **Cognitive Complexity** - Quantifies human comprehension difficulty beyond raw paths
- **Nesting Depth** - Tracks maximum indentation levels
- **Function Length** - Lines of code per function
- **Parameter Count** - Number of function parameters
- **Entropy Score** - Pattern-based complexity adjustment that reduces false positives by up to 70%
- **Purity Level** - Functional purity classification (StrictlyPure, LocallyPure, ReadOnly, Impure)

Source: `src/core/mod.rs:62-92` (FunctionMetrics struct)

See [Complexity Metrics](complexity-metrics.md) for detailed explanations and examples.

### Debt Patterns

Identifies **25+ types of technical debt** across 4 major categories:

**Testing Issues** (6 types):
- Testing gaps in complex code
- Complex test code requiring refactoring
- Test duplication and flaky patterns
- Over-complex assertions

**Architectural Problems** (7 types):
- God objects and god modules
- Feature envy and primitive obsession
- Scattered type implementations
- Orphaned functions and utilities sprawl

**Performance Issues** (8 types):
- Async/await misuse and blocking I/O
- Collection inefficiencies and nested loops
- Memory allocation problems
- Suboptimal data structures

**Code Quality Issues** (6 types):
- Complexity hotspots without test coverage
- Dead code and duplication
- Error swallowing and magic values

Source: `src/priority/mod.rs:158-288` (DebtType enum)

See [Debt Patterns](debt-patterns.md) for detailed detection rules and examples.

### Risk Scoring

Combines complexity, test coverage, coupling, and change frequency through a **multi-factor risk model**:

**Risk Categories**:
- **Critical** - High complexity (>15) + low coverage (<30%)
- **High** - High complexity (>10) + moderate coverage (<60%)
- **Medium** - Moderate complexity (>5) + low coverage (<50%)
- **Low** - Low complexity or high coverage
- **WellTested** - High complexity with high coverage (good examples to learn from)

**Coverage Penalty Calculation**:
- Untested code receives a **2.0x multiplier**
- Partially tested code receives a **1.5x multiplier**
- Coverage gaps are penalized exponentially

**Risk Score Weights** (configurable):
```rust
coverage:          0.5   // Coverage weight
complexity:        0.3   // Cyclomatic weight
cognitive:         0.45  // Cognitive weight
debt:              0.2   // Debt factor weight
untested_penalty:  2.0   // Multiplier for untested code
```

Source: `src/risk/mod.rs:36-42`, `src/risk/strategy.rs:8-28`

See [Risk Scoring](risk-scoring.md) for detailed scoring algorithms.

### Prioritization

Uses a **multi-stage pipeline** to assign priority tiers and estimate test writing impact:

1. **Evidence Collection** - Gather complexity, coverage, and coupling metrics
2. **Context Enrichment** - Add architectural context and change frequency
3. **Baseline Scoring** - Calculate initial risk scores using multi-factor model
4. **ROI Calculation** - Estimate return on investment for test writing
5. **Final Priority** - Assign priority tiers with risk reduction impact estimates

**Priority Tiers**:
- P0 (Critical) - Immediate action required
- P1 (High) - Address in current sprint
- P2 (Medium) - Plan for next cycle
- P3 (Low) - Monitor and review

Source: Features documented in `.prodigy/book-analysis/features.json:risk_assessment.prioritization`

See [Interpreting Results](interpreting-results.md) for guidance on using priority rankings.

## How It Works

Debtmap uses a **functional, multi-layered architecture** for accurate and performant analysis:

### Three-Phase Analysis Pipeline

**Phase 1: Parallel Parsing**
- Language-specific AST generation using tree-sitter (Rust via `syn`)
- Pure functional transformation: source code → AST
- Files parsed once, ASTs cached and cloned for reuse (44% faster)
- Runs in parallel using Rayon for CPU-intensive parsing

**Phase 2: Parallel Analysis with Batching**
- Data flow graph construction (O(1) lookups via multi-index)
- Purity analysis tracking pure vs impure functions
- Pattern detection with entropy analysis
- Metrics computation through pure functions
- Default batch size: 100 items (configurable via `--batch-size`)

**Phase 3: Sequential Aggregation**
- Combine parallel results into unified analysis
- Apply multi-dimensional scoring (complexity + coverage + coupling)
- Priority ranking and tier classification
- Generate actionable recommendations with impact metrics

Source: `ARCHITECTURE.md`, `src/builders/parallel_unified_analysis.rs:21-87`

### Key Architectural Patterns

**Functional Core, Imperative Shell**:
- Pure functions for all metric calculations
- I/O isolated to file reading and output formatting
- Enables easy testing and parallelization

**Multi-Index Call Graph** (O(1) lookups):
- Primary index: exact function ID lookup
- Fuzzy index: name + file matching for generics
- Name index: cross-file function resolution
- Memory overhead: ~7MB for 10,000 functions

**Parallel Processing with Rayon**:
- CPU-bound work runs in parallel
- Sequential aggregation maintains consistency
- Adaptive batching optimizes memory usage

Source: `ARCHITECTURE.md:120-200`

See [Architecture](../architecture.md) for detailed design documentation.

## Key Differentiators

What makes debtmap effective:

- **Pattern-Based Complexity Adjustment** - Entropy analysis reduces false positives by identifying boilerplate patterns
- **Multi-Pass Analysis** - Compares raw vs normalized complexity for accurate attribution
- **Coverage-Risk Correlation** - Finds genuinely risky code, not just complex code
- **Functional Purity Tracking** - Identifies side effects and pure functions for targeted refactoring
- **Context-Aware Detection** - Considers architectural context, not just isolated metrics
- **Evidence-Based Prioritization** - ROI-driven recommendations backed by multiple signals

## Performance

Debtmap achieves high throughput through parallel processing:

| Codebase Size | Target | Actual | Speedup |
|---------------|--------|--------|---------|
| 50 files      | <0.5s  | ~0.3s  | 4x      |
| 250 files     | <1s    | ~0.8s  | 6.25x   |
| 1000 files    | <5s    | ~3.5s  | 5.7x    |

Source: `ARCHITECTURE.md:100-106`

See [Parallel Processing](../parallel-processing.md) for optimization details.
