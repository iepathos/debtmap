---
number: 203
title: State Machine Pattern - Arm-Level Analysis and Actionable Recommendations
category: optimization
priority: high
status: draft
dependencies: [179, 192, 202]
created: 2025-12-04
---

# Specification 203: State Machine Pattern - Arm-Level Analysis and Actionable Recommendations

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [179, 192, 202] (State machine pattern detection enhancements)

## Context

### Current Problem

Debtmap's state machine pattern detector (spec 179, 192, 202) accurately identifies state machine patterns but produces **misleading recommendations** that:

1. **Count all match arms equally** without distinguishing:
   - Already-extracted handler functions (`Commands::Analyze => handle_analyze_command()?`)
   - Trivial inline logic (`Commands::Init { force } => init_config(force)?`)
   - Complex inline handlers needing extraction (`Commands::Validate { 15 fields } => { 40 lines }`)
   - Nested match arms (format conversions, not primary state transitions)

2. **Recommend extracting "transitions" that are already extracted**:
   ```
   Extract 12 state transitions into named functions
   ```
   When the actual breakdown is:
   - 6 primary command arms (main state machine)
   - 2 already delegated to handlers
   - 1 trivial (4 lines)
   - 3 complex inline (need extraction)
   - 6 nested format conversion arms (not state transitions)

3. **Over-estimate complexity reduction**:
   ```
   Extracting transitions will reduce complexity from 23/39 to ~7/23
   ```
   This assumes the main match disappears, but extracting handlers only moves code—the match statement with N arms still contributes cyclomatic complexity of ≥N-1.

### Real-World Example: `main()` Function

**Current output**:
```
#8 SCORE: 20.4 [CRITICAL]
├─ PATTERN: 🔄 State Machine (transitions: 12, matches: 3, confidence: 0.70)
├─ WHY THIS MATTERS: State machine pattern detected with 12 enum pattern
   transitions across 3 match expressions. Extracting transitions will
   reduce complexity from 23/39 to ~7/23.
├─ RECOMMENDED ACTION: Extract 12 state transitions into named functions
```

**Actual code structure**:
```rust
fn main() -> Result<()> {
    // ... setup (10 lines) ...

    match cli.command {
        // ✅ Already extracted (2 arms)
        Commands::Analyze { .. } => handle_analyze_command(command)?,
        Commands::Compare { .. } => handle_compare_command(...)?,

        // ⚠️ Trivial - doesn't need extraction (1 arm)
        Commands::Init { force } => {
            debtmap::commands::init::init_config(force)?;
            Ok(())
        }

        // ❌ Complex inline - NEEDS extraction (3 arms, 95 total lines)
        Commands::Validate { 14 fields } => {
            let validate_config = ValidateConfig { /* 14 field mappings */ };
            debtmap::commands::validate::validate_project(validate_config)?;
            Ok(())
        } // 40 lines

        Commands::ValidateImprovement { 6 fields } => {
            let config = ValidateImprovementConfig {
                format: match format {  // ⚠️ Nested match (4 arms)
                    OutputFormat::Json => ...,
                    OutputFormat::Markdown => ...,
                    OutputFormat::Terminal => ...,
                    OutputFormat::Html => ...,
                },
                // ... rest of config
            };
            validate_improvement(config)?;
            Ok(())
        } // 30 lines

        Commands::ExplainCoverage { 6 fields } => {
            let config = ExplainCoverageConfig {
                format: match format {  // ⚠️ Nested match (2 arms)
                    DebugFormatArg::Text => ...,
                    DebugFormatArg::Json => ...,
                },
                // ... rest
            };
            explain_coverage(config)?;
            Ok(())
        } // 25 lines
    }
}
```

**Expected improved output**:
```
#8 SCORE: 20.4 [CRITICAL]
├─ PATTERN: 🔄 State Machine (primary: 6 arms, nested: 6 arms, confidence: 0.70)
├─ WHY THIS MATTERS: State machine pattern with 6 commands (2 already
   extracted, 1 trivial, 3 need extraction). Inline config building in
   Validate, ValidateImprovement, and ExplainCoverage adds 95 lines.
├─ RECOMMENDED ACTION: Extract 3 command handlers (Validate,
   ValidateImprovement, ExplainCoverage) to match existing pattern
├─ IMPACT: -11 complexity (estimated: 23/39 → 14/27)
```

### Root Cause Analysis

The pattern detector (`state_machine_pattern_detector.rs:119`) computes:
```rust
transition_count: visitor.enum_match_count + visitor.tuple_match_count
```

This sums ALL match arms across ALL nesting levels without:
1. Tracking which arms delegate to handlers
2. Measuring arm complexity (line count, nesting)
3. Distinguishing primary vs nested matches
4. Identifying trivial vs complex inline logic

