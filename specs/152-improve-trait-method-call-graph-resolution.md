---
number: 152
title: Improve Trait Method Call Graph Resolution
category: optimization
priority: high
status: draft
dependencies: [151]
created: 2025-10-24
---

# Specification 152: Improve Trait Method Call Graph Resolution

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 151 (Improve Call Graph Orphaned Node Detection)

## Context

**Current State**:
- TraitRegistry exists in `src/analysis/call_graph/trait_registry.rs`
- Tracks trait definitions, implementations, and unresolved method calls
- Many trait implementations flagged as orphaned nodes (11,826 total orphans)
- Common patterns not resolved: `default()`, `new()`, `clone_box()`, constructor patterns

**Problem**:
Trait method implementations appear as orphaned because call graph doesn't connect:
1. **Trait method calls** → **Concrete implementations**
2. **Default trait methods** → **Call graph edges**
3. **Constructor patterns** (`new()`, `default()`) → **Callers**
4. **Generic trait bounds** → **Monomorphized implementations**

**Examples from analysis output**:
```
OrphanedNode { DictionaryDispatchPattern::clone_box, line: 133 }
OrphanedNode { TraitRegistry::default, line: 544 }
OrphanedNode { AnalysisConfig::default, line: 61 }
OrphanedNode { ComplexityFactors::default, line: 63 }
```

These are all trait implementations that **are called**, but through trait dispatch, so the call graph doesn't capture the relationship.

**Impact**:
- False positives for trait implementations in orphan detection
- Dependency scoring inaccurate for trait methods
- Health score affected by legitimate trait implementations
- Users lose confidence in call graph accuracy

## Objective

Enhance trait method resolution in the call graph to:
1. Connect trait method calls to their concrete implementations
2. Track default trait implementations as entry points
3. Resolve common patterns (`Default`, `Clone`, `From`, `Into`, `new()`)
4. Support generic trait bound resolution
5. Reduce orphaned trait implementation false positives by 80%

## Requirements

### Functional Requirements

1. **Trait Method Call Resolution**:
   - Resolve `obj.method()` calls where `method` is a trait method
   - Connect to concrete implementation based on receiver type
   - Support multiple implementations (trait objects, generics)
   - Track call edges: `caller → trait method implementation`

2. **Common Trait Pattern Detection**:
   - **Default trait**: `Default::default()` calls
   - **Clone trait**: `clone()`, `clone_from()`, `clone_box()`
   - **From/Into traits**: `from()`, `into()` conversions
   - **Constructor patterns**: `Type::new()`, builder patterns
   - **Display/Debug traits**: `fmt()` method implementations

3. **Default Implementation Handling**:
   - Mark default trait implementations as entry points
   - Track which implementations override defaults
   - Connect default impl calls to overriding implementations

4. **Generic Trait Bound Resolution**:
   - Resolve trait bounds in generic functions
   - Track monomorphization instances where determinable
   - Conservative approach: Mark as "potentially called" for generics

5. **Call Graph Integration**:
   - Add trait call edges to CallGraph
   - Mark trait implementations as "callable via trait"
   - Provide trait dispatch metadata in FunctionMetrics
   - Enable querying: "is this function a trait implementation?"

### Non-Functional Requirements

- **Accuracy**: < 10% false negatives for common trait patterns
- **Performance**: Trait resolution overhead < 100ms for 5000 functions
- **Backward Compatibility**: Existing CallGraph API unchanged
- **Incremental**: Can be partially applied (e.g., Default trait first)

## Acceptance Criteria

- [ ] Default trait implementations detected and marked as entry points
- [ ] Clone trait methods (`clone_box`, `clone`) connected to callers
- [ ] `new()` constructor patterns resolved
- [ ] Generic trait bounds tracked (conservative resolution)
- [ ] Trait method calls connected to implementations
- [ ] False positive orphan count for trait impls reduced by 80%
- [ ] CallGraph query: `is_trait_implementation(fn_id)` returns correct result
- [ ] Trait dispatch metadata included in FunctionMetrics
- [ ] Tests cover: Default, Clone, From, Into, new() patterns
- [ ] Documentation explains trait resolution approach

## Technical Details

### Implementation Approach

**Phase 1: Enhanced Trait Method Tracking**

