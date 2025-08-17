---
number: 42
title: Python Callback Function Tracking for Dead Code Detection
category: compatibility
priority: high
status: draft
dependencies: [38]
created: 2025-08-17
---

# Specification 42: Python Callback Function Tracking for Dead Code Detection

**Category**: compatibility
**Priority**: high
**Status**: draft
**Dependencies**: spec 38 (Multi-Language Detector Support - Foundation)

## Context

The current Python call graph analysis in debtmap produces false positives when detecting dead code for nested functions that are passed as callbacks to framework functions like `wx.CallAfter`. This pattern is common in GUI frameworks, async libraries, and event-driven systems where functions are not called directly but are passed as arguments to be executed later.

The recent improvements to line number tracking in commit 0518d75d93694f8ca32052ad2d386ca3559b8b2b provide the foundation for accurate source location mapping, which this specification will leverage to correctly track these callback patterns.

### Current Issue
```python
class DeliveryBoy:
    def deliver_message_added(self, observers, message, index):
        def deliver(observers, message, index):  # Currently flagged as dead code
            for observer in observers:
                observer.on_message_added(message, index)

        wx.CallAfter(deliver, observers, message, index)  # Function reference not tracked
```

The nested `deliver` function is incorrectly flagged as dead code because the call graph analyzer doesn't recognize that passing a function as an argument to `wx.CallAfter` constitutes usage.

## Objective

Enhance the Python call graph analyzer to detect when nested functions are passed as arguments to callback-accepting functions, eliminating false positives in dead code detection for common framework patterns while maintaining accurate line number tracking.

## Requirements

### Functional Requirements
- **Callback Pattern Recognition**: Detect when functions are passed as arguments to known callback-accepting functions
- **Nested Function Tracking**: Properly track nested function definitions and their qualified names
- **Framework Pattern Support**: Support common callback patterns from wxPython, asyncio, threading, and other frameworks
- **Line Number Accuracy**: Leverage improved line tracking to provide exact source locations for callback references
- **Cross-Module Support**: Handle callback patterns in both class methods and standalone functions

### Non-Functional Requirements
- **Performance**: Analysis overhead should be minimal (< 5% increase in Python analysis time)
- **Accuracy**: Reduce false positives for callback patterns by 90%+
- **Maintainability**: Use extensible pattern matching for easy addition of new callback frameworks
- **Backward Compatibility**: No breaking changes to existing call graph API

## Acceptance Criteria

- [ ] `wx.CallAfter(nested_function, ...)` patterns no longer produce false positives
- [ ] Nested functions passed to `asyncio.create_task()`, `threading.Timer()`, and similar functions are correctly tracked
- [ ] Generic callback patterns like `scheduler.submit(func)` are detected
- [ ] Line numbers for callback references are accurate (not defaulting to 0)
- [ ] Both class methods and standalone functions with nested callbacks work correctly
- [ ] Test coverage includes all major Python callback patterns
- [ ] Performance regression tests pass with < 5% overhead
- [ ] Integration with existing dead code detection system maintains all current functionality

## Technical Details

### Implementation Approach

#### 1. Enhanced Nested Function Analysis
Extend `PythonCallGraphAnalyzer` to:
- Track nested function definitions during AST traversal
- Build fully qualified names for nested functions (e.g., `Parent.nested_func`)
- Store nested function line numbers using improved line tracking

#### 2. Callback Pattern Detection
Create new methods in `PythonCallGraphAnalyzer`:
- `check_for_callback_patterns()`: Identify calls to callback-accepting functions
- `is_callback_accepting_function()`: Pattern matching for known callback functions
- `track_function_argument()`: Track function references passed as arguments

#### 3. Function Reference Tracking
Implement `add_function_reference()` to:
- Detect when a function name in an argument position refers to a nested function
- Create call graph edges from the containing function to the nested function
- Handle both direct names (`deliver`) and attribute references (`self.method`)

#### 4. Framework Pattern Library
Support callback patterns from:
- **wxPython**: `wx.CallAfter`, `wx.CallLater`, `Bind` event handlers
- **asyncio**: `create_task`, `ensure_future`, `run_in_executor`
- **threading**: `Timer`, `Thread` target functions
- **multiprocessing**: `Process` target functions, `Pool.apply_async`
- **Generic**: `schedule`, `queue`, `defer`, `setTimeout` patterns

### Architecture Changes

#### Modified Components
1. **`src/analysis/python_call_graph.rs`**:
   - Add callback pattern detection to `analyze_call_expr()`
   - Implement nested function context tracking
   - Enhanced function argument analysis

2. **`src/analyzers/python.rs`**:
   - Integration point for enhanced call graph analysis
   - Ensure proper line number propagation

#### New Data Structures
```rust
struct CallbackPattern {
    function_name: String,
    module_name: Option<String>,
    argument_position: usize,  // Which argument is the callback
}

struct NestedFunctionContext {
    parent_function: String,
    nested_functions: HashMap<String, usize>,  // name -> line number
}
```

