---
number: 106
title: Categorical Debt Grouping and Domain-Specific Analysis
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-09-26
---

# Specification 106: Categorical Debt Grouping and Domain-Specific Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Current debtmap output mixes architectural issues, testing gaps, complexity hotspots, and performance concerns in a single flat list, making it difficult for teams to understand the nature of their technical debt and plan domain-specific improvements.

Analysis shows debtmap detects 25+ distinct debt types (GodObject, TestingGap, ComplexityHotspot, AsyncMisuse, etc.) but presents them without categorical context. Teams need to understand whether their primary debt is architectural (requiring refactoring), testing-related (requiring QA investment), or performance-based (requiring optimization focus).

Real output analysis reveals:
- 1 critical god object issue (architectural)
- 8 similar untested functions (testing process)
- Mixed complexity and performance patterns (optimization)

Teams benefit from understanding debt distribution across domains to allocate resources appropriately and plan targeted improvement initiatives.

## Objective

Implement categorical grouping of debt items by domain impact (Architecture, Testing, Performance, Code Quality), providing teams with strategic insight into debt distribution and enabling domain-focused planning and resource allocation.

## Requirements

### Functional Requirements

1. **Domain Category Classification**
   - **Architecture Issues**: GodObject, FeatureEnvy, PrimitiveObsession, circular dependencies
   - **Testing Gaps**: TestingGap, TestComplexityHotspot, FlakyTestPattern, AssertionComplexity
   - **Performance Issues**: AsyncMisuse, CollectionInefficiency, NestedLoops, BlockingIO
   - **Code Quality**: ComplexityHotspot, Duplication, MagicValues, ErrorSwallowing

2. **Category-Specific Metrics**
   - Total debt score per category
   - Average severity per category
   - Count of items per category
   - Estimated effort per category
   - Business impact assessment per category

3. **Domain-Specific Recommendations**
   - Architecture: Refactoring strategies, design pattern suggestions
   - Testing: Coverage targets, test automation recommendations
   - Performance: Optimization priorities, profiling suggestions
   - Code Quality: Review process improvements, tooling recommendations

4. **Category Display Organization**
   - Categories ordered by total impact (highest debt score first)
   - Within-category ordering by individual item priority
   - Category summaries with key statistics
   - Cross-category dependency identification

### Non-Functional Requirements

- Category assignment must be deterministic and consistent
- Support for future debt type additions without breaking changes
- Configurable category definitions for different team priorities
- Performance impact < 50ms for categorization operations

## Acceptance Criteria

- [ ] All existing debt types are assigned to exactly one primary category
- [ ] Categories are displayed in order of total debt impact
- [ ] Each category shows total score, item count, and estimated effort
- [ ] Category headers include domain-specific context and recommendations
- [ ] Individual items within categories maintain priority ordering
- [ ] Empty categories are omitted from output
- [ ] Category summaries include actionable strategic guidance
- [ ] Cross-category dependencies are identified and highlighted
- [ ] Unknown/future debt types default to "Code Quality" category
- [ ] Configuration allows customizing category assignments

## Technical Details

### Implementation Approach

1. **Category Classification System**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DebtCategory {
    Architecture,
    Testing,
    Performance,
    CodeQuality,
}

impl DebtCategory {
    fn from_debt_type(debt_type: &DebtType) -> Self {
        match debt_type {
            DebtType::GodObject { .. } => DebtCategory::Architecture,
            DebtType::FeatureEnvy { .. } => DebtCategory::Architecture,
            DebtType::TestingGap { .. } => DebtCategory::Testing,
            DebtType::AsyncMisuse { .. } => DebtCategory::Performance,
            DebtType::ComplexityHotspot { .. } => DebtCategory::CodeQuality,
            // ... complete mapping
        }
    }
}
```

2. **Category Aggregation**
```rust
#[derive(Debug, Clone)]
pub struct CategorySummary {
    pub category: DebtCategory,
    pub total_score: f64,
    pub item_count: usize,
    pub estimated_effort_hours: u32,
    pub average_severity: f64,
    pub top_items: Vec<DebtItem>,
}

pub struct CategorizedDebt {
    pub categories: BTreeMap<DebtCategory, CategorySummary>,
    pub cross_category_dependencies: Vec<CrossCategoryDependency>,
}
```

3. **Domain-Specific Recommendations**
```rust
impl DebtCategory {
    fn strategic_guidance(&self, summary: &CategorySummary) -> String {
        match self {
            DebtCategory::Architecture => {
                format!("Focus on breaking down {} complex components. Consider design patterns and dependency injection.", summary.item_count)
            },
            DebtCategory::Testing => {
                format!("Implement {} missing tests. Target {}% coverage improvement with focus on critical paths.", summary.item_count, summary.estimated_coverage_gain())
            },
            // ... category-specific guidance
        }
    }
}
```

### Architecture Changes

- Add `DebtCategory` enum to `priority/mod.rs`
- Create `CategoryAnalyzer` component for grouping and analysis
- Extend `UnifiedAnalysis` with categorized view
- Add category-specific formatters to `io/writers/markdown/`

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct CrossCategoryDependency {
    pub source_category: DebtCategory,
    pub target_category: DebtCategory,
    pub description: String,
    pub impact_level: ImpactLevel,
}

#[derive(Debug, Clone)]
pub enum ImpactLevel {
    Critical,  // Blocks progress in target category
    High,      // Significantly affects target category
    Medium,    // Some effect on target category
    Low,       // Minor interaction
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `priority/mod.rs` - Add category definitions
  - `io/writers/markdown/enhanced.rs` - Add categorical display
  - `priority/unified_scorer.rs` - Include category in analysis
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test category assignment for all existing debt types
  - Test category aggregation with mixed debt items
  - Test strategic guidance generation
  - Test cross-category dependency detection

- **Integration Tests**:
  - Full categorized analysis with realistic codebase
  - Verify category ordering by impact
  - Test empty category handling
  - Validate effort estimation aggregation

- **Performance Tests**:
  - Measure categorization overhead with large debt sets
  - Verify sub-50ms performance requirement

- **User Acceptance**:
  - Compare categorized vs uncategorized output for strategic planning
  - Validate domain expert accuracy of categorization
  - Measure improvement in resource allocation decisions

## Documentation Requirements

- **Code Documentation**: Document category assignment rationale and extensibility
- **User Documentation**: Explain category interpretation and strategic planning usage
- **Architecture Updates**: Document categorization flow and extension points

## Implementation Notes

1. **Category Assignment Philosophy**:
   - Primary impact determines category (god object = architecture, not complexity)
   - Testing debt includes test code issues and missing coverage
   - Performance includes algorithmic and resource management issues
   - Code quality covers readability, maintainability, and general best practices

2. **Strategic Guidance Principles**:
   - Architecture category: Focus on design and component organization
   - Testing category: Emphasize coverage, automation, and quality processes
   - Performance category: Highlight optimization and efficiency concerns
   - Code quality: Address maintainability and developer experience

3. **Cross-Category Dependencies**:
   - Architecture debt often blocks effective testing (god objects hard to test)
   - Performance issues may require architectural changes
   - Complex code affects both quality and testability
   - Identify and highlight these relationships for planning

4. **Extensibility Considerations**:
   - New debt types should easily map to categories
   - Categories should be configurable for different team priorities
   - Strategic guidance should be template-based for customization

## Migration and Compatibility

During prototype phase: This feature is additive and maintains full backward compatibility. Existing output formats continue unchanged unless categorized display is explicitly requested. New category information is added alongside existing priority rankings without modifying core debt item structures.