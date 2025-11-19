---
number: 181
title: Type-Based Clustering for Idiomatic Rust Recommendations
category: foundation
priority: critical
status: draft
dependencies: [179, 180]
created: 2025-01-19
---

# Specification 181: Type-Based Clustering for Idiomatic Rust Recommendations

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: [179 - Coupling Analysis, 180 - Module Split Recommendations]

## Context

Current god object recommendations use behavioral clustering (grouping by verb: render, classify, calculate), which violates idiomatic Rust and functional programming principles:

**Problems with Behavioral Clustering:**
1. **Groups by "how" instead of "what"** - rendering.rs contains methods operating on different data types
2. **Creates utilities modules** - catch-all anti-pattern for methods that don't fit behavioral categories
3. **Violates type ownership** - data scattered across modules as parameters instead of owned by types
4. **Not functional** - doesn't create clear data transformation pipelines
5. **Breaks single responsibility** - rendering.rs mixes formatting of scores, coverage, complexity (different domains)

**Example of the problem:**
```rust
// Current behavioral split (❌ Anti-pattern)
mod rendering {
    fn format_score(score: f64) -> String { }       // Operates on score data
    fn format_coverage(cov: f64) -> String { }      // Operates on coverage data
    fn format_complexity(comp: u32) -> String { }   // Operates on complexity data
    // Mixed data types! No clear ownership!
}

// Idiomatic type-based split (✅ Correct)
mod score {
    struct ScoreSection { value: f64, factors: Vec<Factor> }
    impl ScoreSection {
        fn format(&self) -> String { }  // Data owns behavior
    }
}
mod coverage {
    struct CoverageSection { percentage: f64, gaps: Vec<Gap> }
    impl CoverageSection {
        fn format(&self) -> String { }  // Data owns behavior
    }
}
```

## Objective

Implement type-based clustering that analyzes **what data types methods operate on** and recommends extracting types with their implementations, following idiomatic Rust and functional programming principles.

## Requirements

### Functional Requirements

1. **Type Signature Analysis**
   - Extract parameter types from all methods (syn::ItemFn, syn::ImplItemFn)
   - Extract return types from method signatures
   - Extract Self types from impl blocks
   - Track type usage patterns across methods

2. **Type Affinity Detection**
   - Group methods that operate on the same types
   - Calculate type affinity score for each method pair (simple counting)
   - Identify primary type for each cluster (most common param/return type)

3. **Type Cluster Generation**
   - Create clusters based on shared type usage (not shared behavior)
   - Name clusters after primary data type ("PriorityItem", "GodObjectMetrics")
   - Ensure each cluster represents single data domain
   - Avoid creating technical groupings (no "rendering", "utilities")

### Non-Functional Requirements

1. **Compatibility**: Works with existing call graph and field tracking infrastructure
2. **Performance**: Type analysis adds <5% to god object detection time
3. **Accuracy**: Correctly identifies primary type in 90%+ of clusters
4. **Usability**: Recommendations clearly explain type ownership principles

## Acceptance Criteria

- [ ] Type signature analyzer extracts param/return types from syn AST
- [ ] Type affinity calculator groups methods by shared type usage (simple counting)
- [ ] Cluster naming reflects data domains (not behaviors)
- [ ] Zero "utilities" modules in recommendations (all methods belong to types)
- [ ] Recommendations include example type definitions with impl blocks
- [ ] When run on formatter.rs:
  - Recommends priority_item.rs (not rendering.rs)
  - Shows PriorityItem struct definition
  - No utilities module
  - All methods assigned to data domains
- [ ] When run on god_object_analysis.rs:
  - Recommends pipeline stages (detection, diversity, splitting)
  - Shows transformation types (Metrics → Diversity → Recommendation)
  - Explains data flow between modules

## Technical Details

### Implementation Approach

#### 1. Type Signature Extraction

