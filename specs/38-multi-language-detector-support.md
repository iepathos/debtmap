# Specification 38: Multi-Language Detector Support

## Problem Statement

Debtmap currently has comprehensive detector support only for Rust. Python has basic support (complexity, TODOs, smells), while JavaScript and TypeScript have minimal support. This creates an inconsistent experience across supported languages and limits the tool's effectiveness for polyglot codebases.

### Current State

**Rust (Full Support)**:
- ✅ Performance detectors (nested loops, I/O, allocations, data structures, strings)
- ✅ Organization detectors (god objects, magic values, long parameters, feature envy, primitive obsession)
- ✅ Security detectors (hardcoded secrets, SQL injection, input validation)
- ✅ Resource detectors (missing Drop, async resources, unbounded collections)
- ✅ Testing detectors (missing assertions, overly complex tests, flaky patterns)
- ✅ Error handling detectors (error swallowing patterns)

**Python (Basic Support)**:
- ✅ Complexity metrics
- ✅ TODO/FIXME detection
- ✅ Code smells
- ❌ Performance detectors
- ❌ Organization detectors
- ❌ Security detectors
- ❌ Resource detectors
- ❌ Testing detectors

**JavaScript/TypeScript (Minimal Support)**:
- ✅ Basic analysis via tree-sitter
- ❌ Performance detectors
- ❌ Organization detectors
- ❌ Security detectors
- ❌ Resource detectors
- ❌ Testing detectors

## Solution Design

### Architecture

Create language-specific detector implementations that leverage each language's AST parser:
- Python: Use rustpython-parser's AST
- JavaScript/TypeScript: Use tree-sitter's syntax tree

### Common Detector Interface

```rust
pub trait LanguageDetector {
    fn detect_performance_issues(&self, ast: &dyn Any, path: &Path) -> Vec<PerformanceAntiPattern>;
    fn detect_organization_issues(&self, ast: &dyn Any, path: &Path) -> Vec<OrganizationAntiPattern>;
    fn detect_security_issues(&self, ast: &dyn Any, path: &Path) -> Vec<SecurityVulnerability>;
    fn detect_resource_issues(&self, ast: &dyn Any, path: &Path) -> Vec<ResourceManagementIssue>;
    fn detect_testing_issues(&self, ast: &dyn Any, path: &Path) -> Vec<TestingAntiPattern>;
}
```

### Python-Specific Detectors

#### Performance Detectors
1. **Nested Loop Detection**
   - Identify nested for/while loops
   - Detect list comprehensions with multiple iterators
   - Flag generator expressions with nested iterations

2. **I/O Performance**
   - Detect synchronous file I/O in loops
   - Identify unbuffered reads/writes
   - Flag database queries in loops

3. **Data Structure Issues**
   - List operations that should use sets (membership testing)
   - String concatenation in loops (should use join)
   - Repeated dictionary lookups

4. **Python-Specific Anti-patterns**
   - Using `+=` for string concatenation in loops
   - Not using generators for large sequences
   - Repeated regex compilation

#### Organization Detectors
1. **God Classes**
   - Classes with too many methods/attributes
   - Classes with mixed responsibilities

2. **Magic Values**
   - Hardcoded numeric/string literals
   - Missing constants for repeated values

3. **Long Parameter Lists**
   - Functions with > 5 parameters
   - Missing use of `*args`, `**kwargs` where appropriate

4. **Python-Specific Issues**
   - Global state mutation
   - Missing use of dataclasses/namedtuples
   - Mutable default arguments

#### Security Detectors
1. **Input Validation**
   - Missing validation in web handlers
   - Unsafe use of `eval()`, `exec()`
   - SQL query construction without parameterization

2. **Secret Detection**
   - API keys in code
   - Passwords in plain text
   - Environment variables with secrets

3. **Python-Specific Vulnerabilities**
   - Pickle deserialization of untrusted data
   - Using `assert` for validation (disabled in production)
   - Path traversal in file operations

#### Resource Management
1. **File Handle Management**
   - Files opened without context managers
   - Missing `finally` blocks for cleanup

2. **Connection Management**
   - Database connections not closed
   - Network sockets left open

3. **Python-Specific Resources**
   - Thread/process pool management
   - Generator cleanup
   - Async context manager usage

#### Testing Detectors
1. **Test Quality**
   - Tests without assertions
   - Overly complex test functions
   - Missing test docstrings

