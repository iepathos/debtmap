---
number: 137
title: Enhance God Object Analysis with Call Graph and Cohesion Metrics
category: optimization
priority: medium
status: draft
dependencies: [134]
created: 2025-10-27
---

# Specification 137: Enhance God Object Analysis with Call Graph and Cohesion Metrics

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 134 (metric consistency)

## Context

Current god object detection provides generic splitting suggestions without understanding actual code structure:

**Current Output (ripgrep standard.rs)**:
```
SUGGESTED SPLIT (generic - no detailed analysis available):
  [1] standard_core.rs - Core business logic
  [2] standard_io.rs - Input/output operations
  [3] standard_utils.rs - Helper functions
```

**Problems**:
- Generic names don't reflect actual code responsibilities
- No analysis of which functions actually call each other
- No cohesion analysis to identify natural groupings
- Can't determine actual architectural boundaries
- Users can't act on vague suggestions

**What's Needed**:
- Analyze function call patterns within the module
- Identify cohesive clusters of related functions
- Name responsibilities based on actual functionality
- Suggest splits based on actual data flow and coupling

## Objective

Enhance god object analysis with call graph analysis and cohesion metrics to provide specific, actionable splitting recommendations based on actual code structure and data flow.

## Requirements

### Functional Requirements

1. **Intra-Module Call Graph Analysis**
   - Build call graph of functions within the module
   - Identify clusters of tightly-coupled functions
   - Detect data flow boundaries
   - Find functions with high cohesion (call each other frequently)
   - Identify bridge functions that connect clusters

2. **Responsibility Identification**
   - Name responsibilities based on function purposes, not generic terms
   - Analyze function names and types to infer purpose
   - Group functions by domain concepts (e.g., "rendering", "validation", "formatting")
   - Use AST analysis to identify shared data structures
   - Detect common patterns (builders, visitors, state machines)

3. **Cohesion Metrics**
   - Calculate LCOM (Lack of Cohesion of Methods) for each cluster
   - Measure coupling between potential modules
   - Identify shared dependencies
   - Detect tight coupling (high fan-in/fan-out within cluster)
   - Measure interface size between proposed modules

4. **Actionable Split Recommendations**
   - Suggest specific function groups for each new module
   - Name modules based on actual responsibilities
   - Show which functions should move together
   - Estimate interface complexity (public functions needed)
   - Provide migration order (start with lowest coupling)

### Non-Functional Requirements

1. **Accuracy**: Responsibility names should match actual code purpose >80% of time
2. **Completeness**: All module functions should be assigned to a responsibility
3. **Actionability**: Recommendations should be specific enough to implement
4. **Performance**: Call graph analysis should complete in <5s for 1000 function modules
5. **Clarity**: Output should clearly explain why functions are grouped together

## Acceptance Criteria

- [ ] Intra-module call graph is built for modules >100 functions
- [ ] Functions are grouped into cohesive clusters using call patterns
- [ ] Responsibility names are derived from actual function names and purposes
- [ ] Each suggested module split includes specific function names
- [ ] Cohesion metrics (LCOM) are calculated for each cluster
- [ ] Coupling between proposed modules is measured and reported
- [ ] Generic split suggestions ("core/io/utils") are eliminated
- [ ] ripgrep standard.rs gets specific, named recommendations
- [ ] Split recommendations show migration order (least coupled first)
- [ ] Output explains why functions are grouped together
- [ ] Interface size between modules is estimated
- [ ] Unit tests verify clustering on known code patterns

## Technical Details

### Implementation Approach

1. **Call Graph Construction**
   ```rust
   #[derive(Debug, Clone)]
   pub struct IntraModuleCallGraph {
       nodes: HashMap<String, FunctionNode>,
       edges: Vec<CallEdge>,
       clusters: Vec<FunctionCluster>,
   }

   #[derive(Debug, Clone)]
   pub struct FunctionNode {
       name: String,
       visibility: Visibility,
       calls: Vec<String>,      // Functions this calls
       called_by: Vec<String>,  // Functions that call this
       uses_types: Vec<String>, // Types used in signature/body
   }

   #[derive(Debug, Clone)]
   pub struct CallEdge {
       caller: String,
       callee: String,
       call_count: usize,  // Approximate based on AST structure
   }

   impl IntraModuleCallGraph {
       pub fn build(module: &syn::ItemMod) -> Self {
           // Parse module and extract all functions
           let functions = extract_functions(module);

           // Build call relationships
           let edges = find_call_relationships(&functions);

           // Identify clusters using graph algorithms
           let clusters = identify_clusters(&functions, &edges);

           IntraModuleCallGraph { nodes, edges, clusters }
       }
   }
   ```

