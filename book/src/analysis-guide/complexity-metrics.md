# Complexity Metrics

Debtmap measures complexity using multiple complementary approaches. Each metric captures a different aspect of code difficulty.

## Cyclomatic Complexity

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

## Cognitive Complexity

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

## Entropy-Based Complexity Analysis

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

## Nesting Depth

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

## Function Length

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

## Constructor Detection

Debtmap identifies constructor functions using AST-based analysis (Spec 122), which goes beyond simple name-based detection to catch non-standard constructor patterns.

**Detection Strategy:**

1. **Return Type Analysis**: Functions returning `Self`, `Result<Self>`, or `Option<Self>`
2. **Body Pattern Analysis**: Struct initialization or simple field assignments
3. **Complexity Check**: Low cyclomatic complexity (≤5), no loops, minimal branching

**Why AST-based detection?**

Name-based detection (looking for `new`, `new_*`, `from_*`) misses non-standard constructors:

```rust
// Caught by name-based detection
fn new() -> Self {
    Self { timeout: 30 }
}

// Missed by name-based, caught by AST detection
pub fn create_default_client() -> Self {
    Self { timeout: Duration::from_secs(30) }
}

pub fn initialized() -> Self {
    Self::new()
}
```

**Builder vs Constructor:**

AST analysis distinguishes between constructors and builder methods:

```rust
// Constructor: creates new instance
pub fn new(timeout: u32) -> Self {
    Self { timeout }
}

// Builder method: modifies existing instance (NOT a constructor)
pub fn set_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self  // Returns modified self, not new instance
}
```

**Detection Criteria:**

A function is classified as a constructor if:
- Returns `Self`, `Result<Self>`, or `Option<Self>`
- Contains struct initialization (`Self { ... }`) without loops
- OR delegates to another constructor (`Self::new()`) with minimal logic

**Fallback Behavior:**

If AST parsing fails (syntax errors, unsupported language), Debtmap gracefully falls back to name-based detection (Spec 117):
- `new`, `new_*`
- `try_new*`
- `from_*`

This ensures analysis always completes, even on partially broken code.

**Performance:**

AST-based detection adds < 5% overhead compared to name-only detection. See benchmarks:

```bash
cargo bench --bench constructor_detection_bench
```

**Why it matters:**

Accurately identifying constructors helps:
- Exclude them from complexity thresholds (constructors naturally have high complexity)
- Focus refactoring on business logic, not initialization code
- Understand initialization patterns across the codebase
