# Improving Actionable Guidance in Debtmap

## Current State Analysis

### What Works Well
- Clear metrics (cyclomatic, cognitive complexity)
- Quantified impact estimates (-5 complexity, -3.2 risk)
- Dependency context provided
- Severity classification (CRITICAL, HIGH, LOW)

### Critical Gaps
1. **Generic recommendations**: "Extract 3 pure functions" - but WHICH 3?
2. **No code analysis**: Doesn't identify specific code blocks to extract
3. **Missing examples**: No before/after code snippets
4. **No difficulty assessment**: Is this a 15-minute fix or 2-day refactor?
5. **Lacks context**: Doesn't explain WHY this specific refactoring helps

## Best Practices for Actionable Recommendations

### 1. Specificity Over Generality
**Current**: "Extract 3 pure functions to reduce complexity"
**Better**: "Extract validation (lines 10-25), calculation (lines 30-45), and formatting (lines 50-65) into separate functions"

### 2. Show, Don't Just Tell
**Current**: "Replace loops with map"
**Better**: Include actual code snippet showing the transformation

### 3. Progressive Disclosure
- **Quick Fix**: One-line summary for experienced devs
- **Detailed Steps**: Expand for those who need guidance
- **Learning Mode**: Link to educational content

## Proposed Enhancement System

### Level 1: Code Block Identification

```rust
// New structure for specific guidance
pub struct CodeBlockSuggestion {
    pub lines: Range<usize>,
    pub purpose: String,           // "validation", "calculation", "formatting"
    pub suggested_name: String,     // "validate_input", "calculate_total"
    pub cohesion_score: f64,       // How well these lines belong together
    pub dependencies: Vec<String>,  // Variables/functions used
    pub extraction_difficulty: ExtractionDifficulty,
}

pub enum ExtractionDifficulty {
    Trivial,    // No shared state, clear boundaries
    Simple,     // Minor variable passing needed
    Moderate,   // Some refactoring of data flow
    Complex,    // Significant restructuring required
}
```

### Level 2: Pattern-Based Recommendations

```rust
pub struct PatternBasedGuidance {
    pub detected_pattern: CodePattern,
    pub transformation: TransformationStrategy,
    pub example: CodeExample,
    pub estimated_time: Duration,
}

pub enum CodePattern {
    NestedConditionals { depth: usize },
    ImperativeLoop { operation: LoopOperation },
    MixedConcerns { concerns: Vec<Concern> },
    StateAccumulation { variables: Vec<String> },
    ErrorPropagation { style: ErrorStyle },
}

pub enum TransformationStrategy {
    ExtractGuardClauses,
    ConvertToFunctionalChain,
    SeparatePureFromImpure,
    IntroduceImmutableState,
    UseResultCombinators,
}
```

### Level 3: Contextual Examples

```rust
pub struct ContextualExample {
    pub before: CodeSnippet,
    pub after: CodeSnippet,
    pub explanation: Vec<Step>,
    pub benefits: Vec<Benefit>,
    pub gotchas: Vec<Warning>,
}

pub struct Step {
    pub action: String,
    pub reason: String,
    pub code_diff: Option<String>,
}
```

## Implementation Plan

### Phase 1: Enhanced Code Analysis (Week 1)

#### Task 1: Implement Cohesion Analysis
```rust
// In src/analysis/cohesion.rs
pub fn identify_extractable_blocks(func: &FunctionMetrics) -> Vec<CodeBlockSuggestion> {
    // Analyze variable usage patterns
    // Identify logical boundaries
    // Calculate cohesion scores
    // Generate meaningful names
}
```

#### Task 2: Pattern Detection
```rust
// In src/patterns/detector.rs
pub fn detect_refactoring_patterns(func: &FunctionMetrics) -> Vec<CodePattern> {
    // Detect nested conditionals
    // Find imperative loops
    // Identify mixed concerns
    // Locate state accumulation
}
```

### Phase 2: Smart Recommendations (Week 2)

#### Task 3: Recommendation Engine
```rust
// In src/recommendations/engine.rs
pub fn generate_specific_recommendation(
    func: &FunctionMetrics,
    patterns: Vec<CodePattern>,
    blocks: Vec<CodeBlockSuggestion>,
) -> DetailedRecommendation {
    // Match patterns to strategies
    // Select appropriate examples
    // Estimate difficulty and time
    // Generate step-by-step guide
}
```

#### Task 4: Example Library
```rust
// In src/recommendations/examples.rs
pub struct ExampleLibrary {
    examples: HashMap<(CodePattern, Language), Vec<ContextualExample>>,
}

impl ExampleLibrary {
    pub fn get_relevant_example(
        &self,
        pattern: &CodePattern,
        language: Language,
        context: &Context,
    ) -> Option<ContextualExample> {
        // Find best matching example
        // Adapt to specific context
        // Include relevant warnings
    }
}
```

### Phase 3: Interactive Guidance (Week 3)

#### Task 5: Difficulty Assessment
```rust
pub fn assess_refactoring_difficulty(
    func: &FunctionMetrics,
    suggestion: &CodeBlockSuggestion,
) -> RefactoringDifficulty {
    RefactoringDifficulty {
        time_estimate: estimate_time(suggestion),
        risk_level: assess_risk(suggestion),
        prerequisites: find_prerequisites(func),
        testing_effort: estimate_test_effort(suggestion),
    }
}
```

