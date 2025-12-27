# Semantic Classification

Debtmap performs semantic analysis to classify functions by their architectural role, enabling more accurate complexity scoring and prioritization.

## Overview

Semantic classification identifies the purpose of each function based on AST patterns, helping debtmap:
- Apply appropriate complexity expectations
- Adjust scoring based on function role
- Provide role-specific recommendations

## Function Roles

Debtmap classifies functions into seven distinct roles, each with specific detection criteria and scoring behavior.

### Pure Logic

Functions that compute without side effects. These are the core business logic functions that deserve highest test priority.

**Detection Criteria** (from `src/priority/semantic_classifier/mod.rs:43`):
- Default classification when no other role matches
- Does not match entry point, debug, constructor, enum converter, accessor, pattern matching, I/O wrapper, or orchestrator patterns

**Example:**
```rust
fn calculate_total(items: &[Item]) -> u32 {
    items.iter().map(|i| i.price).sum()
}
```

### Orchestrator

Functions that coordinate other functions with simple delegation logic.

**Detection Criteria** (from `src/priority/semantic_classifier/classifiers.rs:257-328`):
- Name matches orchestration patterns: `orchestrate`, `coordinate`, `manage`, `dispatch`, `route`, `delegate`, `forward`
- Name prefixes: `workflow_`, `pipeline_`, `process_`, `orchestrate_`, `coordinate_`, `execute_flow_`
- Must have at least 2 meaningful callees (non-stdlib functions)
- Cyclomatic complexity ≤ 5
- Delegation ratio ≥ 20% (function calls / total lines)
- Excludes adapter/wrapper patterns (single delegation)

**Example:**
```rust
fn process_order(order: Order) -> Result<Receipt> {
    let validated = validate_order(&order)?;
    let priced = calculate_prices(&validated)?;
    finalize_order(&priced)
}
```

### I/O Wrapper

Functions that wrap I/O operations. Includes simple constructors, accessors, and enum converters.

**Detection Criteria** (from `src/priority/semantic_classifier/classifiers.rs:331-343`):
- Name contains I/O keywords: `read`, `write`, `file`, `socket`, `http`, `request`, `response`, `stream`, `serialize`, `deserialize`, `save`, `load`, etc.
- Short I/O functions (< 20 lines) are always classified as I/O wrappers
- Longer functions (≤ 50 lines) with strong I/O name patterns (`output_`, `write_`, `print_`, etc.) and low nesting (≤ 3)

**Example:**
```rust
fn read_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)?;
    toml::from_str(&content)
}
```

### Entry Point

Main functions and public API endpoints. These have highest classification precedence.

**Detection Criteria** (from `src/priority/semantic_classifier/pattern_matchers.rs:54-63`):
- Name patterns: `main`, `run`, `start`, `init`, `handle`, `process`, `execute`, `serve`, `listen`
- Functions at the top of the call graph
- Highest classification precedence (checked before all other roles)

**Example:**
```rust
fn main() {
    let args = Args::parse();
    run(args).unwrap();
}
```

### Pattern Match

Functions dominated by pattern matching logic, typically with many branches but low cyclomatic complexity.

**Detection Criteria** (from `src/priority/semantic_classifier/classifiers.rs:213-252`):
- Name suggests pattern matching: `detect`, `classify`, `identify`, `determine`, `resolve`, `match`, `parse_type`, `get_type`, `find_type`
- Low cyclomatic complexity (≤ 2) but higher cognitive complexity
- Cognitive/cyclomatic ratio > 5.0 (indicates many if/else or match branches)

**Example:**
```rust
fn handle_event(event: Event) -> Action {
    match event {
        Event::Click(pos) => Action::Select(pos),
        Event::Drag(from, to) => Action::Move(from, to),
        Event::Release => Action::Confirm,
    }
}
```

### Debug

Functions used for troubleshooting and diagnostics. These have the lowest test priority.

**Detection Criteria** (from `src/priority/semantic_classifier/classifiers.rs:14-53`):
- Name prefixes: `debug_`, `print_`, `dump_`, `trace_`
- Name suffixes: `_diagnostics`, `_debug`, `_stats`
- Name contains: `diagnostics`
- Cognitive complexity ≤ 10 (prevents misclassifying complex functions with debug-like names)
- Alternatively: Very simple functions (cognitive < 5, length < 20) with output-focused I/O patterns (`print`, `display`, `show`, `log`, `trace`, `dump`)

