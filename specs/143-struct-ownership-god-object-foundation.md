---
number: 143
title: Struct-Ownership-Based God Object Analysis (Phase 1)
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-01-23
supersedes: 139
related: [144, 145]
---

# Specification 143: Struct-Ownership-Based God Object Analysis (Phase 1)

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None
**Supersedes**: Spec 139 (split into 3 phases)
**Related**: Spec 144 (call graph integration), Spec 145 (multi-language support)

## Context

**Current Problem**: Debtmap's god object detection correctly identifies large files (e.g., `config.rs` with 2459 lines, 201 functions), but generates poor quality refactoring recommendations that would harm code quality rather than improve it.

**Real-World Example from config.rs**:
```
Current Recommendation (BAD):
├─ config_core_operations.rs - Core Operations (151 methods, ~3020 lines)
└─ config_data_access.rs - Data Access (21 methods, ~420 lines)
```

**Why This is Wrong**:
1. **Still a god object**: 151 methods in one module violates single responsibility
2. **Naive grouping**: Groups `get_config()`, `get_ignore_patterns()`, `get_error_handling_config()` together just because they all start with "get"
3. **Ignores struct ownership**: Doesn't consider that methods belong to different structs (ScoringWeights, ThresholdsConfig, ErrorHandlingConfig, etc.)
4. **No size validation**: Recommends modules that exceed reasonable bounds (>40 methods)

**Root Cause Analysis**:

1. **`infer_responsibility_from_method()` is too simplistic** (src/organization/god_object_analysis.rs:259):
   ```rust
   if lower.starts_with("get") || lower.starts_with("set") {
       "Data Access".to_string()  // Groups ALL getters regardless of context
   }
   ```

2. **No struct ownership tracking**: Current code doesn't know which methods belong to which structs

3. **No size validation** (line 336): Only checks `methods.len() > 5` (minimum), no maximum enforcement

4. **Struct-based splitting not used for general files**: `suggest_module_splits_by_domain()` exists but only used for GodModule type

**Impact**:
- Top #1 recommendation in debtmap's own analysis is completely wrong
- Users may follow bad advice and create worse code structure
- Undermines trust in tool's recommendations

## Objective

Transform god object refactoring recommendations from naive method-name-based grouping to intelligent struct-ownership and domain-semantic grouping that produces actionable, size-validated module splits **for Rust codebases**.

**Success Criteria**: For config.rs (2459 lines, 27 structs, 101 functions), recommend 6-8 focused modules of 12-31 functions each instead of 2 giant modules.

**Phase 1 Scope** (This Spec):
- ✅ Struct ownership analysis (Rust-only)
- ✅ Enhanced domain classification (10+ patterns)
- ✅ Size validation (5-40 methods)
- ✅ Basic module split metadata
- ✅ config.rs test case

**Deferred to Later Phases**:
- ⏸️ Cohesion scoring (requires call graph - Spec 144)
- ⏸️ Dependency analysis (requires call graph - Spec 144)
- ⏸️ Python/TypeScript support (Spec 145)
- ⏸️ Recursive splitting (simpler 2-level approach in this phase)

## Requirements

### Functional Requirements

**FR1: Struct Ownership Analysis (Rust-only)**
- Track which methods belong to which struct (impl blocks)
- Identify standalone functions vs struct methods
- Handle multiple impl blocks for the same struct
- Detect and exclude trait implementations from refactoring suggestions

**FR2: Domain-Based Struct Grouping**
- Classify structs into semantic domains (scoring, thresholds, detection, output, etc.)
- Use enhanced pattern matching beyond simple keyword detection
- Handle nested/hierarchical relationships between structs
- Provide clear "unknown" classification for ambiguous cases

**FR3: Size-Aware Module Validation**
- Reject recommendations with >40 methods per module
- Warn for 20-40 methods (borderline acceptable)
- Ensure minimum 5 methods per module (avoid over-fragmentation)
- Use simple 2-level splitting for oversized groups (not recursive)

**FR4: Enhanced Module Split Metadata**
- Track structs AND methods being moved (not just methods)
- Provide implementation priority/order guidance (High/Medium/Low)
- Include warnings for quality issues (e.g., borderline size)
- Estimate line counts more accurately using struct spans

### Non-Functional Requirements

**NFR1: Performance**
- Struct ownership analysis must add <10% to god object detection time
- Baseline: Measure current detection time on config.rs before implementation
- Cache parsed struct information within single analysis run

**NFR2: Accuracy**
- Reduce false recommendation rate by >50%
- Ensure all recommendations have ≤40 methods per module
- All recommended modules have ≥5 methods

