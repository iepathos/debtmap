# Dead Code Analysis

Debtmap's Python dead code detection system uses advanced static analysis to identify unused functions with high accuracy and low false positive rates. The analyzer integrates multiple detection systems to provide confidence-scored results that help you make informed decisions about code removal.

## Overview

The dead code analyzer combines several detection techniques:

- **Static call graph analysis** - Tracks which functions call each other across your codebase
- **Framework pattern detection** - Recognizes entry points from Flask, Django, FastAPI, Click, pytest, and more
- **Test detection** - Identifies test functions and test files to avoid false positives
- **Callback tracking** - Detects functions registered as callbacks or event handlers
- **Import analysis** - Tracks which functions are imported and exported by other modules
- **Coverage integration** - Uses test coverage data when available to identify live code
- **Public API detection** - Uses heuristics to identify external API functions

This multi-layered approach achieves a target false positive rate of less than 10%, compared to 30-40% for naive call graph analysis.

## Confidence Scoring

Every analysis result includes a confidence score to help you prioritize code removal:

### High Confidence (0.8-1.0)

**Safe to remove** - These functions are very likely dead code.

Characteristics:
- No static callers found in the codebase
- Not a framework entry point (route, command, view, etc.)
- Not a test function or in a test file
- Not registered as a callback or event handler
- Not exported in `__all__` or used in public API patterns
- Often private functions (starting with `_`)

Example output:
```
Function: _old_helper
Confidence: High (0.95)
Suggestion: High confidence this function is dead code and can be safely removed.
```

### Medium Confidence (0.5-0.8)

**Review recommended** - These functions might be dead code but require manual verification.

Characteristics:
- No static callers but function is public
- In a test file but not called by any tests
- Might be used dynamically (via `getattr`, plugins, etc.)
- Public API that might be used by external code

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
- Has static callers in the codebase
- Framework entry point (Flask route, Django view, Click command)
- Test function (starts with `test_`, in test file)
- Callback target or event handler
- Magic method (`__init__`, `__str__`, etc.)
- Property accessor or descriptor

Example output:
```
Function: index
Confidence: Low (0.15)
Result: LIVE
Reasons:
  - Framework entry point (Flask route)
  - Function is public
```

## Public API Detection

Debtmap uses advanced heuristics to identify functions that are likely part of your project's external API (introduced in Spec 113). This prevents false positives when analyzing library code.

### Detection Heuristics

The public API detector considers:

1. **Public visibility** - Function doesn't start with `_`
2. **File location patterns** - Functions in `api/`, `public/`, or top-level `__init__.py` files
3. **Naming conventions** - Functions following API naming patterns
4. **Export declarations** - Functions listed in `__all__`
5. **Explicit configuration** - Functions marked as API in `.debtmap.toml`

### Configuration

Configure public API detection in `.debtmap.toml`:

```toml
[external_api]
# Enable/disable automatic public API detection (default: true)
detect_external_api = true

# Explicitly mark specific functions as external APIs
api_functions = [
    "calculate_score",           # Just function name
    "mylib.api::process_data",   # Module-qualified name
]

# Mark entire files as containing external APIs (supports glob patterns)
api_files = [
    "src/api/*.py",              # All files in api directory
    "src/public_interface.py",   # Specific file
    "**/__init__.py",            # All __init__.py files
]
```

Functions identified as public APIs receive lower dead code confidence scores, even if they have no internal callers.

## Framework Support

The analyzer recognizes entry points from popular Python frameworks to avoid false positives:

### Web Frameworks

- **Flask**: `@app.route`, `@app.before_request`, `@app.after_request`, `@app.errorhandler`
- **Django**: View functions, admin actions, signal handlers, middleware methods
- **FastAPI**: `@app.get`, `@app.post`, `@app.put`, `@app.delete`, route decorators

### CLI Frameworks

- **Click**: `@click.command`, `@click.group`, subcommand handlers
- **argparse**: Functions registered as subcommand handlers

### Testing Frameworks

- **pytest**: Functions starting with `test_`, `@pytest.fixture`, parametrized tests
- **unittest**: `TestCase` methods, `setUp`, `tearDown`, `setUpClass`, `tearDownClass`

### Event Systems

- **Qt/PyQt**: Signal connections, slot decorators
- **Tkinter**: Event bindings, button command callbacks

