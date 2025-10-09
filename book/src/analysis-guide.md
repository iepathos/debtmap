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

Debtmap detects 13 types of technical debt, each with a severity weight that affects scoring.

### Basic Markers (Weight = 1)

**Todo**: TODO comments in production code
```rust
// TODO: Add error handling here
fn process() { /* ... */ }
```

**TestTodo**: TODO comments in test code
```rust
#[test]
fn test_feature() {
    // TODO: Add edge case testing
    assert_eq!(process(), expected);
}
```

**When detected**: Comment lines containing `TODO:` or `todo!` macro (Rust)
**Impact**: Low - markers for future work, not immediate problems
**Action**: Plan when to address or document why it's deferred

### Fixable Issues (Weight = 2)

**Fixme**: FIXME comments indicating known problems
```rust
// FIXME: This breaks with negative numbers
fn calculate(value: i32) -> i32 { /* ... */ }
```

**TestComplexity**: Test functions exceeding complexity thresholds
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

**TestDuplication**: Duplicated code in test files
```rust
#[test]
fn test_a() {
    let setup = create_test_data();  // Duplicated
    let result = process(setup);
    assert!(result.is_ok());
}

#[test]
fn test_b() {
    let setup = create_test_data();  // Same setup duplicated
    let result = process_different(setup);
    assert!(result.is_ok());
}
```

**When detected**:
- Comments with `FIXME:` or `fixme!` macro
- Test functions with cyclomatic > 10 or cognitive > 15
- Similar code blocks > 10 lines in test files

**Impact**: Medium - affects test quality and maintainability
**Action**: Refactor tests to be simpler, extract common test utilities

### Code Quality (Weight = 3)

**CodeSmell**: Anti-patterns and bad practices
- God objects (classes with too many responsibilities)
- Long parameter lists (> 5 parameters)
- Feature envy (using more data from other objects than own)
- Primitive obsession (overusing basic types instead of domain objects)
- Magic numbers/strings (unexplained literal values)

```rust
// Code smell: Magic numbers
fn calculate_price(quantity: u32) -> f64 {
    quantity as f64 * 19.99 + 5.0  // What are these numbers?
}

// Better: Named constants
const UNIT_PRICE: f64 = 19.99;
const SHIPPING_COST: f64 = 5.0;

fn calculate_price(quantity: u32) -> f64 {
    quantity as f64 * UNIT_PRICE + SHIPPING_COST
}
```

**Dependency**: Problematic dependencies
- Circular dependencies between modules
- High coupling (too many dependencies)
- Deprecated dependencies

**CodeOrganization**: Poor structure
- Files too large (> 500 lines)
- Modules with unclear responsibilities
- Inconsistent naming patterns

**TestQuality**: Low-quality tests
- Tests with complex assertions
- Flaky test patterns (non-deterministic behavior)
- Tests that test implementation instead of behavior

**When detected**: Pattern analysis, heuristics for each smell type
**Impact**: Medium-High - affects maintainability and evolution
**Action**: Refactor to follow better patterns, extract responsibilities

### Serious Issues (Weight = 4)

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

// File B:
fn process_admin(admin: Admin) -> Result<()> {
    validate_email(&admin.email)?;  // Duplicated
    validate_age(admin.age)?;       // Duplicated
    save_to_database(&admin)?;
    grant_admin_privileges(&admin)?;
    Ok(())
}
```

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

**ResourceManagement**: Resource leaks or improper cleanup
- Files not closed
- Connections not released
- Memory leaks
- Inefficient allocations in hot paths
- Blocking I/O in async contexts

```rust
// Bad: File might not be closed on error
fn read_config() -> Result<Config> {
    let file = File::open("config.toml")?;
    let config = parse_toml(file)?;  // If this fails, file not closed
    Ok(config)
}

