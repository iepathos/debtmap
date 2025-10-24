---
number: 139
title: Struct-Ownership-Based God Object Splitting
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-01-23
---

# Specification 139: Struct-Ownership-Based God Object Splitting

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

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
5. **Poor domain separation**: Mixes scoring, thresholds, detection, and I/O concerns

**Root Cause Analysis**:

1. **`infer_responsibility_from_method()` is too simplistic** (src/organization/god_object_analysis.rs:259):
   ```rust
   if lower.starts_with("get") || lower.starts_with("set") {
       "Data Access".to_string()  // Groups ALL getters regardless of context
   }
   ```

2. **No struct ownership tracking**: Current code (`recommend_module_splits()` at line 328) doesn't know which methods belong to which structs

3. **No size validation** (line 336): Only checks `methods.len() > 5` (minimum), no maximum enforcement

4. **Struct-based splitting not used for general files**: `suggest_module_splits_by_domain()` exists (line 354) but only used for GodModule type, not for large multi-struct files

**Impact**:
- Top #1 recommendation in debtmap's own analysis is completely wrong
- Users may follow bad advice and create worse code structure
- Undermines trust in tool's recommendations
- Wastes developer time investigating poor refactoring suggestions

## Objective

Transform god object refactoring recommendations from naive method-name-based grouping to intelligent struct-ownership and domain-semantic grouping that produces actionable, size-validated module splits.

**Success Criteria**: For config.rs (2459 lines, 27 structs, 101 functions), recommend 6-8 focused modules of 12-31 functions each instead of 2 giant modules.

## Requirements

### Functional Requirements

**FR1: Struct Ownership Analysis**
- Track which methods belong to which struct (impl blocks)
- Identify standalone functions vs struct methods
- Group related structs by domain semantics
- Detect helper functions that support multiple structs

**FR2: Domain-Based Struct Grouping**
- Classify structs into semantic domains (scoring, thresholds, detection, output, etc.)
- Use enhanced pattern matching beyond simple keyword detection
- Support language-specific patterns (Rust configs, Python classes, TS interfaces)
- Handle nested/hierarchical relationships between structs

**FR3: Size-Aware Module Validation**
- Reject recommendations with >40 methods per module
- Warn for 20-40 methods (borderline acceptable)
- Ensure minimum 5 methods per module (avoid over-fragmentation)
- Recursively split oversized groups into sub-modules

**FR4: Multi-Strategy Splitting**
- Primary: Struct ownership + domain grouping (for files with multiple structs)
- Secondary: Function call graph clustering (for procedural code)
- Fallback: Enhanced method-name patterns (when struct info unavailable)
- Combine strategies with weighted scoring

**FR5: Enhanced Module Split Metadata**
- Track structs AND methods being moved (not just methods)
- Calculate cohesion score (0.0-1.0) based on internal vs external calls
- Identify dependencies between proposed modules
- Provide implementation priority/order guidance

### Non-Functional Requirements

**NFR1: Performance**
- Struct ownership analysis must add <10% to god object detection time
- Cache parsed struct information to avoid re-parsing
- Optimize domain classification with trie-based pattern matching

**NFR2: Accuracy**
- Reduce false recommendation rate by >50%
- Ensure all recommendations have <40 methods per module
- Achieve >0.7 average cohesion score for recommended splits

**NFR3: Maintainability**
- Separate struct analysis logic from output formatting
- Use pure functions for domain classification
- Make domain patterns configurable via config file
- Support adding new languages without core logic changes

## Acceptance Criteria

### AC1: Struct Ownership Tracking
- [ ] Parse Rust impl blocks to map methods to structs
- [ ] Track standalone functions separately
- [ ] Support Python class methods
- [ ] Support TypeScript/JavaScript class methods
- [ ] Store struct ownership in ModuleSplit metadata

### AC2: Enhanced Domain Classification
- [ ] Implement `classify_struct_domain_enhanced()` with 10+ patterns
- [ ] Detect scoring/weights domain (scoring, weight, multiplier, factor)
- [ ] Detect thresholds domain (threshold, limit, bound, validation)
- [ ] Detect detection domain (detection, detector, checker, analyzer)
- [ ] Detect output domain (display, output, format, render)
- [ ] Detect language domain (language, rust, python, javascript)
- [ ] Detect error handling domain (error, pattern, severity)
- [ ] Detect context domain (context, rule, matcher)
- [ ] Support custom domain patterns via config
- [ ] Classify unknown structs as "utilities" or "core" appropriately

