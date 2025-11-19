# Spec 184: Hidden Type Extraction for Domain Modeling

**Status**: Draft
**Dependencies**: [181, 182, 183]
**Related**: [178, 179, 180]

## Problem

God objects often reveal missing domain types through parameter patterns and data relationships. Current recommendations don't suggest what new types to create, only how to split existing code. This leaves developers without guidance on proper domain modeling.

**Examples of hidden types in current code**:

```rust
// Parameter clump appears 5 times - hidden type!
fn format_header(score: f64, location: SourceLocation, metrics: &Metrics, verbosity: Verbosity)
fn render_section(score: f64, location: SourceLocation, metrics: &Metrics, verbosity: Verbosity)
fn validate_item(score: f64, location: SourceLocation, metrics: &Metrics)

// Hidden type: PriorityItem
pub struct PriorityItem {
    pub score: f64,
    pub location: SourceLocation,
    pub metrics: Metrics,
}

impl PriorityItem {
    pub fn format(&self, verbosity: Verbosity) -> String { ... }
    pub fn render(&self, verbosity: Verbosity) -> String { ... }
    pub fn validate(&self) -> Result<()> { ... }
}
```

```rust
// Tuple returns - hidden type!
fn analyze_struct(data: &StructData) -> (f64, Vec<String>, DomainDiversity)

// Hidden type: StructAnalysisResult
pub struct StructAnalysisResult {
    pub score: f64,
    pub domains: Vec<String>,
    pub diversity: DomainDiversity,
}
```

## Objective

Implement hidden type extraction that analyzes god objects to discover missing domain types through parameter patterns, tuple returns, and data relationships, then generates full type definitions with methods as refactoring guidance.

## Requirements

### 1. Parameter Clump Analysis

Detect repeated parameter patterns that should be structs:

```rust
// src/organization/hidden_type_extractor.rs

use std::collections::{HashMap, HashSet};
use crate::organization::type_based_clustering::{MethodSignature, TypeInfo};

pub struct HiddenTypeExtractor;

#[derive(Clone, Debug)]
pub struct HiddenType {
    pub suggested_name: String,
    pub fields: Vec<TypeField>,
    pub methods: Vec<TypeMethod>,
    pub occurrences: usize,
    pub confidence: f64,
    pub rationale: String,
    pub example_definition: String,
}

#[derive(Clone, Debug)]
pub struct TypeField {
    pub name: String,
    pub type_info: TypeInfo,
    pub visibility: Visibility,
}

#[derive(Clone, Debug)]
pub struct TypeMethod {
    pub name: String,
    pub signature: String,
    pub purpose: MethodPurpose,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MethodPurpose {
    Constructor,
    Transformation,
    Validation,
    Query,
    Display,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    PubCrate,
}

impl HiddenTypeExtractor {
    /// Find parameter clumps that appear in multiple functions
    pub fn find_parameter_clumps(
        &self,
        signatures: &[MethodSignature],
        min_occurrences: usize,
    ) -> Vec<ParameterClump> {
        let mut clump_map: HashMap<ParamSignature, Vec<String>> = HashMap::new();

        // Group methods by parameter signature
        for sig in signatures {
            let param_sig = self.create_param_signature(&sig.param_types);
            clump_map.entry(param_sig).or_default().push(sig.name.clone());
        }

        // Filter for clumps that appear min_occurrences times
        clump_map.into_iter()
            .filter(|(_, methods)| methods.len() >= min_occurrences)
            .map(|(param_sig, methods)| ParameterClump {
                params: param_sig.types,
                methods,
            })
            .collect()
    }

    fn create_param_signature(&self, params: &[TypeInfo]) -> ParamSignature {
        ParamSignature {
            types: params.to_vec(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParameterClump {
    pub params: Vec<TypeInfo>,
    pub methods: Vec<String>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct ParamSignature {
    types: Vec<TypeInfo>,
}

// Custom Hash/Eq for TypeInfo to make it work in HashMap
impl std::hash::Hash for TypeInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.is_reference.hash(state);
        self.is_mutable.hash(state);
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.is_reference == other.is_reference
            && self.is_mutable == other.is_mutable
    }
}

impl Eq for TypeInfo {}
```

### 2. Tuple Return Detection

Identify tuple returns that should be named structs:

