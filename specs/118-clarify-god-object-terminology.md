---
number: 118
title: Clarify God Object/Module Terminology and Reporting
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-24
---

# Specification 118: Clarify God Object/Module Terminology and Reporting

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently detects large files with many functions and reports them as "GOD OBJECT" issues. However, the terminology and reporting create confusion because:

1. **Language Ambiguity**: In Rust, the term "god object" traditionally refers to a struct/class with too many methods, but debtmap applies it to entire files/modules with many module-level functions
2. **Misleading Metrics**: Reports say "107 methods" when counting module-level functions, not class methods
3. **Valid Pattern Confusion**: Functional programming with many small, focused module functions is a valid pattern, but gets conflated with problematic monolithic structures
4. **Missing Distinction**: No clear distinction between:
   - A file with one god object (struct with 100 methods)
   - A file with 50 focused helper functions and 10 small structs
   - A configuration module with 28 related config structs

**Example Confusion**:
```
#1 SCORE: 67.3 [CRITICAL - FILE - GOD OBJECT]
├─ ./src/priority/formatter.rs (2870 lines, 112 functions)
├─ WHY: This class violates single responsibility principle with 107 methods...
```

This reports a FILE as a "class" and counts module functions as "methods," creating semantic confusion.

## Objective

Improve terminology, detection accuracy, and reporting clarity for god object/module issues to:
- Distinguish between god objects (structs with many methods) and god modules (files with many responsibilities)
- Accurately report what is being counted (class methods vs module functions vs file-level constructs)
- Provide actionable recommendations that match the actual code structure
- Reduce false positive perception while maintaining valid large-file detection

## Requirements

### Functional Requirements

1. **Terminology Differentiation**
   - Introduce distinct labels: "GOD OBJECT" vs "GOD MODULE" vs "LARGE FILE"
   - Use "GOD OBJECT" only when a single struct/class has excessive methods (>30)
   - Use "GOD MODULE" when a file has many responsibilities but diverse structures
   - Use "LARGE FILE" when size is the primary concern without clear god object patterns

2. **Accurate Metrics Reporting**
   - Report "N methods" only when referring to impl block methods
   - Report "N module functions" when counting file-level functions
   - Report "N types" when counting structs/enums/traits in a file
   - Clearly state what entity has "M responsibilities"

3. **Responsibility Analysis Enhancement**
   - Analyze actual code structure to identify responsibilities:
     - Group functions by naming patterns (e.g., `format_*`, `validate_*`, `parse_*`)
     - Detect distinct impl blocks and their purposes
     - Identify configuration vs logic vs I/O concerns
   - Report responsibilities with concrete evidence (function groups)

4. **Context-Aware Recommendations**
   - For true god objects (single struct): "Extract methods into separate traits/structs"
   - For god modules (diverse structures): "Split file by domain/responsibility"
   - For configuration modules: "Group related configs into sub-modules"
   - For functional modules: "Organize by data flow or feature area"

### Non-Functional Requirements

1. **Backward Compatibility**: Maintain existing scoring logic while improving labels
2. **Performance**: No significant performance degradation from enhanced analysis
3. **Clarity**: Recommendations must clearly map to actual code structure
4. **Consistency**: Apply same terminology across all output formats

## Acceptance Criteria

- [ ] God object detection distinguishes between:
  - Single struct with >30 methods → "GOD OBJECT"
  - File with >50 functions across multiple types → "GOD MODULE"
  - File with >1000 lines but focused purpose → "LARGE FILE"
- [ ] Metrics reporting accurately states what is counted:
  - "107 module functions" not "107 methods" for file-level functions
  - "N methods in impl Foo" when counting struct methods
  - "M types (structs/enums)" when relevant
- [ ] "WHY" section correctly identifies the problem:
  - "This struct has 107 methods" vs "This file has 107 functions across 28 types"
  - Provides evidence: "Functions grouped by: formatting (45), validation (22), I/O (15)"
- [ ] Recommendations match the actual issue:
  - God object → extract methods/traits
  - God module → split by responsibility with concrete suggestions
  - Config module → organize into sub-modules
- [ ] All output formats (terminal, markdown, JSON) use consistent terminology
- [ ] Documentation explains the distinction between god object/module/large file
- [ ] Test cases cover each scenario with expected labels and recommendations

## Technical Details

### Implementation Approach

1. **Enhanced Detection Logic**
   ```rust
   enum GodIssueType {
       GodObject {
           type_name: String,
           method_count: usize,
           impl_block_info: ImplBlockInfo,
       },
       GodModule {
           module_path: PathBuf,
           function_count: usize,
           type_count: usize,
           responsibilities: Vec<Responsibility>,
       },
       LargeFile {
           file_path: PathBuf,
           line_count: usize,
           reason: String, // e.g., "generated code", "configuration data"
       },
   }

   struct Responsibility {
       name: String,
       function_count: usize,
       evidence: Vec<String>, // function name patterns
   }
   ```

