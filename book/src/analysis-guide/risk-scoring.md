# Risk Scoring

## Risk Scoring

Debtmap's risk scoring identifies code that is both complex AND poorly tested - the true risk hotspots.

### Unified Scoring System

Debtmap uses a **unified scoring system** (0-10 scale) as the primary prioritization mechanism. This multi-factor approach balances complexity, test coverage, and dependency impact, adjusted by function role.

#### Score Scale and Priority Classifications

Functions receive scores from 0 (minimal risk) to 10 (critical risk):

| Score Range | Priority | Description | Action |
|-------------|----------|-------------|--------|
| **9.0-10.0** | Critical | Severe risk requiring immediate attention | Address immediately |
| **7.0-8.9** | High | Significant risk, should be addressed soon | Plan for this sprint |
| **5.0-6.9** | Medium | Moderate risk, plan for future work | Schedule for next sprint |
| **3.0-4.9** | Low | Minor risk, lower priority | Monitor and address as time permits |
| **0.0-2.9** | Minimal | Well-managed code | Continue monitoring |

#### Scoring Formula

The unified score combines three weighted factors:

```
Base Score = (Complexity Factor × 0.40) + (Coverage Factor × 0.40) + (Dependency Factor × 0.20)

Final Score = Base Score × Role Multiplier
```

**Factor Calculations:**

**Complexity Factor** (0-10 scale):
```
Complexity Factor = min(10, ((cyclomatic / 10) + (cognitive / 20)) × 5)
```
Normalized to 0-10 range based on cyclomatic and cognitive complexity.

**Coverage Factor** (0-10 scale):
```
Coverage Factor = 10 × (1 - coverage_percentage) × complexity_weight
```
Uncovered complex code scores higher than uncovered simple code. Coverage dampens the score - well-tested code gets lower scores.

**Dependency Factor** (0-10 scale):
Based on call graph analysis with specific thresholds:
- **High impact** (score 8-10): 5+ upstream callers, or on critical path from entry point (adds 2-3 points)
- **Moderate impact** (score 4-6): 2-4 upstream callers
- **Low impact** (score 1-3): 0-1 upstream callers
- **Critical path bonus**: Being on a critical path from an entry point adds 2-3 points to the base dependency score

#### Default Weights

The scoring formula uses configurable weights (default values shown):

- **Complexity: 40%** - How difficult the code is to understand and test
- **Coverage: 40%** - How well the code is tested
- **Dependency: 20%** - How many other functions depend on this code

These weights can be adjusted in `.debtmap.toml` to match your team's priorities.

#### Role-Based Prioritization

The unified score is multiplied by a **role multiplier** based on the function's semantic classification:

| Role | Multiplier | Description | Example |
|------|-----------|-------------|---------|
| **Entry Points** | 1.5× | main(), HTTP handlers, API endpoints | User-facing code where bugs have immediate impact |
| **Business Logic** | 1.2× | Core domain functions, algorithms | Critical functionality |
| **Data Access** | 1.0× | Database queries, file I/O | Baseline importance |
| **Infrastructure** | 0.8× | Logging, configuration, monitoring | Supporting code |
| **Utilities** | 0.5× | Helpers, formatters, converters | Lower impact |
| **Test Code** | 0.1× | Test functions, fixtures, mocks | Internal quality |

**How role classification works:**

Debtmap identifies function roles through pattern analysis:
- **Entry points**: Functions named `main`, handlers with routing decorators, public API functions
- **Business logic**: Core domain operations, calculation functions, decision-making code
- **Data access**: Database queries, file operations, network calls
- **Infrastructure**: Logging, config parsing, monitoring, error handling
- **Utilities**: Helper functions, formatters, type converters, validators
- **Test code**: Functions in test modules, test functions, fixtures

**Example: Same complexity, different priorities**

Consider a function with base score 8.0:

```
If classified as Entry Point:
  Final Score = 8.0 × 1.5 = 12.0 (capped at 10.0) → CRITICAL priority

If classified as Business Logic:
  Final Score = 8.0 × 1.2 = 9.6 → CRITICAL priority

If classified as Data Access:
  Final Score = 8.0 × 1.0 = 8.0 → HIGH priority

If classified as Utility:
  Final Score = 8.0 × 0.5 = 4.0 → LOW priority
```

