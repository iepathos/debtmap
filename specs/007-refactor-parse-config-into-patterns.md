---
number: 7
title: Refactor parse_config_into_patterns TOML Parser
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-21
---

# Specification 7: Refactor parse_config_into_patterns TOML Parser

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: none

## Context

The `parse_config_into_patterns` function in `src/analysis/framework_patterns_multi/detector.rs:313` has concerning metrics:
- Nesting depth: 7 (extremely deep)
- Cognitive complexity: 42
- Bug density: 100%
- Lines of code: 54
- Score: 72.46

The function parses nested TOML configuration into framework patterns. The current implementation uses deeply nested `if let` chains to navigate the TOML structure, making it fragile and hard to maintain.

Following Stillwater's "Composition Over Complexity" principle, this should be refactored into a pipeline of small, focused parsing functions.

## Objective

Refactor `parse_config_into_patterns` to:
1. Reduce nesting depth from 7 to ≤2
2. Extract pure parsing helpers for each level
3. Improve error handling with context
4. Make the parsing logic testable at each level

## Requirements

### Functional Requirements

1. **Level-by-level parsing**: Separate functions for each TOML nesting level
2. **Result chaining**: Use `?` operator and `and_then` for cleaner error flow
3. **Preserve fallback behavior**: Handle both nested and flat framework definitions
4. **Better error messages**: Include TOML path in error context

### Non-Functional Requirements

1. **Readability**: Each function should be ≤10 lines
2. **Testability**: Each level independently testable
3. **Maintainability**: Adding new config structure should be straightforward

## Acceptance Criteria

- [ ] Nesting depth reduced from 7 to ≤2
- [ ] Cognitive complexity reduced from 42 to ≤15
- [ ] Test coverage reaches ≥70%
- [ ] All existing TOML configs parse correctly
- [ ] Error messages include TOML path context
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean

## Technical Details

### Current Structure (Problem)

The current code has 7 levels of nesting:
```rust
if let Some(table) = config.as_table() {           // Level 1
    for (lang_key, lang_value) in table {          // Level 2
        if let Some(category_table) = ... {        // Level 3
            for (_category_key, framework_table) { // Level 4
                if let Some(framework_items) = ... { // Level 5
                    for (_framework_key, pattern_value) { // Level 6
                        match pattern_value.clone().try_into() { // Level 7
                            Ok(framework_pattern) => {
                                patterns.entry(...).or_default().push(framework_pattern);
                            }
                            Err(e) => { ... }
                        }
                    }
                } else { ... }
            }
        }
    }
}
```

### Implementation Approach: Flatten with Iterator Chain

```rust
/// Parse TOML config into framework patterns (entry point)
fn parse_config_into_patterns(
    config: &toml::Value,
) -> Result<HashMap<Language, Vec<FrameworkPattern>>> {
    config
        .as_table()
        .ok_or_else(|| anyhow!("Config must be a TOML table"))?
        .iter()
        .try_fold(HashMap::new(), |mut acc, (lang_key, lang_value)| {
            let patterns = parse_language_patterns(lang_key, lang_value)?;
            acc.entry(patterns.0).or_default().extend(patterns.1);
            Ok(acc)
        })
}

/// Parse patterns for a single language
fn parse_language_patterns(
    lang_key: &str,
    lang_value: &toml::Value,
) -> Result<(Language, Vec<FrameworkPattern>)> {
    let language = Language::parse(lang_key)
        .context(format!("Invalid language key: {}", lang_key))?;

    let patterns = lang_value
        .as_table()
        .ok_or_else(|| anyhow!("Language '{}' must be a table", lang_key))?
        .iter()
        .flat_map(|(category, value)| parse_category_patterns(lang_key, category, value))
        .collect();

    Ok((language, patterns))
}

/// Parse patterns from a category (e.g., "web", "testing")
fn parse_category_patterns(
    lang_key: &str,
    category_key: &str,
    category_value: &toml::Value,
) -> Vec<FrameworkPattern> {
    // Try as table of frameworks first
    if let Some(frameworks) = category_value.as_table() {
        frameworks
            .iter()
            .filter_map(|(name, value)| parse_single_pattern(lang_key, category_key, name, value))
            .collect()
    } else {
        // Try parsing the category itself as a pattern
        parse_single_pattern(lang_key, "", category_key, category_value)
            .into_iter()
            .collect()
    }
}

/// Parse a single framework pattern with error context
fn parse_single_pattern(
    lang: &str,
    category: &str,
    name: &str,
    value: &toml::Value,
) -> Option<FrameworkPattern> {
    value
        .clone()
        .try_into::<FrameworkPattern>()
        .map_err(|e| {
            let path = if category.is_empty() {
                format!("{}.{}", lang, name)
            } else {
                format!("{}.{}.{}", lang, category, name)
            };
            eprintln!("Warning: Failed to parse pattern at {}: {}", path, e);
            e
        })
        .ok()
}
```