2. **Cluster Identification**
   ```rust
   #[derive(Debug, Clone)]
   pub struct FunctionCluster {
       name: String,           // Derived from function analysis
       functions: Vec<String>,
       cohesion: f64,         // LCOM metric
       responsibility: String, // Human-readable purpose
       shared_types: Vec<String>,
       internal_calls: usize,  // Calls within cluster
       external_calls: usize,  // Calls to other clusters
   }

   pub fn identify_clusters(
       functions: &[FunctionNode],
       edges: &[CallEdge]
   ) -> Vec<FunctionCluster> {
       // Use community detection algorithms:
       // 1. Louvain method for modularity maximization
       // 2. Or simpler: connected components + density analysis

       let graph = build_petgraph(functions, edges);

       // Find strongly connected components
       let components = tarjan_scc(&graph);

       // Calculate cohesion for each component
       let mut clusters = Vec::new();
       for component in components {
           let cohesion = calculate_lcom(&component, edges);
           let name = infer_responsibility_name(&component, functions);

           clusters.push(FunctionCluster {
               name,
               functions: component.iter().map(|f| f.name.clone()).collect(),
               cohesion,
               responsibility: infer_responsibility_description(&component),
               shared_types: find_shared_types(&component),
               internal_calls: count_internal_calls(&component, edges),
               external_calls: count_external_calls(&component, edges),
           });
       }

       clusters
   }
   ```

3. **Responsibility Naming**
   ```rust
   pub fn infer_responsibility_name(
       cluster: &[FunctionNode],
       all_functions: &[FunctionNode]
   ) -> String {
       // Analyze function names for common patterns
       let verbs = extract_verbs(cluster);
       let nouns = extract_nouns(cluster);

       // Find most common domain terms
       let common_terms = find_common_terms(cluster);

       // Match against known patterns
       if verbs.contains(&"render") || verbs.contains(&"draw") {
           format!("{}Rendering", capitalize(&common_terms[0]))
       } else if verbs.contains(&"validate") || verbs.contains(&"check") {
           format!("{}Validation", capitalize(&common_terms[0]))
       } else if verbs.contains(&"format") || verbs.contains(&"print") {
           format!("{}Formatting", capitalize(&common_terms[0]))
       } else {
           // Fallback to most common noun + most common verb
           format!("{}{}", capitalize(&nouns[0]), capitalize(&verbs[0]))
       }
   }

   pub fn extract_verbs(functions: &[FunctionNode]) -> Vec<String> {
       let verb_patterns = [
           "render", "draw", "paint", "display",
           "validate", "check", "verify", "ensure",
           "format", "print", "write", "output",
           "parse", "read", "load", "fetch",
           "transform", "convert", "map", "filter",
           "create", "build", "construct", "make",
           "update", "modify", "change", "set",
       ];

       functions.iter()
           .flat_map(|f| extract_words(&f.name))
           .filter(|word| verb_patterns.contains(&word.as_str()))
           .collect()
   }
   ```

4. **Cohesion Calculation (LCOM)**
   ```rust
   pub fn calculate_lcom(
       cluster: &[FunctionNode],
       edges: &[CallEdge]
   ) -> f64 {
       // LCOM (Lack of Cohesion of Methods)
       // Lower is better (more cohesive)

       let n = cluster.len();
       if n <= 1 {
           return 0.0; // Single function is perfectly cohesive
       }

       // Count pairs of functions that share no calls
       let mut non_cohesive_pairs = 0;
       let mut cohesive_pairs = 0;

       for i in 0..n {
           for j in (i + 1)..n {
               if functions_share_calls(&cluster[i], &cluster[j], edges) {
                   cohesive_pairs += 1;
               } else {
                   non_cohesive_pairs += 1;
               }
           }
       }

       // LCOM = (non_cohesive - cohesive) / total_pairs
       let total_pairs = (n * (n - 1)) / 2;
       (non_cohesive_pairs as f64 - cohesive_pairs as f64) / total_pairs.max(1) as f64
   }

   fn functions_share_calls(
       f1: &FunctionNode,
       f2: &FunctionNode,
       edges: &[CallEdge]
   ) -> bool {
       // Functions are cohesive if they:
       // 1. Call each other
       // 2. Call the same third function
       // 3. Use the same types

       f1.calls.iter().any(|c| f2.calls.contains(c))
           || f1.calls.contains(&f2.name)
           || f2.calls.contains(&f1.name)
           || f1.uses_types.iter().any(|t| f2.uses_types.contains(t))
   }
   ```

