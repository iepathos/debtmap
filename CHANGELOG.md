# Changelog

All notable changes to debtmap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
