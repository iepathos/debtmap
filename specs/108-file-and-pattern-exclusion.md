---
number: 108
title: File and Pattern Exclusion
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-09
---

# Specification 108: File and Pattern Exclusion

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap has existing ignore patterns in `.debtmap.toml` for file-level exclusions, but lacks CLI override capability and AST-level filtering for inline test modules.

**Current state**:
- `.debtmap.toml` has `[ignore]` section with glob patterns
- File walker respects these patterns
- Works for excluding entire test directories

**Key limitation**: Cannot exclude `#[cfg(test)]` modules within production files, causing test code to inflate file-level complexity scores.

**Real-world impact**: In the Prodigy project analysis, fixing TestingGap items by adding tests caused overall debt scores to increase because test code (with `#[cfg(test)]` modules) contributed to file-level complexity metrics. Users need to separate production code analysis from test code analysis.

## Objective

Add CLI exclusion flags and AST-level test module filtering to complement existing config-based ignore patterns, allowing users to override config patterns and exclude inline test modules from file-level complexity scores.

## Requirements

### Functional Requirements

1. **Command-line exclusion patterns**:
   - Add `--exclude <PATTERN>` flag accepting glob patterns
   - Support multiple exclusions: `--exclude tests --exclude vendor`
   - Value delimiter for comma-separated patterns: `--exclude tests,vendor,generated`
   - CLI patterns **override** (not merge with) config file patterns

2. **AST-level exclusions** (Rust-specific):
   - Add `--exclude-test-modules` flag for Rust test code
   - Detect and exclude `#[cfg(test)]` modules from file-level scores
   - Skip test functions marked with `#[test]` attribute
   - Handle nested test modules correctly
   - Preserve non-test code in mixed files

3. **Output and reporting**:
   - Log excluded file count and patterns used (with `-v`)
   - Include exclusion summary in analysis metadata
   - Preserve original file count vs analyzed file count in JSON output

### Non-Functional Requirements

- **Performance**: Exclusion should happen early (file discovery phase)
- **Minimal overhead**: Pattern matching should be efficient (< 5% overhead)
- **Backward compatibility**: No breaking changes to existing CLI or JSON output
- **Clear documentation**: Examples for common exclusion scenarios
- **Cross-platform**: Patterns work consistently on Unix and Windows

## Acceptance Criteria

- [ ] `--exclude` flag accepts glob patterns and filters files before analysis
- [ ] Multiple `--exclude` flags can be specified
- [ ] Comma-separated exclusion patterns work correctly
- [ ] CLI `--exclude` patterns override config file `[ignore]` patterns
- [ ] `--exclude-test-modules` flag excludes Rust `#[cfg(test)]` modules from file-level scores
- [ ] Test modules don't contribute to file complexity when flag is used
- [ ] Non-test code in mixed files is still analyzed
- [ ] Excluded files don't appear in JSON output items
- [ ] Analysis metadata includes exclusion summary
- [ ] Verbose mode logs excluded file paths and test module count
- [ ] Tests cover CLI override and AST-level filtering
- [ ] Documentation includes exclusion examples
- [ ] Performance impact < 5% for large codebases
- [ ] Works correctly on Windows paths

## Technical Details

### Implementation Approach

**Phase 1: CLI Exclusion Override**
1. Add `--exclude` CLI argument to `Analyze` command in `src/cli.rs`
2. Modify file walker to accept CLI exclusions that override config
3. Test CLI override behavior with existing config patterns

**Phase 2: AST-Level Rust Test Exclusion**
1. Add `--exclude-test-modules` flag to CLI
2. Create `TestModuleDetector` in `src/analyzers/rust/test_detector.rs`
3. Filter out `#[cfg(test)]` modules during Rust AST traversal
4. Update file-level score aggregation to skip test module complexity
5. Preserve non-test code analysis in mixed files