### AC3: Size Validation
- [ ] Implement `validate_and_refine_splits()` function
- [ ] Reject recommendations with >40 methods
- [ ] Warn for 20-40 methods with specific message
- [ ] Filter out groups with <5 methods
- [ ] Recursively split oversized domains into sub-modules
- [ ] Ensure no recommended split exceeds thresholds

### AC4: Enhanced ModuleSplit Structure
- [ ] Add `structs_to_move: Vec<String>` field
- [ ] Add `method_count: usize` field
- [ ] Add `cohesion_score: Option<f64>` field
- [ ] Add `dependencies_in: Vec<String>` field (what this module needs)
- [ ] Add `dependencies_out: Vec<String>` field (what depends on this)
- [ ] Add `warning: Option<String>` field for quality issues
- [ ] Add `priority: Priority` enum (High/Medium/Low) for implementation order

### AC5: Improved Output Format
- [ ] Show structs being moved, not just methods
- [ ] Display cohesion scores for each recommendation
- [ ] Show inter-module dependencies
- [ ] Provide phased implementation guidance (what to split first)
- [ ] Include "keep in original file" recommendations for coordinator code
- [ ] Show expected outcomes (lines reduced, testability improved, etc.)

### AC6: config.rs Test Case
- [ ] Recommend 6-8 modules instead of 2
- [ ] Each recommended module has ≤40 methods
- [ ] Modules align with actual domains (scoring, thresholds, detection, languages, display, error_handling)
- [ ] Provide accurate line count estimates per module
- [ ] Show which structs belong to which module
- [ ] Include implementation guidance for migration

### AC7: Integration Tests
- [ ] Test on config.rs (Rust multi-struct config file)
- [ ] Test on Python class-based god object
- [ ] Test on TypeScript class-based god object
- [ ] Test on procedural code (fallback to call graph clustering)
- [ ] Verify no recommendations exceed 40 methods
- [ ] Verify all recommendations have >5 methods

## Technical Details

### Implementation Approach

**Phase 1: Struct Ownership Analysis**

1. **Parse struct definitions and impl blocks** (Rust-specific first):
   ```rust
   pub struct StructOwnershipAnalyzer {
       struct_to_methods: HashMap<String, Vec<String>>,
       standalone_functions: Vec<String>,
       struct_locations: HashMap<String, (usize, usize)>, // line spans
   }

   impl StructOwnershipAnalyzer {
       pub fn analyze_file(parsed: &syn::File) -> Self {
           // Walk AST to find ItemStruct and ItemImpl
           // Map impl blocks to struct names
           // Track standalone functions
       }

       pub fn get_struct_methods(&self, struct_name: &str) -> &[String] {
           // Return methods for a specific struct
       }
   }
   ```

2. **Extend for Python and TypeScript**:
   - Python: Parse class definitions, track methods via indentation/AST
   - TypeScript: Parse class/interface definitions, track methods

**Phase 2: Domain-Based Grouping**

