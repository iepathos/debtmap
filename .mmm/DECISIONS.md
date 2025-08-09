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