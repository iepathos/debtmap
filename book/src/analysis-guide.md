# Analysis Guide

This guide explains Debtmap's analysis capabilities, metrics, and methodologies in depth. Use this to understand what Debtmap measures, how it scores technical debt, and how to interpret analysis results for maximum impact.

## Overview

Debtmap analyzes code through multiple lenses to provide a comprehensive view of technical health:

- **Complexity Metrics** - Quantifies how difficult code is to understand and test
- **Debt Patterns** - Identifies 13 types of technical debt requiring attention
- **Risk Scoring** - Correlates complexity with test coverage to find truly risky code
- **Prioritization** - Ranks findings by impact to guide refactoring efforts

The goal is to move beyond simple "here are your problems" to "here's what to fix first and why."

## Complexity Metrics

Debtmap measures complexity using multiple complementary approaches. Each metric captures a different aspect of code difficulty.

### Cyclomatic Complexity

Measures the number of linearly independent paths through code - essentially counting decision points.

**How it works:**
- Start with a base complexity of 1
- Add 1 for each: `if`, `else if`, `match` arm, `while`, `for`, `&&`, `||`, `?` operator
- Does NOT increase for `else` (it's the alternate path, not a new decision)

**Thresholds:**
- **1-5**: Simple, easy to test - typically needs 1-3 test cases
- **6-10**: Moderate complexity - needs 4-8 test cases
- **11-20**: Complex, consider refactoring - needs 9+ test cases
- **20+**: Very complex, high risk - difficult to test thoroughly

**Example:**
```rust
fn validate_user(age: u32, has_license: bool, country: &str) -> bool {
    // Complexity: 4
    // Base (1) + if (1) + && (1) + match (1) = 4
    if age >= 18 && has_license {
        match country {
            "US" | "CA" => true,
            _ => false,
        }
    } else {
        false
    }
}
```

### Cognitive Complexity

Measures how difficult code is to understand by considering nesting depth and control flow interruptions.

**How it differs from cyclomatic:**
- Nesting increases weight (deeply nested code is harder to understand)
- Linear sequences don't increase complexity (easier to follow)
- Breaks and continues add complexity (interrupt normal flow)

**Calculation:**
- Each structure (if, loop, match) gets a base score
- Nesting multiplies the weight (nested structures = harder to understand)
- Break/continue/return in middle of function adds cognitive load

**Example:**
```rust
// Cyclomatic: 5, Cognitive: 8
fn process_items(items: Vec<Item>) -> Vec<Result> {
    let mut results = vec![];

    for item in items {                    // +1 cognitive
        if item.is_valid() {               // +2 (nested in loop)
            match item.type {              // +3 (nested 2 levels)
                Type::A => results.push(process_a(item)),
                Type::B => {
                    if item.priority > 5 { // +4 (nested 3 levels)
                        results.push(process_b_priority(item));
                    }
                }
                _ => continue,             // +1 (control flow interruption)
            }
        }
    }

    results
}
```

**Thresholds:**
- **0-5**: Trivial - anyone can understand
- **6-10**: Simple - straightforward logic
- **11-20**: Moderate - requires careful reading
- **21-40**: Complex - difficult to understand
- **40+**: Very complex - needs refactoring

### Entropy-Based Complexity Analysis

Uses information theory to distinguish genuinely complex code from pattern-based repetitive code. This dramatically reduces false positives for validation functions, dispatchers, and configuration parsers.

**How it works:**
1. **Token Entropy** (0.0-1.0): Measures variety in code tokens
   - High entropy (0.7+): Diverse logic, genuinely complex
   - Low entropy (0.0-0.4): Repetitive patterns, less complex than it appears

2. **Pattern Repetition** (0.0-1.0): Detects repetitive structures in AST
   - High repetition (0.7+): Similar blocks repeated (validation checks, case handlers)
   - Low repetition: Unique logic throughout

3. **Branch Similarity** (0.0-1.0): Analyzes similarity between conditional branches
   - High similarity (0.8+): Branches do similar things (consistent handling)
   - Low similarity: Each branch has unique logic

4. **Token Classification**: Categorizes tokens by type with weighted importance
   - Variables, methods, literals weighted differently
   - Focuses on structural complexity over superficial differences

**Dampening logic:** Dampening is applied when multiple factors indicate repetitive patterns:
- Low token entropy (< 0.4) indicates simple, repetitive patterns
- High pattern repetition (> 0.6) shows similar code blocks
- High branch similarity (> 0.7) indicates consistent branching logic

When these conditions are met:
```
effective_complexity = entropy × pattern_factor × similarity_factor
```

**Dampening cap:** The dampening factor has a minimum of 0.7, ensuring no more than 30% reduction in complexity scores. This prevents over-correction of pattern-based code and maintains a baseline complexity floor for functions that still require understanding and maintenance.

**Example:**
```rust
// Without entropy: Cyclomatic = 15 (appears very complex)
// With entropy: Effective = 5 (pattern-based, dampened 67%)
fn validate_config(config: &Config) -> Result<(), ValidationError> {
    if config.name.is_empty() { return Err(ValidationError::EmptyName); }
    if config.port == 0 { return Err(ValidationError::InvalidPort); }
    if config.host.is_empty() { return Err(ValidationError::EmptyHost); }
    if config.timeout == 0 { return Err(ValidationError::InvalidTimeout); }
    // ... 11 more similar checks
    Ok(())
}
```

**Enable in `.debtmap.toml`:**
```toml
[entropy]
enabled = true                 # Enable entropy analysis (default: true)
weight = 0.5                  # Weight in adjustment (0.0-1.0)
use_classification = true     # Advanced token classification
pattern_threshold = 0.7       # Pattern detection threshold
entropy_threshold = 0.4       # Entropy below this triggers dampening
branch_threshold = 0.8        # Branch similarity threshold
max_combined_reduction = 0.3  # Maximum 30% reduction
```

**Output fields in EntropyScore:**
- `unique_variables`: Count of distinct variables in the function (measures variable diversity)
- `max_nesting`: Maximum nesting depth detected (contributes to dampening calculation)
- `dampening_applied`: Actual dampening factor applied to the complexity score

### Nesting Depth

Maximum level of indentation in a function. Deep nesting makes code hard to follow.

**Thresholds:**
- **1-2**: Flat, easy to read
- **3-4**: Moderate nesting
- **5+**: Deep nesting, consider extracting functions

**Example:**
```rust
// Nesting depth: 4 (difficult to follow)
fn process(data: Data) -> Result<Output> {
    if data.is_valid() {                    // Level 1
        for item in data.items {            // Level 2
            if item.active {                // Level 3
                match item.type {           // Level 4
                    Type::A => { /* ... */ }
                    Type::B => { /* ... */ }
                }
            }
        }
    }
}
```

**Refactored:**
```rust
// Nesting depth: 2 (much clearer)
fn process(data: Data) -> Result<Output> {
    if !data.is_valid() {
        return Err(Error::Invalid);
    }

    data.items
        .iter()
        .filter(|item| item.active)
        .map(|item| process_item(item))     // Extract to separate function
        .collect()
}
```

### Function Length

Number of lines in a function. Long functions often violate single responsibility principle.

**Thresholds:**
- **1-20 lines**: Good - focused, single purpose
- **21-50 lines**: Acceptable - may have multiple steps
- **51-100 lines**: Long - consider breaking up
- **100+ lines**: Very long - definitely needs refactoring

**Why length matters:**
- Harder to understand and remember
- Harder to test thoroughly
- Often violates single responsibility
- Difficult to reuse

## Debt Patterns

Debtmap detects 24 types of technical debt, organized into 4 strategic categories. Each debt type is mapped to a category that guides prioritization and remediation strategies.

### Debt Type Enum

The `DebtType` enum defines all specific debt patterns that Debtmap can detect:

**Testing Debt:**
- `TestingGap` - Functions with insufficient test coverage
- `TestTodo` - TODO comments in test code
- `TestComplexity` - Test functions exceeding complexity thresholds
- `TestDuplication` - Duplicated code in test files
- `TestComplexityHotspot` - Complex test logic that's hard to maintain
- `AssertionComplexity` - Complex test assertions
- `FlakyTestPattern` - Non-deterministic test behavior

**Architecture Debt:**
- `ComplexityHotspot` - Functions exceeding complexity thresholds
- `DeadCode` - Unreachable or unused code
- `GodObject` - Classes with too many responsibilities
- `GodModule` - Modules with too many responsibilities
- `FeatureEnvy` - Using more data from other objects than own
- `PrimitiveObsession` - Overusing basic types instead of domain objects
- `MagicValues` - Unexplained literal values

**Performance Debt:**
- `AllocationInefficiency` - Inefficient memory allocations
- `StringConcatenation` - Inefficient string building in loops
- `NestedLoops` - Multiple nested iterations (O(n²) or worse)
- `BlockingIO` - Blocking I/O in async contexts
- `SuboptimalDataStructure` - Wrong data structure for access pattern
- `AsyncMisuse` - Improper async/await usage
- `ResourceLeak` - Resources not properly released
- `CollectionInefficiency` - Inefficient collection operations

**Code Quality Debt:**
- `Risk` - High-risk code (complex + poorly tested)
- `Duplication` - Duplicated code blocks
- `ErrorSwallowing` - Errors caught but ignored

### Debt Categories

The `DebtCategory` enum groups debt types into strategic categories:

```rust
pub enum DebtCategory {
    Architecture,  // Structure, design, complexity
    Testing,       // Coverage, test quality
    Performance,   // Speed, memory, efficiency
    CodeQuality,   // Maintainability, readability
}
```

**Category Mapping:**

| Debt Type | Category | Strategic Focus |
|-----------|----------|-----------------|
| ComplexityHotspot, DeadCode, GodObject, GodModule, FeatureEnvy, PrimitiveObsession, MagicValues | Architecture | Structural improvements, design patterns |
| TestingGap, TestTodo, TestComplexity, TestDuplication, TestComplexityHotspot, AssertionComplexity, FlakyTestPattern | Testing | Test coverage, test quality |
| AllocationInefficiency, StringConcatenation, NestedLoops, BlockingIO, SuboptimalDataStructure, AsyncMisuse, ResourceLeak, CollectionInefficiency | Performance | Runtime efficiency, resource usage |
| Risk, Duplication, ErrorSwallowing | CodeQuality | Maintainability, reliability |

### Examples by Category

#### Architecture Debt

**ComplexityHotspot**: Functions exceeding complexity thresholds
```rust
// Cyclomatic: 22, Cognitive: 35
fn process_transaction(tx: Transaction, account: &mut Account) -> Result<Receipt> {
    if tx.amount <= 0 {
        return Err(Error::InvalidAmount);
    }
    // ... deeply nested logic with many branches
    Ok(receipt)
}
```
**When detected**: Cyclomatic > 10 OR Cognitive > 15 (configurable)
**Action**: Break into smaller functions, extract validation, simplify control flow

**GodObject / GodModule**: Too many responsibilities
```rust
// God module: handles parsing, validation, storage, notifications
mod user_service {
    fn parse_user() { /* ... */ }
    fn validate_user() { /* ... */ }
    fn save_user() { /* ... */ }
    fn send_email() { /* ... */ }
    fn log_activity() { /* ... */ }
    // ... 20+ more functions
}
```
**When detected**: Pattern analysis for responsibility clustering
**Action**: Split into focused modules (parser, validator, repository, notifier)

**MagicValues**: Unexplained literals
```rust
// Bad: Magic numbers
fn calculate_price(quantity: u32) -> f64 {
    quantity as f64 * 19.99 + 5.0  // What are these numbers?
}

// Good: Named constants
const UNIT_PRICE: f64 = 19.99;
const SHIPPING_COST: f64 = 5.0;
fn calculate_price(quantity: u32) -> f64 {
    quantity as f64 * UNIT_PRICE + SHIPPING_COST
}
```

#### Testing Debt

**TestingGap**: Functions with insufficient test coverage
```rust
// 0% coverage - critical business logic untested
fn calculate_tax(amount: f64, region: &str) -> f64 {
    // Complex tax calculation logic
    // No tests exist for this function!
}
```
**When detected**: Coverage data shows function has < 80% line coverage
**Action**: Add unit tests to cover all branches and edge cases

**TestComplexity**: Test functions too complex
```rust
#[test]
fn complex_test() {
    // Cyclomatic: 12 (too complex for a test)
    for input in test_cases {
        if input.is_special() {
            match input.type {
                /* complex test logic */
            }
        }
    }
}
```
**When detected**: Test functions with cyclomatic > 10 or cognitive > 15
**Action**: Split into multiple focused tests, use test fixtures

**FlakyTestPattern**: Non-deterministic tests
```rust
#[test]
fn flaky_test() {
    let result = async_operation().await;  // Timing-dependent
    thread::sleep(Duration::from_millis(100));  // Race condition!
    assert_eq!(result.status, "complete");
}
```
**When detected**: Pattern analysis for timing dependencies, random values
**Action**: Use mocks, deterministic test data, proper async test utilities

#### Performance Debt

**AllocationInefficiency**: Excessive allocations
```rust
// Bad: Allocates on every iteration
fn process_items(items: &[Item]) -> Vec<String> {
    let mut results = Vec::new();
    for item in items {
        results.push(item.name.clone());  // Unnecessary clone
    }
    results
}

// Good: Pre-allocate, avoid clones
fn process_items(items: &[Item]) -> Vec<&str> {
    items.iter().map(|item| item.name.as_str()).collect()
}
```

**BlockingIO**: Blocking operations in async contexts
```rust
// Bad: Blocks async runtime
async fn load_data() -> Result<Data> {
    let file = std::fs::read_to_string("data.json")?;  // Blocking!
    parse_json(&file)
}

// Good: Async I/O
async fn load_data() -> Result<Data> {
    let file = tokio::fs::read_to_string("data.json").await?;
    parse_json(&file)
}
```

**NestedLoops**: O(n²) or worse complexity
```rust
// Bad: O(n²) nested loops
fn find_duplicates(items: &[Item]) -> Vec<(Item, Item)> {
    let mut dupes = vec![];
    for i in 0..items.len() {
        for j in i+1..items.len() {
            if items[i] == items[j] {
                dupes.push((items[i].clone(), items[j].clone()));
            }
        }
    }
    dupes
}

// Good: O(n) with HashSet
fn find_duplicates(items: &[Item]) -> Vec<Item> {
    let mut seen = HashSet::new();
    items.iter().filter(|item| !seen.insert(item)).cloned().collect()
}
```

#### Code Quality Debt

**Duplication**: Duplicated code blocks
```rust
// File A:
fn process_user(user: User) -> Result<()> {
    validate_email(&user.email)?;
    validate_age(user.age)?;
    save_to_database(&user)?;
    send_welcome_email(&user.email)?;
    Ok(())
}

// File B: Duplicated validation
fn process_admin(admin: Admin) -> Result<()> {
    validate_email(&admin.email)?;  // Duplicated
    validate_age(admin.age)?;       // Duplicated
    save_to_database(&admin)?;
    grant_admin_privileges(&admin)?;
    Ok(())
}
```
**When detected**: Similar code blocks > 50 lines (configurable)
**Action**: Extract shared code into reusable functions

**ErrorSwallowing**: Errors caught but ignored
```rust
// Bad: Error swallowed, no context
match risky_operation() {
    Ok(result) => process(result),
    Err(_) => {}, // Silent failure!
}

// Good: Error handled with context
match risky_operation() {
    Ok(result) => process(result),
    Err(e) => {
        log::error!("Risky operation failed: {}", e);
        return Err(e.into());
    }
}
```
**When detected**: Empty catch blocks, ignored Results
**Action**: Add proper error logging and propagation

**Risk**: High-risk code (complex + poorly tested)
```rust
// Cyclomatic: 18, Coverage: 20%, Risk Score: 47.6 (HIGH)
fn process_payment(tx: Transaction) -> Result<Receipt> {
    // Complex payment logic with minimal testing
    // High risk of bugs in production
}
```
**When detected**: Combines complexity metrics with coverage data
**Action**: Either add comprehensive tests OR refactor to reduce complexity

### Debt Scoring Formula

Each debt item gets a score based on priority and type:

```
debt_score = priority_weight × type_weight
```

**Priority weights:**
- Low = 1
- Medium = 3
- High = 5
- Critical = 10

**Combined examples:**
- Low Todo = 1 × 1 = 1
- Medium Fixme = 3 × 2 = 6
- High Complexity = 5 × 5 = 25
- Critical Complexity = 10 × 5 = 50

**Total debt score** = Sum of all debt item scores

Lower is better. Track over time to measure improvement.

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
Based on call graph analysis:
- High upstream caller count (many functions depend on this): 8-10
- On critical paths from entry points: 7-9
- Moderate dependencies: 4-6
- Isolated utilities: 1-3

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

**Note on well-tested functions:**
In unified scoring, well-tested complex code simply scores low (0-2.9 Minimal or 3-4.9 Low) due to coverage dampening - it's not a separate category. The `minimal_count` in the distribution represents functions with unified scores 0-2.9, which includes well-tested complex code.

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

### Reading Function Metrics

**Key fields:**

- `cyclomatic`: Decision points - guides test case count
- `cognitive`: Understanding difficulty - guides refactoring priority
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

## Analyzer Types

Debtmap supports multiple programming languages with varying levels of analysis capability.

### Supported Languages

**Rust** (Full Support)
- **Parser**: syn (native Rust AST)
- **Capabilities**:
  - Full complexity metrics (cyclomatic, cognitive, entropy)
  - Trait implementation tracking
  - Purity detection with confidence scoring
  - Call graph analysis (upstream callers, downstream callees)
  - Semantic function classification (entry points, business logic, data access, infrastructure, utilities, test code)
  - Enhanced call graph with transitive relationships
  - Macro expansion support for accurate complexity analysis
  - Pattern-based adjustments for macros and code generation
  - Visibility tracking (pub, pub(crate), private)
  - Test module detection (#[cfg(test)])

**Semantic Classification:**

Debtmap automatically identifies function roles in Rust code to apply appropriate role multipliers in unified scoring:

- **Entry Points**: Functions named `main`, `start`, or public functions in `bin/` modules
- **Business Logic**: Core domain functions with complex logic, algorithms, business rules
- **Data Access**: Functions performing database queries, file I/O, network operations
- **Infrastructure**: Logging, configuration, monitoring, error handling utilities
- **Utilities**: Helper functions, formatters, type converters, validation functions
- **Test Code**: Functions in `#[cfg(test)]` modules, functions with `#[test]` attribute

This classification feeds directly into the unified scoring system's role multiplier (see Risk Scoring section).

**Python** (Partial Support)
- **Parser**: rustpython-parser
- **Capabilities**:
  - Complexity metrics (cyclomatic, cognitive)
  - Python-specific error handling patterns
  - Purity detection for pure functions
  - Basic debt pattern detection
  - Limited call graph support

**JavaScript** (Partial Support)
- **Parser**: tree-sitter (JavaScript grammar)
- **File extensions**: .js, .jsx, .mjs, .cjs
- **Capabilities**:
  - ECMAScript complexity patterns
  - Basic complexity metrics
  - Function extraction
  - Limited pattern detection

**TypeScript** (Partial Support)
- **Parser**: tree-sitter (TypeScript grammar)
- **File extensions**: .ts, .tsx, .mts, .cts
- **Capabilities**:
  - Similar to JavaScript support
  - Type information currently not utilized
  - Basic complexity metrics
  - Limited pattern detection

**Unsupported Languages:**

Debtmap's `Language` enum contains only the four supported languages: Rust, Python, JavaScript, and TypeScript. Files with unsupported extensions are filtered out during the file discovery phase and never reach the analysis stage.

**File filtering behavior:**
- Discovery scans project for files matching supported extensions
- Unsupported files (`.cpp`, `.java`, `.go`, etc.) are skipped silently
- No analysis, metrics, or debt patterns are generated for filtered files
- Use `--languages` flag to explicitly control which languages to analyze

**Example:**
```bash
# Only analyze Rust files (skip Python/JS/TS)
debtmap analyze . --languages rust

# Analyze Rust and Python only
debtmap analyze . --languages rust,python
```

### Language Detection

Automatic detection by file extension:
```rust
let language = Language::from_path(&path);
```

Explicit language selection:
```bash
debtmap analyze . --languages rust,python
```

### Extensibility

Debtmap's architecture allows adding new languages:

1. **Implement Analyzer trait:**
```rust
pub trait Analyzer: Send + Sync {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast>;
    fn analyze(&self, ast: &Ast) -> FileMetrics;
    fn language(&self) -> Language;
}
```

2. **Register in get_analyzer():**
```rust
pub fn get_analyzer(language: Language) -> Box<dyn Analyzer> {
    match language {
        Language::Rust => Box::new(RustAnalyzer::new()),
        Language::YourLanguage => Box::new(YourAnalyzer::new()),
        // ...
    }
}
```

See `src/analyzers/rust.rs` for a complete implementation example.

## Advanced Features

### Purity Detection

Debtmap detects pure functions - those without side effects that always return the same output for the same input.

**What makes a function pure:**
- No I/O operations (file, network, database)
- No mutable global state
- No random number generation
- No system calls
- Deterministic output

**Purity detection is optional:**
- Both `is_pure` and `purity_confidence` are `Option` types
- May be `None` for some functions or languages where detection is not available
- Rust has the most comprehensive purity detection support

**Confidence scoring (when available):**
- **0.9-1.0**: Very confident (no side effects detected)
- **0.7-0.8**: Likely pure (minimal suspicious patterns)
- **0.5-0.6**: Uncertain (some suspicious patterns)
- **0.0-0.4**: Likely impure (side effects detected)

**Example:**
```rust
// Pure: confidence = 0.95
fn calculate_total(items: &[Item]) -> f64 {
    items.iter().map(|i| i.price).sum()
}

// Impure: confidence = 0.1 (I/O detected)
fn save_total(items: &[Item]) -> Result<()> {
    let total = items.iter().map(|i| i.price).sum();
    write_to_file(total)  // Side effect!
}
```

**Benefits:**
- Pure functions are easier to test
- Can be safely cached or memoized
- Safe to parallelize
- Easier to reason about

### Data Flow Analysis

Debtmap builds a comprehensive `DataFlowGraph` that extends basic call graph analysis with variable dependencies, data transformations, I/O operations, and purity tracking.

#### Call Graph Foundation

**Upstream callers** - Who calls this function
- Indicates impact radius
- More callers = higher impact if it breaks

**Downstream callees** - What this function calls
- Indicates dependencies
- More callees = more integration testing needed

**Example:**
```json
{
  "name": "process_payment",
  "upstream_callers": [
    "handle_checkout",
    "process_subscription",
    "handle_refund"
  ],
  "downstream_callees": [
    "validate_payment_method",
    "calculate_fees",
    "record_transaction",
    "send_receipt"
  ]
}
```

#### Variable Dependency Tracking

`DataFlowGraph` tracks which variables each function depends on:

```rust
pub struct DataFlowGraph {
    // Maps function_id -> set of variable names used
    variable_dependencies: HashMap<String, HashSet<String>>,
    // ...
}
```

**What it tracks:**
- Local variables accessed in function body
- Function parameters
- Captured variables (closures)
- Mutable vs immutable references

**Benefits:**
- Identify functions coupled through shared state
- Detect potential side effect chains
- Guide refactoring to reduce coupling

**Example output:**
```json
{
  "function": "calculate_total",
  "variable_dependencies": ["items", "tax_rate", "discount", "total"],
  "parameter_count": 3,
  "local_var_count": 1
}
```

#### Data Transformation Patterns

`DataFlowGraph` identifies common functional programming patterns:

```rust
pub enum TransformationType {
    Map,        // Transform each element
    Filter,     // Select subset of elements
    Reduce,     // Aggregate to single value
    FlatMap,    // Transform and flatten
    Unknown,    // Other transformations
}
```

**Pattern detection:**
- Recognizes iterator chains (`.map()`, `.filter()`, `.fold()`)
- Identifies functional vs imperative data flow
- Tracks input/output variable relationships

**Example:**
```rust
// Detected as: Filter → Map → Reduce pattern
fn total_active_users(users: &[User]) -> f64 {
    users.iter()
        .filter(|u| u.active)      // Filter transformation
        .map(|u| u.balance)        // Map transformation
        .sum()                      // Reduce transformation
}
```

**Transformation metadata:**
```json
{
  "function": "total_active_users",
  "input_vars": ["users"],
  "output_vars": ["sum_result"],
  "transformation_type": "Reduce",
  "is_functional_style": true,
  "pipeline_length": 3
}
```

#### I/O Operation Detection

Tracks functions performing I/O operations for purity and performance analysis:

**I/O categories tracked:**
- **File I/O**: `std::fs`, `File::open`, `read_to_string`
- **Network I/O**: HTTP requests, socket operations
- **Database I/O**: SQL queries, ORM operations
- **System calls**: Process spawning, environment access
- **Blocking operations**: `thread::sleep`, synchronous I/O in async

**Example detection:**
```rust
// Detected I/O operations: FileRead, FileWrite
fn save_config(config: &Config, path: &Path) -> Result<()> {
    let json = serde_json::to_string(config)?;  // No I/O
    std::fs::write(path, json)?;                 // FileWrite detected
    Ok(())
}
```

**I/O metadata:**
```json
{
  "function": "save_config",
  "io_operations": ["FileWrite"],
  "is_blocking": true,
  "affects_purity": true,
  "async_safe": false
}
```

#### Purity Analysis Integration

`DataFlowGraph` integrates with purity detection to provide comprehensive side effect analysis:

**Side effect tracking:**
- I/O operations (file, network, console)
- Global state mutations
- Random number generation
- System time access
- Non-deterministic behavior

**Purity confidence factors:**
- **1.0**: Pure mathematical function, no side effects
- **0.8**: Pure with deterministic data transformations
- **0.5**: Mixed - some suspicious patterns
- **0.2**: Likely impure - I/O detected
- **0.0**: Definitely impure - multiple side effects

**Example analysis:**
```json
{
  "function": "calculate_discount",
  "is_pure": true,
  "purity_confidence": 0.95,
  "side_effects": [],
  "deterministic": true,
  "safe_to_parallelize": true,
  "safe_to_cache": true
}
```

#### Modification Impact Analysis

`DataFlowGraph` calculates the impact of modifying a function:

```rust
pub struct ModificationImpact {
    pub function_name: String,
    pub affected_functions: Vec<String>,  // Upstream callers
    pub dependency_count: usize,          // Downstream callees
    pub has_side_effects: bool,
    pub risk_level: RiskLevel,
}
```

**Risk level calculation:**
- **Critical**: Many upstream callers + side effects + low test coverage
- **High**: Many callers OR side effects with moderate coverage
- **Medium**: Few callers with side effects OR many callers with good coverage
- **Low**: Few callers, no side effects, or well-tested

**Example impact analysis:**
```json
{
  "function": "validate_payment_method",
  "modification_impact": {
    "affected_functions": [
      "process_payment",
      "refund_payment",
      "update_payment_method",
      "validate_subscription"
    ],
    "affected_count": 4,
    "dependency_count": 8,
    "has_side_effects": true,
    "io_operations": ["DatabaseRead", "NetworkCall"],
    "risk_level": "High",
    "recommendation": "Comprehensive testing required - 4 functions depend on this, performs I/O"
  }
}
```

**Using modification impact:**
```bash
# Analyze impact before refactoring
debtmap analyze . --format json | jq '.functions[] | select(.name == "validate_payment_method") | .modification_impact'
```

**Impact analysis uses:**
- **Refactoring planning**: Understand blast radius before changes
- **Test prioritization**: Focus tests on high-impact functions
- **Code review**: Flag high-risk changes for extra scrutiny
- **Dependency management**: Identify tightly coupled components

#### DataFlowGraph Methods

Key methods for data flow analysis:

```rust
// Add function with its dependencies
pub fn add_function(&mut self, function_id: String, callees: Vec<String>)

// Track variable dependencies
pub fn add_variable_dependency(&mut self, function_id: String, var_name: String)

// Record I/O operations
pub fn add_io_operation(&mut self, function_id: String, io_type: IoType)

// Calculate modification impact
pub fn calculate_modification_impact(&self, function_id: &str) -> ModificationImpact

// Get all functions affected by a change
pub fn get_affected_functions(&self, function_id: &str) -> Vec<String>

// Find functions with side effects
pub fn find_functions_with_side_effects(&self) -> Vec<String>
```

**Integration in analysis pipeline:**
1. Parser builds initial call graph
2. DataFlowGraph extends with variable/I/O tracking
3. Purity analyzer adds side effect information
4. Modification impact calculated for each function
5. Results used in prioritization and risk scoring

**Connection to Unified Scoring:**

The dependency analysis from DataFlowGraph directly feeds into the **unified scoring system's dependency factor** (20% weight):

- **Dependency Factor Calculation**: Functions with high upstream caller count or on critical paths from entry points receive higher dependency scores (8-10)
- **Isolated Utilities**: Functions with few or no callers score lower (1-3) on dependency factor
- **Impact Prioritization**: This helps prioritize functions where bugs have wider impact across the codebase
- **Modification Risk**: The modification impact analysis uses dependency data to calculate blast radius when changes are made

**Example:**
```
Function: validate_payment_method
  Upstream callers: 4 (high impact)
  → Dependency Factor: 8.0

Function: format_currency_string
  Upstream callers: 0 (utility)
  → Dependency Factor: 1.5

Both have same complexity, but validate_payment_method gets higher unified score
due to its critical role in the call graph.
```

This integration ensures that the unified scoring system considers not just internal function complexity and test coverage, but also the function's importance in the broader codebase architecture.

### Entropy-Based Complexity

Advanced pattern detection to reduce false positives.

**Token Classification:**
```rust
enum TokenType {
    Variable,     // Weight: 1.0
    Method,       // Weight: 1.5 (more important)
    Literal,      // Weight: 0.5 (less important)
    Keyword,      // Weight: 0.8
    Operator,     // Weight: 0.6
}
```

**Shannon Entropy Calculation:**
```
H(X) = -Σ p(x) × log₂(p(x))
```
where p(x) is the probability of each token type.

**Dampening Decision:**
```rust
if entropy_score.token_entropy < 0.4
   && entropy_score.pattern_repetition > 0.6
   && entropy_score.branch_similarity > 0.7
{
    // Apply dampening
    effective_complexity = base_complexity × (1 - dampening_factor);
}
```

**Output explanation:**
```
Function: validate_input
  Cyclomatic: 15 → Effective: 5
  Reasoning:
    - High pattern repetition detected (85%)
    - Low token entropy indicates simple patterns (0.32)
    - Similar branch structures found (92% similarity)
    - Complexity reduced by 67% due to pattern-based code
```

### Entropy Analysis Caching

`EntropyAnalyzer` includes an LRU-style cache for performance optimization when analyzing large codebases or performing repeated analysis.

#### Cache Structure

```rust
struct CacheEntry {
    score: EntropyScore,
    timestamp: Instant,
    hit_count: usize,
}
```

**Cache configuration:**
- **Default size**: 1000 entries
- **Eviction policy**: LRU (Least Recently Used)
- **Memory per entry**: ~128 bytes
- **Total memory overhead**: ~128 KB for default size

#### Cache Statistics

The analyzer tracks cache performance:

```rust
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub hit_rate: f64,
    pub memory_bytes: usize,
}
```

**Example stats output:**
```json
{
  "entropy_cache_stats": {
    "hits": 3427,
    "misses": 1573,
    "evictions": 573,
    "hit_rate": 0.685,
    "memory_bytes": 128000
  }
}
```

**Hit rate interpretation:**
- **> 0.7**: Excellent - many repeated analyses, cache is effective
- **0.4-0.7**: Good - moderate reuse, typical for incremental analysis
- **< 0.4**: Low - mostly unique functions, cache less helpful

#### Performance Benefits

**Typical performance gains:**
- **Cold analysis**: 100ms baseline (no cache benefit)
- **Incremental analysis**: 30-40ms (~60-70% faster) for unchanged functions
- **Re-analysis**: 15-20ms (~80-85% faster) for recently analyzed functions

**Best for:**
- **Watch mode**: Analyzing on file save (repeated analysis of same files)
- **CI/CD**: Comparing feature branch to main (overlap in functions)
- **Large codebases**: Many similar functions benefit from pattern caching

**Memory estimation:**
```
Total cache memory = entry_count × 128 bytes

Examples:
- 1,000 entries: ~128 KB (default)
- 5,000 entries: ~640 KB (large projects)
- 10,000 entries: ~1.25 MB (very large)
```

#### Cache Management

**Automatic eviction:**
- When cache reaches size limit, oldest entries evicted
- Hit count influences retention (frequently accessed stay longer)
- Timestamp used for LRU ordering

**Cache invalidation:**
- Function source changes invalidate entry
- Cache cleared between major analysis runs
- No manual invalidation needed

**Configuration (if exposed in future):**
```toml
[entropy.cache]
enabled = true
size = 1000           # Number of entries
ttl_seconds = 3600    # Optional: expire after 1 hour
```

### Context-Aware Analysis

Debtmap adjusts analysis based on code context:

**Pattern Recognition:**
- Validation patterns (repetitive checks)
- Dispatcher patterns (routing logic)
- Builder patterns (fluent APIs)
- Configuration parsers (key-value processing)

**Adjustment Strategies:**
- Reduce false positives for recognized patterns
- Apply appropriate thresholds by pattern type
- Consider pattern confidence in scoring

**Example:**
```rust
// Recognized as "validation_pattern"
// Complexity dampening applied
fn validate_user_input(input: &UserInput) -> Result<()> {
    if input.name.is_empty() { return Err(Error::EmptyName); }
    if input.email.is_empty() { return Err(Error::EmptyEmail); }
    if input.age < 13 { return Err(Error::TooYoung); }
    // ... more similar validations
    Ok(())
}
```

### Coverage Integration

Debtmap parses LCOV coverage data for risk analysis:

**LCOV Support:**
- Standard format from most coverage tools
- Line-level coverage tracking
- Function-level aggregation

**Coverage Index:**
- O(1) exact name lookups (~0.5μs)
- O(log n) line-based fallback (~5-8μs)
- ~200 bytes per function
- Thread-safe (Arc<CoverageIndex>)

#### Performance Characteristics

**Index Build Performance:**
- Index construction: O(n), approximately 20-30ms for 5,000 functions
- Memory usage: ~200 bytes per record (~2MB for 5,000 functions)
- Scales linearly with function count

**Lookup Performance:**
- Exact match (function name): O(1) average, ~0.5μs per lookup
- Line-based fallback: O(log n), ~5-8μs per lookup
- Cache-friendly data structure for hot paths

**Analysis Overhead:**
- Coverage integration overhead: ~2.5x baseline analysis time
- Target overhead: ≤3x (maintained through optimizations)
- Example timing: 53ms baseline → 130ms with coverage (2.45x overhead)
- Overhead includes index build + lookups + coverage propagation

**Thread Safety:**
- Coverage index wrapped in `Arc<CoverageIndex>` for lock-free parallel access
- Multiple analyzer threads can query coverage simultaneously
- No contention on reads, suitable for parallel analysis pipelines

**Memory Footprint:**
```
Total memory = (function_count × 200 bytes) + index overhead

Examples:
- 1,000 functions: ~200 KB
- 5,000 functions: ~2 MB
- 10,000 functions: ~4 MB
```

**Scalability:**
- Tested with codebases up to 10,000 functions
- Performance remains predictable and acceptable
- Memory usage stays bounded and reasonable

**Generating coverage:**
```bash
# Rust
cargo tarpaulin --out lcov --output-dir target/coverage

# Python
pytest --cov --cov-report=lcov

# JavaScript/TypeScript
jest --coverage --coverageReporters=lcov

# Go
go test -coverprofile=coverage.out
gocover-cobertura < coverage.out > coverage.lcov
```

**Using with Debtmap:**
```bash
debtmap analyze . --lcov target/coverage/lcov.info
```

**Coverage dampening:**
When coverage data is provided, debt scores are dampened for well-tested code:
```
final_score = base_score × (1 - coverage_percentage)
```

This ensures well-tested complex code gets lower priority than untested simple code.

## Example Outputs

### High Complexity Function (Needs Refactoring)

**Terminal Output:**
```
#1 SCORE: 9.2 [CRITICAL]
├─ COMPLEXITY: ./src/payments/processor.rs:145 process_transaction()
├─ ACTION: Refactor into 4 smaller functions
├─ IMPACT: Reduce complexity from 25 to 8, improve testability
├─ COMPLEXITY: cyclomatic=25, branches=25, cognitive=38, nesting=5, lines=120
├─ DEPENDENCIES: 3 upstream, 8 downstream
└─ WHY: Exceeds all complexity thresholds, difficult to test and maintain
```

**JSON Output:**
```json
{
  "id": "complexity_src_payments_processor_rs_145",
  "debt_type": "Complexity",
  "priority": "Critical",
  "file": "src/payments/processor.rs",
  "line": 145,
  "message": "Function exceeds complexity threshold",
  "context": "Cyclomatic: 25, Cognitive: 38, Nesting: 5",
  "function_metrics": {
    "name": "process_transaction",
    "cyclomatic": 25,
    "cognitive": 38,
    "nesting": 5,
    "length": 120,
    "is_pure": false,
    "purity_confidence": 0.15,
    "upstream_callers": ["handle_payment", "handle_subscription", "handle_refund"],
    "downstream_callees": ["validate", "calculate_fees", "record_transaction", "send_receipt", "update_balance", "log_transaction", "check_fraud", "notify_user"]
  }
}
```

### Well-Tested Complex Function (Good Example)

**Terminal Output:**
```
Function: calculate_tax (WELL TESTED - Good Example!)
  File: src/tax/calculator.rs:78
  Complexity: Cyclomatic=18, Cognitive=22
  Coverage: 98%
  Risk: LOW

  Why this is good:
  - High complexity is necessary (tax rules are complex)
  - Thoroughly tested with 45 test cases
  - Clear documentation of edge cases
  - Good example to follow for other complex logic
```

### Test Gap (Needs Testing)

**Terminal Output:**
```
#2 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/analyzers/rust_call_graph.rs:38 add_function_to_graph()
├─ ACTION: Add 6 unit tests for full coverage
├─ IMPACT: Full test coverage, -3.7 risk reduction
├─ COMPLEXITY: cyclomatic=6, branches=6, cognitive=8, nesting=2, lines=32
├─ DEPENDENCIES: 0 upstream, 11 downstream
├─ TEST EFFORT: Simple (2-3 hours)
└─ WHY: Business logic with 0% coverage, manageable complexity (cyclo=6, cog=8)
    High impact - 11 functions depend on this
```

**JSON Output:**
```json
{
  "function": "add_function_to_graph",
  "file": "src/analyzers/rust_call_graph.rs",
  "line": 38,
  "current_risk": 8.9,
  "potential_risk_reduction": 3.7,
  "recommendation": {
    "action": "Add unit tests",
    "details": "Add 6 unit tests for full coverage",
    "effort_estimate": "2-3 hours"
  },
  "test_effort": {
    "estimated_difficulty": "Simple",
    "cognitive_load": 8,
    "branch_count": 6,
    "recommended_test_cases": 6
  },
  "complexity": {
    "cyclomatic": 6,
    "cognitive": 8,
    "nesting": 2,
    "length": 32
  },
  "dependencies": {
    "upstream_callers": [],
    "downstream_callees": [
      "get_function_name", "extract_parameters", "parse_return_type",
      "add_to_registry", "update_call_sites", "resolve_types",
      "track_visibility", "record_location", "increment_counter",
      "validate_signature", "log_registration"
    ]
  },
  "roi": 4.5
}
```

### Entropy-Dampened Validation Function

**Terminal Output:**
```
Function: validate_config
  File: src/config/validator.rs:23
  Cyclomatic: 20 → Effective: 7 (65% dampened)
  Risk: LOW

  Entropy Analysis:
    ├─ Token Entropy: 0.28 (low variety - repetitive patterns)
    ├─ Pattern Repetition: 0.88 (high similarity between checks)
    ├─ Branch Similarity: 0.91 (consistent validation structure)
    └─ Reasoning: Complexity reduced by 65% due to pattern-based code

  This appears complex but is actually a repetitive validation pattern.
  Lower priority for refactoring.
```

### Before/After Refactoring Comparison

**Before:**
```
Function: process_order
  Cyclomatic: 22
  Cognitive: 35
  Coverage: 15%
  Risk Score: 52.3 (CRITICAL)
  Debt Score: 50 (Critical Complexity)
```

**After:**
```
Function: process_order (refactored)
  Cyclomatic: 5
  Cognitive: 6
  Coverage: 92%
  Risk Score: 2.1 (LOW)
  Debt Score: 0 (no debt)

Extracted functions:
  - validate_order (Cyclomatic: 4, Coverage: 100%)
  - calculate_totals (Cyclomatic: 3, Coverage: 95%)
  - apply_discounts (Cyclomatic: 6, Coverage: 88%)
  - finalize_order (Cyclomatic: 4, Coverage: 90%)

Impact:
  ✓ Complexity reduced by 77%
  ✓ Coverage improved by 513%
  ✓ Risk reduced by 96%
  ✓ Created 4 focused, testable functions
```

## Next Steps

- **[Output Formats](./output-formats.md)** - Detailed JSON schema and integration patterns
- **[Configuration](./configuration.md)** - Customize thresholds and analysis behavior

For questions or issues, visit [GitHub Issues](https://github.com/iepathos/debtmap/issues).