The recommendation generator (`concise_recommendation.rs:613-691`) then:
1. Uses `transition_count` directly as "work to be done"
2. Calculates overly optimistic complexity reduction
3. Doesn't acknowledge existing good patterns

## Objective

Enhance state machine pattern detection to provide **actionable, context-aware recommendations** by:

1. **Tracking arm-level metrics** to distinguish:
   - Delegated arms (already extracted)
   - Trivial arms (< 10 lines, simple logic)
   - Complex inline arms (≥ 10 lines, needs extraction)
   - Nested match arms (not primary state transitions)

2. **Generating accurate recommendations** that:
   - Show breakdown of extraction status
   - Focus on actionable work (complex inline arms)
   - Acknowledge existing good patterns
   - Provide realistic complexity projections

3. **Improving developer trust** by:
   - Eliminating false positives ("extract what's already extracted")
   - Providing specific guidance ("extract ValidateImprovement handler")
   - Setting realistic expectations for complexity reduction

## Requirements

### Functional Requirements

#### FR1: Arm Delegation Detection
- **Detect pattern**: `Arm => function_call(args)?` or `Arm => handler()`
- **Classification**: Arms that directly call a single function with no inline logic
- **Examples**:
  ```rust
  // ✅ Delegated
  Commands::Analyze { .. } => handle_analyze_command(command)?
  Commands::Compare { .. } => handle_compare_command(before, after, ...)?

  // ❌ Not delegated (has inline logic)
  Commands::Init { force } => {
      debtmap::commands::init::init_config(force)?;
      Ok(())
  }
  ```

#### FR2: Arm Complexity Estimation
- **Metric**: Estimate lines of code in each match arm body
- **Method**: AST-based heuristic counting statements, expressions, blocks
- **Thresholds**:
  - **Trivial**: < 10 lines
  - **Moderate**: 10-20 lines
  - **Complex**: ≥ 20 lines
- **Include**: Nested match expressions, config building, multi-statement blocks

#### FR3: Primary vs Nested Match Distinction
- **Track**: Match expression nesting depth
- **Primary match**: First match in function (main state machine)
- **Nested matches**: Match expressions within arm bodies (format conversions, etc.)
- **Count separately**: Primary arms vs nested arms

#### FR4: Enhanced StateMachineSignals
Extend `StateMachineSignals` struct with breakdown:
```rust
pub struct StateMachineSignals {
    // Existing
    pub transition_count: u32,           // Total arms across all matches
    pub match_expression_count: u32,     // Number of match expressions
    pub has_enum_match: bool,
    pub has_state_comparison: bool,
    pub action_dispatch_count: u32,
    pub confidence: f64,

    // NEW - Arm classification
    pub primary_match_arms: u32,         // Arms in the primary (first) match
    pub nested_match_arms: u32,          // Arms in nested matches
    pub delegated_arms: u32,             // Arms calling handler functions
    pub trivial_arms: u32,               // Arms with < 10 lines
    pub complex_inline_arms: u32,        // Arms with ≥ 10 lines inline logic

    // NEW - Complexity metrics
    pub total_inline_lines: u32,         // Total LOC in complex inline arms
    pub avg_arm_complexity: f32,         // Average lines per arm
}
```

#### FR5: Actionable Recommendation Text
Generate recommendations that:
- Show extraction status breakdown
- Focus on complex inline arms only
- Acknowledge already-extracted arms
- Provide realistic complexity estimates
- Name specific arms when possible (≤ 5 arms)

**Template**:
```
State machine pattern with {primary_arms} {state_type}
({delegated} already extracted, {trivial} trivial, {complex} need extraction{: arm_names}).
Extracting {complex} inline handlers will reduce complexity from {current} to ~{target}.
```

**Examples**:
```
// Detailed (≤ 5 complex arms)
State machine pattern with 6 commands (2 already extracted, 1 trivial,
3 need extraction: Validate, ValidateImprovement, ExplainCoverage).
Extracting 3 handlers will reduce complexity from 23/39 to ~14/27.

// Summary (> 5 complex arms)
State machine pattern with 12 event handlers (4 already extracted, 2 trivial,
6 need extraction). Extracting 6 handlers will reduce complexity from 35/60 to ~20/40.

// Nested matches noted
State machine pattern with 8 commands across 3 match expressions
(2 already extracted, 6 inline). Nested format matches add secondary complexity.
```

#### FR6: Realistic Complexity Projection
Calculate complexity reduction as:
```rust
// Per-arm extraction impact (conservative)
let reduction_per_arm = 3; // Cyclomatic: -3 per extracted arm
let cognitive_reduction_per_arm = 5; // Cognitive: -5 per extracted arm

let total_cyclomatic_reduction = complex_inline_arms * reduction_per_arm;
let total_cognitive_reduction = complex_inline_arms * cognitive_reduction_per_arm;

// Main match still has N arms → contributes N-1 to cyclomatic
let baseline_match_complexity = primary_match_arms.saturating_sub(1);

let projected_cyclomatic = current_cyclomatic
    .saturating_sub(total_cyclomatic_reduction)
    .max(baseline_match_complexity);

let projected_cognitive = current_cognitive
    .saturating_sub(total_cognitive_reduction)
    .max(baseline_match_complexity * 2); // Rough heuristic
```

