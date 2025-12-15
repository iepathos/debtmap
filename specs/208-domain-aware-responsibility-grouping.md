---
number: 208
title: Domain-Aware Responsibility Grouping
category: optimization
priority: high
status: draft
dependencies: [206, 207]
created: 2025-12-15
---

# Specification 208: Domain-Aware Responsibility Grouping

**Category**: optimization
**Priority**: high (P0)
**Status**: draft
**Dependencies**: Spec 206 (Cohesion Gate), Spec 207 (LOC Calculation Fix)

## Context

The current God Object detection uses `group_methods_by_responsibility()` which classifies methods based on **naming patterns** (prefixes like `get_*`, `validate_*`, `parse_*`). This produces implementation-focused categories, not domain responsibilities.

### Current Problem

For a struct like `CrossModuleTracker`:
```
Methods: get_module_calls, is_public_api, resolve_module_call, infer_module_path
Current grouping:
  - "Data Access": [get_module_calls]
  - "Validation": [is_public_api]
  - "Filtering": [resolve_module_call]
  - "unclassified": [infer_module_path]
Result: 4 "responsibilities" (false positive for God Object)
```

### Desired Behavior

```
Methods: get_module_calls, is_public_api, resolve_module_call, infer_module_path
Domain-aware grouping:
  - "Module Tracking": [get_module_calls, is_public_api, resolve_module_call, infer_module_path]
Result: 1 responsibility (correctly NOT a God Object)
```

## Objective

Replace method-prefix-based responsibility classification with domain-aware grouping that:
1. Uses struct name and field names to identify the primary domain
2. Groups methods by their actual domain concern, not implementation pattern
3. Distinguishes between truly separate domains vs. different operations on the same domain

## Requirements

### Functional Requirements

1. **Domain Extraction**: Extract domain keywords from struct name, field names, and method signatures
2. **Domain Grouping**: Group methods by their domain alignment, not prefix pattern
3. **Conflict Resolution**: Handle methods that could belong to multiple domains
4. **Fallback Behavior**: Use existing prefix-based grouping when domain cannot be determined

### Non-Functional Requirements

- Must not significantly impact analysis performance
- Must maintain backwards compatibility with existing output format
- Must produce deterministic results (same input → same output)

## Acceptance Criteria

- [ ] Methods containing struct domain keywords are grouped under primary domain
- [ ] `CrossModuleTracker` with module-related methods produces 1-2 domain groups, not 6+
- [ ] True God Objects (e.g., `ApplicationManager` with parse/render/validate/send methods) still produce multiple domain groups
- [ ] Responsibility count used in scoring reflects actual domain separation
- [ ] Existing tests continue to pass
- [ ] New unit tests validate domain extraction and grouping

## Technical Details

### Implementation Approach

#### 1. Domain Context Extraction

```rust
pub struct DomainContext {
    /// Primary domain keywords from struct name
    pub primary_keywords: Vec<String>,
    /// Secondary keywords from field names/types
    pub secondary_keywords: Vec<String>,
    /// Domain suffixes detected (Tracker, Manager, Builder, etc.)
    pub domain_suffix: Option<String>,
}

pub fn extract_domain_context(
    struct_name: &str,
    field_names: &[String],
    field_types: &[String],
) -> DomainContext {
    // Extract keywords from struct name (already implemented in classifier.rs)
    let primary_keywords = extract_domain_keywords(struct_name);

    // Extract keywords from field names and types
    let secondary_keywords = field_names.iter()
        .chain(field_types.iter())
        .flat_map(|name| extract_domain_keywords(name))
        .collect();

    // Detect domain suffix
    let domain_suffix = detect_domain_suffix(struct_name);

    DomainContext {
        primary_keywords,
        secondary_keywords,
        domain_suffix,
    }
}
```

#### 2. Domain-Aware Grouping

```rust
pub fn group_methods_by_domain(
    methods: &[String],
    context: &DomainContext,
) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for method in methods {
        let domain = infer_method_domain(method, context);
        groups.entry(domain).or_default().push(method.clone());
    }

    groups
}

fn infer_method_domain(method: &str, context: &DomainContext) -> String {
    let method_keywords = extract_domain_keywords(method);

    // Check if method aligns with primary domain
    let matches_primary = context.primary_keywords.iter()
        .any(|pk| method.to_lowercase().contains(pk) || method_keywords.contains(pk));

    if matches_primary {
        // Return the primary domain name
        return context.primary_keywords.join("_");
    }

    // Check secondary domain alignment
    let matching_secondary: Vec<_> = context.secondary_keywords.iter()
        .filter(|sk| method.to_lowercase().contains(*sk) || method_keywords.contains(*sk))
        .collect();

    if !matching_secondary.is_empty() {
        return matching_secondary[0].clone();
    }

    // Fallback to behavioral classification for truly unrelated methods
    infer_responsibility_with_confidence(method, None)
        .category
        .unwrap_or_else(|| "unclassified".to_string())
}
```

#### 3. Integration with God Object Detection

