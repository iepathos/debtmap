---
number: 30
title: Code Organization Anti-Patterns Detection
category: feature
priority: medium
status: draft
dependencies: []
created: 2025-08-16
---

# Specification 30: Code Organization Anti-Patterns Detection

**Category**: feature
**Priority**: medium
**Status**: draft
**Dependencies**: []

## Context

Poor code organization leads to maintainability issues, reduced readability, and increased development complexity. While the current debtmap system identifies complexity metrics, it lacks detection for structural and organizational anti-patterns that significantly impact code quality:

- **God Objects/Classes** - Single types with excessive responsibilities and methods
- **Magic Numbers and Strings** - Hardcoded values without clear meaning or context
- **Long Parameter Lists** - Functions with too many parameters indicating design issues
- **Feature Envy** - Methods that use more functionality from other classes than their own
- **Data Clumps** - Groups of parameters that always appear together
- **Primitive Obsession** - Overuse of primitive types instead of domain-specific types

These organizational issues represent technical debt that accumulates over time and makes code increasingly difficult to maintain and extend.

## Objective

Implement code organization analysis that identifies structural anti-patterns affecting maintainability by:

1. **God Object Detection**: Identify types with excessive methods, fields, or responsibilities
2. **Magic Value Detection**: Find hardcoded numbers and strings that should be constants
3. **Parameter Analysis**: Detect long parameter lists and data clumps
4. **Coupling Analysis**: Identify feature envy and inappropriate dependencies
5. **Type Usage Analysis**: Detect primitive obsession and missing domain types

## Requirements

### Functional Requirements

1. **God Object Detection**
   - Count methods and fields per type (struct, enum, impl block)
   - Analyze method complexity distribution within types
   - Detect types that implement too many traits
   - Identify single responsibility principle violations

2. **Magic Value Detection**
   - Find numeric literals (except 0, 1, -1) without named constants
   - Detect string literals that appear multiple times
   - Identify configuration values hardcoded in business logic
   - Flag magic values in comparison operations and array indexing

3. **Parameter List Analysis**
   - Count function parameters and flag excessive counts
   - Detect parameter groups that frequently appear together (data clumps)
   - Identify boolean parameters that indicate missing abstraction
   - Analyze parameter type patterns for improvement opportunities

4. **Feature Envy Detection**
   - Analyze method calls to identify external dependencies
   - Count method calls on parameters vs. self
   - Detect methods that primarily manipulate external data
   - Identify methods that belong in different modules

5. **Primitive Obsession Detection**
   - Find repeated use of basic types for domain concepts
   - Detect string-based identifiers that should be typed
   - Identify numeric types used for measurements without units
   - Find boolean flags that could be enums

### Non-Functional Requirements

1. **Performance**
   - Organization analysis adds <10% overhead to total analysis time
   - Efficient AST traversal and pattern counting
   - Scalable threshold-based detection

2. **Accuracy**
   - >80% precision for organizational anti-pattern detection
   - Configurable thresholds to reduce false positives
   - Context-aware analysis considering domain requirements

3. **Actionability**
   - Specific refactoring suggestions for each anti-pattern
   - Prioritization based on impact and effort
   - Integration with existing complexity scoring

## Acceptance Criteria

- [ ] **God Object Detection**: Types with excessive methods/fields identified with refactoring suggestions
- [ ] **Magic Value Detection**: Hardcoded literals flagged with constant extraction recommendations
- [ ] **Parameter Analysis**: Long parameter lists and data clumps detected with structuring suggestions
- [ ] **Feature Envy Detection**: Methods accessing external data more than internal data flagged
- [ ] **Primitive Obsession**: Overused primitive types identified with domain type suggestions
- [ ] **Configurable Thresholds**: All detection thresholds configurable per project needs
- [ ] **Impact Assessment**: Each anti-pattern includes maintainability impact estimate
- [ ] **Refactoring Guidance**: Specific suggestions for resolving each detected anti-pattern

## Technical Details

### Implementation Approach

#### 1. Code Organization Analysis Framework (`src/organization/`)

