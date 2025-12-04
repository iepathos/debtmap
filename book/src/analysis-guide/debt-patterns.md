# Debt Patterns

## Debt Patterns

Debtmap detects 27 types of technical debt, organized into 4 strategic categories. Each debt type is mapped to a category that guides prioritization and remediation strategies.

**Source**: All debt types and category mappings verified from src/priority/mod.rs:158-347

### Debt Type Enum

The `DebtType` enum defines all specific debt patterns that Debtmap can detect.

**Note**: Each `DebtType` variant carries structured diagnostic data specific to that pattern. For example, `ComplexityHotspot` includes `cyclomatic`, `cognitive`, and `adjusted_cyclomatic` (entropy-adjusted) fields that provide detailed metrics for the detected issue. This structured data enables precise prioritization and actionable recommendations.

**Source**: DebtType structure defined in src/priority/mod.rs:158-288

**Testing Debt:**
- `TestingGap` - Functions with insufficient test coverage
- `TestTodo` - TODO comments in test code
- `TestComplexity` - Test functions exceeding complexity thresholds
- `TestDuplication` - Duplicated code in test files
- `TestComplexityHotspot` - Complex test logic that's hard to maintain
- `AssertionComplexity` - Complex test assertions
- `FlakyTestPattern` - Non-deterministic test behavior

**Architecture Debt:**
- `GodObject` - Classes with too many responsibilities
- `GodModule` - Modules with too many responsibilities
- `FeatureEnvy` - Using more data from other objects than own
- `PrimitiveObsession` - Overusing basic types instead of domain objects
- `ScatteredType` - Types with methods scattered across multiple files
- `OrphanedFunctions` - Functions that should be methods on a type
- `UtilitiesSprawl` - Utility modules with poor cohesion

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
- `ComplexityHotspot` - Functions exceeding complexity thresholds
- `DeadCode` - Unreachable or unused code
- `MagicValues` - Unexplained literal values
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

**Source**: Verified from src/priority/mod.rs:309-347

| Debt Type | Category | Strategic Focus |
|-----------|----------|-----------------|
| GodObject, GodModule, FeatureEnvy, PrimitiveObsession, ScatteredType, OrphanedFunctions, UtilitiesSprawl | Architecture | Structural improvements, design patterns, type organization |
| TestingGap, TestTodo, TestComplexity, TestDuplication, TestComplexityHotspot, AssertionComplexity, FlakyTestPattern | Testing | Test coverage, test quality |
| AllocationInefficiency, StringConcatenation, NestedLoops, BlockingIO, SuboptimalDataStructure, AsyncMisuse, ResourceLeak, CollectionInefficiency | Performance | Runtime efficiency, resource usage |
| ComplexityHotspot, DeadCode, MagicValues, Risk, Duplication, ErrorSwallowing | CodeQuality | Maintainability, reliability, code clarity |

**Language-Specific Debt Patterns:**

Some debt patterns only apply to languages with specific features:
- **BlockingIO, AsyncMisuse**: Async-capable languages (Rust)
- **AllocationInefficiency, ResourceLeak**: Languages with manual memory management (Rust)
- **Error handling patterns**: Vary by language error model (Result in Rust)

Debtmap automatically applies only the relevant debt patterns during analysis.

### Examples by Category

#### Architecture Debt

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
**When detected**: Complexity-weighted scoring system (see detailed explanation below)
**Action**: Split into focused modules (parser, validator, repository, notifier)

#### Complexity-Weighted God Object Detection

Debtmap uses **complexity-weighted scoring** for god object detection to reduce false positives on well-refactored code. This ensures that a file with 100 simple helper functions doesn't rank higher than a file with 10 complex functions.

**The Problem:**

Traditional god object detection counts methods:
- File A: 100 methods (average complexity: 1.5) → Flagged as god object
- File B: 10 methods (average complexity: 17.0) → Not flagged

But File A might be a well-organized utility module with many small helpers, while File B is truly problematic with highly complex functions that need refactoring.

**The Solution:**

Debtmap weights each function by its cyclomatic complexity using this formula:

```
weight = (max(1, complexity) / 3)^1.5
```

**Weight Examples:**
- Simple helper (complexity 1): weight ≈ 0.19
- Baseline function (complexity 3): weight = 1.0
- Moderate function (complexity 9): weight ≈ 5.2
- Complex function (complexity 17): weight ≈ 13.5
- Critical function (complexity 33): weight ≈ 36.5

**God Object Score Calculation:**

```
weighted_method_count = sum(weight for each function)
complexity_penalty = 0.7 if avg_complexity < 3, 1.0 if 3-10, 1.5 if > 10

god_object_score = (
    (weighted_method_count / threshold) * 40% +
    (field_count / threshold) * 20% +
    (responsibility_count / threshold) * 15% +
    (lines_of_code / 500) * 25%
) * complexity_penalty
```

**Threshold**: God object detected if `score >= 70.0`

**Real-World Example:**

```
shared_cache.rs:
  - 100 functions, average complexity: 1.5
  - Weighted score: ~19.0 (100 * 0.19)
  - God object score: 45.2
  - Result: Not a god object ✓

legacy_parser.rs:
  - 10 functions, average complexity: 17.0
  - Weighted score: ~135.0 (10 * 13.5)
  - God object score: 87.3
  - Result: God object detected ✓
```

