# Changelog

All notable changes to debtmap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.13.6] - 2026-02-14

### Added

- **God Object Ignore Keyword** - New `god_object` keyword for suppression annotations
  - `// debtmap:ignore[god_object] - intentionally large coordinator`
  - Specifically targets god object debt items

### Changed

- **Function-Level Suppression Syntax** - Changed from `debtmap:allow` to `debtmap:ignore`
  - More consistent with existing `debtmap:ignore-start/end` block syntax
  - Accepts both single dash (`-`) and double dash (`--`) as reason separator
  - Example: `// debtmap:ignore[testing] - I/O dispatcher function`

### Fixed

- **Suppression Doc Comment Support** - Recognize `///` doc comments for annotations
  - Previously only `//` regular comments were recognized
  - Now `/// debtmap:ignore[testing] - reason` works correctly

- **Complexity Keyword Suppression** - `complexity` keyword now suppresses both debt types
  - Matches both `Complexity` and `ComplexityHotspot` debt types
  - Added explicit `hotspot` keyword for hotspot-only suppression

- **Markdown Git Context Output** - Aligned with TUI format for consistency
  - Added git context section to priority markdown formatter
  - Shows activity, stability, fix rate, age, contributors, and risk multiplier
  - Terminal formatter now shows commit counts instead of just changes/month

- **Allow Annotations in Coverage Mode** - Function-level suppressions now work correctly
  - `debtmap:ignore[testing]` properly filters items in `--lcov` coverage mode

- **TUI Item Scores Header** - Fixed inconsistent header styling in detail view

- **Context Test File References** - Only include existing files in context suggestions
  - Filters out hypothetical `*_test.rs` paths that don't exist
  - Prevents misleading references when tests are inline modules

### Security

- **RUSTSEC-2026-0002** - Updated ratatui 0.28 → 0.30 to fix vulnerability
- **RUSTSEC-2022-0081** - Removed unused cargo-modules dependency
- **Unmaintained bincode** - Replaced bincode with postcard for serialization

### Dependencies

- Updated minimum dependency versions for better compatibility
- cc: Updated to newer version
- libc: Updated to newer version

### Internal (Refactoring)

- **CLI Module Extraction** - Extracted analyze and validate-improvement handlers
- **TUI Simplification** - Simplified ResultsApp structure with better separation
- **Formatter Refactoring** - Extracted tier header and blast radius helpers
- **Suppression System** - Consolidated function-level ignore handling
- **Organization Module** - Converted to iterator chains for functional style
- **Profiling Extraction** - Separated profiling concern from main_inner

### Testing

- Added format_file_rationale tests
- Added TUI navigation helper tests
- Added missing dispatcher action tests
- Added call graph tests for async and same-file call detection

### Tech Debt Cleanup

- Added `debtmap:ignore` annotations to intentionally complex functions:
  - `with_retry` - retry orchestration
  - `build_adjustment_steps` - multi-phase calculation
  - `build_page_lines` - rendering coordinator
  - `execute_list_action` - action dispatcher
  - `execute_detail_action` - detail view dispatcher
  - `format_file_priority_item_with_verbosity` - formatting coordinator
  - `from (crate::core::errors::Error)` - error conversion

## [0.13.5] - 2026-02-11

### Fixed

- **Role Multiplier Inversion** - Corrected multipliers for accurate debt prioritization
  - PureLogic: 1.2 → 0.7 (pure functions are easier to test, lower priority)
  - IOWrapper: 0.5 → 1.2 (I/O is harder to test, higher priority)
  - EntryPoint: 1.5 → 1.3 (entry points need integration tests)

- **I/O Pattern Detection** - Added missing patterns for function classification
  - Process I/O: spawn, command, process, child, exec
  - Async I/O: timeout, async, await, tokio, future
  - Buffered I/O: bufreader, bufwriter

- **Purity-Based Role Classification** - Impure functions no longer default to PureLogic
  - Functions detected as impure now classified as Unknown (neutral multiplier)
  - Prevents I/O orchestration functions from being misclassified