```rust
impl HiddenTypeExtractor {
    /// Detect tuple returns that should be structs
    pub fn find_tuple_returns(
        &self,
        signatures: &[MethodSignature],
    ) -> Vec<TupleReturn> {
        signatures.iter()
            .filter_map(|sig| {
                if let Some(ret_type) = &sig.return_type {
                    if self.is_tuple(&ret_type.name) {
                        return Some(TupleReturn {
                            method_name: sig.name.clone(),
                            tuple_type: ret_type.name.clone(),
                            components: self.extract_tuple_components(&ret_type.name),
                        });
                    }
                }
                None
            })
            .collect()
    }

    fn is_tuple(&self, type_name: &str) -> bool {
        type_name.starts_with('(') && type_name.ends_with(')')
            || type_name.contains(',')
    }

    fn extract_tuple_components(&self, tuple_type: &str) -> Vec<String> {
        // Parse tuple like "(f64, Vec<String>, DomainDiversity)"
        tuple_type
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct TupleReturn {
    pub method_name: String,
    pub tuple_type: String,
    pub components: Vec<String>,
}
```

### 3. Hidden Type Synthesis

Combine clumps and tuples into complete type definitions:

```rust
impl HiddenTypeExtractor {
    /// Extract hidden types from god object analysis
    pub fn extract_hidden_types(
        &self,
        signatures: &[MethodSignature],
        module_name: &str,
    ) -> Vec<HiddenType> {
        let mut hidden_types = Vec::new();

        // 1. Parameter clumps (3+ occurrences)
        let clumps = self.find_parameter_clumps(signatures, 3);
        for clump in clumps {
            hidden_types.push(self.synthesize_type_from_clump(&clump, module_name));
        }

        // 2. Tuple returns
        let tuples = self.find_tuple_returns(signatures);
        for tuple in tuples {
            hidden_types.push(self.synthesize_type_from_tuple(&tuple, module_name));
        }

        // 3. Deduplicate and merge similar types
        self.deduplicate_types(hidden_types)
    }

    fn synthesize_type_from_clump(
        &self,
        clump: &ParameterClump,
        module_name: &str,
    ) -> HiddenType {
        let suggested_name = self.suggest_type_name_from_params(&clump.params, module_name);

        let fields = clump.params.iter()
            .enumerate()
            .map(|(i, type_info)| TypeField {
                name: self.suggest_field_name(type_info, i),
                type_info: type_info.clone(),
                visibility: Visibility::Public,
            })
            .collect();

        let methods = clump.methods.iter()
            .map(|method_name| TypeMethod {
                name: method_name.clone(),
                signature: format!("pub fn {}(&self, ...) -> ...", method_name),
                purpose: self.infer_method_purpose(method_name),
            })
            .collect();

        let example_definition = self.generate_struct_definition(
            &suggested_name,
            &fields,
            &methods,
        );

        HiddenType {
            suggested_name: suggested_name.clone(),
            fields,
            methods,
            occurrences: clump.methods.len(),
            confidence: self.calculate_confidence(clump.methods.len(), clump.params.len()),
            rationale: format!(
                "Parameter clump ({} parameters) appears in {} methods. \
                 Encapsulating these parameters in a struct improves cohesion and reduces coupling.",
                clump.params.len(),
                clump.methods.len()
            ),
            example_definition,
        }
    }

    fn synthesize_type_from_tuple(
        &self,
        tuple: &TupleReturn,
        module_name: &str,
    ) -> HiddenType {
        let suggested_name = self.suggest_type_name_from_method(&tuple.method_name, module_name);

        let fields: Vec<_> = tuple.components.iter()
            .enumerate()
            .map(|(i, component_type)| TypeField {
                name: format!("field_{}", i),
                type_info: TypeInfo {
                    name: component_type.clone(),
                    is_reference: false,
                    is_mutable: false,
                    generics: vec![],
                },
                visibility: Visibility::Public,
            })
            .collect();

        let methods = vec![
            TypeMethod {
                name: "new".to_string(),
                signature: format!(
                    "pub fn new({}) -> Self",
                    fields.iter()
                        .map(|f| format!("{}: {}", f.name, f.type_info.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                purpose: MethodPurpose::Constructor,
            }
        ];

        let example_definition = self.generate_struct_definition(
            &suggested_name,
            &fields,
            &methods,
        );

        HiddenType {
            suggested_name: suggested_name.clone(),
            fields,
            methods,
            occurrences: 1,
            confidence: 0.7, // Lower confidence for single tuple
            rationale: format!(
                "Method '{}' returns unnamed tuple {}. \
                 Named struct provides better documentation and type safety.",
                tuple.method_name,
                tuple.tuple_type
            ),
            example_definition,
        }
    }

    fn suggest_type_name_from_params(&self, params: &[TypeInfo], module_name: &str) -> String {
        // Extract nouns from type names
        let nouns: Vec<_> = params.iter()
            .map(|p| extract_noun(&p.name))
            .collect();

        // Combine into meaningful name
        if nouns.len() == 2 {
            format!("{}{}", nouns[0], nouns[1])
        } else {
            format!("{}Context", to_pascal_case(module_name))
        }
    }

    fn suggest_type_name_from_method(&self, method_name: &str, module_name: &str) -> String {
        // Extract noun from method name
        if method_name.starts_with("analyze") {
            format!("{}Result", to_pascal_case(&method_name.replace("analyze_", "")))
        } else if method_name.starts_with("calculate") {
            format!("{}Metrics", to_pascal_case(&method_name.replace("calculate_", "")))
        } else {
            format!("{}Result", to_pascal_case(method_name))
        }
    }

    fn suggest_field_name(&self, type_info: &TypeInfo, index: usize) -> String {
        // Convert type name to snake_case field name
        let type_lower = type_info.name.to_lowercase();

        if type_lower.ends_with("location") {
            "location".to_string()
        } else if type_lower.contains("metric") {
            "metrics".to_string()
        } else if type_lower.contains("score") {
            "score".to_string()
        } else {
            format!("field_{}", index)
        }
    }

    fn infer_method_purpose(&self, method_name: &str) -> MethodPurpose {
        if method_name.starts_with("format") || method_name.starts_with("display") {
            MethodPurpose::Display
        } else if method_name.starts_with("validate") || method_name.starts_with("check") {
            MethodPurpose::Validation
        } else if method_name.starts_with("new") || method_name.starts_with("create") {
            MethodPurpose::Constructor
        } else if method_name.starts_with("get") || method_name.starts_with("is") {
            MethodPurpose::Query
        } else {
            MethodPurpose::Transformation
        }
    }

    fn calculate_confidence(&self, occurrences: usize, param_count: usize) -> f64 {
        // Higher occurrences and more parameters = higher confidence
        let occurrence_factor = (occurrences as f64 / 10.0).min(1.0);
        let param_factor = (param_count as f64 / 5.0).min(1.0);

        (occurrence_factor + param_factor) / 2.0
    }

    fn deduplicate_types(&self, mut types: Vec<HiddenType>) -> Vec<HiddenType> {
        // Remove duplicate type suggestions based on field similarity
        types.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        let mut seen_signatures = HashSet::new();
        types.retain(|t| {
            let signature = self.type_signature(t);
            seen_signatures.insert(signature)
        });

        types
    }

    fn type_signature(&self, hidden_type: &HiddenType) -> String {
        hidden_type.fields.iter()
            .map(|f| f.type_info.name.clone())
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn extract_noun(type_name: &str) -> String {
    // Extract core noun from type (e.g., "SourceLocation" -> "Location")
    type_name
        .trim_end_matches("Location")
        .trim_end_matches("Metrics")
        .trim_end_matches("Data")
        .to_string()
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}
```