**Benefits:**

- **Reduces false positives** on utility modules with many simple functions
- **Focuses attention** on truly problematic complex modules
- **Rewards good refactoring** - breaking large functions into small helpers improves score
- **Aligns with reality** - complexity matters more than count for maintainability

**How to View:**

When Debtmap detects a god object, the output includes:
- Raw method count
- Weighted method count
- Average complexity
- God object score
- Recommended module splits based on responsibility clustering

#### Type Organization Debt

**Source**: Type organization patterns defined in src/priority/mod.rs:273-288, detection logic in src/organization/codebase_type_analyzer.rs

These patterns detect issues with how types and their associated functions are organized across the codebase.

**ScatteredType**: Type with methods scattered across multiple files

```rust
// Type definition in types/user.rs
pub struct User {
    id: UserId,
    name: String,
}

// Methods scattered across files:
// In modules/auth.rs:
impl User {
    fn authenticate(&self) -> Result<Session> { /* ... */ }
}

// In modules/validation.rs:
impl User {
    fn validate_email(&self) -> Result<()> { /* ... */ }
}

// In modules/persistence.rs:
impl User {
    fn save(&self) -> Result<()> { /* ... */ }
}

// Problem: User methods spread across 4+ files!
```

**When detected**: Type has methods in 2+ files (severity: Low=2 files, Medium=3-5 files, High=6+ files)
**Action**: Consolidate methods into primary file or create focused trait implementations
**Source**: Detection criteria from src/organization/codebase_type_analyzer.rs:46-47

**OrphanedFunctions**: Functions that should be methods on a type

```rust
// Bad: Orphaned functions operating on User
fn validate_user_email(user: &User) -> Result<()> {
    // Email validation logic
}

fn calculate_user_age(user: &User) -> u32 {
    // Age calculation from birthdate
}

fn format_user_display(user: &User) -> String {
    // Display formatting
}

// Good: Functions converted to methods
impl User {
    fn validate_email(&self) -> Result<()> { /* ... */ }
    fn age(&self) -> u32 { /* ... */ }
    fn display(&self) -> String { /* ... */ }
}
```

**When detected**: Multiple functions with shared type parameter pattern (e.g., all take `&User`)
**Action**: Convert functions to methods on the target type
**Source**: Detection in src/organization/codebase_type_analyzer.rs:58-71

**UtilitiesSprawl**: Utility module with poor cohesion

```rust
// Bad: utils.rs with mixed responsibilities
mod utils {
    fn parse_date(s: &str) -> Date { /* ... */ }
    fn validate_email(s: &str) -> bool { /* ... */ }
    fn calculate_hash(data: &[u8]) -> Hash { /* ... */ }
    fn format_currency(amount: f64) -> String { /* ... */ }
    fn send_notification(msg: &str) { /* ... */ }
    // ... 20+ more unrelated functions
}

// Good: Focused modules
mod date_utils { fn parse(s: &str) -> Date { /* ... */ } }
mod validators { fn email(s: &str) -> bool { /* ... */ } }
mod crypto { fn hash(data: &[u8]) -> Hash { /* ... */ } }
mod formatters { fn currency(amount: f64) -> String { /* ... */ } }
mod notifications { fn send(msg: &str) { /* ... */ } }
```

**When detected**: Utility module has many functions operating on diverse types with low cohesion
**Action**: Split into focused modules based on domain or responsibility
**Source**: Detection in src/organization/codebase_type_analyzer.rs:74-80

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

**ComplexityHotspot**: Functions exceeding complexity thresholds

**Source**: Categorized as CodeQuality in src/priority/mod.rs:340

```rust
// Cyclomatic: 22, Cognitive: 35
fn process_transaction(tx: Transaction, account: &mut Account) -> Result<Receipt> {
    if tx.amount <= 0 {
        return Err(Error::InvalidAmount);
    }
    if account.balance < tx.amount {
        if account.overdraft_enabled {
            if account.overdraft_limit >= tx.amount {
                // More nested branches...
            }
        } else {
            return Err(Error::InsufficientFunds);
        }
    }
    // ... deeply nested logic with many branches
    Ok(receipt)
}
```

**When detected**: Cyclomatic > 10 OR Cognitive > 15 (configurable)
**Action**: Break into smaller functions, extract validation, simplify control flow
**Note**: Includes entropy-adjusted complexity (adjusted_cyclomatic) when available

**DeadCode**: Unreachable or unused code

**Source**: Categorized as CodeQuality in src/priority/mod.rs:341

```rust
// Private function never called within module
fn obsolete_calculation(x: i32) -> i32 {
    x * 2 + 5  // Dead code - no callers
}

// Public function but no external usage
pub fn deprecated_api(data: &str) -> Result<()> {
    // Unreachable in practice
    Ok(())
}
```

**When detected**: Function visibility analysis + call graph shows no callers
**Action**: Remove dead code or document if intentionally kept for future use

**MagicValues**: Unexplained literal values

**Source**: Categorized as CodeQuality in src/priority/mod.rs:345

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

**When detected**: Numeric or string literals without clear context (excludes 0, 1, common patterns)
**Action**: Extract to named constants or configuration

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

