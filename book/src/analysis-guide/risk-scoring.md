# Risk Scoring

Debtmap's risk scoring identifies code that is both complex AND poorly tested - the true risk hotspots.

## Unified Scoring System

Debtmap uses a **unified scoring system** (0-10 scale) as the primary prioritization mechanism. This multi-factor approach balances complexity, test coverage, and dependency impact, adjusted by function role.

**Source**: [src/priority/unified_scorer.rs:22-291](../../src/priority/unified_scorer.rs)

### Score Scale and Priority Classifications

Functions receive scores from 0 (minimal risk) to 10 (critical risk):

| Score Range | Priority | Description | Action |
|-------------|----------|-------------|--------|
| **9.0-10.0** | Critical | Severe risk requiring immediate attention | Address immediately |
| **7.0-8.9** | High | Significant risk, should be addressed soon | Plan for this sprint |
| **5.0-6.9** | Medium | Moderate risk, plan for future work | Schedule for next sprint |
| **3.0-4.9** | Low | Minor risk, lower priority | Monitor and address as time permits |
| **0.0-2.9** | Minimal | Well-managed code | Continue monitoring |

### Scoring Formula

The unified score combines three weighted factors:

```
Base Score = (Complexity Factor × Weight) + (Coverage Factor × Weight) + (Dependency Factor × Weight)

Final Score = Base Score × Role Multiplier × Purity Adjustment
```

**Source**: [src/priority/unified_scorer.rs:170-291](../../src/priority/unified_scorer.rs) (calculate_unified_priority_with_debt)

#### Dynamic Weight Adjustment

**IMPORTANT**: Weights are dynamically adjusted based on coverage data availability.

**When coverage data is available** (default):
- **Complexity**: ~35-40% (via complexity_factor)
- **Coverage**: ~35-40% (via coverage multiplier dampening)
- **Dependency**: ~20-25%

**When coverage data is NOT available**:
- **Complexity**: 50%
- **Dependency**: 25%
- **Debt patterns**: 25% (reserved for additive adjustments)

**Source**:
- With coverage: [src/priority/scoring/calculation.rs:68-82](../../src/priority/scoring/calculation.rs) (calculate_base_score_with_coverage_multiplier)
- Without coverage: [src/priority/scoring/calculation.rs:119-129](../../src/priority/scoring/calculation.rs) (calculate_base_score_no_coverage)

These weights can be adjusted in `.debtmap.toml` to match your team's priorities.

#### Factor Calculations

**Complexity Factor** (0-10 scale):
```rust
// Source: src/priority/scoring/calculation.rs:54-59
Complexity Factor = (raw_complexity / 2.0).clamp(0.0, 10.0)

// Where raw_complexity is weighted combination:
// Default: 30% cyclomatic + 70% cognitive
// For orchestrators: 25% cyclomatic + 75% cognitive
```

Maps normalized complexity (0-20 range) to 0-10 scale. Uses configurable weights that prioritize cognitive complexity (70%) over cyclomatic complexity (30%) as it correlates better with defect density.

**Source**: [src/config/scoring.rs:221-267](../../src/config/scoring.rs) (ComplexityWeightsConfig)

**Coverage Factor** (0-10 scale):
```rust
// Source: src/priority/scoring/calculation.rs:8-21
Coverage Multiplier = 1.0 - coverage_percentage

// Applied as dampening:
Base Score × Coverage Multiplier
```

Coverage acts as a **dampening multiplier**:
- 0% coverage → multiplier = 1.0 (no dampening)
- 50% coverage → multiplier = 0.5 (50% reduction)
- 100% coverage → multiplier = 0.0 (maximum dampening)

Uncovered complex code scores higher than uncovered simple code. Well-tested code gets lower scores.

**Dependency Factor** (0-10 scale):
```rust
// Source: src/priority/scoring/calculation.rs:61-66
Dependency Factor = (upstream_caller_count / 2.0).min(10.0)
```

Based on call graph analysis with linear scaling:
- 0-1 upstream callers → score 0-0.5 (low impact)
- 2-4 upstream callers → score 1.0-2.0 (moderate impact)
- 5+ upstream callers → score 2.5-10.0 (high impact, capped at 10.0)

**Critical path bonus**: Functions on critical paths from entry points receive additional dependency weight.

### Role-Based Prioritization

The unified score is multiplied by a **role multiplier** based on the function's semantic classification.

**Source**: [src/priority/semantic_classifier/mod.rs:24-33](../../src/priority/semantic_classifier/mod.rs) (FunctionRole enum)

#### Role Multipliers

