# Debtmap Architecture

## Design Philosophy
Debtmap follows functional programming principles with a clear separation between pure functions and IO operations.

## Core Principles
1. **Functional Core / Imperative Shell**: Pure functions for analysis, IO at boundaries
2. **Immutability**: Using persistent data structures from `im` crate
3. **Composition**: Building complex analyzers from simple functions
4. **Parallelism**: Leveraging rayon for concurrent file processing
5. **Type Safety**: Strong typing with Result/Option for error handling

## Module Structure

### Core (`src/core/`)
Pure data types and structures:
- `mod.rs`: Core types (AnalysisResults, FileMetrics, etc.)
- `ast.rs`: AST representation and transformations
- `metrics.rs`: Metric calculation functions

### Analyzers (`src/analyzers/`)
Language-specific analysis implementations:
- `mod.rs`: Analyzer trait and composition functions
- `rust.rs`: Rust AST parsing with syn
- `python.rs`: Python AST parsing with rustpython-parser
- `type_tracker.rs`: AST-based type tracking for accurate method resolution
- `type_registry.rs`: Global type registry for struct definitions and field tracking
- `function_registry.rs`: Function signature registry for return type tracking
- `signature_extractor.rs`: AST visitor for extracting function and method signatures
- `rust_call_graph.rs`: Two-pass call graph extraction with type-aware and signature-aware resolution
- `trait_implementation_tracker.rs`: Comprehensive trait definition and implementation tracking
- `trait_resolver.rs`: Dynamic dispatch resolution with method resolution order
- `context_aware.rs`: Context-aware analyzer wrapper for false positive reduction

### Complexity (`src/complexity/`)
Complexity metric calculations:
- `cyclomatic.rs`: Cyclomatic complexity using visitor pattern
- `cognitive.rs`: Cognitive complexity with nesting penalties and pattern adjustments
- `entropy.rs`: Entropy-based complexity scoring using information theory
- `pattern_adjustments.rs`: Pattern-specific complexity adjustments (pattern matching, simple delegation)
- `patterns.rs`: Modern pattern complexity detection
- `visitor_detector.rs`: AST-based visitor pattern detection with logarithmic complexity scaling
- `recursive_detector.rs`: Recursive AST traversal for finding all match expressions
- `threshold_manager.rs`: Configurable complexity thresholds with role-based multipliers
- `message_generator.rs`: Enhanced message generation with specific recommendations
- `if_else_analyzer.rs`: If-else chain detection and refactoring pattern suggestions

### Debt (`src/debt/`)
Technical debt detection:
- `patterns.rs`: Pattern matching for TODOs, FIXMEs
- `duplication.rs`: Content hashing for duplicate detection
- `suppression.rs`: Inline comment suppression parsing and checking
- `smells.rs`: Code smell detection patterns
- `coupling.rs`: Module coupling analysis
- `circular.rs`: Circular dependency detection
- `error_swallowing.rs`: Error swallowing anti-pattern detection


### Context (`src/context/`)
Context-aware detection for false positive reduction:
- `mod.rs`: Core context types (FunctionContext, FileType, FunctionRole)
- `detector.rs`: AST-based context detection for functions
- `rules.rs`: Context-aware rules engine for debt filtering

### Performance (`src/performance/`)
Performance anti-pattern detection with smart context analysis and optimized AST traversal:
- `mod.rs`: Core performance types and conversion functions
- `unified_visitor.rs`: Single-pass AST traversal for collecting all performance data
- `collected_data.rs`: Data structures for storing collected performance information
- `detector_adapter.rs`: Adapters for detectors to analyze pre-collected data
- `io_detector.rs`: I/O pattern detection
- `nested_loop_detector.rs`: Nested loop complexity detection
- `allocation_detector.rs`: Memory allocation pattern detection
- `data_structure_detector.rs`: Inefficient data structure usage detection
- `string_detector.rs`: String processing anti-pattern detection
- `smart_detector.rs`: Smart context-aware performance detection
- `pattern_correlator.rs`: Multi-pattern correlation analysis
- `location_extractor.rs`: Source location extraction utilities
- `context/`: Context analysis framework
  - `mod.rs`: Core context types and traits
  - `module_classifier.rs`: Module type classification (test, production, utility)
  - `intent_classifier.rs`: Function intent analysis (setup, teardown, business logic)
  - `severity_adjuster.rs`: Context-based severity adjustment

### Transformers (`src/transformers/`)
Functional data transformations:
- `mod.rs`: Transformation composition
- `filters.rs`: Configurable filter predicates

