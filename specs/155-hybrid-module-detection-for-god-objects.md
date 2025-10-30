---
number: 155
title: Hybrid Module Detection for God Objects
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-10-30
---

# Specification 155: Hybrid Module Detection for God Objects

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The god object detector currently reports **0 responsibilities** for files that have both structs and many module-level functions. This occurs in common Rust architectural patterns where data structures are separate from behavior.

### Current Buggy Behavior

```
#1 SCORE: 82.3 [CRITICAL]
└─ src/priority/formatter.rs (2822 lines, 106 functions)
└─ WHY: This module contains 106 module functions across 0 responsibilities. ❌
└─ METRICS: Methods: 0, Fields: 10, Responsibilities: 0 ❌
...
└─ STRUCTURE: 15 responsibilities across 69 components ✅
```

**Contradictions:**
- Claims "106 module functions" but shows "Methods: 0"
- Reports "0 responsibilities" despite having 106 diverse functions
- Module structure analyzer correctly finds 15 responsibilities
- Output is confusing and undermines trust

### Root Cause

The god object detector has two analysis paths:

1. **God Class Path** (when structs exist)
   - Only analyzes impl block methods
   - Ignores all module-level functions
   - Selected when ANY struct exists

2. **God File Path** (when NO structs exist)
   - Analyzes all module-level functions
   - Selected only when file has zero structs

**The Gap:** Files with structs AND many module-level functions fall into path #1, losing all standalone function analysis.

### Real-World Example: formatter.rs

```rust
// formatter.rs (2822 lines)

struct FormatContext {        // 10 fields, NO impl block
    rank: usize,
    score: f64,
    // ... 8 more fields
}

struct SeverityInfo { ... }   // 2 fields
impl SeverityInfo {           // 1 method
    fn from_score(...) { }
}

// ... 4 more helper structs with simple impls (5 methods total)

// 106 MODULE-LEVEL FUNCTIONS - ALL IGNORED!
pub fn format_priorities(...) { }
pub fn format_priorities_with_verbosity(...) { }
fn format_default(...) { }
fn format_tail(...) { }
fn format_detailed(...) { }
fn format_god_object_steps(...) { }
fn format_module_structure_analysis(...) { }
// ... 99 more formatting functions
```

**What happens:**
1. Detector finds `FormatContext` (largest struct by field count: 10)
2. Selects "God Class" path
3. Only counts `FormatContext` impl methods
4. `FormatContext` has NO impl block → 0 methods
5. All 106 standalone functions ignored
6. Result: 0 responsibilities

### Impact

**Affected Architectural Patterns:**
- **Data separate from behavior** - Common in functional Rust
- **DTO + module functions** - Configuration structs with loader functions
- **Namespace structs** - Empty structs for module organization
- **Helper structs in functional modules** - Small types + many functions

**Examples in debtmap codebase:**
- `src/priority/formatter.rs` - 106 functions, 0 responsibilities reported
- `src/config.rs` - 181 functions, potentially similar issue
- Any module following "data + functions" pattern

## Objective

Implement hybrid module detection to correctly analyze files that have BOTH structs AND many standalone functions. Ensure all functions are analyzed and accurate responsibility counts are reported for all file patterns.

**Success Criteria:**
- formatter.rs shows ~12 responsibilities (not 0)
- All 106 functions counted and analyzed
- Output consistent with module structure analysis
- No regressions in existing God Class or God File detection

## Requirements

### Functional Requirements

**FR1: Hybrid Module Detection**
- Detect when standalone functions dominate over impl methods
- Use threshold: `standalone_count > 50 AND standalone_count > impl_methods * 3`
- Switch to functional module analysis when threshold met
- Preserve existing behavior for true God Classes and God Files

**FR2: New Detection Type**
- Add `DetectionType::GodModule` variant to enum
- Represents hybrid files: structs + many standalone functions
- Distinct from `GodClass` (impl-heavy) and `GodFile` (no structs)