See [Framework Patterns documentation](context-providers.md) for comprehensive framework support details.

## Confidence Thresholds

You can customize confidence thresholds based on your project's tolerance for false positives vs. false negatives:

```rust
use debtmap::analysis::python_dead_code_enhanced::AnalysisConfig;

let config = AnalysisConfig {
    high_confidence_threshold: 0.8,      // Default: 0.8
    medium_confidence_threshold: 0.5,    // Default: 0.5
    respect_suppression_comments: true,  // Default: true
    include_private_api: true,           // Default: true
    enable_public_api_detection: true,   // Default: true (Spec 113)
    ..Default::default()
};
```

**Tuning recommendations:**

- **Conservative projects** (libraries, public APIs): Raise thresholds to 0.9/0.7 to reduce false positives
- **Aggressive cleanup** (internal tools): Lower thresholds to 0.7/0.4 to catch more dead code
- **Balanced approach** (most projects): Use defaults of 0.8/0.5

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

### Supported Comment Formats

All of these formats are recognized:

- `# debtmap: not-dead` (recommended)
- `# debtmap:not-dead`
- `# noqa: dead-code`
- `# noqa:dead-code`

### Comment Placement

Suppression comments can appear:

- **Above the function** (most common):
  ```python
  # debtmap: not-dead
  def my_function():
      pass
  ```

- **Same line as function definition**:
  ```python
  def my_function():  # debtmap: not-dead
      pass
  ```

- **Below the function definition** (less common):
  ```python
  def my_function():
  # debtmap: not-dead
      pass
  ```

## Coverage Integration

When test coverage data is available, the analyzer uses it to dramatically improve accuracy by marking covered functions as live:

### Generating Coverage Data

```bash
# With pytest and pytest-cov
pytest --cov=myapp --cov-report=json

# With coverage.py directly
coverage run -m pytest
coverage json

# Debtmap automatically detects and uses coverage.json
debtmap analyze myapp/
```

### How It Works

Functions that appear in coverage data are considered live, even if:
- No static callers are found
- They're private functions
- They're not framework entry points

This catches functions called:
- Dynamically via `getattr()` or `exec()`
- Through plugin systems
- By external libraries or C extensions

### Accuracy Improvement

Coverage integration typically provides:
- **60-70% reduction** in false positives for complex codebases
- **Near-zero false positives** for functions with test coverage
- **Confidence in removal** for uncovered code

## Configuration Reference

Complete dead code analysis configuration in `.debtmap.toml`:

```toml
# External API detection (Spec 113)
[external_api]
detect_external_api = true

api_functions = [
    "public_function_name",
    "module::qualified_name",
]

api_files = [
    "src/api/**/*.py",           # Glob patterns supported
    "src/public_interface.py",   # Exact file paths
]

# Note: Confidence thresholds are configured programmatically
# via AnalysisConfig in the Rust API
```

## Understanding Results

### Interpreting Output

When you run dead code analysis, you'll see results like:

```
Dead code analysis for 'calculate_total':
  Result: LIVE
  Confidence: Low (0.2)

  Reasons it's LIVE:
    - HasStaticCallers (called by 3 functions)
    - PublicApi

  Suggestion:
    Function appears to be in use or is a framework/test entry point.
```

### Decision Guide

| Result | Confidence | Action |
|--------|-----------|--------|
| `is_dead: true` | High (0.8-1.0) | **Safe to remove** - Very likely unused |
| `is_dead: true` | Medium (0.5-0.8) | **Review manually** - Might be dead, verify first |
| `is_dead: true` | Low (0.0-0.5) | **Keep** - Likely used dynamically |
| `is_dead: false` | Any | **Keep** - Function is in use |

### CLI Filtering by Confidence

```bash
# Show only high confidence dead code
debtmap analyze --min-confidence=0.8

# Show high and medium confidence
debtmap analyze --min-confidence=0.5

# Show all results (including low confidence)
debtmap analyze --min-confidence=0.0
```

See [CLI Reference](cli-reference.md) for complete command options.

## Common Patterns

### False Positives (and How to Handle Them)

**Public API methods**
```python
class Calculator:
    def add(self, a, b):  # Might be used by external code
        return a + b
```
*Solution*: Add to `api_functions` in `.debtmap.toml` or use suppression comment