1. **Enhanced domain classifier**:
   ```rust
   pub fn classify_struct_domain_enhanced(
       struct_name: &str,
       methods: &[String],
       file_context: &str,
   ) -> String {
       let lower = struct_name.to_lowercase();

       // Specific patterns first (most to least specific)
       if matches_scoring_pattern(&lower) {
           return "scoring".to_string();
       }
       if matches_threshold_pattern(&lower) {
           return "thresholds".to_string();
       }
       // ... more patterns

       // Analyze method names as secondary signal
       let method_pattern = infer_domain_from_methods(methods);
       if !method_pattern.is_empty() {
           return method_pattern;
       }

       // File context as tertiary signal
       if file_context.contains("config") {
           "core".to_string()
       } else {
           "utilities".to_string()
       }
   }

   fn matches_scoring_pattern(name: &str) -> bool {
       name.contains("scoring")
           || name.contains("weight")
           || name.contains("multiplier")
           || name.contains("factor")
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
               "", // file context
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

1. **Validation logic**:
   ```rust
   pub fn validate_and_refine_splits(
       splits: Vec<ModuleSplit>,
       thresholds: &GodObjectThresholds,
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
               if method_count <= thresholds.max_methods {
                   return vec![split];
               }

               // Borderline - warn but allow
               if method_count <= thresholds.max_methods * 2 {
                   return vec![ModuleSplit {
                       warning: Some(format!(
                           "{} methods is high - consider further splitting",
                           method_count
                       )),
                       priority: Priority::Medium,
                       ..split
                   }];
               }

               // Too large - recursively split
               split_module_further(split, thresholds)
           })
           .collect()
   }

   fn split_module_further(
       split: ModuleSplit,
       thresholds: &GodObjectThresholds,
   ) -> Vec<ModuleSplit> {
       // Sub-divide by analyzing struct relationships
       // or by function call patterns within the oversized group
   }
   ```

**Phase 4: Enhanced Output Format**

1. **New output structure** (in formatter.rs):
   ```rust
   // Show phased refactoring strategy
   writeln!(output, "RECOMMENDED REFACTORING STRATEGY:")?;
   writeln!(output)?;
   writeln!(output, "Phase 1: Split by domain ({} focused modules)", splits.len())?;

   for (idx, split) in splits.iter().enumerate() {
       // Show module name
       writeln!(output, "├─ 📦 {}", split.suggested_name)?;

       // Show structs being moved
       writeln!(output, "│   → Structs: {} ({} structs)",
           split.structs_to_move.join(", "),
           split.structs_to_move.len())?;

       // Show method count and line estimate
       writeln!(output, "│   → Methods: {} functions (~{} lines)",
           split.method_count, split.estimated_lines)?;

       // Show cohesion score if available
       if let Some(cohesion) = split.cohesion_score {
           let quality = if cohesion > 0.8 { "Excellent" }
                        else if cohesion > 0.7 { "Good" }
                        else { "Moderate" };
           writeln!(output, "│   → Cohesion: {:.2} ({})", cohesion, quality)?;
       }

       // Show dependencies
       if !split.dependencies_in.is_empty() {
           writeln!(output, "│   → Dependencies: Uses {}",
               split.dependencies_in.join(", "))?;
       }

       // Show priority
       let stars = match split.priority {
           Priority::High => "⭐⭐⭐ High value, clear boundaries",
           Priority::Medium => "⭐⭐ Medium value",
           Priority::Low => "⭐ Low priority",
       };
       writeln!(output, "│   → Priority: {}", stars)?;

       // Show warnings
       if let Some(warning) = &split.warning {
           writeln!(output, "│   ⚠️  {}", warning)?;
       }
       writeln!(output, "│")?;
   }

   // Show what to keep in original file
   writeln!(output, "Phase 2: Keep in {}/mod.rs (coordinator)", module_name)?;
   writeln!(output, "├─ Main configuration container struct")?;
   writeln!(output, "├─ Global accessor functions: get_config(), set_config()")?;
   writeln!(output, "├─ Config loading and validation logic")?;
   writeln!(output, "└─ Estimated: ~200 lines (down from {})", original_lines)?;
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
// Enhanced ModuleSplit with full metadata
pub struct ModuleSplit {
    pub suggested_name: String,
    pub methods_to_move: Vec<String>,
    pub structs_to_move: Vec<String>,        // NEW
    pub responsibility: String,
    pub estimated_lines: usize,
    pub method_count: usize,                 // NEW
    pub cohesion_score: Option<f64>,         // NEW
    pub dependencies_in: Vec<String>,        // NEW
    pub dependencies_out: Vec<String>,       // NEW
    pub warning: Option<String>,             // NEW
    pub priority: Priority,                  // NEW
}

pub enum Priority {
    High,    // Self-contained, clear boundaries, high value
    Medium,  // Some dependencies, moderate complexity
    Low,     // Complex dependencies, lower priority
}

// Struct with method ownership
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
    file_context: &str,
) -> String;

// Struct-based splitting (PRIMARY strategy)
pub fn suggest_splits_by_struct_grouping(
    structs: &[StructMetrics],
    ownership: &StructOwnershipAnalyzer,
    thresholds: &GodObjectThresholds,
) -> Vec<ModuleSplit>;

// Size validation
pub fn validate_and_refine_splits(
    splits: Vec<ModuleSplit>,
    thresholds: &GodObjectThresholds,
) -> Vec<ModuleSplit>;

// Priority assignment based on cohesion and dependencies
pub fn assign_split_priorities(splits: &mut [ModuleSplit], call_graph: &CallGraph);
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
            fn new() -> Self { ... }
            fn get_value(&self) -> &str { ... }
        }

        fn standalone_helper() { ... }
    "#;

    let analyzer = analyze_struct_ownership(&syn::parse_str(code).unwrap());
    assert_eq!(analyzer.get_struct_methods("Config"), &["new", "get_value"]);
    assert_eq!(analyzer.standalone_functions(), &["standalone_helper"]);
}

#[test]
fn test_multiple_impl_blocks() {
    // Test structs with multiple impl blocks
}

