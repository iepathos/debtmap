---
number: 217
title: Trait-Mandated Method Detection for God Object Analysis
category: optimization
priority: medium
status: draft
dependencies: [209, 213]
created: 2025-12-16
---

# Specification 217: Trait-Mandated Method Detection for God Object Analysis

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 209 (Accessor/Boilerplate Detection), 213 (Pure Function Method Weighting)

## Context

### Current Problem

God object detection doesn't distinguish between:
1. **Trait-mandated methods** - Required by trait implementation, cannot be extracted
2. **Self-chosen methods** - Author's design choice, potentially extractable

This leads to misleading recommendations:

```
CallGraphExtractor (32 methods, 8 responsibilities):
Recommendation: "Extract 5 sub-orchestrators"

Reality: 18 methods are syn::Visit trait requirements - cannot be extracted
```

### Why This Matters

Methods exist on a struct for different reasons:

| Reason | Example | Extractable? |
|--------|---------|--------------|
| Trait requirement | `visit_expr()` for `syn::Visit` | No - trait mandates it |
| Trait requirement | `serialize()` for `serde::Serialize` | No |
| Trait requirement | `next()` for `Iterator` | No |
| Trait requirement | `poll()` for `Future` | No |
| Interface contract | `on_click()` handler | No - external caller expects it |
| Author's choice | `helper_function()` | Yes - can move elsewhere |

Recommending "extract methods" for trait-mandated methods is:
- **Impossible** - the trait requires them on this type
- **Misleading** - users lose trust in recommendations
- **Noisy** - drowns out actionable advice

### Impact

- `CallGraphExtractor`: 18 of 32 methods are `syn::Visit` trait methods
- `serde` derive structs: All `Serialize`/`Deserialize` methods are mandated
- Iterator adapters: `next()`, `size_hint()` are mandated
- Async handlers: `poll()` is mandated by `Future`

## Objective

