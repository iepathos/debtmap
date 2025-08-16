---
number: 34
title: Language-Specific Dead Code Handling
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-08-16
---

# Specification 34: Language-Specific Dead Code Handling

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: []

## Context

Currently, debtmap performs dead code detection uniformly across all languages. However, different language ecosystems have varying levels of built-in tooling for detecting unused code:

- **Rust**: The Rust compiler provides comprehensive dead code detection via `#[warn(dead_code)]` by default, making debtmap's detection redundant
- **Python**: Limited dead code detection - tools like `vulture` exist but aren't standard
- **JavaScript/TypeScript**: No built-in dead code detection in the runtime, though some bundlers detect it

Performing redundant dead code analysis for Rust:
1. Wastes computational resources
2. Duplicates warnings users already see
3. Adds noise to the debt report
4. May conflict with intentional `#[allow(dead_code)]` annotations

## Objective

Implement language-specific dead code detection that:

1. **Disables dead code detection for Rust** - Rely on rustc's superior built-in detection
2. **Maintains detection for Python** - Provide value where tooling is limited
3. **Maintains detection for JavaScript/TypeScript** - Fill gap in ecosystem tooling
4. **Preserves cross-language analysis** - Keep unified reporting for multi-language projects

## Requirements

### Functional Requirements

1. **Language-Specific Configuration**
   - Add per-language feature flags for dead code detection
   - Default: Rust=false, Python=true, JavaScript=true, TypeScript=true
   - Allow override via configuration file

2. **Rust-Specific Changes**
   - Skip dead code analysis in `analyzers/rust_call_graph.rs`
   - Remove `DeadCode` debt type from Rust analysis results
   - Preserve call graph construction for other analyses
   - Maintain test detection for coverage analysis

3. **Other Language Preservation**
   - Keep existing dead code detection for Python
   - Keep existing dead code detection for JavaScript/TypeScript
   - Ensure no regression in non-Rust languages

4. **Configuration Schema**
   ```toml
   [languages.rust]
   detect_dead_code = false  # Default: false (rustc handles this)
   
   [languages.python]
   detect_dead_code = true   # Default: true
   
   [languages.javascript]
   detect_dead_code = true   # Default: true
   
   [languages.typescript]
   detect_dead_code = true   # Default: true
   ```

### Non-Functional Requirements

- No performance regression for other analyses
- Maintain backward compatibility for existing configs
- Clear documentation of language-specific behaviors

## Acceptance Criteria

- [ ] Dead code detection disabled by default for Rust
- [ ] Dead code detection remains enabled for Python, JS, TS
- [ ] Configuration option to override defaults per language
- [ ] No `DeadCode` items in Rust analysis output
- [ ] Call graph still constructed for Rust (needed for other analyses)
- [ ] Tests updated to reflect language-specific behavior
- [ ] Documentation updated with language-specific features
- [ ] No performance regression in analysis speed

## Technical Details

### Implementation Approach

1. **Add Language Feature Flags**
   ```rust
   pub struct LanguageConfig {
       pub detect_dead_code: bool,
       pub detect_complexity: bool,  // Always true
       pub detect_duplication: bool,  // Always true
   }
   
   impl Default for LanguageConfig {
       fn default() -> Self {
           Self {
               detect_dead_code: true,
               detect_complexity: true,
               detect_duplication: true,
           }
       }
   }
   
   pub fn get_language_config(lang: Language) -> LanguageConfig {
       match lang {
           Language::Rust => LanguageConfig {
               detect_dead_code: false,  // Rustc handles this
               ..Default::default()
           },
           Language::Python | Language::JavaScript | Language::TypeScript => {
               LanguageConfig::default()
           },
           _ => LanguageConfig::default(),
       }
   }
   ```

2. **Modify Dead Code Detection**
   ```rust
   // In analyze_dead_code function
   pub fn analyze_dead_code(
       graph: &CallGraph,
       config: &Config,
       language: Language,
   ) -> Vec<DebtItem> {
       let lang_config = get_language_config(language);
       
       if !lang_config.detect_dead_code {
           return Vec::new();  // Skip for this language
       }
       
       // Existing dead code analysis...
   }
   ```