2. **Python-Specific Testing Issues**
   - Missing use of pytest fixtures
   - Test pollution (shared state)
   - Missing mock cleanup

### JavaScript/TypeScript-Specific Detectors

#### Performance Detectors
1. **Async/Await Issues**
   - Sequential awaits that could be parallel
   - Missing Promise.all() for independent operations
   - Blocking operations in async functions

2. **DOM Manipulation**
   - Repeated DOM queries
   - Layout thrashing
   - Missing use of DocumentFragment

3. **JavaScript-Specific Anti-patterns**
   - Array operations in loops (map/filter/reduce chains)
   - Missing use of Set/Map for lookups
   - Repeated JSON parsing

#### Organization Detectors
1. **Module Organization**
   - Circular dependencies
   - God modules with mixed concerns
   - Missing proper exports

2. **TypeScript-Specific**
   - Overuse of `any` type
   - Missing type definitions
   - Complex type gymnastics

#### Security Detectors
1. **XSS Vulnerabilities**
   - innerHTML with user input
   - Missing input sanitization
   - Unsafe use of eval()

2. **JavaScript-Specific Security**
   - Prototype pollution risks
   - Missing CSRF protection
   - Exposed sensitive data in client code

#### Resource Management
1. **Memory Leaks**
   - Event listeners not removed
   - Circular references
   - Large object retention

2. **Async Resource Management**
   - Unclosed WebSocket connections
   - Missing AbortController usage
   - Orphaned timers/intervals

#### Testing Detectors
1. **Test Quality**
   - Missing test coverage for async code
   - Tests without proper cleanup
   - Flaky timing-dependent tests

2. **Framework-Specific Issues**
   - Missing React component cleanup
   - Vue lifecycle hook issues
   - Angular subscription leaks

## Implementation Plan

### Phase 1: Python Support (Priority: High)
1. Implement performance detectors using rustpython-parser AST
2. Add organization pattern detection
3. Implement security vulnerability detection
4. Add resource management checks
5. Implement testing pattern detection

### Phase 2: JavaScript/TypeScript Support (Priority: Medium)
1. Enhance tree-sitter integration for deeper AST analysis
2. Implement async/await performance patterns
3. Add DOM-specific performance detection
4. Implement security vulnerability detection
5. Add testing pattern detection

### Phase 3: Language-Specific Optimizations (Priority: Low)
1. Add language-specific pattern libraries
2. Implement framework-specific detectors (Django, React, etc.)
3. Add language-specific severity weights

## Testing Strategy

### Unit Tests
- Test each detector with language-specific code samples
- Verify accurate line number extraction
- Test suppression comment handling

### Integration Tests
- Multi-language project analysis
- Cross-language dependency detection
- Performance benchmarking

### Test Coverage Goals
- 80% coverage for all detector implementations
- 100% coverage for critical security detectors
- Framework-specific test suites

## Performance Considerations

### Optimization Strategies
1. Lazy AST traversal
2. Cached pattern matching
3. Parallel detector execution per language
4. Incremental analysis for unchanged files

### Performance Targets
- < 100ms overhead per file for detector analysis
- < 10% total analysis time increase
- Memory usage proportional to file size

## Migration Path

Since we're prototyping and don't need backward compatibility:
1. Add new detector implementations directly
2. Update analyzer modules to use language-specific detectors
3. Remove any legacy placeholder code

## Success Criteria

1. **Feature Parity**: Python and JavaScript achieve 80% feature parity with Rust detectors
2. **Accuracy**: < 10% false positive rate for all detectors
3. **Performance**: Analysis time increases by < 20% with full detector coverage
4. **Adoption**: Polyglot projects can use all detector features

## Configuration

```toml
[detectors.python]
performance = true
organization = true
security = true
resource = true
testing = true

[detectors.javascript]
performance = true
organization = true
security = true
resource = true
testing = true
dom_specific = true  # JavaScript-only
async_patterns = true  # JavaScript-only

[detectors.typescript]
# Inherits JavaScript settings
type_checking = true  # TypeScript-only
```

## Dependencies

- rustpython-parser (existing)
- tree-sitter (existing)
- No new dependencies required

## Future Enhancements

1. **ML-Based Pattern Detection**: Use machine learning to identify language-specific anti-patterns
2. **Cross-Language Pattern Correlation**: Detect similar issues across different languages
3. **Framework-Specific Detectors**: Add support for popular frameworks (Django, Express, React, etc.)
4. **Custom Pattern Definition**: Allow users to define their own language-specific patterns