```rust
// src/organization/type_based_clustering.rs

use syn::{Type, ReturnType, FnArg, ImplItem};

pub struct TypeSignatureAnalyzer;

pub struct MethodSignature {
    pub name: String,
    pub param_types: Vec<TypeInfo>,
    pub return_type: Option<TypeInfo>,
    pub self_type: Option<TypeInfo>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TypeInfo {
    pub name: String,
    pub is_reference: bool,
    pub is_mutable: bool,
    pub generics: Vec<String>,
}

impl TypeSignatureAnalyzer {
    /// Extract type information from method
    pub fn analyze_method(&self, method: &syn::ImplItemFn) -> MethodSignature {
        let param_types = method.sig.inputs.iter()
            .filter_map(|arg| self.extract_type_from_arg(arg))
            .collect();

        let return_type = match &method.sig.output {
            ReturnType::Type(_, ty) => Some(self.extract_type_info(ty)),
            _ => None,
        };

        MethodSignature {
            name: method.sig.ident.to_string(),
            param_types,
            return_type,
            self_type: None, // Extracted from impl context
        }
    }

    fn extract_type_from_arg(&self, arg: &FnArg) -> Option<TypeInfo> {
        match arg {
            FnArg::Typed(pat_type) => Some(self.extract_type_info(&pat_type.ty)),
            FnArg::Receiver(_) => None, // self
        }
    }

    fn extract_type_info(&self, ty: &Type) -> TypeInfo {
        match ty {
            Type::Path(type_path) => {
                let segment = type_path.path.segments.last();
                let name = segment
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                // Extract generic parameters
                let generics = segment
                    .and_then(|seg| match &seg.arguments {
                        syn::PathArguments::AngleBracketed(args) => {
                            Some(args.args.iter()
                                .filter_map(|arg| match arg {
                                    syn::GenericArgument::Type(ty) => {
                                        Some(self.extract_type_info(ty).name)
                                    }
                                    _ => None,
                                })
                                .collect())
                        }
                        _ => None,
                    })
                    .unwrap_or_default();

                TypeInfo {
                    name,
                    is_reference: false,
                    is_mutable: false,
                    generics,
                }
            },
            Type::Reference(type_ref) => {
                let mut inner = self.extract_type_info(&type_ref.elem);
                inner.is_reference = true;
                inner.is_mutable = type_ref.mutability.is_some();
                inner
            },
            _ => TypeInfo {
                name: "Unknown".to_string(),
                is_reference: false,
                is_mutable: false,
                generics: vec![],
            },
        }
    }
}
```

#### 2. Type Affinity Clustering

