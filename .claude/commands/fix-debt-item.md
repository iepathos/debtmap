---
name: fix-debt-item
description: Fix a specific tech debt item identified by debtmap
---

# Fix Specific Tech Debt Item

Fix a single tech debt item with the provided parameters from debtmap analysis.

## Parameters

The workflow passes a single JSON object containing all debt item information:
- `--json`: Complete JSON object for the debt item

The JSON object contains fields such as:
- `location`: Object with `file`, `function`, `line` fields
- `unified_score`: Object with `final_score`, `coverage_factor`, `complexity_factor`, etc.
- `cyclomatic_complexity`: Cyclomatic complexity metric
- `cognitive_complexity`: Cognitive complexity metric  
- `nesting_depth`: Maximum nesting depth
- `function_length`: Number of lines in the function
- `function_role`: Role classification (e.g., PureLogic, IOWrapper, Orchestration)
- `recommendation`: Object with `primary_action` and other suggestions
- `upstream_dependencies`: Count of upstream dependencies
- `downstream_dependencies`: Count of downstream dependencies
- `expected_impact`: Object with `risk_reduction`, `complexity_reduction`
- `entropy_details`: Object with `entropy_score`, `pattern_repetition`, `adjusted_complexity`
- And any other fields debtmap provides

## Process

### Step 0: Parse the JSON Object

First, parse the provided JSON to extract the necessary fields. The JSON object passed via `--json` contains all debt metrics and recommendations.

Extract key values from the JSON:
- File path: `item.location.file`
- Function name: `item.location.function`
- Line number: `item.location.line`
- Score: `item.unified_score.final_score`
- Complexity metrics: `item.cyclomatic_complexity`, `item.cognitive_complexity`
- Recommended action: `item.recommendation.primary_action`
- And all other relevant fields as needed

### Step 1: Interpret the Recommended Action

The `recommendation.primary_action` field provides debtmap's specific recommendation. Common patterns:

- **"Extract N pure functions (complexity X → Y), then add Z tests"**
  → Priority is refactoring for complexity reduction, followed by testing
  
- **"Add X unit tests covering Y scenarios"**
  → Focus on test coverage without refactoring
  
- **"Refactor to reduce complexity from X to <10"**
  → Focus on complexity reduction using functional patterns
  
- **"Extract business logic from I/O operations"**
  → Separate pure logic from side effects

Use this action as your primary guide, but validate it makes sense given the code.

### Step 1: Analyze the Specific Issue

Read the file and locate the exact function using the location information from the JSON object.

Use the provided metrics from the JSON to understand the issue:
- **Score**: `item.unified_score.final_score` (Priority level)
- **Complexity**: Cyclomatic=`item.cyclomatic_complexity`, Cognitive=`item.cognitive_complexity`, Adjusted=`item.entropy_details.adjusted_complexity`
- **Entropy**: Score=`item.entropy_details.entropy_score`, Repetition=`item.entropy_details.pattern_repetition` (high repetition suggests extractable patterns)
- **Structure**: Nesting depth=`item.nesting_depth`, Length=`item.function_length` lines
- **Role**: `item.function_role` (determines refactoring approach)
- **Dependencies**: `item.upstream_dependencies` upstream, `item.downstream_dependencies` downstream
- **Recommended Action**: `item.recommendation.primary_action`
- **Expected Impact**: Risk reduction=`item.expected_impact.risk_reduction`, Complexity reduction=`item.expected_impact.complexity_reduction`

Priority levels:
- **CRITICAL (Score 10.0)**: Functions with high complexity and zero coverage
- **HIGH (Score 7-9)**: Important business logic with test gaps  
- **MEDIUM (Score 4-6)**: Moderate complexity or coverage issues
- **LOW (Score 1-3)**: Minor improvements

### Step 2: Evaluate Refactoring Approach

**First, check entropy metrics to understand pattern complexity:**

- **High repetition (>60%)**: Code has repetitive patterns - good candidate for extraction
- **Low entropy (<0.4)**: Simple, predictable patterns - might not need refactoring
- **Adjusted complexity < Original**: Entropy analysis suggests true complexity is lower
- **High entropy (>0.7)**: Diverse patterns - may be legitimately complex

**Then consider the function role (`item.function_role`):**

- **PureLogic**: Focus on breaking down complex logic into smaller pure functions
- **IOWrapper**: Extract business logic from I/O operations  
- **Orchestration**: Keep orchestration thin, extract any complex logic
- **Visitor/Parser**: Often legitimately complex, focus on tests instead
- **Unknown**: Analyze the code to determine the actual role

**Then use this decision tree:**