**FR3: Comprehensive Function Analysis**
- Include ALL standalone functions when hybrid detected
- Count responsibility patterns across all functions
- Generate split recommendations based on function groupings
- Report accurate method counts in output

**FR4: Backward Compatibility**
- Preserve existing God Class detection for impl-heavy structs
- Preserve existing God File detection for pure functional modules
- No changes to output format or public API
- Existing tests continue passing

### Non-Functional Requirements

**NFR1: Performance**
- No measurable performance degradation (<1%)
- Detection logic O(n) in function count
- No additional AST parsing required

**NFR2: Accuracy**
- Responsibility count within ±1 of manual classification
- No false positives (hybrid detection for true God Classes)
- No false negatives (missing hybrid modules)

**NFR3: Maintainability**
- Clear, well-documented threshold constants
- Separable detection logic for each type
- Comprehensive test coverage (>90% for modified code)

## Acceptance Criteria

- [ ] `DetectionType::GodModule` enum variant added
- [ ] Hybrid detection logic implemented in `determine_god_object_type`
- [ ] Threshold constants defined: `HYBRID_STANDALONE_THRESHOLD = 50`, `HYBRID_DOMINANCE_RATIO = 3`
- [ ] formatter.rs analyzed as GodModule (not GodClass)
- [ ] formatter.rs shows 12±2 responsibilities (not 0)
- [ ] formatter.rs shows 106 methods counted (not 0)
- [ ] Unit test: hybrid module with 60 standalone + 3 impl methods → GodModule
- [ ] Unit test: God Class with 30 impl methods + 5 standalone → GodClass
- [ ] Unit test: God File with 80 standalone + 0 structs → GodFile
- [ ] Integration test: formatter.rs output shows non-zero responsibilities
- [ ] All existing god object tests pass
- [ ] No clippy warnings introduced
- [ ] Documentation updated with hybrid detection explanation

## Technical Details

### Implementation Approach

**Step 1: Add Detection Type Variant**

Update `src/organization/god_object_detector.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionType {
    /// Single struct with excessive impl methods
    GodClass,

    /// File with excessive standalone functions and no structs
    GodFile,

    /// Hybrid: File with structs AND many standalone functions
    /// Example: DTO structs + 50+ module-level functions
    GodModule,
}
```

**Step 2: Define Threshold Constants**

Add configuration constants:

```rust
/// Minimum standalone functions to consider hybrid detection
const HYBRID_STANDALONE_THRESHOLD: usize = 50;

/// Ratio: standalone must exceed impl methods by this factor
/// Example: 60 standalone, 15 impl methods → 60 > 15*3 = true (hybrid)
const HYBRID_DOMINANCE_RATIO: usize = 3;
```

**Step 3: Implement Hybrid Detection**

Update `determine_god_object_type()` function:

```rust
fn determine_god_object_type(
    primary_type: Option<&TypeAnalysis>,
    visitor: &TypeVisitor,
    standalone_count: usize,
) -> (usize, usize, Vec<String>, u32, DetectionType) {
    if let Some(type_info) = primary_type {
        // Check if this is a hybrid module
        let standalone_dominates = standalone_count > HYBRID_STANDALONE_THRESHOLD
            && standalone_count > type_info.method_count * HYBRID_DOMINANCE_RATIO;

        if standalone_dominates {
            // HYBRID: Structs exist but standalone functions dominate
            // This is primarily a functional module with helper types
            let all_methods = visitor.standalone_functions.clone();
            let total_complexity = estimate_standalone_complexity(standalone_count);

            return (
                standalone_count,
                type_info.field_count,  // Keep field count for context
                all_methods,
                total_complexity,
                DetectionType::GodModule,
            );
        }

        // TRUE GOD CLASS: Struct with many impl methods
        // Filter production methods (exclude tests)
        let struct_method_names: HashSet<_> = type_info.methods.iter().collect();
        let production_complexity: Vec<_> = visitor
            .function_complexity
            .iter()
            .filter(|fc| struct_method_names.contains(&fc.name) && !fc.is_test)
            .cloned()
            .collect();

        let production_methods: Vec<String> = production_complexity
            .iter()
            .map(|fc| fc.name.clone())
            .collect();

        let total_methods = production_methods.len();
        let total_complexity: u32 = production_complexity
            .iter()
            .map(|fc| fc.cyclomatic_complexity)
            .sum();

        (
            total_methods,
            type_info.field_count,
            production_methods,
            total_complexity,
            DetectionType::GodClass,
        )
    } else {
        // PURE GOD FILE: No structs, only standalone functions
        let all_methods = visitor.standalone_functions.clone();
        let total_complexity = estimate_standalone_complexity(standalone_count);

        (
            standalone_count,
            0,
            all_methods,
            total_complexity,
            DetectionType::GodFile,
        )
    }
}

fn estimate_standalone_complexity(count: usize) -> u32 {
    (count * 5) as u32  // Heuristic: avg 5 cyclomatic complexity per function
}
```

