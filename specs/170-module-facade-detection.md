---
number: 170
title: Module Facade Detection and Scoring Adjustment
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-11-04
---

# Specification 170: Module Facade Detection and Scoring Adjustment

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently produces critical false positives when analyzing well-organized Rust codebases. Specifically, Issue #1 in the Prodigy analysis (score: 69.4, CRITICAL) incorrectly identifies `executor.rs` as a god object requiring urgent splitting, when in reality the file is already a well-structured module facade with 13 properly organized submodules.

### The Problem

The current implementation in `ModuleStructureAnalyzer` (`src/analysis/module_structure.rs`) fails to:
1. Detect and parse `#[path = "..."]` attributes that indicate external submodules
2. Distinguish between module facade files (mostly declarations) and monolithic implementations
3. Adjust scoring based on existing module organization
4. Recognize intentional architectural patterns documented in code

This results in:
- **False positives at top priority**: Well-organized code flagged as CRITICAL debt
- **Wasted developer time**: 2+ hours investigating false alarms
- **Reduced tool credibility**: #1 recommendation being demonstrably wrong undermines trust
- **Misleading severity**: "URGENT: 2257 lines, 91 functions! Split by data flow..." for already-split code

### Current Behavior (executor.rs example)

```rust
// File: executor.rs (2257 lines)
//! ## Module Organization
//! The executor is organized into focused submodules:
//! - [`data_structures`]: Core data types
//! - [`pure`]: Pure functions
//! - [`commands`]: Command execution
//! ... (13 total submodules)

#[path = "executor/builder.rs"]
mod builder;
#[path = "executor/commands.rs"]
pub(crate) mod commands;
// ... 11 more #[path] declarations

// Actual implementation: ~200 lines of facades and re-exports
pub use builder::WorkflowExecutorBuilder;
pub use types::{CaptureOutput, CommandType, StepResult};
// ...
```

**Current Analysis**:
- Total lines counted: 2257 (includes all submodule content)
- Functions counted: 91 (aggregates across all submodules)
- Score: 69.4 (CRITICAL)
- Recommendation: "Split file into modules" ❌

**Desired Analysis**:
- Facade file: ~200 lines of declarations
- Submodules: 13 external modules averaging 173 lines each
- Organization: Excellent (pure functions separated, clear responsibilities)
- Score: ~6.9 (LOW, 90% reduction)
- Recommendation: "Well-organized facade, no action required" ✅

## Objective

Implement AST-based module facade detection that accurately identifies and appropriately scores Rust files organized as module facades with external submodules, reducing false positives while maintaining accuracy on genuinely monolithic code.

## Requirements

### Functional Requirements

**FR1: AST-Based Facade Detection**
- Parse `#[path = "..."]` attributes from `syn::ItemMod` nodes
- Detect inline module declarations (`mod foo { ... }`)
- Count module-level functions vs impl block methods
- Calculate ratio of declaration lines to implementation lines
- Identify re-export patterns (`pub use`)

**FR2: Facade Scoring Algorithm**
- Calculate "facade score" (0.0 = monolithic, 1.0 = pure facade)
- Factors:
  - Submodule count (normalized to 5 modules = full factor)
  - Declaration ratio (non-implementation lines / total lines)
  - Implementation concentration (lines in impl blocks vs module-level)
- Classify organization quality: Excellent / Good / Poor / Monolithic

**FR3: Score Adjustment System**
- Reduce god object score based on facade quality
- Quality multipliers:
  - Excellent: 0.1 (90% reduction)
  - Good: 0.3 (70% reduction)
  - Poor: 0.6 (40% reduction)
  - Monolithic: 1.0 (no reduction)
- Additional factors:
  - Submodule count bonus (≥10 modules: 50% extra reduction)
  - Per-module size check (avg <300 lines: further reduction)

**FR4: Enhanced Recommendations**
- Generate facade-aware recommendations
- Show submodule structure and organization
- Provide specific guidance per quality level
- Include monitoring suggestions for well-organized code

**FR5: Backward Compatibility**
- Maintain existing API surface
- Add facade info as `Option<ModuleFacadeInfo>`
- Ensure non-Rust files continue working
- Preserve existing test coverage

### Non-Functional Requirements

**NFR1: Performance**
- Facade detection adds <5% to analysis time
- Leverage existing AST parsing (no re-parsing)
- Cache facade analysis results with module structure

