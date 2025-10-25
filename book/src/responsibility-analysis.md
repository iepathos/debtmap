# Responsibility Analysis

## Overview

Responsibility analysis is a core feature of Debtmap that helps identify violations of the **Single Responsibility Principle (SRP)**, one of the fundamental SOLID design principles. By analyzing function and method names, Debtmap automatically infers the distinct functional responsibilities within a code unit and detects when a single module, struct, or class has taken on too many concerns.

This chapter provides an in-depth look at how Debtmap determines responsibilities, categorizes them, and uses this information to guide refactoring decisions.

## What Are Responsibilities?

In the context of Debtmap, a **responsibility** is a distinct functional domain or concern that a code unit handles. Examples include:

- **Data Access** - Getting and setting values from data structures
- **Validation** - Checking inputs, verifying constraints, ensuring correctness
- **Persistence** - Saving and loading data to/from storage
- **Computation** - Performing calculations and transformations
- **Communication** - Sending and receiving messages or events

According to the Single Responsibility Principle, each module should have **one and only one reason to change**. When a module handles multiple unrelated responsibilities (e.g., validation, persistence, AND computation), it becomes:

- **Harder to understand** - Developers must mentally juggle multiple concerns
- **More fragile** - Changes to one responsibility can break others
- **Difficult to test** - Testing requires complex setup across multiple domains
- **Prone to coupling** - Dependencies from different domains become entangled

Debtmap's responsibility analysis automatically identifies these violations and provides concrete recommendations for splitting modules along responsibility boundaries.

## How Responsibilities Are Detected

### Pattern-Based Inference

Debtmap uses **prefix-based pattern matching** to infer responsibilities from function and method names. This approach is both simple and effective because well-named functions naturally express their intent through conventional prefixes.

**Implementation Location:** `src/organization/god_object_analysis.rs:316-386`

The `infer_responsibility_from_method()` function performs case-insensitive prefix matching:

```rust
pub fn infer_responsibility_from_method(method_name: &str) -> String {
    let lower_name = method_name.to_lowercase();

    if lower_name.starts_with("format_") || lower_name.starts_with("render_") {
        return "Formatting & Output".to_string();
    }
    if lower_name.starts_with("parse_") || lower_name.starts_with("read_") {
        return "Parsing & Input".to_string();
    }
    // ... additional patterns
}
```

This approach works across languages (Rust, Python, JavaScript/TypeScript) because naming conventions are relatively consistent in modern codebases.

### Responsibility Categories

Debtmap recognizes **11 built-in responsibility categories** plus a generic "Utilities" fallback:

| Category | Prefixes | Examples |
|----------|----------|----------|
| **Formatting & Output** | `format_`, `render_`, `write_`, `print_` | `format_json()`, `render_table()`, `write_report()` |
| **Parsing & Input** | `parse_`, `read_`, `extract_` | `parse_config()`, `read_file()`, `extract_fields()` |
| **Filtering & Selection** | `filter_`, `select_`, `find_` | `filter_results()`, `select_top()`, `find_item()` |
| **Transformation** | `transform_`, `convert_`, `map_`, `apply_` | `transform_data()`, `convert_type()`, `map_fields()` |
| **Data Access** | `get_`, `set_` | `get_value()`, `set_name()` |
| **Validation** | `validate_`, `check_`, `verify_`, `is_*` | `validate_input()`, `check_bounds()`, `is_valid()` |
| **Computation** | `calculate_`, `compute_` | `calculate_score()`, `compute_sum()` |
| **Construction** | `create_`, `build_`, `new_*`, `make_` | `create_instance()`, `build_config()`, `new_user()` |
| **Persistence** | `save_`, `load_`, `store_` | `save_data()`, `load_cache()`, `store_result()` |
| **Processing** | `process_`, `handle_` | `process_request()`, `handle_error()` |
| **Communication** | `send_`, `receive_` | `send_message()`, `receive_data()` |
| **Utilities** | *(all others)* | `helper()`, `do_work()`, `utility_fn()` |

### Grouping Methods by Responsibility

Once individual methods are categorized, Debtmap groups them using `group_methods_by_responsibility()`:

**Implementation Location:** `src/organization/god_object_analysis.rs:268-280`

```rust
pub fn group_methods_by_responsibility(methods: &[String]) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for method in methods {
        let responsibility = infer_responsibility_from_method(method);
        groups.entry(responsibility).or_default().push(method.clone());
    }
    groups
}
```

**Output Structure:**
- **Keys**: Responsibility category names (e.g., "Data Access", "Validation")
- **Values**: Lists of method names belonging to each category

The **responsibility count** is simply the number of unique keys in this HashMap.

### Example Analysis

Consider a Rust struct with these methods:

```rust
impl UserManager {
    fn get_user(&self, id: UserId) -> Option<User> { }
    fn set_password(&mut self, id: UserId, password: &str) { }
    fn validate_email(&self, email: &str) -> bool { }
    fn validate_password(&self, password: &str) -> bool { }
    fn save_user(&self, user: &User) -> Result<()> { }
    fn load_user(&self, id: UserId) -> Result<User> { }
    fn send_notification(&self, user_id: UserId, msg: &str) { }
    fn format_user_profile(&self, user: &User) -> String { }
}
```

**Debtmap's Analysis:**

| Method | Inferred Responsibility |
|--------|------------------------|
| `get_user` | Data Access |
| `set_password` | Data Access |
| `validate_email` | Validation |
| `validate_password` | Validation |
| `save_user` | Persistence |
| `load_user` | Persistence |
| `send_notification` | Communication |
| `format_user_profile` | Formatting & Output |

**Result:**
- **Responsibility Count**: 5 (Data Access, Validation, Persistence, Communication, Formatting)
- **Assessment**: This violates SRP - `UserManager` has too many distinct concerns

## Responsibility Scoring

### Integration with God Object Detection

Responsibility count is a critical factor in [God Object Detection](./god-object-detection.md). The scoring algorithm includes:

```
responsibility_factor = min(responsibility_count / 3.0, 3.0)
god_object_score = method_factor × field_factor × responsibility_factor × size_factor
```

**Why divide by 3.0?**
- **1-3 responsibilities**: Normal, well-scoped module
- **4-6 responsibilities**: Warning signs, approaching problematic territory
- **7+ responsibilities**: Severe violation, likely a god object

### Language-Specific Thresholds

Different languages have different expectations for responsibility counts:

| Language | Max Responsibilities | Rationale |
|----------|---------------------|-----------|
| **Rust** | 5 | Strong module system encourages tight boundaries |
| **Python** | 3 | Duck typing makes mixing concerns more dangerous |
| **JavaScript/TypeScript** | 3 | Prototype-based, benefits from focused classes |

These thresholds can be customized in `.debtmap.toml`:

```toml
[god_object_detection.rust]
max_traits = 5      # max_traits = max responsibilities

[god_object_detection.python]
max_traits = 3
```

### Confidence Determination

Responsibility count contributes to overall confidence levels:

**Implementation Location:** `src/organization/god_object_analysis.rs:234-266`

```rust
pub fn determine_confidence(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_sum: u32,
    thresholds: &GodObjectThresholds,
) -> GodObjectConfidence {
    let mut violations = 0;

    if responsibility_count > thresholds.max_traits {
        violations += 1;
    }
    // ... check other metrics

    match violations {
        5 => GodObjectConfidence::Definite,
        3..=4 => GodObjectConfidence::Probable,
        1..=2 => GodObjectConfidence::Possible,
        _ => GodObjectConfidence::NotGodObject,
    }
}
```

## Advanced Responsibility Detection

### Module-Level Analysis

For large modules without a single dominant struct, Debtmap performs **module-level responsibility detection**:

**Implementation Location:** `src/organization/god_object_detector.rs:682-697`

The `classify_responsibility()` function provides extended categorization:

```rust
fn classify_responsibility(prefix: &str) -> String {
    match prefix {
        "get" | "set" => "Data Access",
        "calculate" | "compute" => "Computation",
        "validate" | "check" | "verify" | "ensure" => "Validation",
        "save" | "load" | "store" | "retrieve" | "fetch" => "Persistence",
        "create" | "build" | "new" | "make" | "init" => "Construction",
        "send" | "receive" | "handle" | "manage" => "Communication",
        "update" | "modify" | "change" | "edit" => "Modification",
        "delete" | "remove" | "clear" | "reset" => "Deletion",
        "is" | "has" | "can" | "should" | "will" => "State Query",
        "process" | "transform" => "Processing",
        _ => format!("{} Operations", capitalize_first(prefix)),
    }
}
```

