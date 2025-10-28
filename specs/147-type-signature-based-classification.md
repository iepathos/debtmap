---
number: 147
title: Type Signature-Based Classification
category: foundation
priority: medium
status: draft
dependencies: [127]
created: 2025-10-27
---

# Specification 147: Type Signature-Based Classification

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 127 (Type Flow Tracking Infrastructure)

## Context

Function type signatures (input parameters and return types) provide valuable signals about responsibility, often more reliable than function names:

**Example Type Patterns**:
```rust
// Validation: Takes data, returns Result
fn validate_email(email: &str) -> Result<(), ValidationError>

// Formatting: Takes data, returns String
fn format_user(user: &User) -> String

// Parsing: Takes String, returns structured data
fn parse_config(content: &str) -> Result<Config, ParseError>

// I/O: Returns Result with I/O error type
fn read_file(path: &Path) -> Result<String, io::Error>

// Transformation: Takes data, returns transformed data
fn normalize_path(path: &Path) -> PathBuf
```

Type signatures reveal:
- **Input types**: What kind of data does this function consume?
- **Return types**: What does it produce?
- **Error types**: What can go wrong? (I/O errors, validation errors, parsing errors)
- **Generic constraints**: What capabilities are required? (Read, Write, Send, Sync)

Current name-based classification misses these patterns. A function named `process()` could be validation, formatting, parsing, or computation—but the type signature reveals the truth.

Type signature analysis adds ~15% weight to multi-signal classification, improving accuracy by 5-8 percentage points.

## Objective

Classify function responsibilities based on type signatures: input parameter types, return types, error types, and generic constraints. Enable responsibility detection to recognize common type patterns (parsers, formatters, validators, transformers) independent of function naming.

## Requirements

### Functional Requirements

**Type Pattern Recognition**:
- Detect parser patterns (`String → Result<T>`, `&str → T`)
- Detect formatter patterns (`T → String`, `T → Display`)
- Detect validator patterns (`T → Result<(), Error>`, `T → bool`)
- Detect transformer patterns (`T → U`, same category types)
- Detect I/O patterns (returns `io::Result`, `io::Error`)
- Detect builder patterns (`Self → Self` chains)
- Detect query patterns (`&T → Option<U>`, `&T → Vec<U>`)

**Error Type Analysis**:
- Classify by error type (io::Error → I/O, ValidationError → Validation)
- Track custom error types and their purposes
- Detect error conversion patterns

**Generic Constraint Analysis**:
- Identify trait bounds (Read, Write, Iterator, etc.)
- Classify based on required capabilities
- Detect framework-specific trait requirements

**Multi-Language Support**:
- Rust: Full type signature analysis with Result, Option, generic bounds
- Python: Type hints (if present), return annotations
- TypeScript: Full type signature analysis
- JavaScript: Limited support (JSDoc types if available)

**Classification Output**:
- Inferred responsibility from type signature
- Confidence score based on pattern strength
- Evidence explaining the classification

### Non-Functional Requirements

- **Accuracy**: Correctly classify >80% of functions with clear type patterns
- **Performance**: Type analysis adds <5% overhead
- **Coverage**: Support 20+ common type patterns
- **Extensibility**: New patterns can be added via configuration

## Acceptance Criteria

- [ ] Parser patterns are correctly identified (`String → Result<T>`)
- [ ] Formatter patterns are correctly identified (`T → String`)
- [ ] Validator patterns are correctly identified (`T → Result<(), E>`)
- [ ] I/O error types correctly indicate I/O responsibility
- [ ] Generic trait bounds are analyzed (Read, Write, Iterator)
- [ ] Custom error types are classified by name patterns
- [ ] Type signatures work for Rust, Python (with hints), TypeScript
- [ ] Confidence scores reflect pattern strength
- [ ] Performance overhead <5% on large codebases
- [ ] Test suite includes diverse type signature examples

## Technical Details

### Implementation Approach

**Phase 1: Type Pattern Definitions**