```
Is it a visitor pattern or large switch/match?
├─ YES → Don't refactor, add tests if needed
└─ NO → Continue
   │
   Cyclomatic > 10 AND Score >= 7?
   ├─ NO → Focus on adding tests only
   └─ YES → Continue
      │
      Does it classify/categorize inputs?
      ├─ YES → Extract as pure static function
      └─ NO → Continue
         │
         Does it have repeated similar conditions?
         ├─ YES → Consolidate with pattern matching
         └─ NO → Continue
            │
            Does it have nested loops?
            ├─ YES → Convert to iterator chains
            └─ NO → Consider if refactoring is needed
```

**WARNING: Avoid these anti-patterns:**
- Creating multiple single-use helper methods that are only called from tests
- Extracting helpers that aren't used in production code paths
- Over-engineering: 5+ helper methods for a 15-line function
- Breaking apart clear match expressions
- Adding complexity to reduce metrics

### Step 3: Apply Functional Programming Patterns

Based on the complexity source, apply the appropriate pattern:

#### Pattern 1: Extract Pure Classification Functions (PREFERRED)
For functions with classification/decision logic:

```rust
// Before: Complexity 15
if name.contains("async") || name.contains("await") { 
    CallType::Async 
} else if name.starts_with("handle_") { 
    CallType::Delegate 
} else if name.starts_with("map") { 
    CallType::Pipeline 
} else { 
    CallType::Direct 
}

// After: Extract as pure static function
fn classify_call_type(name: &str) -> CallType {
    match () {
        _ if name.contains("async") || name.contains("await") => CallType::Async,
        _ if name.starts_with("handle_") => CallType::Delegate,
        _ if name.starts_with("map") => CallType::Pipeline,
        _ => CallType::Direct,
    }
}
```

#### Pattern 2: Pattern Consolidation
Use match expressions with guards instead of if-else chains:

```rust
// Combine similar branches using pattern matching
match () {
    _ if condition_a || condition_b => handle_similar_cases(),
    _ if condition_c => handle_special_case(),
    _ => handle_default(),
}
```

#### Pattern 3: Functional Composition
Replace imperative loops with iterator chains:

```rust
// Before: Imperative loop
let mut results = Vec::new();
for item in items {
    if item.is_valid() {
        results.push(item.transform());
    }
}

// After: Functional chain
let results: Vec<_> = items
    .iter()
    .filter(|item| item.is_valid())
    .map(|item| item.transform())
    .collect();
```

#### Pattern 4: Extract Pure Business Logic from I/O
For orchestration/I/O functions:

```rust
// Before: Mixed I/O and logic
fn process_file(path: &Path) {
    let content = fs::read_to_string(path).unwrap();
    // Complex parsing logic here
    println!("{}", result);
}

// After: Pure function + thin I/O wrapper
fn parse_content(content: &str) -> Result<ParsedData, Error> {
    // Pure parsing logic
}

fn process_file(path: &Path) -> Result<(), Error> {
    let content = fs::read_to_string(path)?;
    let result = parse_content(&content)?;
    println!("{}", result);
    Ok(())
}
```

**Note**: For orchestration/I/O functions with low complexity:
- Extract formatting logic to pure functions that return strings
- Extract parsing/validation to separate modules
- Create pure functions for decision logic
- Keep thin I/O wrappers untested (they're not the real debt)

### Step 4: Add Comprehensive Tests

**For Testing Priority:**

Based on the metrics from the JSON object:
- Score >= 7 AND Cyclomatic > 10 → Apply functional refactoring first (see Step 3)
- Coverage-factor > 5 → Focus on comprehensive test coverage
- Otherwise → Add targeted tests for uncovered branches

**For Business Logic Functions:**
Write test cases covering:
- Happy path scenarios
- Edge cases and boundary conditions  
- Error conditions and invalid inputs
- All branches of pattern matching
- Any uncovered paths

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification_logic() {
        // Test extracted pure functions
        assert_eq!(classify_type("test_foo"), Type::Test);
        assert_eq!(classify_type("bar_impl"), Type::Implementation);
    }

    #[test]
    fn test_edge_cases() {
        // Test boundary conditions
    }

    #[test]
    fn test_error_conditions() {
        // Test error handling
    }
}
```

**For Orchestration/I/O Functions:**
- DON'T test the I/O wrapper directly
- DO test any extracted pure functions
- Focus tests on business logic, not I/O operations

### Step 5: Verify the Fix

Run tests to ensure the fix works:
```bash
just ci
```

All tests must pass, no clippy warnings allowed, and code must be properly formatted.

### Step 6: Commit the Changes

**REQUIRED: Create a commit with your fix.**

Create a descriptive commit message that includes:
- What was changed (refactoring applied or tests added)
- The specific function and file that was fixed
- The metrics that guided the fix

**For refactoring commits:**
```bash
git add -A
git commit -m "refactor: reduce complexity in [function_name]

