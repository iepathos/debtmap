---
number: 110
title: Python Exception Propagation Flow Analysis
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-15
---

# Specification 110: Python Exception Propagation Flow Analysis

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The Rust analyzer provides comprehensive error propagation analysis, tracking how `Result` types flow through the call graph, identifying error context chains, and detecting error handling patterns. The Python analyzer currently has only basic exception swallowing detection with no understanding of exception propagation flows, error transformation chains, or exception handling completeness.

Current Python limitations:
- No tracking of which exceptions propagate from callee to caller
- Cannot identify missing exception documentation
- No analysis of exception transformation (catching one type, raising another)
- Limited detection of error handling completeness
- Cannot trace exception origins through the call graph
- No detection of exception handling anti-patterns (except generic patterns)

This gap means Python users miss critical information about:
- Which exceptions a function might raise
- Where exceptions are caught vs propagated
- Exception handling coverage across the codebase
- Error handling consistency and patterns
- Exception transformation chains

## Objective

Implement comprehensive exception propagation flow analysis for Python that tracks exception types through the call graph, identifies exception handling patterns, detects missing error handling, and provides visibility into exception flows similar to Rust's Result propagation analysis.

## Requirements

### Functional Requirements

1. **Exception Type Tracking**
   - Track `raise` statements and their exception types
   - Identify built-in exception types (ValueError, TypeError, etc.)
   - Track custom exception classes and their hierarchy
   - Detect exception transformations (catch one, raise another)
   - Support exception chaining (`raise ... from ...`)

2. **Exception Propagation Analysis**
   - Build exception flow graph through call chain
   - Track which functions can raise which exceptions
   - Identify exception boundaries (where exceptions are caught)
   - Detect exception propagation to top-level
   - Track exception context preservation

3. **Exception Handling Patterns**
   - Detect bare `except:` clauses (anti-pattern)
   - Identify overly broad exception handling (`except Exception`)
   - Detect empty exception handlers (swallowing errors)
   - Find exception handlers that only log and continue
   - Identify proper exception handling patterns
   - Detect re-raising with modifications

4. **Exception Documentation**
   - Extract exception documentation from docstrings
   - Identify undocumented exceptions in public APIs
   - Validate docstring exceptions match actual raises
   - Detect missing Raises sections in docstrings
   - Support Google, NumPy, and Sphinx docstring formats

5. **Exception Hierarchy Analysis**
   - Track custom exception class hierarchies
   - Identify exception subclass relationships
   - Detect exception catching that's too broad
   - Find exception types that are never caught
   - Analyze exception granularity

6. **Call Graph Integration**
   - Integrate exception flows with call graph
   - Track which callers handle which exceptions
   - Identify functions that propagate all exceptions
   - Detect exception handling responsibilities
   - Build upstream/downstream exception views

### Non-Functional Requirements

- **Accuracy**: < 20% false positive rate for missing handlers
- **Performance**: < 15% overhead on Python analysis time
- **Coverage**: Track 95%+ of raised exceptions
- **Type Support**: Handle both built-in and custom exceptions
- **Documentation**: Support 3 major docstring formats

## Acceptance Criteria

- [ ] All `raise` statements tracked with exception types
- [ ] Exception propagation through call graph constructed
- [ ] Bare except clauses and overly broad handlers detected
- [ ] Exception swallowing patterns identified
- [ ] Custom exception hierarchy analyzed
- [ ] Docstring exception documentation extracted and validated
- [ ] Undocumented exceptions in public APIs flagged
- [ ] Exception transformation chains tracked
- [ ] Call graph shows exception flows
- [ ] Integration with existing Python analyzer
- [ ] 95%+ of raised exceptions tracked correctly
- [ ] Unit tests for all exception pattern types
- [ ] Documentation includes exception analysis guide

## Technical Details

### Implementation Approach

1. Create `ExceptionFlowAnalyzer` in `src/analyzers/python/exception_flow.rs`
2. Implement exception type extraction from AST
3. Build exception propagation graph
4. Add docstring parsing for exception documentation
5. Integrate with call graph for flow analysis
6. Create exception pattern detection system

### Architecture Changes