```rust
/// Code organization anti-pattern detection framework
pub mod organization {
    use crate::core::ast::AstNode;
    use crate::core::{DebtItem, Priority};
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum OrganizationAntiPattern {
        GodObject {
            type_name: String,
            method_count: usize,
            field_count: usize,
            responsibility_count: usize,
            suggested_split: Vec<ResponsibilityGroup>,
        },
        MagicValue {
            value_type: MagicValueType,
            value: String,
            occurrence_count: usize,
            suggested_constant_name: String,
            context: ValueContext,
        },
        LongParameterList {
            function_name: String,
            parameter_count: usize,
            data_clumps: Vec<ParameterGroup>,
            suggested_refactoring: ParameterRefactoring,
        },
        FeatureEnvy {
            method_name: String,
            envied_type: String,
            external_calls: usize,
            internal_calls: usize,
            suggested_move: bool,
        },
        PrimitiveObsession {
            primitive_type: String,
            usage_context: PrimitiveUsageContext,
            occurrence_count: usize,
            suggested_domain_type: String,
        },
        DataClump {
            parameter_group: ParameterGroup,
            occurrence_count: usize,
            suggested_struct_name: String,
        },
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum MagicValueType {
        NumericLiteral,
        StringLiteral,
        ArraySize,
        ConfigurationValue,
        BusinessRule,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ValueContext {
        Comparison,
        ArrayIndexing,
        Calculation,
        Timeout,
        BufferSize,
        BusinessLogic,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ParameterRefactoring {
        ExtractStruct,
        UseBuilder,
        SplitFunction,
        UseConfiguration,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum PrimitiveUsageContext {
        Identifier,
        Measurement,
        Status,
        Category,
        BusinessRule,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub struct ResponsibilityGroup {
        pub name: String,
        pub methods: Vec<String>,
        pub fields: Vec<String>,
        pub responsibility: String,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub struct ParameterGroup {
        pub parameters: Vec<Parameter>,
        pub group_name: String,
        pub semantic_relationship: String,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub struct Parameter {
        pub name: String,
        pub type_name: String,
        pub position: usize,
    }
    
    pub trait OrganizationDetector {
        fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<OrganizationAntiPattern>;
        fn detector_name(&self) -> &'static str;
        fn estimate_maintainability_impact(&self, pattern: &OrganizationAntiPattern) -> MaintainabilityImpact;
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum MaintainabilityImpact {
        Critical,  // Severely impacts maintainability
        High,      // Significantly impacts maintainability
        Medium,    // Moderately impacts maintainability
        Low,       // Minor impact on maintainability
    }
}
```

#### 2. God Object Detector (`src/organization/god_object_detector.rs`)