5. **Coupling Analysis**
   ```rust
   pub fn analyze_coupling(clusters: &[FunctionCluster]) -> CouplingMatrix {
       let mut matrix = CouplingMatrix::new(clusters.len());

       for (i, c1) in clusters.iter().enumerate() {
           for (j, c2) in clusters.iter().enumerate() {
               if i == j { continue; }

               let coupling = calculate_coupling(c1, c2);
               matrix.set(i, j, coupling);
           }
       }

       matrix
   }

   #[derive(Debug, Clone)]
   pub struct CouplingScore {
       shared_calls: usize,
       shared_types: usize,
       estimated_interface_size: usize,
   }

   fn calculate_coupling(c1: &FunctionCluster, c2: &FunctionCluster) -> CouplingScore {
       let shared_calls = c1.functions.iter()
           .filter(|f1| c2.functions.iter().any(|f2| calls_function(f1, f2)))
           .count();

       let shared_types: Vec<_> = c1.shared_types.iter()
           .filter(|t| c2.shared_types.contains(t))
           .collect();

       CouplingScore {
           shared_calls,
           shared_types: shared_types.len(),
           estimated_interface_size: shared_calls + shared_types.len(),
       }
   }
   ```

### Splitting Recommendation Algorithm

```rust
pub fn generate_split_recommendations(
    module_name: &str,
    call_graph: &IntraModuleCallGraph
) -> SplitRecommendation {
    let clusters = &call_graph.clusters;

    // Sort clusters by cohesion (highest first)
    let mut sorted = clusters.clone();
    sorted.sort_by(|a, b| b.cohesion.partial_cmp(&a.cohesion).unwrap());

    // Identify clusters to extract (high cohesion + low external coupling)
    let mut recommendations = Vec::new();

    for cluster in sorted {
        let coupling_ratio = cluster.external_calls as f64 /
                            (cluster.internal_calls + cluster.external_calls).max(1) as f64;

        if cluster.cohesion > 0.6 && coupling_ratio < 0.3 {
            // Good candidate for extraction
            recommendations.push(ModuleSplit {
                new_module_name: format!("{}_{}", module_name, cluster.name.to_snake_case()),
                responsibility: cluster.responsibility.clone(),
                functions: cluster.functions.clone(),
                cohesion: cluster.cohesion,
                estimated_interface_size: estimate_interface(&cluster, clusters),
                migration_priority: calculate_priority(&cluster, clusters),
            });
        }
    }

    SplitRecommendation {
        original_module: module_name.to_string(),
        recommended_splits: recommendations,
        remaining_functions: calculate_remaining(clusters, &recommendations),
    }
}

#[derive(Debug, Clone)]
pub struct ModuleSplit {
    new_module_name: String,
    responsibility: String,
    functions: Vec<String>,
    cohesion: f64,
    estimated_interface_size: usize,
    migration_priority: usize, // 1 = do first
}
```

## Dependencies

- **Prerequisites**:
  - Spec 134: Need consistent metrics for clustering
- **Affected Components**:
  - `src/debt/god_object.rs` - Add call graph analysis
  - `src/analysis/call_graph.rs` - New module for call graph
  - `src/analysis/clustering.rs` - New module for clustering algorithms
  - `src/io/output.rs` - Enhanced split recommendations
- **External Dependencies**:
  - `petgraph` - For graph algorithms (already in use)
  - `itertools` - For combinatorics (already in use)
  - May need NLP crate for better name extraction (e.g., `nlp`)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_cluster_identification() {
    let code = r#"
        pub fn render_line() {
            format_line();
            write_output();
        }

        fn format_line() -> String { }
        fn write_output() { }

        pub fn validate_input() {
            check_format();
            verify_bounds();
        }

        fn check_format() -> bool { }
        fn verify_bounds() -> bool { }
    "#;

    let module = parse_module(code);
    let call_graph = IntraModuleCallGraph::build(&module);

    assert_eq!(call_graph.clusters.len(), 2);

    let cluster_names: Vec<_> = call_graph.clusters.iter()
        .map(|c| c.name.as_str())
        .collect();

    assert!(cluster_names.contains(&"Rendering"));
    assert!(cluster_names.contains(&"Validation"));
}

#[test]
fn test_lcom_calculation() {
    // Highly cohesive cluster
    let cohesive = vec![
        FunctionNode {
            name: "func_a".to_string(),
            calls: vec!["func_b".to_string()],
            uses_types: vec!["TypeX".to_string()],
            ..
        },
        FunctionNode {
            name: "func_b".to_string(),
            calls: vec!["func_c".to_string()],
            uses_types: vec!["TypeX".to_string()],
            ..
        },
    ];

    let lcom = calculate_lcom(&cohesive, &[]);
    assert!(lcom < 0.3, "Cohesive functions should have low LCOM");

    // Non-cohesive cluster
    let non_cohesive = vec![
        FunctionNode {
            name: "func_a".to_string(),
            calls: vec![],
            uses_types: vec!["TypeX".to_string()],
            ..
        },
        FunctionNode {
            name: "func_b".to_string(),
            calls: vec![],
            uses_types: vec!["TypeY".to_string()],
            ..
        },
    ];

    let lcom = calculate_lcom(&non_cohesive, &[]);
    assert!(lcom > 0.7, "Non-cohesive functions should have high LCOM");
}