**Phase 3: Reporting and Documentation**
1. Add exclusion summary to analysis metadata
2. Implement verbose exclusion logging (file paths, test module count)
3. Update user documentation with examples
4. Add integration tests for CLI override and AST filtering

### Architecture Changes

```rust
// src/filters/exclusion.rs
pub struct ExclusionMatcher {
    patterns: Vec<glob::Pattern>,
    exclude_test_modules: bool,
    presets: Vec<ExclusionPreset>,
}

impl ExclusionMatcher {
    pub fn new(patterns: Vec<String>, exclude_test_modules: bool) -> Result<Self>;
    pub fn with_presets(presets: Vec<ExclusionPreset>) -> Self;
    pub fn should_exclude(&self, path: &Path) -> bool;
    pub fn should_exclude_test_module(&self, module: &syn::ItemMod) -> bool;
    pub fn get_summary(&self) -> ExclusionSummary;
}

pub enum ExclusionPreset {
    Common,      // tests, vendor, node_modules, target, build
    Tests,       // all test patterns
    Generated,   // generated code patterns
}

pub struct ExclusionSummary {
    pub total_patterns: usize,
    pub files_excluded: usize,
    pub patterns_used: Vec<String>,
}

// src/analyzers/rust/test_detector.rs
pub struct TestModuleDetector {
    exclude_test_modules: bool,
}

impl TestModuleDetector {
    pub fn is_test_module(module: &syn::ItemMod) -> bool;
    pub fn is_test_function(func: &syn::ItemFn) -> bool;
    pub fn filter_test_items(items: Vec<syn::Item>) -> Vec<syn::Item>;
}
```

### Data Structures

```rust
// Update AnalysisConfig
pub struct AnalysisConfig {
    // ... existing fields ...
    pub exclude_patterns: Option<Vec<String>>,
    pub exclude_test_modules: bool,
    pub exclude_presets: Option<Vec<ExclusionPreset>>,
}

// Add to AnalysisMetadata
pub struct AnalysisMetadata {
    // ... existing fields ...
    pub exclusion_summary: Option<ExclusionSummary>,
    pub files_excluded: usize,
    pub files_analyzed: usize,
}
```

### APIs and Interfaces

```rust
// CLI additions in src/cli.rs
Commands::Analyze {
    // ... existing fields ...

    /// Exclude files matching glob patterns (can be repeated)
    #[arg(long = "exclude", value_name = "PATTERN")]
    exclude: Option<Vec<String>>,

    /// Exclude Rust test modules (#[cfg(test)]) from file-level scores
    #[arg(long = "exclude-test-modules")]
    exclude_test_modules: bool,

    /// Apply exclusion preset (common, tests, generated)
    #[arg(long = "exclude-preset", value_enum)]
    exclude_preset: Option<Vec<ExclusionPreset>>,
}

// Config file additions in .debtmap.toml
[analysis]
exclude_patterns = [
    "tests/**",
    "vendor/**",
    "**/*_generated.rs",
    "**/target/**"
]
exclude_test_modules = true
exclude_presets = ["common"]
```

### Integration Points

1. **File discovery** (`src/io/file_walker.rs`):
   - Apply exclusions during directory traversal
   - Skip excluded paths early to avoid unnecessary reads

2. **Rust analyzer** (`src/analyzers/rust_analyzer.rs`):
   - Filter `#[cfg(test)]` modules during AST parsing
   - Exclude test functions from file-level aggregation

3. **Score aggregation** (`src/priority/scoring.rs`):
   - Ensure excluded modules don't contribute to scores
   - Update file-level metrics to reflect only analyzed code

4. **Output generation** (`src/io/output/`):
   - Include exclusion summary in metadata
   - Log excluded file count in verbose mode

## Dependencies

- **Prerequisites**: None (foundation feature)
- **Affected Components**:
  - `src/cli.rs` - CLI argument parsing
  - `src/io/file_walker.rs` - File discovery
  - `src/analyzers/rust_analyzer.rs` - AST filtering
  - `src/config.rs` - Configuration schema
  - `src/priority/scoring.rs` - Score aggregation