```rust
pub struct GodObjectDetector {
    max_methods: usize,
    max_fields: usize,
    max_responsibilities: usize,
    responsibility_analyzer: ResponsibilityAnalyzer,
}

impl OrganizationDetector for GodObjectDetector {
    fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        let type_definitions = self.find_type_definitions(ast);
        
        for type_def in type_definitions {
            let analysis = self.analyze_type(&type_def);
            
            if self.is_god_object(&analysis) {
                let suggested_split = self.suggest_responsibility_split(&analysis);
                
                patterns.push(OrganizationAntiPattern::GodObject {
                    type_name: analysis.name.clone(),
                    method_count: analysis.method_count,
                    field_count: analysis.field_count,
                    responsibility_count: analysis.responsibilities.len(),
                    suggested_split,
                });
            }
        }
        
        patterns
    }
}

impl GodObjectDetector {
    fn analyze_type(&self, type_def: &TypeDefinition) -> TypeAnalysis {
        let methods = self.extract_methods(type_def);
        let fields = self.extract_fields(type_def);
        let responsibilities = self.responsibility_analyzer.analyze_responsibilities(&methods, &fields);
        
        TypeAnalysis {
            name: type_def.name.clone(),
            method_count: methods.len(),
            field_count: fields.len(),
            methods,
            fields,
            responsibilities,
            trait_implementations: self.count_trait_implementations(type_def),
        }
    }
    
    fn is_god_object(&self, analysis: &TypeAnalysis) -> bool {
        analysis.method_count > self.max_methods ||
        analysis.field_count > self.max_fields ||
        analysis.responsibilities.len() > self.max_responsibilities ||
        analysis.trait_implementations > 10 // Implementing too many traits
    }
    
    fn suggest_responsibility_split(&self, analysis: &TypeAnalysis) -> Vec<ResponsibilityGroup> {
        self.responsibility_analyzer.group_by_cohesion(&analysis.methods, &analysis.fields)
    }
    
    fn extract_methods(&self, type_def: &TypeDefinition) -> Vec<MethodInfo> {
        let mut methods = Vec::new();
        
        for impl_block in &type_def.impl_blocks {
            for method in &impl_block.methods {
                methods.push(MethodInfo {
                    name: method.name.clone(),
                    visibility: method.visibility.clone(),
                    parameter_count: method.parameters.len(),
                    complexity: method.complexity,
                    return_type: method.return_type.clone(),
                    external_calls: self.count_external_calls(method),
                    field_accesses: self.count_field_accesses(method, &type_def.fields),
                });
            }
        }
        
        methods
    }
    
    fn count_external_calls(&self, method: &MethodDefinition) -> usize {
        // Count calls to other types/modules
        method.body.traverse_calls()
            .filter(|call| !self.is_self_call(call) && !self.is_standard_library_call(call))
            .count()
    }
    
    fn count_field_accesses(&self, method: &MethodDefinition, fields: &[FieldInfo]) -> usize {
        let field_names: HashSet<_> = fields.iter().map(|f| &f.name).collect();
        
        method.body.traverse_field_accesses()
            .filter(|access| field_names.contains(&access.field_name))
            .count()
    }
}

pub struct ResponsibilityAnalyzer;

impl ResponsibilityAnalyzer {
    fn analyze_responsibilities(&self, methods: &[MethodInfo], fields: &[FieldInfo]) -> Vec<Responsibility> {
        let mut responsibilities = Vec::new();
        
        // Group methods by naming patterns and functionality
        let method_groups = self.group_methods_by_prefix(methods);
        
        for (prefix, group_methods) in method_groups {
            let related_fields = self.find_related_fields(&group_methods, fields);
            
            responsibilities.push(Responsibility {
                name: self.infer_responsibility_name(&prefix, &group_methods),
                methods: group_methods.into_iter().map(|m| m.name.clone()).collect(),
                fields: related_fields.into_iter().map(|f| f.name.clone()).collect(),
                cohesion_score: self.calculate_cohesion(&group_methods, &related_fields),
            });
        }
        
        responsibilities
    }
    
    fn group_methods_by_prefix(&self, methods: &[MethodInfo]) -> HashMap<String, Vec<&MethodInfo>> {
        let mut groups: HashMap<String, Vec<&MethodInfo>> = HashMap::new();
        
        for method in methods {
            let prefix = self.extract_method_prefix(&method.name);
            groups.entry(prefix).or_default().push(method);
        }
        
        groups
    }
    
    fn extract_method_prefix(&self, method_name: &str) -> String {
        // Extract semantic prefixes like "get_", "set_", "calculate_", "validate_", etc.
        const COMMON_PREFIXES: &[&str] = &[
            "get", "set", "is", "has", "can", "should", "will",
            "create", "build", "make", "new", "init",
            "calculate", "compute", "process", "transform",
            "validate", "check", "verify", "ensure",
            "save", "load", "store", "retrieve", "fetch",
            "update", "modify", "change", "edit",
            "delete", "remove", "clear", "reset",
            "send", "receive", "handle", "manage",
        ];
        
        let lower_name = method_name.to_lowercase();
        
        for prefix in COMMON_PREFIXES {
            if lower_name.starts_with(prefix) {
                return prefix.to_string();
            }
        }
        
        // If no common prefix found, use the first word
        method_name.split('_').next().unwrap_or(method_name).to_string()
    }
    
    fn infer_responsibility_name(&self, prefix: &str, methods: &[&MethodInfo]) -> String {
        match prefix {
            "get" | "set" => "Data Access".to_string(),
            "calculate" | "compute" => "Computation".to_string(),
            "validate" | "check" => "Validation".to_string(),
            "save" | "load" | "store" => "Persistence".to_string(),
            "create" | "build" | "new" => "Construction".to_string(),
            "send" | "receive" | "handle" => "Communication".to_string(),
            _ => format!("{} Operations", prefix.to_title_case()),
        }
    }
    
    fn group_by_cohesion(&self, methods: &[MethodInfo], fields: &[FieldInfo]) -> Vec<ResponsibilityGroup> {
        let responsibilities = self.analyze_responsibilities(methods, fields);
        
        responsibilities.into_iter()
            .map(|resp| ResponsibilityGroup {
                name: format!("{}Manager", resp.name.replace(" ", "")),
                methods: resp.methods,
                fields: resp.fields,
                responsibility: resp.name,
            })
            .collect()
    }
}
```

