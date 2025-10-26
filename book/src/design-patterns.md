# Design Pattern Detection

Debtmap can automatically detect common design patterns in your code, helping you understand architectural decisions and identify opportunities for refactoring. This chapter covers the 8 design patterns that Debtmap recognizes and how to configure pattern detection.

## Overview

Pattern detection helps you:
- Understand architectural patterns in use
- Identify potential god objects that are actually intentional patterns (Builder, Registry)
- Find opportunities to apply design patterns
- Validate that patterns are implemented correctly

## Supported Patterns

### Observer Pattern

The Observer pattern is detected when code exhibits event listener registration and callback patterns.

**Detection Criteria:**
- Event listener registration methods
- Callback function storage
- Notification/dispatch mechanisms

**Example:**
```rust
// Detected as Observer pattern
struct EventManager {
    listeners: Vec<Box<dyn Fn(&Event)>>,
}

impl EventManager {
    fn register_listener(&mut self, listener: Box<dyn Fn(&Event)>) {
        self.listeners.push(listener);
    }

    fn notify(&self, event: &Event) {
        for listener in &self.listeners {
            listener(event);
        }
    }
}
```

### Singleton Pattern

Singleton detection identifies static instance management and global state patterns.

**Detection Criteria:**
- Static instance fields
- `get_instance()` or similar methods
- Private constructors
- Thread-safe initialization patterns

### Factory Pattern

Factory patterns are recognized through object creation methods that abstract instantiation logic.

**Detection Criteria:**
- Methods named `create_*`, `make_*`, `build_*`
- Polymorphic return types
- Construction logic abstraction

### Strategy Pattern

Strategy pattern detection finds algorithm selection via traits/interfaces.

**Detection Criteria:**
- Trait/interface parameters
- Algorithm swapping mechanisms
- Polymorphic behavior selection

### Callback Pattern

Callback patterns are identified through function passing and invocation.

**Detection Criteria:**
- Function pointer parameters
- Closure/lambda arguments
- Deferred execution patterns

### Template Method Pattern

Template method detection identifies abstract methods with concrete implementations following a defined structure.

**Detection Criteria:**
- Abstract methods in base classes
- Step-by-step algorithm structure
- Hook methods for customization

### Dependency Injection

Dependency injection patterns are recognized through constructor injection and service locators.

**Detection Criteria:**
- Constructor dependency parameters
- Service locator usage
- Interface-based dependencies

### Builder Pattern

Builder pattern detection identifies fluent API construction patterns and can reduce god object scores for intentional builders.

**Detection Criteria:**
- Fluent setter methods returning `self`
- `build()` method
- Progressive construction pattern

**Score Adjustment:**
When a struct is detected as a Builder pattern, its god object score is reduced by up to 70% because the high method count is intentional and appropriate.

## Configuration

### Enable Pattern Detection

Pattern detection is enabled by default. To disable:

```bash
debtmap analyze . --no-pattern-detection
```

### Select Specific Patterns

Enable only specific patterns:

```bash
debtmap analyze . --patterns observer,singleton,factory
```

### Confidence Threshold

Adjust the pattern detection confidence threshold (0.0-1.0, default: 0.7):

```bash
debtmap analyze . --pattern-threshold 0.8
```

Higher thresholds reduce false positives but may miss uncertain patterns.

### Show Pattern Warnings

Display warnings for uncertain pattern detections:

```bash
debtmap analyze . --show-pattern-warnings
```

## Best Practices

**Use pattern detection to:**
- Validate architectural consistency
- Identify god object false positives
- Document design decisions
- Guide refactoring efforts

**Adjust thresholds when:**
- You're getting too many false positives (increase threshold)
- You're missing obvious patterns (decrease threshold)
- Different patterns require different confidence levels

**Combine with other analysis:**
- Use with god object detection to identify intentional vs problematic complexity
- Enable with architectural analysis to understand system structure
- Pair with multi-pass analysis for deeper insights

## Troubleshooting

### Pattern Not Detected

**Issue:** Expected pattern not recognized

**Solution:**
- Lower confidence threshold with `--pattern-threshold 0.6`
- Enable warnings with `--show-pattern-warnings`
- Check that pattern implementation matches detection criteria
- Some patterns may be too customized to auto-detect

### False Positives

**Issue:** Non-patterns incorrectly identified

**Solution:**
- Increase confidence threshold with `--pattern-threshold 0.8`
- Disable specific patterns that cause issues
- Review code structure for accidental pattern-like signatures

### Impact on God Object Scores

**Issue:** Builder/Registry patterns still flagged as god objects

**Solution:**
- Verify pattern detection is enabled (default)
- Check pattern confidence threshold
- Review struct implementation against detection criteria
- Manual suppression if pattern is intentional but not auto-detected

## See Also

- [God Object Detection](god-object-detection.md) - Understanding score adjustments
- [Architectural Analysis](architectural-analysis.md) - System-level patterns
- [Configuration](configuration.md) - Pattern detection configuration