2. **Responsibility Detection Algorithm**
   - Group module functions by prefix/suffix patterns
   - Analyze impl blocks separately from module functions
   - Detect common responsibility patterns:
     - Formatting: `format_*`, `print_*`, `write_*`
     - Validation: `validate_*`, `check_*`, `verify_*`
     - Parsing: `parse_*`, `read_*`, `extract_*`
     - I/O: `*_io`, `*_writer`, `*_reader`
     - Configuration: `default_*`, `*_config`, getter/setter patterns

3. **Threshold Configuration**
   ```rust
   struct GodDetectionConfig {
       god_object_method_threshold: usize,      // default: 30
       god_module_function_threshold: usize,    // default: 50
       god_module_responsibility_threshold: usize, // default: 4
       large_file_line_threshold: usize,        // default: 1000
   }
   ```

4. **Reporting Enhancement**
   - Template for god object:
     ```
     #N SCORE: X [CRITICAL - GOD OBJECT - {StructName}]
     ├─ {file}:{line} impl {StructName} (M methods)
     ├─ WHY: This struct violates SRP with M methods across N responsibilities
     ├─ RESPONSIBILITIES: {list grouped methods}
     ├─ ACTION: Extract methods into separate traits/structs by responsibility
     ```

   - Template for god module:
     ```
     #N SCORE: X [CRITICAL - GOD MODULE]
     ├─ {file} (L lines, F functions, T types)
     ├─ WHY: This module has M responsibilities: {list with evidence}
     ├─ ACTION: Split into M focused modules: {concrete suggestions}
     ```

### Architecture Changes

1. **File Analysis Enhancement**
   - Extend `FileMetrics` to include:
     - `module_function_count: usize`
     - `largest_impl_block: Option<(String, usize)>` (name, method count)
     - `responsibilities: Vec<Responsibility>`
     - `god_issue_type: Option<GodIssueType>`

2. **Debt Type Refinement**
   - Split `DebtType::GodObject` into:
     - `DebtType::GodObject { type_name, method_count, ... }`
     - `DebtType::GodModule { responsibilities, ... }`
     - `DebtType::LargeFile { line_count, ... }`

3. **Formatter Updates**
   - Update `format_god_object_steps()` to handle each type differently
   - Add `format_responsibilities()` helper for evidence-based reporting
   - Update `determine_file_type_label()` to use new classifications

### Data Structures

```rust
// In src/priority/mod.rs or src/debt/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplBlockInfo {
    pub impl_name: String,
    pub method_count: usize,
    pub file_path: PathBuf,
    pub line_number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Responsibility {
    pub name: String,
    pub function_count: usize,
    pub function_examples: Vec<String>, // First 3-5 function names
    pub pattern: ResponsibilityPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponsibilityPattern {
    Formatting,
    Validation,
    Parsing,
    IoOperations,
    Configuration,
    BusinessLogic,
    DataAccess,
    Other(String),
}
```

### APIs and Interfaces

```rust
// In god object detector
pub fn classify_god_issue(metrics: &FileMetrics) -> Option<GodIssueType> {
    // Returns specific classification based on analysis
}

pub fn detect_responsibilities(
    functions: &[FunctionInfo],
    types: &[TypeInfo],
) -> Vec<Responsibility> {
    // Groups functions and identifies responsibility patterns
}

pub fn analyze_impl_blocks(ast: &Ast) -> Vec<ImplBlockInfo> {
    // Extracts impl block information from AST
}
```

## Dependencies

- **Prerequisites**: None (enhancement to existing god object detection)
- **Affected Components**:
  - `src/priority/mod.rs` - DebtType enum
  - `src/priority/formatter.rs` - God object reporting
  - `src/priority/scoring/` - Recommendations
  - `src/analyzers/*/` - AST analysis for impl blocks
  - `src/debt/god_object.rs` - Core detection logic

## Testing Strategy

### Unit Tests

1. **Classification Tests**
   ```rust
   #[test]
   fn test_classify_true_god_object() {
       // Single struct with 50 methods → GodObject
   }

   #[test]
   fn test_classify_god_module() {
       // 100 functions across 20 types → GodModule
   }

   #[test]
   fn test_classify_config_module() {
       // 28 config structs, each with 3-5 methods → GodModule or acceptable
   }

   #[test]
   fn test_classify_functional_module() {
       // 60 small pure functions → may be acceptable or GodModule
   }
   ```

2. **Responsibility Detection Tests**
   ```rust
   #[test]
   fn test_detect_formatting_responsibility() {
       // Functions: format_header, format_section, print_summary
       // → Formatting responsibility
   }

   #[test]
   fn test_multiple_clear_responsibilities() {
       // format_* (20), validate_* (15), parse_* (10)
       // → 3 distinct responsibilities
   }
   ```

