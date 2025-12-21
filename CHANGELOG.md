# Changelog

All notable changes to debtmap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.11.0] - 2025-12-20

### Added

- **LLM-Optimized Output Format** (Spec 264)
  - New `--llm` output format designed for AI agent consumption
  - Structured context suggestions with specific, actionable guidance
  - Optimized for context window efficiency

- **TUI AI Sensor Model Redesign** (Spec 265)
  - Redesigned TUI to emphasize AI sensor positioning
  - Clearer presentation of identification, severity, and structural context
  - Improved context suggestion display

- **Context Suggestions for God Objects** (Spec 263)
  - God objects now generate targeted context suggestions
  - Provides specific guidance for AI agents analyzing large structs

### Changed

- **Documentation Pivot to AI Sensor Model** (Spec 266)
  - Updated documentation to reflect debtmap's role as an AI sensor
  - Emphasis on providing accurate identification and severity data to AI agents

### Fixed

- **TUI Score Breakdown Transparency**
  - Score breakdown now accurately reflects actual calculation
  - Coverage multiplier correctly displayed in calculation steps
  - Entropy dampening shown as multiplier only for god objects

### Performance

- **Eliminated Redundant File I/O**
  - Removed unnecessary file reads after debt scoring phase completes
  - Improved analysis speed for large codebases

## [0.10.0] - 2025-12-19

### Strategic Direction

- **AI Sensor Model Pivot** (Specs 262-266)
  - Debtmap now positions itself as an "AI sensor" - providing accurate identification, quantified severity, and structural context to AI agents (Claude, Copilot, Cursor) that have semantic understanding
  - Planned removal of template-based recommendations in favor of context window suggestions
  - Key insight: Static analysis can accurately tell you WHERE problems are and HOW SEVERE they are; let AI determine HOW to fix them

### Added

- **Score Calculation Transparency** (Spec 260, 261)
  - New Score Breakdown detail page (Page 2) in TUI showing exact calculation steps
  - Explicit display of each scoring component: complexity, coverage, dependencies, contextual risk
  - Shows specific adjustments and their contributions to final score
  - Removed score clamping for more accurate representation
  - Removed `Score0To100` and `Score0To1` wrapper types in favor of direct values

- **Function-Level Git History Analysis** (Spec 195)
  - Git blame integration for accurate contributor count per function
  - Function-level churn and authorship metrics
  - Continuous scoring for git history risk contribution

- **God Object Detection Improvements**
  - **Method Complexity Weighting** (Spec 211) - Methods weighted by their individual complexity contribution
  - **Accessor and Boilerplate Detection** (Spec 209) - Identifies and discounts simple accessor methods
  - **Pure Function Method Weighting** (Spec 213) - Pure methods contribute less to god object score
  - **Functional Decomposition Recognition** (Spec 215) - Recognizes intentional functional decomposition patterns
  - **Trait-Mandated Method Detection** (Spec 217) - Methods required by trait implementations weighted differently
  - **Test Code Exclusion** (Spec 214) - Test code excluded from LOC metrics
  - **Cohesion Gate** (Spec 206) - Low cohesion now required before flagging as god object
  - **Per-Struct LOC Fix** (Spec 207) - Accurate LOC calculation for individual structs

- **Entropy Type Consolidation** (Spec 218)
  - Unified entropy analysis types across the codebase
  - Improved entropy data propagation through god object analysis pipeline

- **Observability Improvements**
  - **Panic Hook with Crash Reports** (Spec 207) - Structured crash reports for debugging
  - **Structured Tracing with Spans** (Spec 208) - Comprehensive tracing throughout analysis
  - **Rayon Parallel Span Propagation** (Spec 209) - Tracing spans properly propagate through parallel code
  - **SourceMap Overflow Prevention** (Spec 210) - Prevents crashes on large codebases

- **Output Quality Improvements**
  - **Output Invariant Testing** (Spec 230) - Schema validation for all output formats
  - **Debt Item Deduplication** (Spec 231) - Eliminates duplicate items in output
  - **Anti-Pattern Details** (Spec 197) - Exposed in JSON and TUI
  - **Cohesion Metrics** (Spec 198) - Exposed in output for transparency

### Changed