- **External Dependencies**:
  - `glob` crate (already used) for pattern matching
  - `syn` crate (already used) for AST attribute detection

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_glob_pattern_matching() {
        let matcher = ExclusionMatcher::new(vec!["**/*_test.rs".to_string()], false);
        assert!(matcher.should_exclude(Path::new("src/foo_test.rs")));
        assert!(!matcher.should_exclude(Path::new("src/foo.rs")));
    }

    #[test]
    fn test_path_segment_matching() {
        let matcher = ExclusionMatcher::new(vec!["tests/".to_string()], false);
        assert!(matcher.should_exclude(Path::new("tests/integration.rs")));
        assert!(!matcher.should_exclude(Path::new("src/tests_util.rs")));
    }

    #[test]
    fn test_test_module_detection() {
        // Parse Rust code with #[cfg(test)]
        // Verify is_test_module() returns true
    }

    #[test]
    fn test_multiple_patterns() {
        let matcher = ExclusionMatcher::new(
            vec!["tests/**".to_string(), "vendor/**".to_string()],
            false
        );
        assert!(matcher.should_exclude(Path::new("tests/foo.rs")));
        assert!(matcher.should_exclude(Path::new("vendor/lib.rs")));
    }

    #[test]
    fn test_preset_patterns() {
        let matcher = ExclusionMatcher::with_presets(vec![ExclusionPreset::Common]);
        assert!(matcher.should_exclude(Path::new("node_modules/foo.js")));
        assert!(matcher.should_exclude(Path::new("target/debug/foo")));
    }
}
```

### Integration Tests

1. **Exclusion workflow**:
   - Analyze project with `--exclude tests`
   - Verify test files not in JSON output
   - Verify total debt score excludes test code

2. **Config file integration**:
   - Create `.debtmap.toml` with exclusions
   - Verify config patterns are applied
   - Verify CLI overrides config

3. **Rust test module exclusion**:
   - Analyze Rust project with test modules
   - Verify `--exclude-test-modules` reduces file scores
   - Verify test functions excluded from aggregation

4. **Performance test**:
   - Analyze large codebase with/without exclusions
   - Measure overhead (should be < 5%)

## Documentation Requirements

### Code Documentation

- Document `ExclusionMatcher` public API
- Explain preset patterns and their use cases
- Document AST-level test detection algorithm

### User Documentation

Add to debtmap user guide:

```markdown
## Excluding Files from Analysis

### Basic Exclusions

Exclude specific patterns:
```bash
# Exclude all test files
debtmap analyze src --exclude '**/*_test.rs'

# Exclude multiple patterns
debtmap analyze src --exclude tests --exclude vendor --exclude generated

# Comma-separated patterns
debtmap analyze src --exclude 'tests/**,vendor/**,**/*_generated.rs'
```

### Excluding Rust Test Modules

Exclude `#[cfg(test)]` modules from file-level scores:
```bash
debtmap analyze src --exclude-test-modules
```

This prevents test code from inflating file complexity scores while still analyzing production code in the same files.

### Using Presets

Apply common exclusion patterns:
```bash
# Exclude common non-production code
debtmap analyze src --exclude-preset common

# Exclude only test-related files
debtmap analyze src --exclude-preset tests

# Combine presets with custom patterns
debtmap analyze src --exclude-preset common --exclude 'internal/**'
```

Preset patterns:
- `common`: tests, vendor, node_modules, target, build, dist
- `tests`: test files, test modules, spec files
- `generated`: generated code suffixes (_generated, .pb, etc.)

### Configuration File