```rust
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TypePattern {
    pub name: String,
    pub input_pattern: TypeMatcher,
    pub output_pattern: TypeMatcher,
    pub category: ResponsibilityCategory,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub enum TypeMatcher {
    /// Exact type match
    Exact(String),
    /// Regex pattern match
    Regex(Regex),
    /// Generic with constraint
    Generic { name: String, bounds: Vec<String> },
    /// Result with specific error type
    Result { ok_type: Box<TypeMatcher>, error_pattern: Box<TypeMatcher> },
    /// Option type
    Option(Box<TypeMatcher>),
    /// Any type (wildcard)
    Any,
    /// Collection type (Vec, HashMap, etc.)
    Collection { element_type: Box<TypeMatcher> },
}

pub struct TypePatternLibrary {
    patterns: Vec<TypePattern>,
}

impl TypePatternLibrary {
    pub fn default_patterns() -> Self {
        let mut patterns = Vec::new();

        // Parser: String/&str → Result<T, E>
        patterns.push(TypePattern {
            name: "String Parser".into(),
            input_pattern: TypeMatcher::Regex(Regex::new(r"&?str|String").unwrap()),
            output_pattern: TypeMatcher::Result {
                ok_type: Box::new(TypeMatcher::Any),
                error_pattern: Box::new(TypeMatcher::Any),
            },
            category: ResponsibilityCategory::Parsing,
            confidence: 0.85,
        });

        // Formatter: T → String
        patterns.push(TypePattern {
            name: "String Formatter".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Exact("String".into()),
            category: ResponsibilityCategory::Formatting,
            confidence: 0.75,
        });

        // Validator: T → Result<(), ValidationError>
        patterns.push(TypePattern {
            name: "Validator".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Result {
                ok_type: Box::new(TypeMatcher::Exact("()".into())),
                error_pattern: Box::new(TypeMatcher::Regex(Regex::new(r".*Validation.*Error").unwrap())),
            },
            category: ResponsibilityCategory::Validation,
            confidence: 0.90,
        });

        // I/O Operation: Returns io::Result or io::Error
        patterns.push(TypePattern {
            name: "I/O Operation".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Result {
                ok_type: Box::new(TypeMatcher::Any),
                error_pattern: Box::new(TypeMatcher::Regex(Regex::new(r"io::Error|std::io::Error").unwrap())),
            },
            category: ResponsibilityCategory::FileIO,
            confidence: 0.85,
        });

        // Query: &T → Option<U>
        patterns.push(TypePattern {
            name: "Query/Lookup".into(),
            input_pattern: TypeMatcher::Regex(Regex::new(r"&.*").unwrap()),
            output_pattern: TypeMatcher::Option(Box::new(TypeMatcher::Any)),
            category: ResponsibilityCategory::DataAccess,
            confidence: 0.70,
        });

        // Builder: Self → Self
        patterns.push(TypePattern {
            name: "Builder Method".into(),
            input_pattern: TypeMatcher::Exact("Self".into()),
            output_pattern: TypeMatcher::Exact("Self".into()),
            category: ResponsibilityCategory::Construction,
            confidence: 0.80,
        });

        // Collection Query: &T → Vec<U>
        patterns.push(TypePattern {
            name: "Collection Query".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Collection {
                element_type: Box::new(TypeMatcher::Any),
            },
            category: ResponsibilityCategory::DataAccess,
            confidence: 0.65,
        });

        // Transformer: T → U (different types)
        patterns.push(TypePattern {
            name: "Data Transformation".into(),
            input_pattern: TypeMatcher::Any,
            output_pattern: TypeMatcher::Any,
            category: ResponsibilityCategory::Transformation,
            confidence: 0.50,  // Low confidence, very generic
        });

        TypePatternLibrary { patterns }
    }
}
```

**Phase 2: Type Signature Analysis**

