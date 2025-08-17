---
number: 44
title: Enhanced Scoring Differentiation
category: optimization
priority: high
status: draft
dependencies: [35, 14, 24]
created: 2025-01-17
---

# Specification 44: Enhanced Scoring Differentiation

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [35, 14, 24]

## Context

The current scoring system produces monotonous scores where many items have identical values (e.g., 8.3 for all performance issues, 7.8 for all security issues). This poor differentiation makes it difficult for developers to identify which issues are truly most important. The scoring algorithm needs more granular factors to create meaningful distinctions between debt items.

Additionally, test code often dominates the results despite being less critical than production code issues. The system needs to better weight the importance of code based on its role and criticality in the system.

## Objective

Implement a more sophisticated scoring algorithm that produces well-differentiated scores based on multiple factors including function criticality, actual complexity versus theoretical patterns, call frequency, dependency impact, and production versus test code distinction. The new system should produce scores with meaningful variation to guide developers to the most impactful improvements.

## Requirements

### Functional Requirements

1. **Multi-Factor Scoring System**
   - Function criticality based on call graph position
   - Actual measured complexity vs pattern-based detection
   - Call frequency and hot path detection
   - Dependency fan-out and cascade impact
   - Code coverage correlation with complexity
   - Change frequency from git history

2. **Production vs Test Weighting**
   - Separate scoring tracks for production and test code
   - Configurable weight multipliers (default: production 1.0, test 0.3)
   - Option to exclude test code from main report
   - Test-specific debt scoring focused on test quality

3. **Dynamic Score Calculation**
   - Real-time score calculation based on current codebase state
   - Score normalization to 0-10 scale with good distribution
   - Percentile-based scoring for relative importance
   - Exponential decay for cascade effects

4. **Hot Path Detection**
   - Identify frequently called functions via call graph analysis
   - Performance profiling integration (optional)
   - Entry point distance calculation
   - Critical path highlighting

5. **Granular Severity Levels**
   - Replace binary (High/Medium/Low) with continuous scale
   - Factor-specific contributions visible in output
   - Confidence scores for each factor
   - Explanation of why score is high/low

### Non-Functional Requirements

1. **Score Distribution**
   - Scores should follow roughly normal distribution
   - Top 10% of issues should have scores > 8.0
   - Bottom 50% should have scores < 5.0
   - Adjacent items should rarely have identical scores

2. **Performance**
   - Score calculation should add < 10% to analysis time
   - Caching of intermediate calculations
   - Incremental updates when possible

3. **Explainability**
   - Each score component should be traceable
   - Provide breakdown of score calculation
   - Clear rationale for score differences

## Acceptance Criteria

- [ ] No more than 5% of items have identical scores in top 100
- [ ] Score distribution spans at least 6 points (e.g., 3.2 to 9.5)
- [ ] Production code issues score higher than equivalent test issues
- [ ] Hot path functions receive higher scores for same issue type
- [ ] Score calculation breakdown is available via --detailed flag
- [ ] Test code can be excluded via --exclude-tests flag
- [ ] Scores are reproducible given same codebase state
- [ ] Performance overhead is less than 10%
- [ ] Documentation explains all scoring factors
- [ ] Integration tests validate score differentiation

## Technical Details

### Implementation Approach

1. **Enhanced Scoring Formula**
   ```rust
   pub struct EnhancedScore {
       pub base_score: f64,        // Issue severity (1-10)
       pub criticality: f64,        // Function importance (0-2)
       pub complexity_factor: f64,  // Actual complexity (0-2)
       pub coverage_factor: f64,    // Coverage correlation (0-2)
       pub dependency_factor: f64,  // Downstream impact (0-2)
       pub frequency_factor: f64,   // Call/change frequency (0-2)
       pub test_weight: f64,        // Production vs test (0.3-1.0)
       pub confidence: f64,         // Scoring confidence (0-1)
   }
   
   impl EnhancedScore {
       pub fn calculate(&self) -> f64 {
           let raw = self.base_score 
               * self.criticality 
               * self.complexity_factor
               * self.coverage_factor
               * self.dependency_factor
               * self.frequency_factor
               * self.test_weight;
           
           // Normalize to 0-10 scale with good distribution
           Self::normalize(raw)
       }
   }
   ```

