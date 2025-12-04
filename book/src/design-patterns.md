# Design Pattern Detection

Debtmap automatically detects common design patterns in your codebase to provide better architectural insights and reduce false positives in complexity analysis. When recognized design patterns are detected, Debtmap applies appropriate complexity adjustments to avoid penalizing idiomatic code.

## Overview

Debtmap detects 7 design patterns across Python, JavaScript, TypeScript, and Rust:

| Pattern | Primary Language | Detection Confidence |
|---------|-----------------|---------------------|
| Observer | Python, Rust | High (0.8-0.9) |
| Singleton | Python | High (0.85-0.95) |
| Factory | Python | Medium-High (0.7-0.85) |
| Strategy | Python | Medium (0.7-0.8) |
| Callback | Python, JavaScript | High (0.8-0.9) |
| Template Method | Python | Medium (0.7-0.8) |
| Dependency Injection | Python | Medium (0.65-0.75) |

Pattern detection serves multiple purposes:
- **Reduces false positives**: Avoids flagging idiomatic pattern implementations as overly complex
- **Documents architecture**: Automatically identifies architectural patterns in your codebase
- **Validates consistency**: Helps ensure patterns are used correctly and completely
- **Guides refactoring**: Identifies incomplete pattern implementations

## Pattern Detection Details

### Observer Pattern

The Observer pattern is detected in Python and Rust by identifying abstract base classes with concrete implementations.

**Detection Criteria (Python)**:
- Abstract base class with `ABC`, `Protocol`, or `Interface` markers
- Abstract methods decorated with `@abstractmethod`
- Concrete implementations inheriting from the interface
- Methods prefixed with `on_`, `handle_`, or `notify_`
- Registration methods like `add_observer`, `register`, or `subscribe`
- Notification methods like `notify`, `notify_all`, `trigger`, `emit`

**Detection Criteria (Rust)**:
- Trait definitions with callback-style methods
- Multiple implementations of the same trait
- Trait registry tracking for cross-module detection

**Example (Python)**:
```python
from abc import ABC, abstractmethod

class EventObserver(ABC):
    @abstractmethod
    def on_event(self, data):
        """Handle event notification"""
        pass

class LoggingObserver(EventObserver):
    def on_event(self, data):
        print(f"Event occurred: {data}")

class EmailObserver(EventObserver):
    def on_event(self, data):
        send_email(f"Alert: {data}")

class EventManager:
    def __init__(self):
        self.observers = []

    def add_observer(self, observer: EventObserver):
        self.observers.append(observer)

    def notify_all(self, data):
        for observer in self.observers:
            observer.on_event(data)
```

**Confidence**: High (0.8-0.9) when abstract base class, implementations, and registration/notification methods are present. Lower confidence (0.5-0.7) for partial implementations.

### Singleton Pattern

Singleton pattern detection identifies three common Python implementations: module-level singletons, `__new__` override, and decorator-based patterns.

**Detection Criteria**:
- Module-level variable assignments (e.g., `instance = MyClass()`)
- Classes overriding `__new__` to enforce single instance
- Classes decorated with `@singleton` or similar decorators
- Presence of instance caching logic

**Example (Module-level)**:
```python
# config.py
class Config:
    def __init__(self):
        self.settings = {}

    def load(self, path):
        # Load configuration
        pass

# Single instance created at module level
config = Config()
```

**Example (`__new__` override)**:
```python
class DatabaseConnection:
    _instance = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __init__(self):
        if not hasattr(self, 'initialized'):
            self.initialized = True
            self.connect()
```

**Example (Decorator-based)**:
```python
def singleton(cls):
    instances = {}
    def get_instance(*args, **kwargs):
        if cls not in instances:
            instances[cls] = cls(*args, **kwargs)
        return instances[cls]
    return get_instance

@singleton
class Logger:
    def __init__(self):
        self.log_file = open('app.log', 'a')
```

**Confidence**: Very High (0.9-0.95) for `__new__` override and decorator patterns. High (0.85) for module-level singletons with clear naming.

### Factory Pattern

