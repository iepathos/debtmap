# Interpreting Results

## Interpreting Results

### Understanding Output Formats

Debtmap provides three output formats:

**Terminal** (default): Human-readable with colors and tables
```bash
debtmap analyze .
```

**JSON**: Machine-readable for CI/CD integration
```bash
debtmap analyze . --format json --output report.json
```

**Markdown**: Documentation-friendly
```bash
debtmap analyze . --format markdown --output report.md
```

### JSON Structure

```json
{
  "timestamp": "2025-10-09T12:00:00Z",
  "project_path": "/path/to/project",
  "complexity": {
    "metrics": [
      {
        "name": "process_data",
        "file": "src/main.rs",
        "line": 42,
        "cyclomatic": 15,
        "cognitive": 22,
        "est_branches": 20,
        "nesting": 4,
        "length": 68,
        "is_test": false,
        "visibility": "Public",
        "is_trait_method": false,
        "in_test_module": false,
        "entropy_score": {
          "token_entropy": 0.65,
          "pattern_repetition": 0.25,
          "branch_similarity": 0.30,
          "effective_complexity": 0.85
        },
        "is_pure": false,
        "purity_confidence": 0.8,
        "detected_patterns": ["validation_pattern"],
        "upstream_callers": ["main", "process_request"],
        "downstream_callees": ["validate", "save", "notify"]
      }
    ],
    "summary": {
      "total_functions": 150,
      "average_complexity": 5.3,
      "max_complexity": 22,
      "high_complexity_count": 8
    }
  },
  "technical_debt": {
    "items": [
      {
        "id": "complexity_src_main_rs_42",
        "debt_type": "Complexity",
        "priority": "High",
        "file": "src/main.rs",
        "line": 42,
        "column": 1,
        "message": "Function exceeds complexity threshold",
        "context": "Cyclomatic: 15, Cognitive: 22"
      }
    ],
    "by_type": {
      "Complexity": [...],
      "Duplication": [...],
      "Todo": [...]
    }
  }
}
```

#### JSON Output Format Variants

Debtmap supports two JSON output format variants for different integration needs:

**Legacy Format (default):**
- Uses wrapper objects: `{"File": {...}}` and `{"Function": {...}}`
- Compatible with existing tooling and scripts
- Shown in the JSON structure example above

**Unified Format (spec 108 - future enhancement):**
- Uses consistent structure with `"type"` field discriminator
- Simpler parsing for new integrations
- Example structure:
```json
{
  "type": "function",
  "name": "process_data",
  "file": "src/main.rs",
  "line": 42,
  "metrics": { /* ... */ }
}
```

**Note:** The unified format is currently an internal representation and is **not available** as a user-facing CLI option. The legacy format remains the stable default for all current integrations. If you need the unified format exposed as a CLI option (`--format json-unified`), please open a feature request on GitHub.

### Reading Function Metrics

**Key fields:**

- `cyclomatic`: Decision points - guides test case count
- `cognitive`: Understanding difficulty - guides refactoring priority
- `est_branches`: Estimated execution paths (formula: max(nesting_depth, 1) × cyclomatic ÷ 3) - approximates test cases needed for branch coverage
- `nesting`: Indentation depth - signals need for extraction
- `length`: Lines of code - signals SRP violations
- `visibility`: Function visibility (`"Private"`, `"Crate"`, or `"Public"` from FunctionVisibility enum)
- `is_pure`: No side effects - easier to test (Option type, may be None)
- `purity_confidence`: How certain we are about purity 0.0-1.0 (Option type, may be None)
- `is_trait_method`: Whether this function implements a trait method
- `in_test_module`: Whether function is inside a `#[cfg(test)]` module
- `detected_patterns`: Complexity adjustment patterns identified (e.g., "validation_pattern")
- `entropy_score`: Pattern analysis for false positive reduction
- `upstream_callers`: Impact radius if this function breaks
- `downstream_callees`: Functions this depends on

**Entropy interpretation:**
- `token_entropy < 0.4`: Repetitive code, likely pattern-based
- `pattern_repetition > 0.7`: High similarity between blocks
- `branch_similarity > 0.8`: Similar conditional branches
- `effective_complexity < 1.0`: Dampening applied

### Prioritizing Work

Debtmap provides multiple prioritization strategies, with **unified scoring (0-10 scale)** as the recommended default for most workflows:

**1. By Unified Score (default - recommended)**
```bash
debtmap analyze . --top 10
```
Shows top 10 items by **combined complexity, coverage, and dependency factors**, weighted and adjusted by function role.