**NFR2: Accuracy**
- Zero false negatives: Monolithic files still flagged as high priority
- <1% false positives: Only well-organized facades get score reduction
- Tunable thresholds for different project styles

**NFR3: Maintainability**
- Pure functions for facade detection (easily testable)
- Clear separation: detection → scoring → reporting
- Comprehensive unit tests (20+ test cases)
- Integration tests on real codebases

**NFR4: Extensibility**
- Design supports future multi-language facade detection
- Python: `from .submodule import ...`
- JavaScript/TypeScript: `export * from './submodule'`
- Pluggable scoring adjustment system

## Acceptance Criteria

### Core Detection
- [ ] `detect_module_facade()` correctly identifies files with `#[path]` declarations
- [ ] Accurately counts submodules (both `#[path]` and inline)
- [ ] Calculates facade score with correct formula
- [ ] Classifies organization quality into 4 levels
- [ ] Extracts submodule file paths and line numbers

### Scoring Adjustment
- [ ] `adjust_score_for_facade()` reduces scores based on quality
- [ ] Excellent facades: score reduced by ~90%
- [ ] Monolithic files: score unchanged
- [ ] Per-module size factored into adjustment
- [ ] Submodule count bonus applied correctly

### Integration
- [ ] `ModuleStructure` includes `facade_info: Option<ModuleFacadeInfo>`
- [ ] God object scoring calls facade adjustment
- [ ] Existing tests continue passing
- [ ] No performance regression (verified via benchmarks)

### Validation on Real Code
- [ ] Prodigy's `executor.rs` (13 submodules, 2257 lines):
  - Identified as facade: ✅
  - Facade score: >0.9
  - Organization: Excellent
  - Adjusted score: <10 (from 69.4)
  - Recommendation: No action required

- [ ] Genuinely monolithic file (e.g., single 2000-line impl block):
  - Identified as monolithic: ✅
  - Facade score: <0.2
  - Organization: Monolithic
  - Score: Unchanged (high)
  - Recommendation: Split into modules

- [ ] Partially organized file (3 submodules, 1500 lines):
  - Identified as partial facade: ✅
  - Facade score: 0.5-0.7
  - Organization: Good or Poor
  - Score: Moderate reduction (40-70%)
  - Recommendation: Consider further splitting

### Documentation and Testing
- [ ] Unit tests for `detect_module_facade()` with 10+ scenarios
- [ ] Unit tests for `adjust_score_for_facade()` with edge cases
- [ ] Integration tests on 5+ real codebases
- [ ] Documentation in code and user guide
- [ ] Performance benchmarks showing <5% overhead

## Technical Details

### Implementation Approach

#### Phase 1: Data Structures (src/analysis/module_structure.rs)

```rust
/// Module facade detection information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleFacadeInfo {
    /// Whether this file qualifies as a module facade
    pub is_facade: bool,
    /// Number of submodules (both #[path] and inline)
    pub submodule_count: usize,
    /// List of #[path] declarations
    pub path_declarations: Vec<PathDeclaration>,
    /// Facade quality score (0.0-1.0)
    pub facade_score: f64,
    /// Organization quality classification
    pub organization_quality: OrganizationQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathDeclaration {
    pub module_name: String,
    pub file_path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrganizationQuality {
    Excellent,  // ≥10 submodules, facade_score ≥0.8
    Good,       // ≥5 submodules, facade_score ≥0.6
    Poor,       // ≥3 submodules, facade_score ≥0.5
    Monolithic, // <3 submodules or facade_score <0.5
}

// Update existing struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStructure {
    pub total_lines: usize,
    pub components: Vec<ModuleComponent>,
    pub function_counts: FunctionCounts,
    pub responsibility_count: usize,
    pub public_api_surface: usize,
    pub dependencies: ComponentDependencyGraph,
    /// NEW: Facade detection results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facade_info: Option<ModuleFacadeInfo>,
}
```

#### Phase 2: Detection Logic (src/analysis/module_structure.rs)