#### FR7: Return None for Clean Patterns
When `complex_inline_arms == 0`:
- Pattern is already well-factored
- Return `None` from recommendation generator
- Prevents "no action needed" items in output

### Non-Functional Requirements

#### NFR1: Backward Compatibility
- New fields in `StateMachineSignals` are additive
- Existing serialized data (if any) continues to work
- Old recommendation format remains valid during migration

#### NFR2: Performance
- Arm analysis adds < 5% overhead to pattern detection
- AST traversal reuses existing visitor pattern
- No additional file I/O or parsing

#### NFR3: Maintainability
- Arm complexity estimation heuristic is configurable
- Thresholds (trivial/complex) can be adjusted per project
- Clear separation: detection → analysis → recommendation

## Acceptance Criteria

### AC1: Delegation Pattern Detection
```rust
// Given
let code = r#"
    match command {
        Cmd::Foo => handle_foo()?,           // ✅ Delegated
        Cmd::Bar => handle_bar(x, y)?,       // ✅ Delegated
        Cmd::Baz => { simple() }             // ❌ Not delegated (block)
        Cmd::Qux => {                        // ❌ Not delegated (multi-statement)
            let cfg = build_config();
            process(cfg)?;
            Ok(())
        }
    }
"#;

// When
let signals = detector.detect_state_machine(&ast).unwrap();

// Then
assert_eq!(signals.delegated_arms, 2);
assert_eq!(signals.complex_inline_arms, 2);
```

### AC2: Arm Complexity Classification
```rust
// Given
let code = r#"
    match state {
        State::Idle => { do_thing() }                    // Trivial: 1 line
        State::Processing => {                           // Complex: 15 lines
            let config = ComplexConfig {
                field1: value1,
                field2: value2,
                // ... 10 more fields
            };
            nested_match_on_format(config.format)?;
            process_with_config(config)?;
            Ok(())
        }
    }
"#;

// When
let signals = detector.detect_state_machine(&ast).unwrap();

// Then
assert_eq!(signals.trivial_arms, 1);
assert_eq!(signals.complex_inline_arms, 1);
assert!(signals.total_inline_lines >= 15);
```

### AC3: Primary vs Nested Match Tracking
```rust
// Given
let code = r#"
    match command {                          // Primary: 3 arms
        Cmd::A => handle_a()?,
        Cmd::B => {
            let fmt = match format {         // Nested: 2 arms
                Fmt::Json => Format::Json,
                Fmt::Text => Format::Text,
            };
            handle_b(fmt)?;
            Ok(())
        }
        Cmd::C => handle_c()?,
    }
"#;

// When
let signals = detector.detect_state_machine(&ast).unwrap();

// Then
assert_eq!(signals.primary_match_arms, 3);
assert_eq!(signals.nested_match_arms, 2);
assert_eq!(signals.match_expression_count, 2);
```

### AC4: Accurate Recommendation for Mixed Extraction Status
```rust
// Given: main() example from context
let code = /* 6 commands: 2 extracted, 1 trivial, 3 complex */;

// When
let recommendation = generate_state_machine_recommendation(&signals, &metrics);

// Then
assert!(recommendation.rationale.contains("2 already extracted"));
assert!(recommendation.rationale.contains("1 trivial"));
assert!(recommendation.rationale.contains("3 need extraction"));
assert!(recommendation.primary_action.contains("Extract 3"));
assert!(!recommendation.primary_action.contains("Extract 12"));
```

### AC5: Realistic Complexity Projection
```rust
// Given
let signals = StateMachineSignals {
    primary_match_arms: 6,
    complex_inline_arms: 3,
    // ... other fields
};
let current = (23, 39); // (cyclomatic, cognitive)

// When
let recommendation = generate_state_machine_recommendation(&signals, ...);

// Then
// Complexity reduction: 3 arms * 3 = 9 cyclomatic reduction
// Baseline: 6 arms → cyclomatic ≥ 5
let expected_cyclomatic = 23 - 9; // 14
let expected_cognitive = 39 - 15; // 24 (3 arms * 5)

assert!(recommendation.rationale.contains("14"));
assert!(recommendation.rationale.contains("24"));
assert!(!recommendation.rationale.contains("~7/23")); // Old unrealistic target
```

