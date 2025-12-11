---
number: 201
title: Accurate Lines of Code Calculation
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-11
---

# Specification 201: Accurate Lines of Code Calculation

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap's Lines of Code (LOC) calculation has several issues that affect the accuracy of debt density metrics:

### Issue 1: Total LOC Only Includes Files With Debt Items

**Current Behavior** (`src/priority/mod.rs:886-937`):
```rust
for item in &self.items {
    if let Some(line_count) = item.file_line_count {
        unique_files.entry(item.location.file.clone()).or_insert(line_count);
    }
}
let total_lines_of_code: usize = unique_files.values().sum();
```

**Problem**: Only files that have at least one identified debt item are included in the total LOC count. Files without issues are excluded, significantly understating codebase size.

**Evidence**: Self-analysis shows 153K reported LOC vs 247K actual physical lines (~62% of codebase missing from count).

**Impact**: Debt density is artificially inflated because the denominator (total LOC) excludes clean files.

### Issue 2: Multi-line Comment Detection is Incomplete

**Current Behavior** (`src/metrics/loc_counter.rs:235-253`):
```rust
fn is_comment_line(trimmed_line: &str) -> bool {
    trimmed_line.starts_with("//")
    || trimmed_line.starts_with("/*")
    || (trimmed_line.starts_with('*') && !trimmed_line.starts_with("*/"))
    || trimmed_line.starts_with('#')  // Python
}
```

**Problem**: No state tracking for multi-line `/* ... */` blocks. Lines in the middle of block comments that don't start with `*` are miscounted as code.

**Example miscounted as code**:
```rust
/*
This line starts with text, not *
so it's incorrectly counted as code
*/
```

### Issue 3: Rust Attributes Miscounted as Python Comments

**Current Behavior**: `#[derive(...)]` and other Rust attributes match the Python comment pattern (`trimmed_line.starts_with('#')`).

**Problem**: Rust attributes should be counted as code, not comments.

### Issue 4: Function Length Includes All Physical Lines

**Current Behavior** (`src/analyzers/rust_complexity_calculation.rs:181-193`):
```rust
pub fn count_function_lines(item_fn: &syn::ItemFn) -> usize {
    let span = item_fn.span();
    end_line - start_line + 1
}
```

**Problem**: This counts all physical lines including blanks and comments, which is inconsistent with common SLOC (Source Lines of Code) definitions. Not necessarily wrong, but should be documented.

## Objective

Fix LOC calculation to:

1. **Include all analyzed files in total LOC** - not just files with debt items
2. **Correctly handle multi-line comments** - track comment block state
3. **Distinguish Rust attributes from Python comments** - language-aware comment detection
4. **Document the LOC methodology** - clarify what "LOC" means in debtmap context

## Requirements

### Functional Requirements

1. **FR-1: Count All Analyzed Files**
   - Track all files discovered during analysis, not just those with debt items
   - Pass the complete file list to LOC calculation
   - Use the existing `LocCounter` for consistent counting

2. **FR-2: Track Multi-line Comment State**
   - Implement state machine for block comment tracking
   - Handle `/* ... */` blocks that span multiple lines
   - Handle nested block comments if applicable (Rust supports `/* /* */ */`)
   - Correctly identify lines within comment blocks as comments

3. **FR-3: Language-Aware Comment Detection**
   - Accept file extension or language hint in `count_content`
   - For Rust files: Don't treat `#[...]` as comments
   - For Python files: Continue treating `#` as comments
   - For JS/TS files: Handle `//` and `/* */` appropriately

4. **FR-4: Maintain Backward Compatibility**
   - `physical_lines` should still represent raw line count
   - `code_lines` should be accurate SLOC (excluding comments and blanks)
   - Existing APIs should not break

### Non-Functional Requirements

1. **NFR-1: Performance**
   - File counting should remain O(n) where n is file size
   - No additional file I/O beyond what's already done
   - Cache file line counts during analysis phase

2. **NFR-2: Accuracy**
   - Total LOC should match `find . -name "*.rs" | xargs wc -l` for physical lines
   - Comment detection should be >95% accurate for standard code patterns

3. **NFR-3: Testability**
   - All comment detection logic must have unit tests
   - Multi-line comment edge cases must be covered

## Acceptance Criteria

- [ ] Running `debtmap analyze src/` reports total LOC that matches `wc -l` output (±5% for physical lines)
- [ ] Multi-line block comments are correctly detected as comments, not code
- [ ] Rust attributes `#[derive(...)]` are counted as code, not comments
- [ ] Files with no debt items still contribute to total LOC count
- [ ] `LocCount.code_lines + comment_lines + blank_lines == physical_lines` invariant holds
- [ ] All existing tests continue to pass
- [ ] New unit tests cover multi-line comment detection
- [ ] Debt density calculation uses accurate total LOC denominator

## Technical Details

### Implementation Approach

#### Phase 1: Fix Total LOC Calculation

1. Modify analysis pipeline to collect all discovered file paths
2. Pass complete file list to `calculate_total_impact`
3. Count LOC for all files, not just those in `self.items`

**Location**: `src/priority/mod.rs` - add `analyzed_files: Vec<PathBuf>` field

```rust
// In calculate_total_impact:
let mut unique_files: HashMap<PathBuf, usize> = HashMap::new();

// First, count all analyzed files
for file_path in &self.analyzed_files {
    if let Ok(count) = loc_counter.count_file(file_path) {
        unique_files.insert(file_path.clone(), count.physical_lines);
    }
}

// Items can still override with cached counts (for consistency)
for item in &self.items {
    if let Some(line_count) = item.file_line_count {
        unique_files.insert(item.location.file.clone(), line_count);
    }
}
```