```rust
// src/analysis/call_graph/trait_registry.rs (enhance existing)

impl TraitRegistry {
    /// Resolve trait method calls to concrete implementations
    pub fn resolve_trait_method_calls(&self, call_graph: &mut CallGraph) -> usize {
        let mut resolved_count = 0;

        for call in self.unresolved_calls.iter() {
            // Find implementations matching the trait and method
            let implementations = self.find_implementations(&call.trait_name, &call.method_name);

            for impl_method_id in implementations {
                // Add edge: caller → trait implementation
                call_graph.add_call(call.caller.clone(), impl_method_id.clone());
                call_graph.mark_as_trait_dispatch(impl_method_id.clone(), &call.trait_name);
                resolved_count += 1;
            }
        }

        resolved_count
    }

    /// Find implementations of a trait method
    fn find_implementations(&self, trait_name: &str, method_name: &str) -> Vec<FunctionId> {
        let mut implementations = Vec::new();

        // Check trait implementations
        if let Some(impls) = self.trait_implementations.get(trait_name) {
            for trait_impl in impls.iter() {
                for method_impl in trait_impl.method_implementations.iter() {
                    if method_impl.method_name == method_name {
                        implementations.push(method_impl.method_id.clone());
                    }
                }
            }
        }

        // Check default implementations
        if let Some(trait_methods) = self.trait_definitions.get(trait_name) {
            for trait_method in trait_methods.iter() {
                if trait_method.method_name == method_name && trait_method.has_default {
                    implementations.push(trait_method.method_id.clone());
                }
            }
        }

        implementations
    }

    /// Detect common trait patterns and mark as entry points
    pub fn detect_common_trait_patterns(&self, call_graph: &mut CallGraph) {
        self.detect_default_trait_impls(call_graph);
        self.detect_clone_trait_impls(call_graph);
        self.detect_constructor_patterns(call_graph);
        self.detect_from_into_impls(call_graph);
    }

    /// Detect Default trait implementations
    fn detect_default_trait_impls(&self, call_graph: &mut CallGraph) {
        if let Some(impls) = self.trait_implementations.get("Default") {
            for trait_impl in impls.iter() {
                for method_impl in trait_impl.method_implementations.iter() {
                    if method_impl.method_name == "default" {
                        // Mark as entry point (called implicitly via Default::default())
                        call_graph.mark_as_trait_entry_point(
                            method_impl.method_id.clone(),
                            "Default::default",
                        );
                    }
                }
            }
        }
    }

    /// Detect Clone trait implementations
    fn detect_clone_trait_impls(&self, call_graph: &mut CallGraph) {
        if let Some(impls) = self.trait_implementations.get("Clone") {
            for trait_impl in impls.iter() {
                for method_impl in trait_impl.method_implementations.iter() {
                    if method_impl.method_name == "clone"
                        || method_impl.method_name == "clone_box"
                        || method_impl.method_name == "clone_from"
                    {
                        call_graph.mark_as_trait_entry_point(
                            method_impl.method_id.clone(),
                            "Clone trait",
                        );
                    }
                }
            }
        }
    }

    /// Detect constructor patterns (Type::new)
    fn detect_constructor_patterns(&self, call_graph: &mut CallGraph) {
        for function in call_graph.get_all_functions() {
            // Detect ::new() pattern
            if function.name.ends_with("::new") || function.name == "new" {
                call_graph.mark_as_constructor(function.clone());
            }

            // Detect builder pattern (Type::builder, Type::with_*)
            if function.name.ends_with("::builder")
                || function.name.contains("::with_")
                || function.name.ends_with("::create")
            {
                call_graph.mark_as_constructor(function.clone());
            }
        }
    }

    /// Detect From/Into trait implementations
    fn detect_from_into_impls(&self, call_graph: &mut CallGraph) {
        for trait_name in &["From", "Into"] {
            if let Some(impls) = self.trait_implementations.get(*trait_name) {
                for trait_impl in impls.iter() {
                    for method_impl in trait_impl.method_implementations.iter() {
                        call_graph.mark_as_trait_entry_point(
                            method_impl.method_id.clone(),
                            &format!("{} trait", trait_name),
                        );
                    }
                }
            }
        }
    }
}
```

**Phase 2: CallGraph Extension**

