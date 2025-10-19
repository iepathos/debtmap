---
number: 118
title: Pure vs Impure Function Distinction in God Object Scoring
category: optimization
priority: high
status: draft
dependencies: [117]
created: 2025-10-18
---

# Specification 118: Pure vs Impure Function Distinction in God Object Scoring

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 117 (Complexity-Weighted God Object Scoring)

## Context

After implementing complexity weighting (Spec 117), debtmap still produces false positives on functionally-designed code that uses many small pure helper functions. The current implementation treats all functions equally, regardless of whether they have side effects.

**Real-World Pattern**:
```rust
// src/cache/shared_cache.rs - Well-designed with functional principles

// Pure helper functions (65% of total)
fn validate_file_path(path: &Path) -> Result<()> { ... }           // Pure
fn calculate_max_age_duration(days: i64) -> Duration { ... }       // Pure
fn should_prune_after_insertion(pruner: &AutoPruner) -> bool { ... } // Pure
fn create_safe_temp_path(target: &Path) -> PathBuf { ... }         // Pure

// I/O operations at boundaries (35% of total)
pub fn put(&self, key: &str, data: &[u8]) -> Result<()> { ... }   // Impure
pub fn get(&self, key: &str) -> Result<Vec<u8>> { ... }            // Impure
fn execute_cache_storage(&self, ...) -> Result<()> { ... }         // Impure
```

**Current Behavior**:
All 107 functions counted equally:
```
shared_cache.rs: 107 functions → weighted_count = 35 → score = 25
```

**What Should Happen**:
Pure functions should contribute less to god object score:
```
shared_cache.rs:
  - 70 pure functions × 0.5 weight = 35
  - 37 impure functions × 1.0 weight = 37
  - Total weighted: 72 → score = 18 (below threshold)
```

**Why This is Important**:
- **Functional programming principle**: "Pure core, impure shell" is a best practice
- **Good design pattern**: Extract pure logic into testable helper functions
- **Current penalty**: Code that follows FP principles scores worse than imperative code
- **False positives**: Files with many pure helpers flagged incorrectly

## Objective

Extend god object detection to distinguish pure from impure functions, applying reduced weighting to pure functions to reward functional programming patterns and eliminate false positives on well-designed code.

## Requirements

### Functional Requirements

1. **Pure Function Detection**

   Classify functions as pure based on these criteria:

   **Definite Pure** (weight 0.3):
   - No `mut` parameters or receivers (`&mut self`, `mut param`)
   - No I/O operations (`fs::`, `println!`, `eprintln!`)
   - No mutable state access (no field mutation)
   - Returns `Result`, `Option`, or plain value
   - All parameters are references or Copy types

   **Probably Pure** (weight 0.5):
   - Static function or associated function (`fn name()` not `&self`)
   - No obvious side effects in body
   - Simple computations or transformations
   - May use `&self` but doesn't mutate

   **Impure** (weight 1.0):
   - Uses `&mut self` or `mut` parameters
   - Contains I/O operations
   - Modifies external state
   - Async functions (usually I/O-bound)

2. **Purity Analysis Algorithm**

   ```rust
   pub enum PurityLevel {
       Pure,           // Guaranteed no side effects → weight 0.3
       ProbablyPure,   // Likely no side effects → weight 0.5
       Impure,         // Has side effects → weight 1.0
   }

   impl PurityLevel {
       pub fn weight_multiplier(&self) -> f64 {
           match self {
               PurityLevel::Pure => 0.3,
               PurityLevel::ProbablyPure => 0.5,
               PurityLevel::Impure => 1.0,
           }
       }
   }

   fn analyze_function_purity(item_fn: &ItemFn) -> PurityLevel {
       // Check signature
       if has_mutable_params(item_fn) || has_mutable_receiver(item_fn) {
           return PurityLevel::Impure;
       }

       // Check body for side effects
       let body_analysis = analyze_body_for_side_effects(&item_fn.block);

       if body_analysis.has_io_operations {
           return PurityLevel::Impure;
       }

       if body_analysis.has_mutation {
           return PurityLevel::Impure;
       }

       if is_static_or_associated_fn(item_fn) {
           return PurityLevel::Pure;
       }

       PurityLevel::ProbablyPure
   }
   ```