```rust
// src/analyzers/python/exception_flow.rs
pub struct ExceptionFlowAnalyzer {
    exception_registry: HashMap<String, ExceptionInfo>,
    exception_flows: HashMap<FunctionId, ExceptionFlow>,
    custom_exceptions: HashMap<String, ExceptionClass>,
    handling_patterns: Vec<HandlingPattern>,
}

pub struct ExceptionInfo {
    exception_type: ExceptionType,
    location: Location,
    is_documented: bool,
    context_message: Option<String>,
    source_exception: Option<Box<ExceptionInfo>>, // for "raise ... from ..."
}

pub enum ExceptionType {
    Builtin(BuiltinException),
    Custom(String),
    Variable(String), // Exception type from variable
    Unknown,
}

pub enum BuiltinException {
    ValueError,
    TypeError,
    KeyError,
    AttributeError,
    IndexError,
    RuntimeError,
    NotImplementedError,
    IOError,
    OSError,
    // ... all built-in exceptions
}

pub struct ExceptionFlow {
    function_id: FunctionId,
    raised_exceptions: Vec<ExceptionInfo>,
    caught_exceptions: Vec<CaughtException>,
    propagated_exceptions: Vec<ExceptionType>,
    transformed_exceptions: Vec<ExceptionTransformation>,
    handling_completeness: HandlingCompleteness,
}

pub struct CaughtException {
    exception_types: Vec<ExceptionType>,
    location: Location,
    handler_type: HandlerType,
    is_bare_except: bool,
    is_overly_broad: bool,
    handler_action: HandlerAction,
}

pub enum HandlerType {
    Specific,        // except ValueError
    Multiple,        // except (ValueError, KeyError)
    Broad,           // except Exception
    Bare,            // except:
    BaseException,   // except BaseException
}

pub enum HandlerAction {
    Reraise,
    Transform,
    Log,
    Ignore,
    Handle,
}

pub struct ExceptionTransformation {
    caught_type: ExceptionType,
    raised_type: ExceptionType,
    location: Location,
    preserves_context: bool, // uses "raise ... from ..."
}

pub struct HandlingCompleteness {
    all_exceptions_handled: bool,
    missing_handlers: Vec<ExceptionType>,
    overly_broad_handlers: Vec<Location>,
    proper_handlers: usize,
}

pub struct ExceptionClass {
    name: String,
    base_classes: Vec<String>,
    location: Location,
    docstring: Option<String>,
}

pub struct ExceptionDocumentation {
    function_id: FunctionId,
    documented_exceptions: Vec<DocumentedException>,
    docstring_format: DocstringFormat,
}

pub struct DocumentedException {
    exception_type: String,
    description: String,
    location: Location,
}

pub enum DocstringFormat {
    Google,
    NumPy,
    Sphinx,
    Unknown,
}

pub struct ExceptionFlowPattern {
    pattern_type: ExceptionPatternType,
    severity: Severity,
    confidence: f32,
    location: Location,
    explanation: String,
    suggestion: String,
}

pub enum ExceptionPatternType {
    BareExcept,
    OverlyBroadHandler,
    ExceptionSwallowing,
    UndocumentedException,
    MissingHandler,
    ExceptionNotRaised,      // Documented but not raised
    TransformationLost,       // Lost exception context
    PropagateAll,             // Function that catches nothing
    CatchReraise,             // Catches only to re-raise
    LogAndIgnore,             // Logs exception but continues
}
```

### Data Structures

```rust
// Exception type hierarchy
pub const BUILTIN_EXCEPTION_HIERARCHY: &[(&str, &[&str])] = &[
    ("BaseException", &[]),
    ("Exception", &["BaseException"]),
    ("ValueError", &["Exception"]),
    ("TypeError", &["Exception"]),
    ("KeyError", &["LookupError", "Exception"]),
    ("AttributeError", &["Exception"]),
    ("IOError", &["OSError", "Exception"]),
    // ... full hierarchy
];

// Exception handling anti-patterns
pub struct ExceptionAntiPattern {
    pattern: &'static str,
    description: &'static str,
    severity: Severity,
}

pub const EXCEPTION_ANTI_PATTERNS: &[ExceptionAntiPattern] = &[
    ExceptionAntiPattern {
        pattern: "bare_except",
        description: "Bare except clause catches all exceptions including KeyboardInterrupt",
        severity: Severity::High,
    },
    ExceptionAntiPattern {
        pattern: "overly_broad",
        description: "Catching Exception is too broad, catch specific exceptions",
        severity: Severity::Medium,
    },
    ExceptionAntiPattern {
        pattern: "swallow_error",
        description: "Exception caught but not logged or re-raised",
        severity: Severity::High,
    },
    // ... more patterns
];
```

### APIs and Interfaces