**Step 4: Update Output Formatting**

Update `src/priority/formatter.rs` to handle GodModule messaging:

```rust
fn generate_why_message(
    is_god_object: bool,
    fields_count: usize,
    methods_count: usize,
    responsibilities: usize,
    function_count: usize,
    total_lines: usize,
    god_object_type: Option<&crate::organization::GodObjectType>,
    detection_type: Option<DetectionType>,
) -> String {
    if !is_god_object {
        // ... existing non-god-object messaging
    }

    // Determine detection type
    let is_hybrid = matches!(detection_type, Some(DetectionType::GodModule));
    let is_pure_functional = matches!(detection_type, Some(DetectionType::GodFile));

    if is_god_object {
        // God Class: Single struct with many impl methods
        if fields_count > 5 && methods_count > 20 && !is_hybrid && !is_pure_functional {
            format!(
                "This struct violates single responsibility principle with {} methods and {} fields across {} distinct responsibilities. High coupling and low cohesion make it difficult to maintain and test.",
                methods_count,
                fields_count,
                responsibilities
            )
        }
        // Hybrid/Functional: Many module functions
        else if function_count > 50 || is_hybrid || is_pure_functional {
            format!(
                "This module contains {} module functions across {} responsibilities. Large modules with many diverse functions are difficult to navigate, understand, and maintain.",
                function_count,
                responsibilities
            )
        }
        // Default
        else {
            format!(
                "This file contains {} functions with {} distinct responsibilities. Consider splitting by responsibility for better organization.",
                function_count,
                responsibilities
            )
        }
    } else {
        // ... existing non-god-object messaging
    }
}
```

**Step 5: Propagate Detection Type**

Ensure `DetectionType` is available in formatting context:

```rust
// In GodObjectAnalysis struct (already exists)
pub struct GodObjectAnalysis {
    // ... existing fields
    pub detection_type: DetectionType,  // Already present
    // ... remaining fields
}

// Pass to formatter
let god_indicators = GodObjectIndicators {
    // ... existing fields
    detection_type: Some(analysis.detection_type),  // NEW: pass detection type
};
```

### Architecture Changes

**Modified Components:**

1. **`src/organization/god_object_detector.rs`**
   - Add `GodModule` to `DetectionType` enum
   - Add threshold constants
   - Modify `determine_god_object_type()` function
   - Update `analyze_comprehensive()` to propagate detection type

2. **`src/priority/formatter.rs`**
   - Update `generate_why_message()` to handle `GodModule`
   - Accept `detection_type` parameter
   - Adjust messaging logic for hybrid modules

3. **`src/priority/file_metrics.rs`**
   - Add `detection_type` field to `GodObjectIndicators` struct
   - Update struct initialization sites

**Data Flow:**

```
File with structs + many functions
  → TypeVisitor walks AST
  → Counts impl methods + standalone functions
  → determine_god_object_type()
    → Checks hybrid thresholds
    → Returns GodModule type if hybrid
  → group_methods_by_responsibility()
    → Analyzes ALL functions (not just impl methods)
  → Returns GodObjectAnalysis with detection_type
  → Formatter uses detection_type for messaging
  → Output shows accurate method + responsibility counts
```