3. **Combined Weighting Formula**

   Each function's contribution = `complexity_weight × purity_weight`

   ```rust
   fn calculate_total_function_weight(func: &FunctionInfo) -> f64 {
       let complexity_weight = calculate_complexity_weight(func.cyclomatic_complexity);
       let purity_weight = func.purity_level.weight_multiplier();

       complexity_weight * purity_weight
   }

   // Examples:
   // Pure function, complexity 1:    0.33 × 0.3 = 0.10
   // Pure function, complexity 3:    1.0  × 0.3 = 0.30
   // Impure function, complexity 1:  0.33 × 1.0 = 0.33
   // Impure function, complexity 17: 8.2  × 1.0 = 8.20
   ```

4. **Real-World Example: shared_cache.rs**

   **Before purity weighting** (from Spec 117):
   - 107 functions, avg complexity 2.1
   - Weighted count: 35
   - Score: 25 (borderline god object)

   **After purity weighting**:
   - 70 pure functions (complexity 1-3) → weighted: 21 × 0.3 = 6.3
   - 37 impure functions (complexity 1-5) → weighted: 14 × 1.0 = 14
   - Total weighted count: 20.3
   - Score: 12 (not a god object ✓)

### Non-Functional Requirements

1. **Accuracy**: Correctly classify >90% of functions as pure/impure
2. **Performance**: Purity analysis adds <3% overhead
3. **Conservative**: When uncertain, classify as impure (avoid false negatives)
4. **Transparency**: Show purity breakdown in output

## Acceptance Criteria

- [ ] `PurityLevel` enum with three levels (Pure, ProbablyPure, Impure)
- [ ] `analyze_function_purity()` correctly classifies test cases
- [ ] Pure functions detected: no `mut`, no I/O, no state mutation
- [ ] Impure functions detected: uses `&mut`, has I/O, async
- [ ] Combined weight = complexity_weight × purity_weight
- [ ] `shared_cache.rs` scores <15 with purity weighting
- [ ] God object detector shows purity distribution in output
- [ ] Test suite validates purity classification accuracy
- [ ] Documentation explains purity weighting
- [ ] False positive rate <5% on benchmark codebase

## Technical Details

### Implementation Approach

1. **Phase 1: AST-Based Purity Detection**

   ```rust
   // src/organization/purity_analyzer.rs
   pub struct PurityAnalyzer;

   impl PurityAnalyzer {
       /// Analyze function signature for purity indicators
       pub fn analyze_signature(sig: &Signature) -> PurityIndicators {
           PurityIndicators {
               has_mutable_receiver: Self::has_mut_receiver(sig),
               has_mutable_params: Self::has_mut_params(sig),
               is_async: sig.asyncness.is_some(),
               is_static: Self::is_static_fn(sig),
           }
       }

       /// Analyze function body for side effects
       pub fn analyze_body(block: &Block) -> SideEffectAnalysis {
           let mut visitor = SideEffectVisitor::new();
           visitor.visit_block(block);

           SideEffectAnalysis {
               has_io_operations: visitor.io_operations.is_empty(),
               has_mutations: visitor.mutations.is_empty(),
               has_unsafe: visitor.unsafe_blocks > 0,
               macro_calls: visitor.macro_calls.clone(),
           }
       }

       /// Combine signature and body analysis
       pub fn determine_purity(
           sig_indicators: &PurityIndicators,
           body_analysis: &SideEffectAnalysis,
       ) -> PurityLevel {
           // Definite impure cases
           if sig_indicators.has_mutable_receiver
               || sig_indicators.has_mutable_params
               || sig_indicators.is_async
               || body_analysis.has_io_operations
               || body_analysis.has_mutations
               || body_analysis.has_unsafe
           {
               return PurityLevel::Impure;
           }

           // Definite pure case
           if sig_indicators.is_static && !body_analysis.has_mutations {
               return PurityLevel::Pure;
           }

           // Default to probably pure (conservative)
           PurityLevel::ProbablyPure
       }
   }
   ```