### 4. Code Generation

Generate complete struct definitions with documentation:

```rust
impl HiddenTypeExtractor {
    fn generate_struct_definition(
        &self,
        type_name: &str,
        fields: &[TypeField],
        methods: &[TypeMethod],
    ) -> String {
        let mut code = String::new();

        // Documentation
        code.push_str(&format!("/// Extracted type representing {}\n", type_name));
        code.push_str("///\n");
        code.push_str("/// This type was identified from repeated parameter patterns.\n");
        code.push_str("/// Encapsulating these fields improves cohesion and testability.\n");

        // Struct definition
        code.push_str("#[derive(Debug, Clone)]\n");
        code.push_str(&format!("pub struct {} {{\n", type_name));

        for field in fields {
            code.push_str(&format!(
                "    pub {}: {},\n",
                field.name,
                field.type_info.name
            ));
        }

        code.push_str("}\n\n");

        // Implementation block
        code.push_str(&format!("impl {} {{\n", type_name));

        // Constructor
        code.push_str("    /// Create a new instance\n");
        code.push_str("    pub fn new(");
        let params: Vec<_> = fields.iter()
            .map(|f| format!("{}: {}", f.name, f.type_info.name))
            .collect();
        code.push_str(&params.join(", "));
        code.push_str(") -> Self {\n");
        code.push_str("        Self {\n");
        for field in fields {
            code.push_str(&format!("            {},\n", field.name));
        }
        code.push_str("        }\n");
        code.push_str("    }\n");

        // Methods
        for method in methods {
            code.push_str(&format!("\n    /// {}\n", method.name));
            code.push_str(&format!("    {} {{\n", method.signature));
            code.push_str("        todo!(\"Migrate logic from original method\")\n");
            code.push_str("    }\n");
        }

        code.push_str("}\n");

        code
    }
}
```

