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