3. **Update Priority Analysis**
   ```rust
   // In priority/mod.rs
   fn should_detect_dead_code(file_path: &Path, config: &Config) -> bool {
       let language = detect_language(file_path);
       let lang_config = get_language_config(language);
       
       // Allow config override
       config.languages
           .get(&language)
           .and_then(|lc| lc.detect_dead_code)
           .unwrap_or(lang_config.detect_dead_code)
   }
   ```

### Architecture Changes

- Add `LanguageConfig` struct to `core/config.rs`
- Modify `analyze_dead_code` to check language config
- Update configuration parser to support per-language settings
- Preserve call graph construction independent of dead code detection

### Migration Path

1. Existing configs continue to work (backward compatible)
2. Dead code detection automatically disabled for Rust files
3. Users can opt back in via configuration if desired

## Dependencies

- No new external dependencies
- Internal refactoring of existing dead code detection

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_rust_dead_code_disabled_by_default() {
    let config = Config::default();
    let rust_config = get_language_config(Language::Rust);
    assert!(!rust_config.detect_dead_code);
}

#[test]
fn test_python_dead_code_enabled_by_default() {
    let config = Config::default();
    let python_config = get_language_config(Language::Python);
    assert!(python_config.detect_dead_code);
}

#[test]
fn test_config_override() {
    let mut config = Config::default();
    config.languages.rust.detect_dead_code = Some(true);
    // Should override default false for Rust
    assert!(should_detect_dead_code("main.rs", &config));
}
```

### Integration Tests

- Test multi-language project with mixed dead code
- Verify Rust files produce no dead code warnings
- Verify Python/JS files still produce dead code warnings
- Test configuration overrides work correctly

## Documentation Requirements

### User Documentation

Update README.md:
```markdown
## Language-Specific Features

Debtmap tailors its analysis to each language's ecosystem:

### Rust
- **Dead code**: Disabled by default (rustc provides superior detection)
- **Complexity**: Enabled (adds coverage integration and ROI scoring)
- **Duplication**: Enabled (not detected by rustc/clippy)

### Python, JavaScript, TypeScript
- **Dead code**: Enabled (fills gap in ecosystem tooling)
- **All other analyses**: Enabled

### Overriding Defaults

```toml
[languages.rust]
detect_dead_code = true  # Re-enable if needed

[languages.python]
detect_dead_code = false  # Disable if using other tools
```
```

### Code Documentation

```rust
/// Language-specific analysis configuration
/// 
/// Different languages have different levels of built-in tooling.
/// This configuration allows debtmap to avoid redundant analysis
/// while filling gaps where ecosystem tooling is limited.
///
/// # Defaults
/// - Rust: Dead code detection disabled (rustc handles this)
/// - Python: All detection enabled (limited ecosystem tooling)
/// - JavaScript/TypeScript: All detection enabled (no built-in analysis)
```

## Implementation Notes

### Rationale for Rust Exclusion

The Rust compiler's dead code detection is:
1. More accurate (understands macros, generics, trait implementations)
2. Integrated with the build process
3. Respects `#[allow(dead_code)]` attributes
4. Handles cross-crate visibility correctly

Debtmap should focus on what rustc doesn't provide:
- Coverage-based risk analysis
- Complexity scoring with ROI
- Duplication detection
- Architectural debt

### Future Considerations

Consider similar optimizations for other languages:
- TypeScript: Could defer to `tsc --noUnusedLocals`
- Python: Could integrate with `vulture` if present
- Go: Could defer to `go vet`

## Performance Impact

Disabling dead code detection for Rust should:
- Reduce analysis time by ~10-15% for Rust projects
- Reduce memory usage during call graph analysis
- Simplify the analysis pipeline

## Migration and Compatibility

- No breaking changes to existing API
- Existing configurations remain valid
- Default behavior changes only for Rust files
- Users can restore old behavior via configuration