**NFR3: Maintainability**
- Separate struct analysis logic from output formatting
- Use pure functions for domain classification
- Support adding new domain patterns without core logic changes
- Domain patterns hard-coded initially (config file support deferred to v0.4.0)

**NFR4: Backward Compatibility**
- Maintain existing `recommend_module_splits()` signature
- Keep fallback to method-name-based grouping when struct info unavailable
- All existing tests continue to pass

## Acceptance Criteria

### AC1: Struct Ownership Tracking (Rust-only)
- [ ] Parse Rust impl blocks to map methods to structs
- [ ] Track standalone functions separately
- [ ] Handle multiple impl blocks for the same struct (merge methods)
- [ ] Exclude trait implementations (e.g., `impl Display for MyStruct`)
- [ ] Store struct ownership in analysis metadata
- [ ] Unit tests for basic struct ownership extraction
- [ ] Unit tests for multiple impl blocks
- [ ] Unit tests for trait impl exclusion

### AC2: Enhanced Domain Classification
- [ ] Implement `classify_struct_domain_enhanced()` with 10+ patterns
- [ ] Detect scoring/weights domain (scoring, weight, multiplier, factor)
- [ ] Detect thresholds domain (threshold, limit, bound, validation)
- [ ] Detect detection domain (detection, detector, checker, analyzer)
- [ ] Detect output domain (display, output, format, render)
- [ ] Detect language domain (language, rust, python, javascript)
- [ ] Detect error handling domain (error, pattern, severity)
- [ ] Detect context domain (context, rule, matcher)
- [ ] Detect I/O domain (reader, writer, loader, saver)
- [ ] Detect config domain (config, settings, options)
- [ ] Classify unknown structs as "utilities" or "core" appropriately
- [ ] Unit tests for each domain pattern
- [ ] Unit tests for ambiguous/unknown structs

### AC3: Size Validation
- [ ] Implement `validate_and_refine_splits()` function
- [ ] Reject recommendations with >40 methods
- [ ] Warn for 20-40 methods with specific message
- [ ] Filter out groups with <5 methods
- [ ] For oversized domains (>40 methods), use simple 2-level split strategy
- [ ] Ensure no recommended split exceeds thresholds after validation
- [ ] Unit tests for oversized, undersized, and valid splits
- [ ] Unit tests for 2-level splitting strategy

### AC4: Enhanced ModuleSplit Structure
- [ ] Add `structs_to_move: Vec<String>` field
- [ ] Add `method_count: usize` field
- [ ] Add `warning: Option<String>` field for quality issues
- [ ] Add `priority: Priority` enum (High/Medium/Low) for implementation order
- [ ] Update serialization/deserialization for new fields
- [ ] Ensure backward compatibility with existing code
- [ ] Unit tests for ModuleSplit construction

### AC5: Improved Output Format
- [ ] Show structs being moved, not just methods
- [ ] Display method counts per recommendation
- [ ] Show implementation priority for each recommendation
- [ ] Include warnings when present
- [ ] Provide phased implementation guidance (what to split first)
- [ ] Show expected outcomes (estimated lines per module)
- [ ] Integration test for output format

### AC6: config.rs Test Case
- [ ] Recommend 6-8 modules instead of 2
- [ ] Each recommended module has ≤40 methods
- [ ] Modules align with actual domains (scoring, thresholds, detection, etc.)
- [ ] Provide accurate line count estimates per module
- [ ] Show which structs belong to which module
- [ ] Integration test on actual src/config.rs file

### AC7: Error Handling
- [ ] Handle files with no structs (procedural code)
- [ ] Handle files with no impl blocks (struct-only files)
- [ ] Handle parsing errors gracefully (return empty analysis)
- [ ] Handle generic impl blocks (`impl<T>`)
- [ ] Handle empty impl blocks
- [ ] Unit tests for each error condition

### AC8: Performance Validation
- [ ] Measure baseline god object detection time on config.rs
- [ ] Measure detection time after struct ownership implementation
- [ ] Verify <10% increase in analysis time
- [ ] Add performance regression test
- [ ] Document performance characteristics

## Technical Details

### Implementation Approach

**Phase 1: Struct Ownership Analysis**

1. **Parse struct definitions and impl blocks** (Rust-specific):
   ```rust
   pub struct StructOwnershipAnalyzer {
       struct_to_methods: HashMap<String, Vec<String>>,
       method_to_struct: HashMap<String, String>,
       standalone_functions: Vec<String>,
       struct_locations: HashMap<String, (usize, usize)>, // line spans
   }

   impl StructOwnershipAnalyzer {
       pub fn analyze_file(parsed: &syn::File) -> Self {
           // Walk AST to find ItemStruct and ItemImpl
           // Map impl blocks to struct names
           // Track standalone functions
           // Exclude trait implementations
       }

       pub fn get_struct_methods(&self, struct_name: &str) -> &[String] {
           self.struct_to_methods.get(struct_name)
               .map(|v| v.as_slice())
               .unwrap_or(&[])
       }

       pub fn is_trait_impl(&self, method_name: &str) -> bool {
           // Check if method is from trait implementation
       }
   }
   ```