```rust
#[derive(Debug, Clone)]
pub struct TypeSignature {
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub generic_bounds: Vec<GenericBound>,
    pub error_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: String,
    pub is_reference: bool,
    pub is_mutable: bool,
}

#[derive(Debug, Clone)]
pub struct GenericBound {
    pub type_param: String,
    pub trait_bounds: Vec<String>,
}

pub struct TypeSignatureAnalyzer {
    pattern_library: TypePatternLibrary,
}

impl TypeSignatureAnalyzer {
    pub fn analyze_signature(&self, signature: &TypeSignature) -> Option<TypeBasedClassification> {
        // Try to match against known patterns
        for pattern in &self.pattern_library.patterns {
            if self.matches_pattern(signature, pattern) {
                return Some(TypeBasedClassification {
                    category: pattern.category,
                    confidence: pattern.confidence,
                    evidence: format!(
                        "Matches '{}' pattern: {} → {}",
                        pattern.name,
                        self.format_inputs(signature),
                        signature.return_type.as_ref().unwrap_or(&"()".into())
                    ),
                    pattern_name: pattern.name.clone(),
                });
            }
        }

        // Check error type for I/O classification
        if let Some(ref error_type) = signature.error_type {
            if let Some(category) = self.classify_by_error_type(error_type) {
                return Some(TypeBasedClassification {
                    category,
                    confidence: 0.80,
                    evidence: format!("Error type suggests I/O: {}", error_type),
                    pattern_name: "Error Type Classification".into(),
                });
            }
        }

        // Check generic bounds
        if let Some(category) = self.classify_by_generic_bounds(&signature.generic_bounds) {
            return Some(TypeBasedClassification {
                category,
                confidence: 0.70,
                evidence: format!("Generic bounds: {:?}", signature.generic_bounds),
                pattern_name: "Generic Bound Classification".into(),
            });
        }

        None
    }

    fn matches_pattern(&self, signature: &TypeSignature, pattern: &TypePattern) -> bool {
        // Match input pattern
        let input_match = if signature.parameters.is_empty() {
            matches!(pattern.input_pattern, TypeMatcher::Any)
        } else {
            signature.parameters.iter().any(|param| {
                self.type_matches(&param.type_annotation, &pattern.input_pattern)
            })
        };

        // Match output pattern
        let output_match = signature.return_type.as_ref()
            .map(|rt| self.type_matches(rt, &pattern.output_pattern))
            .unwrap_or(false);

        input_match && output_match
    }

    fn type_matches(&self, type_str: &str, matcher: &TypeMatcher) -> bool {
        match matcher {
            TypeMatcher::Exact(expected) => type_str == expected,
            TypeMatcher::Regex(regex) => regex.is_match(type_str),
            TypeMatcher::Any => true,
            TypeMatcher::Result { ok_type, error_pattern } => {
                type_str.starts_with("Result<") &&
                self.extract_result_types(type_str)
                    .map(|(ok, err)| {
                        self.type_matches(&ok, ok_type) &&
                        self.type_matches(&err, error_pattern)
                    })
                    .unwrap_or(false)
            }
            TypeMatcher::Option(inner) => {
                type_str.starts_with("Option<") &&
                self.extract_option_type(type_str)
                    .map(|inner_type| self.type_matches(&inner_type, inner))
                    .unwrap_or(false)
            }
            TypeMatcher::Collection { element_type } => {
                (type_str.starts_with("Vec<") ||
                 type_str.starts_with("HashMap<") ||
                 type_str.starts_with("HashSet<")) &&
                self.extract_collection_element(type_str)
                    .map(|elem| self.type_matches(&elem, element_type))
                    .unwrap_or(false)
            }
            TypeMatcher::Generic { name, bounds } => {
                type_str == name
                // Would need to check bounds from signature
            }
        }
    }

    fn extract_result_types(&self, type_str: &str) -> Option<(String, String)> {
        // Parse "Result<OkType, ErrType>" → (OkType, ErrType)
        let inner = type_str.strip_prefix("Result<")?.strip_suffix(">")?;
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    fn classify_by_error_type(&self, error_type: &str) -> Option<ResponsibilityCategory> {
        let lower = error_type.to_lowercase();

        if lower.contains("io") || lower.contains("file") || lower.contains("network") {
            return Some(ResponsibilityCategory::FileIO);
        }

        if lower.contains("validation") || lower.contains("validate") {
            return Some(ResponsibilityCategory::Validation);
        }

        if lower.contains("parse") {
            return Some(ResponsibilityCategory::Parsing);
        }

        if lower.contains("format") {
            return Some(ResponsibilityCategory::Formatting);
        }

        None
    }

    fn classify_by_generic_bounds(&self, bounds: &[GenericBound]) -> Option<ResponsibilityCategory> {
        for bound in bounds {
            // Check for I/O trait bounds
            if bound.trait_bounds.iter().any(|t| t.contains("Read") || t.contains("Write")) {
                return Some(ResponsibilityCategory::FileIO);
            }

            // Check for iterator trait bounds
            if bound.trait_bounds.iter().any(|t| t.contains("Iterator") || t.contains("IntoIterator")) {
                return Some(ResponsibilityCategory::Iteration);
            }

            // Check for serialization
            if bound.trait_bounds.iter().any(|t| t.contains("Serialize") || t.contains("Deserialize")) {
                return Some(ResponsibilityCategory::Serialization);
            }
        }

        None
    }
}
```

**Phase 3: Language-Specific Type Extraction**