Update `analyze_single_struct` to use domain-aware grouping:

```rust
fn analyze_single_struct(...) -> Option<GodObjectAnalysis> {
    // ... existing code ...

    // NEW: Build domain context
    let domain_context = extract_domain_context(
        &type_analysis.name,
        &type_analysis.fields,
        &type_analysis.field_types,
    );

    // NEW: Use domain-aware grouping
    let responsibility_groups = group_methods_by_domain(method_names, &domain_context);
    let responsibility_count = responsibility_groups.len();

    // ... rest of analysis ...
}
```

### Data Structure Changes

Add to `TypeAnalysis` in `ast_visitor.rs`:
```rust
pub struct TypeAnalysis {
    // ... existing fields ...
    pub fields: Vec<String>,      // Field names
    pub field_types: Vec<String>, // Field type names
}
```

### Domain Detection Heuristics

| Pattern | Domain | Example |
|---------|--------|---------|
| Methods with struct name keywords | Primary domain | `ModuleTracker::get_module()` → "module" |
| Methods with field name keywords | Field domain | Having `cache: Cache` + `get_cached()` → "cache" |
| Behavioral prefixes (parse_, render_, etc.) | Behavioral domain | `parse_json()` when no domain match → "Parsing" |
| No clear domain | Unclassified | Generic utility methods |

## Dependencies

- **Prerequisites**:
  - Spec 206: Cohesion Gate (uses domain keywords, must align)
  - Spec 207: LOC Calculation (accurate metrics for scoring)
- **Affected Components**:
  - `classifier.rs`: Add domain context extraction and grouping
  - `detector.rs`: Use domain-aware grouping in analysis
  - `ast_visitor.rs`: Add field name/type extraction

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_domain_context_extraction() {
    let context = extract_domain_context(
        "ModuleTracker",
        &["modules".into(), "boundaries".into()],
        &["HashMap".into(), "Vec".into()],
    );
    assert!(context.primary_keywords.contains(&"module".to_string()));
    assert_eq!(context.domain_suffix, Some("tracker".to_string()));
}

#[test]
fn test_domain_grouping_cohesive_struct() {
    let context = DomainContext {
        primary_keywords: vec!["module".into()],
        secondary_keywords: vec!["boundary".into()],
        domain_suffix: Some("tracker".into()),
    };

    let methods = vec![
        "get_modules".into(),
        "track_module".into(),
        "resolve_boundary".into(),
        "new".into(),
    ];

    let groups = group_methods_by_domain(&methods, &context);

    // Should have 1-2 groups, not 4
    assert!(groups.len() <= 2, "Cohesive methods should group together");
}

#[test]
fn test_domain_grouping_god_object() {
    let context = DomainContext {
        primary_keywords: vec!["application".into()],
        secondary_keywords: vec![],
        domain_suffix: Some("manager".into()),
    };

    let methods = vec![
        "parse_json".into(),
        "render_html".into(),
        "validate_email".into(),
        "send_notification".into(),
    ];

    let groups = group_methods_by_domain(&methods, &context);

    // Should have 4 groups (none match "application")
    assert!(groups.len() >= 4, "Unrelated methods should stay separate");
}
```

### Integration Tests

```rust
#[test]
fn test_cross_module_tracker_domain_grouping() {
    let content = r#"
        pub struct CrossModuleTracker {
            modules: HashMap<String, Module>,
            calls: Vec<CrossModuleCall>,
        }

        impl CrossModuleTracker {
            pub fn get_module_calls(&self) -> Vec<Call> { vec![] }
            pub fn resolve_module_call(&self, path: &str) -> Option<Call> { None }
            pub fn is_module_public(&self, id: &ModuleId) -> bool { true }
            pub fn track_module(&mut self, m: Module) {}
        }
    "#;

    let analyses = analyze_content(content);

    // With domain-aware grouping, responsibility_count should be low
    if let Some(analysis) = analyses.first() {
        assert!(
            analysis.responsibility_count <= 2,
            "CrossModuleTracker should have ≤2 domain responsibilities, got {}",
            analysis.responsibility_count
        );
    }
}
```

## Documentation Requirements

- **Code Documentation**: Document domain extraction heuristics in classifier.rs
- **Architecture Updates**: Update ARCHITECTURE.md with domain-aware grouping explanation

## Implementation Notes

1. **Preserve Existing Behavior**: Keep `group_methods_by_responsibility()` for fallback and comparison
2. **Gradual Rollout**: Add feature flag to enable/disable domain-aware grouping initially
3. **Logging**: Add debug logging for domain extraction decisions
4. **Performance**: Cache domain context extraction per struct

## Migration and Compatibility

- Existing responsibility names in output may change (from prefix-based to domain-based)
- Old reports comparing responsibility counts will see different numbers
- Consider adding a `--legacy-grouping` flag for backwards compatibility

## Estimated Effort

- Implementation: ~4 hours
- Testing: ~2 hours
- Documentation: ~1 hour
- Total: ~7 hours