This extended mapping covers 10 core categories plus dynamic fallback for custom prefixes.

### Responsibility Groups

The `ResponsibilityGroup` data structure tracks detailed information about each responsibility:

**Implementation Location:** `src/organization/mod.rs:156-161`

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ResponsibilityGroup {
    pub name: String,           // e.g., "DataAccessManager"
    pub methods: Vec<String>,   // Methods in this group
    pub fields: Vec<String>,    // Associated fields
    pub responsibility: String, // e.g., "Data Access"
}
```

This structure enables:
- **Refactoring recommendations** - Suggest splitting by responsibility group
- **Cohesion analysis** - Measure how tightly methods are related
- **Field-method correlation** - Identify which fields belong to which responsibilities

## Refactoring Based on Responsibilities

### Recommended Module Splits

When Debtmap detects a module with multiple responsibilities, it generates actionable refactoring recommendations using `recommend_module_splits()`:

**Implementation Location:** `src/organization/god_object_detector.rs:165-177`

**Process:**
1. Group all methods by their inferred responsibilities
2. Create a `ModuleSplit` for each responsibility group
3. Suggest module names (e.g., "DataAccessManager", "ValidationManager")
4. Estimate lines of code for each new module
5. Order by cohesion (most focused groups first)

**Example Output:**

```
Recommended Splits for UserManager:
  1. DataAccessManager (5 methods, ~80 lines)
     - get_user, set_password, get_email, set_email, update_profile

  2. ValidationManager (4 methods, ~60 lines)
     - validate_email, validate_password, check_permissions, verify_token

  3. PersistenceManager (3 methods, ~50 lines)
     - save_user, load_user, delete_user

  4. NotificationManager (2 methods, ~30 lines)
     - send_notification, send_bulk_notifications
```

### Practical Refactoring Patterns

#### Pattern 1: Extract Service Classes

**Before (God Object):**
```rust
struct UserManager {
    db: Database,
    cache: Cache,
    notifier: Notifier,
}

impl UserManager {
    fn get_user(&self, id: UserId) -> Option<User> { }
    fn validate_email(&self, email: &str) -> bool { }
    fn save_user(&self, user: &User) -> Result<()> { }
    fn send_notification(&self, id: UserId, msg: &str) { }
}
```

**After (Split by Responsibility):**
```rust
// Data Access
struct UserRepository {
    db: Database,
    cache: Cache,
}

// Validation
struct UserValidator;

// Persistence
struct UserPersistence {
    db: Database,
}

// Communication
struct NotificationService {
    notifier: Notifier,
}
```

#### Pattern 2: Use Facade for Composition

After splitting, create a facade to coordinate:

```rust
struct UserFacade {
    repository: UserRepository,
    validator: UserValidator,
    persistence: UserPersistence,
    notifier: NotificationService,
}

impl UserFacade {
    fn register_user(&mut self, user: User) -> Result<()> {
        self.validator.validate_email(&user.email)?;
        self.persistence.save_user(&user)?;
        self.notifier.send_welcome(&user.id)?;
        Ok(())
    }
}
```

#### Pattern 3: Trait-Based Separation (Rust)

Use traits to define responsibility boundaries:

```rust
trait DataAccess {
    fn get_user(&self, id: UserId) -> Option<User>;
}

trait Validation {
    fn validate_email(&self, email: &str) -> bool;
}

trait Persistence {
    fn save_user(&self, user: &User) -> Result<()>;
}