```rust
impl ModuleStructureAnalyzer {
    /// Detect if a Rust file is a module facade
    fn detect_module_facade(&self, ast: &syn::File) -> ModuleFacadeInfo {
        let mut path_declarations = Vec::new();
        let mut inline_modules = 0;
        let mut impl_lines = 0;
        let mut fn_lines = 0;
        let mut total_lines = 0;

        for item in &ast.items {
            let span = item.span();
            total_lines = total_lines.max(span.end().line);

            match item {
                syn::Item::Mod(module) => {
                    if let Some(path) = extract_path_attribute(module) {
                        path_declarations.push(PathDeclaration {
                            module_name: module.ident.to_string(),
                            file_path: path,
                            line: span.start().line,
                        });
                    } else if module.content.is_some() {
                        inline_modules += 1;
                    }
                }
                syn::Item::Impl(impl_block) => {
                    impl_lines += span.end().line - span.start().line;
                }
                syn::Item::Fn(func) => {
                    fn_lines += span.end().line - span.start().line;
                }
                _ => {}
            }
        }

        let submodule_count = path_declarations.len() + inline_modules;
        let implementation_lines = impl_lines + fn_lines;

        // Calculate facade score
        let declaration_ratio = if total_lines > 0 {
            (total_lines - implementation_lines) as f64 / total_lines as f64
        } else {
            0.0
        };

        let submodule_factor = (submodule_count as f64 / 5.0).min(1.0);
        let facade_score = (declaration_ratio * 0.7 + submodule_factor * 0.3)
            .clamp(0.0, 1.0);

        // Classify organization quality
        let organization_quality = classify_organization_quality(
            submodule_count,
            facade_score,
        );

        ModuleFacadeInfo {
            is_facade: submodule_count >= 3 && facade_score >= 0.5,
            submodule_count,
            path_declarations,
            facade_score,
            organization_quality,
        }
    }
}

/// Pure function: Extract #[path = "..."] attribute from module
fn extract_path_attribute(module: &syn::ItemMod) -> Option<String> {
    for attr in &module.attrs {
        if attr.path().is_ident("path") {
            if let syn::Meta::NameValue(meta) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &meta.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.value());
                    }
                }
            }
        }
    }
    None
}

/// Pure function: Classify organization quality
fn classify_organization_quality(
    submodule_count: usize,
    facade_score: f64,
) -> OrganizationQuality {
    match (submodule_count, facade_score) {
        (0..=2, _) => OrganizationQuality::Monolithic,
        (n, s) if n >= 10 && s >= 0.8 => OrganizationQuality::Excellent,
        (n, s) if n >= 5 && s >= 0.6 => OrganizationQuality::Good,
        (n, s) if n >= 3 && s >= 0.5 => OrganizationQuality::Poor,
        _ => OrganizationQuality::Monolithic,
    }
}
```

#### Phase 3: Scoring Adjustment (src/priority/scoring/facade_scoring.rs - NEW FILE)

```rust
use crate::analysis::module_structure::{ModuleFacadeInfo, OrganizationQuality};

/// Adjust god object score based on module facade detection
pub fn adjust_score_for_facade(
    base_score: f64,
    facade_info: &ModuleFacadeInfo,
    method_count: usize,
    total_lines: usize,
) -> f64 {
    if !facade_info.is_facade {
        return base_score;
    }

    // Base multiplier from organization quality
    let quality_multiplier = match facade_info.organization_quality {
        OrganizationQuality::Excellent => 0.1,  // 90% reduction
        OrganizationQuality::Good => 0.3,       // 70% reduction
        OrganizationQuality::Poor => 0.6,       // 40% reduction
        OrganizationQuality::Monolithic => 1.0, // No reduction
    };

    // Bonus for high submodule count
    let submodule_bonus = match facade_info.submodule_count {
        0..=4 => 0.9,  // Minimal bonus
        5..=9 => 0.7,  // 30% additional reduction
        _ => 0.5,      // 50% additional reduction for ≥10 modules
    };

    // Check per-module metrics
    let avg_lines_per_module = total_lines / facade_info.submodule_count.max(1);
    let avg_methods_per_module = method_count / facade_info.submodule_count.max(1);

    let size_multiplier = if avg_lines_per_module < 300 && avg_methods_per_module < 15 {
        0.5 // Well-sized modules
    } else if avg_lines_per_module < 500 && avg_methods_per_module < 25 {
        0.7 // Moderate-sized modules
    } else {
        0.9 // Large modules might still need splitting
    };

    base_score * quality_multiplier * submodule_bonus * size_multiplier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excellent_facade_score_reduction() {
        let facade_info = ModuleFacadeInfo {
            is_facade: true,
            submodule_count: 13,
            path_declarations: vec![],
            facade_score: 0.92,
            organization_quality: OrganizationQuality::Excellent,
        };

        let base_score = 69.4;
        let adjusted = adjust_score_for_facade(&facade_info, 91, 2257);

        // Should reduce by ~90%: 69.4 * 0.1 * 0.5 * 0.5 = ~1.7
        assert!(adjusted < 7.0, "Expected score < 7.0, got {}", adjusted);
        assert!(adjusted > 1.0, "Expected score > 1.0, got {}", adjusted);
    }

    #[test]
    fn test_monolithic_no_reduction() {
        let facade_info = ModuleFacadeInfo {
            is_facade: false,
            submodule_count: 0,
            path_declarations: vec![],
            facade_score: 0.05,
            organization_quality: OrganizationQuality::Monolithic,
        };

        let base_score = 69.4;
        let adjusted = adjust_score_for_facade(&facade_info, 91, 2257);

        assert_eq!(adjusted, base_score, "Monolithic score should not change");
    }
}
```

