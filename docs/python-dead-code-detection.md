# Python Dead Code Detection

Debtmap's Python dead code detection system uses advanced static analysis to identify unused functions with high accuracy and low false positive rates.

## Overview

The dead code analyzer integrates multiple detection systems:

- **Static call graph analysis** - Tracks which functions call each other
- **Framework pattern detection** - Recognizes Flask routes, Django views, Click commands, etc.
- **Test detection** - Identifies test functions and test files
- **Callback tracking** - Detects functions registered as callbacks or event handlers
- **Import analysis** - Tracks which functions are imported by other modules
- **Coverage integration** - Uses test coverage data when available

## Confidence Scoring

Results include confidence scores to help you make informed decisions:

### High Confidence (0.8-1.0)
**Safe to remove** - These functions are very likely dead code.

Characteristics:
- No static callers found
- Not a framework entry point
- Not a test function
- Not registered as a callback
- Not exported in `__all__`
- Private function (starts with `_`)

Example output:
```
Function: _old_helper
Confidence: High (0.95)
Suggestion: High confidence this function is dead code and can be safely removed.
```

### Medium Confidence (0.5-0.8)
**Review recommended** - These functions might be dead code but require manual verification.

Characteristics:
- No static callers but is public
- In a test file but not called
- Might be used dynamically

Example output:
```
Function: legacy_api_method
Confidence: Medium (0.65)
Suggestion: Medium confidence this function is dead code. Manual verification recommended.
Risks:
  - Function is public and may be used by external code.
```

### Low Confidence (0.0-0.5)
**Likely in use** - These functions are probably not dead code.

Characteristics:
- Has static callers
- Framework entry point
- Test function
- Callback target
- Magic method
- Property accessor

Example output:
```
Function: index
Confidence: Low (0.15)
Result: LIVE
Reasons:
  - Framework entry point (Flask route)
  - Function is public
```

## Suppressing False Positives

Mark functions as intentionally unused with suppression comments:

```python
# debtmap: not-dead
def future_api_endpoint():
    """Will be activated in v2.0"""
    pass

def compatibility_shim():  # noqa: dead-code
    """Kept for backwards compatibility"""
    pass
```

Supported comment formats:
- `# debtmap: not-dead`
- `# debtmap:not-dead`
- `# noqa: dead-code`
- `# noqa:dead-code`

Comments can appear:
- On the line above the function definition
- On the same line as the function definition
- On the line below the function definition

## Framework Support

The analyzer recognizes entry points from popular Python frameworks:

### Web Frameworks
- **Flask**: `@app.route`, `@app.before_request`, etc.
- **Django**: Views, admin actions, signal handlers
- **FastAPI**: `@app.get`, `@app.post`, etc.

### CLI Frameworks
- **Click**: `@click.command`, `@click.group`
- **argparse**: Functions used as subcommand handlers

### Testing Frameworks
- **pytest**: Functions starting with `test_`, fixtures
- **unittest**: `TestCase` methods, `setUp`, `tearDown`

### Event Systems
- **Qt/PyQt**: Signal connections, slot decorators
- **Tkinter**: Event bindings, button commands

## Coverage Integration

When test coverage data is available, the analyzer uses it to improve accuracy:

```bash
# Generate coverage data with pytest
pytest --cov=myapp --cov-report=json

# Debtmap will automatically use coverage.json if present
debtmap analyze myapp/
```

Functions that appear in coverage data are considered live, even if no static callers are found.

## Understanding Results

### Example Analysis Output

```
Dead code analysis for 'calculate_total':
  Result: LIVE
  Confidence: Low (0.2)

  Reasons it's LIVE:
    - HasStaticCallers
    - PublicApi

  Suggestion:
    Function appears to be in use or is a framework/test entry point.
```

### Interpretation Guide

**Is dead: false, Confidence: Low**
→ Function is in use, keep it

**Is dead: true, Confidence: High**
→ Safe to remove, very likely unused

**Is dead: true, Confidence: Medium**
→ Review manually before removing

**Is dead: false, Confidence: Medium**
→ Might be used dynamically, investigate further

## Common Patterns

### False Positives

**Public API methods**
```python
class Calculator:
    def add(self, a, b):  # Might be used by external code
        return a + b
```
Mitigation: Mark with suppression comment or ensure proper `__all__` export