**Why use unified scoring:**
- Balances complexity (40%), coverage (40%), and dependency impact (20%)
- Adjusts for function importance (entry points prioritized over utilities)
- Normalized 0-10 scale is intuitive and consistent
- Reduces false positives through coverage propagation
- Best for **sprint planning** and **function-level refactoring decisions**

**Example:**
```bash
# Show top 20 critical items
debtmap analyze . --min-priority 7.0 --top 20

# Focus on high-impact functions (score >= 7.0)
debtmap analyze . --format json | jq '.functions[] | select(.unified_score >= 7.0)'
```

**2. By Risk Category (legacy compatibility)**
```bash
debtmap analyze . --min-priority high
```
Shows only HIGH and CRITICAL priority items using legacy risk scoring.

**Note:** Legacy risk scoring uses additive formulas and unbounded scales. Prefer unified scoring for new workflows.

**3. By Debt Type**
```bash
debtmap analyze . --filter Architecture,Testing
```
Focuses on specific categories:
- `Architecture`: God objects, complexity, dead code
- `Testing`: Coverage gaps, test quality
- `Performance`: Resource leaks, inefficiencies
- `CodeQuality`: Code smells, maintainability

**4. By ROI (with coverage)**
```bash
debtmap analyze . --lcov coverage.lcov --top 20
```
Prioritizes by return on investment for testing/refactoring. Combines unified scoring with test effort estimates to identify high-value work.

**Choosing the right strategy:**

- **Sprint planning for developers**: Use unified scoring (`--top N`)
- **Architectural review**: Use tiered prioritization (`--summary`)
- **Category-focused work**: Use debt type filtering (`--filter`)
- **Testing priorities**: Use ROI analysis with coverage data (`--lcov`)
- **Historical comparisons**: Use legacy risk scoring (for consistency with old reports)

### Tiered Prioritization

**Note:** Tiered prioritization uses **traditional debt scoring** (additive, higher = worse) and is complementary to the unified scoring system (0-10 scale). Both systems can be used together:

- **Unified scoring** (0-10 scale): Best for **function-level prioritization** and sprint planning
- **Tiered prioritization** (debt tiers): Best for **architectural focus** and strategic debt planning

Use `--summary` for tiered view focusing on architectural issues, or default output for function-level unified scores.

Debtmap uses a tier-based system to map debt scores to actionable priority levels. Each tier includes effort estimates and strategic guidance for efficient debt remediation.

#### Tier Levels

The `Tier` enum defines four priority levels based on score thresholds:

```rust
pub enum Tier {
    Critical,  // Score ≥ 90
    High,      // Score 70-89.9
    Moderate,  // Score 50-69.9
    Low,       // Score < 50
}
```

**Score-to-Tier Mapping:**
- **Critical** (≥ 90): Immediate action required - blocks progress
- **High** (70-89.9): Should be addressed this sprint
- **Moderate** (50-69.9): Plan for next sprint
- **Low** (< 50): Background maintenance work

#### Effort Estimates Per Tier

Each tier includes estimated effort based on typical remediation patterns:

| Tier | Estimated Effort | Typical Work |
|------|------------------|--------------|
| **Critical** | 1-2 days | Major refactoring, comprehensive testing, architectural changes |
| **High** | 2-4 hours | Extract functions, add test coverage, fix resource leaks |
| **Moderate** | 1-2 hours | Simplify logic, reduce duplication, improve error handling |
| **Low** | 30 minutes | Address TODOs, minor cleanup, documentation |

**Effort calculation considers:**
- Complexity metrics (cyclomatic, cognitive)
- Test coverage gaps
- Number of dependencies (upstream/downstream)
- Debt category (Architecture debt takes longer than CodeQuality)

#### Tiered Display Grouping

`TieredDisplay` groups similar debt items for batch action recommendations:

```rust
pub struct TieredDisplay {
    pub tier: Tier,
    pub items: Vec<DebtItem>,
    pub total_score: f64,
    pub estimated_total_effort_hours: f64,
    pub batch_recommendations: Vec<String>,
}
```

**Grouping strategy:**
- Groups items by tier and similarity pattern
- Prevents grouping of god objects (always show individually)
- Prevents grouping of Critical items (each needs individual attention)
- Suggests batch actions for similar Low/Moderate items

**Example batch recommendations:**
```json
{
  "tier": "Moderate",
  "total_score": 245.8,
  "estimated_total_effort_hours": 12.5,
  "batch_recommendations": [
    "Extract 5 validation functions from similar patterns",
    "Add test coverage for 8 moderately complex functions (grouped by module)",
    "Refactor 3 functions with similar nested loop patterns"
  ]
}
```

#### Using Tiered Prioritization