2. **Phase 2: Side Effect Visitor**

   ```rust
   struct SideEffectVisitor {
       io_operations: Vec<String>,
       mutations: Vec<String>,
       unsafe_blocks: usize,
       macro_calls: Vec<String>,
   }

   impl<'ast> Visit<'ast> for SideEffectVisitor {
       fn visit_expr_call(&mut self, call: &'ast ExprCall) {
           // Detect I/O: fs::, println!, eprintln!, write!, etc.
           if let Expr::Path(path) = &*call.func {
               let path_str = quote!(#path).to_string();

               if path_str.contains("fs::") || path_str.contains("io::") {
                   self.io_operations.push(path_str);
               }
           }

           visit::visit_expr_call(self, call);
       }

       fn visit_expr_method_call(&mut self, method: &'ast ExprMethodCall) {
           // Detect mutation: push, insert, remove, etc.
           let method_name = method.method.to_string();

           if MUTATING_METHODS.contains(&method_name.as_str()) {
               self.mutations.push(method_name);
           }

           visit::visit_expr_method_call(self, method);
       }

       fn visit_expr_assign(&mut self, assign: &'ast ExprAssign) {
           // Any assignment is mutation
           self.mutations.push("assignment".to_string());
           visit::visit_expr_assign(self, assign);
       }

       fn visit_expr_unsafe(&mut self, _unsafe: &'ast ExprUnsafe) {
           self.unsafe_blocks += 1;
       }
   }

   const MUTATING_METHODS: &[&str] = &[
       "push", "pop", "insert", "remove", "clear", "extend",
       "append", "drain", "swap", "sort", "reverse",
   ];
   ```

3. **Phase 3: Integration with God Object Detector**

   ```rust
   // In god_object_detector.rs
   impl GodObjectDetector {
       pub fn analyze_with_purity(&self, path: &Path, ast: &syn::File) -> GodObjectAnalysis {
           let mut visitor = TypeVisitor::new();
           visitor.visit_file(ast);

           // Analyze purity for each function
           let function_weights: Vec<FunctionWeight> = visitor
               .functions
               .iter()
               .map(|func| {
                   let purity = PurityAnalyzer::analyze(func);
                   let complexity_weight = calculate_complexity_weight(func.complexity);
                   let purity_weight = purity.weight_multiplier();

                   FunctionWeight {
                       name: func.name.clone(),
                       complexity: func.complexity,
                       purity_level: purity,
                       complexity_weight,
                       purity_weight,
                       total_weight: complexity_weight * purity_weight,
                   }
               })
               .collect();

           let weighted_count: f64 = function_weights
               .iter()
               .map(|w| w.total_weight)
               .sum();

           // Generate analysis with purity breakdown
           self.create_analysis_with_purity(weighted_count, function_weights)
       }
   }
   ```

### Architecture Changes

**New Modules**:
- `src/organization/purity_analyzer.rs` - Purity detection logic
- `src/organization/side_effect_visitor.rs` - AST visitor for side effects

**Modified Modules**:
- `src/organization/god_object_detector.rs` - Integrate purity analysis
- `src/organization/god_object_analysis.rs` - Add purity metrics to output

**Data Flow**:
```
Function AST → Analyze Signature → Analyze Body → Determine Purity → Apply Weight
     ↓              ↓                    ↓              ↓              ↓
  ItemFn      has_mut=false       has_io=false    Pure (0.3)    weight × 0.3
```

### Data Structures

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurityLevel {
    Pure,          // Guaranteed no side effects
    ProbablyPure,  // Likely no side effects
    Impure,        // Has side effects
}

#[derive(Debug, Clone)]
pub struct PurityIndicators {
    pub has_mutable_receiver: bool,
    pub has_mutable_params: bool,
    pub is_async: bool,
    pub is_static: bool,
}

#[derive(Debug, Clone)]
pub struct SideEffectAnalysis {
    pub has_io_operations: bool,
    pub has_mutations: bool,
    pub has_unsafe: bool,
    pub macro_calls: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionWeight {
    pub name: String,
    pub complexity: u32,
    pub purity_level: PurityLevel,
    pub complexity_weight: f64,
    pub purity_weight: f64,
    pub total_weight: f64,
}

#[derive(Debug, Clone)]
pub struct PurityDistribution {
    pub pure_count: usize,
    pub probably_pure_count: usize,
    pub impure_count: usize,
    pub pure_weight_contribution: f64,
    pub impure_weight_contribution: f64,
}
```

### APIs and Interfaces

**Public API**:
```rust
impl PurityAnalyzer {
    /// Analyze function and determine purity level
    pub fn analyze(item_fn: &ItemFn) -> PurityLevel;

