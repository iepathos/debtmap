---
number: 109
title: Python Asyncio Error Pattern Detection
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-15
---

# Specification 109: Python Asyncio Error Pattern Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The Python analyzer currently lacks detection for asyncio-specific error patterns that can lead to silent failures, resource leaks, and subtle bugs in async applications. While the Rust analyzer has comprehensive async error detection for tokio patterns (dropped futures, unhandled JoinHandles, silent task panics), Python has no equivalent for asyncio.

Current gaps:
- No detection of unhandled task exceptions
- Missing async resource leak patterns (unclosed streams, connections)
- No tracking of fire-and-forget tasks
- Silent failures in `asyncio.create_task()` without await
- Unbounded task creation leading to memory leaks
- Missing timeout patterns and cancellation handling
- No detection of blocking operations in async context

This creates a significant capability gap between Rust and Python analysis, particularly for modern async-heavy Python applications using FastAPI, aiohttp, or async frameworks.

## Objective

Implement comprehensive asyncio error pattern detection for Python to identify async resource leaks, unhandled task exceptions, fire-and-forget tasks, blocking operations in async context, and other async-specific anti-patterns that lead to production failures.

## Requirements

### Functional Requirements

1. **Task Exception Handling**
   - Detect `asyncio.create_task()` without exception handling
   - Identify `asyncio.gather()` without `return_exceptions=True`
   - Track task references that are never awaited or checked
   - Detect bare `except` clauses that swallow `CancelledError`
   - Flag tasks created but not stored for monitoring

2. **Async Resource Management**
   - Detect async context managers not used with `async with`
   - Identify unclosed async resources (streams, connections, sessions)
   - Track `asyncio.StreamWriter` without `.wait_closed()`
   - Detect `aiohttp.ClientSession` without proper cleanup
   - Flag async generators not properly exhausted or closed
   - Identify resource leaks in long-running tasks

3. **Concurrency Anti-Patterns**
   - Detect unbounded `asyncio.create_task()` loops
   - Identify missing semaphores for resource-limited operations
   - Track concurrent task limits and backpressure patterns
   - Flag potential thundering herd problems
   - Detect event loop blocking operations

4. **Timeout and Cancellation**
   - Detect missing timeouts on network operations
   - Identify improper cancellation handling
   - Track `asyncio.shield()` misuse
   - Flag operations that don't respect cancellation
   - Detect timeout values that are too large or missing

5. **Event Loop Patterns**
   - Detect blocking I/O in async functions (file operations, `time.sleep`)
   - Identify synchronous library calls that should be async
   - Track `asyncio.run()` usage in incorrect contexts
   - Detect nested event loop creation
   - Flag `loop.run_until_complete()` in async context

6. **Common Async Bugs**
   - Detect `await` on non-awaitable objects
   - Identify missing `await` on coroutine calls
   - Track coroutine objects that are never awaited
   - Detect `async def` functions called without `await`
   - Flag synchronous iteration over async iterables

### Non-Functional Requirements

- **Accuracy**: < 15% false positive rate
- **Performance**: < 10% overhead on Python analysis time
- **Coverage**: Detect 90%+ of common asyncio anti-patterns
- **Extensibility**: Support custom async pattern rules
- **Framework Support**: Detect patterns in FastAPI, aiohttp, Quart

## Acceptance Criteria

- [ ] Unhandled task exceptions detected in `asyncio.create_task()`
- [ ] Fire-and-forget tasks flagged with confidence scores
- [ ] Async resource leaks identified (unclosed sessions, streams)
- [ ] Blocking operations in async context detected
- [ ] Missing timeouts on network operations flagged
- [ ] Improper cancellation handling identified
- [ ] Unbounded task creation patterns detected
- [ ] Framework-specific patterns supported (FastAPI, aiohttp)
- [ ] Confidence scoring for each detection (high/medium/low)
- [ ] Integration with existing Python analyzer
- [ ] 90%+ detection rate on test suite of known async bugs
- [ ] Unit tests for each pattern type
- [ ] Documentation includes async pattern catalog

## Technical Details

### Implementation Approach

1. Create `AsyncioPatternDetector` in `src/analyzers/python/asyncio_patterns.rs`
2. Implement AST pattern matching for async/await constructs
3. Add task tracking and lifecycle analysis
4. Integrate with existing Python analyzer pipeline
5. Create confidence scoring system for detections

### Architecture Changes