// Implement only the needed traits per struct
impl DataAccess for UserRepository { }
impl Validation for UserValidator { }
impl Persistence for UserPersistence { }
```

## Data Structures

### GodObjectAnalysis

The main result structure includes responsibility information:

**Implementation Location:** `src/organization/god_object_analysis.rs:5-18`

```rust
pub struct GodObjectAnalysis {
    pub is_god_object: bool,
    pub method_count: usize,
    pub field_count: usize,
    pub responsibility_count: usize,      // Number of distinct responsibilities
    pub lines_of_code: usize,
    pub complexity_sum: u32,
    pub god_object_score: f64,
    pub recommended_splits: Vec<ModuleSplit>,
    pub confidence: GodObjectConfidence,
    pub responsibilities: Vec<String>,    // List of responsibility names
    pub purity_distribution: Option<PurityDistribution>,
}
```

### ModuleSplit

Recommendations for splitting modules:

**Implementation Location:** `src/organization/god_object_analysis.rs:40-45`

```rust
pub struct ModuleSplit {
    pub suggested_name: String,         // e.g., "ValidationManager"
    pub methods_to_move: Vec<String>,   // Methods for this module
    pub responsibility: String,          // Responsibility category
    pub estimated_lines: usize,         // Approximate LOC
}
```

## Testing Responsibility Detection

Debtmap includes comprehensive tests for responsibility detection:

**Implementation Location:** `src/organization/god_object_analysis.rs:623-838`

### Test Coverage

**Key test cases:**
- **Prefix recognition** - Each of the 11 categories is tested individually
- **Case insensitivity** - `Format_Output` and `format_output` both map to "Formatting & Output"
- **Multiple responsibilities** - Grouping diverse methods correctly
- **Empty input handling** - Graceful handling of empty method lists
- **Edge cases** - Methods without recognized prefixes default to "Utilities"

**Example Test:**

```rust
#[test]
fn test_multiple_responsibility_groups() {
    let methods = vec![
        "format_output".to_string(),
        "parse_input".to_string(),
        "get_value".to_string(),
        "validate_data".to_string(),
    ];

    let groups = group_methods_by_responsibility(&methods);

    assert_eq!(groups.len(), 4); // 4 distinct responsibilities
    assert!(groups.contains_key("Formatting & Output"));
    assert!(groups.contains_key("Parsing & Input"));
    assert!(groups.contains_key("Data Access"));
    assert!(groups.contains_key("Validation"));
}
```

## Configuration

### TOML Configuration

Customize responsibility thresholds in `.debtmap.toml`:

```toml
[god_object_detection]
enabled = true

[god_object_detection.rust]
max_traits = 5      # Max responsibilities for Rust

[god_object_detection.python]
max_traits = 3      # Max responsibilities for Python

[god_object_detection.javascript]
max_traits = 3      # Max responsibilities for JavaScript/TypeScript
```

### Tuning Guidelines

**Strict SRP Enforcement:**
```toml
[god_object_detection.rust]
max_traits = 3
```
- Enforces very tight single responsibility
- Suitable for greenfield projects or strict refactoring efforts

**Balanced Approach (Default):**
```toml
[god_object_detection.rust]
max_traits = 5
```
- Allows some flexibility while catching major violations
- Works well for most projects

**Lenient Mode:**
```toml
[god_object_detection.rust]
max_traits = 7
```
- Only flags severe SRP violations
- Useful for large legacy codebases during initial assessment

## Output and Reporting

### Console Output

When analyzing a file with multiple responsibilities:

```
src/services/user_manager.rs
  ⚠️ God Object: 18 methods, 8 fields, 5 responsibilities
     Score: 185 (Confidence: Probable)

     Responsibilities:
       - Data Access (5 methods)
       - Validation (4 methods)
       - Persistence (3 methods)
       - Communication (3 methods)
       - Formatting & Output (3 methods)

     Recommended Splits:
       1. DataAccessManager (5 methods, ~75 lines)
       2. ValidationManager (4 methods, ~60 lines)
       3. PersistenceManager (3 methods, ~45 lines)
