# Debtmap Roadmap

## Completed Specs
- [x] **Spec 01**: Core standalone tool implementation
  - Functional architecture
  - Rust and Python analyzers
  - Complexity and debt detection
  - Multiple output formats
  
- [x] **Spec 02**: Complete implementation to 100%
  - Fixed line number tracking
  - TODO/FIXME detection working
  - Code smell detection (long parameters, large modules, deep nesting)
  - Circular dependency detection
  - Coupling metrics
  - Persistent data structures with im crate
  - Lazy evaluation and monadic patterns
  - Caching for incremental analysis
  - Comprehensive test suite

- [x] **Spec 03**: Inline suppression comments
  - Block suppression with debtmap:ignore-start/end
  - Line-specific suppression with debtmap:ignore
  - Next-line suppression with debtmap:ignore-next-line
  - Type-specific suppression (e.g., [todo,fixme])
  - Wildcard suppression with [*]
  - Optional reason documentation
  - Multi-language support (Rust, Python)
  - Unclosed block warnings

- [x] **Spec 05**: Complexity-Coverage Risk Analysis
  - Optional LCOV file integration via --lcov or --coverage-file
  - Risk scoring combining complexity and coverage
  - Critical risk function identification
  - Test effort estimation based on cognitive complexity
  - ROI-based testing recommendations
  - Works without coverage data (complexity-only mode)
  - Dynamic coverage thresholds based on complexity

- [x] **Spec 07**: Recalibrated Risk Formula
  - Increased coverage weight from 0.3 to 0.5
  - Added exponential penalty for low coverage
  - Integrated technical debt into risk calculation
  - Strategy pattern for risk calculation
  - Risk scores now use full 0-10 range effectively
  - Enhanced test recommendations with debt context

- [x] **Spec 08**: Fix Testing Prioritization Algorithm
  - Multi-stage prioritization pipeline for better recommendations
  - Zero-coverage modules always prioritized first
  - Module criticality scoring (entry points, core, API, etc.)
  - Dynamic ROI calculation with realistic risk reduction
  - Effort estimation based on complexity and module type
  - Cascade impact analysis for dependencies
  - Clear rationale for each recommendation

## Current Phase: Foundation
We have completed the initial implementation with core functionality for analyzing Rust and Python code, including enhanced risk analysis with debt integration.

- [x] **Spec 09**: Fix Complexity Calculation Bugs
  - Cyclomatic complexity now correctly counts all branches
  - Cognitive complexity properly tracks nesting  
  - Functions, methods, and closures are all counted
  - Realistic average complexity values achieved

- [x] **Spec 11**: Context-Aware Risk Analysis
  - Critical path identification from entry points
  - Dependency risk propagation through module graph
  - Git history integration for change frequency
  - Pluggable context provider architecture
  - Weighted risk calculation with context

- [x] **Spec 14**: Dependency-Aware ROI Calculation
  - Dynamic dependency graph construction from module relationships
  - Module type-based impact multipliers
  - Cascade effect calculation with exponential decay
  - Dependency factor for ROI enhancement
  - Meaningful ROI variation based on code criticality

- [x] **Spec 18**: Intelligent Test Function Debt Handling
  - Test functions excluded from main debt score calculation
  - Test functions no longer penalized for lack of coverage
  - Test-specific debt types and recommendations
  - Separate tracking of test debt vs production debt
  - Prevents paradoxical debt score increases when adding tests

- [x] **Spec 19**: Unified Debt Prioritization with Semantic Analysis
  - Single unified priority score combining all metrics
  - Semantic function classification to avoid false positives
  - Call graph construction for delegation detection
  - Coverage propagation through transitive dependencies
  - Clean output formats for different use cases
  - Focus on measurable impact over time estimates

- [x] **Spec 21**: Dead Code Detection
  - Automatic detection of unused functions via call graph
  - Visibility-aware recommendations
  - Framework pattern exclusions
  - Integration with unified priority scoring

- [x] **Spec 22**: Perfect Macro Function Call Detection with cargo-expand
  - cargo-expand integration for expanded AST analysis
  - Eliminates false positives from macro-hidden calls
  - Smart caching for performance
  - Source location mapping
  - Graceful fallback mechanism

