---
number: 99
title: Aggregate Function Scores by File
category: optimization
priority: high
status: draft
dependencies: [96, 97]
created: 2025-09-07
---

# Specification 99: Aggregate Function Scores by File

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [96 - Remove Score Capping, 97 - Add File-Level Scoring]

## Context

Files containing multiple problematic functions should be prioritized higher than files with single issues, but currently each function is scored independently. A file like `rust_call_graph.rs` with 270 functions may have many medium-scoring functions that collectively represent massive technical debt, but each individual function might only score 5-10 points. Aggregating these scores would properly surface files needing comprehensive refactoring.

## Objective

Implement file-level score aggregation that sums function-level debt scores while applying scaling factors for function count, creating a composite score that reflects the total debt burden of each file.

## Requirements

### Functional Requirements
- Sum all function scores within each file
- Apply scaling factor based on function count
- Maintain both individual and aggregated scores
- Support filtering by aggregated score threshold
- Display both file aggregate and top functions within file

### Non-Functional Requirements
- Efficient aggregation without multiple file passes
- Maintain function-level detail for drill-down
- Aggregation must be deterministic
- Support incremental updates for large codebases

## Acceptance Criteria

- [ ] File aggregation implemented in scoring pipeline
- [ ] Files with many problematic functions score higher
- [ ] rust_call_graph.rs aggregate score >500
- [ ] Aggregated scores shown in recommendations
- [ ] Can drill down from file to individual functions
- [ ] Performance impact <3% on analysis time
- [ ] Configuration options for aggregation strategy
- [ ] Tests verify aggregation correctness

## Technical Details

### Aggregation Strategy

```rust
// src/priority/aggregation.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAggregateScore {
    pub file_path: PathBuf,
    pub total_score: f64,
    pub function_count: usize,
    pub problematic_functions: usize,  // Score > threshold
    pub top_function_scores: Vec<(String, f64)>,  // Top 5
    pub aggregate_score: f64,
    pub aggregation_method: AggregationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationMethod {
    Sum,
    WeightedSum,
    LogarithmicSum,
    MaxPlusAverage,
}

impl FileAggregateScore {
    pub fn calculate_aggregate(&mut self) {
        self.aggregate_score = match self.aggregation_method {
            AggregationMethod::Sum => {
                // Simple sum with count scaling
                self.total_score * (1.0 + (self.function_count as f64).ln() / 10.0)
            },
            
            AggregationMethod::WeightedSum => {
                // Weight by problem density
                let density = self.problematic_functions as f64 / self.function_count as f64;
                self.total_score * (1.0 + density) * (self.function_count as f64).sqrt() / 10.0
            },
            
            AggregationMethod::LogarithmicSum => {
                // Logarithmic scaling to prevent runaway scores
                self.total_score * (1.0 + (self.function_count as f64).ln())
            },
            
            AggregationMethod::MaxPlusAverage => {
                // Max function score plus average of others
                let max_score = self.top_function_scores
                    .first()
                    .map(|(_, s)| *s)
                    .unwrap_or(0.0);
                let avg_score = self.total_score / self.function_count.max(1) as f64;
                max_score + (avg_score * self.function_count as f64 * 0.5)
            }
        };
    }
}
```

### Aggregation Pipeline

```rust
// src/priority/mod.rs
pub struct AggregationPipeline {
    function_scores: HashMap<PathBuf, Vec<FunctionScore>>,
    file_aggregates: HashMap<PathBuf, FileAggregateScore>,
    config: AggregationConfig,
}

impl AggregationPipeline {
    pub fn aggregate_file_scores(&mut self) -> Vec<FileAggregateScore> {
        for (path, functions) in &self.function_scores {
            let total_score: f64 = functions.iter().map(|f| f.score).sum();
            let problematic = functions.iter()
                .filter(|f| f.score > self.config.problem_threshold)
                .count();
            
            let mut top_functions: Vec<_> = functions.iter()
                .map(|f| (f.name.clone(), f.score))
                .collect();
            top_functions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            top_functions.truncate(5);
            
            let mut aggregate = FileAggregateScore {
                file_path: path.clone(),
                total_score,
                function_count: functions.len(),
                problematic_functions: problematic,
                top_function_scores: top_functions,
                aggregate_score: 0.0,
                aggregation_method: self.config.method.clone(),
            };
            
            aggregate.calculate_aggregate();
            self.file_aggregates.insert(path.clone(), aggregate);
        }
        
        let mut results: Vec<_> = self.file_aggregates.values().cloned().collect();
        results.sort_by(|a, b| b.aggregate_score.partial_cmp(&a.aggregate_score).unwrap());
        results
    }
}
```