**Example:**
```rust
fn print_call_graph_diagnostics(graph: &CallGraph) {
    for node in graph.nodes() {
        println!("{}: {} callers, {} callees",
            node.name, node.callers.len(), node.callees.len());
    }
}
```

### Unknown

Functions that cannot be classified into any specific role. These receive neutral scoring adjustments.

**Detection Criteria** (from `src/priority/semantic_classifier/mod.rs:32`):
- Reserved for edge cases where classification fails
- In practice, functions default to `PureLogic` when no other role matches

## Classification Precedence

The classifier applies rules in a specific order to ensure correct classification when multiple patterns match (from `src/priority/semantic_classifier/mod.rs:47-113`):

1. **Entry Point** - Highest precedence, checked first
2. **Debug** - Diagnostic functions detected early
3. **Constructor** - Simple constructors (AST-based detection, spec 117/122)
4. **Enum Converter** - Simple enum-to-value converters (spec 124)
5. **Accessor** - Simple getter/accessor methods (spec 125)
6. **Data Flow** - Data transformation orchestrators (spec 126, opt-in)
7. **Pattern Match** - Pattern matching functions
8. **I/O Wrapper** - I/O-focused functions
9. **Orchestrator** - Coordination functions
10. **Pure Logic** - Default fallback

## AST-Based Detection

Semantic classification uses AST analysis to detect function roles beyond simple name matching.

### Constructor Detection (Spec 117/122)

**Source:** `src/priority/semantic_classifier/classifiers.rs:115-183`

Detects constructor functions even with non-standard names by analyzing:
- Return type: Must return `Self`, `Result<Self>`, or `Option<Self>`
- Body patterns: Contains struct initialization with `Self { ... }`
- No loops in function body
- Complexity thresholds: cyclomatic ≤ 5, nesting ≤ 2, length < 30

**Detected Patterns:**
- Standard names: `new`, `default`, `from_*`, `with_*`, `create_*`, `make_*`, `build_*`
- Non-standard names with AST analysis: `create_default_client()` returning `Self`

**Example:**
```rust
// Detected even without standard naming
pub fn create_default_client() -> Self {
    Self {
        timeout: Duration::from_secs(30),
        retries: 3,
    }
}
```

### Enum Converter Detection (Spec 124)

**Source:** `src/priority/semantic_classifier/classifiers.rs:185-211`

Detects simple enum-to-string converter functions:
- Name patterns: `name`, `as_str`, `to_*`
- Body contains exhaustive `match` statement on `self`
- All match arms return string/numeric literals only
- No function calls in match arms (e.g., no `format!()`)
- Cognitive complexity ≤ 3

**Example:**
```rust
pub fn name(&self) -> &'static str {
    match self {
        FrameworkType::Django => "Django",
        FrameworkType::Flask => "Flask",
        FrameworkType::PyQt => "PyQt",
    }
}
```

### Accessor Method Detection (Spec 125)

**Source:** `src/priority/semantic_classifier/mod.rs:121-177`

Detects simple accessor and getter methods:

**Single-word patterns:**
- `id`, `name`, `value`, `kind`, `type`, `status`, `code`, `key`, `index`

**Prefix patterns:**
- `get_*`, `is_*`, `has_*`, `can_*`, `should_*`, `as_*`, `to_*`, `into_*`

**Complexity thresholds:**
- Cyclomatic complexity ≤ 2
- Cognitive complexity ≤ 1
- Length < 10 lines
- Nesting ≤ 1 level
- If AST available, verifies body is simple accessor pattern

**Example:**
```rust
pub fn id(&self) -> u32 {
    self.id
}
```

### Data Flow Classification (Spec 126)

**Source:** `src/priority/semantic_classifier/mod.rs:81-96`

Analyzes data flow patterns to identify orchestration functions based on transformation chains.

**Detection Criteria:**
- Enabled via configuration (opt-in by default)
- High confidence (≥ 0.8)
- High transformation ratio (≥ 0.7)
- Low business logic ratio (< 0.3)

### Debug Function Detection (Spec 119)

**Source:** `src/priority/semantic_classifier/pattern_matchers.rs:7-20`

Detects debug/diagnostic functions using:

**Name patterns:**
- Prefixes: `debug_`, `print_`, `dump_`, `trace_`
- Suffixes: `_diagnostics`, `_debug`, `_stats`
- Contains: `diagnostics`

**Behavioral characteristics:**
- Low cognitive complexity (< 5)
- Short length (< 20 lines)
- Output-focused I/O patterns: `print`, `display`, `show`, `log`, `trace`, `dump`

