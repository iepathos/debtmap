---
number: 97
title: Add File-Level Scoring and Metrics
category: optimization
priority: critical
status: draft
dependencies: [96]
created: 2025-09-07
---

# Specification 97: Add File-Level Scoring and Metrics

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: [96 - Remove Score Capping]

## Context

Debtmap currently only analyzes individual functions, missing critical file-level issues like god objects. The 3,860-line `rust_call_graph.rs` file with 270 functions is analyzed as 270 separate small items rather than being identified as a massive god object requiring urgent refactoring. File-level metrics like total lines, function count, and average complexity would provide crucial context for prioritization.

## Objective

Implement file-level scoring that aggregates function-level metrics and adds file-specific measurements to properly identify and prioritize large, complex files and god objects.

## Requirements

### Functional Requirements
- Calculate file-level metrics (lines, functions, average complexity)
- Aggregate function scores into file scores
- Detect and score god objects at file level
- Include file-level items in debt reports
- Maintain function-level analysis alongside file-level

### Non-Functional Requirements
- Minimal performance impact on analysis time
- File scores should be deterministic
- Integrate seamlessly with existing scoring system
- Support both file and function level reporting

## Acceptance Criteria

- [ ] File-level metrics struct implemented
- [ ] File scoring algorithm implemented and tested
- [ ] God object detection integrated into file scoring
- [ ] rust_call_graph.rs scores >150 as a file
- [ ] File-level items appear in top recommendations
- [ ] Both file and function items can be displayed
- [ ] Performance impact <5% on analysis time
- [ ] Documentation explains file vs function scoring

## Technical Details

### New Data Structures

```rust
// src/priority/file_metrics.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDebtMetrics {
    pub path: PathBuf,
    pub total_lines: usize,
    pub function_count: usize,
    pub class_count: usize,
    pub avg_complexity: f64,
    pub max_complexity: u32,
    pub total_complexity: u32,
    pub coverage_percent: f64,
    pub uncovered_lines: usize,
    pub god_object_indicators: GodObjectIndicators,
    pub function_scores: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectIndicators {
    pub methods_count: usize,
    pub fields_count: usize,
    pub responsibilities: usize,
    pub is_god_object: bool,
    pub god_object_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDebtItem {
    pub metrics: FileDebtMetrics,
    pub score: f64,
    pub priority_rank: usize,
    pub recommendation: String,
    pub impact: FileImpact,
}
```

### File Scoring Algorithm

```rust
impl FileDebtMetrics {
    pub fn calculate_score(&self) -> f64 {
        // Size factor: larger files have higher impact
        let size_factor = (self.total_lines as f64 / 100.0).sqrt();
        
        // Complexity factor: average and total complexity
        let avg_complexity_factor = (self.avg_complexity / 5.0).min(3.0);
        let total_complexity_factor = (self.total_complexity as f64 / 50.0).sqrt();
        let complexity_factor = avg_complexity_factor * total_complexity_factor;
        
        // Coverage factor: lower coverage = higher score
        let coverage_gap = 1.0 - self.coverage_percent;
        let coverage_factor = (coverage_gap * 2.0) + 1.0;
        
        // Function density: too many functions = god object
        let density_factor = if self.function_count > 50 {
            1.0 + ((self.function_count - 50) as f64 * 0.02)
        } else {
            1.0
        };
        
        // God object multiplier
        let god_object_multiplier = if self.god_object_indicators.is_god_object {
            2.0 + self.god_object_indicators.god_object_score
        } else {
            1.0
        };
        
        // Aggregate function scores
        let function_score_sum: f64 = self.function_scores.iter().sum();
        let function_factor = (function_score_sum / 10.0).max(1.0);
        
        // Calculate final score
        size_factor 
            * complexity_factor 
            * coverage_factor 
            * density_factor 
            * god_object_multiplier 
            * function_factor
    }
}
```

### Integration Points

1. **Analyzer Integration**:
   ```rust
   // src/analyzers/mod.rs
   pub trait FileAnalyzer {
       fn analyze_file(&self, path: &Path) -> Result<FileDebtMetrics>;
       fn aggregate_functions(&self, functions: &[FunctionMetrics]) -> FileDebtMetrics;
   }
   ```

2. **Report Generation**:
   ```rust
   // src/priority/mod.rs
   pub enum DebtItem {
       Function(FunctionDebtItem),
       File(FileDebtItem),
       Module(ModuleDebtItem),
   }
   ```

3. **God Object Integration**:
   ```rust
   // src/organization/god_object_detector.rs
   impl GodObjectDetector {
       pub fn analyze_file_metrics(&self, path: &Path) -> GodObjectIndicators {
           // Use existing god object detection logic
           // Return indicators for file-level scoring
       }
   }
   ```

### File Analysis Pipeline

```rust
fn analyze_file_with_metrics(path: &Path) -> FileDebtItem {
    // 1. Parse file and extract functions
    let functions = extract_functions(path)?;
    
    // 2. Analyze each function (existing logic)
    let function_metrics: Vec<_> = functions.iter()
        .map(|f| analyze_function(f))
        .collect();
    
    // 3. Calculate file-level metrics
    let file_metrics = FileDebtMetrics {
        path: path.to_path_buf(),
        total_lines: count_lines(path),
        function_count: functions.len(),
        avg_complexity: calculate_average_complexity(&function_metrics),
        coverage_percent: get_file_coverage(path),
        god_object_indicators: GodObjectDetector::analyze_file_metrics(path),
        function_scores: function_metrics.iter().map(|m| m.score).collect(),
        ..Default::default()
    };
    
    // 4. Calculate file score
    let score = file_metrics.calculate_score();
    
    // 5. Generate recommendation
    let recommendation = generate_file_recommendation(&file_metrics);
    
    FileDebtItem {
        metrics: file_metrics,
        score,
        recommendation,
        ..Default::default()
    }
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 96 (Remove Score Capping) - needed for meaningful file scores
- **Affected Components**: 
  - Analyzer modules
  - Scoring system
  - Report generation
  - God object detector
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test file metric calculation
- **Integration Tests**: End-to-end file analysis
- **Validation Tests**: Verify known god objects score high
- **Performance Tests**: Measure impact on analysis time
- **Comparison Tests**: File vs function score correlation

## Documentation Requirements

- **Metric Definitions**: Document each file-level metric
- **Scoring Algorithm**: Explain file score calculation
- **User Guide**: How to interpret file vs function scores
- **API Documentation**: New FileDebtItem structure

## Implementation Notes

1. **Priority Ordering**:
   - Mix file and function items in recommendations
   - Sort by score regardless of type
   - Show type indicator in output

2. **Threshold Configuration**:
   ```toml
   [file_analysis]
   god_object_method_threshold = 15
   god_object_field_threshold = 10
   high_complexity_file_lines = 500
   ```

3. **Display Format**:
   ```
   #1 SCORE: 186.5 [FILE - GOD OBJECT]
   └─ src/analyzers/rust_call_graph.rs (3860 lines, 270 functions)
      ACTION: Break into 5+ modules: macro_expansion, call_resolution, graph_builder...
   
   #2 SCORE: 45.2 [FILE - HIGH COMPLEXITY]
   └─ src/complexity/entropy.rs (1663 lines, avg complexity: 12.3)
      ACTION: Extract complex functions, reduce file to <500 lines
   ```

## Migration and Compatibility

- File-level scoring is additive (doesn't replace function scoring)
- Existing function-level analysis unchanged
- New `--file-level-only` flag for file-only analysis
- Gradual rollout with feature flag if needed