### APIs and Interfaces

#### Enhanced PythonCallGraphAnalyzer Methods
```rust
impl PythonCallGraphAnalyzer {
    fn check_for_callback_patterns(&mut self, call_expr: &ast::ExprCall, ...) -> Result<()>
    fn is_callback_accepting_function(&self, func_name: &str) -> bool
    fn track_function_argument(&mut self, arg: &ast::Expr, ...) -> Result<()>
    fn add_function_reference(&mut self, func_name: &str, ...) -> Result<()>
    fn build_nested_function_name(&self, func_name: &str) -> String
}
```

#### Callback Pattern Configuration
```rust
const CALLBACK_PATTERNS: &[CallbackPattern] = &[
    CallbackPattern { function_name: "CallAfter", module_name: Some("wx"), argument_position: 0 },
    CallbackPattern { function_name: "create_task", module_name: Some("asyncio"), argument_position: 0 },
    // ... additional patterns
];
```

## Dependencies

- **Prerequisites**: 
  - Spec 38: Multi-Language Detector Support foundation provides the architecture
  - Recent line number tracking improvements (commit 0518d75d) provide accurate source locations

- **Affected Components**: 
  - `src/analysis/python_call_graph.rs`: Core implementation
  - `src/analyzers/python.rs`: Integration point
  - Python dead code detection in unified scorer

- **External Dependencies**: None (uses existing rustpython-parser)

## Testing Strategy

### Unit Tests
- **Callback Pattern Recognition**: Test detection of various callback patterns
- **Nested Function Tracking**: Verify correct qualified name generation
- **Line Number Accuracy**: Ensure callback references use correct line numbers
- **Edge Cases**: Test deeply nested functions, multiple callbacks, complex argument patterns

### Integration Tests
- **Real-World Examples**: Test actual wxPython, asyncio, and threading code patterns
- **Performance Benchmarks**: Verify < 5% overhead on large Python codebases
- **Dead Code Integration**: Ensure proper integration with existing dead code detection

### Test Cases
```python
# Test case 1: wxPython callback
class TestWxCallback:
    def setup_handler(self):
        def handler():  # Should not be flagged as dead
            pass
        wx.CallAfter(handler)

# Test case 2: asyncio callback
async def test_async():
    def worker():  # Should not be flagged as dead
        return "result"
    task = asyncio.create_task(worker())

# Test case 3: Threading callback
def test_threading():
    def background_task():  # Should not be flagged as dead
        time.sleep(1)
    timer = threading.Timer(5.0, background_task)
```

### Performance Tests
- Measure analysis time on large Python codebases
- Memory usage profiling for nested function tracking
- Regression tests to ensure no performance degradation

## Documentation Requirements

### Code Documentation
- Comprehensive docstrings for all new methods
- Inline comments explaining callback pattern detection logic
- Examples of supported callback patterns

### User Documentation
- Update README with improved Python dead code detection
- Add examples of callback patterns that are now properly detected
- Document any new configuration options

### Architecture Updates
- Update ARCHITECTURE.md to reflect enhanced Python call graph capabilities
- Document the callback pattern detection system
- Add Python-specific call graph analysis section

## Implementation Notes

### Pattern Matching Strategy
Use a configuration-driven approach for callback patterns to enable easy extension:
- Define patterns with function names, module names, and argument positions
- Support both exact matches and regex patterns for flexibility
- Allow for framework-specific customization

### Error Handling
- Graceful degradation when callback pattern detection fails
- Proper error logging for debugging callback analysis issues
- Maintain existing call graph functionality even if callback detection fails

### Performance Optimizations
- Lazy evaluation of callback pattern matching
- Efficient storage of nested function mappings
- Minimize AST traversal overhead

### Future Extensions
- Support for lambda functions passed as callbacks
- Method reference callbacks (`self.method` passed to callbacks)
- Dynamic callback registration patterns
- Configuration file for custom callback patterns

## Migration and Compatibility

### Backward Compatibility
- No breaking changes to existing `PythonCallGraphAnalyzer` API
- Existing call graph functionality remains unchanged
- Dead code detection results will improve (fewer false positives) but API remains the same

### Configuration Migration
- No configuration changes required
- Callback pattern detection enabled by default
- Optional configuration for custom callback patterns in future versions

### Testing Migration
- Existing tests continue to pass
- New tests added for callback patterns
- Performance regression tests updated with new baselines

## Success Metrics

### Quantitative Goals
- **False Positive Reduction**: 90%+ reduction in dead code false positives for callback patterns
- **Performance Impact**: < 5% increase in Python analysis time
- **Coverage**: Support for 95%+ of common Python callback patterns
- **Accuracy**: Line number reporting accurate within 1 line for callback references

### Qualitative Goals
- **Developer Experience**: Significantly reduced manual suppression comments needed
- **Framework Support**: Comprehensive support for major Python GUI and async frameworks
- **Maintainability**: Clear, extensible pattern matching system for future framework additions
- **Documentation**: Clear examples and documentation for supported patterns