**Dynamic imports**
```python
# Module loaded dynamically
def handle_command(cmd):  # Called via getattr()
    pass
```
Mitigation: Use suppression comment

**Decorators that register functions**
```python
@registry.register
def handler():  # Registered at import time
    pass
```
Mitigation: Callback tracker should detect this pattern

### True Positives

**Old implementations**
```python
def _old_calculate(x):  # Replaced but not removed
    return x * 2
```
Action: Safe to remove

**Unused helper functions**
```python
def _format_date(date):  # Was used but caller removed
    return date.strftime("%Y-%m-%d")
```
Action: Safe to remove

**Commented-out code alternatives**
```python
def process_v1(data):  # Old version, v2 is now used
    pass
```
Action: Safe to remove

## Best Practices

1. **Start with high confidence items** - Remove these first to build confidence in the tool

2. **Review medium confidence items** - These require manual verification but often find real dead code

3. **Use suppression comments liberally** - Better to mark something as intentionally unused than to have noise

4. **Run with coverage data** - This significantly improves accuracy for dynamically-called functions

5. **Check git history** - Before removing, verify the function wasn't recently added or hasn't been used in the last few releases

6. **Remove incrementally** - Remove a few functions, run tests, commit. Don't remove everything at once.

7. **Look for patterns** - If multiple related functions are flagged, they might all be part of an abandoned feature

## Limitations

### What the analyzer CAN detect:
- Static function calls
- Framework entry points via decorators
- Test functions
- Callback registrations
- Functions in `__all__` exports
- Property decorators
- Magic methods

### What the analyzer CANNOT detect:
- `eval()` or `exec()` usage
- `getattr()` with dynamic string names
- Reflection-based calls
- Functions called from C extensions
- Plugin systems using string-based loading

For these cases, use suppression comments.

## Troubleshooting

### "Function marked as dead but it's actually used"

Possible causes:
1. Dynamic call via `getattr()` → Add suppression comment
2. Called from a plugin → Add suppression comment
3. Framework pattern not recognized → Report issue
4. Callback not detected → Check if decorator is supported

### "Many false positives in my codebase"

Solutions:
1. Run with coverage data: `pytest --cov-report=json`
2. Check framework patterns are recognized
3. Add suppression comments to public API
4. Consider if functions are truly unused

### "Low confidence on obviously dead code"

This is working as intended. The analyzer is conservative to avoid false positives. Review the "Reasons it's LIVE" to understand why confidence is low.

## Migration from Previous Version

If you were using debtmap's previous dead code detection:

### Key Changes

1. **Confidence scores** - Now returns High/Medium/Low confidence levels
2. **Fewer false positives** - Integrates framework and callback detection
3. **Suppression comments** - New way to mark code as intentionally unused
4. **Coverage integration** - Can use pytest-cov data

### Output Format Changes

Old format:
```json
{
  "dead_functions": ["func1", "func2"],
  "unused_count": 2
}
```

New format:
```json
{
  "function": "func1",
  "is_dead": true,
  "confidence": "High",
  "confidence_score": 0.95,
  "reasons": ["NoStaticCallers", "PrivateFunction"],
  "can_remove": true,
  "safe_to_remove": true
}
```

See [migration-dead-code-detection.md](migration-dead-code-detection.md) for detailed migration guide.

## Examples

### Example 1: Flask Application

```python
from flask import Flask
app = Flask(__name__)

@app.route('/')
def index():  # Detected as framework entry point → LIVE
    return helper()

def helper():  # Has caller (index) → LIVE
    return "Hello"

def _old_route():  # No callers, not a route → DEAD (High confidence)
    return "Unused"
```

### Example 2: Test File

```python
import pytest

def test_addition():  # Test function → LIVE
    assert add(1, 2) == 3

def add(a, b):  # Called by test → LIVE
    return a + b

def _unused_helper():  # No callers → DEAD (High confidence)
    return 42
```

### Example 3: Public API

```python
__all__ = ['calculate', 'format_result']

def calculate(x):  # Exported in __all__ → LIVE
    return x * 2

def format_result(x):  # Exported in __all__ → LIVE
    return f"Result: {x}"

def _internal_helper():  # Not exported, no callers → DEAD (High confidence)
    return None
```

## Getting Help

- Report issues: https://github.com/anthropics/debtmap/issues
- Documentation: https://docs.debtmap.dev
- Examples: https://github.com/anthropics/debtmap/tree/main/examples
