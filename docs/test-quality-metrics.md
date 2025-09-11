# Python Test Quality Metrics

## Overview

DebtMap analyzes Python test files to identify quality issues that can impact test reliability, maintainability, and effectiveness. This guide explains the metrics collected and how to interpret them.

## Detected Issues

### 1. Missing Assertions

**Issue Type:** `NoAssertions`  
**Severity:** High

Tests without assertions don't verify behavior and provide false confidence. DebtMap detects when a test has setup or action code but no assertions.

**Example:**
```python
def test_calculate_total():
    items = [10, 20, 30]
    total = sum(items)
    # Missing assertion - test doesn't verify the result
```

**Fix:**
```python
def test_calculate_total():
    items = [10, 20, 30]
    total = sum(items)
    assert total == 60  # Now the test verifies behavior
```

### 2. Excessive Mocking

**Issue Type:** `ExcessiveMocking(count)`  
**Severity:** Medium  
**Default Threshold:** 5 mocks per test

Tests with too many mocks become brittle and may not test real behavior. DebtMap counts mock decorators and inline mock creation.

**Example of excessive mocking:**
```python
@mock.patch('module.service1')
@mock.patch('module.service2')
@mock.patch('module.service3')
@mock.patch('module.service4')
@mock.patch('module.service5')
@mock.patch('module.service6')
def test_complex_operation(mock1, mock2, mock3, mock4, mock5, mock6):
    # Too many mocks - consider integration testing or refactoring
    pass
```

**Better approach:**
```python
def test_complex_operation():
    # Use fewer mocks, test real interactions where possible
    with mock.patch('module.critical_service') as mock_service:
        result = perform_operation()
        assert result.status == 'success'
```

### 3. Overly Complex Tests

**Issue Type:** `OverlyComplex(complexity_score)`  
**Severity:** Medium  
**Default Threshold:** 10

Complex tests are hard to understand and maintain. Complexity is measured by:
- Nested conditionals and loops
- Number of branches
- Depth of nesting

**Example of complex test:**
```python
def test_complex_scenario():
    for item in items:
        if item.type == 'A':
            if item.status == 'active':
                for sub in item.subitems:
                    if sub.validate():
                        # Deep nesting makes test hard to follow
                        pass
```

**Simplified version:**
```python
def test_handles_active_type_a_items():
    active_items = [i for i in items if i.type == 'A' and i.status == 'active']
    valid_subitems = extract_valid_subitems(active_items)
    assert len(valid_subitems) == expected_count

def extract_valid_subitems(items):
    # Helper function reduces test complexity
    return [sub for item in items for sub in item.subitems if sub.validate()]
```

### 4. Poor Test Isolation

**Issue Type:** `PoorIsolation`  
**Severity:** High

Tests that modify global state without cleanup can cause test interdependencies and flaky failures.

**Example of poor isolation:**
```python
def test_updates_configuration():
    global CONFIG
    CONFIG['debug'] = True  # Modifies global state
    result = process_with_config()
    assert result.debug_enabled
    # No cleanup - affects other tests
```

**Properly isolated test:**
```python
def test_updates_configuration():
    original_config = CONFIG.copy()
    try:
        CONFIG['debug'] = True
        result = process_with_config()
        assert result.debug_enabled
    finally:
        CONFIG.update(original_config)  # Restore state
```

### 5. Flaky Test Patterns

**Issue Type:** `FlakyPattern`  
**Severity:** High

Certain patterns commonly lead to intermittent test failures:
- Time-dependent assertions without mocking
- Random data without seeds
- Network calls without mocking
- File system operations without cleanup

**Example of flaky pattern:**
```python
def test_timestamp():
    record = create_record()
    assert record.created_at == datetime.now()  # Flaky - time may change
```

**Stable version:**
```python
@freeze_time("2024-01-01 12:00:00")
def test_timestamp():
    record = create_record()
    assert record.created_at == datetime(2024, 1, 1, 12, 0, 0)
```

## Configuration

Test quality thresholds can be configured in your project:

```python
# In your analysis configuration
analyzer = PythonTestAnalyzer.with_threshold(
    complexity_threshold=15,  # Allow more complex tests
    max_mocks=7,             # Allow more mocks per test
)
```

## Best Practices

1. **Every test should have assertions** - Verify the behavior you're testing
2. **Keep tests simple** - Complex tests are harder to debug when they fail
3. **Minimize mocking** - Test real behavior where possible
4. **Ensure test isolation** - Tests should not affect each other
5. **Avoid flaky patterns** - Make tests deterministic and repeatable

## Framework Support

DebtMap automatically detects and adapts to your testing framework:

- **pytest**: Native assert statements, fixtures, pytest.raises
- **unittest**: self.assert* methods, setUp/tearDown
- **nose**: assert_* functions, with_setup decorators
- **doctest**: Inline documentation tests

## Interpreting Results

When reviewing test quality issues:

1. **Prioritize High severity issues** - These directly impact test reliability
2. **Look for patterns** - Multiple similar issues may indicate systemic problems
3. **Consider context** - Some complex scenarios legitimately require complex tests
4. **Track improvement** - Monitor test quality metrics over time

## Integration with CI/CD

Test quality metrics can be integrated into your CI pipeline:

```bash
# Fail build if test quality issues exceed threshold
debtmap analyze --test-quality --fail-on-high-severity

# Generate test quality report
debtmap analyze --test-quality --output test-quality-report.json
```

## Common Fixes

### Converting to Better Patterns

**From imperative to declarative:**
```python
# Before - Imperative style with complex flow
def test_process():
    data = []
    for i in range(10):
        if i % 2 == 0:
            data.append(i * 2)
    result = process(data)
    assert len(result) == 5

# After - Declarative with clear intent
def test_process_even_numbers():
    even_numbers = [i * 2 for i in range(10) if i % 2 == 0]
    result = process(even_numbers)
    assert len(result) == 5
```

**From mocks to test doubles:**
```python
# Before - Many mocks
@mock.patch('db.connection')
@mock.patch('cache.client')
@mock.patch('queue.publisher')
def test_service(mock_queue, mock_cache, mock_db):
    # Complex mock setup
    pass

# After - Test double
def test_service():
    test_env = TestEnvironment()  # Encapsulates test dependencies
    service = Service(test_env)
    result = service.process()
    assert result.success
```

## Further Reading

- [Python Testing Best Practices](https://docs.pytest.org/en/latest/goodpractices.html)
- [Test Pyramid](https://martinfowler.com/bliki/TestPyramid.html)
- [Mocking Best Practices](https://docs.python.org/3/library/unittest.mock.html)