```rust
// src/analyzers/python/asyncio_patterns.rs
pub struct AsyncioPatternDetector {
    task_registry: HashMap<String, TaskInfo>,
    resource_registry: HashMap<String, AsyncResourceInfo>,
    blocking_operations: HashSet<String>,
    framework_patterns: FrameworkAsyncPatterns,
}

pub struct TaskInfo {
    creation_site: Location,
    is_awaited: bool,
    has_exception_handling: bool,
    has_timeout: bool,
    task_type: TaskType,
}

pub enum TaskType {
    CreateTask,
    Gather,
    WaitFor,
    RunInExecutor,
    Shield,
}

pub struct AsyncResourceInfo {
    resource_type: AsyncResourceType,
    creation_site: Location,
    has_cleanup: bool,
    context_manager_used: bool,
    lifecycle: ResourceLifecycle,
}

pub enum AsyncResourceType {
    ClientSession,      // aiohttp.ClientSession
    StreamWriter,       // asyncio.StreamWriter
    StreamReader,       // asyncio.StreamReader
    Database,           // asyncpg, aiomysql connections
    WebSocket,          // websocket connections
    AsyncGenerator,     // async generator functions
    Lock,               // asyncio.Lock, Semaphore
}

pub enum ResourceLifecycle {
    Created,
    InUse,
    Closed,
    Leaked,
}

pub struct AsyncErrorPattern {
    pattern_type: AsyncErrorType,
    severity: Severity,
    confidence: f32,
    location: Location,
    explanation: String,
    fix_suggestion: String,
}

pub enum AsyncErrorType {
    UnhandledTaskException,
    FireAndForget,
    AsyncResourceLeak,
    BlockingInAsyncContext,
    MissingTimeout,
    ImproperCancellation,
    UnboundedTaskCreation,
    MissingAwait,
    AwaitableNotAwaited,
    EventLoopMisuse,
    CancelledErrorSwallowed,
}

// Framework-specific patterns
pub struct FrameworkAsyncPatterns {
    fastapi: FastAPIPatterns,
    aiohttp: AioHttpPatterns,
    quart: QuartPatterns,
}

pub struct FastAPIPatterns {
    pub dependency_injection: bool,
    pub background_tasks: bool,
}
```

### Data Structures

```rust
// Pattern definitions
pub const BLOCKING_OPERATIONS: &[&str] = &[
    "time.sleep",
    "open",
    "urllib.request",
    "requests.get",
    "requests.post",
    "os.system",
    "subprocess.run",
    "json.load",  // file-based
];

pub const ASYNC_RESOURCE_TYPES: &[(&str, AsyncResourceType)] = &[
    ("aiohttp.ClientSession", AsyncResourceType::ClientSession),
    ("asyncio.StreamWriter", AsyncResourceType::StreamWriter),
    ("asyncpg.Connection", AsyncResourceType::Database),
    ("aiomysql.Connection", AsyncResourceType::Database),
];

pub struct PatternMatcher {
    call_patterns: Vec<CallPattern>,
    context_patterns: Vec<ContextPattern>,
}

pub struct CallPattern {
    module: String,
    function: String,
    detection_rule: DetectionRule,
}

pub enum DetectionRule {
    MustBeAwaited,
    MustHaveExceptionHandling,
    MustHaveTimeout,
    MustBeInAsyncContext,
    MustUseContextManager,
}
```

### APIs and Interfaces

```rust
impl AsyncioPatternDetector {
    pub fn new() -> Self;

    pub fn analyze_function(&mut self, func: &ast::StmtFunctionDef) -> Vec<AsyncErrorPattern>;

    pub fn detect_unhandled_task(&self, call: &ast::ExprCall) -> Option<AsyncErrorPattern>;

    pub fn detect_resource_leak(&self, func: &ast::StmtFunctionDef) -> Vec<AsyncErrorPattern>;

    pub fn detect_blocking_call(&self, call: &ast::ExprCall, in_async_context: bool) -> Option<AsyncErrorPattern>;

    pub fn detect_missing_timeout(&self, call: &ast::ExprCall) -> Option<AsyncErrorPattern>;

    pub fn track_task_lifecycle(&mut self, task_creation: &ast::ExprCall, task_var: &str);

    pub fn track_resource_lifecycle(&mut self, resource_creation: &ast::Expr);

    pub fn check_cancellation_handling(&self, except_handler: &ast::ExceptHandler) -> Option<AsyncErrorPattern>;

    pub fn get_framework_patterns(&self, imports: &[String]) -> Option<&FrameworkAsyncPatterns>;
}

// Integration with main Python analyzer
impl PythonAnalyzer {
    fn analyze_async_patterns(&mut self, tree: &ast::Module) -> Vec<AsyncErrorPattern> {
        let detector = AsyncioPatternDetector::new();
        // ... analysis logic
    }
}
```

### Pattern Detection Examples

