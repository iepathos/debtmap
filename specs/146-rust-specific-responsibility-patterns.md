---
number: 146
title: Rust-Specific Responsibility Patterns
category: optimization
priority: medium
status: draft
dependencies: [141, 142]
created: 2025-10-27
---

# Specification 146: Rust-Specific Responsibility Patterns

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 141 (I/O Detection), Spec 142 (Call Graph)

## Context

Rust has unique language patterns that indicate specific responsibilities, which generic multi-signal analysis may miss:

- **Trait Implementations**: Functions implementing standard traits (Display, From, Into, etc.) have specific responsibilities
- **Error Handling Patterns**: Functions using `?` operator, `Result`, custom error types
- **Async/Concurrency**: `async fn`, `tokio::spawn`, channels, mutexes
- **Memory Management**: `Drop` impls, `Box::leak`, unsafe blocks, raw pointers
- **Builder Patterns**: Chainable methods returning `Self`, `new()` constructors
- **Type Conversions**: `From`/`Into` impls, `as` casts, `try_from`
- **Iterator Implementations**: Custom iterators, `IntoIterator`, adapters
- **Macro Expansion**: Declarative and procedural macro patterns

While Spec 144 (Framework Patterns) handles framework-specific idioms, this specification addresses **language-level Rust patterns** that are universally applicable regardless of framework.

## Objective

Detect Rust-specific language patterns to enhance responsibility classification for Rust code. Recognize trait implementations, async patterns, error handling, and memory management to provide more accurate and Rust-idiomatic classifications.

## Requirements

### Functional Requirements

**Trait Implementation Detection**:
- Detect standard trait implementations (Display, Debug, From, Into, Default, etc.)
- Classify functions by trait purpose (formatting, conversion, construction)
- Track custom trait implementations
- Identify orphan trait implementations (trait + type from different crates)

**Async/Concurrency Pattern Detection**:
- Detect `async fn` and async blocks
- Identify tokio patterns (spawn, channels, select!, join!)
- Detect mutex/rwlock usage patterns
- Track thread spawning and synchronization

**Error Handling Pattern Detection**:
- Identify error propagation with `?` operator
- Detect custom error type definitions
- Track error conversion patterns (`From<E>` for error types)
- Identify panic/unwrap usage (anti-patterns)

**Memory Management Pattern Detection**:
- Detect unsafe blocks and raw pointer usage
- Identify custom `Drop` implementations
- Track reference counting patterns (Rc, Arc)
- Detect memory leaks (`Box::leak`, `mem::forget`)

**Builder Pattern Detection**:
- Identify builder structs (methods returning `Self`)
- Detect constructor patterns (`new`, `with_*`, `from_*`)
- Track builder finalization methods (`build`, `finalize`)

**Type Conversion Detection**:
- Detect `From`/`Into` trait implementations
- Identify `as` cast patterns
- Track `TryFrom`/`TryInto` for fallible conversions
- Detect newtype wrappers

### Non-Functional Requirements

- **Accuracy**: Correctly identify >90% of Rust-specific patterns
- **Performance**: Pattern detection adds <5% overhead
- **Rust Version Support**: Support stable Rust patterns (no nightly-only)
- **Idiomatic**: Classifications align with Rust community terminology

## Acceptance Criteria

- [ ] Standard trait implementations are correctly classified (Display, From, Into, etc.)
- [ ] Async functions and tokio patterns are identified
- [ ] Error handling patterns are detected (`?` operator, custom errors)
- [ ] Unsafe blocks and memory management patterns are flagged
- [ ] Builder patterns are identified (chainable methods)
- [ ] Type conversion implementations are detected
- [ ] Custom Drop implementations are classified as "Resource Cleanup"
- [ ] Iterator implementations are classified as "Iteration Logic"
- [ ] Performance overhead <5% on Rust codebases
- [ ] Test suite includes debtmap's own Rust code examples

## Technical Details

### Implementation Approach

**Phase 1: Trait Implementation Detection**