3. **Reporting Tests**
   ```rust
   #[test]
   fn test_god_object_report_accuracy() {
       // Ensure "N methods in impl Foo" not "N functions"
   }

   #[test]
   fn test_god_module_report_clarity() {
       // Ensure responsibilities listed with evidence
   }
   ```

### Integration Tests

1. **Real Codebase Analysis**
   - Test on `src/priority/formatter.rs` (known god module)
   - Test on `src/config.rs` (configuration module)
   - Verify correct classification and reporting

2. **Output Format Validation**
   - Run analysis with different verbosity levels
   - Verify terminology consistency across formats
   - Check that recommendations match classifications

### Acceptance Tests

1. **False Positive Reduction**
   - Configuration modules should not be reported as critical god objects
   - Functional modules with many small pure functions should be weighted appropriately
   - Clear distinction in reports between different issue types

2. **User Understanding**
   - Reports clearly explain what is being counted
   - Recommendations are actionable given the actual code structure
   - Users can easily identify true problems vs acceptable patterns

## Documentation Requirements

### Code Documentation

1. **Detection Logic**
   - Document classification algorithm and thresholds
   - Explain responsibility pattern matching
   - Provide examples of each god issue type

2. **Configuration**
   - Document threshold tuning for different project styles
   - Explain when to adjust god object vs god module thresholds

### User Documentation

1. **README/User Guide Updates**
   - Add section: "Understanding God Object vs God Module Detection"
   - Provide examples of each classification
   - Explain how to interpret metrics in reports

2. **Output Documentation**
   - Update examples to show new terminology
   - Explain what "M module functions" vs "M methods" means
   - Document responsibility detection patterns

### Architecture Updates

1. **ARCHITECTURE.md**
   - Document god issue classification system
   - Explain responsibility detection algorithm
   - Update debt type taxonomy

## Implementation Notes

### Phased Approach

**Phase 1: Classification Logic**
- Implement `GodIssueType` enum and classification function
- Add responsibility detection algorithm
- Update `FileMetrics` to include new data

**Phase 2: Reporting Updates**
- Update formatters to use new classifications
- Implement evidence-based responsibility reporting
- Update recommendation generation

**Phase 3: Testing and Validation**
- Add comprehensive test suite
- Run on debtmap's own codebase
- Validate against known examples

**Phase 4: Documentation**
- Update user-facing documentation
- Add inline code documentation
- Create examples and guides

### Edge Cases

1. **Mixed Patterns**: File with both god object and god module issues
   - Report both, prioritize by severity

2. **Generated Code**: Large files that are auto-generated
   - Detect common generation patterns
   - Lower priority or exclude from recommendations

3. **Test Files**: Large test files with many test cases
   - Different thresholds for test code
   - Focus on organization rather than splitting

4. **Configuration Data**: JSON/YAML-like data structures in Rust
   - Recognize as configuration pattern
   - Lower severity, different recommendations

### Performance Considerations

1. **Responsibility Detection**: O(n) scan of functions, group by prefix patterns
2. **Impl Block Analysis**: Already available from AST parsing
3. **Caching**: Cache responsibility analysis with file hash
4. **Threshold Tuning**: Start conservative, adjust based on false positive rate

## Migration and Compatibility

### Backward Compatibility

1. **Configuration**: Add new thresholds without breaking existing configs
2. **JSON Output**: Extend with new fields, maintain existing fields
3. **Scoring**: Maintain scores, improve labeling only

### Breaking Changes

1. **DebtType Enum**: Splitting `GodObject` into multiple variants
   - Migration: Map old `GodObject` → new classifications based on metrics
   - Version bump: Minor version (new feature with enum extension)

2. **Report Format**: Text output changes terminology
   - Impact: Users parsing text output may need updates
   - Mitigation: Document changes, provide examples

### Migration Path

1. Implement new classifications alongside existing detection
2. Add feature flag `--precise-god-detection` for opt-in testing
3. Validate on multiple codebases
4. Make new system default, keep old as fallback
5. Deprecate old system after 2-3 releases

## Success Metrics

1. **Reduced Confusion**: User reports of terminology confusion decrease
2. **Improved Accuracy**: False positive rate for god object detection decreases by 50%
3. **Actionable Recommendations**: Users can identify concrete split points from reports
4. **Consistent Terminology**: 100% consistency across output formats

## Future Enhancements

1. **Automated Splitting Suggestions**: Generate actual file split proposals with moved functions
2. **Dependency-Aware Splitting**: Suggest splits that minimize inter-file dependencies
3. **IDE Integration**: Export recommendations in LSP format for IDE quick-fixes
4. **Historical Tracking**: Track god module evolution over time
