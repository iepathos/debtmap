---
number: 100
title: Integrate God Object Detection into Main Scoring
category: optimization
priority: critical
status: draft
dependencies: [97, 99]
created: 2025-09-07
---

# Specification 100: Integrate God Object Detection into Main Scoring

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: [97 - File-Level Scoring, 99 - Aggregate Function Scores]

## Context

The god object detector exists in the codebase but is not integrated into the main scoring pipeline. This means files like `rust_call_graph.rs` with 270 functions and 3,860 lines are not being identified as god objects requiring urgent refactoring. The detector has configurable thresholds for methods, fields, and responsibilities but its output is not factored into debt scores. Integrating this detection would immediately surface the most problematic architectural issues.

## Objective

Fully integrate the existing god object detection logic into the main debt scoring pipeline, ensuring god objects receive dramatically higher scores and appear at the top of prioritization lists with specific refactoring recommendations.

## Requirements

### Functional Requirements
- Integrate GodObjectDetector into main analysis flow
- Apply god object scoring to both classes and files
- Generate specific refactoring recommendations for god objects
- Support language-specific god object patterns
- Track god object metrics over time

### Non-Functional Requirements
- Zero false positives for god object detection
- Clear explanation of why something is a god object
- Configurable thresholds per language/project
- Minimal performance impact (<2% overhead)

## Acceptance Criteria

- [ ] God object detection runs for every analyzed file
- [ ] God objects score minimum 100 points
- [ ] rust_call_graph.rs identified as god object
- [ ] Specific module split recommendations generated
- [ ] God object indicators shown in output
- [ ] Configuration supports threshold customization
- [ ] Tests verify known god objects detected
- [ ] Documentation explains god object criteria

## Technical Details

### Integration Architecture

```rust
// src/analyzers/mod.rs
pub trait EnhancedAnalyzer {
    fn analyze_with_patterns(&self, path: &Path) -> AnalysisResult {
        let functions = self.analyze_functions(path)?;
        let god_object = self.detect_god_object(path, &functions)?;
        let organization = self.detect_organization_issues(path)?;
        
        AnalysisResult {
            functions,
            god_object,
            organization,
            file_metrics: self.calculate_file_metrics(path, &functions),
        }
    }
    
    fn detect_god_object(&self, path: &Path, functions: &[FunctionMetrics]) -> GodObjectResult;
}
```

### Enhanced God Object Detection

```rust
// src/organization/god_object_detector.rs
impl GodObjectDetector {
    pub fn analyze_comprehensive(&self, path: &Path) -> GodObjectAnalysis {
        let ast = parse_file(path)?;
        
        // For Rust files
        let analysis = if path.extension() == Some("rs") {
            self.analyze_rust_file(&ast)
        } else if path.extension() == Some("py") {
            self.analyze_python_file(&ast)
        } else {
            self.analyze_generic_file(path)
        };
        
        GodObjectAnalysis {
            is_god_object: analysis.exceeds_thresholds(),
            method_count: analysis.methods.len(),
            field_count: analysis.fields.len(),
            responsibility_count: analysis.responsibilities.len(),
            lines_of_code: analysis.loc,
            complexity_sum: analysis.total_complexity,
            god_object_score: self.calculate_god_object_score(&analysis),
            recommended_splits: self.recommend_module_splits(&analysis),
            confidence: self.calculate_confidence(&analysis),
        }
    }
    
    fn calculate_god_object_score(&self, analysis: &TypeAnalysis) -> f64 {
        let method_factor = (analysis.method_count as f64 / self.max_methods as f64).min(3.0);
        let field_factor = (analysis.field_count as f64 / self.max_fields as f64).min(3.0);
        let responsibility_factor = (analysis.responsibilities.len() as f64 / 3.0).min(3.0);
        let size_factor = (analysis.loc as f64 / 500.0).min(3.0);
        
        // Exponential scaling for severe violations
        let base_score = method_factor * field_factor * responsibility_factor * size_factor;
        
        if base_score > 2.0 {
            base_score.powf(2.0) * 25.0  // Minimum 100 for clear god objects
        } else {
            base_score * 10.0
        }
    }
    
    fn recommend_module_splits(&self, analysis: &TypeAnalysis) -> Vec<ModuleSplit> {
        let mut recommendations = Vec::new();
        
        // Group methods by responsibility
        let responsibility_groups = self.group_by_responsibility(&analysis.methods);
        
        for (responsibility, methods) in responsibility_groups {
            if methods.len() > 5 {
                recommendations.push(ModuleSplit {
                    suggested_name: format!("{}_{}", 
                        analysis.type_name.to_lowercase(),
                        responsibility.to_lowercase()
                    ),
                    methods_to_move: methods.clone(),
                    responsibility,
                    estimated_lines: self.estimate_lines(&methods),
                });
            }
        }
        
        recommendations
    }
}
```

### Scoring Integration