Detect trait-mandated methods and adjust god object analysis to:
1. **Identify** which methods are trait requirements vs author choices
2. **Weight** trait-mandated methods lower (they're structural, not design debt)
3. **Recommend** actions only for extractable methods
4. **Display** the distinction clearly to users

## Requirements

### 1. Trait Implementation Detection

Detect trait implementations from AST:

```rust
/// Information about a trait implementation
pub struct TraitImplInfo {
    /// Trait being implemented (e.g., "syn::Visit", "Iterator")
    pub trait_path: String,
    /// Methods required by this trait
    pub required_methods: Vec<String>,
    /// Methods provided with default impl (overridden here)
    pub overridden_methods: Vec<String>,
}

/// Detect trait implementations for a struct
fn detect_trait_impls(extracted: &ExtractedFileData, struct_name: &str) -> Vec<TraitImplInfo> {
    extracted.impls
        .iter()
        .filter(|imp| imp.struct_name == struct_name && imp.trait_name.is_some())
        .map(|imp| TraitImplInfo {
            trait_path: imp.trait_name.clone().unwrap(),
            required_methods: imp.methods.iter().map(|m| m.name.clone()).collect(),
            overridden_methods: vec![], // Could detect via default impl analysis
        })
        .collect()
}
```

### 2. Known Trait Registry

Maintain registry of well-known traits and their method signatures:

```rust
/// Registry of known traits and their required methods
pub struct KnownTraitRegistry {
    traits: HashMap<String, KnownTrait>,
}

pub struct KnownTrait {
    /// Full trait path
    pub path: String,
    /// Alternative names (short form, re-exports)
    pub aliases: Vec<String>,
    /// Required method patterns
    pub method_patterns: Vec<MethodPattern>,
    /// Category for recommendation adjustment
    pub category: TraitCategory,
}

pub enum TraitCategory {
    /// AST/tree visitor (syn::Visit, etc.)
    Visitor,
    /// Serialization (serde, etc.)
    Serialization,
    /// Iterator/stream
    Iterator,
    /// Async runtime
    Async,
    /// Comparison/ordering
    Comparison,
    /// Error handling
    Error,
    /// Other standard traits
    Standard,
    /// Unknown/custom
    Custom,
}

/// Built-in known traits
fn default_known_traits() -> KnownTraitRegistry {
    let mut registry = KnownTraitRegistry::new();

    // Visitor patterns
    registry.add(KnownTrait {
        path: "syn::Visit".into(),
        aliases: vec!["syn::visit::Visit".into()],
        method_patterns: vec![MethodPattern::Prefix("visit_".into())],
        category: TraitCategory::Visitor,
    });

    // Serialization
    registry.add(KnownTrait {
        path: "serde::Serialize".into(),
        aliases: vec!["Serialize".into()],
        method_patterns: vec![MethodPattern::Exact("serialize".into())],
        category: TraitCategory::Serialization,
    });

    // Iterator
    registry.add(KnownTrait {
        path: "Iterator".into(),
        aliases: vec!["std::iter::Iterator".into()],
        method_patterns: vec![
            MethodPattern::Exact("next".into()),
            MethodPattern::Exact("size_hint".into()),
        ],
        category: TraitCategory::Iterator,
    });

    // Async
    registry.add(KnownTrait {
        path: "Future".into(),
        aliases: vec!["std::future::Future".into()],
        method_patterns: vec![MethodPattern::Exact("poll".into())],
        category: TraitCategory::Async,
    });

    // Standard traits
    registry.add(KnownTrait {
        path: "Default".into(),
        method_patterns: vec![MethodPattern::Exact("default".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "Drop".into(),
        method_patterns: vec![MethodPattern::Exact("drop".into())],
        category: TraitCategory::Standard,
    });

    registry.add(KnownTrait {
        path: "Clone".into(),
        method_patterns: vec![MethodPattern::Exact("clone".into())],
        category: TraitCategory::Standard,
    });

    // ... more traits

    registry
}
```

### 3. Method Classification

Classify each method as trait-mandated or self-chosen:

```rust
pub enum MethodOrigin {
    /// Required by trait implementation
    TraitMandated {
        trait_name: String,
        category: TraitCategory,
    },
    /// Author's design choice
    SelfChosen,
}

pub struct ClassifiedMethod {
    pub name: String,
    pub origin: MethodOrigin,
    /// Weight for god object scoring (0.0-1.0)
    pub weight: f64,
}

fn classify_method_origin(
    method_name: &str,
    trait_impls: &[TraitImplInfo],
    registry: &KnownTraitRegistry,
) -> MethodOrigin {
    for impl_info in trait_impls {
        if impl_info.required_methods.contains(&method_name.to_string()) {
            let category = registry
                .get(&impl_info.trait_path)
                .map(|t| t.category.clone())
                .unwrap_or(TraitCategory::Custom);

            return MethodOrigin::TraitMandated {
                trait_name: impl_info.trait_path.clone(),
                category,
            };
        }
    }
    MethodOrigin::SelfChosen
}
```

### 4. Adjusted Method Weighting

Apply different weights based on method origin:

```rust
fn calculate_method_weight(origin: &MethodOrigin) -> f64 {
    match origin {
        MethodOrigin::TraitMandated { category, .. } => match category {
            // Visitor methods are structural - many required by design
            TraitCategory::Visitor => 0.1,
            // Serialization is usually derived, low weight
            TraitCategory::Serialization => 0.1,
            // Iterator has few required methods
            TraitCategory::Iterator => 0.3,
            // Async typically just poll()
            TraitCategory::Async => 0.3,
            // Standard traits (Clone, Default, etc.)
            TraitCategory::Standard => 0.2,
            // Unknown traits - moderate weight
            TraitCategory::Custom => 0.4,
            _ => 0.3,
        },
        MethodOrigin::SelfChosen => 1.0,
    }
}
```

### 5. GodObjectAnalysis Extension

Add trait information to analysis:

```rust
/// Trait implementation summary for god object
pub struct TraitMethodSummary {
    /// Total trait-mandated methods
    pub mandated_count: usize,
    /// Breakdown by trait
    pub by_trait: HashMap<String, usize>,
    /// Weighted method count (after applying trait discounts)
    pub weighted_count: f64,
    /// Self-chosen (extractable) method count
    pub extractable_count: usize,
}

// Add to GodObjectAnalysis
pub struct GodObjectAnalysis {
    // ... existing fields ...

    /// Trait-mandated method analysis
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trait_method_summary: Option<TraitMethodSummary>,
}
```

### 6. Adjusted Recommendations

Generate recommendations based on extractable methods only:

```rust
fn create_god_object_recommendation_with_traits(
    god_analysis: &GodObjectAnalysis,
    trait_summary: &TraitMethodSummary,
) -> ActionableRecommendation {
    let extractable = trait_summary.extractable_count;
    let mandated = trait_summary.mandated_count;

    // Build trait info string
    let trait_info = if !trait_summary.by_trait.is_empty() {
        let traits: Vec<String> = trait_summary.by_trait
            .iter()
            .map(|(t, n)| format!("{} ({})", t, n))
            .collect();
        format!("Implements: {}", traits.join(", "))
    } else {
        String::new()
    };

    let primary_action = if extractable > 10 {
        format!(
            "Extract {} self-chosen methods into focused modules",
            extractable
        )
    } else if extractable > 5 {
        format!(
            "Consider grouping {} extractable methods by responsibility",
            extractable
        )
    } else if mandated > 20 {
        "Review trait implementations - consider if all are necessary".to_string()
    } else {
        "Well-structured despite method count - trait implementations drive size".to_string()
    };

    let rationale = format!(
        "{} of {} methods are trait-mandated (non-extractable). {}. \
        Focus refactoring on the {} self-chosen methods.",
        mandated,
        god_analysis.method_count,
        trait_info,
        extractable
    );

    ActionableRecommendation {
        primary_action,
        rationale,
        implementation_steps: vec![],
        related_items: vec![],
        steps: None,
        estimated_effort_hours: None,
    }
}
```

### 7. Display in TUI

Show trait method breakdown:

```
god object structure
  methods                   32 (14 trait-mandated, 18 extractable)
  fields                    12
  responsibilities          8

trait implementations
  syn::Visit                14 methods (visitor traversal)
  Default                   1 method

extractable methods         18
  - construction            2
  - processing              6
  - helpers                 10

recommendation
  action                    Consider grouping 18 extractable methods by
                            responsibility

  rationale                 14 of 32 methods are trait-mandated
                            (non-extractable). Implements: syn::Visit (14),
                            Default (1). Focus refactoring on the 18
                            self-chosen methods.
```

## Acceptance Criteria

- [ ] Detect trait implementations from `impl Trait for Struct` blocks
- [ ] Maintain registry of known traits with method patterns
- [ ] Classify methods as trait-mandated vs self-chosen
- [ ] Apply reduced weight (0.1-0.4) for trait-mandated methods
- [ ] Add `trait_method_summary` to `GodObjectAnalysis`
- [ ] Generate recommendations focused on extractable methods only
- [ ] Display trait breakdown in TUI detail view
- [ ] Test with `CallGraphExtractor` (syn::Visit)
- [ ] Test with structs implementing Iterator, Future, serde traits

## Technical Details

### Implementation Location

1. **Trait Detection**: `src/extraction/rust_extractor.rs`
   - Already extracts `trait_name` in `ExtractedImpl`
   - May need to enhance method extraction per impl

2. **Known Trait Registry**: `src/organization/god_object/traits.rs` (new file)
   - `KnownTraitRegistry` struct
   - `TraitCategory` enum
   - Default trait database

3. **Method Classification**: `src/organization/god_object/classifier.rs`
   - `classify_method_origin()` function
   - Integration with responsibility classification

4. **Weighting**: `src/extraction/adapters/god_object.rs`
   - Apply trait weights in `build_god_object_analysis()`
   - Calculate `TraitMethodSummary`

5. **Recommendations**: `src/builders/unified_analysis_phases/phases/god_object.rs`
   - `create_god_object_recommendation_with_traits()`
   - Update role classification

6. **Display**: `src/tui/results/detail_pages/overview.rs`
   - Add trait implementations section
   - Show extractable vs mandated breakdown

### Data Flow

```
ExtractedFileData
    └── impls (with trait_name)
            │
            ▼
    detect_trait_impls()
            │
            ▼
    TraitImplInfo[]
            │
            ▼
    classify_method_origin() ◄── KnownTraitRegistry
            │
            ▼
    ClassifiedMethod[]
            │
            ▼
    TraitMethodSummary
            │
            ├──► weighted scoring
            └──► recommendations
```

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_detect_syn_visit_impl() {
    let code = r#"
        impl<'ast> syn::Visit<'ast> for Extractor {
            fn visit_expr(&mut self, e: &'ast Expr) {}
            fn visit_stmt(&mut self, s: &'ast Stmt) {}
        }
    "#;
    let impls = detect_trait_impls(&parse(code), "Extractor");
    assert_eq!(impls.len(), 1);
    assert_eq!(impls[0].trait_path, "syn::Visit");
    assert_eq!(impls[0].required_methods.len(), 2);
}

#[test]
fn test_trait_mandated_weight() {
    let origin = MethodOrigin::TraitMandated {
        trait_name: "syn::Visit".into(),
        category: TraitCategory::Visitor,
    };
    assert_eq!(calculate_method_weight(&origin), 0.1);
}

#[test]
fn test_recommendation_focuses_on_extractable() {
    let summary = TraitMethodSummary {
        mandated_count: 18,
        extractable_count: 14,
        by_trait: [("syn::Visit".into(), 18)].into(),
        weighted_count: 15.8,
    };
    let rec = create_god_object_recommendation_with_traits(&analysis, &summary);

    assert!(!rec.primary_action.contains("sub-orchestrators"));
    assert!(rec.rationale.contains("18"));
    assert!(rec.rationale.contains("trait-mandated"));
}
```

### Integration Tests

- Analyze `CallGraphExtractor` - verify 18 syn::Visit methods detected
- Analyze serde-derived struct - verify Serialize/Deserialize detection
- Verify weighted scores are lower for trait-heavy structs

## Documentation Requirements

- Document trait-mandated vs self-chosen distinction
- List known traits in registry
- Explain weighting rationale

## Implementation Notes

### Detecting Trait Impl Methods

Current `ExtractedImpl` has:
```rust
pub struct ExtractedImpl {
    pub struct_name: String,
    pub trait_name: Option<String>,  // Already exists!
    pub methods: Vec<ExtractedFunction>,
    // ...
}
```

We already have the data - just need to use it in god object analysis.

### Handling Derived Traits

For `#[derive(Clone, Debug, Serialize)]`:
- These don't appear as explicit impl blocks
- Detect via attributes on struct
- Add synthetic trait info

### Unknown Traits

For traits not in registry:
- Still detect as trait-mandated
- Use `TraitCategory::Custom`
- Apply moderate weight (0.4)

### Future Enhancements

- Parse trait definitions to auto-detect required methods
- Detect default method overrides
- Analyze trait complexity (how many methods required)