**1. Start with Critical tier:**
```bash
debtmap analyze . --min-priority critical
```
Focus on items with score ≥ 90. These typically represent:
- Complex functions with 0% coverage
- God objects blocking feature development
- Critical resource leaks or security issues

**2. Plan High tier work:**
```bash
debtmap analyze . --min-priority high --format json > sprint-plan.json
```
Schedule 2-4 hours per item for this sprint. Look for:
- Functions approaching complexity thresholds
- Moderate coverage gaps on important code paths
- Performance bottlenecks with clear solutions

**3. Batch Moderate tier items:**
```bash
debtmap analyze . --min-priority moderate
```
Review batch recommendations. Examples:
- "10 validation functions detected - extract common pattern"
- "5 similar test files with duplication - create shared fixtures"
- "8 functions with magic values - create constants module"

**4. Schedule Low tier background work:**
Address during slack time or as warm-up tasks for new contributors.

#### Strategic Guidance by Tier

**Critical Tier Strategy:**
- **Block new features** until addressed
- **Pair programming** recommended for complex items
- **Architectural review** before major refactoring
- **Comprehensive testing** after changes

**High Tier Strategy:**
- **Sprint planning priority**
- **Impact analysis** before changes
- **Code review** from senior developers
- **Integration testing** after changes

**Moderate Tier Strategy:**
- **Batch similar items** for efficiency
- **Extract patterns** across multiple files
- **Incremental improvement** over multiple PRs
- **Regression testing** for affected areas

**Low Tier Strategy:**
- **Good first issues** for new contributors
- **Documentation improvements**
- **Code cleanup** during refactoring nearby code
- **Technical debt gardening** sessions

### Categorized Debt Analysis

Debtmap provides `CategorizedDebt` analysis that groups debt items by category and identifies cross-category dependencies. This helps teams understand strategic relationships between different types of technical debt.

#### CategorySummary

Each category gets a summary with metrics for planning:

```rust
pub struct CategorySummary {
    pub category: DebtCategory,
    pub total_score: f64,
    pub item_count: usize,
    pub estimated_effort_hours: f64,
    pub average_severity: f64,
    pub top_items: Vec<DebtItem>,  // Up to 5 highest priority
}
```

**Effort estimation formulas:**
- **Architecture debt**: `complexity_score / 10 × 2` hours (structural changes take longer)
- **Testing debt**: `complexity_score / 10 × 1.5` hours (writing tests)
- **Performance debt**: `complexity_score / 10 × 1.8` hours (profiling + optimization)
- **CodeQuality debt**: `complexity_score / 10 × 1.2` hours (refactoring)

**Example category summary:**
```json
{
  "category": "Architecture",
  "total_score": 487.5,
  "item_count": 15,
  "estimated_effort_hours": 97.5,
  "average_severity": 32.5,
  "top_items": [
    {
      "debt_type": "GodObject",
      "file": "src/services/user_service.rs",
      "score": 95.0,
      "estimated_effort_hours": 16.0
    },
    {
      "debt_type": "ComplexityHotspot",
      "file": "src/payments/processor.rs",
      "score": 87.3,
      "estimated_effort_hours": 14.0
    }
  ]
}
```

#### Cross-Category Dependencies

`CrossCategoryDependency` identifies blocking relationships between different debt categories:

```rust
pub struct CrossCategoryDependency {
    pub from_category: DebtCategory,
    pub to_category: DebtCategory,
    pub blocking_items: Vec<(DebtItem, DebtItem)>,
    pub impact_level: ImpactLevel,  // Critical, High, Medium, Low
    pub recommendation: String,
}
```

**Common dependency patterns:**

**1. Architecture blocks Testing:**
- **Pattern**: God objects are too complex to test effectively
- **Example**: `UserService` has 50+ functions, making comprehensive testing impractical
- **Impact**: Critical - cannot improve test coverage without refactoring
- **Recommendation**: "Split god object into 4-5 focused modules before adding tests"

**2. Async issues require Architecture changes:**
- **Pattern**: Blocking I/O in async contexts requires architectural redesign
- **Example**: Sync database calls in async handlers
- **Impact**: High - performance problems require design changes
- **Recommendation**: "Introduce async database layer before optimizing handlers"

**3. Complexity affects Testability:**
- **Pattern**: High cyclomatic complexity makes thorough testing difficult
- **Example**: Function with 22 branches needs 22+ test cases
- **Impact**: High - testing effort grows exponentially with complexity
- **Recommendation**: "Reduce complexity to < 10 before writing comprehensive tests"