### AC6: Clean Pattern Returns None
```rust
// Given: All arms already delegated
let code = r#"
    match command {
        Cmd::A => handle_a()?,
        Cmd::B => handle_b()?,
        Cmd::C => handle_c()?,
    }
"#;

// When
let signals = detector.detect_state_machine(&ast).unwrap();
let recommendation = generate_state_machine_recommendation(&signals, ...);

// Then
assert_eq!(signals.complex_inline_arms, 0);
assert_eq!(signals.delegated_arms, 3);
assert!(recommendation.is_none()); // No recommendation generated
```

### AC7: Integration Test - Real `main()` Function
```rust
#[test]
fn test_main_function_recommendation() {
    // Given: Actual debtmap main() function
    let main_fn = parse_file("src/main.rs").find_function("main");

    // When
    let signals = detector.detect_state_machine(&main_fn.body).unwrap();
    let recommendation = generate_state_machine_recommendation(&signals, ...);

    // Then
    assert_eq!(signals.primary_match_arms, 6);
    assert_eq!(signals.delegated_arms, 2); // Analyze, Compare
    assert_eq!(signals.trivial_arms, 1);   // Init
    assert_eq!(signals.complex_inline_arms, 3); // Validate, ValidateImprovement, ExplainCoverage
    assert_eq!(signals.nested_match_arms, 6); // 4 + 2 format conversions

    assert!(recommendation.primary_action.contains("Extract 3"));
    assert!(recommendation.rationale.contains("Validate"));
    assert!(recommendation.rationale.contains("ValidateImprovement"));
    assert!(recommendation.rationale.contains("ExplainCoverage"));
}
```

## Technical Details

### Implementation Approach

#### Phase 1: Enhance StateMachineVisitor (2-3 hours)

**File**: `src/analyzers/state_machine_pattern_detector.rs`

1. **Add ArmMetrics struct**:
```rust
#[derive(Debug, Clone)]
struct ArmMetrics {
    is_delegated: bool,      // Calls single function
    inline_lines: u32,       // Estimated LOC
    has_nested_match: bool,  // Contains nested match
    arm_index: usize,        // Position in match
    is_primary_match: bool,  // In primary vs nested match
}
```

2. **Track match nesting depth**:
```rust
struct StateMachineVisitor {
    // ... existing fields ...
    arm_metrics: Vec<ArmMetrics>,
    match_nesting_depth: u32,
    in_primary_match: bool,
}

impl<'ast> Visit<'ast> for StateMachineVisitor {
    fn visit_expr_match(&mut self, match_expr: &'ast ExprMatch) {
        let was_in_primary = self.in_primary_match;
        if self.match_nesting_depth == 0 {
            self.in_primary_match = true;
        }
        self.match_nesting_depth += 1;

        for (idx, arm) in match_expr.arms.iter().enumerate() {
            let metrics = self.analyze_arm(arm, idx);
            self.arm_metrics.push(metrics);
        }

        syn::visit::visit_expr_match(self, match_expr);
        self.match_nesting_depth -= 1;
        self.in_primary_match = was_in_primary;
    }
}
```

3. **Implement arm analysis**:
```rust
impl StateMachineVisitor {
    fn analyze_arm(&self, arm: &Arm, index: usize) -> ArmMetrics {
        let is_delegated = is_delegated_to_handler(&arm.body);
        let inline_lines = estimate_arm_lines(&arm.body);
        let has_nested_match = contains_nested_match(&arm.body);

        ArmMetrics {
            is_delegated,
            inline_lines,
            has_nested_match,
            arm_index: index,
            is_primary_match: self.in_primary_match,
        }
    }
}

fn is_delegated_to_handler(body: &Expr) -> bool {
    match body {
        // Direct call: handle_foo()?
        Expr::Try(try_expr) => matches!(*try_expr.expr, Expr::Call(_)),
        // Direct call without ?: handle_foo()
        Expr::Call(_) => true,
        // Method call: handler.process()?
        Expr::MethodCall(_) => true,
        // Block with single call statement
        Expr::Block(block) if block.block.stmts.len() == 1 => {
            matches!(
                block.block.stmts[0],
                Stmt::Expr(Expr::Call(_), _) | Stmt::Expr(Expr::Try(_), _)
            )
        }
        _ => false,
    }
}

fn estimate_arm_lines(body: &Expr) -> u32 {
    let mut counter = LineCounter::new();
    counter.visit_expr(body);
    counter.estimated_lines
}

struct LineCounter {
    estimated_lines: u32,
}

impl<'ast> Visit<'ast> for LineCounter {
    fn visit_stmt(&mut self, _stmt: &'ast Stmt) {
        self.estimated_lines += 1;
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Match(_) => self.estimated_lines += 1,
            Expr::If(_) => self.estimated_lines += 1,
            Expr::ForLoop(_) => self.estimated_lines += 1,
            Expr::While(_) => self.estimated_lines += 1,
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }
}

fn contains_nested_match(body: &Expr) -> bool {
    struct MatchFinder { found: bool }
    impl<'ast> Visit<'ast> for MatchFinder {
        fn visit_expr_match(&mut self, _: &'ast ExprMatch) {
            self.found = true;
        }
    }
    let mut finder = MatchFinder { found: false };
    finder.visit_expr(body);
    finder.found
}
```

