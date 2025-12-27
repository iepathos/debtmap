# Dead Code Analysis

Debtmap's dead code detection system uses advanced static analysis to identify unused functions with high accuracy and low false positive rates. The analyzer integrates multiple detection systems to provide confidence-scored results that help you make informed decisions about code removal.

## Overview

The dead code analyzer combines several detection techniques:

- **Static call graph analysis** - Tracks which functions call each other across your codebase
- **Framework pattern detection** - Recognizes entry points from Flask, Django, FastAPI, Click, pytest, and more
- **Test detection** - Identifies test functions and test files to avoid false positives
- **Callback tracking** - Detects functions registered as callbacks or event handlers
- **Import analysis** - Tracks which functions are imported and exported by other modules
- **Coverage integration** - Uses test coverage data when available to identify live code
- **Public API detection** - Uses heuristics to identify external API functions

This multi-layered approach significantly reduces false positives compared to naive call graph analysis, reducing false positives from 30% to less than 5% for library-style modules (see `src/debt/public_api_detector.rs:1-5` for implementation details).

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

Debtmap uses advanced heuristics to identify functions that are likely part of your project's external API. This prevents false positives when analyzing library code by detecting public-facing functions and excluding them from dead code results.

### Detection Heuristics

The public API detector (`src/debt/public_api_detector.rs:325-650`) implements six weighted heuristics:

1. **Naming Convention Heuristic** (weight: 0.30) - Functions without underscore prefix are likely public; functions starting with `_` are marked private (score: 0.0). Module-level functions without underscore score highest.

2. **Docstring Quality Heuristic** (weight: 0.25) - Functions with comprehensive docstrings (structured with `Args:`, `Returns:`, etc.) are likely public API. Longer docstrings (100+ chars) score higher.

3. **Type Annotation Heuristic** (weight: 0.15) - Fully type-annotated functions (parameters and return type) indicate public API quality.

4. **Symmetric Pair Heuristic** (weight: 0.20) - Paired operations like `load`/`save`, `get`/`set`, `open`/`close`, `create`/`destroy`, `start`/`stop`, `acquire`/`release`, `add`/`remove`, `push`/`pop`, `read`/`write`. If one function in a pair is used, its counterpart is likely public.

5. **Module Export Heuristic** (weight: 0.10) - Functions listed in Python's `__all__` or exported in `__init__.py` are definitely public (score: 1.0).

6. **Rust Visibility Heuristic** - For Rust code, `pub` keyword is definitive (score: 1.0), `pub(crate)` scores 0.5, and trait implementations are never considered dead code.

Functions scoring above the threshold (default: 0.6) are marked as public API and excluded from dead code detection.

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
    "public_handler",            # Any function matching this name
]