#[test]
fn test_trait_impl_exclusion() {
    // Ensure trait impls aren't counted as "owned" methods
}
```

**Domain Classification** (`tests/domain_classification_tests.rs`):
```rust
#[test]
fn test_classify_scoring_structs() {
    assert_eq!(classify_struct_domain_enhanced("ScoringWeights", &[], ""), "scoring");
    assert_eq!(classify_struct_domain_enhanced("RoleMultipliers", &[], ""), "scoring");
}

#[test]
fn test_classify_threshold_structs() {
    assert_eq!(classify_struct_domain_enhanced("ThresholdsConfig", &[], ""), "thresholds");
    assert_eq!(classify_struct_domain_enhanced("ValidationThresholds", &[], ""), "thresholds");
}

#[test]
fn test_ambiguous_struct_uses_methods() {
    // When struct name is ambiguous, use method names as signal
    let methods = vec!["detect_pattern".to_string(), "check_violation".to_string()];
    assert_eq!(classify_struct_domain_enhanced("Analyzer", &methods, ""), "detection");
}
```

**Size Validation** (`tests/split_validation_tests.rs`):
```rust
#[test]
fn test_reject_oversized_splits() {
    let split = ModuleSplit {
        method_count: 60,
        ..Default::default()
    };

    let thresholds = GodObjectThresholds { max_methods: 20, ..Default::default() };
    let validated = validate_and_refine_splits(vec![split], &thresholds);

    // Should be split further or rejected
    assert!(validated.iter().all(|s| s.method_count <= 40));
}

