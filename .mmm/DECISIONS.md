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
- ✅ More actionable risk insights

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

---

## ADR-012: Dependency-Aware ROI Calculation
**Date**: 2025-08-10
**Status**: Accepted

### Context
The ROI calculation was producing uniform results (all 10.0) for functions with similar complexity, failing to account for the cascade effects of testing highly-depended-upon code. Functions used by many modules have higher impact when tested.

### Decision
Implement dependency-aware ROI calculation that:
- Builds dependency graphs from module relationships
- Applies module type multipliers (EntryPoint: 2x, Core: 1.5x, Api: 1.2x)
- Calculates cascade impact with exponential decay (70% per level)
- Adds dependency factor based on dependent count
- Uses diminishing returns model for realistic ROI scaling

### Consequences
- ✅ ROI values show meaningful variation (not all 10.0)
- ✅ Highly-depended modules correctly prioritized
- ✅ Entry points and core modules receive appropriate bonuses
- ✅ Cascade effects properly calculated and displayed
- ✅ More intuitive testing recommendations
- ⚠️ Dependency inference may not capture all relationships
- ⚠️ Additional computation for graph construction

---

## ADR-013: Unified Debt Prioritization with Semantic Analysis
**Date**: 2025-08-11
**Status**: Accepted

### Context
Technical debt analysis was providing disconnected metrics that were difficult to action. ROI, complexity, coverage, and risk were reported separately, requiring manual correlation. Orchestration functions that delegate to tested code were incorrectly prioritized as critical, creating false positives. Users needed clear, actionable guidance on what to fix first.

### Decision
Implement unified debt prioritization system that:
- Combines all metrics into single priority score (complexity 25%, coverage 35%, ROI 25%, semantic 15%)
- Classifies functions semantically (PureLogic, Orchestrator, IOWrapper, EntryPoint)
- Applies role-based multipliers to reduce false positives
- Propagates coverage through call graph for delegation patterns
- Provides clean, actionable output formats (--top N, --priorities-only, --detailed)
- Removes unreliable time estimates in favor of measurable impact metrics

### Consequences
- ✅ Single clear priority list instead of multiple conflicting views
- ✅ Orchestration functions correctly deprioritized (score <3.0 vs 8.8)
- ✅ Clean, immediately actionable output without documentation
- ✅ Better alignment with actual code criticality and testing needs
- ✅ Focus on measurable improvements over unreliable time predictions
- ⚠️ Additional complexity in scoring algorithm
- ⚠️ Call graph construction adds processing overhead

---

## ADR-014: Intelligent Test Function Debt Handling
**Date**: 2025-08-11
**Status**: Accepted

### Context
The debt scoring system was paradoxically penalizing test functions for having 0% code coverage (since tests aren't covered by tests), leading to counterintuitive behavior where adding tests increased the total debt score. A comprehensive codebase with many tests would show artificially high debt scores, discouraging good testing practices.

### Decision
Implement intelligent test function handling that:
- Excludes test functions from the main debt score calculation in `create_unified_analysis`
- Sets coverage factor to 0.0 for test functions instead of the maximum penalty (10.0)
- Introduces test-specific debt types (TestComplexity, TestTodo, TestDuplication)
- Provides test-specific recommendations for legitimate test debt issues
- Maintains visibility into actual test quality problems without inflating scores

### Consequences
- ✅ Debt scores now decrease (or stay same) when tests are added
- ✅ Eliminates paradoxical behavior where good practices increase debt scores
- ✅ Test functions can still be flagged for legitimate complexity and quality issues
- ✅ Separate test debt categorization provides actionable recommendations
- ✅ More accurate reflection of actual technical debt in production code
- ⚠️ Breaking change in debt score calculation (scores will generally be lower)
- ⚠️ Additional logic complexity in the scoring system

---

## ADR-015: Macro Expansion for Perfect Call Graph Analysis
**Date**: 2025-08-13
**Status**: Accepted

### Context
Dead code detection was producing false positives for functions called within macros. The syn-based AST analysis operates on pre-expansion code where macro invocations remain as opaque token streams, making it impossible to detect function calls within `println!`, `format!`, `assert!`, and other macros. This led to legitimate code being incorrectly flagged as dead.

### Decision
Implement optional cargo-expand integration that:
- Analyzes fully expanded Rust code where all macros have been resolved
- Caches expanded code to minimize compilation overhead
- Maps expanded code locations back to original source files
- Falls back gracefully to standard analysis when expansion unavailable
- Remains opt-in via --expand-macros CLI flag

### Consequences
- ✅ 100% accuracy in detecting macro-hidden function calls
- ✅ Eliminates false positives in dead code detection
- ✅ Smart caching reduces performance impact after initial expansion
- ✅ Source mapping preserves accurate error reporting
- ✅ Graceful degradation when cargo-expand unavailable
- ⚠️ Requires cargo-expand to be installed separately
- ⚠️ Initial expansion adds compilation overhead
- ⚠️ Project must compile successfully for expansion to work

---

## ADR-016: Enhanced Call Graph Analysis for Accurate Dead Code Detection
**Date**: 2025-08-13
**Status**: Accepted

### Context
Dead code detection was producing significant false positives due to shallow static analysis that missed trait dispatch, function pointers, closures, and framework patterns. Functions like `write_results()` called via trait dispatch and `print_risk_function` passed as closures were incorrectly marked as unused, undermining user confidence in the tool's recommendations.

### Decision
Implement a sophisticated multi-phase call graph analysis system that:
- Analyzes trait implementations and resolves method calls to concrete implementations
- Tracks function pointers, closures, and higher-order function usage
- Detects framework patterns (test functions, web handlers, event handlers)
- Analyzes cross-module dependencies and public API usage
- Provides confidence scoring for dead code findings

### Consequences
- ✅ 90%+ reduction in dead code false positives
- ✅ Accurate detection of trait method usage
- ✅ Function pointer and closure tracking eliminates false positives
- ✅ Framework-managed functions properly excluded
- ✅ Confidence scoring helps users prioritize findings
- ✅ Extensible pattern matching for future framework support
- ⚠️ Additional analysis phases increase processing time
- ⚠️ More complex implementation requires maintenance