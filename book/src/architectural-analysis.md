# Architectural Analysis

Debtmap provides comprehensive architectural analysis including circular dependency detection, coupling metrics, stability analysis, and code duplication detection. This helps identify structural issues that impact maintainability and testability.

## Overview

Architectural analysis identifies:
- Circular dependencies between modules
- Coupling issues (afferent and efferent coupling)
- Bidirectional dependencies
- Violations of stable dependencies principle
- Zone of pain and zone of uselessness
- Code duplication

## Circular Dependency Detection

Circular dependencies create tight coupling and make code harder to test and maintain.

### Detection Method

Uses depth-first search (DFS) cycle detection in the module dependency graph.

```rust
// Example circular dependency
// file_a.rs
use crate::file_b::FunctionB;

pub fn function_a() {
    function_b();
}

// file_b.rs
use crate::file_a::FunctionA;  // Circular!

pub fn function_b() {
    function_a();
}
```

### Severity

**High** - Circular dependencies indicate architectural problems

### Recommendations

1. **Introduce mediator module**: Extract shared logic to new module
2. **Use dependency injection**: Pass dependencies as parameters
3. **Refactor boundaries**: Separate concerns into distinct layers
4. **Break cycles**: Move one module's dependency to interface

## Coupling Metrics

### Afferent Coupling (Ca)

**Definition:** Number of modules that depend on this module (incoming dependencies)

**High afferent coupling** (>5 dependent modules):
- **Severity:** Medium
- **Implication:** Module is widely used, changes have broad impact
- **Recommendation:** Simplify public API, ensure stability

### Efferent Coupling (Ce)

**Definition:** Number of modules this module depends on (outgoing dependencies)

**High efferent coupling** (>5 dependencies):
- **Severity:** Medium
- **Implication:** Module has many external dependencies
- **Recommendation:** Reduce external dependencies, split responsibilities

### Coupling Formulas

```
Afferent Coupling (Ca) = Number of modules depending on this module
Efferent Coupling (Ce) = Number of modules this module depends on
Instability (I) = Ce / (Ca + Ce)
```

**Instability ranges:**
- **I = 0**: Maximally stable (only depended upon)
- **I = 1**: Maximally unstable (only depends on others)
- **I = 0.5**: Balanced

## Bidirectional Dependencies

### Detection

A depends on B **AND** B depends on A

```rust
// Module A
use crate::b::TypeB;

pub struct TypeA {
    b: TypeB,  // A depends on B
}

// Module B
use crate::a::TypeA;

pub struct TypeB {
    a: TypeA,  // B depends on A - Bidirectional!
}
```

### Severity

**High** - Creates tight coupling and circular logic

### Recommendations

1. **Create mediator module**: Extract shared logic to separate module both can depend on
2. **Use events/callbacks**: Break direct coupling with event system
3. **Dependency inversion**: Introduce trait/interface both implement

## Stable Dependencies Principle

### Principle

Modules should depend in the direction of stability. Unstable modules should not depend on stable modules.

### Violation Detection

**Formula:** Instability = Efferent / (Afferent + Efferent)

**Violation:** Instability >0.8 AND >2 dependents

### Severity

**Medium** - Indicates architectural instability

### Recommendations

1. **Increase abstractness**: Introduce traits/interfaces
2. **Reduce dependencies**: Remove unnecessary external dependencies
3. **Split module**: Separate stable core from unstable features

## Zone of Pain

### Detection Criteria

- Low abstractness (<0.2)
- Low instability (<0.2)
- More than 3 dependents

### Implications

**Highly concrete and stable code with many dependents = painful to change**

### Severity

**Medium** - Changes require extensive testing and coordination

### Recommendations

1. **Introduce abstractions**: Create interfaces/traits
2. **Separate concerns**: Extract configuration from logic
3. **Create facades**: Simplify complex stable interfaces

### Example

```rust
// God object in zone of pain
pub struct ConfigManager {
    // Concrete implementation with many fields
    database_config: DbConfig,
    api_config: ApiConfig,
    cache_config: CacheConfig,
    // ... many more

    // Many concrete methods
    pub fn load_database_config() { }
    pub fn save_database_config() { }
    // ... many more
}

// Better: Extract interfaces
pub trait ConfigProvider {
    fn get(&self, key: &str) -> Option<Value>;
}

pub struct FileConfigProvider { }
pub struct EnvConfigProvider { }
```