```rust
impl ExceptionFlowAnalyzer {
    pub fn new() -> Self;

    /// Analyze exception flows in a function
    pub fn analyze_function(&mut self, func: &ast::StmtFunctionDef) -> ExceptionFlow;

    /// Track a raise statement
    pub fn track_raise(&mut self, raise_stmt: &ast::StmtRaise) -> ExceptionInfo;

    /// Analyze exception handler
    pub fn analyze_handler(&self, handler: &ast::ExceptHandler) -> CaughtException;

    /// Extract exception documentation from docstring
    pub fn extract_exception_docs(&self, func: &ast::StmtFunctionDef) -> ExceptionDocumentation;

    /// Build exception flow graph across call chain
    pub fn build_exception_graph(&self, call_graph: &CallGraph) -> ExceptionGraph;

    /// Detect exception handling patterns
    pub fn detect_patterns(&self, flow: &ExceptionFlow) -> Vec<ExceptionFlowPattern>;

    /// Validate exception documentation
    pub fn validate_documentation(&self, flow: &ExceptionFlow, docs: &ExceptionDocumentation) -> Vec<DocumentationIssue>;

    /// Get exception hierarchy
    pub fn get_exception_hierarchy(&self, exception: &str) -> Vec<String>;

    /// Check if exception type is caught by handler
    pub fn is_caught_by(&self, exception: &ExceptionType, handler: &CaughtException) -> bool;
}

// Integration with Python analyzer
impl PythonAnalyzer {
    fn analyze_exception_flows(&mut self, tree: &ast::Module) -> Vec<ExceptionFlowPattern> {
        let mut analyzer = ExceptionFlowAnalyzer::new();
        // ... analysis logic
    }

    fn integrate_with_call_graph(&mut self, exception_flows: &[ExceptionFlow]) {
        // Add exception info to call graph nodes
    }
}
```

### Exception Pattern Detection Examples

```python
# Pattern 1: Bare except (anti-pattern)
def bad_error_handling():
    try:
        risky_operation()
    except:  # DETECTED: BareExcept
        pass

# Pattern 2: Overly broad handler
def broad_handler():
    try:
        specific_operation()
    except Exception:  # DETECTED: OverlyBroadHandler (should catch specific exception)
        log.error("Something went wrong")

# Pattern 3: Exception swallowing
def swallow_error():
    try:
        critical_operation()
    except ValueError:  # DETECTED: ExceptionSwallowing
        pass  # No logging, no re-raise

# Pattern 4: Undocumented exception
def undocumented_exception():
    """Does something important.

    Args:
        value: The value to process

    Returns:
        Processed value
    """
    # DETECTED: UndocumentedException (ValueError not documented)
    if value < 0:
        raise ValueError("Value must be positive")
    return value

# Pattern 5: Exception transformation (good pattern if documented)
def transform_exception():
    """Transform data.

    Raises:
        ValidationError: If data is invalid
    """
    try:
        parse_data()
    except KeyError as e:
        raise ValidationError("Invalid data format") from e  # GOOD: Context preserved

# Pattern 6: Documentation mismatch
def doc_mismatch():
    """Process data.

    Raises:
        ValueError: If value is invalid
    """
    # DETECTED: ExceptionNotRaised (ValueError documented but not raised)
    if not valid:
        raise TypeError("Invalid type")  # Wrong exception type

# Pattern 7: Proper exception handling (no detection)
def proper_handling():
    """Load data.

    Raises:
        FileNotFoundError: If file doesn't exist
        ValueError: If file content is invalid
    """
    try:
        with open(filename) as f:
            data = f.read()
    except FileNotFoundError:
        log.error(f"File not found: {filename}")
        raise  # GOOD: Re-raises after logging

    if not validate(data):
        raise ValueError(f"Invalid data in {filename}")  # GOOD: Documented

# Pattern 8: Exception propagation tracking
def caller():
    """Calls risky function.

    Raises:
        ValueError: Propagated from risky_operation
    """
    return risky_operation()  # TRACKED: ValueError propagates

def risky_operation():
    """Does risky work.

    Raises:
        ValueError: If operation fails
    """
    if condition:
        raise ValueError("Operation failed")
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analyzers/python.rs` - Main Python analyzer integration
  - `src/analysis/python_call_graph/` - Call graph integration
  - `src/core/debt_item.rs` - Add exception flow debt types
  - `src/priority/scoring.rs` - Add exception handling scoring