Factory pattern detection identifies factory functions, factory classes, and factory registries based on naming conventions and structural patterns.

**Detection Criteria**:
- Functions with names containing `create_`, `make_`, `build_`, or `_factory`
- Factory registry patterns (dictionaries mapping types to constructors)
- Functions that return instances of different types based on parameters
- Classes with factory methods

**Example (Factory Function)**:
```python
def create_logger(log_type: str):
    if log_type == "file":
        return FileLogger()
    elif log_type == "console":
        return ConsoleLogger()
    elif log_type == "network":
        return NetworkLogger()
    else:
        raise ValueError(f"Unknown logger type: {log_type}")
```

**Example (Registry-based Factory)**:
```python
# Parser registry
PARSERS = {
    'json': JSONParser,
    'xml': XMLParser,
    'yaml': YAMLParser,
}

def create_parser(format: str):
    parser_class = PARSERS.get(format)
    if parser_class is None:
        raise ValueError(f"No parser for format: {format}")
    return parser_class()
```

**Example (Factory Method)**:
```python
class DocumentFactory:
    @staticmethod
    def create_document(doc_type: str):
        if doc_type == "pdf":
            return PDFDocument()
        elif doc_type == "word":
            return WordDocument()
        else:
            return PlainTextDocument()
```

**Confidence**: Medium-High (0.75-0.85) for functions with factory naming patterns. Lower confidence (0.6-0.7) for registry patterns without factory names.

### Strategy Pattern

Strategy pattern detection identifies interfaces with multiple implementations representing interchangeable algorithms.

**Detection Criteria**:
- Abstract base class or Protocol defining strategy interface
- Multiple concrete implementations
- Strategy interface typically has 1-2 core methods
- Used via composition (strategy object passed to context)

**Example**:
```python
from abc import ABC, abstractmethod

class CompressionStrategy(ABC):
    @abstractmethod
    def compress(self, data: bytes) -> bytes:
        pass

class ZipCompression(CompressionStrategy):
    def compress(self, data: bytes) -> bytes:
        return zlib.compress(data)

class GzipCompression(CompressionStrategy):
    def compress(self, data: bytes) -> bytes:
        return gzip.compress(data)

class LzmaCompression(CompressionStrategy):
    def compress(self, data: bytes) -> bytes:
        return lzma.compress(data)

class FileCompressor:
    def __init__(self, strategy: CompressionStrategy):
        self.strategy = strategy

    def compress_file(self, path):
        data = read_file(path)
        return self.strategy.compress(data)
```

**Confidence**: Medium (0.7-0.8) based on interface structure and implementation count.

### Callback Pattern

Callback pattern detection identifies decorator-based callbacks commonly used in web frameworks and event handlers.

**Detection Criteria**:
- Decorators with patterns like `@route`, `@handler`, `@app.`, `@on`, `@callback`
- Framework-specific decorators (Flask routes, FastAPI endpoints, event handlers)
- Functions registered as callbacks for events or hooks

**Example (Flask Routes)**:
```python
from flask import Flask

app = Flask(__name__)

@app.route('/api/users')
def get_users():
    return {"users": []}

@app.route('/api/users/<id>')
def get_user(id):
    return {"user": find_user(id)}
```

**Example (Event Handler)**:
```python
class EventBus:
    def __init__(self):
        self.handlers = {}

    def on(self, event_name):
        def decorator(func):
            self.handlers[event_name] = func
            return func
        return decorator

bus = EventBus()

@bus.on('user.created')
def handle_user_created(user):
    send_welcome_email(user)

@bus.on('order.placed')
def handle_order_placed(order):
    process_payment(order)
```

**Confidence**: High (0.8-0.9) for framework decorator patterns. Medium (0.6-0.7) for custom callback implementations.

### Template Method Pattern

Template method pattern detection identifies base classes with template methods that call abstract hook methods.

**Detection Criteria**:
- Base class with concrete methods (template methods)
- Abstract methods intended to be overridden (hook methods)
- Template method calls hook methods in a defined sequence
- Subclasses override hook methods but not template method