### Data Structures

**New Enum Variant:**

```rust
pub enum DetectionType {
    GodClass,   // Existing
    GodFile,    // Existing
    GodModule,  // NEW
}
```

**Updated Struct:**

```rust
pub struct GodObjectIndicators {
    // ... existing fields
    pub detection_type: Option<DetectionType>,  // NEW
}
```

### Threshold Rationale

**HYBRID_STANDALONE_THRESHOLD = 50:**
- Chosen based on analysis of debtmap codebase
- 50+ functions indicates significant functional module
- Below 50, likely legitimate helper functions
- Aligns with existing "god file" threshold

**HYBRID_DOMINANCE_RATIO = 3:**
- Standalone must outnumber impl methods 3:1
- Prevents false positives for balanced modules
- Example: 60 standalone, 30 impl methods → 60 > 90? No (GodClass)
- Example: 60 standalone, 15 impl methods → 60 > 45? Yes (GodModule)

**Validation Examples:**

| Standalone | Impl | Threshold? | Dominance? | Result |
|------------|------|------------|------------|--------|
| 106 | 0 | Yes (>50) | Yes (>0) | **GodModule** ✓ |
| 60 | 3 | Yes (>50) | Yes (>9) | **GodModule** ✓ |
| 80 | 30 | Yes (>50) | No (>90) | GodClass |
| 45 | 0 | No (<50) | Yes (>0) | GodClass |
| 0 | 50 | No | No | GodClass |

## Dependencies

**Prerequisites:** None - self-contained bug fix

**Affected Components:**
- `src/organization/god_object_detector.rs` - Core detection logic
- `src/priority/formatter.rs` - Output formatting
- `src/priority/file_metrics.rs` - Data structures
- `src/organization/god_object_analysis.rs` - Helper functions (no changes needed)

**External Dependencies:** None (uses existing infrastructure)

## Testing Strategy

### Unit Tests

**Test 1: Hybrid Module Detection**

```rust
#[test]
fn test_hybrid_module_detected() {
    let code = r#"
struct Config {
    field1: u32,
    field2: String,
    field3: bool,
}

// No impl for Config!

pub fn format_output() { }
pub fn format_header() { }
pub fn format_footer() { }
pub fn format_section() { }
pub fn format_item() { }
// ... 50 more formatting functions ...
    "#;

    let detector = GodObjectDetector::default();
    let ast = syn::parse_file(code).unwrap();
    let path = Path::new("test.rs");
    let analysis = detector.analyze_comprehensive(path, &ast);

    assert_eq!(analysis.detection_type, DetectionType::GodModule);
    assert_eq!(analysis.method_count, 55); // All standalone functions
    assert!(analysis.responsibility_count > 1, "Should detect multiple responsibilities");
    assert!(analysis.responsibility_count > 0, "Should NOT be 0");
}
```

**Test 2: God Class Preserved**

```rust
#[test]
fn test_god_class_with_few_standalone_functions() {
    let code = r#"
struct Database {
    conn: Connection,
    pool: Pool,
}

impl Database {
    pub fn get(&self, id: u32) { }
    pub fn set(&mut self, id: u32, val: String) { }
    pub fn delete(&self, id: u32) { }
    pub fn update(&mut self, id: u32, val: String) { }
    // ... 26 more impl methods ...
}

pub fn main() { }
pub fn init() { }
pub fn cleanup() { }
    "#;

    let detector = GodObjectDetector::default();
    let ast = syn::parse_file(code).unwrap();
    let path = Path::new("test.rs");
    let analysis = detector.analyze_comprehensive(path, &ast);

    assert_eq!(analysis.detection_type, DetectionType::GodClass);
    assert_eq!(analysis.method_count, 30); // Impl methods only
    // Standalone functions (3) should NOT trigger hybrid
}
```

**Test 3: God File Preserved**