- Applied: [action from item.recommendation.primary_action]
- Complexity: [item.cyclomatic_complexity] → [new_complexity] (adjusted: [item.entropy_details.adjusted_complexity])
- Entropy: [item.entropy_details.entropy_score], Repetition: [item.entropy_details.pattern_repetition]
- Function: [item.location.function] in [item.location.file]
- Risk reduction: [item.expected_impact.risk_reduction]
"
```

**For test addition commits:**
```bash
git add -A
git commit -m "test: add coverage for [function_name]

- Added [N] test cases for [item.location.function]
- Coverage factor: [item.unified_score.coverage_factor]
- Function: [item.location.function] in [item.location.file]
- Expected risk reduction: [item.expected_impact.risk_reduction]
"
```

**For combined refactoring + tests:**
```bash
git add -A
git commit -m "fix: refactor and test [function_name]

- Applied: [item.recommendation.primary_action]
- Complexity: [item.cyclomatic_complexity] → [new_complexity]
- Added [N] comprehensive tests
- Function: [item.location.function] in [item.location.file]
- Risk reduction: [item.expected_impact.risk_reduction]
"
```

**IMPORTANT**: 
- Always create a commit after completing the fix
- Include the actual metrics in the commit message
- Reference the specific function and file
- Each agent in the MapReduce workflow creates its own commit

## Implementation Guidelines

### Functional Programming Principles

**Always Prefer:**
- **Pure functions** over stateful methods
- **Immutability** - use `&self` instead of `&mut self` where possible
- **Function composition** - build complex behavior from simple functions
- **Pattern matching** over if-else chains
- **Iterator chains** over imperative loops
- **Type-driven design** - use the type system to enforce invariants

### Common Pitfalls to Avoid

❌ **DON'T:**
- Extract helper methods that are only called once
- Create test-only helper functions (helpers not used in production code)
- Break apart a clear match/switch into multiple functions
- Add abstraction layers for simple logic
- Refactor visitor pattern implementations (they're meant to have many branches)
- Create 5+ helper methods for a 15-line function
- Test I/O directly instead of extracting logic

✅ **DO:**
- Extract reusable classification/decision logic as static pure functions
- Use functional patterns (map, filter, fold) where appropriate
- Consolidate similar patterns into single functions
- Keep related logic together
- Accept that some functions legitimately have high complexity
- Test the extracted pure functions thoroughly

### Idiomatic Rust Patterns

**Use these Rust idioms:**
```rust
// Use ? for error propagation
let data = read_file()?;

// Prefer &str over String in parameters
fn process(input: &str) -> Result<String, Error>

// Use #[derive] for common traits
#[derive(Debug, Clone, PartialEq)]
struct Data { ... }

// Use From/Into for type conversions
impl From<String> for MyType { ... }

// Prefer iterators over indexing
items.iter().map(|x| x.process())

// Use Option/Result combinators
value.map(|v| v.transform()).unwrap_or_default()
```

### Example of Good Refactoring

```rust
// Extract classification logic as pure static function
impl MyStruct {
    // This can be tested in isolation and reused
    fn classify_item(name: &str) -> ItemType {
        match () {
            _ if name.starts_with("test_") => ItemType::Test,
            _ if name.contains("_impl") => ItemType::Implementation,
            _ => ItemType::Regular,
        }
    }
    
    // Main function uses the pure classifier
    fn process(&mut self, name: &str) {
        let item_type = Self::classify_item(name);
        // ... rest of logic
    }
}
```

## Success Criteria

The fix is complete when:
- [ ] The specific function has been refactored or tested
- [ ] All tests pass (`just ci`)
- [ ] Code follows functional programming patterns
- [ ] Implementation uses idiomatic Rust patterns
- [ ] Changes are minimal and focused
- [ ] Backward compatibility is maintained
- [ ] **Changes are committed with descriptive message including metrics**

## Notes

- Focus only on the specific function identified by the parameters
- For Score >= 7 with Complexity > 10: Apply functional refactoring
- For other cases: Add comprehensive tests
- Extract pure functions that can be tested in isolation
- Accept that some functions (visitor patterns, large matches) have legitimate complexity
- **A commit is REQUIRED at the end of each fix** (Step 6)
- This command is designed for MapReduce workflow integration
- Each parallel agent creates its own commit; these are later aggregated in the reduce phase