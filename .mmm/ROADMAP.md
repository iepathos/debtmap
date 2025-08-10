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
  - Legacy risk mode for backwards compatibility via --legacy-risk
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

## Pending Specs

### Enhancements
- [ ] **Spec 10**: Enhance Complexity Detection with Modern Patterns
  - Detect async/await patterns
  - Identify callback chains and promises
  - Recognize functional composition patterns
  - Account for error handling complexity


- [ ] **Spec 12**: Improve ROI Calculation
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