- [x] **Spec 23**: Enhanced Call Graph Analysis for Accurate Dead Code Detection
  - Multi-phase call graph construction (basic, traits, pointers, patterns, cross-module)
  - Trait dispatch detection and method resolution
  - Function pointer and closure tracking
  - Framework pattern recognition (test functions, web handlers, etc.)
  - Cross-module dependency analysis
  - 90%+ reduction in dead code false positives
  - Confidence scoring for dead code findings
  - Extensible pattern matching system

- [x] **Spec 24**: Refined Risk Scoring Methodology
  - Evidence-based risk assessment with measurable factors
  - Statistical baselines from codebase analysis (P50, P90, P95, P99)
  - Role-aware risk calculation for different function types
  - Specific risk analyzers for complexity, coverage, coupling, and change frequency
  - Actionable remediation recommendations with effort estimates
  - Risk classification system from WellDesigned to Critical
  - Confidence scoring for risk predictions
  - Module-type aware thresholds for different code contexts

- [x] **Spec 26**: Language-Specific Call Graph Architecture
  - Renamed EnhancedCallGraph to RustCallGraph for clarity
  - Renamed EnhancedCallGraphBuilder to RustCallGraphBuilder
  - Established clear pattern for language-specific implementations
  - Architecture ready for multi-language expansion
  - Zero functional changes - pure refactoring

- [x] **Spec 28**: Enhanced Markdown Output Formatting
  - Full feature parity with terminal output
  - Unified priority scoring visualization
  - Dead code detection results
  - Testing recommendations with ROI
  - Call graph insights with verbosity levels
  - Collapsible sections for detailed analysis

- [x] **Spec 29**: AST-Based Type Tracking for Accurate Method Call Resolution
  - Variable type tracking with explicit annotations
  - Type inference from struct literals and constructors
  - Nested scope management for variable resolution
  - Method call resolution using type information
  - Eliminates false positives in dead code detection
  - Support for struct fields and function parameters

- [x] **Spec 30**: Enhanced Type Tracking for Field Access and Cross-Module Resolution
  - Global type registry for struct definitions
  - Field type tracking for named and tuple structs
  - Self reference resolution in impl blocks
  - Field access chain resolution (self.a.b.c)
  - Cross-module type resolution via imports
  - Type alias tracking and resolution
  - 50%+ reduction in false positives