```rust
// src/priority/unified_scorer.rs
pub fn calculate_unified_score_with_patterns(
    func: &FunctionMetrics,
    god_object: Option<&GodObjectAnalysis>,
    coverage: Option<f64>,
    call_graph: &CallGraph,
) -> UnifiedScore {
    let base_score = calculate_base_score(func, coverage, call_graph);
    
    // Apply god object multiplier
    let god_object_multiplier = if let Some(go) = god_object {
        if go.is_god_object {
            // Massive boost for functions in god objects
            3.0 + (go.god_object_score / 50.0)
        } else {
            1.0
        }
    } else {
        1.0
    };
    
    UnifiedScore {
        base_score: base_score * god_object_multiplier,
        god_object_indicators: god_object.cloned(),
        ..calculate_standard_score(func, coverage, call_graph)
    }
}
```

### Recommendation Generation

```rust
// src/priority/recommendations.rs
pub fn generate_god_object_recommendation(
    analysis: &GodObjectAnalysis,
    path: &Path,
) -> DetailedRecommendation {
    DetailedRecommendation {
        severity: Severity::Critical,
        
        title: format!(
            "🚨 GOD OBJECT: {} ({} methods, {} fields, {} responsibilities)",
            path.file_name().unwrap().to_str().unwrap(),
            analysis.method_count,
            analysis.field_count,
            analysis.responsibility_count
        ),
        
        description: format!(
            "This file/class has grown too large and handles too many responsibilities. \
             With {} lines and {} total complexity, it's become difficult to maintain, \
             test, and understand. This is the #1 priority for refactoring.",
            analysis.lines_of_code,
            analysis.complexity_sum
        ),
        
        action_items: vec![
            "Break into smaller, focused modules:".to_string(),
            analysis.recommended_splits.iter()
                .map(|split| format!("  - {} ({} methods, ~{} lines)",
                    split.suggested_name,
                    split.methods_to_move.len(),
                    split.estimated_lines
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            "Apply SOLID principles, especially Single Responsibility".to_string(),
            "Create interfaces/traits for better abstraction".to_string(),
        ],
        
        estimated_effort: EffortEstimate::High,
        
        impact: ImpactAssessment {
            complexity_reduction: analysis.complexity_sum as i32 / 2,
            maintainability_improvement: 80,
            testability_improvement: 70,
            risk_reduction: 90,
        },
    }
}
```

### Configuration

```toml
# debtmap.toml
[god_object_detection]
enabled = true

[god_object_detection.rust]
max_methods = 20
max_fields = 15
max_traits = 5
max_lines = 1000
max_complexity = 200

[god_object_detection.python]
max_methods = 15
max_fields = 10
max_lines = 500
max_complexity = 150

[god_object_detection.javascript]
max_methods = 15
max_properties = 20
max_lines = 500
```

### Display Format

```
#1 SCORE: 432.5 [🚨 GOD OBJECT]
   └─ src/analyzers/rust_call_graph.rs
   
   📊 GOD OBJECT METRICS:
   ├─ Methods: 270 (max: 20)
   ├─ Fields: 18 (max: 15)
   ├─ Responsibilities: 6 (max: 3)
   ├─ Lines: 3,860 (max: 1,000)
   └─ Total Complexity: 587
   
   🔧 RECOMMENDED REFACTORING:
   Split into focused modules:
   ├─ call_graph_macro_expansion.rs (~450 lines, 35 methods)
   │  └─ MacroExpansionStats, MacroHandlingConfig, expand_* methods
   ├─ call_graph_resolution.rs (~380 lines, 28 methods)
   │  └─ UnresolvedCall, resolve_* methods
   ├─ call_graph_builder.rs (~520 lines, 42 methods)
   │  └─ CallGraphExtractor, build_* methods
   ├─ call_graph_traits.rs (~290 lines, 22 methods)
   │  └─ Trait resolution methods
   └─ call_graph_analysis.rs (~310 lines, 25 methods)
      └─ Analysis and traversal methods
   
   ⚡ IMPACT: -290 complexity, +80% maintainability, +70% testability
```

## Dependencies

- **Prerequisites**: 
  - Spec 97 (File-Level Scoring) - for file metrics
  - Spec 99 (Aggregate Scores) - for comprehensive file analysis
- **Affected Components**: 
  - All language analyzers
  - God object detector
  - Scoring pipeline
  - Recommendation system
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test god object scoring calculation
- **Integration Tests**: End-to-end god object detection
- **Validation Tests**: Known god objects detected correctly
- **Language Tests**: Test each language's detection
- **Threshold Tests**: Verify configurable thresholds work

## Documentation Requirements

- **God Object Guide**: What makes a god object
- **Refactoring Guide**: How to split god objects
- **Configuration Guide**: Setting appropriate thresholds
- **Best Practices**: Preventing god objects

## Implementation Notes

1. **Gradual Detection**:
   ```rust
   // Support different confidence levels
   pub enum GodObjectConfidence {
       Definite,     // Exceeds all thresholds
       Probable,     // Exceeds most thresholds
       Possible,     // Exceeds some thresholds
       NotGodObject, // Within acceptable limits
   }
   ```

2. **Caching**:
   - Cache god object analysis per file
   - Invalidate on file change
   - Store in analysis cache

3. **Performance**:
   - Run god object detection in parallel
   - Skip detection for small files (<100 lines)
   - Use AST visitor pattern for efficiency

## Migration and Compatibility

- God object detection enabled by default
- Can disable with `--no-god-object-detection`
- Existing scores will change significantly
- Provide transition period with warnings