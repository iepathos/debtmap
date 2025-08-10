# Architectural Decision Records

## ADR-001: Functional Core / Imperative Shell Pattern
**Date**: 2025-08-09
**Status**: Accepted

### Context
Need clear separation between business logic and IO operations for testability and maintainability.

### Decision
Implement functional core with pure functions for all analysis logic, keeping IO operations at the boundaries.

### Consequences
- ✅ Highly testable core logic
- ✅ Easy to reason about data flow
- ✅ Parallelizable by default
- ⚠️ Requires discipline to maintain separation

---

## ADR-002: Use syn for Rust Parsing
**Date**: 2025-08-09
**Status**: Accepted

### Context
Need robust Rust AST parsing for complexity analysis.

### Decision
Use the `syn` crate with full features for parsing Rust code.

### Consequences
- ✅ Battle-tested and well-maintained
- ✅ Full Rust syntax support
- ✅ Good documentation and examples
- ⚠️ Large dependency size

---

## ADR-003: Immutable Data Structures with im
**Date**: 2025-08-09
**Status**: Accepted

### Context
Need efficient immutable collections for functional programming style.

### Decision
Use the `im` crate for persistent data structures (Vector, HashMap).

### Consequences
- ✅ Efficient structural sharing
- ✅ Thread-safe by default
- ✅ Functional programming friendly
- ⚠️ Different API from standard collections

---

## ADR-004: Parallel Processing with Rayon
**Date**: 2025-08-09
**Status**: Accepted

### Context
Need to process multiple files concurrently for performance.

### Decision
Use rayon for parallel iteration over files.

### Consequences
- ✅ Simple parallel iterator API
- ✅ Work-stealing for efficiency
- ✅ Automatic thread pool management
- ⚠️ Not suitable for async IO

---

## ADR-005: SHA-256 for Duplication Detection
**Date**: 2025-08-09
**Status**: Accepted

### Context
Need reliable content hashing for identifying duplicate code blocks.

### Decision
Use SHA-256 hashing on normalized code chunks.

### Consequences
- ✅ Cryptographically secure hashing
- ✅ Very low collision probability
- ✅ Standard and well-understood
- ⚠️ Slower than non-cryptographic hashes

---

## ADR-006: Inline Suppression Comments
**Date**: 2025-08-09
**Status**: Accepted

### Context
Need mechanism to suppress false positives in debt detection, especially in test fixtures.

### Decision
Implement inline comment-based suppression using debtmap:ignore syntax similar to ESLint/Pylint.

### Consequences
- ✅ Fine-grained control over suppressions
- ✅ Self-documenting with reason support
- ✅ Language-agnostic approach
- ✅ No external configuration files needed
- ⚠️ Adds parsing overhead to analysis

---

## ADR-007: Optional Coverage Integration for Risk Analysis
**Date**: 2025-01-09
**Status**: Accepted

### Context
Test coverage alone doesn't indicate risk - a simple untested getter is low risk while an untested complex algorithm is critical. Need to correlate complexity with coverage to identify actual risk.

### Decision
Implement optional LCOV integration that combines with existing complexity metrics to calculate risk scores, prioritize testing efforts, and provide ROI-based recommendations.

### Consequences
- ✅ Identifies high-risk untested complex code
- ✅ Works without coverage data (complexity-only mode)
- ✅ Provides actionable testing recommendations
- ✅ Language-agnostic LCOV format support
- ⚠️ Requires up-to-date coverage data for accuracy

---

## ADR-008: Recalibrated Risk Formula with Debt Integration
**Date**: 2025-08-10
**Status**: Accepted

### Context
Initial risk formula underweighted coverage gaps and didn't account for technical debt accumulation. A codebase with 37% coverage and debt 12.9x over threshold still showed "LOW" risk.

### Decision
Implement enhanced risk formula with strategy pattern, increasing coverage weight to 0.5, adding exponential penalties for low coverage, and integrating debt scores as multiplicative factors.

### Consequences
- ✅ Risk scores properly reflect coverage gaps and debt
- ✅ Full 0-10 risk scale utilization
- ✅ Strategy pattern enables multiple risk formulas
- ✅ Legacy mode for backwards compatibility
- ✅ More actionable risk insights
- ⚠️ Breaking change in risk score values without --legacy-risk flag

---

## ADR-009: Multi-Stage Testing Prioritization Pipeline
**Date**: 2025-08-10
**Status**: Accepted

### Context
Previous testing prioritization produced suboptimal recommendations with uniform ROI values (all 1.1), prioritized low-complexity functions over untested critical modules, and showed unrealistic risk reduction estimates.

### Decision
Implement a multi-stage prioritization pipeline with specialized stages for zero-coverage detection, criticality scoring, complexity risk analysis, dependency impact, and effort optimization. Use dynamic ROI calculation with cascade effects and realistic effort estimation.

### Consequences
- ✅ Zero-coverage modules always prioritized first
- ✅ Entry points and core modules properly weighted
- ✅ Realistic ROI values with meaningful variation
- ✅ Accurate risk reduction estimates (1-20% range)
- ✅ Clear rationale for each recommendation
- ✅ Extensible pipeline architecture
- ⚠️ More complex implementation than simple sorting

---

## ADR-010: Fixed Complexity Calculation Algorithms
**Date**: 2025-08-10
**Status**: Accepted

### Context
Complexity calculations had critical bugs: cyclomatic complexity always returned 1 regardless of branches, if/else statements weren't counted, match expressions were overcounting, and closures/nested functions were missed entirely.

### Decision
Rewrote complexity calculation to properly count all control flow: if statements add 1, else branches add 1, match expressions use n-1 formula (where n is number of arms), loops add 1 each, and closures are tracked as separate functions.

### Consequences
- ✅ Accurate cyclomatic complexity matching standard definitions
- ✅ Realistic average complexity values (3-8 typical range)
- ✅ All functions including closures properly counted
- ✅ Better risk assessment based on true complexity
- ⚠️ Breaking change in complexity values from previous versions

---

## ADR-011: Context-Aware Risk Analysis
**Date**: 2025-08-10
**Status**: Accepted

### Context
Risk analysis previously treated all code paths equally, missing critical context about actual code usage, dependency relationships, historical stability, and business impact. This led to misaligned priorities where rarely-used complex code received the same risk score as critical path functions.

### Decision
Implement a pluggable context provider system that enriches risk analysis with multiple context sources:
- Critical path analysis from entry points (main, API handlers, CLI commands)
- Dependency risk propagation through module graph
- Git history for change frequency and bug density
- Extensible architecture for future providers (runtime metrics, business context)

### Consequences
- ✅ Risk scores better reflect real-world impact
- ✅ Critical paths receive appropriate priority
- ✅ Historical instability factors into risk
- ✅ Extensible for future context sources
- ✅ Optional and backward compatible (via --context flag)
- ⚠️ Additional processing time for context gathering
- ⚠️ Requires git repository for full functionality