### Benefits

1. **Flat structure**: Maximum nesting depth of 2
2. **Pure functions**: `parse_language_patterns`, `parse_category_patterns`, `parse_single_pattern` are pure
3. **Error context**: Each level adds path information
4. **Testable**: Each parsing level can be tested independently
5. **Composable**: Uses iterator chains following functional patterns

### Alternative: Recursive Descent with Type

```rust
enum TomlNode<'a> {
    Language(&'a str, &'a toml::Value),
    Category(&'a str, &'a str, &'a toml::Value),
    Pattern(&'a str, &'a str, &'a str, &'a toml::Value),
}

fn walk_config(config: &toml::Value) -> impl Iterator<Item = TomlNode<'_>> {
    // ... iterator that yields nodes at each level
}

fn parse_config_into_patterns(config: &toml::Value) -> Result<HashMap<Language, Vec<FrameworkPattern>>> {
    walk_config(config)
        .filter_map(|node| match node {
            TomlNode::Pattern(lang, cat, name, value) => {
                parse_single_pattern(lang, cat, name, value)
            }
            _ => None,
        })
        .try_fold(HashMap::new(), |mut acc, (lang, pattern)| {
            acc.entry(lang).or_default().push(pattern);
            Ok(acc)
        })
}
```

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_valid_pattern() {
        let toml_str = r#"
            name = "axum"
            patterns = ["Router", "handler"]
        "#;
        let value: toml::Value = toml::from_str(toml_str).unwrap();
        let pattern = parse_single_pattern("rust", "web", "axum", &value);
        assert!(pattern.is_some());
    }

    #[test]
    fn test_parse_invalid_pattern_returns_none_with_warning() {
        let value = toml::Value::String("not a pattern".into());
        let pattern = parse_single_pattern("rust", "web", "bad", &value);
        assert!(pattern.is_none());
    }

    #[test]
    fn test_parse_language_patterns() {
        let toml_str = r#"
            [web.axum]
            name = "axum"
            patterns = ["Router"]
        "#;
        let config: toml::Value = toml::from_str(toml_str).unwrap();
        let (lang, patterns) = parse_language_patterns("rust", &config).unwrap();
        assert_eq!(lang, Language::Rust);
        assert!(!patterns.is_empty());
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: `src/analysis/framework_patterns_multi/detector.rs`
- **External Dependencies**: None (uses existing `toml` crate)

## Testing Strategy

- **Unit Tests**: Test each parsing level independently
- **Integration Tests**: Parse real framework config files
- **Error Cases**: Invalid TOML, missing fields, type mismatches

## Documentation Requirements

- **Code Documentation**: Document expected TOML structure at each level
- **User Documentation**: Update framework config format documentation if exists

## Implementation Notes

The fallback behavior (trying to parse at category level if framework level fails) is preserved. This allows both nested configs:

```toml
[rust.web.axum]
name = "axum"
patterns = ["Router"]
```

And flat configs:

```toml
[rust.testing]
name = "testing"
patterns = ["#[test]"]
```

The refactoring should maintain this flexibility while making the code clearer.

## Migration and Compatibility

No breaking changes. All existing config files continue to work.
