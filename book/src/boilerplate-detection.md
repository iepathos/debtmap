# Boilerplate Detection

Debtmap identifies repetitive code patterns that could benefit from macro-ification or other abstraction techniques. This helps reduce maintenance burden and improve code consistency.

## Overview

Boilerplate detection analyzes low-complexity repetitive code to identify opportunities for:

- **Macro-ification** - Convert repetitive patterns to declarative or procedural macros
- **Code generation** - Use build scripts to generate repetitive implementations
- **Generic abstractions** - Replace duplicate implementations with generic code
- **Trait derivation** - Use derive macros instead of manual implementations

## Detection Criteria

Debtmap identifies boilerplate using several heuristics:

- **Multiple similar trait implementations** - Same trait implemented repeatedly with similar patterns
- **Low complexity repetitive code** - Simple, repeated code with minimal variation
- **High method uniformity** - Methods with similar signatures and structure
- **Trait pattern analysis** - Common trait implementation patterns

## Pattern Types

### Trait Implementation Boilerplate

```rust
// Repetitive From implementations detected as boilerplate
impl From<ErrorA> for AppError {
    fn from(e: ErrorA) -> Self {
        AppError::A(e)
    }
}

impl From<ErrorB> for AppError {
    fn from(e: ErrorB) -> Self {
        AppError::B(e)
    }
}

// Recommendation: Use a macro or thiserror crate
```

### Builder Pattern Candidates

```rust
// Repetitive builder methods
impl ConfigBuilder {
    pub fn host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

// Recommendation: Use derive_builder or similar
```

### State Machine Patterns

Repetitive state transition implementations may benefit from macro-based state machine generation.

## Configuration

```toml
[boilerplate_detection]
# Enable boilerplate detection
enabled = true

# Minimum similar implementations to flag
min_similar_implementations = 3

# Maximum complexity for boilerplate (low complexity expected)
max_complexity = 5

# Minimum method uniformity percentage
min_uniformity = 0.7
```

## Usage

```bash
# Analyze with boilerplate detection
debtmap analyze .

# Focus on boilerplate patterns
debtmap analyze . --detect-boilerplate

# Show macro recommendations
debtmap analyze . --show-macro-suggestions
```

## Recommendations

Debtmap provides specific recommendations:

- **Derive macro candidates** - Patterns suitable for procedural macros
- **Declarative macro patterns** - Simple repetition suitable for macro_rules!
- **Code generation** - Build-time generation for complex patterns
- **Generic abstractions** - Type-parameterized solutions

## See Also

- [Design Pattern Detection](design-patterns.md) - Higher-level pattern recognition
- [God Object Detection](god-object-detection.md) - Complexity-based refactoring