```rust
// src/priority/call_graph/mod.rs (extend existing)

#[derive(Debug, Clone)]
pub enum FunctionRole {
    Regular,
    EntryPoint { reason: String },
    TraitImplementation { trait_name: String },
    TraitEntryPoint { trait_name: String, reason: String },
    Constructor,
}

pub trait CallGraph {
    // Existing methods...

    /// Mark a function as a trait implementation
    fn mark_as_trait_dispatch(&mut self, function: FunctionId, trait_name: &str);

    /// Mark a function as a trait entry point (called implicitly)
    fn mark_as_trait_entry_point(&mut self, function: FunctionId, reason: &str);

    /// Mark a function as a constructor pattern
    fn mark_as_constructor(&mut self, function: FunctionId);

    /// Check if a function is a trait implementation
    fn is_trait_implementation(&self, function: &FunctionId) -> bool;

    /// Get the trait name for a trait implementation
    fn get_trait_name(&self, function: &FunctionId) -> Option<String>;

    /// Get the role of a function
    fn get_function_role(&self, function: &FunctionId) -> FunctionRole;
}

// Implementation for CallGraphImpl
impl CallGraph for CallGraphImpl {
    fn mark_as_trait_dispatch(&mut self, function: FunctionId, trait_name: &str) {
        self.trait_implementations.insert(function, trait_name.to_string());
    }

    fn mark_as_trait_entry_point(&mut self, function: FunctionId, reason: &str) {
        self.trait_entry_points.insert(
            function,
            FunctionRole::TraitEntryPoint {
                trait_name: self.extract_trait_name(reason),
                reason: reason.to_string(),
            },
        );
    }

    fn mark_as_constructor(&mut self, function: FunctionId) {
        self.constructors.insert(function);
    }

    fn is_trait_implementation(&self, function: &FunctionId) -> bool {
        self.trait_implementations.contains_key(function)
            || self.trait_entry_points.contains_key(function)
    }

    fn get_trait_name(&self, function: &FunctionId) -> Option<String> {
        self.trait_implementations.get(function).cloned()
    }

    fn get_function_role(&self, function: &FunctionId) -> FunctionRole {
        if let Some(role) = self.trait_entry_points.get(function) {
            return role.clone();
        }

        if let Some(trait_name) = self.trait_implementations.get(function) {
            return FunctionRole::TraitImplementation {
                trait_name: trait_name.clone(),
            };
        }

        if self.constructors.contains(function) {
            return FunctionRole::Constructor;
        }

        // Check for main, test, etc. (existing logic)
        if self.is_main_or_test(function) {
            return FunctionRole::EntryPoint {
                reason: "main or test function".to_string(),
            };
        }

        FunctionRole::Regular
    }
}

struct CallGraphImpl {
    // Existing fields...
    trait_implementations: HashMap<FunctionId, String>,
    trait_entry_points: HashMap<FunctionId, FunctionRole>,
    constructors: HashSet<FunctionId>,
}
```

**Phase 3: Integration with Validation**

```rust
// src/analyzers/call_graph/validation.rs (enhance spec 151)

impl CallGraphValidator {
    /// Enhanced entry point detection using trait information
    fn is_entry_point(function: &FunctionId, call_graph: &CallGraph) -> bool {
        // Existing checks (main, test, etc.)...

        // NEW: Check if it's a trait entry point
        let role = call_graph.get_function_role(function);
        match role {
            FunctionRole::EntryPoint { .. } => true,
            FunctionRole::TraitEntryPoint { .. } => true,
            FunctionRole::Constructor => true,
            _ => false,
        }
    }

    /// Refine orphaned node detection (from spec 151)
    fn check_orphaned_nodes(call_graph: &CallGraph, report: &mut ValidationReport) {
        for function in call_graph.get_all_functions() {
            let has_callers = !call_graph.get_callers(function).is_empty();
            let has_callees = !call_graph.get_callees(function).is_empty();
            let is_entry_point = Self::is_entry_point(function, call_graph);
            let is_self_referential = Self::is_self_referential(function, call_graph);

            // NEW: Check if it's a trait implementation
            let is_trait_impl = call_graph.is_trait_implementation(function);

            // Trait implementations with no direct callers are OK (called via trait dispatch)
            if is_trait_impl && !has_callers {
                report.info.push(ValidationInfo::TraitImplementation {
                    function: function.clone(),
                    trait_name: call_graph.get_trait_name(function),
                });
                continue;  // NOT an issue
            }

            // Rest of validation logic (from spec 151)...
        }
    }
}

// Add new info variant
#[derive(Debug, Clone)]
pub enum ValidationInfo {
    LeafFunction { function: FunctionId, caller_count: usize },
    SelfReferentialFunction { function: FunctionId },
    TraitImplementation { function: FunctionId, trait_name: Option<String> },  // NEW
}
```