```rust
#[test]
fn test_god_file_no_structs() {
    let code = r#"
pub fn process_a() { }
pub fn process_b() { }
pub fn process_c() { }
// ... 77 more standalone functions ...
    "#;

    let detector = GodObjectDetector::default();
    let ast = syn::parse_file(code).unwrap();
    let path = Path::new("test.rs");
    let analysis = detector.analyze_comprehensive(path, &ast);

    assert_eq!(analysis.detection_type, DetectionType::GodFile);
    assert_eq!(analysis.method_count, 80);
}
```

**Test 4: Threshold Boundaries**

```rust
#[test]
fn test_hybrid_threshold_boundaries() {
    // Case 1: Exactly at threshold (50 standalone, 0 impl)
    assert_eq!(
        detect_type_for_counts(50, 0),
        DetectionType::GodModule,
        "50 standalone should trigger hybrid"
    );

    // Case 2: Just below threshold (49 standalone, 0 impl)
    assert_eq!(
        detect_type_for_counts(49, 0),
        DetectionType::GodClass,
        "49 standalone should NOT trigger hybrid"
    );

    // Case 3: Dominance ratio exactly at boundary
    assert_eq!(
        detect_type_for_counts(60, 20),
        DetectionType::GodClass,
        "60 standalone, 20 impl (60 == 20*3) should be GodClass"
    );

    // Case 4: Dominance ratio exceeds boundary
    assert_eq!(
        detect_type_for_counts(61, 20),
        DetectionType::GodModule,
        "61 standalone, 20 impl (61 > 20*3) should be GodModule"
    );
}

// Helper function for threshold testing
fn detect_type_for_counts(standalone: usize, impl_methods: usize) -> DetectionType {
    // Build minimal AST with specified counts
    // ... (implementation details)
}
```

**Test 5: Responsibility Counting**

```rust
#[test]
fn test_hybrid_responsibility_counting() {
    let code = r#"
struct Config { }

// Group 1: Formatting (20 functions)
pub fn format_a() { }
pub fn format_b() { }
// ... 18 more

// Group 2: Validation (15 functions)
pub fn validate_a() { }
pub fn validate_b() { }
// ... 13 more

// Group 3: Computation (15 functions)
pub fn calculate_a() { }
pub fn compute_b() { }
// ... 13 more
    "#;

    let detector = GodObjectDetector::default();
    let ast = syn::parse_file(code).unwrap();
    let path = Path::new("test.rs");
    let analysis = detector.analyze_comprehensive(path, &ast);

    assert_eq!(analysis.detection_type, DetectionType::GodModule);
    assert_eq!(analysis.method_count, 50);
    assert!(analysis.responsibility_count >= 3, "Should detect at least 3 groups");
    assert!(analysis.responsibilities.contains(&"Formatting & Output".to_string()));
    assert!(analysis.responsibilities.contains(&"Validation".to_string()));
    assert!(analysis.responsibilities.contains(&"Computation".to_string()));
}
```

### Integration Tests

**Integration Test 1: Formatter.rs Analysis**

```rust
#[test]
fn test_formatter_rs_shows_responsibilities() {
    let output = Command::new("cargo")
        .args(&["run", "--", "analyze", "src/priority/formatter.rs"])
        .output()
        .expect("Failed to run debtmap");

    let output_str = String::from_utf8(output.stdout).unwrap();

    // Should NOT show 0 responsibilities
    assert!(!output_str.contains("0 responsibilities"),
            "formatter.rs should not show 0 responsibilities");

    // Should show substantial responsibility count
    let re = Regex::new(r"(\d+) responsibilities").unwrap();
    let caps = re.captures(&output_str).expect("Should find responsibility count");
    let count: usize = caps[1].parse().unwrap();

    assert!(count >= 10, "Should detect at least 10 responsibilities, got {}", count);
    assert!(count <= 20, "Should detect at most 20 responsibilities, got {}", count);

    // Should show actual method count
    assert!(output_str.contains("106 functions") || output_str.contains("106 module functions"));

    // Should be classified as GodModule (hybrid)
    assert!(output_str.contains("module contains") || output_str.contains("module functions"));
}
```

**Integration Test 2: Config.rs Analysis**