2. **Integration with existing TypeVisitor**:
   - Extend `TypeVisitor` (god_object_detector.rs:715-900) to track trait vs inherent impls
   - Add `trait_implementations: Vec<String>` field to distinguish
   - Filter trait methods from refactoring recommendations

**Phase 2: Domain-Based Grouping**

1. **Enhanced domain classifier**:
   ```rust
   /// Classify a struct into a semantic domain based on naming patterns
   ///
   /// Uses pattern matching on struct names to infer domain responsibility.
   /// Patterns are ordered from most specific to most general.
   pub fn classify_struct_domain_enhanced(
       struct_name: &str,
       methods: &[String],
   ) -> String {
       let lower = struct_name.to_lowercase();

       // Specific patterns first (most to least specific)
       if matches_scoring_pattern(&lower) {
           return "scoring".to_string();
       }
       if matches_threshold_pattern(&lower) {
           return "thresholds".to_string();
       }
       if matches_detection_pattern(&lower) {
           return "detection".to_string();
       }
       if matches_output_pattern(&lower) {
           return "output".to_string();
       }
       if matches_language_pattern(&lower) {
           return "languages".to_string();
       }
       if matches_error_pattern(&lower) {
           return "error_handling".to_string();
       }
       if matches_context_pattern(&lower) {
           return "context".to_string();
       }
       if matches_io_pattern(&lower) {
           return "io".to_string();
       }
       if matches_config_pattern(&lower) {
           return "config".to_string();
       }

       // Analyze method names as secondary signal
       let method_pattern = infer_domain_from_methods(methods);
       if !method_pattern.is_empty() {
           return method_pattern;
       }

       // Default to utilities
       "utilities".to_string()
   }

   fn matches_scoring_pattern(name: &str) -> bool {
       name.contains("scoring")
           || name.contains("weight")
           || name.contains("multiplier")
           || name.contains("factor")
   }

   fn matches_threshold_pattern(name: &str) -> bool {
       name.contains("threshold")
           || name.contains("limit")
           || name.contains("bound")
   }

   fn matches_detection_pattern(name: &str) -> bool {
       name.contains("detection")
           || name.contains("detector")
           || name.contains("checker")
           || name.contains("analyzer")
   }

   fn matches_output_pattern(name: &str) -> bool {
       name.contains("display")
           || name.contains("output")
           || name.contains("format")
           || name.contains("render")
           || name.contains("print")
   }

   fn matches_language_pattern(name: &str) -> bool {
       name.contains("language")
           || name.contains("rust")
           || name.contains("python")
           || name.contains("javascript")
           || name.contains("typescript")
   }

   fn matches_error_pattern(name: &str) -> bool {
       name.contains("error")
           || name.contains("severity")
   }

   fn matches_context_pattern(name: &str) -> bool {
       name.contains("context")
           || name.contains("rule")
           || name.contains("matcher")
   }

   fn matches_io_pattern(name: &str) -> bool {
       name.contains("reader")
           || name.contains("writer")
           || name.contains("loader")
           || name.contains("saver")
   }

   fn matches_config_pattern(name: &str) -> bool {
       name.contains("config")
           || name.contains("settings")
           || name.contains("options")
   }
   ```

2. **Struct grouping logic**:
   ```rust
   pub fn group_structs_by_domain(
       structs: &[StructMetrics],
       ownership: &StructOwnershipAnalyzer,
   ) -> HashMap<String, Vec<StructWithMethods>> {
       let mut groups: HashMap<String, Vec<StructWithMethods>> = HashMap::new();

       for struct_meta in structs {
           let domain = classify_struct_domain_enhanced(
               &struct_meta.name,
               ownership.get_struct_methods(&struct_meta.name),
           );

           let methods = ownership.get_struct_methods(&struct_meta.name).to_vec();
           groups.entry(domain)
               .or_default()
               .push(StructWithMethods {
                   name: struct_meta.name.clone(),
                   methods,
                   line_span: struct_meta.line_span,
               });
       }

       groups
   }
   ```

**Phase 3: Size Validation and Refinement**