Add to `.debtmap.toml`:
```toml
[analysis]
exclude_patterns = [
    "tests/**",
    "vendor/**",
    "**/*_generated.rs"
]
exclude_test_modules = true
exclude_presets = ["common"]
```
```

### Architecture Documentation

Update ARCHITECTURE.md:
- Explain exclusion system architecture
- Document integration with file walker
- Describe AST-level filtering approach

## Implementation Notes

### Pattern Matching Strategy

1. **Use glob crate**: Already a dependency, fast pattern matching
2. **Compile patterns once**: Pre-compile all glob patterns at startup
3. **Check exclusions early**: Filter during file discovery, not after parsing
4. **Path normalization**: Handle Windows/Unix path separators consistently

### AST-Level Exclusion Considerations

1. **Test module detection**:
   - Check for `#[cfg(test)]` attribute on `mod` items
   - Handle nested modules (test modules within test modules)
   - Preserve non-test code in files with test modules

2. **Score aggregation impact**:
   - File-level scores should exclude test function complexity
   - Keep test functions in output if not using `--exclude-test-modules`
   - Document behavior clearly in output

3. **Performance optimization**:
   - Cache test module detection results
   - Skip AST traversal for fully excluded files
   - Use parallel processing for large codebases

### Edge Cases

1. **Overlapping patterns**: Last pattern wins (or most specific?)
2. **Symlinks**: Follow symlinks or exclude them?
3. **Hidden files**: Should `.hidden` files be auto-excluded?
4. **Case sensitivity**: Match case-insensitively on Windows?

### Preset Pattern Definitions

```rust
pub const PRESET_COMMON: &[&str] = &[
    "**/tests/**",
    "**/test/**",
    "**/*_test.rs",
    "**/*_test.py",
    "**/vendor/**",
    "**/node_modules/**",
    "**/target/**",
    "**/build/**",
    "**/dist/**",
];

pub const PRESET_TESTS: &[&str] = &[
    "**/tests/**",
    "**/test/**",
    "**/*_test.*",
    "**/*_spec.*",
    "**/spec/**",
];

pub const PRESET_GENERATED: &[&str] = &[
    "**/*_generated.*",
    "**/*_gen.*",
    "**/*.pb.*",
    "**/*.pb.go",
    "**/*_pb2.py",
];
```

## Migration and Compatibility

### Backward Compatibility

- **No breaking changes**: Existing CLI and JSON output unchanged
- **Opt-in feature**: Users must explicitly add `--exclude` flags
- **Default behavior**: Without exclusions, analyze all files (current behavior)

### Migration Path

For users wanting to exclude test code:

1. **Simple approach**: Add `--exclude tests` to commands
2. **Rust-specific**: Use `--exclude-test-modules` for inline test modules
3. **Config file**: Add `exclude_patterns` to `.debtmap.toml` for persistent exclusions
4. **Presets**: Start with `--exclude-preset common` and adjust

### Compatibility with Existing Workflows

- **MapReduce workflows**: Update `debtmap-reduce.yml` to use exclusions:
  ```yaml
  setup:
    - shell: "debtmap analyze src --exclude-test-modules --lcov target/coverage/lcov.info --output .prodigy/debtmap-before.json --format json"
  ```

- **JSON output**: Add optional `exclusion_summary` field to metadata (backward compatible)

### Deprecation Strategy

None required (new feature, no deprecations)

## Future Enhancements

1. **Include patterns**: `--include` to explicitly whitelist files
2. **Regex support**: `--exclude-regex` for advanced patterns
3. **Interactive mode**: Suggest common exclusions based on detected patterns
4. **Exclusion analysis**: Report what would be excluded before analyzing
5. **Language-specific presets**: `--exclude-preset python-tests`, `--exclude-preset rust-tests`
6. **Exclude by metric**: `--exclude-low-priority` to skip files below threshold

## Success Metrics

- **Adoption**: 30% of users add exclusion patterns within 3 months
- **Performance**: < 5% overhead for exclusion pattern matching
- **Accuracy**: Zero false exclusions (excluding files that should be analyzed)
- **Usability**: < 3 support questions about exclusion patterns per month
- **Use cases**: Covers 90% of common exclusion scenarios (tests, vendor, generated)