**Dynamic imports**
```python
# Module loaded dynamically via importlib
def handle_command(cmd):  # Called via getattr()
    pass
```
*Solution*: Add `# debtmap: not-dead` suppression comment

**Plugin registration**
```python
@registry.register
def handler():  # Registered at import time
    pass
```
*Solution*: Should be detected by callback tracker; if not, add suppression comment

### True Positives (Safe to Remove)

**Old implementations**
```python
def _old_calculate(x):  # Replaced but not removed
    return x * 2
```
*Action*: Safe to remove (high confidence)

**Unused helper functions**
```python
def _format_date(date):  # Was used but caller was removed
    return date.strftime("%Y-%m-%d")
```
*Action*: Safe to remove (high confidence)

**Commented-out code alternatives**
```python
def process_v1(data):  # Old version, v2 is now used
    pass
```
*Action*: Safe to remove (high confidence)

## Best Practices

### Workflow Recommendations

1. **Start with high confidence items** - Remove functions with 0.8+ confidence first to build trust in the tool

2. **Run with coverage data** - Generate `coverage.json` to dramatically improve accuracy:
   ```bash
   pytest --cov=myapp --cov-report=json
   debtmap analyze myapp/
   ```

3. **Review medium confidence items** - These often find real dead code but need manual verification

4. **Use suppression comments liberally** - Better to mark something as intentionally unused than to have noise in results

5. **Check git history** - Before removing, verify the function wasn't recently added:
   ```bash
   git log -p -- path/to/file.py | grep -A5 "def function_name"
   ```

6. **Remove incrementally** - Remove a few functions, run tests, commit. Don't remove everything at once:
   ```bash
   # Remove 3-5 high confidence functions
   pytest  # Verify tests still pass
   git commit -m "Remove dead code: _old_helper, _unused_formatter"
   ```

7. **Look for patterns** - If multiple related functions are flagged, they might all be part of an abandoned feature

### CI/CD Integration

Prevent dead code from accumulating by integrating into your CI pipeline:

```bash
# .github/workflows/dead-code.yml
- name: Check for dead code
  run: |
    debtmap analyze --min-confidence=0.8 --format=json > dead-code.json
    # Fail if high-confidence dead code is found
    if [ $(jq '.dead_code | length' dead-code.json) -gt 0 ]; then
      echo "High-confidence dead code detected!"
      jq '.dead_code[] | "\(.file):\(.line) - \(.function)"' dead-code.json
      exit 1
    fi
```

## Limitations

### What the Analyzer CAN Detect

- ✅ Static function calls across modules
- ✅ Framework entry points via decorators
- ✅ Test functions in test files
- ✅ Callback registrations and event handlers
- ✅ Functions in `__all__` exports
- ✅ Property decorators and descriptors
- ✅ Magic methods (`__init__`, `__str__`, etc.)
- ✅ Functions covered by test coverage data

### What the Analyzer CANNOT Detect

- ❌ `eval()` or `exec()` usage - arbitrary code execution
- ❌ `getattr()` with dynamic string names - runtime attribute lookup
- ❌ Reflection-based calls - `inspect` module usage
- ❌ Functions called from C extensions
- ❌ Plugin systems using string-based loading - dynamic imports

### Mitigation Strategies

For functions the analyzer cannot detect, use suppression comments:

```python
# Called dynamically via getattr in plugin system
# debtmap: not-dead
def handle_dynamic_command():
    pass

# Loaded via string-based plugin system
# debtmap: not-dead
def plugin_entrypoint():
    pass
```

## Troubleshooting

### "Function marked as dead but it's actually used"

**Possible causes and solutions:**

1. **Dynamic call via `getattr()` or `exec()`**
   - *Solution*: Add `# debtmap: not-dead` suppression comment
   - *Example*: Plugin systems, command dispatchers

2. **Called from external code or C extension**
   - *Solution*: Add function to `api_functions` in `.debtmap.toml`
   - *Example*: Public library APIs

3. **Framework pattern not recognized**
   - *Solution*: Report issue on GitHub with framework details
   - *Workaround*: Add suppression comment

4. **Callback registration not detected**
   - *Solution*: Check if decorator is supported; add suppression if not
   - *Example*: Custom registration decorators

### "Too many false positives in my codebase"

**Solutions to try (in order):**

1. **Run with coverage data** - Biggest impact on accuracy:
   ```bash
   pytest --cov=myapp --cov-report=json
   debtmap analyze myapp/
   ```

