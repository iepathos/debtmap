---
number: 104
title: Fix Python Call Graph by Tracking Line Numbers
category: foundation
priority: critical
status: draft
dependencies: [103]
created: 2025-09-26
---

# Specification 104: Fix Python Call Graph by Tracking Line Numbers

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: [103 - Fix Python Call Graph Extraction and Testing]

## Context

The Python call graph extraction was partially fixed in spec 103, but a critical bug remains: the `TwoPassExtractor` creates `FunctionId` objects with `line: 0` instead of actual line numbers from the source code. This causes a mismatch with the initial call graph built from metrics, which uses correct line numbers.

Since `FunctionId` uses line numbers as part of its identity (for equality comparison), functions created by the analyzer (with correct line numbers) don't match functions created by the TwoPassExtractor (with line: 0), preventing call graph edges from connecting to their nodes. This results in all Python functions showing 0 callers and 0 callees, making dead code detection and dependency analysis completely broken for Python.

## Objective

Fix the Python call graph by implementing proper line number tracking in the TwoPassExtractor and all Python analysis components, ensuring FunctionId objects are created with accurate source line numbers that match those from the initial metrics extraction.

## Requirements

### Functional Requirements

1. **Line Number Extraction from AST**
   - Extract accurate line numbers from Python AST nodes
   - Support all statement types: FunctionDef, AsyncFunctionDef, ClassDef
   - Handle nested functions and class methods correctly
   - Account for decorators when determining function start line

2. **TwoPassExtractor Enhancement**
   - Track line numbers for all function definitions during phase one
   - Store line number mapping alongside function names
   - Use correct line numbers when creating FunctionId objects
   - Maintain line number accuracy through both extraction phases

3. **Consistent FunctionId Creation**
   - Ensure FunctionId objects use the same line number whether created from:
     - Initial metrics extraction
     - TwoPassExtractor
     - PythonCallGraphAnalyzer
     - Any other Python analysis component

4. **Source Location Tracking**
   - Implement a source location tracker that works with rustpython_parser
   - Map AST nodes to their source line numbers
   - Handle multi-line function definitions correctly
   - Support Python 3.6+ syntax accurately

### Non-Functional Requirements

1. **Performance**
   - Line number tracking should add < 5% overhead to analysis time
   - Memory usage for line mapping should be minimal
   - No regression in existing analysis speed

2. **Accuracy**
   - Line numbers must match exactly between all components
   - Support for all Python syntax variants
   - Correct handling of edge cases (lambdas, nested functions, decorators)

3. **Maintainability**
   - Clear abstraction for line number extraction
   - Reusable utilities for AST location tracking
   - Well-documented line number conventions

## Acceptance Criteria

- [ ] TwoPassExtractor creates FunctionIds with correct line numbers
- [ ] FunctionIds from metrics match FunctionIds from call graph extraction
- [ ] Python functions show correct caller/callee relationships
- [ ] test_real_scenario.py shows proper call graph connections
- [ ] promptconstruct-frontend analysis shows accurate caller counts
- [ ] All existing Python tests pass with line number tracking
- [ ] New test suite validates line number accuracy
- [ ] No performance regression (< 5% overhead)
- [ ] Documentation updated with line number tracking details

## Technical Details

### Implementation Approach

1. **Add Line Number Extraction Utilities**
```rust
// New module: src/analysis/python_source_mapping.rs
pub struct SourceMapper {
    source: String,
    line_starts: Vec<usize>,
}

impl SourceMapper {
    pub fn new(source: &str) -> Self {
        // Build line start offset map
    }

    pub fn get_line_number(&self, node: &ast::Stmt) -> usize {
        // Extract line number from AST node
    }
}
```

2. **Enhance TwoPassExtractor**
```rust
pub struct TwoPassExtractor {
    // ... existing fields ...
    source_mapper: Option<SourceMapper>, // NEW
    function_lines: HashMap<String, usize>, // NEW
}

impl TwoPassExtractor {
    pub fn new_with_source(file_path: PathBuf, source: &str) -> Self {
        Self {
            // ... existing initialization ...
            source_mapper: Some(SourceMapper::new(source)),
            function_lines: HashMap::new(),
        }
    }

    fn analyze_function_phase_one(&mut self, func_def: &ast::StmtFunctionDef) {
        let line = self.source_mapper.as_ref()
            .map(|mapper| mapper.get_line_number(func_def))
            .unwrap_or(0);

        let func_id = FunctionId {
            name: func_name.clone(),
            file: self.type_tracker.file_path.clone(),
            line, // Use actual line number
        };

        self.function_lines.insert(func_name.clone(), line);
        // ... rest of implementation
    }
}
```