#[test]
fn test_responsibility_naming() {
    let rendering_funcs = vec![
        FunctionNode { name: "render_line".to_string(), .. },
        FunctionNode { name: "draw_border".to_string(), .. },
        FunctionNode { name: "paint_background".to_string(), .. },
    ];

    let name = infer_responsibility_name(&rendering_funcs, &[]);
    assert!(name.contains("Render") || name.contains("Draw"));
}
```

### Integration Tests

```rust
#[test]
fn test_ripgrep_standard_specific_recommendations() {
    let module = parse_file("../ripgrep/crates/printer/src/standard.rs").unwrap();
    let call_graph = IntraModuleCallGraph::build(&module);

    // Should identify multiple cohesive clusters
    assert!(call_graph.clusters.len() >= 3);

    // Clusters should have specific names (not generic)
    for cluster in &call_graph.clusters {
        assert!(!cluster.name.contains("core"));
        assert!(!cluster.name.contains("utils"));
        assert!(!cluster.name.is_empty());
    }

    // Should provide specific function lists
    let recommendations = generate_split_recommendations("standard", &call_graph);
    for split in &recommendations.recommended_splits {
        assert!(!split.functions.is_empty());
        assert!(split.functions.len() > 5, "Clusters should be substantial");
    }
}
```

### Property-Based Tests

```rust
proptest! {
    #[test]
    fn lcom_in_valid_range(functions in valid_function_cluster(2..20)) {
        let lcom = calculate_lcom(&functions, &[]);
        prop_assert!(lcom >= -1.0 && lcom <= 1.0);
    }

    #[test]
    fn all_functions_assigned_to_cluster(module in valid_rust_module()) {
        let call_graph = IntraModuleCallGraph::build(&module);
        let total_functions = module.items.iter()
            .filter(|i| matches!(i, syn::Item::Fn(_)))
            .count();

        let clustered_functions: usize = call_graph.clusters.iter()
            .map(|c| c.functions.len())
            .sum();

        prop_assert_eq!(total_functions, clustered_functions);
    }
}
```

## Documentation Requirements

### Code Documentation

- Document call graph construction algorithm
- Explain clustering approach and rationale
- Provide examples of LCOM calculation
- Document responsibility naming heuristics

### User Documentation

- Explain what call graph analysis provides
- Show examples of good vs bad split recommendations
- Document how to interpret cohesion metrics
- Provide guidance on implementing recommended splits

### Architecture Updates

Update ARCHITECTURE.md:
- Add section on call graph analysis
- Document clustering algorithms used
- Explain cohesion and coupling metrics

## Implementation Notes

### Graph Algorithm Choices

1. **Community Detection**: Louvain method for modularity
   - Pros: Finds natural clusters, scales well
   - Cons: Non-deterministic, may need multiple runs

2. **Strongly Connected Components**: Tarjan's algorithm
   - Pros: Deterministic, finds tightly-coupled groups
   - Cons: May miss looser associations

3. **Hybrid Approach**: Use both
   - Tarjan for tightly-coupled cores
   - Louvain for broader organization
   - Manual merging of small clusters

### Responsibility Naming Challenges

- Function names may not follow conventions
- Abbreviated or cryptic names reduce accuracy
- Multiple responsibilities in one cluster
- Fallback to position-based names ("Cluster1") as last resort

### Performance Considerations

- Call graph for 1000 functions = ~1M possible edges
- Use sparse graph representation
- Limit analysis depth (don't analyze called functions outside module)
- Cache intermediate results

### Future Enhancements

- **Data flow analysis**: Track shared mutable state
- **Type dependency analysis**: Group functions using same types
- **Test-based clustering**: Group functions tested together
- **Historical analysis**: Use git history to find co-changing functions

## Migration and Compatibility

### Breaking Changes

None - this is additive functionality

### Backward Compatibility

- Generic recommendations remain as fallback
- New detailed analysis shown when available
- Output format extended with new fields (JSON compatible)

## Success Metrics

- Zero generic "core/io/utils" recommendations for analyzed modules
- >80% of responsibility names match manual code review
- All functions assigned to clusters (100% coverage)
- Cohesion metrics correlate with manual assessment (>0.7 correlation)
- Users report recommendations are actionable
- Reduced time to implement splitting recommendations