#### Phase 4: Integration (src/organization/god_object_analysis.rs)

```rust
// In calculate_god_object_score_weighted function
pub fn calculate_god_object_score_weighted(
    // ... existing parameters
    module_structure: Option<&crate::analysis::ModuleStructure>,
) -> f64 {
    // ... existing calculation logic
    let mut raw_score = /* calculated score */;

    // NEW: Adjust for module facades
    if let Some(structure) = module_structure {
        if let Some(facade_info) = &structure.facade_info {
            raw_score = crate::priority::scoring::facade_scoring::adjust_score_for_facade(
                raw_score,
                facade_info,
                method_count,
                lines_of_code,
            );
        }
    }

    raw_score
}
```

#### Phase 5: Enhanced Recommendations (src/priority/scoring/recommendation_extended.rs)

```rust
/// Generate facade-aware recommendation
pub fn generate_facade_aware_recommendation(
    facade_info: &ModuleFacadeInfo,
    file_path: &Path,
) -> String {
    match facade_info.organization_quality {
        OrganizationQuality::Excellent => {
            format!(
                "✅ WELL-ORGANIZED: This file is already split into {} submodules \
                with excellent separation of concerns (facade score: {:.2}). \
                No immediate action required. Monitor individual submodule complexity \
                to ensure they remain under 300 lines each.",
                facade_info.submodule_count,
                facade_info.facade_score
            )
        }
        OrganizationQuality::Good => {
            format!(
                "✓ GOOD ORGANIZATION: File has {} submodules (facade score: {:.2}). \
                Current structure is acceptable. Consider monitoring submodule growth \
                and further splitting if individual modules exceed 300 lines.",
                facade_info.submodule_count,
                facade_info.facade_score
            )
        }
        OrganizationQuality::Poor => {
            format!(
                "⚠ PARTIAL ORGANIZATION: File has {} submodules (facade score: {:.2}) \
                but may benefit from additional splitting. Review submodule sizes \
                and responsibilities. Consider extracting larger modules (>400 lines) \
                into additional submodules.",
                facade_info.submodule_count,
                facade_info.facade_score
            )
        }
        OrganizationQuality::Monolithic => {
            format!(
                "🚨 MONOLITHIC FILE: This file lacks proper module organization \
                (facade score: {:.2}). Recommend splitting into 5-8 focused submodules \
                based on distinct responsibilities.",
                facade_info.facade_score
            )
        }
    }
}
```

### Architecture Changes

**Modified Files**:
1. `src/analysis/module_structure.rs` - Add facade detection
2. `src/organization/god_object_analysis.rs` - Integrate facade scoring
3. `src/priority/scoring/recommendation_extended.rs` - Add facade recommendations

**New Files**:
1. `src/priority/scoring/facade_scoring.rs` - Facade scoring logic

**Data Flow**:
```
File AST (syn::File)
    ↓
detect_module_facade(ast) → ModuleFacadeInfo
    ↓
ModuleStructure { facade_info: Some(info) }
    ↓
calculate_god_object_score_weighted() → raw_score
    ↓
adjust_score_for_facade(raw_score, facade_info) → adjusted_score
    ↓
generate_facade_aware_recommendation() → recommendation_text
```

