---
number: 15
title: Automated Tech Debt Prioritization
category: optimization
priority: high
status: superseded
dependencies: [5, 8, 14]
created: 2025-08-10
superseded_by: 19
---

**Note**: This specification has been superseded by [Specification 19: Unified Debt Prioritization with Semantic Analysis](19-unified-debt-prioritization-with-semantic-analysis.md), which combines the prioritization concepts from this spec with semantic analysis capabilities.

# Specification 15: Automated Tech Debt Prioritization

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [5 - Complexity-Coverage Risk Analysis, 8 - Testing Prioritization, 14 - Dependency-Aware ROI]

## Context

Currently, debtmap provides comprehensive technical debt analysis with metrics like complexity scores, test coverage, ROI calculations, and risk assessments. However, determining which specific technical debt items to address first requires manual interpretation of the output and following external prioritization rules (as documented in `.claude/commands/debtmap.md`). 

Users need clear, actionable guidance on what to fix first to maximize their return on investment. The prioritization logic that exists in external documentation should be codified directly within debtmap to provide immediate, prioritized recommendations.

## Objective

Implement an automated prioritization system within debtmap that analyzes all detected technical debt and provides a clear, prioritized list of items to fix, ordered by return on investment (ROI) and impact on codebase health.

## Requirements

### Functional Requirements

1. **Automated Prioritization Engine**
   - Implement a multi-tier prioritization system based on ROI and impact
   - Consider test coverage gaps, complexity hotspots, and critical risk functions
   - Apply weighted scoring across different debt categories

2. **Clear Priority Ordering**
   - Rank all technical debt items by calculated priority score
   - Group items into priority tiers: Critical, High, Medium, Low
   - Provide clear justification for each item's priority level

3. **ROI-Based Scoring**
   - Calculate effort-to-impact ratio for each debt item
   - Consider cascade effects from dependencies
   - Factor in module criticality (entry points, core modules, APIs)
   - Account for risk reduction potential

4. **Actionable Recommendations**
   - Generate specific, actionable fix recommendations for top items
   - Include effort estimates (trivial, simple, moderate, complex)
   - Provide implementation guidance for each recommendation type
   - Show expected impact metrics (coverage increase, complexity reduction)

5. **Prioritization Rules**
   - **Tier 1**: Testing gaps with ROI ≥ 5
   - **Tier 2**: Critical risk functions (high complexity, zero coverage)
   - **Tier 3**: Complexity hotspots exceeding thresholds
   - **Tier 4**: High-priority debt items (large modules, deep nesting)
   - **Tier 5**: Code duplication and coupling issues

### Non-Functional Requirements

1. **Performance**
   - Prioritization should add < 100ms to analysis time
   - Support incremental prioritization updates
   - Efficient sorting algorithms for large debt lists

2. **Clarity**
   - Output must be immediately understandable
   - Use visual indicators for priority levels
   - Provide concise explanations for prioritization decisions

3. **Configurability**
   - Allow customization of prioritization weights
   - Support different prioritization strategies (ROI, risk, effort)
   - Enable filtering by debt categories

## Acceptance Criteria

- [ ] Automated prioritization engine ranks all debt items by calculated priority
- [ ] Output includes a "TOP PRIORITIES" section with 5-10 actionable items
- [ ] Each priority item shows:
  - [ ] Priority rank and tier
  - [ ] Specific file and location
  - [ ] Type of debt (test gap, complexity, duplication, etc.)
  - [ ] Effort estimate
  - [ ] Expected impact (metrics that will improve)
  - [ ] ROI score
  - [ ] Clear fix recommendation
- [ ] Prioritization considers all factors: ROI, risk, complexity, coverage, dependencies
- [ ] Configuration file allows customization of prioritization weights
- [ ] Performance impact is minimal (< 100ms for 1000 debt items)
- [ ] Output format clearly distinguishes prioritized recommendations from raw analysis

## Technical Details

### Implementation Approach

1. **Priority Scoring Algorithm**
   ```rust
   pub struct PriorityScore {
       roi: f64,           // Return on investment (0-10)
       risk: f64,          // Risk score (0-10)
       effort: f64,        // Effort estimate (1-10, inverse)
       impact: f64,        // Potential impact (0-10)
       dependencies: f64,  // Dependency factor (0-10)
   }
   
   pub fn calculate_priority(item: &DebtItem) -> f64 {
       let score = PriorityScore::from(item);
       score.roi * 0.35 +
       score.risk * 0.25 +
       (10.0 - score.effort) * 0.20 +
       score.impact * 0.15 +
       score.dependencies * 0.05
   }
   ```

2. **Prioritization Pipeline**
   - Collect all debt items from analysis
   - Calculate priority scores for each item
   - Apply tiered categorization rules
   - Sort by priority within each tier
   - Generate actionable recommendations for top items

3. **Recommendation Generator**
   - Map debt types to fix templates
   - Include context-specific guidance
   - Estimate effort based on complexity metrics
   - Calculate expected improvements

### Architecture Changes

1. **New Module**: `src/priority/`
   - `mod.rs`: Priority types and traits
   - `scorer.rs`: Priority scoring algorithms
   - `recommender.rs`: Recommendation generation
   - `formatter.rs`: Priority output formatting

2. **Integration Points**
   - Hook into existing risk analysis pipeline
   - Leverage ROI calculations from spec 14
   - Use effort estimates from spec 8
   - Access debt items from all analyzers

### Data Structures