### Configuration

```rust
// src/config.rs
#[derive(Debug, Clone, Deserialize)]
pub struct AggregationConfig {
    pub enabled: bool,
    pub method: AggregationMethod,
    pub problem_threshold: f64,  // Functions scoring above this are "problematic"
    pub min_functions_for_aggregation: usize,  // Don't aggregate files with few functions
    pub display_top_functions: usize,  // How many top functions to show
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            method: AggregationMethod::WeightedSum,
            problem_threshold: 5.0,
            min_functions_for_aggregation: 3,
            display_top_functions: 5,
        }
    }
}
```

### Display Format

```rust
// src/priority/formatter.rs
fn format_aggregate_item(item: &FileAggregateScore) -> String {
    let mut output = format!(
        "📁 FILE AGGREGATE SCORE: {:.1}\n",
        item.aggregate_score
    );
    
    output.push_str(&format!(
        "   └─ {} ({} functions, total score: {:.1})\n",
        item.file_path.display(),
        item.function_count,
        item.total_score
    ));
    
    if item.problematic_functions > 0 {
        output.push_str(&format!(
            "      ⚠️  {} problematic functions (score > {:.1})\n",
            item.problematic_functions,
            5.0  // threshold
        ));
    }
    
    output.push_str("      📊 Top issues:\n");
    for (func_name, score) in &item.top_function_scores {
        output.push_str(&format!(
            "         - {}: {:.1}\n",
            func_name,
            score
        ));
    }
    
    output.push_str(&format!(
        "      🔧 ACTION: Comprehensive refactoring needed\n"
    ));
    
    output
}
```

### Integration with Recommendations

```rust
// src/priority/recommender.rs
pub fn generate_recommendations(
    function_items: Vec<DebtItem>,
    file_aggregates: Vec<FileAggregateScore>,
    config: &RecommendationConfig,
) -> Vec<Recommendation> {
    let mut all_items = Vec::new();
    
    // Add file aggregates as recommendations
    for aggregate in file_aggregates {
        if aggregate.aggregate_score > config.file_aggregate_threshold {
            all_items.push(Recommendation::FileAggregate(aggregate));
        }
    }
    
    // Add individual function items not in aggregated files
    for item in function_items {
        let in_aggregate = file_aggregates.iter()
            .any(|a| a.file_path == item.location.file);
        
        if !in_aggregate || item.score > config.always_show_threshold {
            all_items.push(Recommendation::Function(item));
        }
    }
    
    // Sort by score
    all_items.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap());
    all_items.truncate(config.max_recommendations);
    all_items
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 96 (Remove Score Capping) - for meaningful aggregate scores
  - Spec 97 (File-Level Scoring) - complements file aggregation
- **Affected Components**: 
  - Scoring pipeline
  - Recommendation generation
  - Report formatting
  - Configuration system
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test aggregation calculations
- **Integration Tests**: End-to-end aggregation pipeline
- **Validation Tests**: Known problematic files aggregate correctly
- **Performance Tests**: Measure aggregation overhead
- **Configuration Tests**: Different aggregation methods

## Documentation Requirements

- **Aggregation Methods**: Explain each method and when to use
- **Configuration Guide**: How to tune aggregation parameters
- **Interpretation Guide**: Understanding aggregate vs individual scores
- **Best Practices**: When to focus on aggregates vs functions

## Implementation Notes

1. **Incremental Updates**:
   ```rust
   // Support updating single file without full re-aggregation
   pub fn update_file_aggregate(&mut self, path: &Path) {
       if let Some(functions) = self.function_scores.get(path) {
           // Recalculate just this file
       }
   }
   ```

2. **Memory Efficiency**:
   - Don't store all function details in aggregate
   - Use references where possible
   - Consider streaming for large codebases

3. **Filtering Options**:
   ```bash
   # Show only file aggregates
   debtmap analyze --aggregate-only
   
   # Show files with 10+ problematic functions
   debtmap analyze --min-problematic 10
   
   # Use different aggregation method
   debtmap analyze --aggregation-method logarithmic
   ```

## Migration and Compatibility

- Aggregation is opt-in by default
- Existing function-level analysis unchanged
- Can disable with `--no-aggregation` flag
- Configuration migration for existing users