## Zone of Uselessness

### Detection Criteria

- High abstractness (>0.8)
- High instability (>0.8)

### Implications

**Highly abstract and unstable = overly complex abstractions with no dependents**

### Severity

**Low** - Unnecessary complexity

### Recommendations

1. **Simplify abstractions**: Remove unused trait methods
2. **Consolidate modules**: Merge related abstractions
3. **Remove if unused**: Delete dead abstraction code

## Code Duplication Detection

### Detection Method

Uses SHA256 hash matching of normalized code blocks:
1. Normalize whitespace and comments
2. Hash code blocks (5-10 lines minimum)
3. Find matching hashes

### Configuration

```toml
[duplication]
minimum_chunk_size = 5  # Minimum lines to consider
ignore_comments = true
ignore_whitespace = true
```

### Severity

**Medium** - Duplicated code increases maintenance burden

### Recommendations

1. **Extract to shared function**: Create reusable function
2. **Introduce abstraction**: Use traits for similar patterns
3. **Use macros**: For Rust-specific repetition
4. **Template/generic functions**: Parameterize duplicated logic

### Example

```rust
// Detected duplication
fn process_user_data(data: &str) -> Result<User> {
    let trimmed = data.trim();
    let parsed = serde_json::from_str(trimmed)?;
    validate_user(&parsed)?;
    Ok(parsed)
}

fn process_product_data(data: &str) -> Result<Product> {
    let trimmed = data.trim();
    let parsed = serde_json::from_str(trimmed)?;
    validate_product(&parsed)?;
    Ok(parsed)
}

// Better: Extract common logic
fn parse_and_validate<T: DeserializeOwned + Validate>(
    data: &str
) -> Result<T> {
    let trimmed = data.trim();
    let parsed = serde_json::from_str(trimmed)?;
    parsed.validate()?;
    Ok(parsed)
}
```

## Configuration

### Enable Architectural Analysis

```bash
debtmap analyze . --filter-categories Architecture
```

### Adjust Coupling Thresholds

```toml
[architectural_analysis]
max_afferent_coupling = 5
max_efferent_coupling = 5
instability_threshold = 0.8
min_dependents_for_pain = 3
abstractness_threshold = 0.2
```

### Duplication Settings

```toml
[duplication]
enabled = true
minimum_chunk_size = 7
minimum_occurrences = 2
ignore_test_code = true
```

## Best Practices

**Module design:**
- Keep modules focused and cohesive
- Minimize inter-module dependencies
- Follow dependency inversion principle
- Use interfaces at module boundaries

**Coupling management:**
- Aim for high cohesion, low coupling
- Depend on abstractions, not concretions
- Monitor instability metrics
- Refactor bidirectional dependencies immediately

**Duplication:**
- Extract common patterns early
- Use DRY principle judiciously (avoid premature abstraction)
- Prefer explicit duplication over wrong abstraction
- Validate that "duplication" is truly duplicated logic

**Stability:**
- Stable modules should be abstract
- Unstable modules can be concrete
- Dependencies should flow toward stability

## Use Cases

### Architecture Review

```bash
# Full architectural analysis
debtmap analyze . --filter-categories Architecture --format markdown
```

### Dependency Visualization

```bash
# Generate call graph with architectural metrics
debtmap analyze . --show-dependencies --format json > architecture.json
```

### Refactoring Planning

```bash
# Find zone of pain modules for refactoring
debtmap analyze . --filter-categories Architecture | grep "Zone of Pain"
```

### Coupling Audit

```bash
# Identify high coupling
debtmap analyze . --filter-categories Architecture --min-priority high
```

## Troubleshooting

### False Positive Circular Dependencies

**Issue:** Test helpers create apparent cycles

**Solution:**
- Exclude test directories from analysis
- Use separate module for test utilities
- Restructure test dependencies

### Duplication in Generated Code

**Issue:** Code generation creates duplication

**Solution:**
- Exclude generated files via config
- Adjust minimum chunk size
- Use suppression for unavoidable duplication

### Zone of Pain on Necessary Code

**Issue:** Core stable code flagged

**Solution:**
- Expected for framework/library core
- Focus on extracting interfaces
- Document architectural decisions
- Use suppression if intentional

## See Also

- [God Object Detection](god-object-detection.md) - Related organizational analysis
- [Configuration](configuration.md) - Configure architectural thresholds
- [Dependency Analysis](#) - Understanding module dependencies