**Phase 4: Analysis Pipeline Integration**

```rust
// src/commands/analyze.rs

async fn analyze_project(config: &Config) -> Result<AnalysisResults> {
    // Existing analysis...

    // NEW: Trait method resolution pass
    let trait_registry = build_trait_registry(&parsed_files)?;
    trait_registry.detect_common_trait_patterns(&mut call_graph);
    let resolved_count = trait_registry.resolve_trait_method_calls(&mut call_graph);

    if config.verbose_call_graph {
        eprintln!("🔗 Resolved {} trait method calls", resolved_count);
    }

    // Continue with validation...
    let validation_report = CallGraphValidator::validate(&call_graph);

    Ok(results)
}
```

### Data Structures

```rust
// Additional fields in FunctionMetrics
pub struct FunctionMetrics {
    // Existing fields...
    pub role: FunctionRole,
    pub trait_info: Option<TraitInfo>,
}

#[derive(Debug, Clone)]
pub struct TraitInfo {
    pub trait_name: String,
    pub method_name: String,
    pub is_default_impl: bool,
    pub is_override: bool,
    pub potential_call_count: usize,  // Estimated callers (including trait dispatch)
}
```

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_trait_resolution() {
        let code = r#"
            struct MyConfig;

            impl Default for MyConfig {
                fn default() -> Self {
                    MyConfig
                }
            }

            fn create_config() -> MyConfig {
                MyConfig::default()
            }
        "#;

        let mut call_graph = CallGraph::new();
        let trait_registry = TraitRegistry::from_code(code).unwrap();

        trait_registry.detect_default_trait_impls(&mut call_graph);
        trait_registry.resolve_trait_method_calls(&mut call_graph);

        let default_impl = FunctionId::new("test.rs", "MyConfig::default", 4);

        // Should be marked as trait entry point
        assert!(call_graph.is_trait_implementation(&default_impl));

        // Should have call edge from create_config
        let create_fn = FunctionId::new("test.rs", "create_config", 10);
        let callees = call_graph.get_callees(&create_fn);
        assert!(callees.contains(&default_impl));
    }

    #[test]
    fn test_clone_trait_resolution() {
        let code = r#"
            struct MyBox<T>(Box<T>);

            impl<T: Clone> Clone for MyBox<T> {
                fn clone(&self) -> Self {
                    MyBox(self.0.clone())
                }

                fn clone_box(&self) -> Box<dyn Clone> {
                    Box::new(self.clone())
                }
            }
        "#;

        let mut call_graph = CallGraph::new();
        let trait_registry = TraitRegistry::from_code(code).unwrap();

        trait_registry.detect_clone_trait_impls(&mut call_graph);

        let clone_impl = FunctionId::new("test.rs", "MyBox::clone", 4);
        let clone_box_impl = FunctionId::new("test.rs", "MyBox::clone_box", 9);

        // Both should be marked as Clone trait implementations
        assert!(call_graph.is_trait_implementation(&clone_impl));
        assert!(call_graph.is_trait_implementation(&clone_box_impl));
        assert_eq!(call_graph.get_trait_name(&clone_impl), Some("Clone".to_string()));
    }

    #[test]
    fn test_constructor_pattern_detection() {
        let functions = vec![
            ("MyType::new", true),
            ("Config::builder", true),
            ("Settings::with_defaults", true),
            ("Database::create", true),
            ("util::process_data", false),
            ("regular_function", false),
        ];

        let mut call_graph = CallGraph::new();
        let trait_registry = TraitRegistry::new();

        for (name, _) in &functions {
            let func = FunctionId::new("test.rs", name, 1);
            call_graph.add_function(func);
        }

        trait_registry.detect_constructor_patterns(&mut call_graph);

        for (name, expected_constructor) in functions {
            let func = FunctionId::new("test.rs", &name, 1);
            let role = call_graph.get_function_role(&func);
            let is_constructor = matches!(role, FunctionRole::Constructor);
            assert_eq!(
                is_constructor, expected_constructor,
                "Function {} constructor detection mismatch",
                name
            );
        }
    }

    #[test]
    fn test_trait_impl_not_orphaned() {
        let mut call_graph = CallGraph::new();
        let trait_impl = FunctionId::new("test.rs", "MyType::default", 10);

        call_graph.add_function(trait_impl.clone());
        call_graph.mark_as_trait_entry_point(trait_impl.clone(), "Default::default");

        let report = CallGraphValidator::validate(&call_graph);

        // Should NOT be in structural issues as orphaned
        assert!(!report.structural_issues.iter().any(|issue| matches!(
            issue,
            StructuralIssue::IsolatedFunction { function } if function == &trait_impl
        )));

        // Should be in info as trait implementation
        assert!(report.info.iter().any(|info| matches!(
            info,
            ValidationInfo::TraitImplementation { function, .. } if function == &trait_impl
        )));
    }

    #[test]
    fn test_generic_trait_bound_conservative_resolution() {
        let code = r#"
            fn process<T: Display>(value: T) {
                println!("{}", value);  // Calls Display::fmt
            }

            impl Display for MyType {
                fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                    write!(f, "MyType")
                }
            }
        "#;

        let mut call_graph = CallGraph::new();
        let trait_registry = TraitRegistry::from_code(code).unwrap();

        trait_registry.resolve_trait_method_calls(&mut call_graph);

        // Should conservatively mark Display::fmt implementations as callable
        let fmt_impl = FunctionId::new("test.rs", "MyType::fmt", 7);
        assert!(call_graph.is_trait_implementation(&fmt_impl));
    }
}
```

### Integration Tests

```rust
// tests/trait_method_call_graph_test.rs