```rust
use syn::{ImplItem, ImplItemMethod, ItemImpl, Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StandardTrait {
    // Formatting
    Display,
    Debug,

    // Conversions
    From,
    Into,
    TryFrom,
    TryInto,
    AsRef,
    AsMut,

    // Construction
    Default,
    Clone,

    // Resource Management
    Drop,

    // Iteration
    Iterator,
    IntoIterator,

    // Operators
    Add, Sub, Mul, Div,
    Deref, DerefMut,

    // Comparison
    PartialEq, Eq,
    PartialOrd, Ord,

    // Hashing
    Hash,

    // Serialization (common crates)
    Serialize,
    Deserialize,
}

pub struct RustPatternDetector {
    trait_patterns: HashMap<StandardTrait, ResponsibilityCategory>,
}

impl RustPatternDetector {
    pub fn new() -> Self {
        let mut trait_patterns = HashMap::new();

        trait_patterns.insert(StandardTrait::Display, ResponsibilityCategory::Formatting);
        trait_patterns.insert(StandardTrait::Debug, ResponsibilityCategory::Formatting);

        trait_patterns.insert(StandardTrait::From, ResponsibilityCategory::TypeConversion);
        trait_patterns.insert(StandardTrait::Into, ResponsibilityCategory::TypeConversion);
        trait_patterns.insert(StandardTrait::TryFrom, ResponsibilityCategory::TypeConversion);
        trait_patterns.insert(StandardTrait::TryInto, ResponsibilityCategory::TypeConversion);

        trait_patterns.insert(StandardTrait::Default, ResponsibilityCategory::Construction);
        trait_patterns.insert(StandardTrait::Clone, ResponsibilityCategory::Construction);

        trait_patterns.insert(StandardTrait::Drop, ResponsibilityCategory::ResourceCleanup);

        trait_patterns.insert(StandardTrait::Iterator, ResponsibilityCategory::Iteration);
        trait_patterns.insert(StandardTrait::IntoIterator, ResponsibilityCategory::Iteration);

        // Operators are "Computation"
        for op_trait in [StandardTrait::Add, StandardTrait::Sub, StandardTrait::Mul, StandardTrait::Div] {
            trait_patterns.insert(op_trait, ResponsibilityCategory::Computation);
        }

        RustPatternDetector { trait_patterns }
    }

    pub fn detect_trait_impl(&self, impl_block: &ItemImpl) -> Option<TraitImplClassification> {
        // Check if this is a trait implementation
        let trait_path = impl_block.trait_.as_ref()?.1.clone();
        let trait_name = extract_trait_name(&trait_path)?;

        // Match against standard traits
        let standard_trait = self.match_standard_trait(&trait_name)?;
        let category = self.trait_patterns.get(&standard_trait)?;

        Some(TraitImplClassification {
            trait_name: trait_name.to_string(),
            standard_trait: Some(standard_trait),
            category: *category,
            confidence: 0.95,  // High confidence for trait impls
            evidence: format!("Implements {} trait", trait_name),
        })
    }

    fn match_standard_trait(&self, trait_name: &str) -> Option<StandardTrait> {
        match trait_name {
            "Display" => Some(StandardTrait::Display),
            "Debug" => Some(StandardTrait::Debug),
            "From" => Some(StandardTrait::From),
            "Into" => Some(StandardTrait::Into),
            "TryFrom" => Some(StandardTrait::TryFrom),
            "TryInto" => Some(StandardTrait::TryInto),
            "Default" => Some(StandardTrait::Default),
            "Clone" => Some(StandardTrait::Clone),
            "Drop" => Some(StandardTrait::Drop),
            "Iterator" => Some(StandardTrait::Iterator),
            "IntoIterator" => Some(StandardTrait::IntoIterator),
            "Add" | "Sub" | "Mul" | "Div" => Some(StandardTrait::Add),  // Simplification
            "Serialize" => Some(StandardTrait::Serialize),
            "Deserialize" => Some(StandardTrait::Deserialize),
            _ => None,
        }
    }
}
```

**Phase 2: Async/Concurrency Pattern Detection**