### Risk (`src/risk/`)
Risk analysis and coverage correlation:
- `mod.rs`: Risk types and analyzer with strategy pattern
- `strategy.rs`: Risk calculation strategies (Enhanced and Legacy)
- `lcov.rs`: LCOV coverage file parsing
- `correlation.rs`: Complexity-coverage correlation analysis
- `priority.rs`: Advanced multi-stage test prioritization pipeline
  - PrioritizationPipeline: Orchestrates prioritization stages
  - ZeroCoverageStage: Boosts untested modules
  - CriticalPathStage: Scores module criticality
  - ComplexityRiskStage: Factors in complexity risks
  - DependencyImpactStage: Analyzes dependency impacts
  - EffortOptimizationStage: Optimizes for ROI
  - ROICalculator: Dynamic ROI calculation with cascade effects
  - EffortEstimator: Realistic effort estimation
- `insights.rs`: Risk insight generation and formatting
- `context/`: Context-aware risk analysis
  - `mod.rs`: Context provider trait and aggregator
  - `critical_path.rs`: Critical path analysis from entry points
  - `dependency.rs`: Dependency risk propagation
  - `git_history.rs`: Git history and change frequency analysis
- `evidence/`: Evidence-based risk assessment (spec 24)
  - `mod.rs`: Core risk types and evidence structures
  - `complexity_analyzer.rs`: Role-aware complexity risk analysis
  - `coverage_analyzer.rs`: Test coverage gap risk analysis
  - `coupling_analyzer.rs`: Module coupling and dependency risk
  - `change_analyzer.rs`: Change frequency and hotspot analysis
- `thresholds/`: Statistical risk thresholds
  - `mod.rs`: Baseline distributions and percentile-based thresholds
- `evidence_calculator.rs`: Main evidence-based risk calculator

### Priority (`src/priority/`)
Unified debt prioritization system:
- `mod.rs`: Core types for unified analysis
- `call_graph.rs`: Function call graph construction, analysis, and dead code detection
- `semantic_classifier.rs`: Function role classification (PureLogic, Orchestrator, etc.)
- `coverage_propagation.rs`: Transitive coverage calculation through call graph
- `unified_scorer.rs`: Unified priority scoring algorithm combining all metrics, includes dead code detection
- `formatter.rs`: Clean output formatters for different verbosity levels
- `debt_aggregator.rs`: Aggregates all detected debt issues by function location and calculates debt scores

### Expansion (`src/expansion/`)
Macro expansion for perfect call graph analysis:
- `mod.rs`: Public API and configuration
- `expander.rs`: cargo-expand integration
- `cache.rs`: Expansion caching system
- `source_map.rs`: Source location mapping

### Testing (`src/testing/`)
Testing quality pattern detection:
- `mod.rs`: Core types and trait for testing anti-patterns
- `assertion_detector.rs`: Detects tests without assertions
- `complexity_detector.rs`: Identifies overly complex tests
- `flaky_detector.rs`: Finds flaky test patterns (timing, randomness, external deps)

### Analysis (`src/analysis/`)
Rust-specific call graph analysis with multi-phase construction:
- `call_graph/mod.rs`: Rust-specific call graph builder and core types (RustCallGraph, RustCallGraphBuilder)
- `call_graph/trait_registry.rs`: Enhanced trait dispatch detection and resolution, Visit trait pattern detection, integration with trait_implementation_tracker
- `call_graph/function_pointer.rs`: Function pointer and closure tracking
- `call_graph/framework_patterns.rs`: Framework pattern recognition, Visit trait exclusion
- `call_graph/cross_module.rs`: Cross-module dependency analysis
- Type tracking integration: Uses TypeTracker, GlobalTypeRegistry, FunctionSignatureRegistry, and TraitImplementationTracker for accurate method call resolution based on variable types, field access chains, function return types, and trait implementations

### IO (`src/io/`)
Side-effectful operations:
- `walker.rs`: File system traversal
- `output.rs`: Result formatting and writing (includes risk insights)

## Data Flow
```
Files → Parse → AST → Transform → Analyze → Filter → Output
         ↓                ↓           ↓        ↓
      (Pure)          (Pure)      (Pure)   (Pure)   (IO)
```

## Key Design Patterns
1. **Visitor Pattern**: For AST traversal
2. **Builder Pattern**: For configuration
3. **Strategy Pattern**: For output formats
4. **Functional Composition**: For pipeline construction

## Performance Considerations
- Parallel file processing with rayon
- Lazy evaluation where possible
- Efficient persistent data structures
- Content hashing for duplication detection