1. **Validation logic** (simple 2-level approach):
   ```rust
   /// Validate and refine module splits to ensure proper sizing
   ///
   /// Filters out splits that are too small (<5 methods) or too large (>40 methods).
   /// For oversized splits, uses a simple 2-level strategy to divide them.
   pub fn validate_and_refine_splits(
       splits: Vec<ModuleSplit>,
   ) -> Vec<ModuleSplit> {
       splits
           .into_iter()
           .flat_map(|split| {
               let method_count = split.method_count;

               // Too small - skip
               if method_count < 5 {
                   return vec![];
               }

               // Perfect size
               if method_count <= 20 {
                   return vec![ModuleSplit {
                       priority: Priority::High,
                       ..split
                   }];
               }

               // Borderline - warn but allow
               if method_count <= 40 {
                   return vec![ModuleSplit {
                       warning: Some(format!(
                           "{} methods is borderline - consider further splitting",
                           method_count
                       )),
                       priority: Priority::Medium,
                       ..split
                   }];
               }

               // Too large - use simple 2-level split
               split_into_two_levels(split)
           })
           .collect()
   }

   /// Simple 2-level splitting for oversized modules
   fn split_into_two_levels(split: ModuleSplit) -> Vec<ModuleSplit> {
       // Divide structs into two roughly equal groups
       let mid = split.structs_to_move.len() / 2;

       let (first_half_structs, second_half_structs) =
           split.structs_to_move.split_at(mid);

       // Calculate method counts for each half
       let first_half_methods = /* count methods in first_half_structs */;
       let second_half_methods = /* count methods in second_half_structs */;

       vec![
           ModuleSplit {
               suggested_name: format!("{}_part1", split.suggested_name),
               structs_to_move: first_half_structs.to_vec(),
               method_count: first_half_methods,
               priority: Priority::Medium,
               warning: Some("Auto-split due to size".to_string()),
               ..split.clone()
           },
           ModuleSplit {
               suggested_name: format!("{}_part2", split.suggested_name),
               structs_to_move: second_half_structs.to_vec(),
               method_count: second_half_methods,
               priority: Priority::Medium,
               warning: Some("Auto-split due to size".to_string()),
               ..split
           },
       ]
   }
   ```

**Phase 4: Enhanced Output Format**

1. **New output structure** (in formatter.rs):
   ```rust
   // Show phased refactoring strategy
   writeln!(output, "RECOMMENDED REFACTORING STRATEGY:")?;
   writeln!(output)?;
   writeln!(output, "Suggested Module Splits ({} modules):", splits.len())?;
   writeln!(output)?;

   // Sort by priority (High > Medium > Low)
   let mut sorted_splits = splits.clone();
   sorted_splits.sort_by_key(|s| match s.priority {
       Priority::High => 0,
       Priority::Medium => 1,
       Priority::Low => 2,
   });

   for split in sorted_splits.iter() {
       // Show module name and priority
       let priority_icon = match split.priority {
           Priority::High => "⭐⭐⭐",
           Priority::Medium => "⭐⭐",
           Priority::Low => "⭐",
       };
       writeln!(output, "├─ {} {} - {}",
           priority_icon,
           split.suggested_name,
           split.responsibility)?;

       // Show structs being moved
       writeln!(output, "│   → Structs: {} ({} structs)",
           split.structs_to_move.join(", "),
           split.structs_to_move.len())?;

       // Show method count and line estimate
       writeln!(output, "│   → Methods: {} functions (~{} lines)",
           split.method_count, split.estimated_lines)?;

       // Show warnings
       if let Some(warning) = &split.warning {
           writeln!(output, "│   ⚠️  {}", warning)?;
       }
       writeln!(output, "│")?;
   }
   ```

### Architecture Changes

**New Files**:
- `src/organization/struct_ownership.rs` - Struct ownership analysis
- `src/organization/domain_classifier.rs` - Enhanced domain classification
- `src/organization/split_validator.rs` - Size validation and refinement

**Modified Files**:
- `src/organization/god_object_analysis.rs` - Use new struct-based approach
- `src/organization/god_object_detector.rs` - Integrate struct ownership
- `src/priority/formatter.rs` - Enhanced output format

### Data Structures

```rust
// Enhanced ModuleSplit with Phase 1 metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleSplit {
    pub suggested_name: String,
    pub methods_to_move: Vec<String>,
    pub structs_to_move: Vec<String>,        // NEW
    pub responsibility: String,
    pub estimated_lines: usize,
    pub method_count: usize,                 // NEW
    pub warning: Option<String>,             // NEW
    pub priority: Priority,                  // NEW
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    High,    // Clear boundaries, self-contained, high value
    Medium,  // Some complexity, moderate value
    Low,     // Complex or unclear boundaries
}

// Struct with method ownership
#[derive(Debug, Clone)]
pub struct StructWithMethods {
    pub name: String,
    pub methods: Vec<String>,
    pub line_span: (usize, usize),
}

// Struct ownership analyzer
pub struct StructOwnershipAnalyzer {
    struct_to_methods: HashMap<String, Vec<String>>,
    method_to_struct: HashMap<String, String>,
    standalone_functions: Vec<String>,
    struct_locations: HashMap<String, (usize, usize)>,
}
```

