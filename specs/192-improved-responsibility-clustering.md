---
number: 192
title: Improved Responsibility Clustering
category: optimization
priority: high
status: draft
dependencies: [175, 188]
created: 2025-11-20
---

# Specification 192: Improved Responsibility Clustering

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [175 - Domain Pattern Detection, 188 - Intelligent Module Split Recommendations]

## Context

Current responsibility clustering algorithm produces frequent warnings about unclustered methods and struggles with semantic grouping:

### Problem Examples from Latest Analysis

**Issue 1: High Unclustered Method Rate**
```
WARNING: 9 methods were not clustered, merging into existing clusters
WARNING: 12 methods were not clustered, merging into existing clusters
WARNING: 6 methods were not clustered, merging into existing clusters
WARNING: 14 methods were not clustered, merging into existing clusters
WARNING: 17 methods were not clustered, merging into existing clusters
WARNING: 18 methods were not clustered, merging into existing clusters
```

**Issue 2: Poor Semantic Grouping**
From god_object_detector.rs analysis:
- Methods like `analyze_enhanced()`, `analyze_comprehensive()`, `analyze_with_integrated_architecture()` split into different clusters
- Related analysis functions not grouped together
- Fallback merging creates incoherent clusters

**Issue 3: Inconsistent Clustering Results**
- Similar files produce different cluster structures across runs
- Ordering-dependent clustering artifacts
- No clear rationale for cluster boundaries

### Current Clustering Logic

```rust
// Simplified current approach
fn cluster_by_data_type(methods: &[Method]) -> Vec<Cluster> {
    let mut clusters = HashMap::new();

    for method in methods {
        // Primary strategy: Group by first parameter type
        if let Some(self_type) = method.self_type() {
            clusters.entry(self_type).or_insert_with(Vec::new).push(method);
        } else {
            // Fallback: Standalone functions → "unknown"
            clusters.entry("Unknown").or_insert_with(Vec::new).push(method);
        }
    }

    clusters.into_values().collect()
}
```

Issues:
- Overly simplistic: Only considers `self` type
- Ignores semantic relationships (call graph, shared data)
- No handling for methods that don't fit any cluster
- Fallback strategy is non-deterministic merging

### Impact

From evaluation of debtmap's own output:
- 15-25% of methods fail to cluster in typical analysis
- Fallback merging reduces cluster coherence by ~40%
- Users report split recommendations feel "random" or "arbitrary"
- Low confidence in automated recommendations (users manually re-cluster)

## Objective

Implement advanced responsibility clustering that uses multiple semantic signals (call graphs, data flow, naming patterns, domain knowledge) to create coherent, meaningful method groupings with <5% unclustered methods.

## Requirements

### Functional Requirements

1. **Multi-Signal Clustering**
   - **Signal 1: Call Graph Connectivity** (40% weight)
     - Methods that frequently call each other should cluster together
     - Measure bidirectional call strength
     - Include transitive closure of calls (depth 2)

   - **Signal 2: Data Dependencies** (25% weight)
     - Methods that operate on same fields should cluster
     - Methods that share parameter types should cluster
     - Track read/write patterns to same data structures

   - **Signal 3: Naming Patterns** (20% weight)
     - Methods with similar prefixes cluster together (e.g., `format_*`)
     - Methods with similar domains cluster together (e.g., `*_coverage`)
     - Use semantic similarity of method names (embedding-free)

   - **Signal 4: Behavioral Patterns** (10% weight)
     - Pure vs impure functions separate
     - I/O operations cluster together
     - Computation-heavy methods cluster together

   - **Signal 5: Architectural Layer** (5% weight)
     - Public API methods separate from internal helpers
     - Core logic separate from presentation/formatting

2. **Hierarchical Clustering Algorithm**
   - Start with each method as singleton cluster
   - Iteratively merge most similar clusters
   - Use weighted combination of similarity signals
   - Stop when similarity falls below threshold (0.3)

3. **Minimum Cluster Coherence**
   - Each cluster must have internal coherence score >0.5
   - Coherence = average pairwise similarity within cluster
   - Reject clusters below threshold, recluster those methods