This ensures that complex code in critical paths gets higher priority than equally complex utility code.

#### Coverage Propagation

Coverage impact flows through the call graph using **transitive coverage**:

```
Transitive Coverage = Direct Coverage + Σ(Caller Coverage × Weight)
```

**How it works:**

Functions called by well-tested code inherit some coverage benefit, reducing their urgency. This helps identify which untested functions are on critical paths versus safely isolated utilities.

**Example scenarios:**

**Scenario 1: Untested function with well-tested callers**
```
Function A: 0% direct coverage
  Called by:
    - handle_request (95% coverage)
    - process_payment (90% coverage)
    - validate_order (88% coverage)

Transitive coverage: ~40% (inherits coverage benefit from callers)
Final priority: Lower than isolated 0% coverage function
```

**Scenario 2: Untested function on critical path**
```
Function B: 0% direct coverage
  Called by:
    - main (0% coverage)
    - startup (10% coverage)

Transitive coverage: ~5% (minimal coverage benefit)
Final priority: Higher - on critical path with no safety net
```

Coverage propagation prevents false alarms about utility functions called only by well-tested code, while highlighting genuinely risky untested code on critical paths.

#### Unified Score Example

```
Function: process_payment
  Location: src/payments.rs:145

Metrics:
  - Cyclomatic complexity: 18
  - Cognitive complexity: 25
  - Test coverage: 20%
  - Upstream callers: 3 (high dependency)
  - Role: Business Logic

Calculation:
  Complexity Factor = min(10, ((18/10) + (25/20)) × 5) = min(10, 8.75) = 8.75
  Coverage Factor = 10 × (1 - 0.20) × 1.0 = 8.0
  Dependency Factor = 7.5 (3 upstream callers, moderate impact)

  Base Score = (8.75 × 0.40) + (8.0 × 0.40) + (7.5 × 0.20)
             = 3.5 + 3.2 + 1.5
             = 8.2

  Final Score = 8.2 × 1.2 (Business Logic multiplier)
              = 9.84 → CRITICAL priority
```

### Legacy Risk Scoring (Pre-0.2.x)

Prior to the unified scoring system, Debtmap used a simpler additive risk formula. This is still available for compatibility but unified scoring is now the default and provides better prioritization.

### Risk Categories

**Note:** The `RiskLevel` enum (Low, Medium, High, Critical) is used for **legacy risk scoring compatibility**. When using **unified scoring** (0-10 scale), refer to the priority classifications shown in the Unified Scoring System section above.

#### Legacy RiskLevel Enum

For legacy risk scoring, Debtmap classifies functions into four risk levels:

```rust
pub enum RiskLevel {
    Low,       // Score < 10
    Medium,    // Score 10-24
    High,      // Score 25-49
    Critical,  // Score ≥ 50
}
```

**Critical** (legacy score ≥ 50)
- High complexity (cyclomatic > 15) AND low coverage (< 30%)
- Untested code that's likely to break and hard to fix
- **Action**: Immediate attention required - add tests or refactor

**High** (legacy score 25-49)
- High complexity (cyclomatic > 10) AND moderate coverage (< 60%)
- Risky code with incomplete testing
- **Action**: Should be addressed soon

**Medium** (legacy score 10-24)
- Moderate complexity (cyclomatic > 5) AND low coverage (< 50%)
- OR: High complexity with good coverage
- **Action**: Plan for next sprint

**Low** (legacy score < 10)
- Low complexity OR high coverage
- Well-managed code
- **Action**: Monitor, low priority

#### Unified Scoring Priority Levels

When using unified scoring (default), functions are classified using the 0-10 scale:

- **Critical** (9.0-10.0): Immediate attention
- **High** (7.0-8.9): Address this sprint
- **Medium** (5.0-6.9): Plan for next sprint
- **Low** (3.0-4.9): Monitor and address as time permits
- **Minimal** (0.0-2.9): Well-managed code

**Well-tested complex code** is an **outcome** in both systems, not a separate category:
- Complex function (cyclomatic 18, cognitive 25) with 95% coverage
- Unified score: ~2.5 (Minimal priority due to coverage dampening)
- Legacy risk score: ~8 (Low risk)
- Falls into low-priority categories because good testing mitigates complexity
- This is the desired state for inherently complex business logic

### Legacy Risk Calculation