**Example**:
```python
from abc import ABC, abstractmethod

class DataProcessor(ABC):
    def process(self, data):
        """Template method defining the algorithm skeleton"""
        raw = self.load_data(data)
        validated = self.validate(raw)
        transformed = self.transform(validated)
        self.save(transformed)

    @abstractmethod
    def load_data(self, source):
        """Hook: Load data from source"""
        pass

    @abstractmethod
    def validate(self, data):
        """Hook: Validate data"""
        pass

    def transform(self, data):
        """Hook: Transform data (optional override)"""
        return data

    @abstractmethod
    def save(self, data):
        """Hook: Save processed data"""
        pass

class CSVProcessor(DataProcessor):
    def load_data(self, source):
        return read_csv(source)

    def validate(self, data):
        return [row for row in data if row]

    def save(self, data):
        write_csv('output.csv', data)
```

**Confidence**: Medium (0.7-0.8) based on combination of abstract and concrete methods in base class.

### Dependency Injection Pattern

Dependency injection pattern detection identifies classes that receive dependencies through constructors or setters rather than creating them internally.

**Detection Criteria**:
- Constructor parameters accepting interface/protocol types
- Setter methods for injecting dependencies
- Optional dependencies with default values
- Absence of hard-coded object instantiation inside the class

**Example (Constructor Injection)**:
```python
class UserService:
    def __init__(self,
                 user_repository: UserRepository,
                 email_service: EmailService,
                 logger: Logger):
        self.user_repo = user_repository
        self.email_service = email_service
        self.logger = logger

    def create_user(self, username, email):
        user = self.user_repo.create(username, email)
        self.email_service.send_welcome(email)
        self.logger.info(f"Created user: {username}")
        return user
```

**Example (Setter Injection)**:
```python
class ReportGenerator:
    def __init__(self):
        self.data_source = None
        self.formatter = None

    def set_data_source(self, source):
        self.data_source = source

    def set_formatter(self, formatter):
        self.formatter = formatter

    def generate(self):
        data = self.data_source.fetch()
        return self.formatter.format(data)
```

**Confidence**: Medium (0.65-0.75) based on constructor signatures and absence of direct instantiation.

## Internal Pattern Detection

Debtmap also detects certain patterns internally for analysis purposes, but these are not exposed as user-facing design pattern detection features. These internal patterns help improve the accuracy of other analyses like god object detection and complexity calculations.

### Builder Pattern (Internal Use Only)

The Builder pattern is detected internally during **god object detection** to avoid false positives. Classes that follow the builder pattern are given adjusted scores in god object analysis since builder classes naturally have many methods and fields.

**Note**: Builder pattern detection is **not available** via the `--patterns` CLI flag. It's used only internally for scoring adjustments.

**Internal Detection Criteria**:
- Struct with builder suffix or builder-related naming
- Methods returning `Self` for chaining
- Final `build()` method returning the constructed type
- Type-state pattern usage (optional)

**Example** (Internal Detection):
```rust
pub struct HttpClientBuilder {
    base_url: Option<String>,
    timeout: Duration,
    headers: HashMap<String, String>,
}

impl HttpClientBuilder {
    pub fn new() -> Self { /* ... */ }

    // Chaining methods detected internally
    pub fn base_url(mut self, url: impl Into<String>) -> Self { /* ... */ }
    pub fn timeout(mut self, timeout: Duration) -> Self { /* ... */ }
    pub fn header(mut self, key: String, value: String) -> Self { /* ... */ }

    pub fn build(self) -> Result<HttpClient> { /* ... */ }
}
```

**Why Internal Only**: Builder patterns are a legitimate design choice for complex object construction. Debtmap detects them to prevent flagging builder classes as god objects, but doesn't report them as design patterns since they don't require complexity adjustments like other patterns.

**Source**: `src/organization/builder_pattern.rs` - Used for god object detection score adjustment

### Visitor Pattern (Internal Use Only)

The Visitor pattern is detected internally for **complexity analysis normalization**. When exhaustive pattern matching is detected (typical of visitor patterns), Debtmap applies logarithmic complexity scaling instead of linear scaling to avoid penalizing idiomatic exhaustive match expressions.