| Role | Multiplier | Description | When Applied |
|------|-----------|-------------|--------------|
| **EntryPoint** | 1.5× | main(), HTTP handlers, API endpoints | User-facing code where bugs have immediate impact |
| **PureLogic** (complex) | 1.3× | Business logic with complexity > 5.0 | Critical domain functions |
| **PureLogic** (simple) | 1.0× | Business logic with complexity ≤ 5.0 | Baseline importance for domain code |
| **Orchestrator** | 0.8× | Coordinates 5+ other functions | Delegation-heavy code with low cognitive load |
| **PatternMatch** | 0.6× | Simple pattern matching functions | Low complexity branching logic |
| **IOWrapper** | 0.5× | Thin I/O layer (file, network, database) | Simple wrappers around external systems |
| **Debug** | 0.3× | Debug/diagnostic functions | Lowest test priority |

**Source**:
- Multiplier values: [src/priority/unified_scorer.rs:385-399](../../src/priority/unified_scorer.rs) (calculate_role_multiplier)
- Configuration defaults: [src/config/scoring.rs:147-220](../../src/config/scoring.rs) (RoleMultipliers)

**Note**: PureLogic has a **dynamic multiplier** that adjusts based on complexity. Simple business logic (≤ 5.0 complexity) gets baseline priority, while complex business logic (> 5.0) receives elevated priority (1.3×).

#### How Role Classification Works

Debtmap identifies function roles through a rule-based classifier with specific detection heuristics:

**Source**: [src/priority/semantic_classifier/mod.rs:46-114](../../src/priority/semantic_classifier/mod.rs) (classify_by_rules)

**Detection Rules (in priority order):**

1. **EntryPoint** - Detected by:
   - Name patterns: `main`, `handle_*`, `run_*`
   - Call graph analysis: no upstream callers (entry point to call graph)
   - **Source**: Line 54

2. **Debug** - Detected by:
   - Name patterns: `debug_*`, `dump_*`, `log_*`, `print_*`, `display_*`, `trace_*`, `*_diagnostics`, `*_debug`, `*_stats`
   - Complexity limit: cognitive ≤ 10
   - **Source**: Line 59, [src/priority/semantic_classifier/classifiers.rs:14-65](../../src/priority/semantic_classifier/classifiers.rs)

3. **Constructors** (classified as PureLogic) - Detected by:
   - Name patterns: `new`, `with_*`, `from_*`, `default`, `create_*`, `make_*`, `build_*`
   - Complexity thresholds: cyclomatic ≤ 2, cognitive ≤ 3, length < 15, nesting ≤ 1
   - **Source**: Line 64, [src/priority/semantic_classifier/classifiers.rs:67-115](../../src/priority/semantic_classifier/classifiers.rs)

4. **Accessors** (classified as IOWrapper) - Detected by:
   - Name patterns: `get_*`, `is_*`, `has_*`, `can_*`, `should_*`, `as_*`, `to_*`, single-word accessors (`id`, `name`, `value`, etc.)
   - Complexity thresholds: cyclomatic ≤ 2, cognitive ≤ 1, length < 10, nesting ≤ 1
   - **Source**: Line 77, [src/priority/semantic_classifier/mod.rs:147-177](../../src/priority/semantic_classifier/mod.rs) (is_accessor_method)

5. **PatternMatch** - Detected by:
   - Simple match/if-else chains
   - Low complexity relative to branch count
   - **Source**: Line 99

6. **IOWrapper** - Detected by:
   - Simple file/network/database operations
   - Thin wrapper around I/O primitives
   - **Source**: Line 104

7. **Orchestrator** - Detected by:
   - High delegation ratio (calls 5+ functions)
   - Low cognitive complexity relative to cyclomatic complexity
   - Coordinates other functions without complex logic
   - **Source**: Line 109

8. **PureLogic** (default) - Applied when:
   - None of the above patterns match
   - Assumed to be core business logic

#### Example: Same Complexity, Different Priorities

Consider a function with base score 8.0:

```
If classified as EntryPoint:
  Final Score = 8.0 × 1.5 = 12.0 (capped at 10.0) → CRITICAL priority

If classified as PureLogic (complex):
  Final Score = 8.0 × 1.3 = 10.4 (capped at 10.0) → CRITICAL priority

If classified as PureLogic (simple):
  Final Score = 8.0 × 1.0 = 8.0 → HIGH priority

If classified as Orchestrator:
  Final Score = 8.0 × 0.8 = 6.4 → MEDIUM priority

If classified as IOWrapper:
  Final Score = 8.0 × 0.5 = 4.0 → LOW priority
```

This ensures that complex code in critical paths gets higher priority than equally complex utility code.

**Real Example from Codebase**:

A payment processing function with cyclomatic complexity 18 and cognitive complexity 25:
- If it directly implements business logic → **PureLogic (complex)** → 1.3× multiplier
- If it mainly delegates to other payment functions → **Orchestrator** → 0.8× multiplier
- If it's a thin wrapper around a payment API → **IOWrapper** → 0.5× multiplier

### Coverage Propagation

Coverage impact flows through the call graph using **transitive coverage** and **indirect coverage** analysis.

**Source**: [src/priority/coverage_propagation.rs:291-387](../../src/priority/coverage_propagation.rs)

#### How It Works

Transitive coverage is calculated via call graph traversal with distance-based dampening:

```rust
// Source: src/priority/coverage_propagation.rs:342-364
Indirect Coverage = Σ(Caller Coverage × 0.7^distance)

Where:
- distance = hops from tested code (MAX_DEPTH = 3)
- DISTANCE_DISCOUNT = 0.7 (70% per hop)
- Well-tested threshold = 0.8 (80% coverage)
```

**Implementation Details**:

1. **Transitive coverage** is calculated via recursive call graph traversal
2. Results are stored in `UnifiedDebtItem.transitive_coverage` field (**Source**: [src/priority/unified_scorer.rs:50](../../src/priority/unified_scorer.rs))
3. Weights decay exponentially with call graph depth:
   - 1 hop away: contribution × 0.7
   - 2 hops away: contribution × 0.49 (0.7²)
   - 3 hops away: contribution × 0.343 (0.7³)
4. Used to adjust coverage factor in scoring, reducing false positives for utility functions

#### Coverage Urgency Calculation

The system calculates **coverage urgency** (0-10 scale) by blending direct and transitive coverage:

```rust
// Source: src/priority/coverage_propagation.rs:237-270
Effective Coverage = (Direct Coverage × 0.7) + (Transitive Coverage × 0.3)

Coverage Urgency = (1.0 - Effective Coverage) × Complexity Weight × 10.0
```

Complexity weighting uses logarithmic scaling to prioritize complex functions.

#### Example Scenarios

**Scenario 1: Untested function with well-tested callers**
```
Function A: 0% direct coverage
  Called by (1 hop):
    - handle_request (95% coverage): contributes 95% × 0.7 = 66.5%
    - process_payment (90% coverage): contributes 90% × 0.7 = 63%
    - validate_order (88% coverage): contributes 88% × 0.7 = 61.6%

Indirect coverage: ~66% (highest contributor)
Effective coverage: (0% × 0.7) + (66% × 0.3) = ~20%
Final priority: Lower than isolated 0% coverage function
```

**Scenario 2: Untested function on critical path**
```
Function B: 0% direct coverage
  Called by (1 hop):
    - main (0% coverage): contributes 0% × 0.7 = 0%
    - startup (10% coverage): contributes 10% × 0.7 = 7%

Indirect coverage: ~7% (minimal coverage benefit)
Effective coverage: (0% × 0.7) + (7% × 0.3) = ~2%
Final priority: Higher - on critical path with no safety net
```

**Scenario 3: Multi-hop propagation**
```
Function C: 0% direct coverage
  Called by utility_helper (40% coverage, 1 hop):
    utility_helper is called by:
      - api_handler (95% coverage, 2 hops): contributes 95% × 0.7² = 46.6%

Indirect coverage via 2-hop path: ~46%
Effective coverage: ~14%
Final priority: Moderate - benefits from indirect testing
```

Coverage propagation prevents false alarms about utility functions called only by well-tested code, while highlighting genuinely risky untested code on critical paths.

### Unified Score Example

Updated example using actual implementation:

```
Function: process_payment
  Location: src/payments.rs:145

Metrics:
  - Cyclomatic complexity: 18
  - Cognitive complexity: 25
  - Test coverage: 20%
  - Upstream callers: 3
  - Classified role: PureLogic (complex, since complexity > 5.0)

Step 1: Calculate raw complexity
  Raw Complexity = (cyclomatic × 0.3) + (cognitive × 0.7)
                 = (18 × 0.3) + (25 × 0.7)
                 = 5.4 + 17.5
                 = 22.9

Step 2: Normalize to 0-10 scale
  Complexity Factor = (22.9 / 2.0).clamp(0.0, 10.0)
                    = 10.0 (capped)
  // Source: src/priority/scoring/calculation.rs:54-59

Step 3: Calculate coverage multiplier
  Coverage Multiplier = 1.0 - 0.20 = 0.80
  // Source: src/priority/scoring/calculation.rs:8-21

Step 4: Calculate dependency factor
  Dependency Factor = (3 / 2.0).min(10.0) = 1.5
  // Source: src/priority/scoring/calculation.rs:61-66

Step 5: Calculate base score (with dynamic weights)
  Base Score = (Complexity Factor × weight) + (Coverage dampening) + (Dependency Factor × weight)

  // Actual implementation uses coverage as dampening multiplier
  Base = ((10.0 × 0.35) + (1.5 × 0.20)) × 0.80
       = (3.5 + 0.3) × 0.80
       = 3.04
  // Source: src/priority/scoring/calculation.rs:68-82

Step 6: Apply role multiplier
  Role Multiplier = 1.3 (PureLogic with complexity > 5.0)
  // Source: src/priority/unified_scorer.rs:385-399

  Final Score = 3.04 × 1.3 = 3.95 → LOW priority

Note: The 20% coverage dampening significantly reduces the final score.
If this function had 0% coverage:
  Coverage Multiplier = 1.0 (no dampening)
  Base Score = 3.8
  Final Score = 3.8 × 1.3 = 4.94 → LOW priority

If this function had 0% coverage AND higher dependency (8 callers):
  Dependency Factor = (8 / 2.0).min(10.0) = 4.0
  Base Score = ((10.0 × 0.35) + (4.0 × 0.20)) × 1.0 = 4.3
  Final Score = 4.3 × 1.3 = 5.59 → MEDIUM priority
```

**Key Insight**: Coverage acts as a **dampening multiplier**, not an additive factor. The example in the original documentation overestimated risk by treating coverage as additive. The actual implementation properly dampens scores for tested code.

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

**Note on debt_score**: The `debt_score` comes from **DebtAggregator** which combines multiple debt dimensions:
- Testing debt (unwrap calls, untested error paths)
- Resource debt (unclosed files, memory leaks)
- Duplication debt (code clones)

**Source**: [src/priority/debt_aggregator/](../../src/priority/debt_aggregator/)

**Example (legacy scoring):**
```
Function: process_payment
  - Cyclomatic complexity: 18
  - Cognitive complexity: 25
  - Coverage: 20%
  - Debt score: 15 (from DebtAggregator)

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
- Dynamic weights adjust based on coverage data availability
- Role multipliers adjust priority based on function importance
- Coverage propagation reduces false positives for utility functions
- Purity adjustments reward functional programming patterns

### Test Effort Assessment

Debtmap estimates testing difficulty based on complexity metrics using an advanced effort model.

**Source**: [src/risk/roi/effort.rs](../../src/risk/roi/effort.rs) (AdvancedEffortModel)

#### How Effort is Calculated

Test effort estimation involves two components:

1. **Test case count**: Estimated from **cyclomatic complexity** (branch coverage)
   - Each branch represents a code path that needs testing
   - Formula approximates test cases needed for comprehensive branch coverage

2. **Time estimate**: Calculated from **cognitive complexity** (comprehension difficulty)
   - Higher cognitive complexity means more time to understand and write tests
   - Includes setup cost, assertion cost, and complexity multipliers
   - Optional learning system can adjust estimates based on historical data

**Difficulty Levels:**
- **Trivial** (cognitive < 5): 1-2 test cases, < 1 hour
- **Simple** (cognitive 5-10): 3-5 test cases, 1-2 hours
- **Moderate** (cognitive 10-20): 6-10 test cases, 2-4 hours
- **Complex** (cognitive 20-40): 11-20 test cases, 4-8 hours
- **VeryComplex** (cognitive > 40): 20+ test cases, 8+ hours

**Test Effort includes:**
- **Cognitive load**: How hard to understand the function
- **Branch count** (cyclomatic): Number of paths to test
- **Recommended test cases**: Estimated from cyclomatic complexity
- **Estimated hours**: Derived from cognitive complexity with setup overhead

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

#### Legacy vs Unified Risk Distribution Fields

**IMPORTANT**: The field names differ between legacy and unified scoring systems:

| Unified Scoring (0-10 scale) | Legacy Scoring (RiskCategory enum) |
|------------------------------|-------------------------------------|
| `minimal_count` (0-2.9) | Not present |
| `low_count` (3.0-4.9) | `low_count` |
| `medium_count` (5.0-6.9) | `medium_count` |
| `high_count` (7.0-8.9) | `high_count` |
| `critical_count` (9.0-10.0) | `critical_count` |
| Not present | `well_tested_count` (legacy outcome) |

**Sources**:
- Unified priority tiers: [src/priority/tiers.rs](../../src/priority/tiers.rs)
- Legacy RiskCategory enum: [src/risk/mod.rs:36-42](../../src/risk/mod.rs)

**Note on minimal_count:**

In unified scoring (0-10 scale), `minimal_count` represents functions scoring 0-2.9, which includes:
- Simple utility functions with low complexity
- Helper functions with minimal risk
- Well-tested complex code that scores low due to coverage dampening

This is not a separate risk category but an **outcome** of the unified scoring system. Complex business logic with 95% test coverage appropriately receives a minimal score (0-2.9), reflecting that good testing mitigates complexity risk.

**When using legacy scoring**, there is **NO** `minimal_count` field. Instead, you'll see `well_tested_count` which represents functions that are both complex and well-tested (the desired outcome).

### Testing Recommendations

When coverage data is provided, Debtmap generates prioritized testing recommendations with ROI analysis.

**Source**: [src/risk/roi/mod.rs:66-113](../../src/risk/roi/mod.rs)

#### ROI Calculation

The ROI calculation is much richer than a simple risk/effort ratio. It includes cascade impacts, module multipliers, and complexity weighting:

```rust
// Source: src/risk/roi/mod.rs:66-113
ROI = ((Direct_Impact × Module_Multiplier) + (Cascade_Impact × Cascade_Weight))
      × Dependency_Factor × Complexity_Weight / Adjusted_Effort