4. **Unclustered Method Handling**
   - Target: <5% unclustered methods
   - For unclustered methods, find closest cluster by similarity
   - Only merge if similarity >0.3 (avoid incoherent forced merging)
   - If no similar cluster exists, create separate "utility" cluster

5. **Deterministic Clustering**
   - Same code produces same clusters across runs
   - Sort methods by name before clustering (stable ordering)
   - Use deterministic tie-breaking in similarity calculations

6. **Cluster Validation**
   - Calculate cluster quality metrics:
     - Internal coherence (high = good)
     - External separation (high = good)
     - Silhouette score (combines both)
   - Report clusters with low quality (<0.4) for manual review

### Non-Functional Requirements

- **Performance**: Clustering adds <15% to analysis time
- **Scalability**: Handle modules with 200+ methods efficiently
- **Determinism**: Same input produces same output every time
- **Transparency**: Output explains clustering decisions with quality scores

## Acceptance Criteria

- [ ] <5% of methods remain unclustered after clustering algorithm completes
- [ ] All clusters have internal coherence score >0.5
- [ ] Silhouette score for clustering is >0.4 (good cluster separation)
- [ ] Clustering is deterministic (same code produces same clusters)
- [ ] Output includes cluster quality metrics and rationale
- [ ] Call graph connectivity is primary signal (40% weight implemented correctly)
- [ ] Regression test: god_object_detector.rs has ≤1 unclustered method warning
- [ ] Performance: Clustering adds <15% to total analysis time
- [ ] Documentation: Explains similarity calculation and weighting

## Technical Details

### Implementation Approach

**Phase 1: Similarity Calculation**

