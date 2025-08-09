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

## Current Phase: Foundation
We have completed the initial implementation with core functionality for analyzing Rust and Python code.

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