## Enhanced Output Format

```
#4 SCORE: 62.0 [CRITICAL] god_object_analysis.rs (27 methods, 15 structs)

Hidden Type Extraction:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

3 Hidden Types Discovered:

[HIGH CONFIDENCE: 0.95] StructAnalysisContext
  Rationale: Parameter clump (4 parameters) appears in 8 methods.
             Encapsulating these parameters in a struct improves cohesion and reduces coupling.

  Current Usage (Parameter Clump):
    ✗ analyze_struct(data: &StructData, metrics: &Metrics, config: &Config, verbosity: Verbosity)
    ✗ validate_struct(data: &StructData, metrics: &Metrics, config: &Config, verbosity: Verbosity)
    ✗ format_struct(data: &StructData, metrics: &Metrics, config: &Config, verbosity: Verbosity)
    ... 5 more methods

  Suggested Definition:

    /// Extracted type representing StructAnalysisContext
    ///
    /// This type was identified from repeated parameter patterns.
    /// Encapsulating these fields improves cohesion and testability.
    #[derive(Debug, Clone)]
    pub struct StructAnalysisContext {
        pub data: StructData,
        pub metrics: Metrics,
        pub config: Config,
        pub verbosity: Verbosity,
    }

    impl StructAnalysisContext {
        /// Create a new instance
        pub fn new(data: StructData, metrics: Metrics, config: Config, verbosity: Verbosity) -> Self {
            Self { data, metrics, config, verbosity }
        }

        /// Analyze the struct in this context
        pub fn analyze(&self) -> AnalysisResult {
            todo!("Migrate logic from analyze_struct")
        }

        /// Validate the struct
        pub fn validate(&self) -> Result<()> {
            todo!("Migrate logic from validate_struct")
        }

        /// Format for display
        pub fn format(&self) -> String {
            todo!("Migrate logic from format_struct")
        }
    }

  Migration Path:
    1. Create StructAnalysisContext in new file: struct_analysis_context.rs
    2. Update method signatures to accept &StructAnalysisContext
    3. Move logic from static functions to impl methods
    4. Remove individual parameters from call sites

[MEDIUM CONFIDENCE: 0.75] DiversityAnalysisResult
  Rationale: Method 'analyze_diversity' returns unnamed tuple (f64, Vec<String>, DomainDiversity).
             Named struct provides better documentation and type safety.

  Current Usage (Tuple Return):
    ✗ fn analyze_diversity(...) -> (f64, Vec<String>, DomainDiversity)

  Suggested Definition:

    /// Result of domain diversity analysis
    #[derive(Debug, Clone)]
    pub struct DiversityAnalysisResult {
        pub diversity_score: f64,
        pub domains: Vec<String>,
        pub diversity_metrics: DomainDiversity,
    }

    impl DiversityAnalysisResult {
        /// Create a new instance
        pub fn new(diversity_score: f64, domains: Vec<String>, diversity_metrics: DomainDiversity) -> Self {
            Self { diversity_score, domains, diversity_metrics }
        }

        /// Check if diversity indicates god object
        pub fn is_god_object(&self, threshold: f64) -> bool {
            self.diversity_score > threshold
        }
    }

  Migration Path:
    1. Create DiversityAnalysisResult in diversity.rs
    2. Update analyze_diversity to return DiversityAnalysisResult
    3. Update call sites to use named fields instead of tuple destructuring

[MEDIUM CONFIDENCE: 0.70] PriorityItem
  Rationale: Parameter clump (3 parameters) appears in 5 methods.
             Encapsulating these parameters in a struct improves cohesion and reduces coupling.

  Current Usage (Parameter Clump):
    ✗ format_header(score: f64, location: SourceLocation, metrics: &Metrics)
    ✗ render_section(score: f64, location: SourceLocation, metrics: &Metrics)
    ✗ validate_item(score: f64, location: SourceLocation, metrics: &Metrics)
    ... 2 more methods

  Suggested Definition:

    /// Priority item for formatting and display
    #[derive(Debug, Clone)]
    pub struct PriorityItem {
        pub score: f64,
        pub location: SourceLocation,
        pub metrics: Metrics,
    }

    impl PriorityItem {
        pub fn new(score: f64, location: SourceLocation, metrics: Metrics) -> Self {
            Self { score, location, metrics }
        }

        pub fn format_header(&self) -> String {
            todo!("Migrate logic from format_header")
        }

        pub fn render_section(&self) -> String {
            todo!("Migrate logic from render_section")
        }
    }

    impl fmt::Display for PriorityItem {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.format_header())
        }
    }

Refactoring Priority:
  1. ✅ StructAnalysisContext (High confidence, 8 methods affected)
  2. ✅ DiversityAnalysisResult (Improves type safety, single method)
  3. ⚠ PriorityItem (Medium confidence, 5 methods affected)
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_parameter_clumps() {
        let signatures = vec![
            MethodSignature {
                name: "foo".to_string(),
                param_types: vec![
                    TypeInfo { name: "String".to_string(), .. },
                    TypeInfo { name: "usize".to_string(), .. },
                ],
                return_type: None,
                self_type: None,
            },
            MethodSignature {
                name: "bar".to_string(),
                param_types: vec![
                    TypeInfo { name: "String".to_string(), .. },
                    TypeInfo { name: "usize".to_string(), .. },
                ],
                return_type: None,
                self_type: None,
            },
            MethodSignature {
                name: "baz".to_string(),
                param_types: vec![
                    TypeInfo { name: "String".to_string(), .. },
                    TypeInfo { name: "usize".to_string(), .. },
                ],
                return_type: None,
                self_type: None,
            },
        ];

        let extractor = HiddenTypeExtractor;
        let clumps = extractor.find_parameter_clumps(&signatures, 3);

        assert_eq!(clumps.len(), 1);
        assert_eq!(clumps[0].methods.len(), 3);
        assert_eq!(clumps[0].params.len(), 2);
    }

    #[test]
    fn test_find_tuple_returns() {
        let signatures = vec![
            MethodSignature {
                name: "analyze".to_string(),
                param_types: vec![],
                return_type: Some(TypeInfo {
                    name: "(f64, Vec<String>)".to_string(),
                    is_reference: false,
                    is_mutable: false,
                    generics: vec![],
                }),
                self_type: None,
            },
        ];

        let extractor = HiddenTypeExtractor;
        let tuples = extractor.find_tuple_returns(&signatures);

        assert_eq!(tuples.len(), 1);
        assert_eq!(tuples[0].components.len(), 2);
    }

    #[test]
    fn test_hidden_type_confidence() {
        let extractor = HiddenTypeExtractor;

        // High occurrences + many params = high confidence
        let conf1 = extractor.calculate_confidence(10, 5);
        assert!(conf1 > 0.9);

        // Low occurrences + few params = low confidence
        let conf2 = extractor.calculate_confidence(2, 2);
        assert!(conf2 < 0.5);
    }
}
```