```rust
#[derive(Debug, Clone)]
pub struct AsyncPattern {
    pub pattern_type: AsyncPatternType,
    pub confidence: f64,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsyncPatternType {
    AsyncFunction,
    TaskSpawning,
    ChannelCommunication,
    MutexUsage,
    SelectMacro,
    JoinMacro,
}

impl RustPatternDetector {
    pub fn detect_async_patterns(&self, function: &FunctionAst) -> Vec<AsyncPattern> {
        let mut patterns = Vec::new();

        // Async function
        if function.is_async {
            patterns.push(AsyncPattern {
                pattern_type: AsyncPatternType::AsyncFunction,
                confidence: 1.0,
                evidence: "Function is declared as async".into(),
            });
        }

        // Task spawning (tokio::spawn, async_std::task::spawn)
        for call in &function.calls {
            if call.name.contains("spawn") &&
               (call.name.contains("tokio") || call.name.contains("async_std")) {
                patterns.push(AsyncPattern {
                    pattern_type: AsyncPatternType::TaskSpawning,
                    confidence: 0.9,
                    evidence: format!("Spawns async task: {}", call.name),
                });
            }
        }

        // Channel usage (tokio::sync::mpsc, crossbeam::channel)
        for call in &function.calls {
            if call.name.contains("channel") ||
               call.name.contains("send") ||
               call.name.contains("recv") {
                patterns.push(AsyncPattern {
                    pattern_type: AsyncPatternType::ChannelCommunication,
                    confidence: 0.8,
                    evidence: "Uses channel communication".into(),
                });
                break;
            }
        }

        // Mutex/RwLock usage
        if function.body_text.contains("Mutex::new") ||
           function.body_text.contains(".lock()") ||
           function.body_text.contains("RwLock") {
            patterns.push(AsyncPattern {
                pattern_type: AsyncPatternType::MutexUsage,
                confidence: 0.85,
                evidence: "Uses mutex/rwlock for synchronization".into(),
            });
        }

        patterns
    }

    pub fn classify_from_async_patterns(&self, patterns: &[AsyncPattern]) -> Option<ResponsibilityCategory> {
        if patterns.is_empty() {
            return None;
        }

        // Task spawning = Concurrency Management
        if patterns.iter().any(|p| p.pattern_type == AsyncPatternType::TaskSpawning) {
            return Some(ResponsibilityCategory::ConcurrencyManagement);
        }

        // Async function with channel = Communication
        if patterns.iter().any(|p| p.pattern_type == AsyncPatternType::ChannelCommunication) {
            return Some(ResponsibilityCategory::CommunicationOrchestration);
        }

        // Just async = Asynchronous Operation
        if patterns.iter().any(|p| p.pattern_type == AsyncPatternType::AsyncFunction) {
            return Some(ResponsibilityCategory::AsynchronousOperation);
        }

        None
    }
}
```

**Phase 3: Error Handling Pattern Detection**

```rust
#[derive(Debug, Clone)]
pub struct ErrorPattern {
    pub pattern_type: ErrorPatternType,
    pub count: usize,
    pub locations: Vec<SourceLocation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPatternType {
    QuestionMarkOperator,      // Uses ? for error propagation
    CustomErrorType,            // Defines custom error type
    ErrorConversion,            // Implements From for error conversion
    UnwrapUsage,                // Anti-pattern: unwrap()
    PanicUsage,                 // Anti-pattern: panic!()
    ExpectUsage,                // Better than unwrap: expect()
}

impl RustPatternDetector {
    pub fn detect_error_patterns(&self, function: &FunctionAst) -> Vec<ErrorPattern> {
        let mut patterns = Vec::new();

        // Count ? operator usage
        let question_mark_count = function.body_text.matches('?').count();
        if question_mark_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::QuestionMarkOperator,
                count: question_mark_count,
                locations: vec![],  // Would need AST parsing for exact locations
            });
        }

        // Detect unwrap() usage (anti-pattern)
        let unwrap_count = function.body_text.matches(".unwrap()").count();
        if unwrap_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::UnwrapUsage,
                count: unwrap_count,
                locations: vec![],
            });
        }

        // Check return type for Result
        if let Some(ref return_type) = function.return_type {
            if return_type.contains("Result<") {
                patterns.push(ErrorPattern {
                    pattern_type: ErrorPatternType::QuestionMarkOperator,
                    count: 1,  // Return type indicates error handling
                    locations: vec![],
                });
            }
        }

        patterns
    }

    pub fn classify_from_error_patterns(&self, patterns: &[ErrorPattern]) -> Option<ResponsibilityCategory> {
        // High ? usage = Error Propagation & Handling
        let question_mark_count: usize = patterns.iter()
            .filter(|p| p.pattern_type == ErrorPatternType::QuestionMarkOperator)
            .map(|p| p.count)
            .sum();

        if question_mark_count >= 3 {
            return Some(ResponsibilityCategory::ErrorHandling);
        }

        // Custom error type = Error Handling
        if patterns.iter().any(|p| p.pattern_type == ErrorPatternType::CustomErrorType) {
            return Some(ResponsibilityCategory::ErrorHandling);
        }

        None
    }
}
```

**Phase 4: Builder Pattern Detection**