**4. Performance requires Architecture:**
- **Pattern**: O(n²) nested loops need different data structures
- **Example**: Linear search in loops should use HashMap
- **Impact**: Medium - optimization requires structural changes
- **Recommendation**: "Refactor data structure before micro-optimizations"

**Example cross-category dependency:**
```json
{
  "from_category": "Architecture",
  "to_category": "Testing",
  "impact_level": "Critical",
  "blocking_items": [
    {
      "blocker": {
        "debt_type": "GodObject",
        "file": "src/services/user_service.rs",
        "functions": 52,
        "score": 95.0
      },
      "blocked": {
        "debt_type": "TestingGap",
        "file": "src/services/user_service.rs",
        "coverage": 15,
        "score": 78.0
      }
    }
  ],
  "recommendation": "Split UserService into focused modules (auth, profile, settings, notifications) before attempting to improve test coverage. Current structure makes comprehensive testing impractical.",
  "estimated_unblock_effort_hours": 16.0
}
```

#### Using Categorized Debt Analysis

**View all category summaries:**
```bash
debtmap analyze . --format json | jq '.categorized_debt.summaries'
```

**Focus on specific category:**
```bash
debtmap analyze . --filter Architecture --top 10
```

**Identify blocking relationships:**
```bash
debtmap analyze . --format json | jq '.categorized_debt.cross_category_dependencies[] | select(.impact_level == "Critical")'
```

**Strategic planning workflow:**

1. **Review category summaries:**
   - Identify which category has highest total score
   - Check estimated effort hours per category
   - Note average severity to gauge urgency

2. **Check cross-category dependencies:**
   - Find Critical and High impact blockers
   - Prioritize blockers before blocked items
   - Plan architectural changes before optimization

3. **Plan remediation order:**
   ```
   Example decision tree:
   - Architecture score > 400? → Address god objects first
   - Testing gap with low complexity? → Quick wins, add tests
   - Performance issues + architecture debt? → Refactor structure first
   - High code quality debt but good architecture? → Incremental cleanup
   ```

4. **Use category-specific strategies:**
   - **Architecture**: Pair programming, design reviews, incremental refactoring
   - **Testing**: TDD for new code, characterization tests for legacy
   - **Performance**: Profiling first, optimize hot paths, avoid premature optimization
   - **CodeQuality**: Code review focus, linting rules, consistent patterns

#### CategorizedDebt Output Structure

```json
{
  "categorized_debt": {
    "summaries": [
      {
        "category": "Architecture",
        "total_score": 487.5,
        "item_count": 15,
        "estimated_effort_hours": 97.5,
        "average_severity": 32.5,
        "top_items": [...]
      },
      {
        "category": "Testing",
        "total_score": 356.2,
        "item_count": 23,
        "estimated_effort_hours": 53.4,
        "average_severity": 15.5,
        "top_items": [...]
      },
      {
        "category": "Performance",
        "total_score": 234.8,
        "item_count": 12,
        "estimated_effort_hours": 42.3,
        "average_severity": 19.6,
        "top_items": [...]
      },
      {
        "category": "CodeQuality",
        "total_score": 189.3,
        "item_count": 31,
        "estimated_effort_hours": 22.7,
        "average_severity": 6.1,
        "top_items": [...]
      }
    ],
    "cross_category_dependencies": [
      {
        "from_category": "Architecture",
        "to_category": "Testing",
        "impact_level": "Critical",
        "blocking_items": [...],
        "recommendation": "..."
      }
    ]
  }
}
```

### Debt Density Metric

Debt density normalizes technical debt scores across projects of different sizes, providing a per-1000-lines-of-code metric for fair comparison.

#### Formula

```
debt_density = (total_debt_score / total_lines_of_code) × 1000
```

**Example calculation:**
```
Project A:
  - Total debt score: 1,250
  - Total lines of code: 25,000
  - Debt density: (1,250 / 25,000) × 1000 = 50

Project B:
  - Total debt score: 2,500
  - Total lines of code: 50,000
  - Debt density: (2,500 / 50,000) × 1000 = 50
```

Projects A and B have **equal debt density** (50) despite B having twice the absolute debt, because B is also twice as large. They have proportionally similar technical debt.

#### Interpretation Guidelines

Use these thresholds to assess codebase health:

| Debt Density | Assessment | Description |
|-------------|-----------|-------------|
| **0-50** | Clean | Well-maintained codebase, minimal debt |
| **51-100** | Moderate | Typical technical debt, manageable |
| **101-150** | High | Significant debt, prioritize remediation |
| **150+** | Critical | Severe debt burden, may impede development |