```rust
// Rust type extraction (using syn)
pub fn extract_rust_signature(function: &syn::ItemFn) -> TypeSignature {
    let parameters: Vec<Parameter> = function.sig.inputs.iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                Some(Parameter {
                    name: extract_param_name(&pat_type.pat),
                    type_annotation: quote!(#pat_type.ty).to_string(),
                    is_reference: is_reference_type(&pat_type.ty),
                    is_mutable: is_mutable(&pat_type.pat),
                })
            } else {
                None
            }
        })
        .collect();

    let return_type = match &function.sig.output {
        syn::ReturnType::Type(_, ty) => Some(quote!(#ty).to_string()),
        syn::ReturnType::Default => None,
    };

    let error_type = extract_error_type_from_return(&return_type);

    let generic_bounds = function.sig.generics.params.iter()
        .filter_map(|param| {
            if let syn::GenericParam::Type(type_param) = param {
                let bounds: Vec<String> = type_param.bounds.iter()
                    .map(|bound| quote!(#bound).to_string())
                    .collect();

                Some(GenericBound {
                    type_param: type_param.ident.to_string(),
                    trait_bounds: bounds,
                })
            } else {
                None
            }
        })
        .collect();

    TypeSignature {
        parameters,
        return_type,
        generic_bounds,
        error_type,
    }
}

// Python type extraction (from AST annotations)
pub fn extract_python_signature(function: &PythonFunction) -> TypeSignature {
    let parameters: Vec<Parameter> = function.args.iter()
        .map(|arg| Parameter {
            name: arg.name.clone(),
            type_annotation: arg.annotation.clone().unwrap_or_else(|| "Any".into()),
            is_reference: false,  // Python doesn't have explicit references
            is_mutable: true,     // Python is generally mutable
        })
        .collect();

    let return_type = function.returns.clone();

    TypeSignature {
        parameters,
        return_type,
        generic_bounds: vec![],  // Python doesn't have Rust-style bounds
        error_type: None,  // Would need to analyze raises/exceptions
    }
}

// TypeScript type extraction
pub fn extract_typescript_signature(function: &TsFunction) -> TypeSignature {
    // Similar to Python but with full TypeScript type system support
    // Would use ts-morph or similar for AST parsing
    todo!()
}
```

**Phase 4: Integration with Multi-Signal**

```rust
impl TypeSignatureAnalyzer {
    pub fn classify_function(&self, function: &FunctionAst) -> Option<TypeBasedClassification> {
        let signature = match function.language {
            Language::Rust => extract_rust_signature(&function.rust_ast?),
            Language::Python => extract_python_signature(&function.python_ast?),
            Language::TypeScript => extract_typescript_signature(&function.ts_ast?),
            Language::JavaScript => return None,  // Limited JS support
        };

        self.analyze_signature(&signature)
    }
}

// Used in multi-signal aggregation (Spec 145)
pub fn collect_type_signature_signal(
    function: &FunctionAst,
    analyzer: &TypeSignatureAnalyzer,
) -> Option<TypeBasedClassification> {
    analyzer.classify_function(function)
}
```

### Architecture Changes

**New Module**: `src/analysis/type_signatures/`
- `analyzer.rs` - Main type signature analysis
- `patterns.rs` - Type pattern library
- `extractors/` - Language-specific type extraction
  - `rust.rs` - Rust type extraction (using syn)
  - `python.rs` - Python type extraction
  - `typescript.rs` - TypeScript type extraction

**Integration Point**: `src/analysis/multi_signal_aggregation.rs`
- Add type signature signal to SignalSet
- Weight: 15% in default configuration
- Combine with other signals for final classification

**Dependencies**:
```toml
[dependencies]
syn = { version = "2.0", features = ["full", "parsing"] }
quote = "1.0"
```

## Dependencies

- **Prerequisites**: Spec 127 (Type Flow Tracking) provides foundation
- **Affected Components**:
  - `src/analysis/` - new type_signatures module
  - `src/analysis/multi_signal_aggregation.rs` - integration