```rust
#[test]
fn test_config_rs_analysis() {
    let output = Command::new("cargo")
        .args(&["run", "--", "analyze", "src/config.rs"])
        .output()
        .expect("Failed to run debtmap");

    let output_str = String::from_utf8(output.stdout).unwrap();

    // Should show non-zero responsibilities
    assert!(!output_str.contains("0 responsibilities"));

    // Should count substantial function count
    let re = Regex::new(r"(\d+) functions").unwrap();
    let caps = re.captures(&output_str).expect("Should find function count");
    let count: usize = caps[1].parse().unwrap();

    assert!(count > 100, "Config.rs has 181 functions, should count most of them");
}
```

**Integration Test 3: Regression Prevention**

```rust
#[test]
fn test_no_regression_on_god_classes() {
    // Test on a known God Class file
    let output = Command::new("cargo")
        .args(&["run", "--", "analyze", "src/analysis/python_type_tracker/mod.rs"])
        .output()
        .expect("Failed to run debtmap");

    let output_str = String::from_utf8(output.stdout).unwrap();

    // Should still be detected as god object
    assert!(output_str.contains("CRITICAL") || output_str.contains("god object"));

    // Should show struct-focused messaging
    assert!(output_str.contains("struct") || output_str.contains("methods"));
}
```

### Manual Verification

After implementation, verify:

```bash
# 1. Run formatter.rs analysis
cargo run -- analyze src/priority/formatter.rs

# Expected: Shows ~12 responsibilities, 106 functions
# Should say: "This module contains 106 module functions across 12 responsibilities"

# 2. Run config.rs analysis
cargo run -- analyze src/config.rs

# Expected: Shows non-zero responsibilities, ~181 functions

# 3. Run full codebase analysis
cargo run -- analyze .

# Expected:
# - formatter.rs not in top 10 (or much lower score)
# - No files showing "0 responsibilities" with >50 functions
# - Consistent messaging across similar file patterns

# 4. Run test suite
cargo test --all

# Expected: All tests pass, no regressions
```

## Documentation Requirements

### Code Documentation

**Enum Variant Documentation:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionType {
    /// Single struct with excessive impl methods.
    ///
    /// Example: A `UserManager` struct with 40 impl methods across
    /// multiple responsibilities (validation, persistence, formatting).
    GodClass,

    /// File with excessive standalone functions and no structs.
    ///
    /// Example: A functional module with 80 top-level functions
    /// for data processing, no struct definitions.
    GodFile,

    /// Hybrid: File with both structs AND many standalone functions.
    ///
    /// Detected when standalone functions dominate (>50 functions and
    /// >3x the impl method count). Common in modules following "data
    /// separate from behavior" patterns.
    ///
    /// Example: A formatter module with DTO structs (10 fields) and
    /// 106 formatting functions.
    ///
    /// # Detection Criteria
    ///
    /// A file is classified as `GodModule` when:
    /// - Contains at least one struct definition
    /// - Has >50 standalone functions
    /// - Standalone count > (impl method count * 3)
    GodModule,
}
```

**Threshold Constants Documentation:**

```rust
/// Minimum standalone functions required to trigger hybrid detection.
///
/// Files with fewer standalone functions are assumed to have helper
/// functions that complement the primary struct's impl methods.
///
/// Chosen based on analysis of Rust projects: 50+ functions typically
/// indicates a functional module rather than helpers.
const HYBRID_STANDALONE_THRESHOLD: usize = 50;