- **Context Test File References** - Only include existing files in context suggestions
  - Filters out hypothetical `*_test.rs` paths that don't exist
  - Prevents misleading references when tests are inline modules

- **TUI Item Scores Header** - Fixed inconsistent header styling in detail view

### Added

- **Function-Level Debt Suppression** - `debtmap:ignore` annotation support
  - Suppress specific functions from debt reports with inline comments
  - Useful for intentionally complex functions or false positives

### Testing

- Added call graph tests for async and same-file call detection
- Validates tokio::spawn closure call attribution

## [0.13.4] - 2026-02-08

### Performance

- **Git Commit Processing** - Parallelize commit processing with rayon for faster analysis
- **Git Diff Stats** - Use `diff.foreach()` for O(N) stats calculation instead of O(N²)

### Fixed

- **Windows Release Builds** - Install OpenSSL via vcpkg for Windows release workflow

### Documentation

- **LLMs.txt** - Fixed inaccurate language and scoring claims

## [0.13.3] - 2026-02-08

### Added

- **TUI Content Scrolling** - Detail view now supports scrolling through content with arrow keys
  - Enables viewing long debt item details without truncation

### Fixed

- **TUI Navigation** - Arrow keys now correctly navigate pages in detail view
- **Windows CI Builds** - Install OpenSSL via vcpkg for Windows build compatibility
- **Dependency Compatibility** - Added minimum `time` version constraint for CI stability

### Changed

- **Dependencies** - Updated Cargo.lock with latest dependency versions
  - memchr: 2.7.6 → 2.8.0
  - Updated various dependency version constraints

### Internal (Refactoring)

- **God Object Analyzer** - Split god object files into focused modules
- **Complexity Analysis** - Extracted pure functions from `detect_match_expression`
- **General Refactoring** - Extracted pure functions with improved test coverage

### Testing

- Added comprehensive tests for `is_function_used_by_pattern`
- Added coverage for scoring, main CLI, and TUI navigation
- Added edge case tests for `ensure_balanced_distribution`
- Added coverage for TUI actions and navigation
- Added tests for `From<CoreError>` implementation
- Added coverage for git search functionality
- Ignored slow trybuild UI test by default for faster CI

### Infrastructure

- Added `--context` flag to debtmap validate commands in CI
- Applied cargo fmt formatting

## [0.13.2] - 2026-02-06

### Added

- **LLM Integration Resources**
  - Added `llms.txt` following the emerging standard for LLM-friendly documentation
  - Added `debtmap-analyst` Claude skill for AI-assisted code analysis
  - Skill includes debtmap usage guidance and Rust/FP refactoring patterns

### Changed

- **Dependencies** - Updated Cargo.lock with latest dependency versions
  - trybuild: 1.0.114 → 1.0.115

### Internal

- Consolidated typos configuration into `_typos.toml` with cleaner formatting
- Added file exclusions for `*.lock` and `target/` directory in typos config

## [0.13.1] - 2026-01-30

### Fixed

- **Git Context** - Fix zero git metrics when analyzing from subdirectories
  - GitHistoryProvider now uses git2's canonical workdir as repo_root
  - Path resolution handles symlinks (e.g., macOS /var → /private/var)
  - Fixes change frequency, bug density, age, and author metrics

### Changed

- **Dependencies** - Updated Cargo.lock with latest dependency versions
  - clap: 4.5.54 → 4.5.56
  - openssl-src: 300.5.4+3.5.4 → 300.5.5+3.5.5
  - zerocopy: 0.8.34 → 0.8.36

## [0.13.0] - 2026-01-24

### Added

- **Visual Dashboard** - Interactive D3.js dashboard for exploring analysis results
  - Risk Quadrant visualization plotting functions by complexity vs coverage gap
  - Churn-based bubble sizing shows frequently changed code
  - Top Debt Items sortable table
  - Module Flow chord diagram
  - Risk Radar multi-dimensional comparison
  - Hosted at https://iepathos.github.io/debtmap/dashboard/
  - Also available locally via `viz-dev/dashboard.html`