**Context matters:**
- **Early-stage projects**: Often have higher density (rapid iteration)
- **Mature projects**: Should trend toward lower density over time
- **Legacy systems**: May have high density, track trend over time
- **Greenfield rewrites**: Aim for density < 50

#### Using Debt Density

**1. Compare projects fairly:**
```bash
# Small microservice (5,000 LOC, debt = 250)
# Debt density: 50

# Large monolith (100,000 LOC, debt = 5,000)
# Debt density: 50

# Equal health despite size difference
```

**2. Track improvement over time:**
```
Sprint 1: 50,000 LOC, debt = 7,500, density = 150 (High)
Sprint 5: 52,000 LOC, debt = 6,500, density = 125 (Improving)
Sprint 10: 54,000 LOC, debt = 4,860, density = 90 (Moderate)
```

**3. Set team goals:**
```
Current density: 120
Target density: < 80 (by Q4)
Reduction needed: 40 points

Strategy:
- Fix 2-3 Critical items per sprint
- Prevent new debt (enforce thresholds)
- Refactor before adding features in high-debt modules
```

**4. Benchmark across teams/projects:**
```json
{
  "team_metrics": [
    {
      "project": "auth-service",
      "debt_density": 45,
      "assessment": "Clean",
      "trend": "stable"
    },
    {
      "project": "billing-service",
      "debt_density": 95,
      "assessment": "Moderate",
      "trend": "improving"
    },
    {
      "project": "legacy-api",
      "debt_density": 165,
      "assessment": "Critical",
      "trend": "worsening"
    }
  ]
}
```

#### Limitations

**Debt density doesn't account for:**
- **Code importance**: 100 LOC in payment logic ≠ 100 LOC in logging utils
- **Complexity distribution**: One 1000-line god object vs. 1000 simple functions
- **Test coverage**: 50% coverage on critical paths vs. low-priority features
- **Team familiarity**: New codebase vs. well-understood legacy system

**Best practices:**
- Use density as **one metric among many**
- Combine with category analysis and tiered prioritization
- Focus on **trend** (improving/stable/worsening) over absolute number
- Consider **debt per module** for more granular insights

#### Debt Density in CI/CD

**Track density over time:**
```bash
# Generate report with density
debtmap analyze . --format json --output debt-report.json

# Extract density for trending
DENSITY=$(jq '.debt_density' debt-report.json)

# Store in metrics database
echo "debtmap.density:${DENSITY}|g" | nc -u -w0 statsd 8125
```

**Set threshold gates:**
```yaml
# .github/workflows/debt-check.yml
- name: Check debt density
  run: |
    DENSITY=$(debtmap analyze . --format json | jq '.debt_density')
    if (( $(echo "$DENSITY > 150" | bc -l) )); then
      echo "❌ Debt density too high: $DENSITY (limit: 150)"
      exit 1
    fi
    echo "✅ Debt density acceptable: $DENSITY"
```

### Actionable Insights

Each recommendation includes:

**ACTION**: What to do
- "Add 6 unit tests for full coverage"
- "Refactor into 3 smaller functions"
- "Extract validation to separate function"

**IMPACT**: Expected improvement
- "Full test coverage, -3.7 risk"
- "Reduce complexity from 22 to 8"
- "Eliminate 120 lines of duplication"

**WHY**: Rationale
- "Business logic with 0% coverage, manageable complexity"
- "High complexity with low coverage threatens stability"
- "Repeated validation pattern across 5 files"

**Example workflow:**
1. Run analysis with coverage: `debtmap analyze . --lcov coverage.lcov`
2. Filter to CRITICAL items: `--min-priority critical`
3. Review top 5 recommendations
4. Start with highest ROI items
5. Rerun analysis to track progress

### Common Patterns to Recognize

**Pattern 1: High Complexity, Well Tested**
```
Complexity: 25, Coverage: 95%, Risk: LOW
```
This is actually good! Complex but thoroughly tested code. Learn from this approach.

**Pattern 2: Moderate Complexity, No Tests**
```
Complexity: 12, Coverage: 0%, Risk: CRITICAL
```
Highest priority - manageable complexity, should be easy to test.

**Pattern 3: Low Complexity, No Tests**
```
Complexity: 3, Coverage: 0%, Risk: LOW
```
Low priority - simple code, less risky without tests.

**Pattern 4: Repetitive High Complexity (Dampened)**
```
Cyclomatic: 20, Effective: 7 (65% dampened), Risk: LOW
```
Validation or dispatch pattern - looks complex but is repetitive. Lower priority.

**Pattern 5: God Object**
```
File: services.rs, Functions: 50+, Responsibilities: 15+
```
Architectural issue - split before adding features.

