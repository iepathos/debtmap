---
number: 149
title: Fix Generic Fallback for Module Functions
category: foundation
priority: critical
status: draft
dependencies: [141, 142, 145]
created: 2025-10-28
---

# Specification 149: Fix Generic Fallback for Module Functions

**Category**: foundation
**Priority**: critical
**Status**: ready
**Dependencies**: Multi-signal aggregation infrastructure (already implemented in `src/analysis/multi_signal_aggregation.rs`)

**Note**: This spec references conceptual "Specs 141-145" which don't exist as standalone specification documents but are already implemented in the codebase:
- Spec 141 (I/O Detection) → `src/analysis/io_detection.rs` (IoDetector)
- Spec 142 (Call Graph) → `src/analysis/call_graph.rs` (RustCallGraph)
- Spec 143 (Purity Analysis) → `src/organization/purity_analyzer.rs` (PurityAnalyzer)
- Spec 145 (Multi-Signal Aggregation) → `src/analysis/multi_signal_aggregation.rs` (ResponsibilityAggregator)

## Context

In the latest debtmap output, **formatter.rs** (#2 in recommendations) shows a regression to generic splits:

```
#2 SCORE: 102 [CRITICAL - FILE - GOD MODULE]
└─ ./src/priority/formatter.rs (2895 lines, 116 functions)
└─ ACTION: Split into: 1) Core formatter 2) Section writers...

  - SUGGESTED SPLIT (generic - no detailed analysis available):
  -  [1] formatter_core.rs - Core business logic
  -  [2] formatter_io.rs - Input/output operations
  -  [3] formatter_utils.rs - Helper functions
```

**This is marked as "generic - no detailed analysis available"**, which means:
1. Multi-signal classification (Specs 141-145) is not being applied
2. System fell back to generic heuristic-based splits
3. Missing responsibility-based analysis for module-level functions

**Root Cause**: Module-level functions (not methods on structs) are not being routed through multi-signal classification pipeline. The system has separate code paths:
- **Struct methods** → Multi-signal analysis ✅ (working, see #1, #6, #8)
- **Module functions** → Generic fallback ❌ (broken, see #2, #3, #7, #9)

This is a **critical gap** because many Rust modules use free functions rather than struct methods, especially for:
- Formatters and parsers
- Utility modules
- Functional-style code
- Pure computation modules

## Objective

Fix the code path for module-level functions to route through multi-signal classification (Specs 141-145), ensuring they receive the same analysis quality as struct methods. Eliminate generic fallback splits in favor of responsibility-based, evidence-driven recommendations.

## Requirements

### Functional Requirements

**Module Function Classification**:
- Apply I/O detection (Spec 141) to module functions
- Apply call graph analysis (Spec 142) to module functions
- Apply type signature analysis (Spec 147) to module functions
- Apply purity analysis (Spec 143) to module functions
- Route through multi-signal aggregation (Spec 145)

**Fallback Elimination**:
- Remove "generic - no detailed analysis available" fallback
- Ensure all modules get multi-signal classification
- Only use fallback when explicitly configured or for edge cases

**Specific Fixes**:
- **formatter.rs**: Should detect formatting functions (T → String patterns)
- **god_object_detector.rs**: Should detect orchestration, analysis, validation patterns
- **semantic_classifier.rs**: Should detect classification, validation, rule-matching patterns
- **python_exception_flow.rs**: Should detect parsing, analysis, flow tracking patterns

**Error Handling**:
- If multi-signal classification fails, log error (don't silent fallback)
- Provide diagnostic info about why classification failed
- Allow explicit fallback override for debugging

### Non-Functional Requirements

- **Performance**: Module function classification same cost as struct methods
- **Accuracy**: Module functions achieve same >85% accuracy as struct methods
- **Consistency**: Same evidence quality for module functions and methods
- **Debugging**: Clear logging when fallback occurs and why

## Acceptance Criteria

- [ ] formatter.rs (#2) receives multi-signal classification (no generic fallback)
- [ ] god_object_detector.rs (#3) receives responsibility-based splits
- [ ] semantic_classifier.rs (#7) receives multi-signal classification
- [ ] python_exception_flow.rs (#9) receives multi-signal classification
- [ ] "generic - no detailed analysis available" message eliminated from output
- [ ] Module functions and struct methods produce equivalent evidence quality
- [ ] Fallback only occurs with explicit configuration or logged errors
- [ ] Test suite covers both module functions and struct methods
- [ ] Performance regression <5% compared to current implementation
- [ ] All integration tests pass with new classification path

## Technical Details

### Implementation Approach

**Phase 1: Identify Code Path Gap**

Current code (hypothetical, based on output):

```rust
// In src/organization/god_object_detector.rs or similar

pub fn recommend_splits(file: &FileAnalysis) -> Vec<ModuleSplit> {
    if file.has_structs_with_many_methods() {
        // Path 1: Struct-based analysis
        let struct_analysis = analyze_struct_methods(file);
        classify_with_multi_signal(struct_analysis)  // ✅ WORKS
    } else {
        // Path 2: Module function fallback
        generic_fallback_splits(file)  // ❌ BROKEN - Missing multi-signal
    }
}

fn generic_fallback_splits(file: &FileAnalysis) -> Vec<ModuleSplit> {
    // OLD HEURISTIC-BASED CODE
    vec![
        ModuleSplit::new("core", "Core business logic"),
        ModuleSplit::new("io", "Input/output operations"),
        ModuleSplit::new("utils", "Helper functions"),
    ]
}
```

**Phase 2: Unified Classification Path**

New unified approach:

```rust
pub fn recommend_splits(file: &FileAnalysis) -> Vec<ModuleSplit> {
    // Collect all functions (both module-level and methods)
    let all_functions = collect_all_functions(file);

    // UNIFIED PATH: Route all functions through multi-signal
    let classifications = all_functions.iter()
        .map(|func| classify_function_with_multi_signal(func))
        .collect::<Vec<_>>();

    // Group by classified responsibility
    let responsibility_groups = group_by_responsibility(&classifications);

    // Generate splits based on actual responsibilities
    generate_responsibility_based_splits(responsibility_groups)
}

/// Data structure for function analysis (needs to be added/extended)
#[derive(Debug, Clone)]
pub struct FunctionAnalysis {
    pub name: String,
    pub body: String,  // Function body as string for multi-signal analysis
    pub return_type: Option<String>,
    pub parameters: Vec<Parameter>,
    pub line_count: usize,
    pub start_line: usize,
    pub is_public: bool,
    pub is_async: bool,
    pub ast_node: Option<syn::ItemFn>,  // Original AST node for detailed analysis
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_name: String,
}

/// Extract module-level functions from syn::File AST
fn extract_module_functions(ast: &syn::File) -> Vec<FunctionAnalysis> {
    use syn::visit::Visit;

    let mut functions = Vec::new();

    for item in &ast.items {
        if let syn::Item::Fn(item_fn) = item {
            functions.push(FunctionAnalysis {
                name: item_fn.sig.ident.to_string(),
                body: quote::quote!(#item_fn).to_string(),  // Convert AST to string
                return_type: extract_return_type(&item_fn.sig.output),
                parameters: extract_parameters(&item_fn.sig.inputs),
                line_count: estimate_line_count(&item_fn.block),
                start_line: 0,  // TODO: Extract from span
                is_public: matches!(item_fn.vis, syn::Visibility::Public(_)),
                is_async: item_fn.sig.asyncness.is_some(),
                ast_node: Some(item_fn.clone()),
            });
        }
    }

    functions
}

fn collect_all_functions(file: &FileAnalysis) -> Vec<FunctionAnalysis> {
    let mut functions = Vec::new();

    // Module-level functions
    functions.extend(file.module_functions.iter().cloned());

    // Struct methods
    for struct_data in &file.structs {
        functions.extend(struct_data.methods.iter().cloned());
    }

    // Trait implementations
    for impl_block in &file.impls {
        functions.extend(impl_block.methods.iter().cloned());
    }

    functions
}

/// Classified function with evidence (needs to be added)
#[derive(Debug, Clone)]
pub struct ClassifiedFunction {
    pub function: FunctionAnalysis,
    pub classification: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: AggregatedClassification,
}

fn classify_function_with_multi_signal(func: &FunctionAnalysis) -> ClassifiedFunction {
    // Create aggregator (already exists in codebase)
    let aggregator = ResponsibilityAggregator::new();

    // Build signal set using existing infrastructure
    let signals = SignalSet {
        io_signal: aggregator.collect_io_signal(&func.body, Language::Rust),
        purity_signal: aggregator.collect_purity_signal(&func.body, Language::Rust),
        type_signal: aggregator.collect_type_signature_signal(
            &func.return_type,
            &func.parameters.iter().map(|p| p.type_name.as_str()).collect::<Vec<_>>(),
        ),
        name_signal: Some(aggregator.collect_name_signal(&func.name)),
        call_graph_signal: None,  // TODO: Requires call graph context
        framework_signal: None,   // TODO: Requires file context
    };

    // Aggregate signals (already implemented)
    let evidence = aggregator.aggregate(&signals);

    ClassifiedFunction {
        function: func.clone(),
        classification: evidence.primary,
        confidence: evidence.confidence,
        evidence,
    }
}

fn group_by_responsibility(
    classifications: &[ClassifiedFunction]
) -> HashMap<ResponsibilityCategory, Vec<ClassifiedFunction>> {
    let mut groups: HashMap<ResponsibilityCategory, Vec<ClassifiedFunction>> = HashMap::new();

    for classified in classifications {
        groups.entry(classified.classification)
            .or_insert_with(Vec::new)
            .push(classified.clone());
    }

    groups
}

fn generate_responsibility_based_splits(
    groups: HashMap<ResponsibilityCategory, Vec<ClassifiedFunction>>
) -> Vec<ModuleSplit> {
    let mut splits = Vec::new();

    for (responsibility, functions) in groups {
        // Skip if too few functions
        if functions.len() < 3 {
            continue;
        }

        // Calculate aggregate confidence
        let avg_confidence: f64 = functions.iter()
            .map(|f| f.confidence)
            .sum::<f64>() / functions.len() as f64;

        splits.push(ModuleSplit {
            suggested_name: format!("{}_module", responsibility.to_snake_case()),
            responsibility: responsibility.as_str().to_string(),
            methods_to_move: functions.iter().map(|f| f.function.name.clone()).collect(),
            structs_to_move: vec![],  // Module functions don't belong to structs
            estimated_lines: functions.iter().map(|f| f.function.line_count).sum(),
            method_count: functions.len(),
            warning: None,
            priority: calculate_priority(avg_confidence, functions.len()),
            cohesion_score: Some(avg_confidence),
            dependencies_in: vec![],  // TODO: Extract from call graph
            dependencies_out: vec![],  // TODO: Extract from call graph
            domain: String::new(),
            rationale: Some(aggregate_evidence(&functions)),
            method: SplitAnalysisMethod::MethodBased,
            severity: None,
            interface_estimate: None,
        });
    }

    // Sort by confidence and size
    splits.sort_by(|a, b| {
        b.confidence.partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.functions.len().cmp(&a.functions.len()))
    });

    splits
}

/// Helper: Calculate priority based on confidence and function count
fn calculate_priority(confidence: f64, function_count: usize) -> Priority {
    if confidence > 0.70 && function_count > 10 {
        Priority::High
    } else if confidence > 0.50 || function_count > 5 {
        Priority::Medium
    } else {
        Priority::Low
    }
}

/// Helper: Aggregate evidence from multiple classified functions
fn aggregate_evidence(functions: &[ClassifiedFunction]) -> String {
    use std::collections::HashMap;

    let mut signal_counts: HashMap<&str, usize> = HashMap::new();
    let mut total_confidence = 0.0;

    for func in functions {
        for evidence in &func.evidence.evidence {
            *signal_counts.entry(evidence.description.as_str()).or_insert(0) += 1;
        }
        total_confidence += func.confidence;
    }

    let avg_confidence = total_confidence / functions.len() as f64;

    // Find most common signals
    let mut signal_list: Vec<_> = signal_counts.into_iter().collect();
    signal_list.sort_by(|a, b| b.1.cmp(&a.1));

    let top_signals: Vec<String> = signal_list
        .iter()
        .take(3)
        .map(|(signal, count)| format!("{} ({} functions)", signal, count))
        .collect();

    format!(
        "Avg confidence: {:.2}. Top signals: {}",
        avg_confidence,
        top_signals.join(", ")
    )
}
```

**Phase 3: Formatter-Specific Detection**

For `formatter.rs`, apply type signature patterns:

```rust
fn classify_formatter_functions(func: &FunctionAnalysis) -> Option<TypeBasedClassification> {
    // Detect T → String pattern
    if func.return_type.as_ref().map(|rt| rt.contains("String")).unwrap_or(false) {
        return Some(TypeBasedClassification {
            category: ResponsibilityCategory::Formatting,
            confidence: 0.80,
            evidence: format!(
                "Returns String, likely formatting function: {} → String",
                func.parameters.first().map(|p| &p.type_name).unwrap_or(&"T".to_string())
            ),
            pattern_name: "Formatter Pattern".into(),
        });
    }

    // Detect &mut Write pattern
    if func.parameters.iter().any(|p| {
        p.type_name.contains("Write") || p.type_name.contains("Formatter")
    }) {
        return Some(TypeBasedClassification {
            category: ResponsibilityCategory::Formatting,
            confidence: 0.85,
            evidence: "Takes &mut Write/Formatter, formatting function".into(),
            pattern_name: "Writer Pattern".into(),
        });
    }

    None
}
```

**Phase 4: Fallback Error Handling**

Only fallback with explicit logging:

```rust
pub fn recommend_splits_with_fallback(file: &FileAnalysis) -> Vec<ModuleSplit> {
    match recommend_splits(file) {
        Ok(splits) if !splits.is_empty() => splits,
        Ok(_) => {
            log::warn!(
                "No responsibility-based splits found for {}, attempting fallback",
                file.path.display()
            );
            fallback_with_diagnostics(file)
        }
        Err(e) => {
            log::error!(
                "Multi-signal classification failed for {}: {}",
                file.path.display(),
                e
            );
            fallback_with_diagnostics(file)
        }
    }
}

fn fallback_with_diagnostics(file: &FileAnalysis) -> Vec<ModuleSplit> {
    log::debug!("Fallback diagnostics for {}:", file.path.display());
    log::debug!("  - Module functions: {}", file.module_functions.len());
    log::debug!("  - Structs: {}", file.structs.len());
    log::debug!("  - Total functions: {}", count_total_functions(file));

    // Only use generic fallback if explicitly configured
    if ALLOW_GENERIC_FALLBACK {
        generic_fallback_splits(file)
    } else {
        // Return empty splits with warning in output
        vec![ModuleSplit::warning(
            "Unable to generate responsibility-based splits. \
             File may need manual analysis or contains edge case patterns."
        )]
    }
}
```

### AST Integration Details

**Extracting Module Functions from syn::File**:

The current `TypeVisitor` in `src/organization/god_object/mod.rs` primarily tracks struct methods. We need to extend it to also capture module-level functions:

```rust
// Add to TypeVisitor in src/organization/god_object/mod.rs
impl<'ast> Visit<'ast> for TypeVisitor {
    // Existing impl for syn::ItemStruct, syn::ItemImpl...

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        // Extract module-level function
        let function_info = FunctionInfo {
            name: node.sig.ident.to_string(),
            visibility: extract_visibility(&node.vis),
            is_async: node.sig.asyncness.is_some(),
            parameters: extract_sig_parameters(&node.sig.inputs),
            return_type: extract_sig_return_type(&node.sig.output),
            body: quote::quote!(#node).to_string(),
            line_count: estimate_block_lines(&node.block),
        };

        self.module_functions.push(function_info);

        // Continue visiting nested items
        syn::visit::visit_item_fn(self, node);
    }
}

// Helper functions for AST extraction
fn extract_visibility(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn extract_sig_parameters(inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>) -> Vec<Parameter> {
    inputs.iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                Some(Parameter {
                    name: extract_pat_name(&pat_type.pat),
                    type_name: quote::quote!(#pat_type.ty).to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn extract_sig_return_type(output: &syn::ReturnType) -> Option<String> {
    match output {
        syn::ReturnType::Type(_, ty) => Some(quote::quote!(#ty).to_string()),
        syn::ReturnType::Default => None,
    }
}

fn extract_pat_name(pat: &syn::Pat) -> String {
    match pat {
        syn::Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
        _ => "unknown".to_string(),
    }
}

fn estimate_block_lines(block: &syn::Block) -> usize {
    // Simple estimation: count statements + 2 for braces
    block.stmts.len() + 2
}
```

**Converting AST to String for Analysis**:

The `ResponsibilityAggregator` methods expect function bodies as strings. We use the `quote` crate to convert AST nodes:

```rust
use quote::ToTokens;

// Convert function item to string
let body_str = node.to_token_stream().to_string();

// For better formatting, use prettyplease (optional)
let formatted = prettyplease::unparse(&syn::parse_quote! {
    #node
});
```

**Integration Point**:

The `GodObjectDetector::analyze_enhanced` method needs to populate `module_functions`:

```rust
// In src/organization/god_object_detector.rs
pub fn analyze_enhanced(&self, path: &Path, ast: &syn::File) -> EnhancedGodObjectAnalysis {
    let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
    visitor.visit_file(ast);  // This now extracts both struct methods AND module functions

    // Rest of analysis uses visitor.module_functions...
}
```

### Architecture Changes

**Modified Module**: `src/organization/god_object_detector.rs`
- Remove separate code paths for structs vs modules
- Unify classification through multi-signal pipeline
- Add diagnostic logging for fallback cases

**Modified Module**: `src/organization/god_object_analysis.rs`
- Extend `recommend_module_splits()` to handle all functions
- Remove `is_god_object` gate before multi-signal classification
- Add confidence thresholds for split generation

**New Module**: `src/organization/unified_classification.rs` (optional refactor)
- Extract unified classification logic
- Centralize function collection and grouping
- Provide single entry point for all classification

**Configuration**: Add fallback control
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ClassificationConfig {
    pub allow_generic_fallback: bool,  // Default: false
    pub min_functions_for_split: usize,  // Default: 3
    pub min_confidence_for_split: f64,   // Default: 0.50
}
```

### Fallback Policy

**Goal**: Eliminate "generic - no detailed analysis available" message from all normal operation.

**When Fallback is NEVER Acceptable**:
- Module with 10+ functions → MUST use multi-signal classification
- File being reported as god object → MUST have evidence-based splits
- Any file in production analysis output → NO generic fallbacks visible to users

**When Low-Confidence Evidence-Based Splits Are Used Instead**:
If multi-signal classification produces low confidence (<0.50), we should:
1. **Still show the splits** with low confidence markers
2. **Include the evidence** that was available (even if weak)
3. **Add recommendation for manual review**

Example output for low-confidence case:
```
  - RECOMMENDED SPLITS (lower confidence - manual review suggested):
  -  [L] formatter_output.rs - Output Generation (32 functions) [Confidence: 0.42]
       Evidence: Weak type signals (0.35), Name patterns (0.48)
       Recommendation: Consider manual classification or additional context
```

**When Fallback May Be Acceptable (with explicit logging)**:
1. **Empty files** (0 functions) → Skip entirely, don't generate splits
2. **Test-only files** (100% test functions) → Mark as "test utilities", no split needed
3. **Single-function files** → No split needed regardless of confidence

**Diagnostic Logging Requirements**:
```rust
// When confidence is low but we proceed
if avg_confidence < 0.50 {
    log::warn!(
        "{}: Low confidence classification ({:.2}). Manual review recommended.",
        file_path.display(),
        avg_confidence
    );
}

// When no signals are available (should be rare)
if signals.all_none() {
    log::error!(
        "{}: No classification signals available. Multi-signal analysis may be broken.",
        file_path.display()
    );
    // This should trigger investigation, not silent fallback
}

// When split generation produces no groups
if responsibility_groups.is_empty() {
    log::warn!(
        "{}: No responsibility groups identified from {} functions. Check classification logic.",
        file_path.display(),
        function_count
    );
}
```

**Configuration Defaults**:
```toml
# aggregation_config.toml
[classification]
allow_generic_fallback = false  # NEVER allow generic splits
min_functions_for_split = 3     # Need at least 3 functions to suggest a split
min_confidence_for_split = 0.30 # Show splits even with low confidence (with warning)
min_confidence_for_high_priority = 0.70  # High confidence threshold for priority marking
```

**User-Facing Behavior**:
- **High confidence (>0.70)**: Show splits normally with [H] or [M] markers
- **Medium confidence (0.50-0.70)**: Show splits with [M] or [L] markers
- **Low confidence (0.30-0.50)**: Show splits with [L] marker + "manual review recommended"
- **Very low confidence (<0.30)**: Don't show splits, log diagnostic message internally

**No Generic Fallback Path**:
Remove this entirely from formatter.rs:1050:
```rust
// DELETE THIS:
writeln!(
    output,
    "  {} SUGGESTED SPLIT (generic - no detailed analysis available):",
    "-".yellow()
)
```

Replace with evidence-based output that always includes confidence scores.

## Dependencies

### Required Infrastructure (Already Implemented)

- ✅ **I/O Detection**: `src/analysis/io_detection.rs` - `IoDetector` with `detect_io()` method
- ✅ **Purity Analysis**: `src/organization/purity_analyzer.rs` - `PurityAnalyzer` with `analyze_code()` method
- ✅ **Type Signatures**: `src/analysis/type_signatures/analyzer.rs` - `TypeSignatureAnalyzer`
- ✅ **Multi-Signal Aggregation**: `src/analysis/multi_signal_aggregation.rs` - `ResponsibilityAggregator::aggregate()`
- ✅ **Call Graph**: `src/analysis/call_graph.rs` - `RustCallGraph` (optional, can be None initially)
- ✅ **Framework Detection**: `src/analysis/framework_patterns_multi/detector.rs` - `FrameworkDetector`

### Test Infrastructure (Already Exists)

- ✅ `tests/multi_signal_integration_test.rs` - Integration tests for multi-signal aggregation
- ✅ `tests/multi_signal_accuracy_test.rs` - Accuracy validation tests
- ✅ `aggregation_config.toml` - Configuration for signal weights

### Components to Modify

- **`src/organization/god_object/mod.rs`** (Priority: HIGH)
  - Extend `TypeVisitor` to capture module-level functions via `visit_item_fn()`
  - Add `module_functions: Vec<FunctionInfo>` field to visitor

- **`src/organization/god_object_detector.rs`** (Priority: HIGH)
  - Replace generic fallback path with unified classification
  - Route module functions through `ResponsibilityAggregator`
  - Remove conditional logic for struct vs. module paths (lines 210-219)

- **`src/organization/god_object_analysis.rs`** (Priority: MEDIUM)
  - Update `recommend_module_splits()` to use multi-signal classification
  - Replace `group_methods_by_responsibility()` name-based logic (line 470) with `ResponsibilityAggregator`

- **`src/priority/formatter.rs`** (Priority: HIGH)
  - Remove hardcoded "generic - no detailed analysis available" message (line 1050)
  - Replace `format_generic_split_suggestions()` with evidence-based formatting

### New Components to Add

- **`src/organization/function_classifier.rs`** (Optional - recommended for clean architecture)
  - Extract unified function classification logic
  - Centralize `collect_all_functions()`, `classify_function_with_multi_signal()`, etc.
  - Make reusable across god object and other analyzers

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_functions_use_multi_signal() {
        let file = FileAnalysis {
            path: PathBuf::from("test_module.rs"),
            module_functions: vec![
                FunctionAnalysis {
                    name: "format_output".into(),
                    return_type: Some("String".into()),
                    ..Default::default()
                },
                FunctionAnalysis {
                    name: "parse_input".into(),
                    return_type: Some("Result<Config, Error>".into()),
                    ..Default::default()
                },
            ],
            structs: vec![],
            ..Default::default()
        };

        let splits = recommend_splits(&file).unwrap();

        // Should not be generic fallback
        assert!(!splits.iter().any(|s| s.name.contains("core")));
        assert!(!splits.iter().any(|s| s.name.contains("utils")));

        // Should have responsibility-based names
        assert!(splits.iter().any(|s| {
            s.responsibility == ResponsibilityCategory::Formatting
        }));
        assert!(splits.iter().any(|s| {
            s.responsibility == ResponsibilityCategory::Parsing
        }));
    }

    #[test]
    fn formatter_detection() {
        let func = FunctionAnalysis {
            name: "format_recommendation".into(),
            return_type: Some("String".into()),
            parameters: vec![
                Parameter {
                    name: "recommendation".into(),
                    type_name: "&Recommendation".into(),
                },
            ],
            ..Default::default()
        };

        let classification = classify_function_with_multi_signal(&func);

        assert_eq!(classification.classification, ResponsibilityCategory::Formatting);
        assert!(classification.confidence > 0.70);
    }

    #[test]
    fn no_silent_fallback() {
        let file = FileAnalysis::default();

        // Configure to disallow fallback
        let config = ClassificationConfig {
            allow_generic_fallback: false,
            ..Default::default()
        };

        let splits = recommend_splits_with_config(&file, &config);

        // Should not silently return generic splits
        // Either returns empty or warning split
        if !splits.is_empty() {
            assert!(splits[0].is_warning());
        }
    }
}
```

### Integration Tests

```rust
#[test]
fn formatter_rs_gets_multi_signal_classification() {
    // Use actual debtmap formatter.rs for testing
    let file = parse_file("src/priority/formatter.rs");
    let splits = recommend_splits(&file).unwrap();

    // Should NOT be generic
    assert!(
        !splits.iter().any(|s| s.name == "formatter_core"),
        "Should not use generic 'core' fallback"
    );

    // Should have responsibility-based names
    assert!(
        splits.iter().any(|s| {
            s.responsibility == ResponsibilityCategory::Formatting ||
            s.responsibility == ResponsibilityCategory::OutputGeneration
        }),
        "Should detect formatting responsibility"
    );

    // Should have evidence
    for split in &splits {
        assert!(split.evidence.is_some(), "All splits should have evidence");
        assert!(split.confidence > 0.50, "Should have reasonable confidence");
    }
}

#[test]
fn all_god_modules_use_multi_signal() {
    let test_files = vec![
        "src/priority/formatter.rs",
        "src/organization/god_object_detector.rs",
        "src/priority/semantic_classifier.rs",
        "src/analyzers/python_exception_flow.rs",
    ];

    for file_path in test_files {
        let file = parse_file(file_path);
        let splits = recommend_splits(&file).unwrap();

        // None should use generic fallback
        assert!(
            !splits.iter().any(|s| s.name.contains("_core") || s.name.contains("_utils")),
            "File {} should not use generic fallback",
            file_path
        );

        // All should have confidence scores
        for split in &splits {
            assert!(
                split.confidence > 0.0,
                "Split {} in {} should have confidence score",
                split.name,
                file_path
            );
        }
    }
}
```

## Documentation Requirements

### User Documentation

Update README.md:
```markdown
## Classification Accuracy

Debtmap applies the same multi-signal classification to:
- Struct methods (impl blocks)
- Module-level functions
- Trait implementations

All functions receive:
- I/O pattern detection
- Call graph analysis
- Type signature analysis
- Purity analysis
- Framework/language pattern detection

**No generic fallbacks** - every recommendation is evidence-based.
```

### Developer Documentation

Update ARCHITECTURE.md:
```markdown
## Unified Classification Pipeline

1. **Function Collection**: Gather all functions (module, methods, traits)
2. **Multi-Signal Analysis**: Apply all detectors (I/O, call graph, types, etc.)
3. **Aggregation**: Weighted voting to determine responsibility
4. **Grouping**: Group functions by classified responsibility
5. **Split Generation**: Create module splits from responsibility groups

**No separate code paths** for different function types. All functions
flow through the same classification pipeline.
```

## Implementation Notes

### Performance Considerations

Module function classification may be more expensive if files have many functions:

```rust
// Optimize for files with 100+ functions
fn classify_functions_parallel(functions: &[FunctionAnalysis]) -> Vec<ClassifiedFunction> {
    use rayon::prelude::*;

    if functions.len() > 50 {
        functions.par_iter()
            .map(classify_function_with_multi_signal)
            .collect()
    } else {
        functions.iter()
            .map(classify_function_with_multi_signal)
            .collect()
    }
}
```

### Caching Optimizations

Cache classifications to avoid recomputation:

```rust
pub struct ClassificationCache {
    cache: DashMap<FunctionId, ClassifiedFunction>,
}

impl ClassificationCache {
    pub fn get_or_classify(&self, func: &FunctionAnalysis) -> ClassifiedFunction {
        let id = FunctionId::from(func);

        self.cache.entry(id)
            .or_insert_with(|| classify_function_with_multi_signal(func))
            .clone()
    }
}
```

## Migration and Compatibility

### Breaking Changes

None - this is a bug fix that improves existing functionality.

### Backward Compatibility

Old output format with generic splits will be replaced with responsibility-based splits. Users may see different (better) recommendations, but format is compatible.

## Expected Impact

### Accuracy Improvement

**Before (current)**:
- Struct methods: ~85% accuracy (multi-signal working)
- Module functions: ~50% accuracy (generic fallback)
- **Average**: ~67% accuracy

**After (fixed)**:
- Struct methods: ~85% accuracy (unchanged)
- Module functions: ~85% accuracy (fixed)
- **Average**: ~85% accuracy

### Output Quality

**Before**:
```
#2 formatter.rs
  - SUGGESTED SPLIT (generic - no detailed analysis available):
  -  [1] formatter_core.rs
  -  [2] formatter_io.rs
  -  [3] formatter_utils.rs
```

**After**:
```
#2 formatter.rs
  - RECOMMENDED SPLITS (4 modules):
  -  [H] formatter_output.rs - Output Generation & Formatting (45 functions) [Confidence: 0.82]
       Evidence: Type pattern T → String (0.80), Call graph leaf nodes (0.75)

  -  [M] formatter_parsing.rs - Input Parsing (12 functions) [Confidence: 0.78]
       Evidence: Type pattern &str → Result<T> (0.85), I/O reads (0.70)

  -  [M] formatter_validation.rs - Validation (8 functions) [Confidence: 0.72]
       Evidence: Type pattern T → Result<(), E> (0.80), Pure functions (0.65)

  -  [L] formatter_helpers.rs - Helper Functions (18 functions) [Confidence: 0.58]
       Evidence: Mixed signals, manual review recommended
```

### User Experience

- No more "generic - no detailed analysis available" messages
- Consistent quality across all recommendations
- Evidence-based splits for all files
- Higher confidence in recommendations

## Success Metrics

- [ ] Zero "generic - no detailed analysis available" in output
- [ ] formatter.rs gets responsibility-based splits (not core/io/utils)
- [ ] All god modules (#2, #3, #7, #9) get multi-signal classification
- [ ] Module function accuracy matches struct method accuracy (~85%)
- [ ] User-reported issues about generic splits drop to zero
- [ ] Classification confidence scores present for all splits

## Implementation Checklist

### Phase 1: AST Extraction (1 day)
- [ ] Add `visit_item_fn()` to `TypeVisitor` in `src/organization/god_object/mod.rs`
- [ ] Add `module_functions: Vec<FunctionInfo>` field to visitor struct
- [ ] Implement helper functions: `extract_visibility()`, `extract_sig_parameters()`, `extract_sig_return_type()`
- [ ] Test: Verify module functions are captured from test files
- [ ] Commit: "feat: extract module-level functions in TypeVisitor"

### Phase 2: Multi-Signal Integration (2 days)
- [ ] Create `FunctionAnalysis` struct with `body: String` field
- [ ] Implement `classify_function_with_multi_signal()` using `ResponsibilityAggregator`
- [ ] Add `collect_all_functions()` to gather module functions + struct methods
- [ ] Implement `group_by_responsibility()` using multi-signal classifications
- [ ] Test: Unit test that module functions get multi-signal classification
- [ ] Commit: "feat: route module functions through multi-signal classification"

### Phase 3: Split Generation (1 day)
- [ ] Implement `generate_responsibility_based_splits()` with evidence aggregation
- [ ] Add `calculate_priority()` helper based on confidence + function count
- [ ] Implement `aggregate_evidence()` to summarize signals
- [ ] Replace `suggest_module_splits_by_domain()` calls with new unified path
- [ ] Test: Integration test with formatter.rs showing evidence-based splits
- [ ] Commit: "feat: generate responsibility-based splits with confidence scores"

### Phase 4: Remove Generic Fallback (1 day)
- [ ] Delete hardcoded "generic - no detailed analysis available" from `src/priority/formatter.rs:1050`
- [ ] Remove `format_generic_split_suggestions()` function entirely
- [ ] Add low-confidence warning display instead
- [ ] Add diagnostic logging for edge cases (empty files, test-only files)
- [ ] Update formatter to show confidence scores for all splits
- [ ] Test: Ensure no "generic" message appears in any output
- [ ] Commit: "fix: remove generic fallback, show evidence-based splits with confidence"

### Phase 5: Testing & Validation (1 day)
- [ ] Run full test suite: `cargo test --all-features`
- [ ] Test with actual problem files: formatter.rs, god_object_detector.rs, semantic_classifier.rs
- [ ] Verify confidence scores > 0.50 for all splits
- [ ] Check that evidence is displayed for each split
- [ ] Performance test: Ensure <5% regression on large files
- [ ] Manual review: Compare before/after output quality
- [ ] Commit: "test: validate multi-signal classification for module functions"

### Phase 6: Documentation (0.5 days)
- [ ] Update README.md with unified classification pipeline description
- [ ] Update ARCHITECTURE.md with flow diagram
- [ ] Add examples to docs showing evidence-based vs generic splits
- [ ] Document when low-confidence splits are shown
- [ ] Commit: "docs: document unified multi-signal classification for all functions"

**Total Estimated Time**: 5-6 days