### Testing Strategy

#### Unit Tests (src/analysis/module_structure.rs)

```rust
#[cfg(test)]
mod facade_detection_tests {
    use super::*;

    #[test]
    fn test_detect_pure_facade_with_path_attributes() {
        let code = r#"
            #[path = "executor/builder.rs"]
            mod builder;
            #[path = "executor/commands.rs"]
            mod commands;
            #[path = "executor/pure.rs"]
            mod pure;

            pub use builder::Builder;
            pub use commands::execute;
        "#;

        let ast = syn::parse_file(code).unwrap();
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let facade_info = analyzer.detect_module_facade(&ast);

        assert!(facade_info.is_facade);
        assert_eq!(facade_info.submodule_count, 3);
        assert_eq!(facade_info.path_declarations.len(), 3);
        assert!(facade_info.facade_score > 0.8);
        assert_eq!(facade_info.organization_quality, OrganizationQuality::Good);
    }

    #[test]
    fn test_detect_monolithic_file_no_modules() {
        let code = r#"
            struct Foo { x: u32 }

            impl Foo {
                fn method1(&self) -> u32 { self.x }
                fn method2(&self) -> u32 { self.x * 2 }
                // ... 20 more methods
            }

            fn standalone1() {}
            fn standalone2() {}
            // ... many more functions
        "#;

        let ast = syn::parse_file(code).unwrap();
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let facade_info = analyzer.detect_module_facade(&ast);

        assert!(!facade_info.is_facade);
        assert_eq!(facade_info.submodule_count, 0);
        assert!(facade_info.facade_score < 0.3);
        assert_eq!(facade_info.organization_quality, OrganizationQuality::Monolithic);
    }

    #[test]
    fn test_detect_partial_facade_mixed_content() {
        let code = r#"
            #[path = "sub1.rs"]
            mod sub1;
            #[path = "sub2.rs"]
            mod sub2;

            struct LocalStruct { x: u32 }

            impl LocalStruct {
                fn method1(&self) -> u32 { self.x }
                fn method2(&self) -> u32 { self.x * 2 }
                fn method3(&self) -> u32 { self.x * 3 }
            }

            fn local_fn1() {}
            fn local_fn2() {}
            fn local_fn3() {}
        "#;

        let ast = syn::parse_file(code).unwrap();
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let facade_info = analyzer.detect_module_facade(&ast);

        assert!(!facade_info.is_facade); // Only 2 modules, threshold is 3
        assert_eq!(facade_info.submodule_count, 2);
        assert!(facade_info.facade_score > 0.3 && facade_info.facade_score < 0.7);
    }

    #[test]
    fn test_extract_path_attribute_formats() {
        // Test various #[path] formats
        let cases = vec![
            (r#"#[path = "foo.rs"]"#, Some("foo.rs")),
            (r#"#[path="bar/baz.rs"]"#, Some("bar/baz.rs")),
            (r#"#[derive(Debug)]"#, None),
            (r#"#[cfg(test)]"#, None),
        ];

        for (attr_str, expected) in cases {
            // Test path extraction logic
            // (Implementation details)
        }
    }

    #[test]
    fn test_classify_organization_quality_thresholds() {
        assert_eq!(
            classify_organization_quality(13, 0.92),
            OrganizationQuality::Excellent
        );

        assert_eq!(
            classify_organization_quality(6, 0.65),
            OrganizationQuality::Good
        );

        assert_eq!(
            classify_organization_quality(3, 0.55),
            OrganizationQuality::Poor
        );

        assert_eq!(
            classify_organization_quality(1, 0.2),
            OrganizationQuality::Monolithic
        );
    }
}
```

#### Integration Tests (tests/module_facade_integration_test.rs)