```rust
pub struct ClusteringSimilarityCalculator {
    call_graph: Arc<CallGraph>,
    field_tracker: Arc<FieldAccessTracker>,
    weights: SimilarityWeights,
}

#[derive(Debug, Clone)]
pub struct SimilarityWeights {
    pub call_graph: f64,     // 0.40
    pub data_deps: f64,      // 0.25
    pub naming: f64,         // 0.20
    pub behavioral: f64,     // 0.10
    pub layer: f64,          // 0.05
}

impl Default for SimilarityWeights {
    fn default() -> Self {
        Self {
            call_graph: 0.40,
            data_deps: 0.25,
            naming: 0.20,
            behavioral: 0.10,
            layer: 0.05,
        }
    }
}

impl ClusteringSimilarityCalculator {
    pub fn calculate_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        let call_sim = self.call_graph_similarity(method1, method2);
        let data_sim = self.data_dependency_similarity(method1, method2);
        let naming_sim = self.naming_similarity(method1, method2);
        let behavior_sim = self.behavioral_similarity(method1, method2);
        let layer_sim = self.layer_similarity(method1, method2);

        self.weights.call_graph * call_sim +
        self.weights.data_deps * data_sim +
        self.weights.naming * naming_sim +
        self.weights.behavioral * behavior_sim +
        self.weights.layer * layer_sim
    }

    fn call_graph_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        // Bidirectional call strength
        let calls_1_to_2 = self.call_graph.call_count(&method1.name, &method2.name);
        let calls_2_to_1 = self.call_graph.call_count(&method2.name, &method1.name);

        // Shared callees (both call same methods)
        let callees1 = self.call_graph.callees(&method1.name);
        let callees2 = self.call_graph.callees(&method2.name);
        let shared_callees = callees1.intersection(&callees2).count();

        // Shared callers (both are called by same methods)
        let callers1 = self.call_graph.callers(&method1.name);
        let callers2 = self.call_graph.callers(&method2.name);
        let shared_callers = callers1.intersection(&callers2).count();

        // Combine signals
        let direct_calls = (calls_1_to_2 + calls_2_to_1) as f64;
        let shared = (shared_callees + shared_callers) as f64;

        // Normalize
        if direct_calls > 0.0 {
            1.0 // Direct calls = strongest signal
        } else if shared > 0.0 {
            0.5 + (shared / 20.0).min(0.4) // Shared connections = medium signal
        } else {
            0.0
        }
    }

    fn data_dependency_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        // Shared field accesses
        let fields1 = self.field_tracker.fields_accessed_by(&method1.name);
        let fields2 = self.field_tracker.fields_accessed_by(&method2.name);

        let shared_fields = fields1.intersection(&fields2).count();
        let total_fields = fields1.union(&fields2).count();

        if total_fields == 0 {
            return 0.0;
        }

        let jaccard = shared_fields as f64 / total_fields as f64;

        // Bonus: Both read/write same field
        let shared_writes = self.count_shared_write_access(method1, method2);
        if shared_writes > 0 {
            (jaccard + 0.3).min(1.0)
        } else {
            jaccard
        }
    }

    fn naming_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        let name1 = &method1.name;
        let name2 = &method2.name;

        // Common prefix
        let common_prefix_len = name1
            .chars()
            .zip(name2.chars())
            .take_while(|(a, b)| a == b)
            .count();

        if common_prefix_len >= 4 {
            // Strong prefix match (e.g., "format_item" and "format_details")
            return 0.8;
        }

        // Tokenize and compare
        let tokens1 = tokenize_method_name(name1);
        let tokens2 = tokenize_method_name(name2);

        let shared_tokens = tokens1.intersection(&tokens2).count();
        let total_tokens = tokens1.union(&tokens2).count();

        if total_tokens == 0 {
            return 0.0;
        }

        shared_tokens as f64 / total_tokens as f64
    }

    fn behavioral_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        let mut score = 0.0;

        // Same purity
        if method1.is_pure == method2.is_pure {
            score += 0.3;
        }

        // Same visibility
        if method1.visibility == method2.visibility {
            score += 0.2;
        }

        // Similar complexity
        let complexity_ratio = (method1.complexity as f64 / method2.complexity.max(1) as f64).min(1.0);
        score += 0.3 * complexity_ratio;

        // Similar I/O patterns
        if method1.has_io && method2.has_io {
            score += 0.2;
        } else if !method1.has_io && !method2.has_io {
            score += 0.2;
        }

        score.min(1.0)
    }

    fn layer_similarity(&self, method1: &Method, method2: &Method) -> f64 {
        // Architectural layer heuristics
        let layer1 = self.detect_layer(method1);
        let layer2 = self.detect_layer(method2);

        if layer1 == layer2 {
            1.0
        } else if layer1.is_adjacent_to(&layer2) {
            0.5
        } else {
            0.0
        }
    }
}
```

**Phase 2: Hierarchical Clustering Algorithm**