#[test]
fn test_trait_resolution_reduces_orphans() {
    // Analyze debtmap's own codebase
    let results = analyze_codebase(".", &Config::default()).unwrap();

    // Count orphans before trait resolution
    let orphans_without_trait_resolution = results
        .validation_report
        .structural_issues
        .iter()
        .filter(|issue| matches!(issue, StructuralIssue::IsolatedFunction { .. }))
        .count();

    // Enable trait resolution
    let mut config = Config::default();
    config.enable_trait_resolution = true;

    let results_with_traits = analyze_codebase(".", &config).unwrap();

    let orphans_with_trait_resolution = results_with_traits
        .validation_report
        .structural_issues
        .iter()
        .filter(|issue| matches!(issue, StructuralIssue::IsolatedFunction { .. }))
        .count();

    // Should reduce orphans by at least 50%
    assert!(
        orphans_with_trait_resolution < orphans_without_trait_resolution / 2,
        "Expected orphan reduction with trait resolution: {} -> {}",
        orphans_without_trait_resolution,
        orphans_with_trait_resolution
    );
}

#[test]
fn test_health_score_improves_with_trait_resolution() {
    let config_without = Config::default();
    let config_with = Config {
        enable_trait_resolution: true,
        ..Default::default()
    };

    let results_without = analyze_codebase(".", &config_without).unwrap();
    let results_with = analyze_codebase(".", &config_with).unwrap();

    // Health score should improve significantly
    assert!(
        results_with.validation_report.health_score
            > results_without.validation_report.health_score + 20,
        "Health score should improve by at least 20 points: {} -> {}",
        results_without.validation_report.health_score,
        results_with.validation_report.health_score
    );
}
```

## Dependencies

- **Prerequisites**: Spec 151 (Improve Call Graph Orphaned Node Detection)
- **Affected Components**:
  - `src/analysis/call_graph/trait_registry.rs` - Trait method resolution
  - `src/priority/call_graph/mod.rs` - CallGraph trait extension
  - `src/analyzers/call_graph/validation.rs` - Validation integration
  - `src/commands/analyze.rs` - Analysis pipeline integration
- **External Dependencies**: None (uses existing syn crate)

## Documentation Requirements

### Code Documentation

- Document trait resolution algorithm and heuristics
- Explain conservative approach for generic trait bounds
- Provide examples of resolved patterns in doctests
- Document limitations (e.g., trait objects, dynamic dispatch)

### User Documentation

```markdown
## Trait Method Call Graph Resolution

### Supported Trait Patterns