```rust
pub struct TypeAffinityAnalyzer;

pub struct TypeCluster {
    pub primary_type: TypeInfo,
    pub methods: Vec<String>,
    pub type_affinity_score: f64,
    pub input_types: HashSet<String>,
    pub output_types: HashSet<String>,
}

impl TypeAffinityAnalyzer {
    /// Cluster methods by type affinity (shared type usage)
    pub fn cluster_by_type_affinity(
        &self,
        signatures: &[MethodSignature],
    ) -> Vec<TypeCluster> {
        // 1. Calculate type affinity matrix
        let affinity_matrix = self.build_type_affinity_matrix(signatures);

        // 2. Use existing community detection with type affinity as weights
        // Reuse Louvain algorithm from behavioral_decomposition.rs
        let clusters = self.cluster_by_affinity(
            signatures,
            &affinity_matrix,
        );

        // 3. Identify primary type for each cluster
        for cluster in &mut clusters {
            cluster.primary_type = self.identify_primary_type(&cluster.methods, signatures);
        }

        clusters
    }

    fn build_type_affinity_matrix(
        &self,
        signatures: &[MethodSignature],
    ) -> HashMap<(String, String), f64> {
        let mut affinity = HashMap::new();

        for sig1 in signatures {
            for sig2 in signatures {
                if sig1.name == sig2.name {
                    continue;
                }

                let score = self.calculate_type_affinity(sig1, sig2);
                if score > 0.0 {
                    affinity.insert((sig1.name.clone(), sig2.name.clone()), score);
                }
            }
        }

        affinity
    }

    /// Calculate type affinity between two method signatures
    ///
    /// # Affinity Scoring (Simplified)
    ///
    /// Simple counting approach - methods belong together if they share types:
    ///
    /// - **Shared domain types**: +1 per shared type (ignoring primitives)
    ///   - Example: `analyze(metrics: &Metrics)` + `format(metrics: &Metrics)` → 1
    ///   - Primitives like String, Vec ignored - too generic
    ///
    /// - **Same self type**: +1
    ///   - Example: Both methods in `impl Analyzer`
    ///
    /// That's it. No fancy weights, just count shared domain types.
    fn calculate_type_affinity(&self, sig1: &MethodSignature, sig2: &MethodSignature) -> f64 {
        let mut score = 0.0;

        // Count shared domain types (ignore primitives)
        let shared_domain_types = sig1.param_types.iter()
            .filter(|t1| self.is_domain_type(&t1.name))
            .filter(|t1| sig2.param_types.iter().any(|t2| t1.name == t2.name))
            .count();

        score += shared_domain_types as f64;

        // Same self type
        if sig1.self_type == sig2.self_type && sig1.self_type.is_some() {
            score += 1.0;
        }

        score
    }

    /// Check if type is domain-specific (not primitive or stdlib)
    fn is_domain_type(&self, type_name: &str) -> bool {
        !matches!(
            type_name,
            "String" | "str" | "Vec" | "Option" | "Result" |
            "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" |
            "usize" | "isize" | "u32" | "i32" | "u64" | "i64" |
            "f32" | "f64" | "bool" | "char" | "Path" | "PathBuf"
        ) && !type_name.starts_with('&')
    }

    /// Check if two types match, handling generic wrappers
    ///
    /// Examples:
    /// - `Metrics` matches `Metrics` (exact)
    /// - `Option<Metrics>` matches `Metrics` (unwrap generic)
    /// - `Vec<Item>` matches `Item` (unwrap generic)
    fn types_match(&self, type1: &str, type2: &str) -> bool {
        if type1 == type2 {
            return true;
        }

        // Extract inner type from generic wrappers
        let extract_inner = |t: &str| -> &str {
            if let Some(start) = t.find('<') {
                if let Some(end) = t.rfind('>') {
                    return &t[start + 1..end];
                }
            }
            t
        };

        extract_inner(type1) == extract_inner(type2)
    }

    /// Identify primary type for a cluster of methods
    ///
    /// # Algorithm
    ///
    /// 1. Count type occurrences (params + returns)
    /// 2. If tie, use tie-breaking rules:
    ///    - Prefer domain types over primitives
    ///    - Prefer return types (output) over param types (input)
    ///    - Prefer non-wrapper types (avoid Option<T>, Vec<T>)
    ///    - Prefer longer, more specific type names
    /// 3. Extract base type from generics
    fn identify_primary_type(
        &self,
        methods: &[String],
        signatures: &[MethodSignature],
    ) -> TypeInfo {
        #[derive(Debug, Clone)]
        struct TypeCandidate {
            name: String,
            count: usize,
            is_domain_type: bool,
            return_occurrences: usize,
            param_occurrences: usize,
        }

        // Count type occurrences with detailed tracking
        let mut type_candidates: HashMap<String, TypeCandidate> = HashMap::new();

        for method in methods {
            if let Some(sig) = signatures.iter().find(|s| &s.name == method) {
                // Count parameter types
                for param in &sig.param_types {
                    let base_name = self.extract_base_type(&param.name);
                    type_candidates
                        .entry(base_name.clone())
                        .and_modify(|c| {
                            c.count += 1;
                            c.param_occurrences += 1;
                        })
                        .or_insert_with(|| TypeCandidate {
                            name: base_name.clone(),
                            count: 1,
                            is_domain_type: self.is_domain_type(&base_name),
                            return_occurrences: 0,
                            param_occurrences: 1,
                        });
                }

                // Count return types (with bonus weight)
                if let Some(ret) = &sig.return_type {
                    let base_name = self.extract_base_type(&ret.name);
                    type_candidates
                        .entry(base_name.clone())
                        .and_modify(|c| {
                            c.count += 1;
                            c.return_occurrences += 1;
                        })
                        .or_insert_with(|| TypeCandidate {
                            name: base_name.clone(),
                            count: 1,
                            is_domain_type: self.is_domain_type(&base_name),
                            return_occurrences: 1,
                            param_occurrences: 0,
                        });
                }
            }
        }

        // Remove primitives and stdlib types if domain types exist
        let has_domain_types = type_candidates.values().any(|c| c.is_domain_type);
        if has_domain_types {
            type_candidates.retain(|_, c| c.is_domain_type);
        }

        // Select primary type using tie-breaking rules
        let primary_candidate = type_candidates
            .values()
            .max_by(|a, b| {
                // Rule 1: Most occurrences wins
                match a.count.cmp(&b.count) {
                    std::cmp::Ordering::Equal => {
                        // Rule 2: Prefer types that appear as returns (outputs)
                        match a.return_occurrences.cmp(&b.return_occurrences) {
                            std::cmp::Ordering::Equal => {
                                // Rule 3: Prefer domain types
                                match a.is_domain_type.cmp(&b.is_domain_type) {
                                    std::cmp::Ordering::Equal => {
                                        // Rule 4: Prefer longer names (more specific)
                                        a.name.len().cmp(&b.name.len())
                                    }
                                    other => other,
                                }
                            }
                            other => other,
                        }
                    }
                    other => other,
                }
            })
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        TypeInfo {
            name: primary_candidate,
            is_reference: false,
            is_mutable: false,
            generics: vec![],
        }
    }

    /// Extract base type from generic wrappers
    ///
    /// Examples:
    /// - `Option<Metrics>` → `Metrics`
    /// - `Vec<Item>` → `Item`
    /// - `Result<Data, Error>` → `Data` (first generic arg)
    /// - `Metrics` → `Metrics` (unchanged)
    fn extract_base_type(&self, type_name: &str) -> String {
        // Handle generic types
        if let Some(start) = type_name.find('<') {
            if let Some(end) = type_name.rfind('>') {
                let inner = &type_name[start + 1..end];
                // For multi-generic types (e.g., Result<T, E>), take first
                if let Some(comma) = inner.find(',') {
                    return inner[..comma].trim().to_string();
                }
                return inner.trim().to_string();
            }
        }

        // Handle references
        type_name.trim_start_matches('&').trim_start_matches("mut ").to_string()
    }
}
```