### APIs and Interfaces

**New Public Functions**:

```rust
// Struct ownership analysis
pub fn analyze_struct_ownership(parsed: &syn::File) -> StructOwnershipAnalyzer;

// Enhanced domain classification
pub fn classify_struct_domain_enhanced(
    struct_name: &str,
    methods: &[String],
) -> String;

// Struct-based splitting (PRIMARY strategy)
pub fn suggest_splits_by_struct_grouping(
    structs: &[StructMetrics],
    ownership: &StructOwnershipAnalyzer,
) -> Vec<ModuleSplit>;

// Size validation
pub fn validate_and_refine_splits(
    splits: Vec<ModuleSplit>,
) -> Vec<ModuleSplit>;

// Priority assignment based on size and clarity
pub fn assign_split_priorities(splits: &mut [ModuleSplit]);
```

**Modified Functions**:

```rust
// Update to use struct-based approach as primary strategy
pub fn recommend_module_splits(
    type_name: &str,
    methods: &[String],
    responsibility_groups: &HashMap<String, Vec<String>>,
    struct_ownership: Option<&StructOwnershipAnalyzer>,  // NEW parameter
) -> Vec<ModuleSplit>;
```

## Dependencies

**Prerequisites**: None (this is a standalone improvement)

**Affected Components**:
- `src/organization/god_object_analysis.rs` - Core splitting logic
- `src/organization/god_object_detector.rs` - Detection and analysis
- `src/priority/formatter.rs` - Output formatting

**External Dependencies**: None (uses existing syn parser for Rust)

## Testing Strategy

### Unit Tests

**Struct Ownership Analysis** (`tests/struct_ownership_tests.rs`):
```rust
#[test]
fn test_rust_struct_ownership_simple() {
    let code = r#"
        struct Config {
            value: String,
        }

        impl Config {
            fn new() -> Self { Config { value: String::new() } }
            fn get_value(&self) -> &str { &self.value }
        }

        fn standalone_helper() { }
    "#;

    let parsed = syn::parse_file(code).unwrap();
    let analyzer = analyze_struct_ownership(&parsed);
    assert_eq!(analyzer.get_struct_methods("Config"), &["new", "get_value"]);
    assert!(analyzer.standalone_functions.contains(&"standalone_helper".to_string()));
}

#[test]
fn test_multiple_impl_blocks() {
    let code = r#"
        struct Config { }

        impl Config {
            fn new() -> Self { Config {} }
        }

        impl Config {
            fn get_value(&self) -> i32 { 42 }
        }
    "#;

    let parsed = syn::parse_file(code).unwrap();
    let analyzer = analyze_struct_ownership(&parsed);

    // Should merge methods from multiple impl blocks
    let methods = analyzer.get_struct_methods("Config");
    assert_eq!(methods.len(), 2);
    assert!(methods.contains(&"new".to_string()));
    assert!(methods.contains(&"get_value".to_string()));
}

#[test]
fn test_trait_impl_exclusion() {
    let code = r#"
        struct Config { }

        impl Config {
            fn new() -> Self { Config {} }
        }

        impl Display for Config {
            fn fmt(&self, f: &mut Formatter) -> Result { Ok(()) }
        }
    "#;

    let parsed = syn::parse_file(code).unwrap();
    let analyzer = analyze_struct_ownership(&parsed);

    // Should only include inherent impl methods, not trait methods
    assert_eq!(analyzer.get_struct_methods("Config"), &["new"]);
}

#[test]
fn test_generic_impl_blocks() {
    let code = r#"
        struct Container<T> {
            value: T,
        }

        impl<T> Container<T> {
            fn new(value: T) -> Self { Container { value } }
            fn get(&self) -> &T { &self.value }
        }
    "#;

    let parsed = syn::parse_file(code).unwrap();
    let analyzer = analyze_struct_ownership(&parsed);

    // Should handle generic impl blocks
    assert_eq!(analyzer.get_struct_methods("Container"), &["new", "get"]);
}
```

