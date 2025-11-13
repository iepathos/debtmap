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

### CLI Options

Enable or configure pattern detection using command-line flags:

```bash
# Disable all pattern detection
debtmap analyze --no-pattern-detection

# Enable only specific patterns (all 7 available patterns shown)
debtmap analyze --patterns observer,singleton,factory,strategy,callback,template_method,dependency_injection

# Enable a subset of patterns
debtmap analyze --patterns observer,singleton,factory

# Set confidence threshold (0.0-1.0)
debtmap analyze --pattern-threshold 0.8

# Show warnings for uncertain pattern detections
debtmap analyze --show-pattern-warnings
```

**Available Patterns for `--patterns` Flag**:
- `observer` - Observer pattern detection
- `singleton` - Singleton pattern detection
- `factory` - Factory pattern detection
- `strategy` - Strategy pattern detection
- `callback` - Callback pattern detection
- `template_method` - Template method pattern detection
- `dependency_injection` - Dependency injection detection

**Note**: Builder and Visitor patterns are detected internally but are **not available** via the `--patterns` flag. See [Internal Pattern Detection](#internal-pattern-detection) for details.

### Pattern Detection Output

Pattern detection results are integrated into debtmap's output in different formats:

**Terminal Format**: Detected patterns are shown in a dedicated section of the analysis output:
```
Design Patterns Detected:
  Observer Pattern (confidence: 0.88)
    Interface: EventListener (event_system.py:4)
    Implementations: AuditLogger, SessionManager
```

**JSON Format**: Pattern results are included in the `pattern_instances` field:
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

**Markdown Format**: Patterns are documented in a dedicated section with cross-references to source files.

**Using `--show-pattern-warnings`**: This flag reveals low-confidence detections (below the threshold) that might indicate:
- Incomplete pattern implementations
- Patterns in development
- False positives to review

Use this flag during initial analysis to understand what patterns debtmap sees:
```bash
debtmap analyze --show-pattern-warnings --pattern-threshold 0.7
```

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
- [Boilerplate vs Complexity](./boilerplate-vs-complexity.md) - How complexity is calculated
- [Scoring Strategies](./scoring-strategies.md) - Complexity adjustments and multipliers

## Practical Examples

### Example 1: Analyzing a Web Framework

Analyzing a Flask application with callback patterns:

```bash
debtmap analyze --patterns callback --show-pattern-warnings myapp/
```

Output excerpt:
```
Design Patterns Detected:
  Callback Pattern (15 instances, confidence: 0.85-0.92)
    - @app.route decorators: 12
    - @app.before_request decorators: 2
    - @app.errorhandler decorators: 1

Complexity Adjustments:
  - Route handlers: -40% complexity (pattern boilerplate)
  - Error handlers: -50% complexity (expected pattern)
```

### Example 2: Detecting Observer Pattern

Analyzing a codebase with event-driven architecture:

```bash
debtmap analyze --patterns observer --pattern-threshold 0.75
```

Code:
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

Output:
```
Design Patterns:
  Observer Pattern (confidence: 0.88)
    Interface: EventListener (event_system.py:4)
    Implementations:
      - AuditLogger (event_system.py:9)
      - SessionManager (event_system.py:13)
    Registration: add_listener (event_system.py:21)
    Notification: notify_login (event_system.py:24)
```

## Use Cases

### 1. False Positive Reduction

**Problem**: Complex factory functions flagged as too complex
**Solution**: Enable factory pattern detection to apply appropriate complexity adjustments

```bash
debtmap analyze --patterns factory --pattern-threshold 0.7
```

### 2. Architecture Documentation

**Problem**: Undocumented design patterns in legacy codebase
**Solution**: Run pattern detection to automatically identify architectural patterns

```bash
debtmap analyze --show-pattern-warnings > architecture-report.txt
```

### 3. Pattern Consistency Validation

**Problem**: Inconsistent Observer implementations across the codebase
**Solution**: Use pattern detection to identify all Observer instances and compare their structure

```bash
debtmap analyze --patterns observer --output-format json > observers.json
```

### 4. Refactoring Guidance

**Problem**: Code smells that might be incomplete pattern implementations
**Solution**: Detect partial patterns with lower confidence thresholds

```bash
debtmap analyze --pattern-threshold 0.5 --show-pattern-warnings
```

## Troubleshooting

### Pattern Not Detected

**Symptoms**: Expected pattern not appearing in output

**Possible Causes**:
1. Confidence below threshold
   - Solution: Lower `--pattern-threshold` or use `--show-pattern-warnings`
2. Pattern disabled
   - Solution: Check `--patterns` flag (only enabled patterns will be detected)
3. Implementation doesn't match detection criteria
   - Solution: Review pattern-specific criteria above

### Builder or Visitor Pattern Not Available via CLI

**Symptoms**: Using `--patterns builder` or `--patterns visitor` has no effect

**Explanation**: Builder and Visitor patterns are detected **internally only** and are not available as user-facing pattern detection features:
- **Builder**: Used internally during god object detection to adjust scores for builder classes
- **Visitor**: Used internally for complexity analysis to apply logarithmic scaling to exhaustive match expressions

**Solution**: These patterns are detected automatically when needed for internal analyses. They don't require manual enablement and won't appear in pattern detection output. See [Internal Pattern Detection](#internal-pattern-detection) for details.

**Available user-facing patterns**: `observer`, `singleton`, `factory`, `strategy`, `callback`, `template_method`, `dependency_injection`

### False Positive Detection

**Symptoms**: Pattern detected incorrectly

**Possible Causes**:
1. Naming collision (e.g., `create_` function that isn't a factory)
   - Solution: Increase `--pattern-threshold` to require stronger evidence
2. Coincidental structural match
   - Solution: Increase `--pattern-threshold` to reduce false positives

### Incomplete Cross-File Detection

**Symptoms**: Pattern implementations in other files not linked to interface

**Possible Causes**:
1. Dynamic imports not tracked
   - Solution: Use static imports where possible
2. Interface not explicitly imported
   - Solution: Add explicit import even if not type-checking

## Best Practices

1. **Start with defaults**: The default 0.7 threshold works well for most projects
2. **Use `--show-pattern-warnings`** during initial analysis to see borderline detections
3. **Enable relevant patterns**: Use `--patterns` to focus on patterns most relevant to your codebase
4. **Adjust thresholds**: Increase `--pattern-threshold` to reduce false positives, or lower it to detect incomplete implementations
5. **Combine with complexity analysis**: Use pattern detection to understand complexity adjustments
6. **Review low-confidence detections**: They may indicate incomplete implementations worth refactoring

## Summary

Debtmap's design pattern detection provides:
- **7 user-facing patterns** covering common OOP and functional patterns (Observer, Singleton, Factory, Strategy, Callback, Template Method, Dependency Injection)
- **2 internal patterns** (Builder, Visitor) used for god object detection and complexity normalization
- **Configurable confidence thresholds** for precision vs. recall tradeoff
- **Cross-file detection** for patterns spanning multiple modules
- **Rust trait support** for idiomatic Rust pattern detection
- **Separate from complexity adjustments** - patterns are detected for documentation, not scoring

Pattern detection improves the accuracy of technical debt analysis by recognizing idiomatic code patterns and applying appropriate complexity adjustments. Internal pattern detection helps prevent false positives in god object and complexity analyses without exposing implementation details to users.