```

### JSON Output

For programmatic analysis, use `--format json`:

```json
{
  "file": "src/services/user_manager.rs",
  "is_god_object": true,
  "responsibility_count": 5,
  "responsibilities": [
    "Data Access",
    "Validation",
    "Persistence",
    "Communication",
    "Formatting & Output"
  ],
  "recommended_splits": [
    {
      "suggested_name": "DataAccessManager",
      "methods_to_move": ["get_user", "set_password", "get_email"],
      "responsibility": "Data Access",
      "estimated_lines": 75
    }
  ]
}
```

## Best Practices

### Writing SRP-Compliant Code

1. **Name functions descriptively** - Use standard prefixes (`get_`, `validate_`, etc.)
2. **Group related functions** - Keep similar responsibilities together
3. **Limit responsibility count** - Aim for 1-3 responsibilities per module
4. **Review regularly** - Run Debtmap periodically to catch responsibility creep
5. **Refactor early** - Split modules before they hit thresholds

### Code Review Guidelines

When reviewing responsibility analysis results:

1. **Check responsibility boundaries** - Are they logically distinct?
2. **Validate groupings** - Do the recommended splits make sense?
3. **Consider dependencies** - Will splitting introduce more coupling?
4. **Estimate refactoring cost** - Is the improvement worth the effort?
5. **Prioritize by score** - Focus on high-scoring god objects first

### Team Adoption

**Phase 1: Assessment**
- Run Debtmap on codebase
- Review responsibility violations
- Identify top 10 problematic modules

**Phase 2: Education**
- Share responsibility analysis results with team
- Discuss SRP and its benefits
- Agree on responsibility threshold standards

**Phase 3: Incremental Refactoring**
- Start with highest-scoring modules
- Apply recommended splits
- Measure improvement with follow-up analysis

**Phase 4: Continuous Monitoring**
- Integrate Debtmap into CI/CD
- Track responsibility counts over time
- Prevent new SRP violations from merging

## Limitations and Edge Cases

### False Positives

**Scenario 1: Utilities Module**
```rust
// utilities.rs - 15 helper functions with different prefixes
fn format_date() { }
fn parse_config() { }
fn validate_email() { }
// ... 12 more diverse utilities
```

**Issue:** Flagged as having multiple responsibilities, but it's intentionally a utility collection.

**Solution:** Either accept the flagging (utilities should perhaps be split) or increase `max_traits` threshold.

### False Negatives

**Scenario 2: Poor Naming**
```rust
impl DataProcessor {
    fn process_data(&mut self) { /* does everything */ }
    fn handle_stuff(&mut self) { /* also does everything */ }
    fn do_work(&mut self) { /* yet more mixed concerns */ }
}
```

**Issue:** All methods map to "Processing" or "Utilities", so responsibility count is low despite clear SRP violations.

**Solution:** Encourage better naming conventions in your team. Debtmap relies on descriptive function names.

### Language-Specific Challenges

**Rust:** Trait implementations may group methods by trait rather than responsibility, artificially inflating counts.

**Python:** Dynamic typing and duck typing make responsibility boundaries less clear from signatures alone.

**JavaScript:** Prototype methods and closures may not follow conventional naming patterns.

## Integration with Other Features

### God Object Detection

Responsibility analysis is a core component of [God Object Detection](./god-object-detection.md). The responsibility count contributes to:
- God object scoring
- Confidence level determination
- Refactoring recommendations

### Tiered Prioritization

High responsibility counts increase priority in [Tiered Prioritization](./tiered-prioritization.md) through the god object multiplier.

### Risk Assessment

Modules with multiple responsibilities receive higher risk scores in risk assessment, as they are more prone to bugs and harder to maintain.

## Related Documentation

- [God Object Detection](./god-object-detection.md) - Full god object analysis including responsibility detection
- [Configuration](./configuration.md) - TOML configuration reference
- [Metrics Reference](./metrics-reference.md) - All metrics including responsibility count
- [Architecture](./architecture.md) - High-level design including analysis pipelines

## Summary

Responsibility analysis in Debtmap:

- **Automatically detects SRP violations** through pattern-based method name analysis
- **Categorizes methods** into 11 built-in responsibility types
- **Provides actionable refactoring recommendations** with suggested module splits
- **Integrates with god object detection** for holistic architectural analysis
- **Supports language-specific thresholds** for Rust, Python, and JavaScript/TypeScript
- **Is fully configurable** via `.debtmap.toml` and CLI flags

By surfacing responsibility violations early and suggesting concrete refactoring paths, Debtmap helps teams maintain clean, modular architectures that follow the Single Responsibility Principle.