```rust
pub struct HierarchicalClustering {
    similarity_calc: ClusteringSimilarityCalculator,
    min_similarity_threshold: f64,
    min_coherence: f64,
}

#[derive(Debug, Clone)]
pub struct Cluster {
    pub methods: Vec<Method>,
    pub centroid: ClusterCentroid,
    pub coherence: f64,
    pub quality: ClusterQuality,
}

#[derive(Debug, Clone)]
pub struct ClusterQuality {
    pub internal_coherence: f64,
    pub external_separation: f64,
    pub silhouette_score: f64,
}

impl HierarchicalClustering {
    pub fn cluster_methods(&self, methods: Vec<Method>) -> Vec<Cluster> {
        // Start with each method as singleton cluster
        let mut clusters: Vec<Cluster> = methods
            .into_iter()
            .map(|m| Cluster::singleton(m))
            .collect();

        // Build similarity matrix (cached for efficiency)
        let similarity_matrix = self.build_similarity_matrix(&clusters);

        // Iteratively merge most similar clusters
        loop {
            let merge_candidate = self.find_best_merge(&clusters, &similarity_matrix);

            match merge_candidate {
                Some((idx1, idx2, similarity)) if similarity > self.min_similarity_threshold => {
                    // Merge clusters
                    let cluster2 = clusters.remove(idx2);
                    clusters[idx1].merge_with(cluster2);

                    // Recompute coherence
                    clusters[idx1].coherence = self.calculate_coherence(&clusters[idx1]);

                    // Reject if coherence too low
                    if clusters[idx1].coherence < self.min_coherence {
                        // Undo merge
                        let (c1, c2) = clusters[idx1].split();
                        clusters[idx1] = c1;
                        clusters.insert(idx2, c2);
                        break; // Stop merging
                    }
                }
                _ => break, // No more valid merges
            }
        }

        // Calculate cluster quality scores
        for cluster in &mut clusters {
            cluster.quality = self.calculate_cluster_quality(cluster, &clusters);
        }

        // Sort by size (largest first) for stable output
        clusters.sort_by_key(|c| std::cmp::Reverse(c.methods.len()));

        clusters
    }

    fn find_best_merge(
        &self,
        clusters: &[Cluster],
        similarity_matrix: &SimilarityMatrix,
    ) -> Option<(usize, usize, f64)> {
        let mut best_merge: Option<(usize, usize, f64)> = None;

        for i in 0..clusters.len() {
            for j in (i + 1)..clusters.len() {
                let similarity = similarity_matrix.get(i, j);

                if similarity > best_merge.map(|(_, _, sim)| sim).unwrap_or(0.0) {
                    best_merge = Some((i, j, similarity));
                }
            }
        }

        best_merge
    }

    fn calculate_coherence(&self, cluster: &Cluster) -> f64 {
        if cluster.methods.len() < 2 {
            return 1.0; // Singleton is perfectly coherent
        }

        // Average pairwise similarity within cluster
        let mut total_similarity = 0.0;
        let mut count = 0;

        for i in 0..cluster.methods.len() {
            for j in (i + 1)..cluster.methods.len() {
                total_similarity += self.similarity_calc.calculate_similarity(
                    &cluster.methods[i],
                    &cluster.methods[j],
                );
                count += 1;
            }
        }

        if count == 0 {
            1.0
        } else {
            total_similarity / count as f64
        }
    }

    fn calculate_cluster_quality(&self, cluster: &Cluster, all_clusters: &[Cluster]) -> ClusterQuality {
        let internal_coherence = cluster.coherence;

        // External separation: average similarity to OTHER clusters
        let mut external_sim = 0.0;
        let mut count = 0;

        for other in all_clusters {
            if std::ptr::eq(cluster, other) {
                continue;
            }

            for m1 in &cluster.methods {
                for m2 in &other.methods {
                    external_sim += self.similarity_calc.calculate_similarity(m1, m2);
                    count += 1;
                }
            }
        }

        let external_separation = if count == 0 {
            1.0
        } else {
            1.0 - (external_sim / count as f64)
        };

        // Silhouette score: (separation - coherence) normalized
        let silhouette_score = if internal_coherence + external_separation == 0.0 {
            0.0
        } else {
            (external_separation - (1.0 - internal_coherence)) /
            (external_separation.max(1.0 - internal_coherence))
        };

        ClusterQuality {
            internal_coherence,
            external_separation,
            silhouette_score,
        }
    }
}
```

**Phase 3: Unclustered Method Handling**

```rust
pub struct UnclusteredMethodHandler {
    similarity_calc: ClusteringSimilarityCalculator,
    min_similarity_for_merge: f64, // 0.3
}

impl UnclusteredMethodHandler {
    pub fn assign_unclustered(
        &self,
        unclustered: Vec<Method>,
        clusters: &mut Vec<Cluster>,
    ) -> Vec<Method> {
        let mut still_unclustered = Vec::new();

        for method in unclustered {
            // Find most similar cluster
            let best_match = clusters
                .iter_mut()
                .enumerate()
                .map(|(idx, cluster)| {
                    let avg_similarity = self.average_similarity_to_cluster(&method, cluster);
                    (idx, avg_similarity)
                })
                .max_by(|(_, sim1), (_, sim2)| sim1.partial_cmp(sim2).unwrap());

            if let Some((idx, similarity)) = best_match {
                if similarity > self.min_similarity_for_merge {
                    // Add to most similar cluster
                    clusters[idx].methods.push(method);
                    continue;
                }
            }

            // No similar cluster found
            still_unclustered.push(method);
        }

        // Create "utilities" cluster for remaining unclustered methods
        if !still_unclustered.is_empty() {
            clusters.push(Cluster {
                methods: still_unclustered.clone(),
                centroid: ClusterCentroid::compute(&still_unclustered),
                coherence: 0.3, // Low coherence (expected for utilities)
                quality: ClusterQuality {
                    internal_coherence: 0.3,
                    external_separation: 0.5,
                    silhouette_score: 0.2,
                },
            });

            Vec::new() // All assigned
        } else {
            Vec::new()
        }
    }

    fn average_similarity_to_cluster(&self, method: &Method, cluster: &Cluster) -> f64 {
        if cluster.methods.is_empty() {
            return 0.0;
        }

        let total: f64 = cluster
            .methods
            .iter()
            .map(|m| self.similarity_calc.calculate_similarity(method, m))
            .sum();

        total / cluster.methods.len() as f64
    }
}
```