- **Unified Extraction Architecture** (Specs 211-214)
  - Single-pass parsing for Rust files via `UnifiedFileExtractor`
  - Extraction adapters for consistent data transformation
  - Eliminated redundant AST traversals
  - Complete file parsing migration to unified extractor (Spec 204)
  - Consolidated god object detection to extraction adapter (Spec 212)

- **God Object Default Role**
  - Changed default role classification from `Orchestrator` to `PureLogic`
  - More accurate default assumption for unclassified structs

- **Scoring Improvements**
  - Isolated components now contribute zero dependency risk
  - Continuous (non-stepped) scoring for git history risk
  - Removed redundant god object warning from score breakdown
  - Fixed double-counted god_mult in calculation steps

### Performance

- **Parallel Debt Item Scoring** (Spec 196)
  - Rayon-based parallel scoring for debt items
  - Significant speedup on large codebases

- **File Line Count Caching** (Spec 195)
  - Per-file line count caching eliminates redundant file reads

- **Shared Context Detection**
  - ContextDetector shared across parallel metric processing
  - Eliminated redundant file I/O in Phase 3 analysis

### Internal (Refactoring)

- **God Object Remediation**
  - Split `ResultsApp` into focused modules (navigation, rendering, state)
  - Decomposed `unified.rs` god module into focused submodules
  - Split `analyze.rs` command into focused modules
  - Split `validate.rs` into focused modules
  - Split `cfg_builder.rs` into focused modules
  - Extracted clustering module from god object analysis
  - Decomposed `evidence_calculator` into focused modules
  - Split `effects.rs` into focused modules
  - Split `trait_registry.rs` into focused modules
  - Split `module_structure.rs` into focused modules
  - Unified TUI copy with rendering via shared builders

- **Nesting Calculation Consolidation** (Specs 201-203)
  - Single source of truth for nesting depth calculation
  - Comprehensive test coverage for nesting scenarios
  - Fixed else-if chain nesting calculation (Spec 198)

- **God Object Extraction Adapter Rewrite** (Spec 197)
  - Complete rewrite of extraction adapter for cleaner architecture

### Fixed

- TUI progress updates throughout all analysis stages
- Correct calculation steps displayed for god objects vs functions
- Detailed calculation steps shown for god objects in TUI
- Dampened cyclomatic calculation using correct input

## [0.9.2] - 2025-12-13

### Added

- **Effects System Expansion** (Spec 268)
  - Extended Stillwater effects integration across the codebase
  - Improved effect composition patterns for analysis pipelines

- **Effects-Based Progress System** (Spec 262)
  - Replaced callback-based progress reporting with Stillwater effects
  - Cleaner separation of progress concerns from core analysis logic

- **Call Graph Purity Propagation** (Spec 261)
  - Purity information now propagates through the call graph
  - Functions calling impure functions are correctly marked as impure
  - More accurate purity analysis across the entire codebase

- **Improved Purity Display Actionability** (Spec 260)
  - Enhanced TUI display of purity information with actionable insights
  - Clearer guidance on why functions are marked pure or impure

- **Match Expression CFG Modeling** (Spec 253)
  - Control flow graph now correctly models match expressions
  - Improved accuracy of data flow analysis through pattern matching

- **Pattern Destructuring Support for CFG** (Spec 252)
  - CFG analysis now handles pattern destructuring correctly
  - Better tracking of variable bindings in complex patterns

### Changed

- **Pure Core Extraction for Unified Analysis** (Spec 265)
  - Extracted pure business logic from unified_analysis module
  - Improved testability and functional composition

- **Data Flow Module Decomposition** (Spec 264)
  - Split monolithic data flow analysis into focused modules
  - Better separation of concerns in data flow analysis

- **Modular CLI Structure** (Spec 267)
  - Decomposed main.rs into modular CLI components
  - Cleaner organization of command-line interface code

- **TUI Actions Decomposition** (Spec 269)
  - Split TUI action handling into focused modules
  - Improved maintainability of interactive interface code