#### Task 6: Progressive Disclosure UI
```rust
pub enum GuidanceLevel {
    Summary,     // One-line action
    Detailed,    // Step-by-step
    Tutorial,    // Full explanation with examples
}

pub fn format_guidance(
    recommendation: &DetailedRecommendation,
    level: GuidanceLevel,
) -> String {
    match level {
        GuidanceLevel::Summary => format_summary(recommendation),
        GuidanceLevel::Detailed => format_detailed_steps(recommendation),
        GuidanceLevel::Tutorial => format_tutorial_with_examples(recommendation),
    }
}
```

## Example: Transformed Output

### Before (Current)
```
#1 SCORE: 9.5 [CRITICAL]
├─ RISK: ./src/refactoring/guidance/mod.rs:220 pattern_to_string()
├─ ACTION: Extract 3 pure functions to reduce complexity from 9 to 3
├─ IMPACT: -1.9 risk
└─ WHY: Risk score 2.4: Moderate complexity (cyclomatic: 9)
```

### After (Enhanced)
```
#1 SCORE: 9.5 [CRITICAL] ⏱️ ~25 min
├─ RISK: ./src/refactoring/guidance/mod.rs:220 pattern_to_string()
├─ ACTION: Extract pattern matching logic into 3 specialized converters
├─ IMPACT: -1.9 risk, +3 testability, -6 cognitive load
├─ DIFFICULTY: Simple (clear boundaries, no shared state)
│
├─ SPECIFIC EXTRACTIONS:
│  ├─ Lines 222-229: Extract to `convert_loop_patterns(pattern)`
│  │  └─ Handles: MapOverLoop, FilterPredicate, FoldAccumulation
│  ├─ Lines 230-235: Extract to `convert_control_patterns(pattern)`  
│  │  └─ Handles: PatternMatchOverIfElse, Pipeline, Recursion
│  └─ Lines 236-241: Extract to `convert_advanced_patterns(pattern)`
│     └─ Handles: ComposeFunctions, PartialApplication, Monadic
│
├─ QUICK FIX:
│  ```rust
│  // Replace current match with:
│  match pattern {
│      p if is_loop_pattern(p) => convert_loop_patterns(p),
│      p if is_control_pattern(p) => convert_control_patterns(p),
│      p if is_advanced_pattern(p) => convert_advanced_patterns(p),
│  }
│  ```
│
├─ BENEFITS:
│  • Each function becomes independently testable
│  • Pattern groups become explicit in code structure
│  • New patterns can be added to specific converters
│  • Reduces cognitive load from 10 to ~3 per function
│
└─ NEXT STEPS:
   1. Create the 3 helper functions above the main function
   2. Move respective match arms into each helper
   3. Add unit tests for each helper (9 test cases total)
   4. Consider using a trait if pattern types grow further
```

## Success Metrics

### Quantitative
- **Specificity Score**: % of recommendations with line-specific guidance (target: >80%)
- **Example Coverage**: % of patterns with code examples (target: >90%)
- **Time Accuracy**: Actual vs estimated refactoring time (target: ±20%)

### Qualitative
- **Developer Feedback**: Survey on actionability (target: 4.5/5)
- **Adoption Rate**: % of recommendations acted upon (target: >60%)
- **Learning Impact**: Developers report improved understanding

## Technical Implementation Details

### 1. AST Integration for Block Detection
```rust
// Integrate with tree-sitter for precise code block identification
pub fn analyze_function_blocks(ast: &Node) -> Vec<LogicalBlock> {
    let mut blocks = Vec::new();
    let mut visitor = BlockVisitor::new();
    
    walk_tree(ast, |node| {
        match node.kind() {
            "if_statement" => visitor.mark_conditional_block(node),
            "for_statement" | "while_statement" => visitor.mark_loop_block(node),
            "match_expression" => visitor.mark_pattern_block(node),
            _ => {}
        }
    });
    
    visitor.identify_cohesive_blocks()
}
```

### 2. Machine Learning for Name Suggestions
```rust
// Use embeddings to suggest meaningful function names
pub fn suggest_function_name(
    code_block: &str,
    context: &Context,
) -> Vec<SuggestedName> {
    let embeddings = generate_code_embeddings(code_block);
    let similar_functions = find_similar_in_corpus(embeddings);
    
    similar_functions.iter()
        .map(|f| extract_naming_pattern(f))
        .take(3)
        .collect()
}
```

### 3. Incremental Refactoring Steps
```rust
// Generate safe, incremental refactoring steps
pub fn generate_safe_refactoring_steps(
    func: &FunctionMetrics,
    target: &RefactoringGoal,
) -> Vec<SafeStep> {
    let mut steps = Vec::new();
    
    // Start with the safest extractions
    steps.push(SafeStep::ExtractConstants);
    steps.push(SafeStep::ExtractPureCalculations);
    
    // Then handle state management
    if has_state_mutations(func) {
        steps.push(SafeStep::IntroduceImmutableState);
    }
    
    // Finally, restructure control flow
    steps.push(SafeStep::SimplifyConditionals);
    steps.push(SafeStep::ExtractOrchestration);
    
    steps
}
```

## Conclusion

By implementing these improvements, debtmap will transform from a tool that identifies problems into an intelligent assistant that guides developers through the refactoring process. The key is moving from "what" to "how" with specific, contextual, and actionable guidance.