## Role-Specific Expectations

Different roles have different coverage and complexity expectations:

| Role | Coverage Expectation | Complexity Tolerance |
|------|---------------------|---------------------|
| Pure Logic | High | Low |
| Orchestrator | Medium | Medium |
| I/O Wrapper | Low | Low |
| Entry Point | Low | Medium |
| Pattern Match | Medium | Variable |
| Debug | Low | Low |
| Unknown | Medium | Medium |

## Scoring Adjustments

Semantic classification affects scoring through role multipliers. These values adjust the priority score for each function role (from `src/config/scoring.rs:307-333`):

```toml
[scoring.role_multipliers]
pure_logic = 1.2       # Prioritized (highest test priority)
orchestrator = 0.8     # Reduced priority
io_wrapper = 0.7       # Minor reduction
entry_point = 0.9      # Slight reduction
pattern_match = 0.6    # Moderate reduction
debug = 0.3           # Lowest test priority
unknown = 1.0         # No adjustment
```

**Scoring Formula:**
- Higher multipliers (> 1.0) increase function priority
- Lower multipliers (< 1.0) decrease function priority
- `pure_logic = 1.2` means pure logic functions are prioritized 20% higher
- `debug = 0.3` means debug functions are de-prioritized significantly

## Configuration

### Semantic Classification

```toml
[semantic]
enabled = true
role_detection = true
adjust_coverage_expectations = true
```

### Constructor Detection (Spec 117/122)

From `src/config/detection.rs:54-98`:

```toml
[classification.constructors]
# Name patterns for constructor functions
patterns = ["new", "default", "from_", "with_", "create_", "make_", "build_", "of_", "empty", "zero", "any"]

# Complexity thresholds
max_cyclomatic = 2       # Maximum cyclomatic complexity
max_cognitive = 3        # Maximum cognitive complexity
max_length = 15          # Maximum function length
max_nesting = 1          # Maximum nesting depth

# Enable AST-based detection for non-standard constructor names
ast_detection = true     # Analyzes return types and body patterns
```

### Accessor Detection (Spec 125)

From `src/config/detection.rs:135-226`:

```toml
[classification.accessors]
enabled = true

# Single-word accessor names
single_word_patterns = ["id", "name", "value", "kind", "type", "status", "code", "key", "index"]

# Prefix patterns for accessors
prefix_patterns = ["get_", "is_", "has_", "can_", "should_", "as_", "to_", "into_"]

# Complexity thresholds
max_cyclomatic = 2       # Maximum cyclomatic complexity
max_cognitive = 1        # Maximum cognitive complexity (stricter than constructors)
max_length = 10          # Maximum function length
max_nesting = 1          # Maximum nesting depth
```

### Data Flow Classification (Spec 126)

From `src/config/detection.rs:228-273`:

```toml
[classification.data_flow]
enabled = false          # Opt-in feature
min_confidence = 0.8     # Minimum confidence required
min_transformation_ratio = 0.7  # Minimum transformation ratio for orchestrator
max_business_logic_ratio = 0.3  # Maximum business logic for orchestrator
```

### Debug Function Detection

Debug function detection is controlled by name patterns in `src/priority/semantic_classifier/pattern_matchers.rs:7-20`. The detection thresholds are:
- Cognitive complexity ≤ 10 for name-matched functions
- Cognitive complexity < 5 and length < 20 for behavior-matched functions

## Troubleshooting

### Function Classified Incorrectly

If a function is classified with the wrong role:

1. **Check classification precedence** - Entry points take highest precedence
2. **Review complexity thresholds** - High complexity can disqualify certain roles
3. **Examine name patterns** - Some roles require specific naming conventions
4. **Enable AST detection** - Set `classification.constructors.ast_detection = true` for better constructor detection

### Constructor Not Detected

If a simple constructor is classified as PureLogic:

1. Ensure function name matches patterns or returns `Self`
2. Check complexity thresholds: cyclomatic ≤ 2, cognitive ≤ 3, length < 15
3. Enable AST detection for non-standard names
4. Verify no loops in function body

### Debug Function Not Detected

If a diagnostic function has high priority:

1. Ensure name matches debug patterns (`debug_*`, `print_*`, `*_diagnostics`, etc.)
2. Check cognitive complexity is ≤ 10
3. Functions with high complexity are intentionally excluded to prevent misclassification

## See Also

- [Role-Based Adjustments](scoring-strategies/role-based.md)
- [Functional Composition Analysis](functional-analysis.md)