- **Simplified Mutation Analysis** (Spec 257)
  - Reduced mutation analysis to binary signals (mutates/doesn't mutate)
  - Removed over-engineered complexity tracking
  - Clearer, more actionable mutation information

- **Removed CFG Liveness Complexity** (Spec 258)
  - Simplified control flow graph analysis
  - Removed unnecessary liveness tracking overhead

- **Removed Dead Store Analysis** (Spec 256)
  - Eliminated dead store detection which produced too many false positives
  - Focused analysis on higher-value signals

- **Purity Analysis Relocated**
  - Moved purity analysis from patterns page to data flow page in TUI
  - More logical organization of analysis results

### Fixed

- **Data Flow Page Visibility**
  - Fixed Data Flow page not showing when purity info is available
  - Ensures users can access purity analysis results

- **Constants False Positive in Purity Analysis** (Spec 259)
  - Fixed constants being incorrectly flagged in purity analysis
  - More accurate identification of truly impure code

- **False Positive Dead Stores for Pattern Bindings**
  - Eliminated incorrect dead store warnings for pattern-bound variables

- **Option/Collection Unwrap Elimination** (Spec 266)
  - Replaced unsafe unwrap calls on Options and collections with proper error handling
  - Improved robustness and safety

- **Critical Unwrap Elimination for Lock Safety** (Spec 263)
  - Replaced unwrap on mutex locks with proper error handling
  - Prevents potential panics in concurrent code

### Internal

- Removed escape and taint analysis from data flow module (unused complexity)
- Removed dead cfg_analysis checks from display logic
- Applied clippy fixes across the codebase
- Cleaned up implemented spec files

## [0.9.1] - 2025-12-12

### Added

- **Unified View Data Pipeline** (Specs 250-253)
  - New `PreparedDebtView` unified data model for consistent output across all formats
  - View preparation pipeline separates data transformation from rendering
  - Output format unification ensures TUI, markdown, JSON, and terminal show identical data
  - Refactored `format_split_recommendations_markdown` for cleaner code structure

- **Accurate LOC Calculation** (Spec 201)
  - Lines of code now calculated from actual source spans, not approximations
  - More precise function size metrics for complexity scoring

- **Clean Match Dispatcher Pattern Recognition** (Spec 206)
  - Improved detection of clean match dispatcher patterns
  - Reduces false positives for well-structured match statements
  - Better identification of delegation patterns vs actual complexity

- **Data Flow Analysis for Impl Block Methods** (Spec 202)
  - Enhanced data flow analysis now supports methods within impl blocks
  - Improved accuracy of mutation tracking in object-oriented Rust code

- **Multi-Debt Type Accumulation** (Spec 228)
  - Functions now accumulate multiple independent debt classifications by default
  - Provides comprehensive technical debt assessment per function
  - Example: A function can be flagged as both a complexity hotspot AND have testing gaps
  - Supports three independent debt types: Testing Gaps, Complexity Hotspots, Dead Code
  - Each debt type appears as a separate entry in the output for the same function

### Fixed

- **Severity Classification Consistency** (Spec 251)
  - Fixed critical bug where severity labels were inconsistent between output formats
  - Unified severity classification to use 70/50/30 thresholds for 0-100 scale
  - TUI, markdown, JSON, and terminal output now show identical severity labels
  - Removed duplicated severity calculation logic from TUI modules
  - Improved clarity: scores 70+ are CRITICAL, 50+ are HIGH, 30+ are MEDIUM, <30 are LOW

- **Function Dependencies in TUI and JSON**
  - Fixed function dependencies showing zeros in TUI and JSON output
  - Dependency counts now correctly propagate through all output formats

- **TUI Display Issues**
  - Fixed popup areas to prevent background bleed-through
  - Improved clipboard copy to match rendered layout

- **Test Stability**
  - Fixed doctest and eliminated test race condition
  - Improved test reliability across the suite

### Changed

- **View Pipeline Refactoring**
  - Split `view_pipeline.rs` into focused modules for better maintainability
  - Cleaner separation of concerns in the view layer

## [0.8.0] - 2025-12-05

### Breaking Changes

- **Legacy Code Cleanup for 1.0 Release** (Spec 201)
  - **Removed**: `src/organization/god_object/legacy_compat.rs` module
    - This module provided backward compatibility shims that are no longer needed
    - All functionality is available through modern replacements
  - **Removed**: Deprecated formatter functions
    - `format_priority_item_legacy()` - Use `format_priority_item()` instead (uses pure + writer pattern)
    - `apply_formatted_sections()` - Functionality integrated into `format_priority_item()`
  - **Renamed**: Cognitive complexity function for clarity
    - `calculate_cognitive_legacy()` → `calculate_cognitive_visitor_based()`
    - Function behavior unchanged, name now accurately describes visitor-based implementation
  - **Impact**: External library users should migrate to modern equivalents
  - **Migration**: All modern replacements existed in v0.8.0+, no feature loss

### Changed

- **God Object Detector Refactoring** (Specs 181a-181i)
  - Completed migration of god object detection to pure functional architecture
  - Extracted pure functions for scoring, classification, and recommendation logic
  - Separated I/O operations (AST visiting) from pure computation
  - Organized into modular structure: types, thresholds, predicates, scoring, classifier, recommender, detector, ast_visitor
  - Removed legacy monolithic implementations (god_object_detector.rs, god_object_analysis.rs)
  - Improved testability and maintainability through functional composition
  - Updated ARCHITECTURE.md with new module structure documentation

- **Formatter Refactoring** (Specs 203-207)
  - Separated pure formatting logic from I/O operations (Spec 203)
  - Consolidated pattern display system for consistent output (Spec 204)
  - Broke up monolithic formatter into focused modules (Spec 205)
  - Modularized formatter_markdown.rs for better maintainability (Spec 206)
  - Modularized formatter_verbosity.rs into separate components (Spec 207)
  - Improved code organization and testability of formatting layer

### Added

- **Interactive TUI Results Explorer** (Spec 211)
  - Specification added for keyboard-driven results exploration interface
  - Progressive disclosure design to handle large result sets (386+ items)
  - Planned features: search, filtering, sorting, and editor integration
  - Addresses scalability issues with flat text output for large codebases

- **Zen Minimalist TUI Progress Visualization** (Spec 210)
  - Beautiful ratatui-based TUI for real-time analysis progress
  - 9-stage pipeline visualization with smooth 60 FPS animations
  - Responsive layout adapting to terminal size (minimal/compact/standard/full)
  - Hierarchical progress with active stage expansion and rich statistics
  - Pure functional rendering logic separated from state management
  - Added `--no-tui` and `--quiet` flags for flexibility

- **Composable Pipeline Architecture** (Spec 209)
  - Type-safe composable pipeline system for analysis stages
  - Stage trait with PureStage and FallibleStage implementations
  - PipelineBuilder with fluent API for pipeline construction
  - Conditional stage support via `.when()` method
  - Automatic progress reporting and per-stage timing
  - Foundation for replacing monolithic unified_analysis_computation

- **Pure Function Extraction** (Spec 208)
  - Extracted pure business logic into focused pipeline stage modules
  - Created modules: call_graph, purity, debt, scoring, filtering, aggregation
  - All functions pure, small (<20 lines), and well-documented
  - Improved testability and composability of analysis logic

- **Stillwater Effects Integration** (Spec 207)
  - Comprehensive documentation for Stillwater effects system
  - Effect system architecture added to ARCHITECTURE.md
  - Examples demonstrating effect composition patterns
  - Reader, Retry, and Validation pattern documentation
  - Migration strategy for imperative to effect-based code

- **Batched Git History Optimization** (Spec 206)
  - Single comprehensive git log query replaces multiple per-file queries
  - Pure parsing functions for git log output transformation
  - HashMap-based lookups for O(1) access to git history
  - 25x+ performance improvement: 260s → <10s for --context analysis
  - Integrated with GitHistoryProvider as optimized fast path

- **Behavioral Decomposition Refactoring**
  - Extracted types module for core data structures
  - Extracted behavioral categorization into separate module
  - Extracted clustering algorithms module
  - Extracted field analysis and recommendations module
  - Improved code organization and maintainability

- **Lock-Free Context Sharing for Parallel Risk Analysis** (Spec 204)
  - Implemented lock-free context sharing using Arc-wrapped immutable structures
  - Enables efficient parallel risk analysis without synchronization overhead
  - Improved performance for multi-threaded analysis workflows
  - Thread-safe access to shared context during risk assessment

- **State Machine Arm-Level Analysis** (Spec 203)
  - Deep analysis of state machine implementation patterns
  - Detects complex state transitions and arm-level complexity
  - Identifies potential issues in state machine design
  - Enhanced detection of state transition anti-patterns

- **Contextual Risk Integration for Priority Scoring** (Spec 202)
  - Integrated contextual risk factors into priority scoring algorithm
  - Risk scores now consider surrounding code context and dependencies
  - More accurate prioritization of technical debt items
  - Added contextual risk display to priority output for better insights

- **Enhanced Progress Feedback** (Spec 201)
  - Complete progress feedback for all analysis phases
  - Live updates during file discovery, parsing, and analysis
  - Unified progress display across all operations
  - Clear indication of call graph preparation and multi-pass analysis phases

- **Output Quality Improvements** (Spec 201)
  - Filter "no action needed" items from analysis output
  - Only show actionable technical debt items
  - Cleaner, more focused recommendations

- **Unified Progress Flow Display** (Spec 195)
  - Consistent progress indicators across all analysis stages
  - Real-time feedback on long-running operations
  - Improved user experience for large codebase analysis

### Improved

- **Analysis Architecture** (Specs 202, 208)
  - Inverted multi-pass analysis default for better performance (Spec 202)
  - Multi-pass analysis now explicitly enabled when needed
  - Merged dual responsibility classification systems (Spec 208)
  - Extracted severity and coverage classification for clarity (Spec 202)
  - Simplified classification logic and reduced code duplication

- **Data Flow Analysis** (Spec 201)
  - Enhanced mutation tracking capabilities
  - Better identification of state transition patterns
  - Improved detection of mutable state usage

### Fixed

- Fixed TUI display corruption and output loss issues
- Fixed duplicate progress output in TUI statistics
- Fixed clustering integration tests after behavioral_decomposition refactor
- Fixed spec 210 implementation gaps for TUI system
- Fixed spec 209 implementation gaps for pipeline architecture
- Fixed spec 206 batched git history implementation
- Fixed parameters with leading underscore to follow Rust conventions
- Resolved state machine pattern detector test failures
- Fixed CI pipeline issues in contextual risk tests
- Resolved test failures in formatter and coverage tests
- Updated domain pattern tests to use new API
- Completed implementation gaps in specs 181i, 201, 202, 203, and 204
- Removed useless test with assert!(true) from concise_recommendation
- Fixed file-level debt item formatting and display
- Updated color validation tests after formatter refactoring
- Fixed god_object_config_rs_test to reference existing files after refactoring

### Internal

- Added spec 211 for Interactive TUI Results Explorer
- Added specs 207-209 for Stillwater pipeline refactoring
- Added comprehensive module documentation for behavioral_decomposition
- Added composable pipeline architecture documentation to ARCHITECTURE.md
- Created example demonstrating effect composition patterns
- Applied automated code formatting across the codebase
- Applied clippy fixes for better code quality
- Removed completed spec files (181b, 181e, 181h, 206, 210)
- Removed misplaced example files from src/
- Updated comprehensive book documentation to fix drift across all chapters
- Restructured analysis guide into multi-subsection format for better organization
- Added missing chapters for call graph and boilerplate detection

## [0.7.0] - 2025-12-02

### Added

- **State Transition Metrics Display** (Spec 190)
  - Display state transition metrics in all output formats (JSON, table, detailed text)
  - Show state transition patterns in priority recommendations
  - Added metrics for mutation frequency, transition complexity, and state field counts

- **Enhanced State Field Detection** (Spec 202)
  - Pattern-based heuristics for identifying state fields in Rust code
  - Detection of common naming patterns (state, cache, buffer, queue, pool, registry, etc.)
  - Improved accuracy in identifying mutable state and tracking state transitions

### Changed - Strategic Direction

- **Rust-Only Focus**: Debtmap now exclusively supports Rust code analysis
  - Removed Python, JavaScript, and TypeScript language support
  - Enables deeper Rust language integration and more accurate analysis
  - Allows focus on Rust-specific patterns, idioms, and best practices
  - Significantly reduced dependency footprint and complexity

### Removed

- **Python Language Support** (Spec 191)
  - Removed Python AST extraction and analysis
  - Removed Python-specific analyzers: asyncio patterns, dead code detection, exception flow, purity analysis
  - Removed Python complexity calculations and pattern detection
  - Removed Python resource tracking: async resources, circular references, context managers, unbounded collections
  - Removed Python testing analyzers: test detection, assertion patterns, flaky test detection, framework detection
  - Removed Python-specific benchmarks and integration tests
  - Removed 17,000+ lines of Python-specific code

- **JavaScript and TypeScript Support** (Spec 192)
  - Removed JavaScript complexity analysis
  - Removed JavaScript dead code detection
  - Removed JavaScript resource management tracking
  - Removed JavaScript testing analyzers
  - Removed 3,000+ lines of JavaScript/TypeScript code
  - Removed framework pattern detection for web frameworks

- **Multi-Language Infrastructure**
  - Removed cross-language analysis utilities
  - Removed language-agnostic pattern extraction
  - Simplified file walker to focus on `.rs` files
  - Removed language-specific configuration options

### Changed

- **Dependency Updates**
  - Updated `stillwater` from 0.11.0 to 0.13.0 for improved bracket analysis
  - Updated `criterion` from 0.7.0 to 0.8.0 for enhanced benchmarking capabilities

### Improved

- **Architecture Refinement** (Spec 181)
  - Implemented "Pure Core, Imperative Shell" pattern for analyzers
  - Separated I/O operations from pure analysis logic
  - Improved testability and composability of analysis functions
  - Better adherence to functional programming principles

- **Code Quality**
  - Applied Stillwater evaluation recommendations
  - Improved code organization and modularity
  - Enhanced type safety and error handling
  - Reduced cyclomatic complexity in key modules

### Fixed

- **Stillwater API Compatibility**
  - Updated bracket API usage for stillwater 0.13.0 compatibility
  - Ensured correct bracket analysis integration with latest stillwater version

- **Tier Classification Accuracy** (Commit 2b037ce0)
  - Fixed tier classification to use sophisticated scoring metrics instead of raw cyclomatic complexity
  - Tier classification now respects weighted complexity (30% cyclomatic + 70% cognitive), entropy dampening, nesting depth, and other advanced metrics
  - Functions with high cognitive complexity (e.g., cyclo=9, cognitive=16, nesting=4) now correctly appear as T2 (Complex) instead of being hidden as T4 (Maintenance)
  - Resolves false negatives where high-cognitive-load functions were incorrectly filtered from analysis results
  - Example: `reconcile_state()` in examples/rethinking-code-quality-analysis now correctly ranks as #1 priority
  - Impact: Tier classification now aligns with debtmap's core philosophy that cognitive load and nesting matter more than raw cyclomatic complexity

### Benefits of Rust-Only Approach

- **Accuracy**: Native syn-based AST parsing provides 100% Rust language coverage
- **Performance**: Optimized for Rust analysis without multi-language overhead
- **Codebase Size**: Reduced from ~51,000 lines to ~38,000 lines (-25%)
- **Dependencies**: Streamlined dependency tree by removing multi-language parsers
- **Features**: Deep integration with Rust-specific constructs:
  - Trait implementations
  - Macro expansion tracking
  - Lifetime and ownership analysis
  - Async/await pattern detection
  - Property-based test detection
- **Maintenance**: Simpler codebase focused on a single language ecosystem

### Migration Guide

For projects using debtmap on Python, JavaScript, or TypeScript codebases:
- Use debtmap v0.6.0 or earlier for multi-language support
- Consider language-specific alternatives:
  - Python: pylint, radon, vulture
  - JavaScript/TypeScript: ESLint, SonarJS, CodeClimate

For Rust projects:
- No changes required - continue using debtmap as normal
- Expect improved accuracy and new Rust-specific features in future releases

### Documentation

- Updated ARCHITECTURE.md to reflect Rust-only implementation (Spec 193)
- Updated analysis guide to focus on Rust metrics
- Updated getting started guide to remove Python/JS/TS references
- Updated README with Rust-only focus and capabilities
- Updated CONTRIBUTING.md with Rust-specific development guidelines
- All documentation now consistently describes Rust-only capabilities

### Internal

- Added STILLWATER_EVALUATION.md documenting architectural assessment
- Created specs 181-200 for future improvements:
  - Terminal output UX enhancements (specs 194-200)
  - State transition metrics display (spec 190)
  - Additional architectural refinements (specs 182-189)
- Added spec 201 for data flow analysis of state transitions and mutation tracking
- Added spec 202 for enhanced state field detection with pattern-based heuristics
- Applied automated code formatting across the codebase