#### 3. Magic Value Detector (`src/organization/magic_value_detector.rs`)

```rust
pub struct MagicValueDetector {
    ignore_common_values: bool,
    min_occurrence_threshold: usize,
    context_analyzer: ValueContextAnalyzer,
}

impl OrganizationDetector for MagicValueDetector {
    fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        
        // Find numeric literals
        let numeric_literals = self.find_numeric_literals(ast);
        patterns.extend(self.analyze_numeric_literals(numeric_literals));
        
        // Find string literals
        let string_literals = self.find_string_literals(ast);
        patterns.extend(self.analyze_string_literals(string_literals));
        
        patterns
    }
}

impl MagicValueDetector {
    fn find_numeric_literals(&self, ast: &AstNode) -> Vec<NumericLiteral> {
        let mut literals = Vec::new();
        
        ast.traverse_depth_first(|node| {
            if let AstNode::NumericLiteral(literal) = node {
                if !self.should_ignore_numeric_value(&literal.value) {
                    literals.push(literal.clone());
                }
            }
        });
        
        literals
    }
    
    fn should_ignore_numeric_value(&self, value: &str) -> bool {
        if !self.ignore_common_values {
            return false;
        }
        
        // Common values that are typically not magic numbers
        const COMMON_VALUES: &[&str] = &[
            "0", "1", "-1", "2", "10", "100", "1000",
            "0.0", "1.0", "-1.0", "0.5", "2.0"
        ];
        
        COMMON_VALUES.contains(&value)
    }
    
    fn analyze_numeric_literals(&self, literals: Vec<NumericLiteral>) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        let literal_counts = self.count_literal_occurrences(&literals);
        
        for (value, occurrences) in literal_counts {
            if occurrences.len() >= self.min_occurrence_threshold {
                let context = self.context_analyzer.analyze_numeric_context(&occurrences);
                let suggested_name = self.suggest_constant_name(&value, &context);
                
                patterns.push(OrganizationAntiPattern::MagicValue {
                    value_type: MagicValueType::NumericLiteral,
                    value: value.clone(),
                    occurrence_count: occurrences.len(),
                    suggested_constant_name: suggested_name,
                    context,
                });
            }
        }
        
        patterns
    }
    
    fn suggest_constant_name(&self, value: &str, context: &ValueContext) -> String {
        match context {
            ValueContext::Timeout => format!("TIMEOUT_{}_{}", 
                self.value_to_identifier(value), 
                self.infer_time_unit(value)
            ),
            ValueContext::BufferSize => format!("BUFFER_SIZE_{}", self.value_to_identifier(value)),
            ValueContext::ArrayIndexing => format!("INDEX_{}", self.value_to_identifier(value)),
            ValueContext::BusinessLogic => format!("BUSINESS_RULE_{}", self.value_to_identifier(value)),
            ValueContext::Calculation => format!("FACTOR_{}", self.value_to_identifier(value)),
            ValueContext::Comparison => format!("THRESHOLD_{}", self.value_to_identifier(value)),
        }
    }
    
    fn value_to_identifier(&self, value: &str) -> String {
        value.replace('.', "_DOT_")
             .replace('-', "NEG_")
             .to_uppercase()
    }
    
    fn infer_time_unit(&self, value: &str) -> String {
        let num: f64 = value.parse().unwrap_or(0.0);
        
        match num {
            n if n < 1.0 => "MILLIS",
            n if n < 1000.0 => "SECONDS", 
            n if n < 60000.0 => "MINUTES",
            _ => "HOURS",
        }.to_string()
    }
}

pub struct ValueContextAnalyzer;

impl ValueContextAnalyzer {
    fn analyze_numeric_context(&self, occurrences: &[LiteralOccurrence]) -> ValueContext {
        let contexts: Vec<_> = occurrences.iter()
            .map(|occ| self.classify_usage_context(occ))
            .collect();
        
        // Return the most common context
        self.most_frequent_context(contexts)
    }
    
    fn classify_usage_context(&self, occurrence: &LiteralOccurrence) -> ValueContext {
        match &occurrence.usage_pattern {
            UsagePattern::BinaryOperation { operator, .. } => {
                match operator.as_str() {
                    "==" | "!=" | "<" | ">" | "<=" | ">=" => ValueContext::Comparison,
                    "+" | "-" | "*" | "/" | "%" => ValueContext::Calculation,
                    _ => ValueContext::BusinessLogic,
                }
            }
            UsagePattern::ArrayAccess => ValueContext::ArrayIndexing,
            UsagePattern::FunctionArgument { function_name, .. } => {
                if self.is_timeout_function(function_name) {
                    ValueContext::Timeout
                } else if self.is_buffer_function(function_name) {
                    ValueContext::BufferSize
                } else {
                    ValueContext::BusinessLogic
                }
            }
            UsagePattern::Assignment => ValueContext::BusinessLogic,
        }
    }
    
    fn is_timeout_function(&self, function_name: &str) -> bool {
        const TIMEOUT_FUNCTIONS: &[&str] = &[
            "sleep", "timeout", "wait", "delay", "duration",
            "set_timeout", "with_timeout", "expires_in"
        ];
        
        let lower_name = function_name.to_lowercase();
        TIMEOUT_FUNCTIONS.iter().any(|tf| lower_name.contains(tf))
    }
    
    fn is_buffer_function(&self, function_name: &str) -> bool {
        const BUFFER_FUNCTIONS: &[&str] = &[
            "buffer", "capacity", "reserve", "with_capacity",
            "allocate", "new_with_capacity"
        ];
        
        let lower_name = function_name.to_lowercase();
        BUFFER_FUNCTIONS.iter().any(|bf| lower_name.contains(bf))
    }
}
```