```rust
#[test]
fn test_executor_rs_facade_detection() {
    // Load Prodigy's executor.rs
    let code = std::fs::read_to_string(
        "../prodigy/src/cook/workflow/executor.rs"
    ).unwrap();

    let ast = syn::parse_file(&code).unwrap();
    let analyzer = ModuleStructureAnalyzer::new_rust();
    let structure = analyzer.analyze_rust_ast(&ast);

    let facade_info = structure.facade_info.as_ref().unwrap();

    // Verify detection
    assert!(facade_info.is_facade);
    assert_eq!(facade_info.submodule_count, 13);
    assert!(facade_info.facade_score > 0.85);
    assert_eq!(facade_info.organization_quality, OrganizationQuality::Excellent);

    // Verify score adjustment
    let base_score = 69.4;
    let adjusted = adjust_score_for_facade(
        base_score,
        facade_info,
        91,
        2257,
    );

    assert!(adjusted < 10.0, "Adjusted score should be LOW priority");
}

#[test]
fn test_monolithic_god_object_unchanged() {
    // Test on a genuinely monolithic file
    let code = generate_monolithic_test_file(2000, 50);

    let ast = syn::parse_file(&code).unwrap();
    let analyzer = ModuleStructureAnalyzer::new_rust();
    let structure = analyzer.analyze_rust_ast(&ast);

    let facade_info = structure.facade_info.as_ref().unwrap();

    assert!(!facade_info.is_facade);
    assert_eq!(facade_info.organization_quality, OrganizationQuality::Monolithic);

    let base_score = 65.0;
    let adjusted = adjust_score_for_facade(
        base_score,
        facade_info,
        50,
        2000,
    );

    assert_eq!(adjusted, base_score, "Monolithic score unchanged");
}
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `ModuleStructureAnalyzer` - Add facade detection method
- `GodObjectAnalysis` - Integrate facade scoring
- `calculate_god_object_score_weighted()` - Call scoring adjustment
- Report generation - Display facade information

**External Dependencies**:
- `syn` crate (already in use) - AST parsing for `#[path]` attributes
- No new external dependencies required

## Documentation Requirements

**Code Documentation**:
- Document `ModuleFacadeInfo` struct and all fields
- Explain facade score calculation formula
- Document `adjust_score_for_facade()` multipliers
- Add examples to all public functions

**User Documentation** (book/src/):
- New chapter: "Module Facade Detection"
- Explain what constitutes a facade
- Show before/after examples
- Document scoring methodology
- Provide guidance on interpreting results

**Architecture Updates** (ARCHITECTURE.md):
- Add facade detection to analysis pipeline diagram
- Document scoring adjustment flow
- Explain design rationale for thresholds

## Implementation Notes

### Facade Score Formula

```
facade_score = (declaration_ratio * 0.7) + (submodule_factor * 0.3)

where:
  declaration_ratio = (total_lines - impl_lines - fn_lines) / total_lines
  submodule_factor = min(submodule_count / 5.0, 1.0)
```

**Rationale**:
- 70% weight on declaration ratio - primary indicator of facade pattern
- 30% weight on submodule count - validates organizational structure
- Normalized to 5 submodules as "typical" well-organized file

### Score Adjustment Formula

```
adjusted_score = base_score * quality_multiplier * submodule_bonus * size_multiplier

where:
  quality_multiplier = {0.1, 0.3, 0.6, 1.0} based on organization quality
  submodule_bonus = {0.9, 0.7, 0.5} based on submodule count
  size_multiplier = {0.5, 0.7, 0.9} based on per-module size
```

**Example** (executor.rs):
```
base_score = 69.4
quality_multiplier = 0.1 (Excellent)
submodule_bonus = 0.5 (13 submodules)
size_multiplier = 0.5 (173 avg lines per module)

adjusted = 69.4 * 0.1 * 0.5 * 0.5 = 1.735
```

### Threshold Rationale

**Facade Detection Threshold** (≥3 modules, ≥0.5 facade_score):
- 3 modules: Minimum to demonstrate intentional organization
- 0.5 score: At least 50% declarations, preventing false positives

**Organization Quality Thresholds**:
- **Excellent**: ≥10 modules + ≥0.8 score - Exemplary organization like executor.rs
- **Good**: ≥5 modules + ≥0.6 score - Solid organization, acceptable
- **Poor**: ≥3 modules + ≥0.5 score - Minimal organization, could improve
- **Monolithic**: <3 modules or <0.5 score - Not organized as facade

### Edge Cases

1. **Empty facade files**: Handle files with only module declarations, no code
2. **Mixed inline/external modules**: Count both types toward total
3. **Conditional modules** (`#[cfg(...)] mod foo`): Count as modules
4. **Test modules**: Exclude from facade scoring if in `#[cfg(test)]`
5. **Macro-generated modules**: May not have `#[path]`, detect via inline content

