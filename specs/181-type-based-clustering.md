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
   - Calculate type affinity score for each method pair
   - Identify primary type for each cluster (most common param/return type)
   - Detect implicit types (parameter groups appearing together in 3+ methods)

3. **Type Cluster Generation**
   - Create clusters based on shared type usage (not shared behavior)
   - Name clusters after primary data type ("PriorityItem", "GodObjectMetrics")
   - Ensure each cluster represents single data domain
   - Avoid creating technical groupings (no "rendering", "utilities")

4. **Implicit Type Extraction**
   - Detect repeated parameter patterns across methods
   - Suggest extracting struct for parameter groups appearing 3+ times
   - Detect tuple returns that should be structs
   - Recommend type names based on domain analysis

### Non-Functional Requirements

1. **Compatibility**: Works with existing call graph and field tracking infrastructure
2. **Performance**: Type analysis adds <5% to god object detection time
3. **Accuracy**: Correctly identifies primary type in 90%+ of clusters
4. **Usability**: Recommendations clearly explain type ownership principles

## Acceptance Criteria

- [ ] Type signature analyzer extracts param/return types from syn AST
- [ ] Type affinity calculator groups methods by shared type usage
- [ ] Cluster naming reflects data domains (not behaviors)
- [ ] Zero "utilities" modules in recommendations (all methods belong to types)
- [ ] Implicit type detector finds parameter clumps (3+ occurrences)
- [ ] Recommendations include example type definitions
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

    fn calculate_type_affinity(&self, sig1: &MethodSignature, sig2: &MethodSignature) -> f64 {
        let mut score = 0.0;

        // Shared parameter types (strong signal)
        let shared_params = sig1.param_types.iter()
            .filter(|t1| sig2.param_types.iter().any(|t2| t1.name == t2.name))
            .count();
        score += shared_params as f64 * 0.5;

        // Return type matches param type (transformation pipeline)
        if let Some(ret1) = &sig1.return_type {
            if sig2.param_types.iter().any(|p| p.name == ret1.name) {
                score += 1.0; // A → B pipeline connection
            }
        }

        // Same self type (if impl methods)
        if sig1.self_type == sig2.self_type && sig1.self_type.is_some() {
            score += 0.3;
        }

        score
    }

    fn identify_primary_type(
        &self,
        methods: &[String],
        signatures: &[MethodSignature],
    ) -> TypeInfo {
        // Count type occurrences across all params and returns
        let mut type_counts: HashMap<String, usize> = HashMap::new();

        for method in methods {
            if let Some(sig) = signatures.iter().find(|s| &s.name == method) {
                for param in &sig.param_types {
                    *type_counts.entry(param.name.clone()).or_insert(0) += 1;
                }
                if let Some(ret) = &sig.return_type {
                    *type_counts.entry(ret.name.clone()).or_insert(0) += 1;
                }
            }
        }

        // Most common type is primary type
        let primary_name = type_counts.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        TypeInfo {
            name: primary_name,
            is_reference: false,
            is_mutable: false,
            generics: vec![],
        }
    }
}
```

#### 3. Implicit Type Detector

```rust
pub struct ImplicitTypeDetector;

pub struct ImplicitType {
    pub suggested_name: String,
    pub fields: Vec<(String, TypeInfo)>,
    pub methods: Vec<String>,
    pub occurrences: usize,
}

impl ImplicitTypeDetector {
    /// Find parameter groups that should be extracted as types
    pub fn detect_implicit_types(
        &self,
        signatures: &[MethodSignature],
    ) -> Vec<ImplicitType> {
        // 1. Find parameter clumps (same params in 3+ methods)
        let clumps = self.find_parameter_clumps(signatures);

        // 2. Find tuple returns that should be structs
        let tuple_returns = self.find_tuple_returns(signatures);

        // 3. Suggest type names and structure
        let mut implicit_types = vec![];

        for clump in clumps {
            implicit_types.push(ImplicitType {
                suggested_name: self.suggest_type_name(&clump.params),
                fields: clump.params.clone(),
                methods: clump.methods.clone(),
                occurrences: clump.methods.len(),
            });
        }

        implicit_types
    }