```

**Formula Components:**

1. **Direct Impact**: Risk reduction from testing this function directly

2. **Module Multiplier** (based on module type):
   - EntryPoint = 2.0 (highest priority for user-facing code)
   - Core = 1.5 (domain logic)
   - Api = 1.2 (API endpoints)
   - Model = 1.1 (data models)
   - IO = 1.0 (baseline for I/O operations)

3. **Cascade Impact**: Risk reduction in dependent functions
   - Calculated using cascade analyzer
   - **Cascade Weight**: Configurable (default 0.5)
   - **Max Cascade Depth**: 3 hops (configurable)

4. **Dependency Factor**: Amplifies ROI based on number of dependents
   ```rust
   Dependency_Factor = 1.0 + min(dependent_count × 0.1, 1.0)
   ```
   - Capped at 2.0× multiplier
   - Rewards testing functions with many dependents

5. **Complexity Weight**: Penalizes trivial delegation functions
   - (cyclomatic=1, cognitive=0-1): 0.1 (trivial delegation)
   - (cyclomatic=1, cognitive=2-3): 0.3 (very simple)
   - (cyclomatic=2-3, any): 0.5 (simple)
   - (cyclomatic=4-5, any): 0.7 (moderate)
   - Other: 1.0 (complex, full weight)

6. **Adjusted Effort**: Base effort adjusted by learning system (if enabled)
   - Learning system tracks historical test writing effort
   - Adjusts estimates based on actual time spent

**ROI Scaling** (for intuitive 0-10 scale):
- raw_roi > 20.0: `10.0 + ln(raw_roi - 20.0)` (logarithmic dampening)
- 10.0 < raw_roi ≤ 20.0: `5.0 + (raw_roi - 20.0) × 0.5` (linear dampening)
- Otherwise: raw_roi (no scaling)

**Sources**:
- ROI model: [src/risk/roi/models.rs:4-11](../../src/risk/roi/models.rs)
- Effort estimation: [src/risk/roi/effort.rs](../../src/risk/roi/effort.rs)
- Cascade impact: [src/risk/roi/cascade.rs](../../src/risk/roi/cascade.rs)

#### Example ROI Output

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
    "recommended_test_cases": 12,
    "estimated_hours": 6.5
  },
  "roi": 8.2,
  "roi_breakdown": {
    "direct_impact": 35.2,
    "module_multiplier": 1.5,
    "cascade_impact": 12.4,
    "cascade_weight": 0.5,
    "dependency_factor": 1.3,
    "complexity_weight": 1.0,
    "adjusted_effort": 6.5
  },
  "rationale": "High complexity with low coverage (20%) and 3 downstream dependencies. Testing will reduce risk by 74%. Cascade effect improves 8 dependent functions.",
  "dependencies": {
    "upstream_callers": ["handle_payment_request"],
    "downstream_callees": ["validate_amount", "check_balance", "record_transaction"],
    "dependent_count": 13
  },
  "confidence": 0.85
}
```

**Interpreting ROI:**
- **ROI > 5.0**: Excellent return on investment, prioritize highly
- **ROI 3.0-5.0**: Good return, address soon
- **ROI 1.0-3.0**: Moderate return, plan for future work
- **ROI < 1.0**: Low return, consider other priorities

**Key Insight**: The cascade impact calculation means that testing a critical utility function with many dependents can have higher ROI than testing a complex but isolated function. This helps identify "force multiplier" tests that improve coverage across multiple modules.