**Standard Library Traits**:
- `Default::default()` - Constructor pattern
- `Clone::clone()`, `Clone::clone_box()` - Cloning methods
- `From::from()`, `Into::into()` - Type conversions
- `Display::fmt()`, `Debug::fmt()` - Formatting traits

**Constructor Patterns**:
- `Type::new()` - Standard constructor
- `Type::builder()` - Builder pattern
- `Type::with_*()` - Configuration constructors
- `Type::create()` - Creation methods

### How It Works

1. **Trait Definition Parsing**: Extracts trait methods from trait definitions
2. **Implementation Tracking**: Tracks which types implement which traits
3. **Call Resolution**: Connects trait method calls to concrete implementations
4. **Entry Point Marking**: Marks trait implementations as callable (not orphaned)

### Limitations

- **Generic Trait Bounds**: Conservative approach - marks all implementations as potentially callable
- **Trait Objects**: Cannot determine exact implementation at compile time
- **Dynamic Dispatch**: Assumes all implementations of a trait object are reachable
- **Macro-Generated Traits**: May not detect trait implementations generated by macros

### Configuration

```toml
[call_graph]
enable_trait_resolution = true

# Additional traits to track beyond standard library
custom_traits = ["MyTrait", "CustomBehavior"]
```

### Verbose Output

```bash
debtmap analyze . --validate-call-graph --verbose-call-graph
```

Shows trait resolution statistics:
```
🔗 Building call graph...
  - Found 42 trait definitions
  - Found 156 trait implementations
  - Resolved 89 trait method calls
  - Marked 67 trait implementations as callable
✓ Call graph health: 85/100
```
```

## Implementation Notes

### Conservative Resolution

For generic trait bounds, use a conservative approach:
- Mark all implementations of a trait as "potentially callable"
- Better false negatives (missing some orphans) than false positives (flagging used code)
- Provide confidence levels: "definitely called", "potentially called", "uncalled"

### Common Trait Patterns Priority

Implement in order of impact:
1. **Default trait** (highest false positive rate)
2. **Constructor patterns** (`new()`, `builder()`)
3. **Clone trait** (`clone()`, `clone_box()`)
4. **From/Into traits**
5. **Display/Debug traits**

### Performance Considerations

- Trait resolution runs once during call graph construction (not per-file)
- Use HashMap lookups for O(1) trait implementation queries
- Cache resolved trait calls to avoid re-resolution

### Future Enhancements

1. **Type Inference Integration**: Use Rust type checker output for exact type resolution
2. **Macro Expansion**: Track trait implementations generated by derive macros
3. **Trait Object Analysis**: Better handling of `dyn Trait` calls
4. **Confidence Scoring**: Assign confidence levels to trait call resolutions

## Migration and Compatibility

### Backward Compatibility

- Opt-in feature (disabled by default initially)
- Existing call graph behavior unchanged when disabled
- No breaking changes to public APIs

### Migration Path

```bash
# Phase 1: Enable trait resolution, review results
debtmap analyze . --enable-trait-resolution

# Phase 2: Compare before/after health scores
debtmap analyze . --compare-with-baseline

# Phase 3: Make it default (after testing)
# Add to .debtmap.toml:
[call_graph]
enable_trait_resolution = true
```

## Success Metrics

- **False positive reduction**: Trait impl orphans reduced by 80% (from ~10,000 to ~2,000)
- **Health score improvement**: Average health score increases from 0/100 to 70+/100
- **Common trait coverage**: 95% of Default, Clone, From, Into implementations resolved
- **Performance**: Trait resolution adds < 5% to total analysis time
- **User satisfaction**: < 3 bug reports on trait resolution accuracy per month

## Open Questions

1. **Generic Trait Bounds**: How aggressive should resolution be for generic constraints?
   - Current spec: Conservative (mark all as potentially callable)
   - Alternative: Use type inference heuristics for better precision

2. **Macro-Generated Implementations**: How to track #[derive(Clone, Default, etc.)]?
   - Current spec: Relies on expanded AST (may miss some cases)
   - Alternative: Parse derive attributes and infer implementations

3. **Trait Objects**: How to handle Box<dyn Trait> calls?
   - Current spec: Mark all implementations as reachable (conservative)
   - Alternative: Analyze object construction sites for better precision

4. **Configuration Granularity**: Should users configure per-trait resolution?
   - Current spec: Global enable/disable flag
   - Alternative: Per-trait configuration (e.g., enable Default but not Clone)