```rust
pub struct PrioritizedDebt {
    pub item: DebtItem,
    pub priority_score: f64,
    pub tier: PriorityTier,
    pub recommendation: Recommendation,
    pub effort_estimate: EffortLevel,
    pub expected_impact: ImpactMetrics,
}

pub struct Recommendation {
    pub action: String,
    pub rationale: String,
    pub implementation_hints: Vec<String>,
    pub related_items: Vec<DebtItem>,
}

pub enum PriorityTier {
    Critical,  // Must fix immediately
    High,      // Fix in current sprint
    Medium,    // Plan for next milestone
    Low,       // Consider when convenient
}
```

### APIs and Interfaces

```rust
pub trait PriorityStrategy {
    fn calculate_priority(&self, item: &DebtItem, context: &AnalysisContext) -> f64;
    fn generate_recommendation(&self, item: &DebtItem) -> Recommendation;
}

pub struct PriorityEngine {
    strategy: Box<dyn PriorityStrategy>,
    config: PriorityConfig,
}

impl PriorityEngine {
    pub fn prioritize(&self, items: Vec<DebtItem>) -> Vec<PrioritizedDebt>;
    pub fn get_top_priorities(&self, count: usize) -> Vec<PrioritizedDebt>;
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 5: Complexity-Coverage Risk Analysis (for risk scores)
  - Spec 8: Testing Prioritization (for ROI calculations)
  - Spec 14: Dependency-Aware ROI (for cascade effects)
- **Affected Components**:
  - `src/cli.rs`: New CLI flags for prioritization control
  - `src/io/output.rs`: Enhanced output formatting
  - `src/core/mod.rs`: Extended AnalysisResults structure
- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - Priority scoring algorithm with various debt types
  - Tiered categorization logic
  - Recommendation generation for each debt category
  - Edge cases (empty lists, single items, ties)

- **Integration Tests**:
  - End-to-end prioritization with real codebase analysis
  - Verify priority ordering matches expected rules
  - Test configuration overrides
  - Validate output formatting

- **Performance Tests**:
  - Benchmark prioritization of 1000+ debt items
  - Measure memory usage for large debt lists
  - Verify < 100ms overhead requirement

- **User Acceptance**:
  - Confirm recommendations are actionable and clear
  - Validate priority ordering makes practical sense
  - Ensure effort estimates are realistic

## Documentation Requirements

- **Code Documentation**:
  - Document prioritization algorithm and scoring factors
  - Explain recommendation generation logic
  - Provide examples of each priority tier

- **User Documentation**:
  - Add "Understanding Priorities" section to README
  - Document configuration options for customization
  - Provide workflow examples for using priorities

- **Architecture Updates**:
  - Update ARCHITECTURE.md with priority module details
  - Document integration with existing analysis pipeline

## Implementation Notes

1. **Incremental Rollout**:
   - Start with basic prioritization based on existing ROI
   - Add sophisticated scoring in phases
   - Gather user feedback to refine weights

2. **Customization Considerations**:
   - Some teams may prioritize security over performance
   - Allow strategy plugins for domain-specific prioritization
   - Support priority overrides via configuration

3. **Output Integration**:
   - Prioritized recommendations should appear prominently
   - Consider separate `--priorities-only` flag for focused output
   - Maintain backward compatibility with existing output formats

## Migration and Compatibility

- **Breaking Changes**: None - prioritization is additive
- **Configuration Migration**: New priority section in config file
- **Output Compatibility**: Existing parsers will work unchanged
- **API Stability**: New APIs are additive, no changes to existing interfaces

## Example Output

```
═══════════════════════════════════════════
         PRIORITIZED TECH DEBT FIXES
═══════════════════════════════════════════

🎯 TOP 5 FIXES (Ordered by ROI)
───────────────────────────────────────────

PRIORITY #1 - CRITICAL (ROI: 9.8)
├─ Type: Testing Gap - Zero Coverage
├─ Location: src/core/cache.rs:14::default()
├─ Effort: Trivial (1-3 test cases)
├─ Impact: +54% module coverage, -35% risk score
├─ Fix: Add unit tests for default initialization
└─ Rationale: Core module with high dependencies, trivial to test

PRIORITY #2 - CRITICAL (ROI: 8.5)
├─ Type: Complexity Hotspot
├─ Location: src/risk/insights.rs:4::format_recommendations()
├─ Effort: Moderate (2-3 hours)
├─ Impact: -11 cyclomatic complexity, -15 cognitive complexity
├─ Fix: Extract formatting logic into separate functions
└─ Rationale: Most complex function, affects output quality

PRIORITY #3 - HIGH (ROI: 7.2)
├─ Type: Code Duplication
├─ Location: src/analyzers/rust.rs (3 instances)
├─ Effort: Simple (1 hour)
├─ Impact: -150 lines of code, improved maintainability
├─ Fix: Extract common parsing logic to shared function
└─ Rationale: Reduces maintenance burden, prevents drift

[Additional priorities 4-5...]

💡 QUICK WINS (< 30 minutes each)
───────────────────────────────────────────
• Add missing tests for 5 trivial functions
• Fix 3 TODO comments in critical paths
• Remove 2 instances of dead code

📊 SUMMARY
───────────────────────────────────────────
Total fixes recommended: 15
Estimated effort: 12 hours
Expected improvements:
  - Coverage: +18%
  - Complexity: -25%
  - Risk Score: -42%
  - Lines of Code: -200
```