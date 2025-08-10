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