**Note**: Visitor pattern detection is **not available** via the `--patterns` CLI flag. It's used only internally for complexity scaling adjustments.

**Internal Detection Criteria**:
- Trait with visit methods for different types
- Implementations providing behavior for each visited type
- Exhaustive pattern matching across enum variants
- Used primarily for AST traversal or data structure processing

**Example** (Internal Detection):
```rust
trait Visitor {
    fn visit_function(&mut self, func: &Function);
    fn visit_class(&mut self, class: &Class);
    fn visit_module(&mut self, module: &Module);
}

impl Visitor for ComplexityVisitor {
    fn visit_function(&mut self, func: &Function) {
        // Exhaustive matching detected for complexity scaling
        match &func.body {
            FunctionBody::Simple => { /* ... */ }
            FunctionBody::Complex(statements) => { /* ... */ }
        }
    }
}
```

**Why Internal Only**: Visitor patterns often involve exhaustive pattern matching which can appear complex by traditional metrics. Debtmap detects these patterns to apply logarithmic scaling (`log2(match_arms) * avg_complexity`) instead of linear, preventing false positives in complexity analysis. This is a complexity adjustment mechanism, not a user-visible pattern detection feature.

**Source**: `src/complexity/visitor_detector.rs` - Used for complexity analysis, not pattern reporting

## Configuration

### Current Implementation Status

Pattern detection is currently **internal-only** and used for analysis adjustments. The CLI flags for pattern detection exist in the codebase but are not yet fully integrated into the analysis pipeline.

**Status Summary**:
- ✅ Pattern detection logic implemented (7 user-facing patterns)
- ✅ CLI flags defined (`--no-pattern-detection`, `--patterns`, `--pattern-threshold`, `--show-pattern-warnings`)
- ⚠️ CLI flags not yet wired to analysis pipeline
- ⚠️ Pattern detection results not currently exposed in output formats

Pattern detection is primarily used internally for:
- Adjusting complexity scores to avoid false positives
- Informing god object detection (Builder pattern)
- Normalizing exhaustive pattern matching complexity (Visitor pattern)

### CLI Options (Defined but Not Yet Active)

The following CLI flags are defined in the codebase (`src/cli.rs:228-241`) but are not yet fully integrated:

```bash
# Disable all pattern detection (planned)
debtmap analyze --no-pattern-detection

# Enable only specific patterns (planned)
debtmap analyze --patterns observer,singleton,factory,strategy,callback,template_method,dependency_injection

# Set confidence threshold (planned)
debtmap analyze --pattern-threshold 0.8

# Show warnings for uncertain pattern detections (planned)
debtmap analyze --show-pattern-warnings
```

**Planned Patterns for `--patterns` Flag** (when integration is complete):
- `observer` - Observer pattern detection
- `singleton` - Singleton pattern detection
- `factory` - Factory pattern detection
- `strategy` - Strategy pattern detection
- `callback` - Callback pattern detection
- `template_method` - Template method pattern detection
- `dependency_injection` - Dependency injection detection

