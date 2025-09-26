# Python Analysis Documentation

## Overview

Debtmap provides comprehensive static analysis for Python codebases, including complexity metrics, call graph extraction, type tracking, and technical debt detection.

## Call Graph Extraction

The Python call graph extraction uses a two-pass algorithm to accurately resolve function and method calls:

### Phase 1: Type Information Collection
- Extracts class hierarchies and method definitions
- Tracks variable assignments and type annotations
- Collects all function/method calls for later resolution
- Records line numbers for all function definitions

### Phase 2: Call Resolution
- Resolves method calls using type information
- Links function calls to their definitions
- Maintains accurate line numbers for both callers and callees

## Line Number Tracking

### Algorithm

The line number extraction system accurately determines the source location of Python functions and methods:

1. **Source-based extraction**: When analyzing Python files, the full source content is provided to the analyzer
2. **Pattern matching**: The `estimate_line_number` function searches for function definitions using these patterns:
   - `def function_name(` for regular functions
   - `async def function_name(` for async functions
   - Handles indented methods within classes
   - Correctly identifies decorated functions

3. **Edge case handling**:
   - Comments containing `def` are ignored
   - String literals with `def` are not matched
   - Multi-line function signatures are handled by finding the line with the `def` keyword
   - Nested functions are correctly identified

### FunctionId Matching Strategy

The system uses a two-level approach for matching function identities:

1. **Type Tracker**: Maintains a mapping of class methods with their qualified names (e.g., `Calculator.reset`)
2. **Call Graph Extractor**:
   - Creates FunctionId objects with accurate line numbers during the initial parse
   - When resolving method calls, looks up the FunctionId with the correct line number from its internal map
   - This ensures that call graph connections use accurate source locations

### Example

```python
class Calculator:
    def __init__(self):        # Line 3
        self.value = 0
        self.reset()           # Call to line 7

    def reset(self):          # Line 7
        self.value = 0
```

The system will:
1. Extract `Calculator.__init__` at line 3
2. Extract `Calculator.reset` at line 7
3. Identify the call from `__init__` to `reset`
4. Store the connection with accurate line numbers

## Type Tracking

The Python type tracker (`PythonTypeTracker`) maintains:

- **Class hierarchies**: Inheritance relationships between classes
- **Method resolution**: Finding methods in class hierarchies including inherited methods
- **Type inference**: Determining types from:
  - Literals (strings, numbers, booleans)
  - Type annotations
  - Constructor calls
  - Variable assignments

## Technical Debt Detection

Python-specific debt patterns detected include:

- **Long functions**: Functions exceeding recommended line counts
- **Complex conditionals**: Deeply nested if/elif/else chains
- **Missing type hints**: Functions without type annotations
- **Broad exception handling**: Catching bare `Exception` or using empty except blocks
- **Code duplication**: Similar code patterns across files

## Testing

The Python analysis module includes comprehensive test coverage:

- **Unit tests** for individual components:
  - `estimate_line_number` function with various Python syntax patterns
  - `new_with_source` constructor initialization
  - Type inference and resolution

- **Integration tests** for end-to-end scenarios:
  - Call graph extraction with accurate line numbers
  - Method resolution in class hierarchies
  - Cross-module function calls

## Implementation Details

### Key Components

1. **TwoPassExtractor** (`src/analysis/python_type_tracker.rs`):
   - Main entry point for Python call graph extraction
   - Manages the two-phase extraction process
   - Maintains source line information for accurate line number extraction

2. **PythonTypeTracker** (`src/analysis/python_type_tracker.rs`):
   - Tracks type information during analysis
   - Resolves method calls based on receiver types
   - Maintains class hierarchy information

3. **Python Parser** (`src/analyzers/python.rs`):
   - Uses `rustpython_parser` for AST generation
   - Extracts various metrics from Python code
   - Integrates with the call graph extraction system

### Performance Considerations

- **Lazy evaluation**: Line numbers are computed on-demand during function registration
- **Caching**: Function name to ID mappings are cached to avoid repeated lookups
- **Memory efficiency**: Source lines are stored as a vector of strings for quick access

## Future Improvements

Potential enhancements to Python analysis:

1. **Advanced type inference**: Support for generic types and type variables
2. **Dynamic analysis integration**: Combining static analysis with runtime information
3. **Import tracking**: Following imports across modules for better call resolution
4. **Decorator analysis**: Understanding decorator effects on function behavior
5. **Async/await tracking**: Special handling for asynchronous call flows