3. **Update Call Graph Building**
```rust
// In process_python_files_for_call_graph_with_types
for file_path in &python_files {
    match io::read_file(file_path) {
        Ok(content) => {
            match rustpython_parser::parse(&content, ...) {
                Ok(module) => {
                    // Pass source content to extractor
                    let mut extractor = TwoPassExtractor::new_with_source(
                        file_path.to_path_buf(),
                        &content, // NEW: pass source
                    );
                    let file_call_graph = extractor.extract(&module);
                    call_graph.merge(file_call_graph);
                }
                // ... error handling
            }
        }
    }
}
```

### Architecture Changes

1. **New Module**: `src/analysis/python_source_mapping.rs`
   - Provides utilities for mapping AST nodes to source locations
   - Reusable across all Python analysis components

2. **Enhanced TwoPassExtractor**
   - Accepts source content during initialization
   - Tracks line numbers throughout extraction
   - Maintains function-to-line mapping

3. **Updated Python Analyzer**
   - Ensure consistent line number extraction
   - Use same source mapping utilities

### Data Structures

```rust
/// Line number mapping for Python functions
pub struct FunctionLineMap {
    /// Maps fully-qualified function names to their line numbers
    functions: HashMap<String, usize>,
    /// Source file path
    file_path: PathBuf,
}

/// Enhanced with line tracking
pub struct PythonFunctionInfo {
    pub name: String,
    pub qualified_name: String,
    pub line: usize,
    pub is_method: bool,
    pub is_async: bool,
    pub parent_class: Option<String>,
}
```

## Dependencies

- **Prerequisites**: Spec 103 (initial Python call graph fix)
- **Affected Components**:
  - `src/analysis/python_type_tracker.rs`
  - `src/analysis/python_call_graph/`
  - `src/analyzers/python.rs`
  - `src/builders/call_graph.rs`
- **External Dependencies**: rustpython_parser (existing)

## Testing Strategy

### Unit Tests
- Test line number extraction for various Python constructs
- Verify FunctionId equality with correct line numbers
- Test source mapping utilities with edge cases
- Validate line tracking through both extraction phases

### Integration Tests
- Test complete flow from source to call graph
- Verify caller/callee relationships are established
- Test with real Python projects
- Validate against known call graphs

### Regression Tests
- Ensure existing Python analysis still works
- Verify no performance degradation
- Test backward compatibility

### Test Cases
```python
# Test various function definition styles
def simple_function():  # Line 1
    pass

@decorator  # Line 4
def decorated_function():  # Line 5
    pass

class MyClass:  # Line 8
    def method(self):  # Line 9
        def nested():  # Line 10
            pass

async def async_func():  # Line 13
    await something()

lambda_func = lambda x: x + 1  # Line 16
```

## Documentation Requirements

### Code Documentation
- Document line number extraction algorithm
- Explain FunctionId matching strategy
- Add examples of proper usage

### User Documentation
- Update Python analysis documentation
- Add troubleshooting for line number issues
- Include examples of correct output

### Architecture Updates
- Document python_source_mapping module
- Update Python analysis flow diagram
- Add sequence diagram for line tracking

## Implementation Notes

1. **Rustpython_parser Limitations**
   - The parser may not provide direct line numbers for all nodes
   - May need to use the `range` field or implement custom tracking
   - Consider using the `location` module if available

2. **Edge Cases**
   - Multi-line function signatures
   - Functions with complex decorators
   - Lambda expressions (may not have traditional line numbers)
   - Class methods vs standalone functions

3. **Compatibility Considerations**
   - Ensure line numbers match between different Python versions
   - Handle differences in AST structure across Python 3.x versions
   - Account for line ending differences (LF vs CRLF)

4. **Performance Optimization**
   - Cache line number mappings per file
   - Avoid redundant source parsing
   - Use efficient data structures for lookups

## Migration and Compatibility

This fix is backward compatible in terms of API but will change the behavior:
- Existing code will continue to compile
- Python call graphs will now be populated correctly
- JSON output will contain accurate caller/callee data
- No breaking changes to public interfaces

## Validation Approach

1. **Before Fix**
```bash
./target/debug/debtmap analyze test_real_scenario.py -f json | jq '.items[0]'
# Shows: "upstream_callers": [], "downstream_callees": []
```

2. **After Fix**
```bash
./target/debug/debtmap analyze test_real_scenario.py -f json | jq '.items[0]'
# Shows: "upstream_callers": ["MainFrame.__init__"], "downstream_callees": ["print"]
```

3. **End-to-End Validation**
   - Run on promptconstruct-frontend project
   - Verify MainFrame init methods show __init__ as caller
   - Check that on_key_down shows proper callees
   - Validate that dead code detection is accurate

## Risk Assessment

- **Medium Risk**: Changes core Python analysis logic
- **High Impact**: Fixes critical functionality
- **Testing Required**: Comprehensive test coverage needed
- **Performance**: Minor overhead acceptable for correctness

## Success Metrics

- 100% of Python functions have correct line numbers
- Call graph accuracy improves from 0% to >95%
- Dead code false positive rate drops from 100% to <10%
- No measurable performance regression
- All Python projects analyzed correctly