#[test]
fn test_filter_undersized_splits() {
    let split = ModuleSplit {
        method_count: 3,
        ..Default::default()
    };

    let validated = validate_and_refine_splits(vec![split], &GodObjectThresholds::default());
    assert_eq!(validated.len(), 0); // Too small, filtered out
}
```

### Integration Tests

**config.rs Test Case** (`tests/god_object_config_rs_test.rs`):
```rust
#[test]
fn test_config_rs_recommendation_quality() {
    // Run god object detection on actual config.rs
    let analysis = analyze_file_for_god_objects("src/config.rs");

    let splits = &analysis.recommended_splits;

    // Should recommend 6-8 modules, not 2
    assert!(splits.len() >= 6 && splits.len() <= 8,
        "Expected 6-8 modules, got {}", splits.len());

    // No module should exceed 40 methods
    for split in splits {
        assert!(split.method_count <= 40,
            "Module {} has {} methods (max 40)",
            split.suggested_name, split.method_count);
    }

    // Should have a scoring module
    assert!(splits.iter().any(|s| s.responsibility == "scoring"));

    // Should have a thresholds module
    assert!(splits.iter().any(|s| s.responsibility == "thresholds"));

    // Each module should have reasonable line estimates
    for split in splits {
        let lines_per_method = split.estimated_lines / split.method_count;
        assert!(lines_per_method >= 10 && lines_per_method <= 50,
            "Unrealistic line estimate for {}", split.suggested_name);
    }
}
```

**Python Class Test** (`tests/python_god_class_test.rs`):
```rust
#[test]
fn test_python_class_splitting() {
    let python_code = r#"
class UserManager:
    def create_user(self): ...
    def delete_user(self): ...
    def get_user(self): ...
    def validate_email(self): ...
    # ... 30 more methods
    "#;

    // Test that Python classes also get struct-based splitting
}
```

### Performance Tests

```rust
#[test]
fn test_struct_ownership_performance() {
    let large_file = generate_file_with_n_structs(100);

    let start = Instant::now();
    let _analyzer = analyze_struct_ownership(&large_file);
    let duration = start.elapsed();

    // Should complete in <100ms for 100 structs
    assert!(duration.as_millis() < 100);
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
   - Comment the multi-strategy approach (struct > call graph > method names)
   - Explain how cohesion scores are calculated
   - Document the recursive splitting logic for oversized groups

### User Documentation

1. **CLAUDE.md updates**:
   ```markdown
   ## God Object Recommendations

   Debtmap uses struct-ownership analysis to recommend module splits:

   - **Primary**: Groups structs by domain semantics (scoring, thresholds, etc.)
   - **Validation**: Ensures 5-40 methods per recommended module
   - **Metadata**: Shows cohesion scores, dependencies, implementation priorities
   ```

2. **Example output** in documentation showing the enhanced format

### Architecture Documentation

Update ARCHITECTURE.md with:
- New struct ownership analysis subsystem
- Multi-strategy splitting decision flow
- Size validation rules and thresholds

## Implementation Notes

### Pattern Matching Best Practices

1. **Order patterns from specific to general**:
   ```rust
   // Good: Check specific patterns first
   if name.contains("scoring_weight") { ... }
   else if name.contains("scoring") { ... }
   else if name.contains("weight") { ... }

   // Bad: Generic patterns first
   if name.contains("weight") { ... }  // Would match "scoring_weight" too early
   ```

2. **Use multiple signals** when struct name is ambiguous:
   - Primary: struct name patterns
   - Secondary: method name patterns
   - Tertiary: file context

3. **Make patterns configurable** for easy extension:
   ```toml
   [god_object_detection.domain_patterns]
   scoring = ["scoring", "weight", "multiplier", "factor"]
   thresholds = ["threshold", "limit", "bound", "validation"]
   ```

### Gotchas

1. **Trait implementations** shouldn't count as owned methods:
   ```rust
   impl Display for Config {
       fn fmt(&self, f: &mut Formatter) -> Result { ... }
   }
   // Don't count fmt() as a Config method for splitting purposes
   ```

2. **Generic impl blocks** need special handling:
   ```rust
   impl<T> Container<T> {
       fn new() -> Self { ... }
   }
   // Associate with Container, not generic T
   ```

3. **Line estimation** should account for:
   - Struct definitions
   - Default implementations
   - Helper functions
   - Import statements

### Testing Edge Cases

- Files with no structs (procedural code)
- Files with one giant struct (use method-based fallback)
- Files with many tiny structs (ensure minimum group size)
- Circular dependencies between recommended modules
- Structs with trait implementations (filter out)

## Migration and Compatibility

### Breaking Changes

None - this is purely an improvement to recommendation quality. The output format changes but doesn't affect the detection logic or existing workflows.

### Backward Compatibility

- Keep existing `recommend_module_splits()` signature, add optional parameter
- Maintain fallback to method-name-based grouping when struct info unavailable
- Ensure output is still readable without new metadata fields

### Deprecation Path

1. **v0.3.0**: Introduce struct-based splitting (this spec)
2. **v0.3.1**: Add deprecation warning for purely method-name-based approach
3. **v0.4.0**: Make struct-based splitting the default, remove old logic

### Configuration

Add new config section:
```toml
[god_object_detection]
use_struct_ownership = true  # Enable new approach
max_methods_per_module = 40  # Size validation threshold
min_methods_per_module = 5   # Avoid over-fragmentation
enable_cohesion_scoring = true  # Calculate and show cohesion

[god_object_detection.domain_patterns]
# Customizable domain classification patterns
scoring = ["scoring", "weight", "multiplier"]
thresholds = ["threshold", "limit", "validation"]
# ... more patterns
```

## Success Metrics

### Quantitative Metrics

1. **Recommendation Quality**:
   - ✅ 0% of recommendations exceed 40 methods (currently: 50%)
   - ✅ Average module size: 15-25 methods (currently: 86 methods)
   - ✅ Number of recommended modules: 6-8 for config.rs (currently: 2)

2. **Accuracy**:
   - ✅ >80% of recommendations align with actual domain boundaries
   - ✅ <5% of recommendations need further splitting
   - ✅ Average cohesion score >0.7

3. **Performance**:
   - ✅ <10% increase in god object detection time
   - ✅ <100ms for struct ownership analysis on 100-struct file

### Qualitative Metrics

1. **User Trust**:
   - Recommendations look sensible and actionable
   - Users can follow recommendations without manual validation
   - Domain separation is obvious and intuitive

2. **Code Quality**:
   - Recommended splits actually improve maintainability
   - Dependencies between modules are minimal
   - Each module has clear, single responsibility

## Future Enhancements

### Post-v0.3.0 Improvements

1. **Call Graph Integration** (v0.4.0):
   - Use actual call graph data to validate groupings
   - Calculate cohesion based on internal vs external calls
   - Detect circular dependencies in recommendations

2. **Machine Learning Patterns** (v0.5.0):
   - Learn domain patterns from well-organized codebases
   - Auto-detect custom domain patterns per project
   - Suggest better names based on common patterns

3. **Interactive Refinement** (v0.6.0):
   - Allow users to adjust recommendations interactively
   - Learn from user feedback to improve future recommendations
   - Generate migration scripts automatically

4. **Cross-Language Consistency** (v0.4.0):
   - Ensure Python, JavaScript, TypeScript use same quality standards
   - Adapt patterns for language-specific idioms
   - Support language-specific module systems (Python packages, JS ES modules)