#### 4. Parameter List Analyzer (`src/organization/parameter_analyzer.rs`)

```rust
pub struct ParameterAnalyzer {
    max_parameters: usize,
    data_clump_detector: DataClumpDetector,
}

impl OrganizationDetector for ParameterAnalyzer {
    fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        let functions = self.find_functions(ast);
        
        for function in functions {
            // Check for long parameter lists
            if function.parameters.len() > self.max_parameters {
                let data_clumps = self.data_clump_detector.find_data_clumps(&function.parameters);
                let refactoring = self.suggest_parameter_refactoring(&function, &data_clumps);
                
                patterns.push(OrganizationAntiPattern::LongParameterList {
                    function_name: function.name.clone(),
                    parameter_count: function.parameters.len(),
                    data_clumps,
                    suggested_refactoring: refactoring,
                });
            }
            
            // Check for data clumps even in shorter parameter lists
            let data_clumps = self.data_clump_detector.find_significant_clumps(&function.parameters);
            for clump in data_clumps {
                patterns.push(OrganizationAntiPattern::DataClump {
                    parameter_group: clump.clone(),
                    occurrence_count: self.count_clump_occurrences(&clump, &functions),
                    suggested_struct_name: self.suggest_struct_name(&clump),
                });
            }
        }
        
        patterns
    }
}

impl ParameterAnalyzer {
    fn suggest_parameter_refactoring(
        &self, 
        function: &FunctionInfo, 
        data_clumps: &[ParameterGroup]
    ) -> ParameterRefactoring {
        if !data_clumps.is_empty() {
            ParameterRefactoring::ExtractStruct
        } else if function.parameters.len() > 8 {
            ParameterRefactoring::UseBuilder
        } else if self.has_many_boolean_parameters(function) {
            ParameterRefactoring::UseConfiguration
        } else {
            ParameterRefactoring::SplitFunction
        }
    }
    
    fn has_many_boolean_parameters(&self, function: &FunctionInfo) -> bool {
        let bool_count = function.parameters.iter()
            .filter(|p| p.type_name == "bool")
            .count();
        
        bool_count > 2
    }
    
    fn count_clump_occurrences(&self, clump: &ParameterGroup, functions: &[FunctionInfo]) -> usize {
        functions.iter()
            .filter(|f| self.function_has_clump(f, clump))
            .count()
    }
    
    fn function_has_clump(&self, function: &FunctionInfo, clump: &ParameterGroup) -> bool {
        // Check if function contains the same parameter pattern
        self.data_clump_detector.has_matching_pattern(&function.parameters, &clump.parameters)
    }
    
    fn suggest_struct_name(&self, clump: &ParameterGroup) -> String {
        if !clump.group_name.is_empty() {
            format!("{}Config", clump.group_name.to_title_case())
        } else {
            // Infer name from parameter names
            let common_prefix = self.find_common_prefix(&clump.parameters);
            if !common_prefix.is_empty() {
                format!("{}Parameters", common_prefix.to_title_case())
            } else {
                "ConfigParameters".to_string()
            }
        }
    }
    
    fn find_common_prefix(&self, parameters: &[Parameter]) -> String {
        if parameters.is_empty() {
            return String::new();
        }
        
        let first_name = &parameters[0].name;
        let mut common_len = first_name.len();
        
        for param in parameters.iter().skip(1) {
            let current_common = first_name.chars()
                .zip(param.name.chars())
                .take_while(|(a, b)| a == b)
                .count();
            
            common_len = common_len.min(current_common);
        }
        
        if common_len > 2 {
            first_name[..common_len].trim_end_matches('_').to_string()
        } else {
            String::new()
        }
    }
}

pub struct DataClumpDetector;

impl DataClumpDetector {
    fn find_data_clumps(&self, parameters: &[Parameter]) -> Vec<ParameterGroup> {
        let mut clumps = Vec::new();
        
        // Group parameters by semantic similarity
        let groups = self.group_parameters_by_semantics(parameters);
        
        for (semantic_group, group_params) in groups {
            if group_params.len() >= 3 {
                clumps.push(ParameterGroup {
                    parameters: group_params,
                    group_name: semantic_group.clone(),
                    semantic_relationship: semantic_group,
                });
            }
        }
        
        clumps
    }
    
    fn group_parameters_by_semantics(&self, parameters: &[Parameter]) -> HashMap<String, Vec<Parameter>> {
        let mut groups: HashMap<String, Vec<Parameter>> = HashMap::new();
        
        for param in parameters {
            let semantic_group = self.identify_semantic_group(param);
            groups.entry(semantic_group).or_default().push(param.clone());
        }
        
        groups
    }
    
    fn identify_semantic_group(&self, parameter: &Parameter) -> String {
        let name = parameter.name.to_lowercase();
        
        // Define semantic patterns
        const SEMANTIC_PATTERNS: &[(&str, &[&str])] = &[
            ("coordinate", &["x", "y", "z", "width", "height", "depth"]),
            ("time", &["start", "end", "duration", "timeout", "delay"]),
            ("user", &["user", "username", "userid", "email", "name"]),
            ("config", &["config", "setting", "option", "preference"]),
            ("network", &["host", "port", "url", "endpoint", "address"]),
            ("file", &["path", "filename", "directory", "extension"]),
            ("authentication", &["token", "key", "secret", "auth", "credential"]),
            ("pagination", &["page", "limit", "offset", "size", "count"]),
        ];
        
        for (group_name, keywords) in SEMANTIC_PATTERNS {
            if keywords.iter().any(|keyword| name.contains(keyword)) {
                return group_name.to_string();
            }
        }
        
        // If no semantic pattern matches, group by type
        format!("type_{}", parameter.type_name)
    }
}
```