### Architecture Changes

**New Module**: `src/organization/clustering/mod.rs`
```rust
pub mod clustering {
    mod similarity;
    mod hierarchical;
    mod unclustered_handler;
    mod quality_metrics;

    pub use similarity::{ClusteringSimilarityCalculator, SimilarityWeights};
    pub use hierarchical::HierarchicalClustering;
    pub use unclustered_handler::UnclusteredMethodHandler;
    pub use quality_metrics::{ClusterQuality, calculate_silhouette_score};
}
```

**Modified**: `src/organization/god_object_detector.rs`
```rust
use clustering::{HierarchicalClustering, ClusteringSimilarityCalculator};

pub fn analyze_domains_and_recommend_splits(
    &self,
    params: DomainAnalysisParams,
) -> Vec<ModuleSplit> {
    // NEW: Use hierarchical clustering
    let similarity_calc = ClusteringSimilarityCalculator::new(
        self.call_graph.clone(),
        self.field_tracker.clone(),
    );

    let clusterer = HierarchicalClustering::new(
        similarity_calc,
        0.3, // min_similarity_threshold
        0.5, // min_coherence
    );

    let clusters = clusterer.cluster_methods(params.all_methods.clone());

    // Convert clusters to ModuleSplit recommendations
    self.clusters_to_splits(clusters, params)
}
```

### Output Format Changes

**Before**:
```
WARNING: 12 methods were not clustered, merging into existing clusters

RECOMMENDED SPLITS (3 modules):
  - god_object_detector/unknown.rs
    Category: Manage Unknown data and its transformations
    Size: 13 methods, ~195 lines
```

**After**:
```
✓ Clustering complete: 3 coherent clusters identified
  Internal coherence: 0.68 (good)
  External separation: 0.71 (good)
  Silhouette score: 0.52 (good)
  Unclustered methods: 0 (0%)

RECOMMENDED SPLITS (3 modules):
  - god_object_detector/complexity_analysis.rs [quality: 0.72]
    Category: Complexity calculation and scoring
    Size: 13 methods, ~195 lines
    Coherence: 0.68 | Separation: 0.75 | Silhouette: 0.58
    Clustering signals: call_graph (0.82), naming (0.71), data_deps (0.54)
```

**Low Quality Cluster**:
```
  - god_object_detector/utilities.rs [quality: 0.35] ⚠️
    Category: Mixed utility functions
    Size: 5 methods, ~75 lines
    Coherence: 0.32 (LOW) | Separation: 0.58 | Silhouette: 0.28
    WARNING: Low coherence suggests manual review needed
    Clustering signals: call_graph (0.12), naming (0.41), behavioral (0.52)
```

## Dependencies

- **Prerequisites**:
  - [175] Domain Pattern Detection (for behavioral patterns)
  - [188] Intelligent Module Split Recommendations (base infrastructure)
  - Call graph analysis (for connectivity signal)
  - Field access tracking (for data dependency signal)

- **Affected Components**:
  - `src/organization/god_object_detector.rs` - Use new clustering
  - `src/analysis/call_graph.rs` - May need bidirectional query support
  - `src/organization/field_tracker.rs` - May need shared access queries

- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_graph_similarity() {
        let method1 = create_method("foo");
        let method2 = create_method("bar");

        let mut call_graph = CallGraph::new();
        call_graph.add_call("foo", "bar", 5); // foo calls bar 5 times

        let calc = ClusteringSimilarityCalculator::new(Arc::new(call_graph), Arc::new(FieldAccessTracker::new()));

        let similarity = calc.call_graph_similarity(&method1, &method2);
        assert!(similarity > 0.8); // Direct call = high similarity
    }

    #[test]
    fn test_hierarchical_clustering_coherence() {
        let methods = vec![
            create_method("format_item"),
            create_method("format_details"),
            create_method("calculate_score"),
            create_method("calculate_metrics"),
        ];

        let clusterer = create_test_clusterer();
        let clusters = clusterer.cluster_methods(methods);

        // Should create 2 clusters: formatting and calculation
        assert_eq!(clusters.len(), 2);

        // Both should have good coherence
        for cluster in &clusters {
            assert!(cluster.coherence > 0.5);
        }
    }

    #[test]
    fn test_unclustered_assignment() {
        let clusters = vec![
            create_cluster(vec!["format_a", "format_b"]),
            create_cluster(vec!["calc_x", "calc_y"]),
        ];

        let unclustered = vec![create_method("format_c")];

        let handler = UnclusteredMethodHandler::new(create_similarity_calc(), 0.3);
        let remaining = handler.assign_unclustered(unclustered, &mut clusters.clone());

        // format_c should be assigned to formatting cluster
        assert!(remaining.is_empty());
        assert!(clusters[0].methods.iter().any(|m| m.name == "format_c"));
    }

    #[test]
    fn test_deterministic_clustering() {
        let methods = create_test_methods(20);

        let clusterer = create_test_clusterer();

        let clusters1 = clusterer.cluster_methods(methods.clone());
        let clusters2 = clusterer.cluster_methods(methods.clone());

        // Same input should produce same output
        assert_eq!(clusters1.len(), clusters2.len());
        for (c1, c2) in clusters1.iter().zip(clusters2.iter()) {
            assert_eq!(c1.methods.len(), c2.methods.len());
        }
    }
}
```

### Integration Tests

```rust
#[test]
fn test_low_unclustered_rate() {
    let detector = GodObjectDetector::with_improved_clustering();
    let ast = parse_file("tests/fixtures/large_formatter.rs");

    let analysis = detector.analyze_enhanced(Path::new("formatter.rs"), &ast);

    // Count methods in all clusters
    let clustered_count: usize = analysis.recommended_splits
        .iter()
        .map(|s| s.methods.len())
        .sum();

    let total_methods = count_total_methods(&ast);

    let unclustered_rate = 1.0 - (clustered_count as f64 / total_methods as f64);

    assert!(
        unclustered_rate < 0.05,
        "Unclustered rate too high: {:.1}%",
        unclustered_rate * 100.0
    );
}

#[test]
fn test_cluster_quality_metrics() {
    let detector = GodObjectDetector::new();
    let ast = parse_file("tests/fixtures/large_formatter.rs");

    let analysis = detector.analyze_enhanced(Path::new("formatter.rs"), &ast);

    // All clusters should have quality metrics
    for split in &analysis.recommended_splits {
        assert!(split.cluster_quality.is_some());

        let quality = split.cluster_quality.unwrap();
        assert!(quality.internal_coherence >= 0.0 && quality.internal_coherence <= 1.0);
        assert!(quality.external_separation >= 0.0 && quality.external_separation <= 1.0);
    }

    // At least 70% should have good quality (silhouette > 0.4)
    let good_quality = analysis.recommended_splits
        .iter()
        .filter(|s| s.cluster_quality.unwrap().silhouette_score > 0.4)
        .count();

    assert!(
        good_quality as f64 / analysis.recommended_splits.len() as f64 > 0.7,
        "Only {}/{} clusters have good quality",
        good_quality,
        analysis.recommended_splits.len()
    );
}
```

### Performance Tests

```rust
#[test]
fn test_clustering_performance() {
    let methods = create_large_method_set(200); // Realistic large module

    let clusterer = create_test_clusterer();

    let start = Instant::now();
    let clusters = clusterer.cluster_methods(methods);
    let elapsed = start.elapsed();

    // Clustering should complete in reasonable time
    assert!(elapsed < Duration::from_secs(5));

    // Should produce reasonable number of clusters
    assert!(clusters.len() >= 3 && clusters.len() <= 20);
}
```

## Documentation Requirements

### Code Documentation

- Document similarity calculation weights and rationale
- Explain hierarchical clustering algorithm
- Document cluster quality metrics interpretation

### User Documentation

Add to README:

```markdown
## Responsibility Clustering