    fn find_parameter_clumps(&self, signatures: &[MethodSignature]) -> Vec<ParameterClump> {
        // Group methods by parameter signature
        let mut param_groups: HashMap<Vec<String>, Vec<String>> = HashMap::new();

        for sig in signatures {
            let param_key: Vec<String> = sig.param_types.iter()
                .map(|t| t.name.clone())
                .collect();

            param_groups.entry(param_key).or_default().push(sig.name.clone());
        }

        // Filter for groups with 3+ methods
        param_groups.into_iter()
            .filter(|(_, methods)| methods.len() >= 3)
            .map(|(params, methods)| ParameterClump {
                params: params.into_iter()
                    .map(|name| (name.clone(), TypeInfo { name, ..Default::default() }))
                    .collect(),
                methods,
            })
            .collect()
    }

    fn suggest_type_name(&self, fields: &[(String, TypeInfo)]) -> String {
        // Use dominant type name or domain inference
        let types: Vec<_> = fields.iter().map(|(_, t)| &t.name).collect();

        // If UnifiedDebtItem appears, suggest "PriorityItem"
        if types.contains(&&"UnifiedDebtItem".to_string()) {
            return "PriorityItem".to_string();
        }

        // If GodObjectAnalysis appears, suggest based on context
        if types.contains(&&"GodObjectAnalysis".to_string()) {
            return "GodObjectSection".to_string();
        }

        // Generic fallback
        "ExtractedType".to_string()
    }
}

struct ParameterClump {
    params: Vec<(String, TypeInfo)>,
    methods: Vec<String>,
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

    /// Suggested implicit type extraction (Spec 181, 184)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit_type_suggestion: Option<ImplicitTypeSuggestion>,
}

/// Implicit type suggestion for parameter clumps or tuple returns
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImplicitTypeSuggestion {
    pub type_name: String,
    pub fields: Vec<(String, String)>,  // (field_name, field_type)
    pub occurrences: usize,
    pub confidence: f64,
    pub rationale: String,
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
        ImplicitTypeDetector,
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

    // 2. Cluster by type affinity
    let affinity_analyzer = TypeAffinityAnalyzer;
    let type_clusters = affinity_analyzer.cluster_by_type_affinity(&signatures);

    // 3. Detect implicit types
    let implicit_detector = ImplicitTypeDetector;
    let implicit_types = implicit_detector.detect_implicit_types(&signatures);

    // 4. Convert to ModuleSplit recommendations
    type_clusters.into_iter().map(|cluster| {
        // Find matching implicit type suggestion
        let implicit_suggestion = implicit_types.iter()
            .find(|t| cluster.methods.iter().any(|m| t.methods.contains(m)))
            .map(|t| ImplicitTypeSuggestion {
                type_name: t.suggested_name.clone(),
                fields: t.fields.iter()
                    .map(|(name, info)| (name.clone(), info.name.clone()))
                    .collect(),
                occurrences: t.occurrences,
                confidence: 0.8, // Based on clustering strength
                rationale: format!(
                    "Parameter pattern appears in {} methods with shared type {}",
                    t.occurrences, cluster.primary_type.name
                ),
            });

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
            implicit_type_suggestion: implicit_suggestion,
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

3. **Implicit Type Detection**:
   - Test parameter clump detection (3+ occurrences)
   - Test type name suggestion
   - Test tuple return detection
   - Test field extraction

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

## Migration and Compatibility

### Breaking Changes

- None - this is additive functionality

### Backwards Compatibility

- Type-based clustering runs alongside behavioral (not replacing)
- Can use both and compare results
- Gradual migration path

### Configuration

```toml
[analysis.god_object]
clustering_strategy = "type-based"  # "behavioral" | "type-based" | "both"
detect_implicit_types = true
min_type_affinity = 0.3
min_parameter_clump_size = 3
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