### ModuleSplit Extensions

Add new fields to `ModuleSplit` struct for type-based clustering:

```rust
// src/organization/god_object_analysis.rs

pub struct ModuleSplit {
    // ... existing fields ...

    /// Core type that owns the methods in this module (Spec 181)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_type: Option<String>,

    /// Data flow showing input and output types (Spec 181, 182)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_flow: Vec<String>,
}
```

### Integration with Existing Infrastructure

```rust
// src/organization/god_object_detector.rs

fn generate_idiomatic_splits(
    all_methods: &[String],
    field_tracker: Option<&FieldAccessTracker>,
    ast: &syn::File,
    base_name: &str,
) -> Vec<ModuleSplit> {
    use crate::organization::type_based_clustering::{
        TypeSignatureAnalyzer,
        TypeAffinityAnalyzer,
    };

    // 1. Extract type signatures
    let type_analyzer = TypeSignatureAnalyzer;
    let signatures = ast.items.iter()
        .filter_map(|item| match item {
            syn::Item::Impl(impl_block) => Some(impl_block),
            _ => None,
        })
        .flat_map(|impl_block| &impl_block.items)
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method) => Some(type_analyzer.analyze_method(method)),
            _ => None,
        })
        .collect::<Vec<_>>();

    // 2. Cluster by type affinity (simple counting)
    let affinity_analyzer = TypeAffinityAnalyzer;
    let type_clusters = affinity_analyzer.cluster_by_type_affinity(&signatures);

    // 3. Convert to ModuleSplit recommendations
    type_clusters.into_iter().map(|cluster| {
        ModuleSplit {
            suggested_name: format!("{}/{}", base_name, cluster.primary_type.name.to_lowercase()),
            responsibility: format!(
                "Manage {} data and its transformations",
                cluster.primary_type.name
            ),
            methods_to_move: cluster.methods,
            core_type: Some(cluster.primary_type.name.clone()),
            data_flow: cluster.input_types.into_iter()
                .chain(cluster.output_types)
                .collect(),
            ..Default::default()
        }
    }).collect()
}
```

### Output Format Enhancement

```rust
// Example recommendation output

#1 SCORE: 149 [CRITICAL]
└─ ./src/priority/formatter.rs (3000 lines, 103 functions)

IDIOMATIC RUST REFACTORING:

Extract domain types that own their behavior:

1. formatter/priority_item.rs (25-30 methods, ~400 lines)
   Purpose: Format individual priority items
   Core type: PriorityItem

   SUGGESTED TYPE DEFINITION:

   pub struct PriorityItem {
       pub score: f64,
       pub location: PathBuf,
       pub metrics: FileMetrics,
       pub verbosity: u8,
   }

   impl PriorityItem {
       pub fn new(item: &UnifiedDebtItem, verbosity: u8) -> Self {
           Self {
               score: item.score(),
               location: item.location().clone(),
               metrics: item.metrics().clone(),
               verbosity,
           }
       }

       pub fn format(&self) -> String {
           let header = self.format_header();
           let metrics = self.format_metrics();
           format!("{}\n{}", header, metrics)
       }

       fn format_header(&self) -> String { /* ... */ }
       fn format_metrics(&self) -> String { /* ... */ }
   }

   BENEFITS:
   ✅ Data owns behavior (idiomatic Rust)
   ✅ No parameter passing (self instead of 4+ params)
   ✅ Testable (mock PriorityItem directly)
   ✅ Type-driven design

2. formatter/god_object_section.rs (20-25 methods, ~350 lines)
   Purpose: Format god object analysis results
   Core type: GodObjectSection

   [Similar structure with type definition...]
```