    /// Get detailed purity information
    pub fn analyze_detailed(item_fn: &ItemFn) -> PurityAnalysisResult;

    /// Check if function signature indicates purity
    pub fn is_pure_signature(sig: &Signature) -> bool;
}

impl GodObjectDetector {
    /// Analyze with both complexity and purity weighting
    pub fn analyze_with_purity(
        &self,
        path: &Path,
        ast: &syn::File,
    ) -> GodObjectAnalysisWithPurity;
}
```

**Internal APIs**:
```rust
fn has_mut_receiver(sig: &Signature) -> bool;
fn has_mut_params(sig: &Signature) -> bool;
fn contains_io_operations(block: &Block) -> bool;
fn contains_mutations(block: &Block) -> bool;
```

## Dependencies

- **Prerequisites**: Spec 117 (Complexity-Weighted God Object Scoring)
- **Affected Components**:
  - `src/organization/god_object_detector.rs`
  - `src/organization/god_object_analysis.rs`
  - Existing purity detector in `src/analyzers/purity_detector.rs` (may be reusable)
- **External Dependencies**: `syn` (already used), `quote` (for path stringification)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_pure_function_detection() {
    let code = quote! {
        fn pure_calculation(x: i32, y: i32) -> i32 {
            x + y
        }
    };
    let func = parse_function(&code);
    assert_eq!(PurityAnalyzer::analyze(&func), PurityLevel::Pure);
}

#[test]
fn test_impure_function_with_mutation() {
    let code = quote! {
        fn impure_mutation(&mut self, value: i32) {
            self.field = value;
        }
    };
    let func = parse_function(&code);
    assert_eq!(PurityAnalyzer::analyze(&func), PurityLevel::Impure);
}

#[test]
fn test_impure_function_with_io() {
    let code = quote! {
        fn impure_io() -> Result<String> {
            fs::read_to_string("file.txt")
        }
    };
    let func = parse_function(&code);
    assert_eq!(PurityAnalyzer::analyze(&func), PurityLevel::Impure);
}

#[test]
fn test_combined_purity_complexity_weight() {
    let pure_simple = FunctionInfo {
        complexity: 1,
        purity: PurityLevel::Pure,
    };
    assert_eq!(calculate_total_weight(&pure_simple), 0.10); // 0.33 × 0.3

    let impure_complex = FunctionInfo {
        complexity: 17,
        purity: PurityLevel::Impure,
    };
    assert_eq!(calculate_total_weight(&impure_complex), 8.2); // 8.2 × 1.0
}
```

### Integration Tests

```rust
#[test]
fn test_shared_cache_purity_classification() {
    let analysis = analyze_file("src/cache/shared_cache.rs");

    // Verify pure function detection
    assert!(analysis.purity_distribution.pure_count > 50);
    assert!(analysis.weighted_count < 25.0);
    assert!(analysis.god_object_score < 15.0);
}

#[test]
fn test_purity_distribution_output() {
    let analysis = analyze_file("src/cache/shared_cache.rs");
    let output = format_analysis(&analysis);

    // Should show purity breakdown
    assert!(output.contains("70 pure functions"));
    assert!(output.contains("purity weight: 0.3"));
}
```

### Performance Tests

```rust
#[bench]
fn bench_purity_analysis(b: &mut Bencher) {
    let ast = parse_large_file();
    b.iter(|| {
        PurityAnalyzer::analyze_all_functions(&ast);
    });
}
```

### User Acceptance

- [ ] Run debtmap on self with purity weighting enabled
- [ ] Verify `shared_cache.rs` scores <15 (not a god object)
- [ ] Verify functional code with pure helpers scores better than imperative code
- [ ] False positive rate <5% on 5 test codebases

## Documentation Requirements

### Code Documentation

```rust
/// Determine if a function is pure (has no side effects).
///
/// # Purity Classification
///
/// **Pure** (weight 0.3):
/// - No mutable parameters or receivers
/// - No I/O operations
/// - No state mutation
/// - Returns deterministic result
///
/// **Probably Pure** (weight 0.5):
/// - Static or associated function
/// - No obvious side effects
/// - Simple computations
///
/// **Impure** (weight 1.0):
/// - Uses `&mut self` or `mut` parameters
/// - Contains I/O operations
/// - Modifies external state
///
/// # Examples
///
/// ```rust
/// // Pure function
/// fn add(x: i32, y: i32) -> i32 { x + y }
///
/// // Impure function
/// fn save(&mut self, data: &[u8]) -> Result<()> {
///     fs::write(&self.path, data)
/// }
/// ```
pub fn analyze_function_purity(item_fn: &ItemFn) -> PurityLevel
```

### User Documentation

Update `README.md`:

```markdown
## God Object Detection

Debtmap uses both **complexity weighting** and **purity analysis** to
accurately identify god objects while avoiding false positives on
well-designed functional code.

### Purity Weighting

Functions are weighted based on their side effects:

- **Pure functions** (0.3×): No mutation, no I/O, deterministic
- **Probably pure** (0.5×): Static functions with no obvious side effects
- **Impure functions** (1.0×): Uses mutation, I/O, or async operations

### Why This Matters

Code following functional programming principles naturally has many
small pure helper functions. Without purity weighting, this good
practice would be incorrectly flagged as a god object.

**Example**:
```rust
// Functional design - NOT a god object
struct Cache {
    // 70 pure helper functions (complexity 1-3)
    // 37 impure I/O operations (complexity 1-5)
    // Total weight: 20.3 → Score: 12
}

// God object - correctly flagged
struct Analysis {
    // 15 impure functions (complexity 17-33)
    // Total weight: 150 → Score: 65
}
```
```

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
### Purity Analysis

**Location**: `src/organization/purity_analyzer.rs`

**Purpose**: Distinguish pure from impure functions to reward functional
programming patterns in god object detection.

**Algorithm**:
1. Analyze function signature (mutable params, async, static)
2. Analyze function body (I/O, mutations, unsafe)
3. Classify as Pure (0.3), ProbablyPure (0.5), or Impure (1.0)
4. Apply weight multiplier to complexity weight

**Integration**: Used by `GodObjectDetector` in combination with
complexity weighting (Spec 117).
```

## Implementation Notes

### Gotchas

1. **Conservative Classification**: When unsure, classify as impure to avoid false negatives
2. **Macro Calls**: Cannot analyze macro expansions, assume impure unless known pure macros
3. **Trait Methods**: May be pure but require whole-program analysis (future enhancement)
4. **Generic Functions**: Purity depends on type parameters (assume impure for now)

### Best Practices

1. **Whitelist Pure Macros**: Maintain list of known pure macros (e.g., `vec!`, `format!`)
2. **Cache Results**: Purity analysis result can be cached per function
3. **Transparent Output**: Always show purity distribution in verbose mode
4. **Gradual Rollout**: Add `--purity-weighted` flag initially

### Example Output

```
#3 SCORE: 12.0 [OK - NOT A GOD OBJECT]
├─ ./src/cache/shared_cache.rs (2196 lines, 107 functions)
├─ PURITY DISTRIBUTION:
│  ├─ Pure: 70 functions (65%) → weight contribution: 6.3
│  ├─ Probably Pure: 0 functions (0%)
│  └─ Impure: 37 functions (35%) → weight contribution: 14.0
├─ WEIGHTED METRICS:
│  ├─ Raw function count: 107
│  ├─ Complexity-weighted: 35.0
│  ├─ Purity-weighted: 20.3
│  └─ Final score: 12.0
├─ WHY: Functional design with many pure helpers (good practice)
└─ ACTION: No refactoring needed - well-designed cache module
```

## Migration and Compatibility

### Breaking Changes

None. Purity weighting is additive enhancement to complexity weighting (Spec 117).

### Migration Path

1. **Phase 1 (v0.2.9)**: Add `--purity-weighted` flag (requires `--complexity-weighted`)
2. **Phase 2 (v0.3.0)**: Enable purity weighting by default with complexity weighting
3. **Phase 3 (v0.4.0)**: Remove legacy flags, fully integrated

### Compatibility Considerations

- **JSON Output**: Add `purity_distribution` field without removing existing fields
- **Existing Scripts**: Continue to work with existing `weighted_count` field
- **Performance**: Minimal overhead (<3%) acceptable for accuracy improvement
