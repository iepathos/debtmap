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
- `rust_call_graph.rs`: Two-pass call graph extraction with type-aware resolution

### Complexity (`src/complexity/`)
Complexity metric calculations:
- `cyclomatic.rs`: Cyclomatic complexity using visitor pattern
- `cognitive.rs`: Cognitive complexity with nesting penalties

### Debt (`src/debt/`)
Technical debt detection:
- `patterns.rs`: Pattern matching for TODOs, FIXMEs
- `duplication.rs`: Content hashing for duplicate detection
- `suppression.rs`: Inline comment suppression parsing and checking
- `smells.rs`: Code smell detection patterns
- `coupling.rs`: Module coupling analysis
- `circular.rs`: Circular dependency detection

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

### Expansion (`src/expansion/`)
Macro expansion for perfect call graph analysis:
- `mod.rs`: Public API and configuration
- `expander.rs`: cargo-expand integration
- `cache.rs`: Expansion caching system
- `source_map.rs`: Source location mapping

### Analysis (`src/analysis/`)
Rust-specific call graph analysis with multi-phase construction:
- `call_graph/mod.rs`: Rust-specific call graph builder and core types (RustCallGraph, RustCallGraphBuilder)
- `call_graph/trait_registry.rs`: Trait dispatch detection and resolution, Visit trait pattern detection
- `call_graph/function_pointer.rs`: Function pointer and closure tracking
- `call_graph/framework_patterns.rs`: Framework pattern recognition, Visit trait exclusion
- `call_graph/cross_module.rs`: Cross-module dependency analysis
- Type tracking integration: Uses TypeTracker and GlobalTypeRegistry for accurate method call resolution based on variable types and field access chains

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