**Note:** The legacy risk calculation is still supported for compatibility but has been superseded by the unified scoring system (see above). Unified scoring provides better prioritization through its multi-factor, weighted approach with role-based adjustments.

The legacy risk score uses a simpler additive formula:

```rust
risk_score = complexity_factor + coverage_factor + debt_factor

where:
  complexity_factor = (cyclomatic / 5) + (cognitive / 10)
  coverage_factor = (1 - coverage_percentage) × 50
  debt_factor = debt_score / 10  // If debt data available
```

**Example (legacy scoring):**
```
Function: process_payment
  - Cyclomatic complexity: 18
  - Cognitive complexity: 25
  - Coverage: 20%
  - Debt score: 15

Calculation:
  complexity_factor = (18 / 5) + (25 / 10) = 3.6 + 2.5 = 6.1
  coverage_factor = (1 - 0.20) × 50 = 40
  debt_factor = 15 / 10 = 1.5

  risk_score = 6.1 + 40 + 1.5 = 47.6 (HIGH RISK)
```

**When to use legacy scoring:**
- Comparing with historical data from older Debtmap versions
- Teams with existing workflows built around the old scale
- Gradual migration to unified scoring

**Why unified scoring is better:**
- Normalized 0-10 scale is more intuitive
- Weighted factors (40% complexity, 40% coverage, 20% dependency) provide better balance
- Role multipliers adjust priority based on function importance
- Coverage propagation reduces false positives for utility functions

### Test Effort Assessment

Debtmap estimates testing difficulty based on cognitive complexity:

**Difficulty Levels:**
- **Trivial** (cognitive < 5): 1-2 test cases, < 1 hour
- **Simple** (cognitive 5-10): 3-5 test cases, 1-2 hours
- **Moderate** (cognitive 10-20): 6-10 test cases, 2-4 hours
- **Complex** (cognitive 20-40): 11-20 test cases, 4-8 hours
- **VeryComplex** (cognitive > 40): 20+ test cases, 8+ hours

**Test Effort includes:**
- **Cognitive load**: How hard to understand the function
- **Branch count**: Number of paths to test
- **Recommended test cases**: Suggested number of tests

### Risk Distribution

Debtmap provides codebase-wide risk metrics:

```json
{
  "risk_distribution": {
    "critical_count": 12,
    "high_count": 45,
    "medium_count": 123,
    "low_count": 456,
    "minimal_count": 234,
    "total_functions": 870
  },
  "codebase_risk_score": 1247.5
}
```

**Interpreting distribution:**
- **Healthy codebase**: Most functions in Low/Minimal priority (unified scoring) or Low/WellTested (legacy)
- **Needs attention**: Many Critical/High priority functions
- **Technical debt**: High codebase risk score

**Note on minimal_count:**

In unified scoring (0-10 scale), `minimal_count` represents functions scoring 0-2.9, which includes:
- Simple utility functions
- Helper functions with low complexity
- Well-tested complex code that scores low due to coverage dampening

This is not a separate risk category but an **outcome** of the unified scoring system. Complex business logic with 95% test coverage appropriately receives a minimal score, reflecting that good testing mitigates complexity risk.

**Important:** `minimal_count` does not appear in the standard `risk_categories` from features.json (Low, Medium, High, Critical, WellTested). It's specific to unified scoring's 0-10 scale priority classifications (Minimal, Low, Medium, High, Critical).

### Testing Recommendations

When coverage data is provided, Debtmap generates prioritized testing recommendations with ROI analysis:

```json
{
  "function": "process_transaction",
  "file": "src/payments.rs",
  "line": 145,
  "current_risk": 47.6,
  "potential_risk_reduction": 35.2,
  "test_effort_estimate": {
    "estimated_difficulty": "Complex",
    "cognitive_load": 25,
    "branch_count": 18,
    "recommended_test_cases": 12
  },
  "roi": 4.4,
  "rationale": "High complexity with low coverage (20%) and 3 downstream dependencies. Testing will reduce risk by 74%.",
  "dependencies": {
    "upstream_callers": ["handle_payment_request"],
    "downstream_callees": ["validate_amount", "check_balance", "record_transaction"]
  }
}
```

**ROI calculation:**
```
roi = potential_risk_reduction / estimated_effort_hours
```

Higher ROI = better return on testing investment