## Dependencies

- **Prerequisites**:
  - Existing call graph infrastructure (behavioral_decomposition.rs)
  - syn AST parsing capabilities
  - Community detection algorithm
- **Affected Components**:
  - `god_object_detector.rs` - Add type-based analysis
  - `mod.rs` - Export new type_based_clustering module
  - `formatter.rs` - Update recommendation output format
- **External Dependencies**: None (uses existing syn)

## Testing Strategy

### Unit Tests

1. **Type Extraction**:
   - Test extracting types from simple methods
   - Test handling references and mutability
   - Test generic type extraction
   - Test self type detection

2. **Type Affinity**:
   - Test affinity calculation for shared params
   - Test pipeline detection (A → B transformations)
   - Test clustering with type weights
   - Test primary type identification

3. **Type Cluster Generation**:
   - Test cluster naming based on primary type
   - Test that clusters avoid behavioral groupings
   - Test data flow identification

### Integration Tests

1. **Real Codebases**:
   - Test on formatter.rs - verify PriorityItem recommendation
   - Test on god_object_analysis.rs - verify pipeline stages
   - Test on formatter_verbosity.rs - verify no utilities module
   - Verify all methods assigned to type domains

2. **Comparison Tests**:
   - Compare behavioral vs type-based recommendations
   - Verify type-based eliminates utilities modules
   - Verify better domain boundaries

### Validation

- Manual review of recommendations for 5 god objects
- Verify type suggestions are accurate
- Verify no "rendering", "utilities", or technical groupings
- Verify recommendations follow Rust idioms

## Documentation Requirements

### Code Documentation

1. Document type affinity algorithm and scoring
2. Document type name suggestion heuristics
3. Add examples of type extraction patterns
4. Document integration with existing infrastructure

### User Documentation

1. Add section explaining type-based recommendations
2. Show examples of idiomatic Rust refactoring
3. Explain difference from behavioral clustering
4. Provide migration guide from behavioral to type-based

### Architecture Updates

1. Document type-based clustering subsystem
2. Explain type affinity vs behavioral affinity
3. Show recommendation generation pipeline
4. Add decision tree for type vs behavioral clustering

## Implementation Notes

### Type Name Heuristics

```rust
// Priority order for type naming:
// 1. Explicit struct type in parameters (highest confidence)
// 2. Domain inference from method names
// 3. Return type analysis
// 4. Fallback to generic names
```

### Edge Cases

1. **Methods with no types** (only primitives) → Use domain inference
2. **Multiple equally common types** → Use first in data flow
3. **Generic types** → Extract base type name
4. **Trait methods** → Group by trait implementation

### Performance Optimization

1. **Cache type extraction** - Don't re-parse syn AST
2. **Lazy affinity calculation** - Only for methods in same file
3. **Parallel analysis** - Use rayon for type extraction
4. **Memoization** - Cache type affinity scores

## Trait-Aware Clustering

### Detecting Trait Implementations

```rust
impl TypeAffinityAnalyzer {
    /// Detect trait implementations and cluster accordingly
    ///
    /// Groups methods implementing the same trait together, even if
    /// they operate on different concrete types.
    pub fn detect_trait_clusters(
        &self,
        ast: &syn::File,
    ) -> Vec<TraitCluster> {
        let mut trait_impls: HashMap<String, Vec<String>> = HashMap::new();

        // Find all trait implementations
        for item in &ast.items {
            if let syn::Item::Impl(impl_block) = item {
                // Check if this is a trait impl
                if let Some((_, trait_path, _)) = &impl_block.trait_ {
                    let trait_name = trait_path.segments.last()
                        .map(|s| s.ident.to_string())
                        .unwrap_or_default();

                    // Collect methods from this impl
                    let methods: Vec<_> = impl_block.items.iter()
                        .filter_map(|item| {
                            if let syn::ImplItem::Fn(method) = item {
                                Some(method.sig.ident.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();

                    trait_impls.entry(trait_name).or_default().extend(methods);
                }
            }
        }

        // Convert to TraitCluster
        trait_impls.into_iter()
            .map(|(trait_name, methods)| TraitCluster {
                trait_name,
                methods,
                suggested_extraction: TraitExtractionSuggestion::SeparateModule,
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct TraitCluster {
    pub trait_name: String,
    pub methods: Vec<String>,
    pub suggested_extraction: TraitExtractionSuggestion,
}

#[derive(Clone, Debug)]
pub enum TraitExtractionSuggestion {
    /// Extract to separate module (e.g., `display.rs` for Display impl)
    SeparateModule,

    /// Keep with type definition (small trait impls)
    WithType,

    /// Extract as extension trait
    ExtensionTrait,
}
```