2. **Configure public API detection** - Mark your external APIs:
   ```toml
   [external_api]
   api_files = ["src/api/**/*.py", "src/public/**/*.py"]
   ```

3. **Add framework patterns** - Report unrecognized frameworks on GitHub

4. **Add suppression comments** - Mark intentionally unused functions

5. **Adjust confidence thresholds** - Raise to 0.9/0.7 for conservative analysis

### "Low confidence on obviously dead code"

This is working as intended - the analyzer is **conservative** to avoid false positives.

**What to do:**

1. **Review the "Reasons it's LIVE"** - Understand why confidence is low
2. **Check if function is truly unused** - Verify no dynamic calls
3. **Run with coverage** - Coverage data will increase confidence for truly dead code
4. **Accept medium/low confidence** - Manual review is valuable for complex cases

## Examples

### Example 1: Flask Application

```python
from flask import Flask
app = Flask(__name__)

@app.route('/')
def index():  # ✅ LIVE - Framework entry point
    return helper()

def helper():  # ✅ LIVE - Called by index()
    return format_response("Hello")

def format_response(msg):  # ✅ LIVE - Called by helper()
    return f"<html>{msg}</html>"

def _old_route():  # ❌ DEAD - No callers, not a route (High: 0.95)
    return "Unused"
```

**Analysis results:**
- `index`: LIVE (Low: 0.15) - Flask route decorator detected
- `helper`: LIVE (Low: 0.25) - Has static caller (index)
- `format_response`: LIVE (Low: 0.30) - Has static caller (helper)
- `_old_route`: DEAD (High: 0.95) - No callers, private function

### Example 2: Test File

```python
import pytest

def test_addition():  # ✅ LIVE - Test function
    assert add(1, 2) == 3

def add(a, b):  # ✅ LIVE - Called by test
    return a + b

@pytest.fixture
def sample_data():  # ✅ LIVE - pytest fixture
    return [1, 2, 3]

def _unused_helper():  # ❌ DEAD - No callers (High: 0.90)
    return 42

def _old_test_helper():  # ❌ DEAD - Was used, now orphaned (High: 0.92)
    return "test data"
```

**Analysis results:**
- `test_addition`: LIVE (Low: 0.10) - Test function pattern
- `add`: LIVE (Low: 0.20) - Called by test
- `sample_data`: LIVE (Low: 0.15) - pytest fixture decorator
- `_unused_helper`: DEAD (High: 0.90) - No callers in test file
- `_old_test_helper`: DEAD (High: 0.92) - Orphaned helper

### Example 3: Public API with Configuration

```python
# src/api/calculator.py

__all__ = ['calculate', 'format_result']

def calculate(x):  # ✅ LIVE - Exported in __all__
    return _internal_multiply(x, 2)

def format_result(x):  # ✅ LIVE - Exported in __all__
    return f"Result: {x}"

def _internal_multiply(a, b):  # ✅ LIVE - Called by calculate
    return a * b

def _internal_helper():  # ❌ DEAD - Not exported, no callers (High: 0.88)
    return None

# Public API but not in __all__
def legacy_api():  # ⚠️ MEDIUM - Public but no callers (Medium: 0.65)
    """Kept for backwards compatibility"""
    pass
```

**.debtmap.toml configuration:**
```toml
[external_api]
api_files = ["src/api/**/*.py"]

# Explicitly mark legacy API
api_functions = ["legacy_api"]
```

**Analysis results:**
- `calculate`: LIVE (Low: 0.20) - In `__all__`, has callers
- `format_result`: LIVE (Low: 0.25) - In `__all__`
- `_internal_multiply`: LIVE (Low: 0.30) - Called by calculate
- `_internal_helper`: DEAD (High: 0.88) - Private, no callers
- `legacy_api`: LIVE (Low: 0.35) - Marked as API in config

## Getting Help

- **Documentation**: See [Troubleshooting Guide](troubleshooting.md) for common issues
- **Report issues**: https://github.com/anthropics/debtmap/issues
- **Examples**: Check the [Examples chapter](examples.md) for more scenarios
- **Related topics**:
  - [Coverage Integration](coverage-integration.md) - Detailed coverage setup
  - [Suppression Patterns](suppression-patterns.md) - Advanced suppression techniques
  - [Configuration](configuration.md) - Complete configuration reference