- [x] **Spec 31**: Testing Quality Patterns Detection
  - Test functions without assertions detection
  - Overly complex test identification with complexity scoring
  - Flaky test pattern detection (timing dependencies, random values, external services)
  - Framework-aware test function recognition (#[test], test_, _test patterns)
  - Test simplification suggestions (reduce mocking, extract helpers, parameterize)
  - Integration with existing debt tracking system
  - New DebtType::TestQuality for test-specific issues

- [x] **Spec 32**: Trait Implementation Tracking for Dynamic Dispatch Resolution
  - Comprehensive trait definition extraction
  - Trait implementation mapping to types
  - Trait object resolution for dynamic dispatch
  - Generic trait bound resolution
  - Blanket implementation detection
  - Method resolution order implementation
  - Associated type tracking
  - Supertrait relationship tracking
  - 15-20% reduction in trait-related false positives

- [x] **Spec 33**: Functional Error Handling Refactor
  - Eliminated 25+ instances of error swallowing patterns
  - Replaced `if let Ok(...)` with proper Result handling
  - Added contextual error messages with `.with_context()`
  - Implemented proper error logging with log levels
  - Improved debugging capabilities
  - Maintained backwards compatibility
  - Updated tests for new error semantics

- [x] **Spec 34**: Error Swallowing Debt Detection
  - Added new DebtType::ErrorSwallowing variant
  - AST-based pattern detection for error swallowing
  - Detects multiple anti-patterns (if let Ok, let _, .ok(), etc.)
  - Context-aware priority classification
  - Integration with suppression system
  - Lower priority for test functions
  - Note: Detection works but items not shown in priority output

- [x] **Spec 35**: Debt Pattern Unified Scoring Integration
  - Comprehensive integration between pattern detection and unified scoring
  - FunctionDebtProfile structure aggregates all issues per function
  - DebtAggregator provides efficient indexing and lookup
  - All detected issues directly influence priority scores
  - Replaces heuristic-based scoring with actual detection results
  - Performance optimized for large codebases

- [x] **Spec 38**: Multi-Language Detector Support (Foundation)
  - Established detector architecture pattern for multi-language support
  - Created foundation for Python-specific detectors (performance, organization, security, resource, testing)
  - Architecture ready for JavaScript/TypeScript detector implementation
  - Demonstrated approach for extending language-specific analysis beyond basic metrics
  - Note: Full implementation requires API compatibility updates with rustpython-parser 0.4

- [x] **Spec 41**: Test Performance as Tech Debt
  - Recognizes test performance issues as valid technical debt
  - Configurable detection for performance patterns in test files
  - Severity reduction for test performance issues (default: 1 level)
  - Option to completely disable test performance detection
  - Documents test performance as lower-priority debt
  - Helps teams gradually improve test suite performance

- [x] **Spec 42**: Smart Pattern Matching for Performance Detection
  - Context-aware pattern detection distinguishing test fixtures from production issues
  - Module type classification (test, production, utility, benchmark)
  - Function intent recognition (setup, teardown, business logic, I/O wrapper)
  - Dynamic severity adjustment based on context
  - Pattern correlation for multi-pattern analysis
  - 70%+ reduction in false positives while maintaining sensitivity
  - Configurable confidence thresholds for different contexts

- [x] **Spec 44**: Enhanced Scoring Differentiation
  - Multi-factor scoring system with function criticality analysis
  - Hot path detection based on call graph position
  - Production vs test code weighting (default: 1.0 vs 0.3)
  - Call frequency and dependency impact calculation
  - Score normalization for better distribution (0-10 scale)
  - Deterministic jitter to prevent identical scores
  - --enhanced-scoring and --legacy-scoring CLI flags
  - --exclude-tests option to filter test code from analysis

- [x] **Spec 47**: Unified AST Traversal Optimization
  - Single-pass AST traversal for all performance detectors
  - Unified data collection in comprehensive data structures
  - Parallel pattern detection on pre-collected data
  - 60-80% reduction in AST traversal overhead
  - Context sharing between detectors
  - Optimized detector adapters for collected data analysis
  - Controlled by DEBTMAP_OPTIMIZED_PERF environment variable

- [x] **Spec 48**: Fix Ignore Configuration Implementation
  - Fixed critical bug where ignore patterns were not being applied
  - Configuration patterns now properly passed to FileWalker
  - Support for glob patterns (*, **, ?, [abc], [!abc])
  - Pattern matching against relative paths, absolute paths, and filenames
  - 95% reduction in false positives from test files
  - Tests added for configuration and file discovery
  - Documentation updated with pattern syntax and examples

## Pending Specs

### Enhancements
- [ ] **Spec 10**: Enhance Complexity Detection with Modern Patterns
  - Detect async/await patterns
  - Identify callback chains and promises
  - Recognize functional composition patterns
  - Account for error handling complexity

- [ ] **Spec 12**: Improve ROI Calculation (Completed as part of Spec 14)
  - Enhanced return on investment calculations

- [ ] **Spec 13**: Add Risk Categories
  - Categorized risk assessment system

## Upcoming Milestones

### Phase 1: Language Expansion (Q1 2025)
- [ ] Add JavaScript/TypeScript support
- [ ] Add Go support
- [ ] Add Java support
- [ ] Integrate tree-sitter for universal parsing

### Phase 2: Advanced Analysis (Q2 2025)
- [ ] Implement incremental analysis
- [ ] Add caching layer for performance
- [ ] Create custom rule definitions
- [ ] Add security vulnerability detection

### Phase 3: Integration (Q3 2025)
- [ ] Language Server Protocol implementation
- [ ] CI/CD pipeline integration
- [ ] Git hook support
- [ ] IDE extensions (VS Code, IntelliJ)

### Phase 4: Intelligence (Q4 2025)
- [ ] Automatic refactoring suggestions
- [ ] Historical trend analysis
- [ ] Team productivity metrics

## Success Metrics
- Process 100k+ lines in under 5 seconds
- Support 10+ programming languages
- 95% accuracy in complexity calculations
- Zero false positives in critical debt items

## Technical Debt
- Improve line number tracking in AST analysis
- Add more comprehensive test coverage
- Optimize memory usage for large files
- Implement proper configuration file parsing