**Domain Classification** (`tests/domain_classification_tests.rs`):
```rust
#[test]
fn test_classify_scoring_structs() {
    assert_eq!(classify_struct_domain_enhanced("ScoringWeights", &[]), "scoring");
    assert_eq!(classify_struct_domain_enhanced("RoleMultipliers", &[]), "scoring");
    assert_eq!(classify_struct_domain_enhanced("ComplexityFactor", &[]), "scoring");
}

#[test]
fn test_classify_threshold_structs() {
    assert_eq!(classify_struct_domain_enhanced("ThresholdsConfig", &[]), "thresholds");
    assert_eq!(classify_struct_domain_enhanced("ValidationLimits", &[]), "thresholds");
    assert_eq!(classify_struct_domain_enhanced("MaxBounds", &[]), "thresholds");
}

#[test]
fn test_classify_detection_structs() {
    assert_eq!(classify_struct_domain_enhanced("PatternDetector", &[]), "detection");
    assert_eq!(classify_struct_domain_enhanced("GodObjectChecker", &[]), "detection");
    assert_eq!(classify_struct_domain_enhanced("ComplexityAnalyzer", &[]), "detection");
}

#[test]
fn test_classify_output_structs() {
    assert_eq!(classify_struct_domain_enhanced("OutputFormatter", &[]), "output");
    assert_eq!(classify_struct_domain_enhanced("DisplayConfig", &[]), "output");
    assert_eq!(classify_struct_domain_enhanced("RenderOptions", &[]), "output");
}

#[test]
fn test_classify_unknown_struct() {
    // Should classify unknown structs as utilities
    assert_eq!(classify_struct_domain_enhanced("MyCustomStruct", &[]), "utilities");
}

#[test]
fn test_ambiguous_struct_uses_methods() {
    // When struct name is ambiguous, use method names as signal
    let methods = vec!["detect_pattern".to_string(), "check_violation".to_string()];
    assert_eq!(classify_struct_domain_enhanced("Analyzer", &methods), "detection");
}
```

**Size Validation** (`tests/split_validation_tests.rs`):
```rust
#[test]
fn test_reject_oversized_splits() {
    let split = ModuleSplit {
        suggested_name: "oversized".to_string(),
        methods_to_move: vec![],
        structs_to_move: vec!["S1".into(), "S2".into(), "S3".into()],
        responsibility: "test".to_string(),
        estimated_lines: 1000,
        method_count: 60,
        warning: None,
        priority: Priority::Medium,
    };

    let validated = validate_and_refine_splits(vec![split]);

    // Should be split into 2 parts
    assert_eq!(validated.len(), 2);
    assert!(validated.iter().all(|s| s.method_count <= 40));
}

#[test]
fn test_filter_undersized_splits() {
    let split = ModuleSplit {
        suggested_name: "undersized".to_string(),
        methods_to_move: vec![],
        structs_to_move: vec![],
        responsibility: "test".to_string(),
        estimated_lines: 50,
        method_count: 3,
        warning: None,
        priority: Priority::Low,
    };

    let validated = validate_and_refine_splits(vec![split]);
    assert_eq!(validated.len(), 0); // Too small, filtered out
}

#[test]
fn test_accept_valid_splits() {
    let split = ModuleSplit {
        suggested_name: "valid".to_string(),
        methods_to_move: vec![],
        structs_to_move: vec![],
        responsibility: "test".to_string(),
        estimated_lines: 200,
        method_count: 15,
        warning: None,
        priority: Priority::Medium,
    };

    let validated = validate_and_refine_splits(vec![split.clone()]);
    assert_eq!(validated.len(), 1);
    assert_eq!(validated[0].method_count, 15);
    assert_eq!(validated[0].priority, Priority::High); // Upgraded to high
}

#[test]
fn test_warn_borderline_splits() {
    let split = ModuleSplit {
        suggested_name: "borderline".to_string(),
        methods_to_move: vec![],
        structs_to_move: vec![],
        responsibility: "test".to_string(),
        estimated_lines: 500,
        method_count: 35,
        warning: None,
        priority: Priority::High,
    };

    let validated = validate_and_refine_splits(vec![split]);
    assert_eq!(validated.len(), 1);
    assert_eq!(validated[0].priority, Priority::Medium); // Downgraded
    assert!(validated[0].warning.is_some());
    assert!(validated[0].warning.as_ref().unwrap().contains("borderline"));
}
```

### Integration Tests