```rust
#[derive(Debug, Clone)]
pub struct BuilderPattern {
    pub is_builder: bool,
    pub chainable_methods: Vec<String>,
    pub finalizer_method: Option<String>,
    pub confidence: f64,
}

impl RustPatternDetector {
    pub fn detect_builder_pattern(&self, struct_methods: &[FunctionAst]) -> BuilderPattern {
        let mut chainable_methods = Vec::new();
        let mut finalizer_method = None;

        for method in struct_methods {
            // Chainable: returns Self
            if let Some(ref return_type) = method.return_type {
                if return_type == "Self" || return_type.ends_with("Self") {
                    chainable_methods.push(method.name.clone());
                }
                // Finalizer: build() or finalize()
                else if method.name == "build" || method.name == "finalize" {
                    finalizer_method = Some(method.name.clone());
                }
            }
        }

        let is_builder = chainable_methods.len() >= 2 || finalizer_method.is_some();
        let confidence = if chainable_methods.len() >= 3 && finalizer_method.is_some() {
            0.95
        } else if chainable_methods.len() >= 2 {
            0.80
        } else {
            0.50
        };

        BuilderPattern {
            is_builder,
            chainable_methods,
            finalizer_method,
            confidence,
        }
    }
}
```

**Phase 5: Integration with Multi-Signal**

```rust
pub fn classify_rust_function(
    function: &FunctionAst,
    rust_detector: &RustPatternDetector,
) -> Option<RustSpecificClassification> {
    // Priority order: Most specific patterns first

    // 1. Trait implementations (highest confidence)
    if let Some(trait_impl) = rust_detector.detect_trait_impl(function.parent_impl?) {
        return Some(RustSpecificClassification {
            category: trait_impl.category,
            confidence: trait_impl.confidence,
            evidence: trait_impl.evidence,
            rust_pattern: RustPattern::TraitImplementation(trait_impl),
        });
    }

    // 2. Async/concurrency patterns
    let async_patterns = rust_detector.detect_async_patterns(function);
    if let Some(category) = rust_detector.classify_from_async_patterns(&async_patterns) {
        return Some(RustSpecificClassification {
            category,
            confidence: 0.85,
            evidence: format!("Async patterns: {:?}", async_patterns),
            rust_pattern: RustPattern::AsyncConcurrency(async_patterns),
        });
    }

    // 3. Error handling patterns
    let error_patterns = rust_detector.detect_error_patterns(function);
    if let Some(category) = rust_detector.classify_from_error_patterns(&error_patterns) {
        return Some(RustSpecificClassification {
            category,
            confidence: 0.75,
            evidence: format!("Error handling: {:?}", error_patterns),
            rust_pattern: RustPattern::ErrorHandling(error_patterns),
        });
    }

    None
}
```

### Architecture Changes

**New Module**: `src/analysis/rust_patterns/`
- `detector.rs` - Main Rust pattern detection
- `traits.rs` - Standard trait detection
- `async_patterns.rs` - Async/concurrency patterns
- `error_patterns.rs` - Error handling patterns
- `builders.rs` - Builder pattern detection
- `conversions.rs` - Type conversion patterns

**Integration Point**: `src/analysis/multi_signal_aggregation.rs`
- Add Rust-specific signal to SignalSet
- Weight: 5-10% for Rust code (0% for other languages)
- Override generic classifications with Rust-specific when high confidence

## Dependencies

- **Prerequisites**: Spec 141 (I/O Detection), Spec 142 (Call Graph)
- **Optional Integration**: Spec 145 (Multi-Signal Aggregation)
- **Affected Components**:
  - `src/analysis/` - new rust_patterns module
  - `src/analyzers/rust_analyzer.rs` - integration point