### Integration Tests

```rust
// tests/hidden_type_extraction_integration.rs

#[test]
fn test_extract_priority_item_from_formatter() {
    let code = r#"
        impl Formatter {
            fn format_header(score: f64, location: SourceLocation, metrics: &Metrics) -> String {
                todo!()
            }

            fn render_section(score: f64, location: SourceLocation, metrics: &Metrics) -> String {
                todo!()
            }

            fn validate_item(score: f64, location: SourceLocation, metrics: &Metrics) -> Result<()> {
                todo!()
            }
        }
    "#;

    let ast = syn::parse_file(code).unwrap();
    let extractor = HiddenTypeExtractor;
    let signatures = extract_method_signatures(&ast);

    let hidden_types = extractor.extract_hidden_types(&signatures, "formatter");

    assert_eq!(hidden_types.len(), 1);
    assert_eq!(hidden_types[0].fields.len(), 3);
    assert_eq!(hidden_types[0].occurrences, 3);
    assert!(hidden_types[0].confidence > 0.6);
}
```

## Dependencies

- **Spec 181**: Type signature extraction for parameter analysis
- **Spec 183**: Anti-pattern detection for parameter passing
- Rust `syn` crate for AST parsing

## Migration Path

1. **Phase 1**: Implement parameter clump detection
2. **Phase 2**: Add tuple return detection
3. **Phase 3**: Implement type synthesis and naming
4. **Phase 4**: Add code generation for struct definitions
5. **Phase 5**: Integrate with god object detector output
6. **Phase 6**: Validate on debtmap's own codebase

## Success Criteria

- Detects parameter clumps appearing 3+ times with 95% accuracy
- Identifies all tuple returns
- Generates valid, compilable struct definitions
- Suggests meaningful type and field names
- Provides migration path for each hidden type
- Confidence scores accurately reflect type validity
- Output includes before/after code examples