**config.rs Test Case** (`tests/god_object_config_rs_test.rs`):
```rust
#[test]
fn test_config_rs_recommendation_quality() {
    // Run god object detection on actual config.rs
    let code = std::fs::read_to_string("src/config.rs")
        .expect("Failed to read config.rs");
    let parsed = syn::parse_file(&code).expect("Failed to parse config.rs");

    let detector = GodObjectDetector::with_source_content(&code);
    let analysis = detector.analyze_enhanced(Path::new("src/config.rs"), &parsed);

    let splits = match analysis.classification {
        GodObjectType::GodModule { suggested_splits, .. } => suggested_splits,
        _ => panic!("Expected GodModule classification for config.rs"),
    };

    // Should recommend 6-8 modules, not 2
    assert!(splits.len() >= 6 && splits.len() <= 8,
        "Expected 6-8 modules, got {}", splits.len());

    // No module should exceed 40 methods
    for split in &splits {
        assert!(split.method_count <= 40,
            "Module {} has {} methods (max 40)",
            split.suggested_name, split.method_count);
    }

    // Should have a scoring module
    assert!(splits.iter().any(|s| s.responsibility == "scoring"),
        "Expected scoring module");

    // Should have a thresholds module
    assert!(splits.iter().any(|s| s.responsibility == "thresholds"),
        "Expected thresholds module");

    // Each module should have at least 5 methods
    for split in &splits {
        assert!(split.method_count >= 5,
            "Module {} has only {} methods (min 5)",
            split.suggested_name, split.method_count);
    }

    // All modules should have structs assigned
    for split in &splits {
        assert!(!split.structs_to_move.is_empty(),
            "Module {} has no structs assigned", split.suggested_name);
    }
}
```

### Performance Tests

```rust
#[test]
fn test_struct_ownership_performance() {
    let code = std::fs::read_to_string("src/config.rs")
        .expect("Failed to read config.rs");
    let parsed = syn::parse_file(&code).expect("Failed to parse");

    // Measure baseline (without struct ownership)
    let baseline_start = Instant::now();
    let detector_baseline = GodObjectDetector::with_source_content(&code);
    let _analysis_baseline = detector_baseline.analyze_comprehensive(
        Path::new("src/config.rs"),
        &parsed
    );
    let baseline_duration = baseline_start.elapsed();

    // Measure with struct ownership
    let enhanced_start = Instant::now();
    let detector_enhanced = GodObjectDetector::with_source_content(&code);
    let _analysis_enhanced = detector_enhanced.analyze_enhanced(
        Path::new("src/config.rs"),
        &parsed
    );
    let enhanced_duration = enhanced_start.elapsed();

    // Should be within 10% of baseline
    let overhead_percent = ((enhanced_duration.as_millis() as f64
        / baseline_duration.as_millis() as f64) - 1.0) * 100.0;

    assert!(overhead_percent < 10.0,
        "Struct ownership adds {}% overhead (max 10%)", overhead_percent);
}
```

## Error Handling

### Error Scenarios

1. **File with no structs** (procedural code):
   - Return empty `StructOwnershipAnalyzer`
   - Fall back to method-name-based grouping
   - Log debug message about fallback

2. **File with no impl blocks** (struct-only):
   - Create analyzer with empty method mappings
   - Return empty splits (nothing to refactor)

3. **Parsing errors**:
   - Catch `syn::Error` gracefully
   - Return `GodObjectAnalysis::default()`
   - Log warning about parsing failure

4. **Generic impl blocks**:
   - Extract type name from generic (e.g., `Container<T>` → `Container`)
   - Associate methods with base struct name

5. **Empty impl blocks**:
   - Skip empty impls
   - Don't create entries in ownership map

### Error Handling Implementation

```rust
pub fn analyze_struct_ownership(parsed: &syn::File) -> Result<StructOwnershipAnalyzer, String> {
    let mut analyzer = StructOwnershipAnalyzer::default();

    for item in &parsed.items {
        match item {
            syn::Item::Struct(item_struct) => {
                // Handle struct definitions
                analyzer.track_struct(item_struct);
            }
            syn::Item::Impl(item_impl) => {
                // Handle impl blocks, skip trait impls
                if item_impl.trait_.is_none() {
                    analyzer.track_impl(item_impl)
                        .map_err(|e| format!("Failed to track impl: {}", e))?;
                }
            }
            syn::Item::Fn(item_fn) => {
                // Handle standalone functions
                analyzer.track_standalone_function(item_fn);
            }
            _ => {}
        }
    }

    Ok(analyzer)
}
```

## Documentation Requirements

### Code Documentation

1. **Module-level docs** for new files:
   - `src/organization/struct_ownership.rs` - Explain ownership tracking approach
   - `src/organization/domain_classifier.rs` - Document pattern matching logic
   - `src/organization/split_validator.rs` - Explain validation rules

2. **Function-level docs**:
   - Document all public functions with examples
   - Explain the reasoning behind domain classification patterns
   - Document the size thresholds and why they were chosen