#### 5. Integration with Main Analysis Pipeline

```rust
// In src/analyzers/rust.rs
use crate::organization::{
    OrganizationDetector, GodObjectDetector, MagicValueDetector,
    ParameterAnalyzer, FeatureEnvyDetector, PrimitiveObsessionDetector
};

fn analyze_organization_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    let detectors: Vec<Box<dyn OrganizationDetector>> = vec![
        Box::new(GodObjectDetector::new()),
        Box::new(MagicValueDetector::new()),
        Box::new(ParameterAnalyzer::new()),
        Box::new(FeatureEnvyDetector::new()),
        Box::new(PrimitiveObsessionDetector::new()),
    ];
    
    let ast_node = convert_syn_to_ast_node(file);
    let mut organization_items = Vec::new();
    
    for detector in detectors {
        let anti_patterns = detector.detect_anti_patterns(&ast_node);
        
        for pattern in anti_patterns {
            let impact = detector.estimate_maintainability_impact(&pattern);
            let debt_item = convert_organization_pattern_to_debt_item(pattern, impact, path);
            organization_items.push(debt_item);
        }
    }
    
    organization_items
}

fn convert_organization_pattern_to_debt_item(
    pattern: OrganizationAntiPattern,
    impact: MaintainabilityImpact,
    path: &Path
) -> DebtItem {
    let (priority, message, context) = match pattern {
        OrganizationAntiPattern::GodObject { type_name, method_count, field_count, suggested_split, .. } => {
            (
                Priority::High,
                format!("God object '{}' with {} methods and {} fields", type_name, method_count, field_count),
                Some(format!("Consider splitting into: {}", 
                    suggested_split.iter().map(|g| &g.name).collect::<Vec<_>>().join(", ")))
            )
        }
        OrganizationAntiPattern::MagicValue { value, occurrence_count, suggested_constant_name, .. } => {
            (
                Priority::Medium,
                format!("Magic value '{}' appears {} times", value, occurrence_count),
                Some(format!("Extract constant: const {} = {};", suggested_constant_name, value))
            )
        }
        OrganizationAntiPattern::LongParameterList { function_name, parameter_count, suggested_refactoring, .. } => {
            (
                Priority::Medium,
                format!("Function '{}' has {} parameters", function_name, parameter_count),
                Some(format!("Consider: {:?}", suggested_refactoring))
            )
        }
        OrganizationAntiPattern::FeatureEnvy { method_name, envied_type, external_calls, internal_calls, .. } => {
            (
                Priority::Medium,
                format!("Method '{}' makes {} external calls vs {} internal calls", method_name, external_calls, internal_calls),
                Some(format!("Consider moving to '{}'", envied_type))
            )
        }
        OrganizationAntiPattern::PrimitiveObsession { primitive_type, usage_context, suggested_domain_type, .. } => {
            (
                Priority::Low,
                format!("Primitive obsession: '{}' used for {:?}", primitive_type, usage_context),
                Some(format!("Consider domain type: {}", suggested_domain_type))
            )
        }
        OrganizationAntiPattern::DataClump { parameter_group, suggested_struct_name, .. } => {
            (
                Priority::Medium,
                format!("Data clump with {} parameters", parameter_group.parameters.len()),
                Some(format!("Extract struct: {}", suggested_struct_name))
            )
        }
    };
    
    DebtItem {
        id: format!("organization-{}-{}", path.display(), get_line_from_pattern(&pattern)),
        debt_type: DebtType::CodeOrganization, // New debt type
        priority,
        file: path.to_path_buf(),
        line: get_line_from_pattern(&pattern),
        message,
        context,
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_god_object_detection() {
        let source = r#"
            struct UserManager {
                users: Vec<User>,
                settings: Settings,
                cache: Cache,
                logger: Logger,
                validator: Validator,
                // ... many more fields
            }
            
            impl UserManager {
                fn create_user(&self) {}
                fn update_user(&self) {}
                fn delete_user(&self) {}
                fn validate_user(&self) {}
                fn log_user_action(&self) {}
                fn cache_user_data(&self) {}
                fn send_notification(&self) {}
                fn calculate_metrics(&self) {}
                fn generate_report(&self) {}
                fn backup_data(&self) {}
                fn restore_data(&self) {}
                fn authenticate_user(&self) {}
                fn authorize_action(&self) {}
                fn encrypt_data(&self) {}
                fn decrypt_data(&self) {}
                // ... many more methods
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = GodObjectDetector::new();
        let patterns = detector.detect_anti_patterns(&ast);
        
        assert!(!patterns.is_empty());
        if let OrganizationAntiPattern::GodObject { type_name, method_count, suggested_split, .. } = &patterns[0] {
            assert_eq!(type_name, "UserManager");
            assert!(method_count > &10);
            assert!(!suggested_split.is_empty());
        } else {
            panic!("Expected god object pattern");
        }
    }
    
    #[test]
    fn test_magic_number_detection() {
        let source = r#"
            fn process_data() {
                let timeout = 5000; // Magic number
                let buffer_size = 8192; // Magic number
                let max_retries = 3; // Magic number
                
                if timeout > 5000 { // Same magic number
                    println!("Timeout exceeded");
                }
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = MagicValueDetector::new();
        let patterns = detector.detect_anti_patterns(&ast);
        
        assert!(!patterns.is_empty());
        
        let magic_5000 = patterns.iter().find(|p| {
            if let OrganizationAntiPattern::MagicValue { value, .. } = p {
                value == "5000"
            } else {
                false
            }
        });
        assert!(magic_5000.is_some());
    }
    
    #[test]
    fn test_long_parameter_list_detection() {
        let source = r#"
            fn create_user(
                username: String,
                email: String,
                first_name: String,
                last_name: String,
                age: u32,
                address: String,
                phone: String,
                country: String,
                is_active: bool,
                is_verified: bool,
                subscription_type: String,
                created_at: DateTime,
            ) -> User {
                // Implementation
                User::new()
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = ParameterAnalyzer::new();
        let patterns = detector.detect_anti_patterns(&ast);
        
        assert!(!patterns.is_empty());
        if let OrganizationAntiPattern::LongParameterList { function_name, parameter_count, .. } = &patterns[0] {
            assert_eq!(function_name, "create_user");
            assert!(parameter_count > &8);
        } else {
            panic!("Expected long parameter list pattern");
        }
    }
}
```