**Note**: Builder and Visitor patterns are detected internally but will not be available via the `--patterns` flag. See [Internal Pattern Detection](#internal-pattern-detection) for details.

### Roadmap: Pattern Detection Output

When fully integrated, pattern detection results will appear in debtmap's output formats:

**Planned Terminal Format**:
```
Design Patterns Detected:
  Observer Pattern (confidence: 0.88)
    Interface: EventListener (event_system.py:4)
    Implementations: AuditLogger, SessionManager
```

**Planned JSON Format**:
```json
{
  "pattern_instances": [
    {
      "pattern_type": "Observer",
      "confidence": 0.88,
      "location": "event_system.py:4",
      "implementations": ["AuditLogger", "SessionManager"]
    }
  ]
}
```

**Current Workaround**: Pattern detection is used internally during analysis to improve accuracy. To see the effects of pattern detection:
1. Run analysis with and without `--no-pattern-detection` (when implemented)
2. Compare complexity scores and god object detection results
3. Patterns are being detected, but not explicitly reported in output

## Confidence Scoring

Pattern detection uses a confidence scoring system (0.0-1.0) to indicate match quality:

- **0.9-1.0**: Very High - Strong structural match with all key elements present
- **0.8-0.9**: High - Clear pattern with most elements present
- **0.7-0.8**: Medium-High - Pattern present with some uncertainty
- **0.6-0.7**: Medium - Possible pattern with limited evidence
- **0.5-0.6**: Low - Weak match, may be false positive

**Default Threshold**: 0.7 - Only patterns with 70% or higher confidence are reported by default.

**Adjusting Thresholds**:
```bash
# More strict (fewer patterns, higher confidence)
debtmap analyze --pattern-threshold 0.85

# More lenient (more patterns, lower confidence)
debtmap analyze --pattern-threshold 0.6 --show-pattern-warnings
```

**How Confidence is Calculated**:

Each pattern detector calculates confidence holistically based on multiple factors:

1. **Structural completeness**: Are all expected elements present?
2. **Naming conventions**: Do names match expected patterns?
3. **Implementation count**: Are there enough implementations to confirm the pattern?
4. **Cross-validation**: Do different detection heuristics agree?

For example, Observer pattern confidence is calculated holistically based on:
- Presence of abstract base class with appropriate markers (`ABC`, `Protocol`, etc.)
- Number of concrete implementations found
- Detection of registration methods (`add_observer`, `register`, `subscribe`)
- Detection of notification methods (`notify`, `notify_all`, `trigger`, `emit`)
- Naming conventions matching observer patterns

Higher confidence requires more structural elements to be present. The calculation is not a simple sum of individual weights but rather a holistic assessment of pattern completeness.

## Cross-File Pattern Detection

Debtmap can detect patterns that span multiple files, particularly for the Observer pattern where interfaces and implementations may be in separate modules.

**How Cross-File Detection Works**:

1. **Import Tracking**: Debtmap tracks imports to understand module dependencies
2. **Interface Registry**: Abstract base classes are registered globally
3. **Implementation Matching**: Implementations in other files are matched to registered interfaces
4. **Cross-Module Context**: A shared context links related files

**Example**:

```python
# interfaces/observer.py
from abc import ABC, abstractmethod

class EventObserver(ABC):
    @abstractmethod
    def on_event(self, data):
        pass

# observers/logging_observer.py
from interfaces.observer import EventObserver

class LoggingObserver(EventObserver):
    def on_event(self, data):
        log(data)

# observers/email_observer.py
from interfaces.observer import EventObserver

class EmailObserver(EventObserver):
    def on_event(self, data):
        send_email(data)
```

Debtmap detects this as a single Observer pattern with cross-file implementations.

**Limitations**:
- Only works for explicitly imported interfaces
- Requires static import analysis (dynamic imports may not be tracked)
- Most effective within a single project (not across external dependencies)

## Rust-Specific Pattern Detection

### Trait-Based Patterns

Rust pattern detection leverages the trait system for identifying patterns:

**Trait Registry**: Tracks trait definitions and implementations across modules
```rust
// Trait registered for pattern detection
pub trait EventHandler {
    fn handle(&self, event: &Event);
}

// Multiple implementations tracked
impl EventHandler for LogHandler { /* ... */ }
impl EventHandler for MetricsHandler { /* ... */ }
impl EventHandler for AlertHandler { /* ... */ }
```

**Observer Pattern via Traits**:
```rust
pub trait Observable {
    fn subscribe(&mut self, observer: Box<dyn Observer>);
    fn notify(&self, event: &Event);
}

pub trait Observer {
    fn on_event(&self, event: &Event);
}
```

**Differences from Python Detection**:
- Traits are more explicit than Python's ABC
- Type system ensures implementation correctness
- No runtime reflection needed for detection
- Pattern matching exhaustiveness helps identify Visitor pattern

## Integration with Complexity Analysis

Debtmap has two separate but complementary systems for patterns:

### 1. Design Pattern Detection (This Feature)

The 7 user-facing design patterns documented in this chapter (Observer, Singleton, Factory, Strategy, Callback, Template Method, Dependency Injection) are **detected and reported** to users. These patterns appear in the output to document architectural choices but do not directly adjust complexity scores.

**Purpose**: Architectural documentation and pattern identification

**Output**: Pattern instances with confidence scores in terminal, JSON, and markdown formats

### 2. Complexity Pattern Adjustments (Internal System)

Debtmap has a separate internal system in `src/complexity/python_pattern_adjustments.rs` that detects specific complexity patterns and applies multipliers. These are **different patterns** from the user-facing design patterns:

**Internal complexity patterns include**:
- Dictionary Dispatch (0.5x multiplier)
- Strategy Pattern detection via conditionals (0.6x multiplier)
- Comprehension patterns (0.8x multiplier)
- Other Python-specific complexity patterns

**Purpose**: Adjust complexity scores to avoid penalizing idiomatic code

**Output**: Applied automatically during complexity calculation, not reported separately

### Relationship Between the Systems

Currently, these are **independent systems**:
- Design pattern detection focuses on architectural patterns
- Complexity adjustments focus on implementation patterns

The design pattern detection results are primarily for documentation and architectural insights. The complexity scoring uses its own pattern recognition to apply appropriate adjustments.

### Visitor Pattern Special Case

The Visitor pattern (internal-only) is used for complexity analysis. When exhaustive pattern matching is detected, debtmap applies **logarithmic scaling**:

```
visitor_complexity = log2(match_arms) * average_arm_complexity
```

This prevents exhaustive pattern matching from being flagged as overly complex. See [Visitor Pattern (Internal Use Only)](#visitor-pattern-internal-use-only) for more details.

**See Also**:
- [Complexity Analysis](./analysis-guide/complexity-metrics.md) - How complexity is calculated
- [Scoring Strategies](./scoring-strategies.md) - Complexity adjustments and multipliers

## Practical Examples

### Example 1: Observer Pattern Code Structure

Pattern detection identifies Observer implementations even though results are not yet shown in output:

```python
# event_system.py
from abc import ABC, abstractmethod

class EventListener(ABC):
    @abstractmethod
    def on_user_login(self, user):
        pass

class AuditLogger(EventListener):
    def on_user_login(self, user):
        audit_log.write(f"User {user.id} logged in")

class SessionManager(EventListener):
    def on_user_login(self, user):
        create_session(user)

class EventDispatcher:
    def __init__(self):
        self.listeners = []

    def add_listener(self, listener):
        self.listeners.append(listener)

    def notify_login(self, user):
        for listener in self.listeners:
            listener.on_user_login(user)
```

**Current Behavior**: Debtmap internally detects this Observer pattern (confidence ~0.88) and uses it to adjust complexity scoring. The pattern structure is recognized but not reported in output.

**Source**: Pattern detection logic in `src/analysis/patterns/observer.rs`

### Example 2: Factory Pattern Detection Criteria

Debtmap detects factory patterns based on naming and structure:

```python
def create_logger(log_type: str):
    """Factory function - detected by 'create_' prefix"""
    if log_type == "file":
        return FileLogger()
    elif log_type == "console":
        return ConsoleLogger()
    else:
        return NetworkLogger()
```

**Current Behavior**: The factory pattern detector (in `src/analysis/patterns/factory.rs`) identifies this as a Factory pattern with medium-high confidence (~0.75-0.85) based on:
- Function name contains `create_`
- Returns different types based on parameter
- Multiple instantiation paths

This information is used internally to adjust complexity scores for factory functions.

### Example 3: Impact on Complexity Analysis

While pattern detection results aren't directly shown, their effect can be observed:

```bash
# Run standard analysis
debtmap analyze myapp/

# When --no-pattern-detection is fully integrated, compare results
# (currently this flag exists but isn't fully wired)
```

**Expected Differences**:
- Factory functions: Lower complexity scores with pattern detection
- Observer implementations: Adjusted scores for callback registration
- Template methods: Reduced penalty for abstract method patterns
- Builder classes: Not flagged as god objects despite many methods

## Use Cases

### 1. False Positive Reduction (Active)

**Problem**: Complex factory functions flagged as too complex
**Solution**: Pattern detection automatically adjusts complexity scores for recognized factory patterns

**Current Behavior**:
```bash
debtmap analyze myapp/
```

Factory functions are automatically detected and receive adjusted complexity scores. This happens internally without requiring specific flags.

**Source**: `src/analysis/patterns/factory.rs` applies multipliers to factory function complexity

### 2. Builder Pattern God Object Prevention (Active)

**Problem**: Builder classes flagged as god objects due to many chaining methods
**Solution**: Builder pattern detection automatically excludes builder classes from god object analysis

**Current Behavior**:
```rust
// This builder class is automatically recognized
pub struct HttpClientBuilder {
    // Many fields and methods
    pub fn base_url(mut self, url: String) -> Self { /* ... */ }
    pub fn timeout(mut self, duration: Duration) -> Self { /* ... */ }
    pub fn build(self) -> HttpClient { /* ... */ }
}
```

Debtmap detects the builder pattern (chaining methods returning `Self`, final `build()` method) and adjusts scoring accordingly.

**Source**: `src/organization/builder_pattern.rs` for god object detection adjustment

### 3. Future Use Case: Architecture Documentation (Planned)

**Problem**: Undocumented design patterns in legacy codebase
**Solution**: When pattern output is integrated, users will be able to generate architectural pattern reports

**Planned Command**:
```bash
debtmap analyze --show-pattern-warnings --output-format json > architecture-report.json
```

**Current Workaround**: Review complexity adjustments and god object scoring to infer where patterns are detected

### 4. Future Use Case: Pattern Consistency Validation (Planned)

**Problem**: Inconsistent Observer implementations across the codebase
**Solution**: When integrated, users can filter analysis to specific pattern types

**Planned Command**:
```bash
debtmap analyze --patterns observer --output-format json > observers.json
```

**Current Status**: Pattern detection logic exists and works internally, but results aren't yet exposed in output

## Troubleshooting

### Pattern Detection Not Visible in Output

**Symptoms**: Cannot see detected patterns in analysis output

**Explanation**: Pattern detection is currently **internal-only**. Patterns are detected and used to adjust complexity scoring, but results are not exposed in terminal, JSON, or markdown output.

**Current Behavior**:
- Patterns ARE being detected (see `src/analysis/patterns/mod.rs`)
- Detection results affect complexity scores and god object analysis
- Pattern information is not included in output formatting

**Solution**: To benefit from pattern detection:
1. Run standard analysis - patterns are automatically detected
2. Check if complexity scores seem adjusted for factory/callback patterns
3. Verify builder classes aren't flagged as god objects

**Future Integration**: CLI flags and output formatting will be connected when pattern output integration is complete.

### CLI Flags Have No Effect

**Symptoms**: Using `--patterns`, `--pattern-threshold`, or `--show-pattern-warnings` doesn't change results

**Explanation**: These CLI flags are defined in `src/cli.rs:228-241` but are not yet fully wired to the analysis pipeline.

**Current Status**:
- ✅ CLI argument parsing works
- ⚠️ Values not passed to pattern detector
- ⚠️ Pattern detection runs with default settings regardless of flags

**Workaround**: Pattern detection runs automatically with default settings (threshold 0.7, all 7 patterns enabled).

### Builder or Visitor Pattern Not Available via CLI

**Symptoms**: Cannot specify `--patterns builder` or `--patterns visitor`

**Explanation**: Builder and Visitor patterns are **intentionally internal-only** and will not be available as user-facing pattern detection features:
- **Builder**: Used during god object detection to adjust scores for builder classes (see `src/organization/builder_pattern.rs`)
- **Visitor**: Used for complexity analysis to apply logarithmic scaling to exhaustive match expressions (see `src/complexity/visitor_detector.rs`)

**Solution**: These patterns are automatically detected when needed for internal analyses. They won't appear in the `--patterns` flag even when that feature is fully integrated.

**Available user-facing patterns**: `observer`, `singleton`, `factory`, `strategy`, `callback`, `template_method`, `dependency_injection`

### False Positive Complexity Adjustments

**Symptoms**: Function with `create_` prefix gets lower complexity score but isn't actually a factory

**Possible Causes**:
1. Naming collision (e.g., `create_session()` that doesn't create objects)
2. Overly broad pattern matching heuristics

**Current Workaround**: Pattern detection cannot currently be disabled or tuned per-file. The `--no-pattern-detection` flag exists but isn't yet wired to the detector.

**Future Solution**: When CLI integration is complete:
```bash
debtmap analyze --no-pattern-detection  # Disable all pattern detection
debtmap analyze --pattern-threshold 0.9  # Require very high confidence
```

## Best Practices

### Current Recommendations

1. **Trust automatic detection**: Pattern detection runs automatically with sensible defaults (threshold 0.7, all 7 patterns enabled)
2. **Review complexity scores**: Lower-than-expected complexity for factory/callback functions indicates pattern detection is working
3. **Check builder classes**: If builder classes aren't flagged as god objects, builder pattern detection is working correctly
4. **Follow pattern idioms**: Use standard naming conventions (`create_`, `make_`, `@abstractmethod`, etc.) to ensure patterns are recognized
5. **Structure code clearly**: Well-structured patterns (clear base classes, explicit implementations) have higher confidence scores

### When CLI Integration is Complete

Future best practices when CLI flags are fully wired:

1. **Start with defaults**: The default 0.7 threshold will work well for most projects
2. **Use `--show-pattern-warnings`** during initial analysis to see borderline detections
3. **Tune thresholds per-project**: Adjust `--pattern-threshold` based on your codebase's idioms
4. **Disable selectively**: Use `--no-pattern-detection` to compare scores with/without adjustments
5. **Review pattern reports**: Examine detected patterns to understand architectural decisions

## Summary

### Current State

Debtmap's design pattern detection **exists and works internally** with the following characteristics:

**Implemented Features**:
- ✅ **7 user-facing patterns**: Observer, Singleton, Factory, Strategy, Callback, Template Method, Dependency Injection
- ✅ **2 internal patterns**: Builder (for god object detection), Visitor (for complexity normalization)
- ✅ **Pattern detection logic**: Fully implemented in `src/analysis/patterns/`
- ✅ **Confidence scoring**: 0.0-1.0 scale with holistic assessment
- ✅ **Cross-file detection**: Tracks imports and interfaces across modules
- ✅ **Rust trait support**: Leverages trait system for pattern detection
- ✅ **Complexity integration**: Automatically adjusts scores to reduce false positives

**Partially Implemented**:
- ⚠️ **CLI flags**: Defined in `src/cli.rs` but not wired to pattern detector
- ⚠️ **Output formatting**: `PatternInstance` type exists but not exposed in output

**Not Yet Implemented**:
- ❌ **Pattern output in terminal/JSON/markdown**: Detection results not shown to users
- ❌ **User configuration**: Cannot currently control pattern detection via CLI or config file
- ❌ **Pattern-specific reports**: Cannot filter or focus on specific pattern types

### Impact

Pattern detection **significantly improves analysis accuracy** even without visible output:
- **Reduces false positives**: Factory functions, callbacks, and template methods get appropriate complexity scores
- **Prevents god object misclassification**: Builder classes recognized and excluded from god object detection
- **Normalizes exhaustive matching**: Visitor pattern detection applies logarithmic scaling to pattern matching
- **Supports multiple languages**: Works across Python, JavaScript, TypeScript, and Rust

### Future Integration

When CLI and output integration is complete, users will be able to:
- View detected patterns in analysis output
- Control pattern detection via `--patterns`, `--pattern-threshold`, and `--show-pattern-warnings` flags
- Generate architectural documentation from pattern detection results
- Validate pattern consistency across codebases

The foundation is solid - pattern detection works correctly and provides value. The remaining work is connecting the detection logic to user-facing configuration and output.