### Performance Considerations

- AST already parsed by existing analysis, no additional parsing
- Facade detection: O(n) in number of AST items (typically <100)
- Minimal memory overhead: ~200 bytes per `ModuleFacadeInfo`
- Expected performance impact: <5% increase in analysis time

## Migration and Compatibility

### Backward Compatibility

**API Changes**:
- `ModuleStructure.facade_info` added as `Option<ModuleFacadeInfo>`
- Existing code without facade_info will still work (Option defaults to None)
- Serialization: `#[serde(skip_serializing_if = "Option::is_none")]` maintains JSON compatibility

**Scoring Changes**:
- God object scores will decrease for well-organized facades
- **Not a breaking change**: Lower scores are more accurate, not functionally different
- Reports will show adjusted scores with explanation

### Migration Strategy

**Phase 1** (Spec 170 implementation):
- Add facade detection, scoring available but not applied by default
- Feature flag: `--enable-facade-detection` for opt-in testing

**Phase 2** (After validation):
- Enable facade detection by default
- Add configuration option to disable: `facade_detection: false`

**Phase 3** (After user feedback):
- Tune thresholds based on real-world usage
- Remove feature flag, always enabled

### Configuration

```toml
# debtmap.toml
[analysis]
facade_detection = true  # Enable facade detection (default: true)

[facade_thresholds]
min_submodules = 3        # Minimum modules for facade (default: 3)
min_facade_score = 0.5    # Minimum facade score (default: 0.5)

[facade_quality_thresholds]
excellent_modules = 10    # Modules for "Excellent" (default: 10)
excellent_score = 0.8     # Score for "Excellent" (default: 0.8)
good_modules = 5          # Modules for "Good" (default: 5)
good_score = 0.6          # Score for "Good" (default: 0.6)
```

## Success Metrics

**Quantitative**:
- False positive rate: <1% (down from current ~10% on well-organized codebases)
- executor.rs score: <10 (from 69.4)
- Performance overhead: <5%
- Test coverage: >95% for facade detection code

**Qualitative**:
- User feedback: "Debtmap now recognizes our module organization"
- Trust improvement: #1 recommendation no longer demonstrably wrong
- Developer time saved: 2 hours per false positive investigation

**Validation Criteria**:
- ✅ Prodigy's executor.rs correctly identified as Excellent facade
- ✅ Monolithic files still flagged as high priority
- ✅ Partially organized files receive moderate scores
- ✅ Recommendations provide actionable, accurate guidance
- ✅ Zero false negatives in test suite (100 real-world files)

## Open Questions

1. **Should inline modules count equally to external modules?**
   - Current: Yes, both count toward submodule_count
   - Alternative: Weight external modules higher (they're more separated)
   - Decision: Count equally for simplicity, revisit if needed

2. **How to handle deeply nested module hierarchies?**
   - Current: Only analyze top-level file
   - Alternative: Recursively analyze submodules
   - Decision: Top-level only for Spec 170, defer deep analysis to future spec

3. **Should test modules affect facade scoring?**
   - Current: Test modules excluded from facade detection
   - Alternative: Include but with different thresholds
   - Decision: Exclude tests, they follow different patterns

4. **Threshold configurability vs. opinionated defaults?**
   - Current: Opinionated defaults, configurable via debtmap.toml
   - Alternative: No configuration, fixed thresholds
   - Decision: Provide defaults but allow tuning for different team preferences

## Related Specifications

- **Spec 134**: God object detection (baseline for facade scoring integration)
- **Spec 140**: Domain diversity metrics (complementary organizational analysis)
- **Spec 152**: Module structure analysis (foundation for facade detection)

## Future Enhancements

**Multi-Language Support** (Future Spec 171):
- Python facade detection (`from .module import *`)
- JavaScript/TypeScript export analysis
- Unified facade scoring across languages

**Submodule Quality Analysis** (Future Spec 172):
- Analyze complexity of individual submodules
- Report per-submodule metrics
- Flag large submodules for splitting

**Facade Evolution Tracking** (Future Spec 173):
- Track module additions over time
- Monitor refactoring progress
- Historical facade quality trends

**Automated Refactoring Suggestions** (Future Spec 174):
- Suggest specific module extractions
- Recommend responsibility-based splits
- Generate module structure from clustering