- **File-Level debt_type** - File items now include `debt_type` field in JSON output
  - Enables consistent filtering across function and file items

- **Smart Tooltip Positioning** - Dashboard tooltips now stay within viewport bounds

### Changed

- **Removed `--format html` output** - Use the online dashboard with JSON output instead
  - Generate JSON: `debtmap analyze . --format json -o debtmap.json`
  - Load in dashboard: https://iepathos.github.io/debtmap/dashboard/
  - All processing happens client-side for privacy

### Fixed

- **Dashboard Display Consistency** - Dashboard now matches TUI behavior for debt item display
- **TUI Context Page Alignment** - Fixed "context to read" label alignment to start at column 0 like other section headers
- **Test Race Condition** - Fixed validation test race condition
- **Dashboard Auto-load** - Dashboard no longer tries to auto-fetch on startup

### Internal

- Extracted pure functions from modules for improved testability and maintainability
- Extracted tests and helpers in priority module
- Deployed dashboard to GitHub Pages via docs workflow

## [0.12.1] - 2026-01-15

### Fixed

- **Duplicate Config Struct** - Removed duplicate `OutputConfig` struct definition

### Internal (Refactoring)

- **Complexity Reduction** - Reduced complexity across multiple modules
  - Extracted pure helpers for complexity calculations
  - Reduced complexity in `main_inner` and `find_cross_file_implementations`
  - Fixed git context handling during refactoring
- **Progress System** - Extracted progress helpers in unified_analysis module
- **Pure Function Extraction** - Continued extracting pure helpers across modules for better testability

## [0.12.0] - 2026-01-13

### Fixed

- **Zero-Score Filtering** - Items with zero score are now properly excluded from results
  - God objects with zero score no longer appear in output
  - All query types now correctly exclude zero-score items
  - Prevents noise from non-actionable debt items

- **Compare Command Input Types** - Fixed input parsing to use output format types
  - Compare command now correctly parses JSON input files
  - Improved type consistency between output and input formats

- **Test Caller Filtering for Production Blast Radius** (Spec 267)
  - Fixed path-based test attribute detection (`#[tokio::test]`, `#[actix_rt::test]`)
    - `syn::Path::is_ident()` only matches single identifiers, not paths
    - Now properly checks path segments for async test framework attributes
  - Fixed caller classification path mismatch
    - Caller strings have short paths (`overflow.rs:func`) but call graph has full paths
    - Added name-based fallback when path-based lookup fails
  - Fixed module-qualified name matching
    - Call graph stores `test::func_name` but caller strings only have `func_name`
    - Now matches if function name ends with `::name` pattern
  - Fixed god object caller classification
    - File-scope debt items were bypassing caller classification entirely
    - Now properly separates production and test callers for god objects
  - Result: Production blast radius now correctly excludes test callers
    - Example: overflow.rs went from 129 (90+39) to 55 (16+39) after filtering 74 test callers

- **Score Dampening for Well-Tested Stable Cores** (Spec 269)
  - Fixed coupling classification threshold from `< 0.3` to `<= 0.35`
    - Instability of 0.3023 displays as "0.30" but failed strict `< 0.3` check
    - Now correctly classifies stable modules with borderline instability
  - Applied score dampening for WellTestedCore classification in god objects
    - Well-tested stable cores (instability <= 0.35, 70%+ test callers) now get 80% score reduction
    - These foundations should not appear as priority debt items
  - Result: Well-tested stable files no longer flagged as high-priority debt
    - Example: overflow.rs (74 test / 16 production callers) now scores 10 (Low) instead of 50 (High)

- **Dependency Compatibility**
  - Set minimum tokio version to 1.21 for `JoinSet` support
  - Ensures compatibility with async runtime features

### Changed

- **MSRV Bump to 1.89** - Updated minimum supported Rust version to match toolchain
  - Aligns with current stable Rust features
  - Enables use of newer language features

### Infrastructure

- Updated `actions/setup-python` from v5 to v6
- Updated `github/codeql-action` from v3 to v4

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