- **External Dependencies**: None (uses existing `rustpython_parser`)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_detect_bare_except() {
        let code = r#"
def test():
    try:
        risky()
    except:
        pass
"#;
        let patterns = analyze_exceptions(code);
        assert!(patterns.iter().any(|p| matches!(p.pattern_type, ExceptionPatternType::BareExcept)));
    }

    #[test]
    fn test_track_exception_propagation() {
        let code = r#"
def caller():
    return callee()

def callee():
    raise ValueError("error")
"#;
        let flows = analyze_exception_flows(code);
        let caller_flow = flows.get("caller").unwrap();
        assert!(caller_flow.propagated_exceptions.contains(&ExceptionType::Builtin(BuiltinException::ValueError)));
    }

    #[test]
    fn test_exception_transformation() {
        let code = r#"
def transform():
    try:
        parse()
    except KeyError as e:
        raise ValueError("Invalid") from e
"#;
        let flow = analyze_function(code);
        assert_eq!(flow.transformed_exceptions.len(), 1);
        assert!(flow.transformed_exceptions[0].preserves_context);
    }

    #[test]
    fn test_docstring_validation() {
        let code = r#"
def documented():
    '''
    Raises:
        ValueError: If invalid
    '''
    raise ValueError("error")
"#;
        let issues = validate_documentation(code);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_undocumented_exception() {
        let code = r#"
def undocumented():
    '''Does something'''
    raise ValueError("error")
"#;
        let patterns = analyze_exceptions(code);
        assert!(patterns.iter().any(|p| matches!(p.pattern_type, ExceptionPatternType::UndocumentedException)));
    }
}
```

### Integration Tests

1. **Real codebase analysis**:
   - Analyze Python projects with comprehensive error handling
   - Verify exception flow tracking across modules
   - Measure false positive rate

2. **Docstring format tests**:
   - Test Google-style docstrings
   - Test NumPy-style docstrings
   - Test Sphinx-style docstrings
   - Verify exception extraction from each format

3. **Call graph integration**:
   - Test exception propagation through call chains
   - Verify upstream/downstream exception tracking
   - Test multi-level exception transformations

4. **Performance tests**:
   - Analyze large Python codebase (10,000+ lines)
   - Measure analysis overhead (target: < 15%)
   - Profile exception flow construction

## Documentation Requirements

### Code Documentation

- Document exception tracking algorithm
- Explain exception hierarchy resolution
- Provide guidelines for adding new patterns
- Document docstring parsing strategies

### User Documentation

Add to debtmap user guide:

```markdown
## Exception Propagation Analysis

Debtmap analyzes exception flows through your Python code:

### Exception Tracking

Track which exceptions are raised and how they propagate:

```python
def caller():
    '''
    Raises:
        ValueError: Propagated from risky_operation
    '''
    return risky_operation()  # ValueError propagates to caller
```

### Exception Documentation

Document all exceptions in public APIs:

```python
def public_function(value):
    '''Process a value.

    Args:
        value: The value to process

    Returns:
        Processed value

    Raises:
        ValueError: If value is negative
        TypeError: If value is not a number
    '''
    if not isinstance(value, (int, float)):
        raise TypeError("Value must be a number")
    if value < 0:
        raise ValueError("Value must be non-negative")
    return value * 2
```

### Exception Handling Patterns

Avoid common anti-patterns:

```python
# Bad: Bare except
try:
    operation()
except:  # Catches ALL exceptions including KeyboardInterrupt
    pass

# Bad: Overly broad
try:
    operation()
except Exception:  # Too broad
    pass

# Good: Specific exception
try:
    operation()
except ValueError as e:
    log.error(f"Invalid value: {e}")
    raise

# Good: Exception transformation with context
try:
    parse_data()
except KeyError as e:
    raise ValidationError("Invalid data") from e  # Preserves context
```

### Configuration

Control exception analysis:

```toml
[analysis.python.exceptions]
detect_bare_except = true
detect_broad_handlers = true
detect_swallowing = true
detect_undocumented = true
validate_docstrings = true
min_confidence = 0.7

# Docstring formats to parse
docstring_formats = ["google", "numpy", "sphinx"]

# Exceptions that don't need documentation (internal APIs)
undocumented_exceptions_allowed = ["AssertionError", "NotImplementedError"]
```
```

### Architecture Updates

Update ARCHITECTURE.md:
- Add exception flow analysis to Python analyzer section
- Document exception propagation graph construction
- Explain docstring parsing strategy
- Add diagram showing exception flow tracking

## Implementation Notes

### Phase 1: Exception Tracking (Week 1)
- Implement basic raise statement tracking
- Add exception type identification
- Track built-in exceptions
- Unit tests for exception tracking

### Phase 2: Handler Analysis (Week 2)
- Implement exception handler analysis
- Add pattern detection (bare except, overly broad)
- Detect exception swallowing
- Unit tests for handler patterns

### Phase 3: Documentation (Week 3)
- Implement docstring parsing (Google, NumPy, Sphinx)
- Add documentation validation
- Detect undocumented exceptions
- Integration tests for documentation

### Phase 4: Propagation (Week 4)
- Build exception flow graph
- Integrate with call graph
- Track exception transformations
- Performance optimization

### Docstring Parsing Strategy

```rust
fn parse_exception_docs(docstring: &str) -> Vec<DocumentedException> {
    // Try Google style first
    if let Some(docs) = parse_google_style(docstring) {
        return docs;
    }

    // Try NumPy style
    if let Some(docs) = parse_numpy_style(docstring) {
        return docs;
    }

    // Try Sphinx style
    if let Some(docs) = parse_sphinx_style(docstring) {
        return docs;
    }

    vec![]
}