/// Dominance ratio: standalone functions must exceed impl methods by this factor.
///
/// Prevents false positives for balanced OOP/functional modules. A ratio of 3:1
/// ensures standalone functions truly dominate the file's purpose.
///
/// Examples:
/// - 60 standalone, 15 impl → 60 > 45? Yes → Hybrid
/// - 60 standalone, 25 impl → 60 > 75? No → God Class
const HYBRID_DOMINANCE_RATIO: usize = 3;
```

**Function Documentation:**

```rust
/// Determine the type of god object based on struct and function counts.
///
/// This function implements a three-way classification:
///
/// 1. **GodClass**: Struct with many impl methods, few standalone functions
/// 2. **GodFile**: No structs, only standalone functions
/// 3. **GodModule**: Structs + many standalone functions (hybrid)
///
/// # Hybrid Detection Logic
///
/// A file is classified as hybrid when:
/// - At least one struct exists (`primary_type.is_some()`)
/// - Standalone count > [`HYBRID_STANDALONE_THRESHOLD`] (default: 50)
/// - Standalone count > impl method count * [`HYBRID_DOMINANCE_RATIO`] (default: 3)
///
/// # Examples
///
/// ```rust
/// // God Class: 30 impl methods, 5 standalone
/// // → Analyze impl methods only
///
/// // God File: 0 structs, 80 standalone
/// // → Analyze all standalone functions
///
/// // Hybrid (God Module): 1 struct, 106 standalone
/// // → Analyze all standalone functions
/// ```
fn determine_god_object_type(...) -> (...) {
    // implementation
}
```

### User Documentation

Add to debtmap user guide (`book/src/god-object-detection.md`):

```markdown
## God Object Types

Debtmap detects three types of god objects:

### God Class

A **struct with excessive impl methods** across multiple responsibilities.

**Example:**
```rust
struct UserManager {
    db: Database,
}

impl UserManager {
    fn get_user(...) { }
    fn validate_email(...) { }
    fn save_to_db(...) { }
    fn format_profile(...) { }
    // ... 26 more methods
}
```

**Detection:** Struct with >15 methods and >3 responsibilities.

### God File

A **file with excessive standalone functions** and no struct definitions.

**Example:**
```rust
pub fn process_data(...) { }
pub fn validate_input(...) { }
pub fn format_output(...) { }
// ... 77 more functions
```

**Detection:** File with >50 standalone functions and no structs.

### God Module (Hybrid)

A **file with both structs AND many standalone functions**, where functions dominate.

**Example:**
```rust
struct Config {  // DTO with no impl
    field1: u32,
    field2: String,
}

pub fn format_output(...) { }
pub fn format_header(...) { }
pub fn format_section(...) { }
// ... 103 more formatting functions
```

**Detection:** File with structs + >50 standalone functions (3:1 ratio).

**Common in:** Functional Rust modules with helper types, formatters with DTOs, modules following "data separate from behavior" patterns.

### How Detection Works

1. Debtmap counts impl methods and standalone functions
2. Selects primary struct (if any) by method + field count
3. Checks if standalone functions dominate:
   - Standalone > 50? AND
   - Standalone > impl methods × 3?
4. If yes → **God Module**, else → **God Class** or **God File**
```

### Architecture Documentation

Add to `ARCHITECTURE.md` or inline documentation:

```markdown
## God Object Detection: Hybrid Modules

### Problem

Files with DTOs + many functions were incorrectly classified as "God Class"
and only analyzed impl methods, ignoring all standalone functions.

### Solution

Added `DetectionType::GodModule` for hybrid files. Detection thresholds:
- `HYBRID_STANDALONE_THRESHOLD = 50` - Minimum standalone count
- `HYBRID_DOMINANCE_RATIO = 3` - Standalone must exceed impl by 3:1

### Algorithm

```
if has_structs:
    if standalone > 50 AND standalone > impl * 3:
        → GodModule (analyze all functions)
    else:
        → GodClass (analyze impl methods)
else:
    → GodFile (analyze all functions)
```

### Examples