#### Phase 2: Update StateMachineSignals (1 hour)

**File**: `src/priority/complexity_patterns.rs:78-87`

```rust
pub struct StateMachineSignals {
    // Existing fields (unchanged)
    pub transition_count: u32,
    pub match_expression_count: u32,
    pub has_enum_match: bool,
    pub has_state_comparison: bool,
    pub action_dispatch_count: u32,
    pub confidence: f64,

    // NEW fields
    pub primary_match_arms: u32,
    pub nested_match_arms: u32,
    pub delegated_arms: u32,
    pub trivial_arms: u32,
    pub complex_inline_arms: u32,
    pub total_inline_lines: u32,
    pub avg_arm_complexity: f32,
}
```

Update `detect_state_machine()` to populate new fields:
```rust
pub fn detect_state_machine(&self, block: &Block) -> Option<StateMachineSignals> {
    let mut visitor = StateMachineVisitor::new();
    visitor.visit_block(block);

    // Classify arms
    let mut primary = 0;
    let mut nested = 0;
    let mut delegated = 0;
    let mut trivial = 0;
    let mut complex = 0;
    let mut total_lines = 0;

    for arm in &visitor.arm_metrics {
        if arm.is_primary_match {
            primary += 1;
        } else {
            nested += 1;
        }

        if arm.is_delegated {
            delegated += 1;
        } else if arm.inline_lines < 10 {
            trivial += 1;
        } else {
            complex += 1;
            total_lines += arm.inline_lines;
        }
    }

    let avg_complexity = if !visitor.arm_metrics.is_empty() {
        visitor.arm_metrics.iter().map(|a| a.inline_lines).sum::<u32>() as f32
            / visitor.arm_metrics.len() as f32
    } else {
        0.0
    };

    Some(StateMachineSignals {
        transition_count: visitor.enum_match_count + visitor.tuple_match_count,
        match_expression_count: visitor.match_expression_count,
        has_enum_match: visitor.has_enum_match,
        has_state_comparison: !state_fields.is_empty(),
        action_dispatch_count: visitor.action_dispatch_count,
        confidence,
        primary_match_arms: primary,
        nested_match_arms: nested,
        delegated_arms: delegated,
        trivial_arms: trivial,
        complex_inline_arms: complex,
        total_inline_lines: total_lines,
        avg_arm_complexity: avg_complexity,
    })
}
```

#### Phase 3: Update Recommendation Generator (2-3 hours)

**File**: `src/priority/scoring/concise_recommendation.rs:613-691`

```rust
fn generate_state_machine_recommendation(
    transitions: u32,
    match_expression_count: u32,
    cyclomatic: u32,
    cognitive: u32,
    nesting: u32,
    metrics: &FunctionMetrics,
) -> Option<ActionableRecommendation> {  // Changed: now returns Option
    // Extract signals from metrics
    let signals = metrics
        .language_specific
        .as_ref()
        .and_then(|lang| match lang {
            LanguageSpecificData::Rust(rust) => rust.state_machine_signals.as_ref(),
        })?;

    // Early return if no work needed
    if signals.complex_inline_arms == 0 {
        return None;
    }

    // Build explanation
    let state_type = if metrics.name.contains("main") { "commands" }
                     else if metrics.name.contains("handle") { "states" }
                     else { "transitions" };

    let breakdown = if signals.primary_match_arms > 0 {
        format!(
            "{} {} ({} already extracted, {} trivial, {} need extraction{})",
            signals.primary_match_arms,
            state_type,
            signals.delegated_arms,
            signals.trivial_arms,
            signals.complex_inline_arms,
            if signals.complex_inline_arms <= 5 {
                // Name specific arms if we can extract them from AST
                // (requires additional context - may skip in v1)
                "".to_string()
            } else {
                "".to_string()
            }
        )
    } else {
        format!("{} state transitions", signals.transition_count)
    };

    // Calculate realistic complexity reduction
    let reduction_per_arm = 3;
    let cognitive_reduction_per_arm = 5;

    let total_cyclo_reduction = signals.complex_inline_arms * reduction_per_arm;
    let total_cog_reduction = signals.complex_inline_arms * cognitive_reduction_per_arm;

    let baseline_match = signals.primary_match_arms.saturating_sub(1);

    let projected_cyclo = cyclomatic
        .saturating_sub(total_cyclo_reduction)
        .max(baseline_match);
    let projected_cog = cognitive
        .saturating_sub(total_cog_reduction)
        .max(baseline_match * 2);

    // Generate recommendation
    let extraction_impact = RefactoringImpact::state_transition_extraction(
        signals.complex_inline_arms
    );

    let steps = vec![
        ActionStep {
            description: format!(
                "Extract {} inline {} into handler functions",
                signals.complex_inline_arms,
                if signals.complex_inline_arms == 1 { "handler" } else { "handlers" }
            ),
            impact: format!(
                "-{} complexity ({} inline LOC moved, {} impact)",
                total_cyclo_reduction,
                signals.total_inline_lines,
                extraction_impact.confidence.as_str()
            ),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Pattern: Commands::Foo { fields } => handle_foo_command(fields)?".to_string(),
                "# Move config building into handle_foo_command()".to_string(),
            ],
        },
        ActionStep {
            description: "Verify all command arms delegate to handlers".to_string(),
            impact: format!(
                "Consistent pattern: {} of {} arms delegated",
                signals.delegated_arms + signals.complex_inline_arms,
                signals.primary_match_arms
            ),
            difficulty: Difficulty::Easy,
            commands: vec![
                "cargo clippy".to_string(),
                "cargo test --all".to_string(),
            ],
        },
    ];

    let estimated_effort = (signals.complex_inline_arms as f32) * 0.75;

    Some(ActionableRecommendation {
        primary_action: format!(
            "Extract {} inline {} (state machine cleanup)",
            signals.complex_inline_arms,
            if signals.complex_inline_arms == 1 { "handler" } else { "handlers" }
        ),
        rationale: format!(
            "State machine pattern with {}. \
             Extracting {} inline handlers will reduce complexity from {}/{} to ~{}/{} \
             and establish consistent delegation pattern.",
            breakdown,
            signals.complex_inline_arms,
            cyclomatic, cognitive,
            projected_cyclo, projected_cog
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort.max(0.5)),
    })
}
```