#### Phase 2: Multi-line Comment State Tracking

Modify `LocCounter::count_content` to track block comment state:

```rust
pub fn count_content(&self, content: &str, language: Option<Language>) -> LocCount {
    let mut in_block_comment = false;
    let mut block_comment_depth = 0; // For nested comments (Rust)

    for line in content.lines() {
        let trimmed = line.trim();

        // Process multi-line comment boundaries
        let (is_comment, new_in_block, new_depth) =
            classify_line(trimmed, in_block_comment, block_comment_depth, language);

        in_block_comment = new_in_block;
        block_comment_depth = new_depth;

        // Categorize line
        if trimmed.is_empty() {
            blank_lines += 1;
        } else if is_comment {
            comment_lines += 1;
        } else {
            code_lines += 1;
        }
    }
}
```

#### Phase 3: Language-Aware Comment Detection

Add language parameter to counting functions:

```rust
#[derive(Clone, Copy, Debug)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Unknown,
}

fn classify_line(
    trimmed: &str,
    in_block: bool,
    depth: usize,
    language: Option<Language>
) -> (bool, bool, usize) {
    // ... language-specific logic
}
```

For Rust specifically:
- `//` starts single-line comment
- `/*` starts block comment (nestable)
- `*/` ends block comment
- `#[...]` is an attribute (code, not comment)
- `#![...]` is an inner attribute (code, not comment)

### Architecture Changes

1. Add `analyzed_files: Vec<PathBuf>` to `UnifiedAnalysis` struct
2. Populate `analyzed_files` during file discovery phase
3. Add `Language` parameter to `LocCounter::count_content`
4. Add state tracking fields for block comment detection

### Data Structures

```rust
// Extended LocCountingConfig
pub struct LocCountingConfig {
    pub include_tests: bool,
    pub include_generated: bool,
    pub count_comments: bool,
    pub count_blanks: bool,
    pub exclude_patterns: Vec<String>,
    pub language_detection: bool,  // NEW: auto-detect from extension
}

// Comment tracking state (internal)
struct CommentState {
    in_block_comment: bool,
    block_depth: usize,  // For Rust nested comments
}
```

### APIs and Interfaces

```rust
// Extended method signature
impl LocCounter {
    /// Count lines with optional language hint
    pub fn count_content_with_language(
        &self,
        content: &str,
        language: Option<Language>
    ) -> LocCount;

    /// Count file with auto-detected language
    pub fn count_file(&self, path: &Path) -> Result<LocCount, io::Error> {
        let language = Language::from_extension(path.extension());
        let content = fs::read_to_string(path)?;
        Ok(self.count_content_with_language(&content, Some(language)))
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/metrics/loc_counter.rs` - Core counting logic
  - `src/priority/mod.rs` - Total LOC aggregation
  - `src/priority/scoring/construction.rs` - File line count caching
  - `src/analyzers/rust.rs` - File discovery
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Multi-line Comment Tests**
   ```rust
   #[test]
   fn test_multiline_block_comment() {
       let code = "/* comment\nstill comment\nend */\ncode";
       let count = counter.count_content_with_language(code, Some(Language::Rust));
       assert_eq!(count.comment_lines, 3);
       assert_eq!(count.code_lines, 1);
   }

   #[test]
   fn test_nested_block_comments_rust() {
       let code = "/* outer /* inner */ still outer */\ncode";
       // ...
   }
   ```

2. **Rust Attribute Tests**
   ```rust
   #[test]
   fn test_rust_attributes_are_code() {
       let code = "#[derive(Debug)]\nstruct Foo;";
       let count = counter.count_content_with_language(code, Some(Language::Rust));
       assert_eq!(count.code_lines, 2);
       assert_eq!(count.comment_lines, 0);
   }
   ```

3. **Total LOC Invariant Tests**
   ```rust
   #[test]
   fn test_loc_invariant() {
       let count = counter.count_content(code);
       assert_eq!(
           count.physical_lines,
           count.code_lines + count.comment_lines + count.blank_lines
       );
   }
   ```

### Integration Tests

1. Compare `debtmap analyze .` total LOC with `wc -l` output
2. Verify debt density changes appropriately with fix
3. Test on codebases with varying comment styles

### Performance Tests

1. Ensure no regression in analysis time
2. Profile LOC counting on large files

## Documentation Requirements

- **Code Documentation**: Document the LOC counting methodology in `loc_counter.rs`
- **User Documentation**: Update any user-facing docs that mention LOC
- **Architecture Updates**: None required

## Implementation Notes

1. **Backward Compatibility**: The `count_content` method without language parameter should remain functional, defaulting to conservative behavior.

2. **Edge Cases to Handle**:
   - String literals containing comment markers: `"/* not a comment */"`
   - Raw strings in Rust: `r#"/* also not a comment */"#`
   - Heredocs in shell scripts embedded in code
   - Doc comments (`///` and `//!` in Rust) - these ARE comments

3. **String Literal Detection**: Full string literal detection requires parsing. For a simpler solution, accept some inaccuracy for code containing comment markers inside strings. Document this limitation.

4. **Performance**: Reading files for LOC that have already been parsed is wasteful. Consider extracting line counts from the AST during parsing phase when available.

## Migration and Compatibility

- **Breaking Changes**: None - all changes are additive or fix bugs
- **Migration**: No migration required
- **Compatibility**: Existing JSON output format unchanged, but `total_loc` values will increase (more accurate)

## Success Metrics

1. **Accuracy**: Total LOC matches `wc -l` within 5%
2. **Consistency**: `code_lines + comment_lines + blank_lines == physical_lines` always holds
3. **No Regression**: All existing tests pass
4. **Meaningful Density**: Debt density reflects actual codebase size