- **External Dependencies**:
  - `syn` (already in use for Rust parsing)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_display_trait() {
        let code = r#"
        impl Display for MyType {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                write!(f, "MyType")
            }
        }
        "#;

        let impl_block = parse_impl_block(code);
        let detector = RustPatternDetector::new();

        let classification = detector.detect_trait_impl(&impl_block).unwrap();
        assert_eq!(classification.category, ResponsibilityCategory::Formatting);
        assert_eq!(classification.standard_trait, Some(StandardTrait::Display));
    }

    #[test]
    fn detect_async_spawn() {
        let code = r#"
        async fn process_tasks() {
            tokio::spawn(async {
                // Task logic
            });
        }
        "#;

        let ast = parse_rust(code);
        let detector = RustPatternDetector::new();

        let patterns = detector.detect_async_patterns(&ast.functions[0]);
        assert!(patterns.iter().any(|p| p.pattern_type == AsyncPatternType::TaskSpawning));
    }

    #[test]
    fn detect_error_propagation() {
        let code = r#"
        fn read_config() -> Result<Config, Error> {
            let file = File::open("config.toml")?;
            let content = read_to_string(file)?;
            let config = parse_toml(&content)?;
            Ok(config)
        }
        "#;

        let ast = parse_rust(code);
        let detector = RustPatternDetector::new();

        let patterns = detector.detect_error_patterns(&ast.functions[0]);
        let question_marks: usize = patterns.iter()
            .filter(|p| p.pattern_type == ErrorPatternType::QuestionMarkOperator)
            .map(|p| p.count)
            .sum();

        assert!(question_marks >= 3);
    }

    #[test]
    fn detect_builder_pattern() {
        let code = r#"
        struct Builder { /* fields */ }

        impl Builder {
            fn new() -> Self { Builder {} }
            fn with_name(mut self, name: String) -> Self { self }
            fn with_age(mut self, age: u32) -> Self { self }
            fn build(self) -> Config { Config {} }
        }
        "#;

        let ast = parse_rust(code);
        let detector = RustPatternDetector::new();

        let pattern = detector.detect_builder_pattern(&ast.impl_blocks[0].methods);
        assert!(pattern.is_builder);
        assert_eq!(pattern.finalizer_method, Some("build".to_string()));
        assert!(pattern.confidence > 0.9);
    }
}
```

### Integration Tests

```rust
#[test]
fn rust_patterns_on_debtmap_code() {
    let files = vec![
        "src/analyzers/rust_analyzer.rs",
        "src/config.rs",
        "src/io/reader.rs",
    ];

    let detector = RustPatternDetector::new();

    for file_path in files {
        let ast = parse_file(file_path);

        for function in ast.functions() {
            let rust_classification = classify_rust_function(function, &detector);

            if let Some(classification) = rust_classification {
                println!("{}: {} ({:.2})",
                    function.name,
                    classification.category,
                    classification.confidence
                );
            }
        }
    }
}
```

## Documentation Requirements

### User Documentation

Update README.md:
```markdown
## Rust-Specific Pattern Detection

For Rust code, debtmap recognizes language-specific patterns:

**Trait Implementations**:
- Display/Debug → Formatting
- From/Into → Type Conversion
- Default/Clone → Construction
- Drop → Resource Cleanup
- Iterator → Iteration Logic

**Async Patterns**:
- async fn → Asynchronous Operation
- tokio::spawn → Concurrency Management
- Channels → Communication Orchestration

**Error Handling**:
- High ? usage → Error Propagation
- Custom error types → Error Handling
- unwrap() usage → Flagged as anti-pattern

**Builder Patterns**:
- Chainable methods → Configuration Builder
- build() method → Finalization
```

## Implementation Notes

### Syn Integration

Use `syn` crate for accurate Rust AST parsing:

```rust
use syn::{parse_file, Item, ItemImpl, ImplItem};

pub fn extract_trait_implementations(code: &str) -> Vec<ItemImpl> {
    let syntax_tree = parse_file(code).unwrap();

    syntax_tree.items
        .into_iter()
        .filter_map(|item| {
            if let Item::Impl(impl_block) = item {
                if impl_block.trait_.is_some() {
                    return Some(impl_block);
                }
            }
            None
        })
        .collect()
}
```

## Migration and Compatibility

### Language-Specific Signals

Rust patterns only apply to Rust code:

```rust
impl SignalSet {
    pub fn collect_for_function(...) -> Self {
        let rust_signal = if context.language == Language::Rust {
            Some(classify_rust_function(function, &context.rust_detector))
        } else {
            None
        };

        SignalSet {
            // ... other signals
            rust_specific_signal: rust_signal,
        }
    }
}
```

## Expected Impact

### Accuracy Improvement for Rust

- **Generic multi-signal**: ~85% accuracy on Rust code
- **+ Rust patterns**: ~90% accuracy on Rust code
- **Improvement**: +5 percentage points for Rust

### Better Rust-Specific Classifications

```rust
// Before (generic)
impl Display for User {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { ... }
}
// Classification: "General Logic"

// After (Rust-specific)
impl Display for User {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { ... }
}
// Classification: "Formatting (Display trait)" (0.95 confidence)
```

### Foundation for Language-Specific Analysis

This pattern can be extended to Python and JavaScript:
- Python: Decorators, magic methods, context managers
- JavaScript: Prototypes, promises, async/await patterns