- `formatter.rs`: 106 standalone, 5 impl → **GodModule** ✓
- `Database`: 30 impl, 5 standalone → **GodClass** ✓
- `utils.rs`: 80 standalone, 0 structs → **GodFile** ✓
```

## Implementation Notes

### Edge Cases

**Case 1: Struct with no impl block**
- FormatContext has 10 fields, 0 methods
- Previously selected as primary → 0 methods counted
- Now: Standalone functions dominate → GodModule

**Case 2: Multiple small structs + many functions**
- 5 structs with 1-2 methods each (10 methods total)
- 60 standalone functions
- Result: 60 > 50 AND 60 > 30 → GodModule ✓

**Case 3: Balanced hybrid**
- 30 impl methods + 60 standalone functions
- Result: 60 > 50 BUT 60 < 90 → GodClass
- Rationale: Impl methods are still significant

**Case 4: Just below threshold**
- 49 standalone functions, 0 impl methods
- Result: 49 < 50 → GodClass (not hybrid)
- May show 0 responsibilities (existing issue, different fix needed)

### Performance Considerations

- Threshold checks are O(1)
- No additional AST traversal required
- Counts already computed by TypeVisitor
- Expected performance impact: <0.1%

### Configuration

Consider making thresholds configurable in `.debtmap.toml`:

```toml
[god_object_detection]
hybrid_standalone_threshold = 50
hybrid_dominance_ratio = 3
```

This can be added in a future spec if needed.

### Related Improvements

**Not in scope for this spec:**
1. Unified responsibility detection across module structure analyzer and god object detector
2. Adjusting primary type selection algorithm (field weight)
3. Fixing 0 responsibility report for <50 standalone functions

These should be separate specs if prioritized.

## Migration and Compatibility

### Breaking Changes

None. This is a bug fix that improves accuracy.

### Backward Compatibility

**API Stability:**
- `DetectionType` enum is non-exhaustive (can add variants)
- Existing variants preserved
- Serialization format unchanged

**Output Changes:**
- Files previously showing "0 responsibilities" will show accurate counts
- May affect some tests that assert specific responsibility counts
- Overall scores may change for hybrid modules (more accurate)

### Migration Path

**Phase 1: Implementation and Testing**
1. Add GodModule variant
2. Implement hybrid detection
3. Run test suite
4. Fix any broken tests (update expected values)

**Phase 2: Validation**
1. Run on debtmap codebase
2. Verify formatter.rs shows 12±2 responsibilities
3. Verify config.rs shows non-zero responsibilities
4. Spot-check other files for consistency

**Phase 3: Deployment**
1. Merge to main
2. Immediate rollout (no feature flag needed)
3. Monitor for unexpected behavior
4. Update documentation

### Rollback Strategy

If issues discovered:
1. Git revert the commit
2. Investigate root cause
3. Add more tests for edge case
4. Re-implement with fix

## Success Metrics

### Correctness Metrics

- [ ] formatter.rs: 0 → 12±2 responsibilities
- [ ] formatter.rs: 0 → 106 method count
- [ ] config.rs: Shows non-zero responsibilities
- [ ] No files >50 functions showing 0 responsibilities
- [ ] All god object tests pass
- [ ] No new clippy warnings

### Regression Metrics

- [ ] Existing God Class files still detected correctly
- [ ] Existing God File files still detected correctly
- [ ] No change in false positive rate (<5%)
- [ ] No change in false negative rate (<5%)

### Impact Metrics

- [ ] Output consistency: Module structure vs god object responsibilities within ±3
- [ ] User confidence: Self-analysis produces actionable recommendations
- [ ] Coverage: All architectural patterns correctly classified

## Timeline Estimate

- **Enum variant + constants**: 15 minutes
- **Core detection logic**: 1.5 hours
- **Formatter updates**: 30 minutes
- **Unit tests**: 1.5 hours
- **Integration tests**: 45 minutes
- **Documentation**: 45 minutes
- **Testing and validation**: 45 minutes

**Total**: ~6 hours for complete implementation, testing, and documentation

## Related Specifications

- **Spec 154**: Fix Module Structure Line Range Calculation (similar impact on accuracy)
- **Spec 133**: God Object Detection Refinement (related but different focus)
- **Spec 146**: Rust Specific Responsibility Patterns (uses god object results)

## Follow-Up Work

### Immediate (This Spec)
- Implement hybrid detection
- Add comprehensive tests
- Update documentation

### Future (Separate Specs)
- Unified responsibility detection across analyzers
- Configurable thresholds via .debtmap.toml
- Improved primary type selection algorithm
- Machine learning for threshold tuning