- **External Dependencies**:
  - `syn` (for Rust type parsing)
  - `quote` (for type stringification)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_pattern() {
        let signature = TypeSignature {
            parameters: vec![Parameter {
                name: "input".into(),
                type_annotation: "&str".into(),
                is_reference: true,
                is_mutable: false,
            }],
            return_type: Some("Result<Config, ParseError>".into()),
            generic_bounds: vec![],
            error_type: Some("ParseError".into()),
        };

        let analyzer = TypeSignatureAnalyzer::new();
        let classification = analyzer.analyze_signature(&signature).unwrap();

        assert_eq!(classification.category, ResponsibilityCategory::Parsing);
        assert!(classification.confidence > 0.8);
    }

    #[test]
    fn formatter_pattern() {
        let signature = TypeSignature {
            parameters: vec![Parameter {
                name: "user".into(),
                type_annotation: "&User".into(),
                is_reference: true,
                is_mutable: false,
            }],
            return_type: Some("String".into()),
            generic_bounds: vec![],
            error_type: None,
        };

        let analyzer = TypeSignatureAnalyzer::new();
        let classification = analyzer.analyze_signature(&signature).unwrap();

        assert_eq!(classification.category, ResponsibilityCategory::Formatting);
    }

    #[test]
    fn validator_pattern() {
        let signature = TypeSignature {
            parameters: vec![Parameter {
                name: "email".into(),
                type_annotation: "&str".into(),
                is_reference: true,
                is_mutable: false,
            }],
            return_type: Some("Result<(), ValidationError>".into()),
            generic_bounds: vec![],
            error_type: Some("ValidationError".into()),
        };

        let analyzer = TypeSignatureAnalyzer::new();
        let classification = analyzer.analyze_signature(&signature).unwrap();

        assert_eq!(classification.category, ResponsibilityCategory::Validation);
        assert!(classification.confidence > 0.85);
    }

    #[test]
    fn io_error_classification() {
        let signature = TypeSignature {
            parameters: vec![],
            return_type: Some("Result<String, io::Error>".into()),
            generic_bounds: vec![],
            error_type: Some("io::Error".into()),
        };

        let analyzer = TypeSignatureAnalyzer::new();
        let classification = analyzer.analyze_signature(&signature).unwrap();

        assert_eq!(classification.category, ResponsibilityCategory::FileIO);
    }

    #[test]
    fn generic_bound_classification() {
        let signature = TypeSignature {
            parameters: vec![Parameter {
                name: "reader".into(),
                type_annotation: "R".into(),
                is_reference: false,
                is_mutable: true,
            }],
            return_type: Some("Result<Data, io::Error>".into()),
            generic_bounds: vec![GenericBound {
                type_param: "R".into(),
                trait_bounds: vec!["Read".into()],
            }],
            error_type: Some("io::Error".into()),
        };

        let analyzer = TypeSignatureAnalyzer::new();
        let classification = analyzer.analyze_signature(&signature).unwrap();

        assert_eq!(classification.category, ResponsibilityCategory::FileIO);
    }
}
```

### Integration Tests

```rust
#[test]
fn type_signatures_on_debtmap_code() {
    let files = vec![
        "src/io/reader.rs",
        "src/analyzers/rust_analyzer.rs",
        "src/config.rs",
    ];

    let analyzer = TypeSignatureAnalyzer::new();

    for file_path in files {
        let ast = parse_file(file_path);

        for function in ast.functions() {
            if let Some(classification) = analyzer.classify_function(function) {
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
## Type Signature Analysis

Debtmap analyzes function type signatures to infer responsibilities:

**Common Patterns**:
- `&str → Result<T, E>` → Parsing
- `T → String` → Formatting
- `T → Result<(), ValidationError>` → Validation
- `Path → Result<T, io::Error>` → File I/O
- `&T → Option<U>` → Data Access/Query
- `Self → Self` → Builder Pattern

**Error Type Classification**:
- `io::Error` → I/O Operations
- `ValidationError` → Validation
- `ParseError` → Parsing

**Generic Bounds**:
- `T: Read` → File I/O
- `T: Iterator` → Iteration
- `T: Serialize` → Serialization
```

## Implementation Notes

### Handling Complex Types

For complex generic types, use heuristics:

```rust
// HashMap<String, Vec<User>> → Collection access
// Result<Option<T>, E> → Nested Result/Option handling
// impl Iterator<Item = T> → Iterator pattern
```

### Python Type Hints

Only works when type hints are present:

```python
def parse_config(content: str) -> dict:  # Detectable
    ...

def parse_config(content):  # Not detectable via types
    ...
```

## Migration and Compatibility

### Gradual Adoption

Type signatures complement other signals:

```rust
// Strong type signal + weak name signal = Better classification
fn process(s: &str) -> Result<Config, ParseError>
// Type: Parsing (0.85)
// Name: Generic (0.40)
// Combined: Parsing (weighted aggregation)
```

## Expected Impact

### Accuracy Improvement

- **Without type signatures**: ~82% accuracy
- **With type signatures**: ~87% accuracy
- **Improvement**: +5 percentage points

### Better Classification Examples

```rust
// Before (name-based)
fn process(data: &str) -> Result<Config, ParseError>
// Classification: "Data Processing" (generic)

// After (type-based)
fn process(data: &str) -> Result<Config, ParseError>
// Classification: "Parsing" (0.85 confidence)
// Evidence: Matches 'String Parser' pattern: &str → Result<Config, ParseError>
```

### Multi-Signal Weight

In Spec 145 (Multi-Signal Aggregation):
- I/O Detection: 40%
- Call Graph: 30%
- **Type Signatures: 15%** ← This spec
- Purity: 10%
- Framework/Language: 5%
- Name: 5%

Type signatures provide orthogonal signal to I/O and call graph, improving classification when combined.