2. **Criticality Analysis**
   ```rust
   pub struct CriticalityAnalyzer {
       call_graph: CallGraph,
       entry_points: Vec<FunctionId>,
       hot_paths: HashSet<FunctionId>,
   }
   
   impl CriticalityAnalyzer {
       pub fn calculate_criticality(&self, function: &Function) -> f64 {
           let mut score = 1.0;
           
           // Distance from entry points
           if let Some(distance) = self.distance_from_entry(function) {
               score *= 2.0 / (1.0 + distance as f64);
           }
           
           // Number of callers (fan-in)
           let caller_count = self.call_graph.callers(function).count();
           score *= 1.0 + (caller_count as f64).ln();
           
           // Hot path bonus
           if self.hot_paths.contains(&function.id) {
               score *= 1.5;
           }
           
           score.min(2.0) // Cap at 2x multiplier
       }
   }
   ```

3. **Distribution Normalization**
   ```rust
   pub struct ScoreNormalizer {
       percentiles: Vec<f64>,
   }
   
   impl ScoreNormalizer {
       pub fn normalize(&self, raw_score: f64) -> f64 {
           // Map to percentile-based 0-10 scale
           let percentile = self.find_percentile(raw_score);
           
           // Use sigmoid for smooth distribution
           10.0 * (1.0 / (1.0 + (-0.1 * (percentile - 50.0)).exp()))
       }
   }
   ```

### Architecture Changes

1. Add `scoring` module with enhanced scoring system
2. Integrate with existing priority module
3. Add criticality analysis to call graph module
4. Extend debt items with scoring breakdown

### Data Structures

```rust
pub struct ScoringContext {
    pub call_graph: CallGraph,
    pub coverage_map: Option<CoverageMap>,
    pub git_history: Option<GitHistory>,
    pub hot_paths: HashSet<FunctionId>,
    pub test_files: HashSet<PathBuf>,
}

pub struct ScoreBreakdown {
    pub total: f64,
    pub components: HashMap<String, f64>,
    pub explanation: String,
    pub confidence: f64,
}
```

### APIs and Interfaces

```rust
pub trait EnhancedScorer {
    fn score_with_context(
        &self,
        item: &DebtItem,
        context: &ScoringContext,
    ) -> ScoreBreakdown;
}
```

## Dependencies

- **Prerequisites**:
  - Spec 35: Debt Pattern Unified Scoring (base scoring system)
  - Spec 14: Dependency-Aware ROI (cascade calculations)
  - Spec 24: Refined Risk Scoring (statistical baselines)

- **Affected Components**:
  - Priority module
  - Debt detection system
  - Output formatters
  - CLI interface

- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - Score calculation with various factors
  - Distribution normalization
  - Criticality analysis accuracy

- **Integration Tests**:
  - Score differentiation on sample codebases
  - Production vs test weighting
  - Hot path detection validation

- **Statistical Tests**:
  - Verify score distribution properties
  - Check for sufficient variation
  - Validate percentile calculations

- **Performance Tests**:
  - Measure scoring overhead
  - Cache effectiveness
  - Large codebase performance

## Documentation Requirements

- **Code Documentation**:
  - Explain each scoring factor
  - Document normalization algorithm
  - Provide scoring examples

- **User Documentation**:
  - Guide to understanding scores
  - Configuration options for weighting
  - How to interpret score breakdowns

- **Architecture Updates**:
  - Document scoring module design
  - Explain integration with existing systems
  - Describe caching strategy

## Implementation Notes

1. Start with basic multi-factor scoring, add factors incrementally
2. Use existing call graph for criticality analysis
3. Consider machine learning for weight optimization in future
4. Ensure scores are deterministic for testing
5. Provide verbose mode showing all score components

## Migration and Compatibility

- New scoring is default, with --legacy-scoring flag for old behavior
- Existing priority thresholds may need adjustment
- Score explanation helps users understand changes
- Gradual rollout with A/B testing possible
- Configuration migration tool for custom thresholds