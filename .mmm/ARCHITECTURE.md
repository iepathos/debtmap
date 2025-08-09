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

### Transformers (`src/transformers/`)
Functional data transformations:
- `mod.rs`: Transformation composition
- `filters.rs`: Configurable filter predicates

### IO (`src/io/`)
Side-effectful operations:
- `walker.rs`: File system traversal
- `output.rs`: Result formatting and writing

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