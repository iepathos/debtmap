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

pub struct HiddenTypeExtractor {
    config: HiddenTypeConfig,
}

/// Configuration for hidden type extraction
#[derive(Clone, Debug)]
pub struct HiddenTypeConfig {
    /// Minimum occurrences to suggest a type (default: 3)
    pub min_occurrences: usize,

    /// Weight for occurrences in confidence formula (default: 10.0)
    pub occurrence_weight: f64,

    /// Weight for parameter count in confidence formula (default: 5.0)
    pub param_count_weight: f64,
}

impl Default for HiddenTypeConfig {
    fn default() -> Self {
        Self {
            min_occurrences: 3,
            occurrence_weight: 10.0,
            param_count_weight: 5.0,
        }
    }
}

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
    /// Create a new extractor with default configuration
    pub fn new() -> Self {
        Self {
            config: HiddenTypeConfig::default(),
        }
    }

    /// Create a new extractor with custom configuration
    pub fn with_config(config: HiddenTypeConfig) -> Self {
        Self { config }
    }

    /// Find parameter clumps that appear in multiple functions
    ///
    /// Uses **fuzzy matching** to handle type variations:
    /// - `String` matches `&str`
    /// - `&T` matches `T`
    /// - `Option<T>` matches `T`
    /// - `&mut T` matches `&T`
    pub fn find_parameter_clumps(
        &self,
        signatures: &[MethodSignature],
        min_occurrences: usize,
    ) -> Vec<ParameterClump> {
        let mut clump_groups: Vec<(NormalizedParamSignature, Vec<String>)> = Vec::new();

        // Group methods by normalized parameter signature
        for sig in signatures {
            let normalized_sig = self.create_normalized_signature(&sig.param_types);

            // Find existing group with fuzzy match
            let mut found = false;
            for (existing_sig, methods) in &mut clump_groups {
                if self.signatures_match_fuzzy(existing_sig, &normalized_sig) {
                    methods.push(sig.name.clone());
                    found = true;
                    break;
                }
            }

            if !found {
                clump_groups.push((normalized_sig, vec![sig.name.clone()]));
            }
        }

        // Filter for clumps that appear min_occurrences times
        clump_groups.into_iter()
            .filter(|(_, methods)| methods.len() >= min_occurrences)
            .map(|(normalized_sig, methods)| ParameterClump {
                params: normalized_sig.types,
                methods,
            })
            .collect()
    }

    fn create_normalized_signature(&self, params: &[TypeInfo]) -> NormalizedParamSignature {
        NormalizedParamSignature {
            types: params.iter()
                .map(|t| self.normalize_type_for_matching(t))
                .collect(),
        }
    }

    /// Normalize type for fuzzy matching
    ///
    /// Transformations:
    /// - `&str` → `String`
    /// - `&T` → `T`
    /// - `&mut T` → `T`
    /// - `Option<T>` → `T`
    /// - Remove generic wrappers
    fn normalize_type_for_matching(&self, type_info: &TypeInfo) -> TypeInfo {
        let mut normalized = type_info.clone();

        // Remove reference/mutability
        if normalized.is_reference {
            normalized.is_reference = false;
            normalized.is_mutable = false;
        }

        // Normalize String/str
        if normalized.name == "str" {
            normalized.name = "String".to_string();
        }

        // Remove Option wrapper
        if normalized.name.starts_with("Option<") {
            if let Some(inner) = self.extract_generic_inner(&normalized.name) {
                normalized.name = inner;
            }
        }

        // Extract from Vec, Box, etc. for comparison
        if matches!(normalized.name.as_str(), s if s.starts_with("Vec<") || s.starts_with("Box<")) {
            if let Some(inner) = self.extract_generic_inner(&normalized.name) {
                // Keep the wrapper but store the inner type for comparison
                normalized.generics = vec![inner];
            }
        }

        normalized
    }

    fn extract_generic_inner(&self, type_name: &str) -> Option<String> {
        if let Some(start) = type_name.find('<') {
            if let Some(end) = type_name.rfind('>') {
                return Some(type_name[start + 1..end].trim().to_string());
            }
        }
        None
    }

    /// Check if two normalized signatures match with fuzzy rules
    fn signatures_match_fuzzy(
        &self,
        sig1: &NormalizedParamSignature,
        sig2: &NormalizedParamSignature,
    ) -> bool {
        if sig1.types.len() != sig2.types.len() {
            return false;
        }

        sig1.types.iter()
            .zip(&sig2.types)
            .all(|(t1, t2)| self.types_fuzzy_match(t1, t2))
    }

    fn types_fuzzy_match(&self, t1: &TypeInfo, t2: &TypeInfo) -> bool {
        // After normalization, should be exact match
        t1.name == t2.name
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

#[derive(Clone, Debug)]
struct NormalizedParamSignature {
    types: Vec<TypeInfo>,
}

// Note: TypeInfo Hash/Eq/PartialEq are derived in Spec 181
// This ensures all fields (including generics) are included in equality/hashing
```

### 2. Tuple Return Detection

Identify tuple returns that should be named structs:

```rust
impl HiddenTypeExtractor {
    /// Detect tuple returns that should be structs
    ///
    /// Analyzes return types from method signatures to find tuples.
    /// Uses syn AST instead of string parsing for correctness.
    pub fn find_tuple_returns(
        &self,
        ast: &syn::File,
    ) -> Vec<TupleReturn> {
        let mut tuple_returns = Vec::new();

        // Extract from impl blocks
        for item in &ast.items {
            if let syn::Item::Impl(impl_block) = item {
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        if let syn::ReturnType::Type(_, ty) = &method.sig.output {
                            if let syn::Type::Tuple(tuple_type) = ty.as_ref() {
                                // Found a tuple return
                                let components = tuple_type.elems.iter()
                                    .map(|elem| self.type_to_string(elem))
                                    .collect();

                                tuple_returns.push(TupleReturn {
                                    method_name: method.sig.ident.to_string(),
                                    tuple_type: format!("({})",
                                        tuple_type.elems.iter()
                                            .map(|t| self.type_to_string(t))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    ),
                                    components,
                                });
                            }
                        }
                    }
                }
            }
        }

        tuple_returns
    }

    /// Convert syn::Type to string representation
    fn type_to_string(&self, ty: &syn::Type) -> String {
        match ty {
            syn::Type::Path(type_path) => {
                quote::quote!(#type_path).to_string()
            }
            syn::Type::Reference(type_ref) => {
                let mutability = if type_ref.mutability.is_some() { "mut " } else { "" };
                format!("&{}{}", mutability, self.type_to_string(&type_ref.elem))
            }
            syn::Type::Tuple(tuple) => {
                format!("({})",
                    tuple.elems.iter()
                        .map(|t| self.type_to_string(t))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            _ => quote::quote!(#ty).to_string(),
        }
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
        ast: &syn::File,
        module_name: &str,
    ) -> Vec<HiddenType> {
        let mut hidden_types = Vec::new();

        // 1. Parameter clumps (configurable minimum occurrences)
        let clumps = self.find_parameter_clumps(signatures, self.config.min_occurrences);
        for clump in clumps {
            hidden_types.push(self.synthesize_type_from_clump(&clump, module_name));
        }

        // 2. Tuple returns (using syn AST for accurate parsing)
        let tuples = self.find_tuple_returns(ast);
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

    /// Suggest type name from parameter types
    ///
    /// Uses semantic analysis of type names to infer domain:
    /// 1. Extract nouns from parameter types
    /// 2. Identify dominant domain (metrics, config, data, etc.)
    /// 3. Generate meaningful name based on domain
    fn suggest_type_name_from_params(&self, params: &[TypeInfo], module_name: &str) -> String {
        // Extract nouns from type names
        let nouns: Vec<_> = params.iter()
            .map(|p| extract_noun(&p.name))
            .collect();

        // Identify domain
        let domain = self.identify_domain(&nouns, module_name);

        // Generate name based on domain and structure
        match domain.as_str() {
            "metrics" if nouns.iter().any(|n| n.contains("Location")) => {
                "MetricsContext".to_string()
            }
            "config" | "settings" => {
                format!("{}Config", to_pascal_case(module_name))
            }
            "analysis" | "analyzer" => {
                "AnalysisContext".to_string()
            }
            _ => {
                // Combine most meaningful nouns
                if nouns.len() == 2 {
                    format!("{}{}", nouns[0], nouns[1])
                } else if nouns.len() > 2 {
                    // Use first and last
                    format!("{}{}", nouns[0], nouns[nouns.len() - 1])
                } else if !nouns.is_empty() {
                    format!("{}Context", nouns[0])
                } else {
                    format!("{}Context", to_pascal_case(module_name))
                }
            }
        }
    }

    /// Identify domain from noun collection
    fn identify_domain(&self, nouns: &[String], module_name: &str) -> String {
        // Check for domain-specific terms
        for noun in nouns {
            let lower = noun.to_lowercase();
            if matches!(lower.as_str(), "metrics" | "metric") {
                return "metrics".to_string();
            }
            if matches!(lower.as_str(), "config" | "configuration" | "settings") {
                return "config".to_string();
            }
            if matches!(lower.as_str(), "analysis" | "analyzer") {
                return "analysis".to_string();
            }
        }

        // Fallback to module name
        module_name.to_lowercase()
    }

    /// Suggest type name from method name
    ///
    /// Uses method verb and context to infer appropriate type suffix
    fn suggest_type_name_from_method(&self, method_name: &str, module_name: &str) -> String {
        // Common verb → suffix mappings
        let (verb, remainder) = self.split_method_verb(method_name);

        match verb {
            "analyze" => format!("{}Result", to_pascal_case(&remainder)),
            "calculate" | "compute" => format!("{}Metrics", to_pascal_case(&remainder)),
            "validate" | "check" => format!("{}Validation", to_pascal_case(&remainder)),
            "parse" => format!("{}Data", to_pascal_case(&remainder)),
            "format" | "render" => format!("{}Output", to_pascal_case(&remainder)),
            "build" | "create" => format!("{}Builder", to_pascal_case(&remainder)),
            _ => {
                // Fallback: use method name or module context
                if !remainder.is_empty() {
                    format!("{}Result", to_pascal_case(&remainder))
                } else {
                    format!("{}Result", to_pascal_case(module_name))
                }
            }
        }
    }

    fn split_method_verb(&self, method_name: &str) -> (String, String) {
        // Split on first underscore
        if let Some(pos) = method_name.find('_') {
            let verb = method_name[..pos].to_string();
            let remainder = method_name[pos + 1..].to_string();
            (verb, remainder)
        } else {
            (method_name.to_string(), String::new())
        }
    }

    fn suggest_field_name(&self, type_info: &TypeInfo, index: usize) -> String {
        let type_name = &type_info.name;

        // Common type mappings
        let field_name = match type_name.as_str() {
            // Exact matches
            "SourceLocation" | "Location" | "PathBuf" | "Path" => "location",
            "FileMetrics" | "Metrics" => "metrics",
            "Config" | "Configuration" => "config",
            "Verbosity" => "verbosity",
            _ => {
                // Heuristic matching
                let lower = type_name.to_lowercase();
                if lower.contains("location") || lower.contains("path") {
                    "location"
                } else if lower.contains("metric") {
                    "metrics"
                } else if lower.contains("score") || lower.contains("rating") {
                    "score"
                } else if lower.contains("config") || lower.contains("setting") {
                    "config"
                } else if lower.contains("data") {
                    "data"
                } else if lower.contains("result") {
                    "result"
                } else if lower.contains("context") {
                    "context"
                } else {
                    // Last resort: convert type name to snake_case
                    &self.to_snake_case(type_name)
                }
            }
        };

        field_name.to_string()
    }

    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        }
        result
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

    /// Calculate confidence score for hidden type suggestion
    ///
    /// Confidence Formula:
    /// - occurrence_factor = min(occurrences / occurrence_weight, 1.0)
    /// - param_factor = min(param_count / param_count_weight, 1.0)
    /// - confidence = (occurrence_factor + param_factor) / 2.0
    ///
    /// Score interpretation:
    /// - 0.9-1.0: Very High (10+ occurrences, 5+ params)
    /// - 0.7-0.89: High (7+ occurrences, 3+ params)
    /// - 0.5-0.69: Medium (5+ occurrences, 2+ params)
    /// - 0.0-0.49: Low (< 5 occurrences or 1 param)
    fn calculate_confidence(&self, occurrences: usize, param_count: usize) -> f64 {
        let occurrence_factor = (occurrences as f64 / self.config.occurrence_weight).min(1.0);
        let param_factor = (param_count as f64 / self.config.param_count_weight).min(1.0);

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

/// Extract core noun from compound type names
///
/// Examples:
/// - "SourceLocation" -> "Location"
/// - "FileMetrics" -> "Metrics"
/// - "HttpRequestHandler" -> "HttpRequest"
/// - "UserData" -> "User"
fn extract_noun(type_name: &str) -> String {
    // Common suffixes to remove
    const SUFFIXES: &[&str] = &[
        "Location", "Metrics", "Data", "Info", "Details",
        "Handler", "Manager", "Service", "Provider", "Factory",
        "Builder", "Analyzer", "Processor", "Controller"
    ];

    for suffix in SUFFIXES {
        if type_name.ends_with(suffix) && type_name.len() > suffix.len() {
            return type_name[..type_name.len() - suffix.len()].to_string();
        }
    }

    // No suffix found, return as-is
    type_name.to_string()
}

// Note: to_pascal_case is shared with Spec 183 (see Shared Utilities section)
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
- **Spec 183**: Anti-pattern detection for parameter passing and shared utilities
- Rust `syn` crate for AST parsing
- Rust `quote` crate for type-to-string conversion
- Rust `serde` crate for serialization

### External Crate Additions

```toml
[dependencies]
syn = { version = "2.0", features = ["full", "parsing", "visit"] }
quote = "1.0"
serde = { version = "1.0", features = ["derive"] }
```

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