### Integration with Type Clustering

```rust
// Combine type-based and trait-based clustering
let type_clusters = affinity_analyzer.cluster_by_type_affinity(&signatures);
let trait_clusters = affinity_analyzer.detect_trait_clusters(&ast);

// Merge: trait clusters get priority for standard traits
let merged = merge_type_and_trait_clusters(type_clusters, trait_clusters);
```

## Migration and Compatibility

### Breaking Changes

- **None** - This is additive functionality that extends existing god object analysis

### Backwards Compatibility Strategy

#### Phase 1: Parallel Execution (Non-Breaking)

```rust
// Run both behavioral and type-based clustering
let behavioral_splits = generate_behavioral_splits(...);  // Existing
let type_based_splits = generate_type_based_splits(...);  // New (Spec 181)

// Store both in analysis result
god_object_analysis.recommended_splits = behavioral_splits;  // Default
god_object_analysis.type_based_splits = Some(type_based_splits);  // Optional field
```

#### Phase 2: Gradual Migration (User Choice)

```toml
[analysis.god_object]
# User can choose clustering strategy
clustering_strategy = "behavioral"  # Default, existing behavior
# clustering_strategy = "type-based"  # New approach
# clustering_strategy = "both"  # Show both for comparison
```

#### Phase 3: Quality-Based Selection (Automatic)

```rust
// Automatically choose best approach based on anti-pattern detection
let behavioral_quality = anti_pattern_detector.calculate_split_quality(&behavioral_splits);
let type_based_quality = anti_pattern_detector.calculate_split_quality(&type_based_splits);

if type_based_quality.quality_score > behavioral_quality.quality_score + 10.0 {
    // Type-based is significantly better
    god_object_analysis.recommended_splits = type_based_splits;
    god_object_analysis.analysis_method = SplitAnalysisMethod::TypeBased;
} else {
    // Keep existing behavioral approach
    god_object_analysis.recommended_splits = behavioral_splits;
    god_object_analysis.analysis_method = SplitAnalysisMethod::Behavioral;
}
```

### Schema Evolution

Add optional fields to `ModuleSplit` (already present in spec):

```rust
pub struct ModuleSplit {
    // ... existing fields ...

    /// Core type that owns the methods in this module (Spec 181)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_type: Option<String>,

    /// Data flow showing input and output types (Spec 181, 182)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_flow: Vec<String>,

    /// Suggested implicit type extraction (Spec 181, 184)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit_type_suggestion: Option<ImplicitTypeSuggestion>,

    /// Analysis method that generated this split
    #[serde(default)]
    pub method: SplitAnalysisMethod,  // Already exists
}
```

Old consumers ignore new fields (backward compatible).

### Configuration

```toml
[analysis.god_object]
# Clustering strategy selection
clustering_strategy = "type-based"  # "behavioral" | "type-based" | "both" | "auto"

# Type-based clustering tuning
detect_implicit_types = true
min_type_affinity = 0.3
enable_trait_clustering = true

# Parameter clump detection (Spec 184 integration)
min_parameter_clump_size = 3

# Performance budget
max_type_analysis_time_ms = 150
```

## Success Metrics

- Type-based clustering identifies primary type in 90%+ of clusters
- Zero utilities modules in recommendations
- Recommendations align with Rust idioms (manual validation)
- All methods assigned to data domains (100% coverage)
- User feedback: recommendations are more actionable
- Type suggestions are implemented in 70%+ of cases

## Future Enhancements

1. **Trait extraction**: Suggest traits for common behavior
2. **Newtype patterns**: Detect primitives that should be newtypes
3. **Builder patterns**: Suggest builders for complex constructors
4. **Type hierarchy**: Detect and suggest trait hierarchies
5. **Generic inference**: Suggest generic type parameters