### Architecture Changes

**Modified files**:
1. `src/analyzers/state_machine_pattern_detector.rs` - Enhanced visitor
2. `src/priority/complexity_patterns.rs` - Extended StateMachineSignals
3. `src/priority/scoring/concise_recommendation.rs` - Updated recommendation generator

**New types**:
- `ArmMetrics` (private to detector)
- `LineCounter` visitor (private to detector)

**API changes**:
- `StateMachineSignals` gains 7 new fields (backward compatible via serde defaults)
- `generate_state_machine_recommendation()` returns `Option<ActionableRecommendation>` instead of `ActionableRecommendation`

### Data Structures

```rust
// Internal to state_machine_pattern_detector.rs
#[derive(Debug, Clone)]
struct ArmMetrics {
    is_delegated: bool,
    inline_lines: u32,
    has_nested_match: bool,
    arm_index: usize,
    is_primary_match: bool,
}

// Updated public API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateMachineSignals {
    // Existing fields (unchanged)
    pub transition_count: u32,
    pub match_expression_count: u32,
    pub has_enum_match: bool,
    pub has_state_comparison: bool,
    pub action_dispatch_count: u32,
    pub confidence: f64,

    // New fields (with serde defaults for backward compat)
    #[serde(default)]
    pub primary_match_arms: u32,
    #[serde(default)]
    pub nested_match_arms: u32,
    #[serde(default)]
    pub delegated_arms: u32,
    #[serde(default)]
    pub trivial_arms: u32,
    #[serde(default)]
    pub complex_inline_arms: u32,
    #[serde(default)]
    pub total_inline_lines: u32,
    #[serde(default)]
    pub avg_arm_complexity: f32,
}
```

## Dependencies

### Prerequisites
- **Spec 179**: Coupling & Dependency Visualization (state machine detection foundation)
- **Spec 192**: State-aware coordinator detection (refined pattern detection)
- **Spec 202**: Enhanced state field detection (improved confidence scoring)

### Affected Components
- `StateMachinePatternDetector` - Core logic changes
- `ComplexityPattern::StateMachine` - Receives enhanced signals
- `generate_state_machine_recommendation()` - New logic
- Output formatters - May need updates if displaying new fields

### External Dependencies
None (uses existing `syn` crate for AST analysis)

## Testing Strategy

### Unit Tests

**File**: `src/analyzers/state_machine_pattern_detector.rs`