```python
# Pattern 1: Unhandled task exception
async def bad_task_handling():
    # BAD: No exception handling
    asyncio.create_task(risky_operation())  # DETECTED: UnhandledTaskException

    # GOOD: With exception handling
    task = asyncio.create_task(risky_operation())
    try:
        await task
    except Exception as e:
        logger.error(f"Task failed: {e}")

# Pattern 2: Async resource leak
async def resource_leak():
    # BAD: No cleanup
    session = aiohttp.ClientSession()  # DETECTED: AsyncResourceLeak
    await session.get("https://example.com")
    # Missing: await session.close()

    # GOOD: With context manager
    async with aiohttp.ClientSession() as session:
        await session.get("https://example.com")

# Pattern 3: Blocking operation in async context
async def blocking_call():
    # BAD: Blocking sleep in async function
    time.sleep(5)  # DETECTED: BlockingInAsyncContext

    # GOOD: Async sleep
    await asyncio.sleep(5)

# Pattern 4: Missing timeout
async def missing_timeout():
    # BAD: No timeout on network operation
    response = await session.get(url)  # DETECTED: MissingTimeout

    # GOOD: With timeout
    async with asyncio.timeout(10):
        response = await session.get(url)

# Pattern 5: Fire and forget
async def fire_and_forget():
    # BAD: Task created but never awaited
    asyncio.create_task(background_work())  # DETECTED: FireAndForget
    return "Done"

    # GOOD: Store for monitoring
    task = asyncio.create_task(background_work())
    background_tasks.add(task)
    task.add_done_callback(background_tasks.discard)

# Pattern 6: Unbounded task creation
async def unbounded_tasks():
    # BAD: No limit on concurrent tasks
    for item in large_list:
        asyncio.create_task(process(item))  # DETECTED: UnboundedTaskCreation

    # GOOD: With semaphore
    sem = asyncio.Semaphore(10)
    async def limited_process(item):
        async with sem:
            await process(item)

    tasks = [asyncio.create_task(limited_process(item)) for item in large_list]
    await asyncio.gather(*tasks)
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analyzers/python.rs` - Main Python analyzer integration
  - `src/core/debt_item.rs` - Add asyncio debt types
  - `src/priority/scoring.rs` - Add async pattern scoring
- **External Dependencies**: None (uses existing `rustpython_parser`)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_detect_unhandled_task_exception() {
        let code = r#"
async def test():
    asyncio.create_task(risky_operation())
"#;
        let patterns = analyze_code(code);
        assert!(patterns.iter().any(|p| matches!(p.pattern_type, AsyncErrorType::UnhandledTaskException)));
    }

    #[test]
    fn test_detect_resource_leak() {
        let code = r#"
async def test():
    session = aiohttp.ClientSession()
    await session.get("https://example.com")
"#;
        let patterns = analyze_code(code);
        assert!(patterns.iter().any(|p| matches!(p.pattern_type, AsyncErrorType::AsyncResourceLeak)));
    }

    #[test]
    fn test_detect_blocking_in_async() {
        let code = r#"
async def test():
    time.sleep(5)
"#;
        let patterns = analyze_code(code);
        assert!(patterns.iter().any(|p| matches!(p.pattern_type, AsyncErrorType::BlockingInAsyncContext)));
    }

    #[test]
    fn test_proper_context_manager_no_detection() {
        let code = r#"
async def test():
    async with aiohttp.ClientSession() as session:
        await session.get("https://example.com")
"#;
        let patterns = analyze_code(code);
        assert!(patterns.is_empty());
    }
}
```

### Integration Tests

1. **Real async codebase analysis**:
   - Analyze FastAPI application with known async issues
   - Verify detection of all injected async bugs
   - Measure false positive rate

2. **Framework-specific tests**:
   - Test FastAPI background tasks detection
   - Test aiohttp session management patterns
   - Test asyncpg connection pooling patterns

3. **Performance tests**:
   - Analyze large async codebase (10,000+ lines)
   - Measure analysis overhead (target: < 10%)
   - Profile pattern matching performance

4. **Accuracy tests**:
   - Test suite of 100 known async bug patterns
   - Measure detection rate (target: 90%+)
   - Measure false positive rate (target: < 15%)

## Documentation Requirements

### Code Documentation

- Document each async pattern type with examples
- Explain confidence scoring algorithm
- Provide guidelines for adding new patterns
- Document framework-specific pattern detection

### User Documentation

Add to debtmap user guide:

```markdown
## Asyncio Error Pattern Detection

Debtmap detects common asyncio anti-patterns and potential bugs:

### Unhandled Task Exceptions

Tasks created without exception handling can fail silently:

```python
# Bad
asyncio.create_task(risky_operation())

# Good
task = asyncio.create_task(risky_operation())
task.add_done_callback(lambda t: t.result())  # Raises if exception
```

### Async Resource Leaks

Resources that aren't properly closed:

```python
# Bad
session = aiohttp.ClientSession()
await session.get(url)

# Good
async with aiohttp.ClientSession() as session:
    await session.get(url)
```

### Blocking Operations

Synchronous operations that block the event loop:

```python
# Bad
time.sleep(5)  # Blocks event loop

# Good
await asyncio.sleep(5)  # Yields to event loop
```

### Configuration

Control async pattern detection:

```toml
[analysis.python.asyncio]
detect_unhandled_tasks = true
detect_resource_leaks = true
detect_blocking_calls = true
detect_missing_timeouts = true
min_confidence = 0.7
```
```

### Architecture Updates

Update ARCHITECTURE.md:
- Add asyncio pattern detection to Python analyzer section
- Document pattern detection algorithm
- Explain integration with main analysis pipeline
- Add diagram showing async pattern detection flow

## Implementation Notes

### Phase 1: Core Detection (Week 1)
- Implement basic pattern matching for task creation
- Add unhandled task exception detection
- Create confidence scoring system
- Unit tests for core patterns

### Phase 2: Resource Tracking (Week 2)
- Implement resource lifecycle tracking
- Add async resource leak detection
- Track context manager usage
- Integration tests for resource patterns

### Phase 3: Advanced Patterns (Week 3)
- Add blocking operation detection
- Implement timeout detection
- Add unbounded task creation detection
- Framework-specific pattern support

### Phase 4: Integration (Week 4)
- Integrate with main Python analyzer
- Add to debt item generation
- Update scoring system
- Performance optimization
- Documentation

### Confidence Scoring Algorithm

```rust
fn calculate_confidence(pattern: &AsyncErrorPattern, context: &AnalysisContext) -> f32 {
    let mut confidence = 1.0;

    // Reduce confidence if pattern is in test code
    if context.is_test_code {
        confidence *= 0.5;
    }

    // Reduce confidence if exception handling is nearby
    if context.has_nearby_exception_handler {
        confidence *= 0.7;
    }

    // Increase confidence if pattern repeats
    if context.pattern_count > 3 {
        confidence *= 1.2;
    }

    // Framework-specific adjustments
    if let Some(framework) = context.framework {
        confidence *= framework.confidence_multiplier();
    }

    confidence.clamp(0.0, 1.0)
}
```

### Framework Integration

```rust
pub fn detect_framework_patterns(imports: &[String]) -> Option<AsyncFramework> {
    if imports.iter().any(|i| i.contains("fastapi")) {
        Some(AsyncFramework::FastAPI)
    } else if imports.iter().any(|i| i.contains("aiohttp")) {
        Some(AsyncFramework::AioHttp)
    } else if imports.iter().any(|i| i.contains("quart")) {
        Some(AsyncFramework::Quart)
    } else {
        None
    }
}
```

### Performance Optimization

- Cache pattern matching results
- Skip async analysis for non-async files
- Parallel processing of functions
- Lazy evaluation of expensive checks

## Migration and Compatibility

### Backward Compatibility

- No breaking changes to existing Python analysis
- New debt items are additive
- Existing JSON output remains compatible
- Can be disabled via configuration

### Configuration Options

```toml
[analysis.python]
enable_asyncio_detection = true

[analysis.python.asyncio]
detect_unhandled_tasks = true
detect_resource_leaks = true
detect_blocking_calls = true
detect_missing_timeouts = true
detect_unbounded_tasks = true
min_confidence = 0.7

# Ignore patterns
ignore_test_files = true
ignore_frameworks = []
custom_async_resources = []
```

### Migration Path

1. **Default disabled**: Initial release with feature flag
2. **Opt-in period**: Users enable via configuration
3. **Gradual rollout**: Enable by default after validation
4. **Feedback integration**: Adjust patterns based on user feedback

## Success Metrics

- **Detection rate**: 90%+ on test suite of 100 known async bugs
- **False positive rate**: < 15%
- **Performance overhead**: < 10% on Python analysis time
- **User adoption**: 40%+ of Python projects enable detection
- **Bug prevention**: Catch 50+ async bugs in real codebases within 3 months

## Future Enhancements

1. **Trio support**: Extend to trio async framework
2. **Custom pattern DSL**: Allow users to define custom async patterns
3. **Auto-fix suggestions**: Generate code fixes for detected patterns
4. **IDE integration**: Real-time async pattern detection in editors
5. **Async call graph**: Track task dependencies and communication
6. **Deadlock detection**: Identify potential async deadlock patterns