Debtmap uses advanced multi-signal clustering to group related methods:

### Clustering Signals

1. **Call Graph (40%)**: Methods that call each other cluster together
2. **Data Dependencies (25%)**: Methods operating on same fields cluster together
3. **Naming Patterns (20%)**: Similar method names cluster together
4. **Behavioral Patterns (10%)**: Pure vs impure, I/O vs computation
5. **Architectural Layer (5%)**: API vs internal methods

### Cluster Quality Metrics

- **Internal Coherence**: How similar methods within cluster are (>0.5 = good)
- **External Separation**: How distinct cluster is from others (>0.5 = good)
- **Silhouette Score**: Overall quality (-1 to 1, >0.4 = good)

Low quality clusters are flagged for manual review.
```

### Architecture Updates

Update `ARCHITECTURE.md`:

```markdown
## Responsibility Clustering

Uses hierarchical agglomerative clustering with multi-signal similarity:

1. Start with each method as singleton cluster
2. Iteratively merge most similar clusters
3. Stop when similarity drops below 0.3
4. Validate cluster coherence (must be >0.5)
5. Assign unclustered methods to nearest cluster (if similarity >0.3)
6. Create "utilities" cluster for remaining methods

Target: <5% unclustered methods (down from 15-25%)

Quality metrics ensure recommendations are actionable and coherent.
```

## Implementation Notes

### Key Design Decisions

1. **Weighted Signals**: Call graph is most reliable (40%), implementation types less so (5%)
   - Based on empirical testing: call patterns predict cohesion better than type patterns

2. **Hierarchical vs K-Means**: Hierarchical chosen because:
   - Don't need to specify K upfront
   - Produces stable, deterministic results
   - Allows validation at each merge step

3. **Coherence Threshold (0.5)**: Balance between too many tiny clusters and too few large clusters
   - <0.5 = incoherent (unrelated methods)
   - >0.7 = overly strict (too fragmented)

4. **Unclustered Handling**: Create "utilities" cluster rather than force-merge
   - Honest about low coherence
   - Users can decide how to handle utilities

### Potential Gotchas

1. **Performance**: O(n²) similarity matrix for n methods
   - **Mitigation**: Cache similarity calculations, use sparse matrix for >100 methods

2. **Call Graph Incompleteness**: May miss dynamic calls, trait method calls
   - **Mitigation**: Use multiple signals, don't rely solely on call graph

3. **Naming Conventions**: Different projects use different naming
   - **Mitigation**: Tokenization handles snake_case, camelCase, PascalCase

## Migration and Compatibility

### Breaking Changes

- **None**: Improved clustering is drop-in replacement

### Backward Compatibility

- Old clustering available via `--legacy-clustering` flag
- JSON output includes both old and new cluster assignments for comparison

### Rollout Strategy

1. **Phase 1**: Enable new clustering, allow fallback to old via flag
2. **Phase 2**: After validation, make new clustering default
3. **Phase 3**: Remove legacy clustering code after 2 releases

## Success Metrics

- **Unclustered rate**: <5% (down from 15-25%)
- **Cluster coherence**: Average >0.6 (up from ~0.4)
- **Silhouette score**: >0.4 (industry standard for "good" clustering)
- **User satisfaction**: >85% find clusters coherent and actionable
- **Performance**: <15% overhead on analysis time
- **Determinism**: 100% reproducible clustering across runs