fn parse_google_style(docstring: &str) -> Option<Vec<DocumentedException>> {
    // Look for "Raises:" section
    // Format:
    //   Raises:
    //       ValueError: Description
    //       TypeError: Description
}

fn parse_numpy_style(docstring: &str) -> Option<Vec<DocumentedException>> {
    // Look for "Raises" section
    // Format:
    //   Raises
    //   ------
    //   ValueError
    //       Description
}

fn parse_sphinx_style(docstring: &str) -> Option<Vec<DocumentedException>> {
    // Look for :raises: tags
    // Format:
    //   :raises ValueError: Description
    //   :raises TypeError: Description
}
```

### Exception Hierarchy Resolution

```rust
fn is_subclass_of(exception: &str, base: &str, registry: &ExceptionRegistry) -> bool {
    if exception == base {
        return true;
    }

    // Check custom exceptions
    if let Some(exc_class) = registry.custom_exceptions.get(exception) {
        for base_class in &exc_class.base_classes {
            if is_subclass_of(base_class, base, registry) {
                return true;
            }
        }
    }

    // Check built-in exceptions
    if let Some(bases) = BUILTIN_EXCEPTION_HIERARCHY.get(exception) {
        for exc_base in bases {
            if is_subclass_of(exc_base, base, registry) {
                return true;
            }
        }
    }

    false
}

fn is_caught_by_handler(exception: &ExceptionType, handler: &CaughtException) -> bool {
    match handler.handler_type {
        HandlerType::Bare => true,  // Catches everything
        HandlerType::BaseException => true,  // Catches everything
        HandlerType::Broad => {
            // Check if exception is subclass of Exception
            matches!(exception, ExceptionType::Builtin(_))
        }
        HandlerType::Specific | HandlerType::Multiple => {
            // Check if exception matches any caught type
            handler.exception_types.iter().any(|caught| {
                exception_matches(exception, caught)
            })
        }
    }
}
```

### Performance Optimization

- Cache exception hierarchy lookups
- Skip exception analysis for non-function code
- Parallel processing of functions
- Lazy evaluation of expensive checks
- Efficient docstring parsing with early exit

## Migration and Compatibility

### Backward Compatibility

- No breaking changes to existing Python analysis
- New debt items are additive
- Existing JSON output remains compatible
- Can be disabled via configuration

### Configuration Options

```toml
[analysis.python]
enable_exception_flow_analysis = true

[analysis.python.exceptions]
detect_bare_except = true
detect_broad_handlers = true
detect_swallowing = true
detect_undocumented = true
validate_docstrings = true
min_confidence = 0.7

# Docstring parsing
docstring_formats = ["google", "numpy", "sphinx"]
require_raises_section = false  # Only for public APIs

# Ignore rules
undocumented_exceptions_allowed = ["AssertionError", "NotImplementedError"]
ignore_test_files = true
ignore_private_functions = true  # Functions starting with _
```

### Migration Path

1. **Default disabled**: Initial release with feature flag
2. **Opt-in period**: Users enable via configuration
3. **Feedback period**: Collect false positives and adjust
4. **Gradual rollout**: Enable by default after validation

## Success Metrics

- **Tracking coverage**: 95%+ of raised exceptions tracked
- **False positive rate**: < 20% for missing handlers
- **Documentation accuracy**: 90%+ correct exception doc extraction
- **Performance overhead**: < 15% on Python analysis time
- **User adoption**: 35%+ of Python projects enable analysis
- **Bug prevention**: Catch 100+ exception handling issues in real codebases

## Future Enhancements

1. **Exception impact analysis**: Rank exceptions by call graph reach
2. **Auto-documentation**: Generate docstring Raises sections
3. **Exception recovery suggestions**: Suggest proper handling strategies
4. **Cross-language exception mapping**: Map Python exceptions to Rust Result patterns
5. **Exception flow visualization**: Generate exception flow diagrams
6. **Custom exception linting**: Enforce project-specific exception conventions
7. **Async exception tracking**: Integrate with asyncio error detection (Spec 109)