// Good: RAII ensures cleanup
fn read_config() -> Result<Config> {
    let file = File::open("config.toml")?;
    let config = parse_toml(file)?;
    Ok(config)
    // File automatically closed when dropped
}
```

**When detected**:
- Duplication: Similar code blocks > 50 lines (configurable)
- Error swallowing: Empty catch blocks, ignored Results
- Resource management: Pattern analysis for leaks, async/await misuse

**Impact**: High - can cause bugs, performance issues, data loss
**Action**: Extract shared code, add proper error handling, fix resource cleanup

### High Severity (Weight = 5)

**Complexity**: Functions exceeding complexity thresholds
```rust
// Cyclomatic: 22, Cognitive: 35
fn process_transaction(tx: Transaction, account: &mut Account) -> Result<Receipt> {
    if tx.amount <= 0 {
        return Err(Error::InvalidAmount);
    }

    if account.balance < tx.amount {
        if account.overdraft_enabled {
            if account.overdraft_limit >= tx.amount - account.balance {
                // Process with overdraft
                match tx.type {
                    TransactionType::Debit => { /* complex logic */ }
                    TransactionType::Credit => { /* complex logic */ }
                    TransactionType::Transfer => { /* complex logic */ }
                }
            } else {
                return Err(Error::OverdraftLimitExceeded);
            }
        } else {
            return Err(Error::InsufficientFunds);
        }
    } else {
        // Regular processing
        match tx.type {
            TransactionType::Debit => { /* complex logic */ }
            TransactionType::Credit => { /* complex logic */ }
            TransactionType::Transfer => { /* complex logic */ }
        }
    }

    // More complex validation and processing...
    Ok(receipt)
}
```

**When detected**: Cyclomatic > 10 OR Cognitive > 15 (configurable)
**Impact**: Very High - hard to test, likely has bugs, difficult to modify
**Action**: Break into smaller functions, extract validation, simplify control flow

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

### Risk Categories

Functions are classified into five risk categories:

**Critical** (score ≥ 50)
- High complexity (cyclomatic > 15) AND low coverage (< 30%)
- Untested code that's likely to break and hard to fix
- **Action**: Immediate attention required - add tests or refactor

**High** (score 25-49)
- High complexity (cyclomatic > 10) AND moderate coverage (< 60%)
- Risky code with incomplete testing
- **Action**: Should be addressed soon

**Medium** (score 10-24)
- Moderate complexity (cyclomatic > 5) AND low coverage (< 50%)
- OR: High complexity with good coverage
- **Action**: Plan for next sprint

**Low** (score 5-9)
- Low complexity OR high coverage
- Well-managed code
- **Action**: Monitor, low priority

**Note**: The risk categorization is based on the RiskLevel enum which includes Low, Medium, High, and Critical levels. Well-tested complex code will typically fall into the Low risk category due to coverage dampening.

### Risk Calculation

Risk score combines multiple factors:

```rust
risk_score = complexity_factor + coverage_factor + debt_factor

where:
  complexity_factor = (cyclomatic / 5) + (cognitive / 10)
  coverage_factor = (1 - coverage_percentage) × 50
  debt_factor = debt_score / 10  // If debt data available
```

**Example:**
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
    "well_tested_count": 234,
    "total_functions": 870
  },
  "codebase_risk_score": 1247.5
}
```

**Interpreting distribution:**
- **Healthy codebase**: Most functions in Low/WellTested
- **Needs attention**: Many Critical/High risk functions
- **Technical debt**: High codebase risk score

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
        "visibility": "pub",
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

Debtmap provides multiple prioritization strategies:

**1. By Unified Score (default)**
```bash
debtmap analyze . --top 10
```
Shows top 10 items by combined complexity, coverage, and dependency factors.

**2. By Risk Category**
```bash
debtmap analyze . --min-priority high
```
Shows only HIGH and CRITICAL priority items.

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
Prioritizes by return on investment for testing/refactoring.

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
  - Pattern-based adjustments for macros
  - Visibility tracking (pub, pub(crate), private)
  - Test module detection (#[cfg(test)])

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

**Unknown** (Unsupported)
- Files with unsupported extensions are classified as `Language::Unknown`
- These files are skipped during analysis
- No metrics or debt patterns are extracted

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

### Call Graph Analysis

Debtmap builds a call graph showing function relationships:

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

**Impact analysis:**
- **Entry points**: Functions with no upstream callers (main, handlers)
- **Critical paths**: Functions with many upstream callers
- **Integration complexity**: Functions with many downstream callees

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

**Performance:**
- Index build: ~20-30ms for 5,000 functions
- Analysis overhead: ~2.5x baseline (target: ≤3x)
- Example: 53ms → 130ms for 100 files

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