```rust
#[test]
fn test_delegation_detection() {
    let cases = vec![
        ("Cmd::A => handle_a()?", true),
        ("Cmd::B => handle_b(x, y)?", true),
        ("Cmd::C => { handle_c() }", false), // Block wrapper
        ("Cmd::D => { let x = 1; handle_d(x)? }", false), // Multi-statement
    ];

    for (code, expected_delegated) in cases {
        let arm = parse_arm(code);
        assert_eq!(is_delegated_to_handler(&arm.body), expected_delegated,
                   "Failed for: {}", code);
    }
}

#[test]
fn test_arm_complexity_estimation() {
    let arm = parse_arm(r#"
        Cmd::Validate { path, config } => {
            let validate_config = ValidateConfig {
                path,
                config,
                format: match format {
                    Fmt::Json => Format::Json,
                    Fmt::Markdown => Format::Markdown,
                },
                output: None,
                top: 10,
            };
            debtmap::commands::validate::validate_project(validate_config)?;
            Ok(())
        }
    "#);

    let lines = estimate_arm_lines(&arm.body);
    assert!(lines >= 10, "Expected >= 10 lines, got {}", lines);
}

#[test]
fn test_primary_vs_nested_match_tracking() {
    let code = r#"
        fn test() {
            match cmd {
                Cmd::A => handle_a()?,
                Cmd::B => {
                    let fmt = match format {
                        Fmt::Json => json(),
                        Fmt::Text => text(),
                    };
                    handle_b(fmt)?
                }
            }
        }
    "#;

    let func = parse_function(code);
    let signals = detector.detect_state_machine(&func.body).unwrap();

    assert_eq!(signals.primary_match_arms, 2);
    assert_eq!(signals.nested_match_arms, 2);
}
```

### Integration Tests

**File**: `tests/state_machine_arm_analysis_test.rs`

```rust
#[test]
fn test_main_function_analysis() {
    let source = std::fs::read_to_string("src/main.rs").unwrap();
    let file = syn::parse_file(&source).unwrap();
    let main_fn = file.items.iter()
        .find_map(|item| match item {
            syn::Item::Fn(f) if f.sig.ident == "main" => Some(f),
            _ => None,
        })
        .expect("main function not found");

    let detector = StateMachinePatternDetector::new();
    let signals = detector.detect_state_machine(&main_fn.block)
        .expect("Should detect state machine in main()");

    // Verify breakdown
    assert_eq!(signals.primary_match_arms, 6, "6 Commands");
    assert_eq!(signals.delegated_arms, 2, "Analyze, Compare");
    assert_eq!(signals.trivial_arms, 1, "Init is trivial");
    assert_eq!(signals.complex_inline_arms, 3,
               "Validate, ValidateImprovement, ExplainCoverage");
    assert!(signals.nested_match_arms >= 6, "Format conversion matches");

    // Verify recommendation
    let metrics = /* build FunctionMetrics from main_fn */;
    let rec = generate_state_machine_recommendation(
        signals.transition_count,
        signals.match_expression_count,
        23, 39, 2,
        &metrics
    );

    assert!(rec.is_some(), "Should generate recommendation");
    let rec = rec.unwrap();

    assert!(rec.primary_action.contains("Extract 3"));
    assert!(rec.rationale.contains("2 already extracted"));
    assert!(rec.rationale.contains("1 trivial"));
    assert!(rec.rationale.contains("3 need extraction"));
}

#[test]
fn test_clean_state_machine_no_recommendation() {
    let code = r#"
        fn dispatch(cmd: Command) -> Result<()> {
            match cmd {
                Command::Start => handle_start()?,
                Command::Stop => handle_stop()?,
                Command::Restart => handle_restart()?,
            }
            Ok(())
        }
    "#;

    let func = parse_function(code);
    let signals = detector.detect_state_machine(&func.body).unwrap();

    assert_eq!(signals.complex_inline_arms, 0);
    assert_eq!(signals.delegated_arms, 3);

    let rec = generate_state_machine_recommendation(...);
    assert!(rec.is_none(), "Clean pattern should not generate recommendation");
}
```

### Regression Tests

Ensure existing state machine tests still pass:
```bash
cargo test state_machine_pattern_detection_test
```

Verify no regressions in:
- False positive rate (clean dispatchers)
- Confidence scoring
- Coordinator vs state machine distinction

## Documentation Requirements

### Code Documentation

1. **Module-level docs** (`state_machine_pattern_detector.rs`):
   ```rust
   //! # State Machine Pattern Detection with Arm-Level Analysis
   //!
   //! Detects state machine patterns and analyzes each match arm to provide
   //! actionable recommendations:
   //!
   //! - **Delegation detection**: Identifies arms already extracted to handlers
   //! - **Complexity estimation**: Measures inline logic per arm
   //! - **Primary vs nested tracking**: Distinguishes main state machine from
   //!   nested format conversions
   //!
   //! ## Example
   //!
   //! ```rust
   //! let signals = detector.detect_state_machine(&func_body)?;
   //! println!("Complex inline arms: {}", signals.complex_inline_arms);
   //! println!("Already delegated: {}", signals.delegated_arms);
   //! ```
   ```

2. **Function docs** for public methods:
   ```rust
   /// Detect state machine pattern with arm-level analysis.
   ///
   /// Returns enhanced signals including:
   /// - Arm classification (delegated, trivial, complex)
   /// - Primary vs nested match breakdown
   /// - Complexity metrics per arm
   ///
   /// # Returns
   ///
   /// `None` if no state machine pattern detected (confidence < 0.5).
   /// `Some(signals)` with detailed arm analysis.
   ```

### User Documentation

**Update**: `book/src/analysis-guide/interpreting-results.md`

Add section:
```markdown
### State Machine Pattern Recommendations