# Mark entire files as containing external APIs (supports glob patterns)
api_files = [
    "src/api/**/*.py",           # All Python files in api directory recursively
    "src/lib.rs",                # Rust library entry point (all public functions)
    "src/public_interface.py",   # Specific Python file
    "**/__init__.py",            # All __init__.py files in any directory
    "**/public_*.py",            # Any file starting with 'public_'
    "myapp/api.py",              # Specific API module
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

- **Qt/PyQt**: Signal connections, slot decorators (`@pyqtSlot`)
- **Tkinter**: Event bindings, button command callbacks, widget event handlers

### Framework Detection Matrix

| Framework | Pattern | Decorator/Keyword | Detection Method | Example |
|-----------|---------|-------------------|------------------|---------|
| **Flask** | Routes | `@app.route` | Decorator analysis | `@app.route('/')` |
| Flask | Before request | `@app.before_request` | Decorator analysis | Handler hooks |
| Flask | Error handlers | `@app.errorhandler` | Decorator analysis | Custom error pages |
| **Django** | Views | Function-based views | Module structure | `def my_view(request):` |
| Django | Admin actions | `@admin.action` | Decorator analysis | Admin panel actions |
| Django | Signals | `@receiver` | Decorator analysis | Signal handlers |
| **FastAPI** | Routes | `@app.get`, `@app.post` | Decorator analysis | REST endpoints |
| FastAPI | Dependencies | `Depends()` | Call graph analysis | Dependency injection |
| **Click** | Commands | `@click.command` | Decorator analysis | CLI commands |
| Click | Groups | `@click.group` | Decorator analysis | Command groups |
| **pytest** | Tests | `test_*` prefix | Naming convention | `def test_foo():` |
| pytest | Fixtures | `@pytest.fixture` | Decorator analysis | Test fixtures |
| **unittest** | Tests | `TestCase` methods | Class hierarchy | `class TestFoo(TestCase):` |
| unittest | Setup/Teardown | `setUp`, `tearDown` | Method naming | Lifecycle methods |
| **Qt/PyQt** | Slots | `@pyqtSlot` | Decorator analysis | Signal handlers |
| Qt | Connections | `.connect()` calls | Call graph analysis | Event wiring |
| **Tkinter** | Callbacks | `command=func` | Assignment tracking | Button callbacks |

See [Framework Patterns documentation](context-providers.md) for comprehensive framework support details and language-specific patterns.

## Confidence Thresholds

Dead code detection uses internal confidence thresholds to classify results:

| Confidence Level | Range | Meaning |
|-----------------|-------|---------|
| **High** | 0.8 - 1.0 | Safe to remove - very likely dead code |
| **Medium** | 0.5 - 0.8 | Review recommended - manual verification needed |
| **Low** | 0.0 - 0.5 | Likely in use - keep the function |

These thresholds are applied internally during analysis. Functions exceeding the high confidence threshold (0.8) with no callers and no framework indicators are flagged as removable.

**Tuning recommendations:**

- **Conservative projects** (libraries, public APIs): Focus on high confidence results (0.8+) only
- **Aggressive cleanup** (internal tools): Review medium confidence results (0.5+) for additional cleanup opportunities
- **Balanced approach** (most projects): Start with high confidence, then review medium as needed

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

### How Coverage is Used Internally

Coverage data is integrated through the priority scoring system (`src/priority/scoring/classification.rs:71-88`). Functions with coverage data are evaluated using `TransitiveCoverage`:

- **Direct coverage** - Percentage of function lines executed by tests
- **Transitive coverage** - Coverage inherited from callers
- **Uncovered lines** - Specific lines not executed

Functions with direct coverage above 80% are considered well-tested and unlikely to be dead code.

**CLI usage** - Coverage is loaded automatically via the `--coverage-file` flag:

```bash
# Generate coverage data first
pytest --cov=myapp --cov-report=json

# Run debtmap with coverage
debtmap analyze myapp/ --coverage-file coverage.json
```

### Accuracy Improvement

Coverage integration substantially improves accuracy by:
- **Significantly reducing false positives** - Eliminates most false positives in complex codebases
- **High accuracy for covered functions** - Functions with test coverage are correctly identified as live
- **Clear removal candidates** - Uncovered code with no static callers is more confidently dead
- **Dynamic call detection** - Catches functions called via `getattr()`, plugins, or other dynamic mechanisms that static analysis misses

**Coverage data format**: Debtmap uses the standard `coverage.json` format produced by `coverage.py` and `pytest-cov`. The file should be in your project root and contain executed line numbers for each source file.

## Configuration Reference

### TOML Configuration

Complete dead code analysis configuration in `.debtmap.toml`:

```toml
# Language-specific dead code detection
# Source: src/config/languages.rs:25-38 (LanguageFeatures struct)
[languages.python]
detect_dead_code = true           # Enable Python dead code analysis (default: true)

[languages.rust]
detect_dead_code = true           # Enable Rust dead code analysis (default: true)

[languages.javascript]
detect_dead_code = true           # Enable JavaScript dead code analysis

[languages.typescript]
detect_dead_code = true           # Enable TypeScript dead code analysis

# External API detection
# Source: src/priority/external_api_detector.rs:10-22 (ExternalApiConfig struct)
[external_api]
detect_external_api = true        # Enable automatic public API detection (default: true)

api_functions = [
    "public_function_name",       # Function name only
    "module::qualified_name",     # Module-qualified format
]

api_files = [
    "src/api/**/*.py",            # Glob patterns supported (** for recursive)
    "src/public_interface.py",    # Exact file paths
    "**/__init__.py",             # All package entry points
    "**/public_*.py",             # Files starting with 'public_'
]
```

### Advanced Configuration: Public API Detection Weights

The `PublicApiConfig` struct (`src/debt/public_api_detector.rs:56-90`) provides fine-tuned control over heuristic weights. These are currently configured programmatically:

| Setting | Default | Description |
|---------|---------|-------------|
| `naming_convention_weight` | 0.30 | Weight for underscore prefix detection |
| `docstring_weight` | 0.25 | Weight for documentation quality |
| `type_annotation_weight` | 0.15 | Weight for type annotation presence |
| `symmetric_pair_weight` | 0.20 | Weight for paired function detection |
| `module_export_weight` | 0.10 | Weight for `__all__` exports |
| `public_api_threshold` | 0.60 | Minimum score to mark as public API |

**Custom Symmetric Pairs** - You can add project-specific paired functions:

```rust
// Source: src/debt/public_api_detector.rs:500-518
// Built-in pairs: load/save, get/set, open/close, create/destroy,
// start/stop, acquire/release, add/remove, push/pop, read/write

// Custom pairs can be added via PublicApiConfig::custom_symmetric_pairs
let config = PublicApiConfig {
    custom_symmetric_pairs: vec![
        ("fetch".to_string(), "submit".to_string()),
        ("serialize".to_string(), "deserialize".to_string()),
    ],
    ..Default::default()
};
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

### Decision Tree for Confidence Interpretation

Use this decision tree to determine what action to take:

```
Is the function flagged as dead?
│
├─ NO → Keep the function (it's in use)
│
└─ YES → What is the confidence level?
    │
    ├─ HIGH (0.8-1.0)
    │   ├─ Is it a public API function? → Review, add suppression comment if keeping
    │   └─ Is it private (_prefix)? → **SAFE TO REMOVE**
    │
    ├─ MEDIUM (0.5-0.8)
    │   ├─ Check git history: recently added? → Keep for now, review in next sprint
    │   ├─ Has coverage data been generated? → Run with coverage first
    │   ├─ Is it used dynamically (getattr, plugins)? → Add suppression comment
    │   └─ No clear reason to keep? → **REVIEW MANUALLY, likely safe to remove**
    │
    └─ LOW (0.0-0.5)
        ├─ Review "Reasons it's LIVE" → If reasons are valid, keep it
        ├─ Function is public and might be external API? → Keep it
        └─ Truly unused but marked live incorrectly? → Report issue or use suppression
```

### Confidence Level Quick Reference

**When to act without review:**
- `is_dead: true` + `confidence: HIGH` + `private function (_prefix)` → **Remove immediately**
- `is_dead: true` + `confidence: HIGH` + `in test file` + `not test function` → **Remove immediately**

**When to review before acting:**
- `is_dead: true` + `confidence: MEDIUM` → **Manual review required**
- `is_dead: true` + `confidence: HIGH` + `public function` → **Check git history, verify external usage**

**When to keep:**
- `is_dead: false` → **Always keep (function is live)**
- `is_dead: true` + `confidence: LOW` → **Keep (too uncertain to remove)**

### Filtering Results by Confidence

To filter dead code results by confidence level, you can process the JSON output:

```bash
# Analyze and output JSON
debtmap analyze --format=json > results.json

# Dead code items are part of the debt_items array with debt_type "DeadCode"
# Filter for dead code items using jq
jq '.debt_items | map(select(.debt_type == "DeadCode"))' results.json

# Filter by visibility (Public, Private, Crate)
jq '.debt_items | map(select(.debt_type == "DeadCode" and .visibility == "Private"))' results.json

# Get summary of dead code findings
jq '[.debt_items[] | select(.debt_type == "DeadCode")] | length' results.json
```

**Note**: Dead code detection results are integrated into the standard debt item format for consistent analysis alongside other technical debt types.

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

```yaml
# .github/workflows/dead-code.yml
- name: Check for dead code
  run: |
    debtmap analyze --format=json > analysis.json
    # Count dead code items
    DEAD_CODE_COUNT=$(jq '[.debt_items[] | select(.debt_type == "DeadCode")] | length' analysis.json)
    if [ $DEAD_CODE_COUNT -gt 0 ]; then
      echo "Dead code detected: $DEAD_CODE_COUNT items"
      jq '.debt_items[] | select(.debt_type == "DeadCode") | "\(.file):\(.line) - \(.function_name)"' analysis.json
      exit 1
    fi
```

## Rust-Specific Dead Code Detection

For Rust codebases, debtmap provides enhanced detection with visibility-aware analysis:

### Visibility-Based Detection

Dead code detection (`src/priority/scoring/classification.rs:515-543`) respects Rust's visibility system:

- **`pub` functions** - Analyzed with external API heuristics; may be used by external crates
- **`pub(crate)` functions** - Internal API; checked for callers within the crate
- **Private functions** - Must have internal callers to be considered live

### Trait Implementation Protection

Trait methods are automatically excluded from dead code detection (`src/priority/scoring/classification.rs:545-593`):

```rust
// These are NEVER flagged as dead code (from src/debt/public_api_detector.rs:619-650):
impl Clone for MyType { fn clone(&self) -> Self { ... } }  // Clone trait
impl Default for MyType { fn default() -> Self { ... } }    // Default trait
impl From<T> for MyType { fn from(t: T) -> Self { ... } }   // From trait

// Common trait methods automatically recognized:
// fmt, clone, default, from, into, try_from, try_into, as_ref, as_mut,
// drop, deref, index, add, sub, mul, div, eq, ne, cmp, hash,
// serialize, deserialize, next, size_hint
```

### Framework Callback Patterns

The analyzer recognizes common framework patterns (`src/priority/scoring/classification.rs:596-612`):

```rust
// Functions with these name patterns are protected from dead code detection:
fn handle_event() { }      // Contains "handler"
fn on_click() { }          // Contains "on_"
fn route_request() { }     // Contains "route"
fn middleware_auth() { }   // Contains "middleware"
fn spawn_worker() { }      // Contains "spawn"
fn poll_status() { }       // Contains "poll"
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