## Configuration

```toml
[organization]
enabled = true
detectors = ["god_objects", "magic_values", "long_parameters", "feature_envy", "primitive_obsession"]

[organization.god_objects]
max_methods = 15
max_fields = 10
max_responsibilities = 3
analyze_trait_implementations = true

[organization.magic_values]
ignore_common_values = true
min_occurrence_threshold = 2
detect_string_literals = true
detect_numeric_literals = true

[organization.long_parameters]
max_parameters = 5
detect_data_clumps = true
suggest_builder_pattern = true

[organization.feature_envy]
external_call_threshold = 5
internal_call_ratio = 0.3

[organization.primitive_obsession]
track_string_identifiers = true
track_numeric_measurements = true
suggest_domain_types = true
```

## Expected Impact

After implementation:

1. **Improved Maintainability**: Systematic identification of organizational debt that impacts long-term maintenance
2. **Better Code Structure**: Guidance for breaking down large classes and improving organization
3. **Reduced Magic Values**: Elimination of hardcoded values through constant extraction
4. **Cleaner APIs**: Better parameter organization and function design
5. **Domain Modeling**: Encouragement of proper domain types over primitive obsession

This organizational analysis complements existing complexity and security detection by focusing on structural and design-level technical debt that accumulates over time and significantly impacts code maintainability.