Debtmap analyzes state machines (large `match` expressions) to provide
actionable extraction guidance:

**Example Output**:
```
State machine pattern with 6 commands (2 already extracted, 1 trivial,
3 need extraction: Validate, ValidateImprovement, ExplainCoverage).
Extracting 3 handlers will reduce complexity from 23/39 to ~14/27.
```

**Breakdown**:
- **2 already extracted**: Arms that delegate to handler functions ✅
- **1 trivial**: Simple arms (< 10 lines) that don't need extraction
- **3 need extraction**: Complex inline arms (≥ 10 lines) to refactor

**Recommendation acknowledges existing good patterns** while focusing on
actionable improvements.
```

### Architecture Updates

**Update**: `ARCHITECTURE.md`

Add to "Pattern Detection" section:
```markdown
#### Arm-Level Analysis (Spec 203)

State machine detector performs **arm-level analysis** to distinguish:

1. **Delegated arms**: `Cmd::Foo => handle_foo()?` (already extracted)
2. **Trivial arms**: `Cmd::Bar => { simple_call() }` (< 10 lines)
3. **Complex inline arms**: Multi-statement config building (≥ 10 lines)
4. **Nested matches**: Format conversions within arms (not primary transitions)

This enables **context-aware recommendations** that:
- Acknowledge existing good patterns
- Focus on actionable improvements
- Provide realistic complexity projections
```

## Implementation Notes

### Edge Cases

1. **Single-arm match**: Not a state machine, should return `None` in detector
2. **All arms delegated**: `complex_inline_arms == 0` → return `None` in recommendation
3. **Deeply nested matches** (> 3 levels): Track depth, may adjust confidence
4. **Macro-generated arms**: AST may not reflect source structure, best-effort estimation

### Configuration (Future Enhancement)

Consider adding to `.debtmap.toml`:
```toml
[pattern_detection.state_machine]
trivial_arm_threshold = 10    # Lines below this are "trivial"
complex_arm_threshold = 20    # Lines above this are "complex"
delegation_patterns = [
    "handle_{}_command",      # Custom handler naming patterns
    "process_{}",
]
```

### Performance Considerations

- **Arm analysis overhead**: ~5% increase in pattern detection time
- **Caching**: Consider caching arm metrics if function is analyzed multiple times
- **Large matches**: Functions with > 50 arms may need optimization (rare)

### Gotchas

1. **Formatting affects line estimates**: Use AST node count, not source line spans
2. **Delegation detection is heuristic**: May miss complex wrappers
3. **Nested match counting**: Avoid double-counting arms in nested contexts

## Migration and Compatibility

### Breaking Changes

**None** - This is a backward-compatible enhancement:
- New fields in `StateMachineSignals` have `#[serde(default)]`
- Old serialized data deserializes with default values (0)
- `generate_state_machine_recommendation()` signature changes to return `Option`, but this is internal

### Migration Path

1. **Deploy spec 203**
2. **Observe output changes**:
   - Fewer "extract N transitions" recommendations (clean patterns filtered)
   - More accurate complexity projections
   - Breakdown shows extraction status
3. **No user action required** - recommendations auto-improve

### Compatibility Testing

```bash
# Verify no regressions in test suite
cargo test --all

# Check existing state machine detections still work
cargo run -- analyze . --format json > new.json
diff <(jq '.items[] | select(.pattern == "StateMachine")' old.json) \
     <(jq '.items[] | select(.pattern == "StateMachine")' new.json)
```

## Success Metrics

### Quantitative

- **False positive reduction**: Clean state machines (all arms delegated) generate 0 recommendations
- **Accuracy improvement**: Complexity projections within ±20% of actual reduction
- **Recommendation clarity**: 100% of state machine recommendations include extraction status breakdown

### Qualitative

- **Developer trust**: Users report recommendations "match reality"
- **Actionability**: Developers can act on recommendations without re-analyzing code
- **Specificity**: Recommendations name specific arms to extract (when ≤ 5)

### Before/After Comparison

**Before Spec 203**:
```
Extract 12 state transitions into named functions
Complexity: 23/39 → ~7/23
```

**After Spec 203**:
```
Extract 3 inline handlers (Validate, ValidateImprovement, ExplainCoverage)
State machine with 6 commands (2 already extracted, 1 trivial, 3 need extraction)
Complexity: 23/39 → ~14/27
```

**Improvement**: Clear, actionable, acknowledges existing work, realistic targets.