3. **Algorithm explanation**:
   - Comment the struct-based splitting approach
   - Explain the 2-level splitting strategy
   - Document trait impl exclusion logic

### User Documentation

Update CLAUDE.md with:
```markdown
## God Object Recommendations

Debtmap uses struct-ownership analysis to recommend module splits:

- **Struct Tracking**: Identifies which methods belong to which structs
- **Domain Classification**: Groups structs by semantic domain (scoring, thresholds, etc.)
- **Size Validation**: Ensures 5-40 methods per recommended module
- **Priority Guidance**: Shows high/medium/low priority for implementation order

Example output:
```
RECOMMENDED REFACTORING STRATEGY:

Suggested Module Splits (7 modules):

├─ ⭐⭐⭐ config/scoring - scoring
│   → Structs: ScoringWeights, RoleMultipliers (2 structs)
│   → Methods: 18 functions (~360 lines)
│
├─ ⭐⭐⭐ config/thresholds - thresholds
│   → Structs: ThresholdsConfig, ValidationLimits (2 structs)
│   → Methods: 14 functions (~280 lines)
│
...
```
```

## Migration and Compatibility

### Breaking Changes

None - this is purely an improvement to recommendation quality.

### Backward Compatibility

- Keep existing `recommend_module_splits()` signature with optional parameter
- Maintain fallback to method-name-based grouping when struct info unavailable
- Ensure output is still readable without new metadata fields
- All existing tests continue to pass

### Rollout Strategy

1. **v0.3.0**: Introduce struct-based splitting (this spec)
   - Feature flag: `use_struct_ownership` (default: true)
   - Can be disabled for debugging/comparison

2. **v0.3.1**: Remove feature flag, make struct-based splitting default

3. **v0.4.0**: Remove old method-name-based logic entirely

## Success Metrics

### Quantitative Metrics

1. **Recommendation Quality**:
   - ✅ 0% of recommendations exceed 40 methods (currently: 50%)
   - ✅ Average module size: 15-25 methods (currently: 86 methods)
   - ✅ Number of recommended modules: 6-8 for config.rs (currently: 2)

2. **Accuracy**:
   - ✅ >80% of recommendations align with actual domain boundaries
   - ✅ <5% of recommendations flagged with warnings
   - ✅ All recommendations have 5-40 methods

3. **Performance**:
   - ✅ <10% increase in god object detection time
   - ✅ <100ms for struct ownership analysis on config.rs

### Qualitative Metrics

1. **User Trust**:
   - Recommendations look sensible and actionable
   - Users can follow recommendations without manual validation
   - Domain separation is obvious and intuitive

2. **Code Quality**:
   - Recommended splits actually improve maintainability
   - Each module has clear, single responsibility

## Implementation Plan

### Phase 1: Foundation (Week 1, Days 1-3)
- [ ] Create `struct_ownership.rs` with basic analyzer
- [ ] Implement struct and impl tracking
- [ ] Add trait impl exclusion
- [ ] Write unit tests for struct ownership

### Phase 2: Domain Classification (Week 1, Days 4-5)
- [ ] Create `domain_classifier.rs`
- [ ] Implement 10+ domain patterns
- [ ] Add method-name fallback classification
- [ ] Write unit tests for domain classification

### Phase 3: Validation & Priority (Week 2, Days 1-2)
- [ ] Create `split_validator.rs`
- [ ] Implement size validation (5-40 methods)
- [ ] Add 2-level splitting for oversized modules
- [ ] Implement priority assignment
- [ ] Write unit tests for validation

### Phase 4: Integration (Week 2, Days 3-4)
- [ ] Extend `ModuleSplit` with new fields
- [ ] Update `GodObjectDetector` to use struct ownership
- [ ] Update output formatter
- [ ] Integration test on config.rs

### Phase 5: Polish & Performance (Week 2, Day 5)
- [ ] Performance testing and optimization
- [ ] Documentation updates
- [ ] Error handling refinement
- [ ] Final integration testing

## Related Specifications

- **Spec 144**: Call Graph Integration for Cohesion Scoring (Phase 2)
  - Adds `cohesion_score` calculation
  - Adds `dependencies_in/out` analysis
  - Enables circular dependency detection

- **Spec 145**: Multi-Language God Object Support (Phase 3)
  - Python class method tracking
  - TypeScript/JavaScript support
  - Language-specific domain patterns

## Notes

- Domain patterns are hard-coded in Phase 1 (config file support in v0.4.0)
- Trait implementations are excluded from refactoring (can't be moved separately)
- Generic impl blocks are supported (extracted base type name)
- Performance baseline must be measured before implementation begins
