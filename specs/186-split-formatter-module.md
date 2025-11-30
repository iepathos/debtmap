---
number: 186
title: Split formatter.rs into Focused Submodules
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 186: Split formatter.rs into Focused Submodules

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The `src/formatter.rs` file has grown to **3,094 lines**, violating the Stillwater philosophy principle of "Composition Over Complexity" and the project's guideline of keeping files under 200 lines. This massive file contains multiple distinct responsibilities:

1. **Formatting rules** - Logic for how to format different data types
2. **Output generation** - Converting formatted data to strings (I/O)
3. **Data structures** - Types for formatting configuration and results
4. **Format-specific logic** - JSON, YAML, text, markdown formatters
5. **Color and styling** - Terminal color and style application

According to STILLWATER_EVALUATION.md (lines 689-695), this file should be split into focused submodules following the pattern established in spec 181 (god_object_detector split). Large monolithic files make it difficult to:

- Navigate and understand the code
- Test individual components
- Apply functional programming principles
- Maintain and modify formatting logic
- Collaborate effectively (merge conflicts)

The file mixes pure formatting logic (transforming data) with I/O operations (writing output), violating the "Pure Core, Imperative Shell" pattern.

## Objective

Refactor `src/formatter.rs` (3,094 lines) into a modular structure under `src/format/` with clear separation of concerns:

- **rules.rs** - Pure formatting rules and transformations (no I/O)
- **output.rs** - I/O operations for writing formatted output
- **types.rs** - Data structures and type definitions
- **json.rs** - JSON-specific formatting logic
- **yaml.rs** - YAML-specific formatting logic
- **text.rs** - Plain text formatting logic
- **markdown.rs** - Markdown formatting logic
- **style.rs** - Color and style application (terminal formatting)
- **mod.rs** - Public API and composition

Each module should be under 500 lines and follow functional programming principles with pure core functions and I/O at boundaries.

## Requirements

### Functional Requirements

1. **Module Structure**
   - Create `src/format/` directory
   - Split functionality into 9 focused modules
   - Each module has single responsibility
   - No module exceeds 500 lines

2. **Pure Core Implementation**
   - Formatting rules operate on data structures (no I/O)
   - Format-specific logic uses pure transformations
   - Style application is pure (returns styled string)
   - All business logic deterministic

3. **I/O Separation**
   - File writing in `output.rs` only
   - No print statements in formatting logic
   - Progress tracking separate from formatting
   - Clear boundary between pure and impure

4. **Public API**
   - Preserve existing public API in `mod.rs`
   - Re-export necessary types and functions
   - Maintain backward compatibility
   - Clear documentation of module boundaries

5. **Dependency Direction**
   - `types.rs` has no internal dependencies (foundation)
   - `style.rs` depends only on `types.rs`
   - `rules.rs` depends on `types.rs` and `style.rs`
   - Format-specific modules (`json.rs`, `yaml.rs`, etc.) depend on `types.rs` and `rules.rs`
   - `output.rs` depends on all modules (top layer)
   - `mod.rs` composes all modules

### Non-Functional Requirements

1. **Performance**
   - No performance regression from refactoring
   - Same memory usage characteristics
   - Efficient string handling

2. **Maintainability**
   - Each module independently testable
   - Clear boundaries between concerns
   - Comprehensive module-level documentation
   - Easy to add new output formats

3. **Testability**
   - Existing tests continue to pass
   - New tests added for individual modules
   - Unit tests for pure formatting functions
   - Integration tests for full pipeline

## Acceptance Criteria

- [ ] Directory `src/format/` created with 9 module files
- [ ] `types.rs` contains data structures (<300 lines)
- [ ] `style.rs` contains color/style logic (<200 lines)
- [ ] `rules.rs` contains pure formatting rules (<500 lines)
- [ ] `json.rs` contains JSON formatting (<400 lines)
- [ ] `yaml.rs` contains YAML formatting (<400 lines)
- [ ] `text.rs` contains text formatting (<400 lines)
- [ ] `markdown.rs` contains markdown formatting (<400 lines)
- [ ] `output.rs` contains I/O operations (<300 lines)
- [ ] `mod.rs` provides public API (<200 lines)
- [ ] Original `formatter.rs` deleted
- [ ] All existing tests pass without modification
- [ ] Each module has module-level documentation
- [ ] Pure functions separated from I/O operations
- [ ] No circular dependencies between modules
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes with no failures

## Technical Details

### Implementation Approach

**Current Structure (Monolithic):**
```
src/formatter.rs (3,094 lines)
  ├─ FormattingConfig struct
  ├─ format_results() function
  ├─ format_json() function
  ├─ format_yaml() function
  ├─ format_text() function
  ├─ format_markdown() function
  ├─ apply_colors() function
  ├─ write_output() function
  └─ ... hundreds more functions
```

**Target Structure (Modular):**
```
src/format/
  ├─ mod.rs          (~200 lines - public API)
  ├─ types.rs        (~300 lines - data structures)
  ├─ style.rs        (~200 lines - color/style)
  ├─ rules.rs        (~500 lines - formatting rules)
  ├─ json.rs         (~400 lines - JSON formatting)
  ├─ yaml.rs         (~400 lines - YAML formatting)
  ├─ text.rs         (~400 lines - text formatting)
  ├─ markdown.rs     (~400 lines - markdown formatting)
  └─ output.rs       (~300 lines - I/O operations)
```

### Module Responsibilities

**types.rs (Foundation - Data Structures)**

```rust
//! Core data types for formatting.
//!
//! This module contains all data structures used across the formatting
//! system. No dependencies on other modules.

/// Configuration for formatting output.
#[derive(Debug, Clone)]
pub struct FormattingConfig {
    pub format: OutputFormat,
    pub color: bool,
    pub show_metrics: bool,
    pub show_context: bool,
    pub max_context_lines: usize,
}

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Yaml,
    Text,
    Markdown,
}

/// Formatted output data.
#[derive(Debug, Clone)]
pub struct FormattedOutput {
    pub content: String,
    pub format: OutputFormat,
    pub metadata: OutputMetadata,
}

/// Metadata about formatted output.
#[derive(Debug, Clone)]
pub struct OutputMetadata {
    pub item_count: usize,
    pub total_issues: usize,
    pub formatted_at: DateTime<Utc>,
}

/// Style for terminal output.
#[derive(Debug, Clone)]
pub struct Style {
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

/// Terminal colors.
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}
```

**style.rs (Pure - Color and Style Application)**

```rust
//! Terminal color and style application.
//!
//! Pure functions for applying colors and styles to text.
//! No I/O operations.

use crate::format::types::{Color, Style};

/// Applies style to text (pure function).
///
/// Returns styled string without performing any I/O.
pub fn apply_style(text: &str, style: &Style) -> String {
    if !should_use_colors() {
        return text.to_string();
    }

    let mut styled = String::new();

    if let Some(fg) = style.fg_color {
        styled.push_str(&color_code(fg));
    }

    if style.bold {
        styled.push_str("\x1b[1m");
    }

    if style.italic {
        styled.push_str("\x1b[3m");
    }

    if style.underline {
        styled.push_str("\x1b[4m");
    }

    styled.push_str(text);
    styled.push_str("\x1b[0m"); // Reset

    styled
}

/// Returns ANSI color code for given color (pure).
fn color_code(color: Color) -> &'static str {
    match color {
        Color::Black => "\x1b[30m",
        Color::Red => "\x1b[31m",
        Color::Green => "\x1b[32m",
        Color::Yellow => "\x1b[33m",
        Color::Blue => "\x1b[34m",
        Color::Magenta => "\x1b[35m",
        Color::Cyan => "\x1b[36m",
        Color::White => "\x1b[37m",
    }
}

/// Checks if colors should be used (reads env, but cached).
fn should_use_colors() -> bool {
    // Cache result to make function mostly pure
    static USE_COLORS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

    *USE_COLORS.get_or_init(|| {
        std::env::var("NO_COLOR").is_err()
            && atty::is(atty::Stream::Stdout)
    })
}

/// Predefined styles for common use cases.
pub mod styles {
    use super::*;

    pub fn error() -> Style {
        Style {
            fg_color: Some(Color::Red),
            bold: true,
            ..Default::default()
        }
    }

    pub fn warning() -> Style {
        Style {
            fg_color: Some(Color::Yellow),
            ..Default::default()
        }
    }

    pub fn success() -> Style {
        Style {
            fg_color: Some(Color::Green),
            ..Default::default()
        }
    }

    pub fn info() -> Style {
        Style {
            fg_color: Some(Color::Blue),
            ..Default::default()
        }
    }
}
```

**rules.rs (Pure - Formatting Rules)**

```rust
//! Pure formatting rules and transformations.
//!
//! Functions for transforming analysis results into structured
//! format-agnostic representations. All functions are pure.

use crate::format::types::*;

/// Formats analysis results into structured data (pure).
///
/// This function transforms raw analysis data into a format-agnostic
/// intermediate representation. No I/O performed.
pub fn format_analysis_results(
    results: &AnalysisResults,
    config: &FormattingConfig,
) -> FormattedData {
    FormattedData {
        items: format_items(&results.items, config),
        summary: format_summary(&results.summary, config),
        metadata: format_metadata(&results.metadata),
    }
}

/// Formats individual items (pure).
fn format_items(
    items: &[AnalysisItem],
    config: &FormattingConfig,
) -> Vec<FormattedItem> {
    items
        .iter()
        .map(|item| format_item(item, config))
        .collect()
}

/// Formats a single item (pure).
fn format_item(
    item: &AnalysisItem,
    config: &FormattingConfig,
) -> FormattedItem {
    FormattedItem {
        title: format_title(&item.title),
        description: format_description(&item.description, config),
        metrics: if config.show_metrics {
            Some(format_metrics(&item.metrics))
        } else {
            None
        },
        context: if config.show_context {
            Some(format_context(&item.context, config.max_context_lines))
        } else {
            None
        },
    }
}

/// Formats summary section (pure).
fn format_summary(
    summary: &AnalysisSummary,
    config: &FormattingConfig,
) -> FormattedSummary {
    FormattedSummary {
        total_items: summary.total_items,
        by_severity: format_severity_breakdown(&summary.by_severity),
        by_category: format_category_breakdown(&summary.by_category),
    }
}

// ... more pure formatting functions
```

**json.rs (Pure - JSON Formatting)**

```rust
//! JSON formatting implementation.
//!
//! Converts structured data to JSON format. Pure functions only.

use crate::format::{rules::FormattedData, types::*};
use serde_json::{json, Value};

/// Converts formatted data to JSON (pure).
pub fn to_json(data: &FormattedData, config: &FormattingConfig) -> Value {
    json!({
        "items": format_items_json(&data.items, config),
        "summary": format_summary_json(&data.summary),
        "metadata": format_metadata_json(&data.metadata),
    })
}

/// Formats items as JSON array (pure).
fn format_items_json(
    items: &[FormattedItem],
    config: &FormattingConfig,
) -> Value {
    let json_items: Vec<Value> = items
        .iter()
        .map(|item| format_item_json(item, config))
        .collect();

    Value::Array(json_items)
}

/// Formats single item as JSON object (pure).
fn format_item_json(
    item: &FormattedItem,
    config: &FormattingConfig,
) -> Value {
    let mut obj = json!({
        "title": item.title,
        "description": item.description,
    });

    if let Some(ref metrics) = item.metrics {
        obj["metrics"] = format_metrics_json(metrics);
    }

    if config.show_context {
        if let Some(ref context) = item.context {
            obj["context"] = format_context_json(context);
        }
    }

    obj
}

// ... more JSON formatting functions
```

**yaml.rs, text.rs, markdown.rs (Similar Structure)**

Each format-specific module follows the same pattern:
- Pure functions that transform `FormattedData` to format-specific representation
- No I/O operations
- Deterministic transformations
- Easily testable

**output.rs (I/O - Writing Output)**

```rust
//! Output operations (I/O layer).
//!
//! Functions for writing formatted output to files, stdout, etc.
//! This is the I/O boundary - all functions here perform side effects.

use crate::format::{json, markdown, text, types::*, yaml};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Writes formatted output to destination (I/O).
///
/// This function performs I/O operations and is not pure.
pub fn write_output(
    data: &FormattedData,
    config: &FormattingConfig,
    dest: OutputDestination,
) -> io::Result<()> {
    let content = render_to_string(data, config)?;

    match dest {
        OutputDestination::Stdout => write_to_stdout(&content),
        OutputDestination::Stderr => write_to_stderr(&content),
        OutputDestination::File(path) => write_to_file(&content, &path),
    }
}

/// Renders formatted data to string (pure-ish).
fn render_to_string(
    data: &FormattedData,
    config: &FormattingConfig,
) -> io::Result<String> {
    match config.format {
        OutputFormat::Json => {
            let json = json::to_json(data, config);
            Ok(serde_json::to_string_pretty(&json)?)
        }
        OutputFormat::Yaml => {
            let yaml = yaml::to_yaml(data, config);
            Ok(serde_yaml::to_string(&yaml)?)
        }
        OutputFormat::Text => Ok(text::to_text(data, config)),
        OutputFormat::Markdown => Ok(markdown::to_markdown(data, config)),
    }
}

/// Writes content to stdout (I/O).
fn write_to_stdout(content: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(content.as_bytes())?;
    handle.flush()
}

/// Writes content to file (I/O).
fn write_to_file(content: &str, path: &Path) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    file.flush()
}

// ... more I/O functions
```

**mod.rs (Public API - Composition)**

```rust
//! Formatting module.
//!
//! Provides functionality for formatting analysis results in various
//! output formats (JSON, YAML, text, markdown).
//!
//! # Architecture
//!
//! - `types` - Core data structures
//! - `style` - Color and style application (pure)
//! - `rules` - Formatting rules (pure)
//! - `json`, `yaml`, `text`, `markdown` - Format-specific logic (pure)
//! - `output` - I/O operations (impure)
//!
//! # Examples
//!
//! ```
//! use debtmap::format::{format_results, FormattingConfig, OutputFormat};
//!
//! let config = FormattingConfig {
//!     format: OutputFormat::Json,
//!     color: true,
//!     ..Default::default()
//! };
//!
//! let formatted = format_results(&analysis_results, &config);
//! ```

pub mod types;
pub mod style;
pub mod rules;
pub mod json;
pub mod yaml;
pub mod text;
pub mod markdown;
pub mod output;

// Re-exports for convenience
pub use types::{
    FormattedOutput, FormattingConfig, OutputFormat, OutputMetadata,
};
pub use output::{write_output, OutputDestination};

/// High-level API: Format and write results.
///
/// This composes all formatting modules to provide a simple API.
pub fn format_and_write(
    results: &AnalysisResults,
    config: &FormattingConfig,
    dest: OutputDestination,
) -> io::Result<()> {
    // Pure: Format data
    let formatted_data = rules::format_analysis_results(results, config);

    // I/O: Write output
    output::write_output(&formatted_data, config, dest)
}

/// Pure API: Format results without I/O.
///
/// Useful for testing and cases where you want the formatted data
/// without writing it anywhere.
pub fn format_results(
    results: &AnalysisResults,
    config: &FormattingConfig,
) -> FormattedData {
    rules::format_analysis_results(results, config)
}
```

### Architecture Changes

**Before:**
```
src/
  formatter.rs (3,094 lines - everything mixed together)
```

**After:**
```
src/
  format/
    mod.rs         (~200 lines - public API, composition)
    types.rs       (~300 lines - data structures, no dependencies)
    style.rs       (~200 lines - pure color/style functions)
    rules.rs       (~500 lines - pure formatting rules)
    json.rs        (~400 lines - pure JSON formatting)
    yaml.rs        (~400 lines - pure YAML formatting)
    text.rs        (~400 lines - pure text formatting)
    markdown.rs    (~400 lines - pure markdown formatting)
    output.rs      (~300 lines - I/O operations)
```

### Dependency Graph

```
                    types.rs (foundation)
                       ↑
                       |
         ┌─────────────┼─────────────┐
         ↓             ↓             ↓
     style.rs      rules.rs     (other modules)
         ↑             ↑
         |             |
         └─────────────┼──────────────┐
                       ↓              ↓
            json/yaml/text/markdown   output.rs
                       ↓
                    mod.rs (composition)
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/formatter.rs` - Will be deleted
  - All code importing from `formatter` - Update imports
  - Tests importing formatter functions
- **External Dependencies**: None (uses existing dependencies)

## Testing Strategy

### Unit Tests (Per Module)

```rust
// format/rules.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_analysis_results_pure() {
        let results = create_test_results();
        let config = FormattingConfig::default();

        let formatted1 = format_analysis_results(&results, &config);
        let formatted2 = format_analysis_results(&results, &config);

        // Deterministic - same input, same output
        assert_eq!(formatted1.items.len(), formatted2.items.len());
    }

    #[test]
    fn test_format_items_respects_config() {
        let items = create_test_items();
        let config_with_metrics = FormattingConfig {
            show_metrics: true,
            ..Default::default()
        };
        let config_without_metrics = FormattingConfig {
            show_metrics: false,
            ..Default::default()
        };

        let with_metrics = format_items(&items, &config_with_metrics);
        let without_metrics = format_items(&items, &config_without_metrics);

        assert!(with_metrics[0].metrics.is_some());
        assert!(without_metrics[0].metrics.is_none());
    }
}

// format/json.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_json_structure() {
        let data = create_test_formatted_data();
        let config = FormattingConfig::default();

        let json = to_json(&data, &config);

        assert!(json.is_object());
        assert!(json["items"].is_array());
        assert!(json["summary"].is_object());
    }

    #[test]
    fn test_json_serialization_valid() {
        let data = create_test_formatted_data();
        let config = FormattingConfig::default();

        let json = to_json(&data, &config);
        let serialized = serde_json::to_string(&json);

        assert!(serialized.is_ok());
    }
}

// format/style.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_style_pure() {
        let text = "Hello, world!";
        let style = Style {
            fg_color: Some(Color::Red),
            bold: true,
            ..Default::default()
        };

        let styled1 = apply_style(text, &style);
        let styled2 = apply_style(text, &style);

        assert_eq!(styled1, styled2);
    }
}
```

### Integration Tests

```rust
// tests/formatting_integration.rs
#[test]
fn test_full_formatting_pipeline() {
    let results = load_test_analysis_results();
    let config = FormattingConfig {
        format: OutputFormat::Json,
        color: false,
        show_metrics: true,
        show_context: true,
        max_context_lines: 5,
    };

    // Should complete without error
    let formatted = format_results(&results, &config);

    assert!(!formatted.items.is_empty());
    assert!(formatted.items[0].metrics.is_some());
}

#[test]
fn test_all_output_formats() {
    let results = load_test_analysis_results();

    for format in [OutputFormat::Json, OutputFormat::Yaml,
                   OutputFormat::Text, OutputFormat::Markdown] {
        let config = FormattingConfig {
            format,
            ..Default::default()
        };

        let result = format_results(&results, &config);
        assert!(!result.items.is_empty());
    }
}
```

### Backward Compatibility Tests

```rust
#[test]
fn test_public_api_unchanged() {
    // Verify old API still works
    let results = create_test_results();
    let config = FormattingConfig::default();

    // This should compile and work exactly as before
    let formatted = format_results(&results, &config);
    assert!(formatted.is_ok());
}
```

## Documentation Requirements

### Module-Level Documentation

Each module gets comprehensive docs:

```rust
// format/rules.rs
//! Pure formatting rules and transformations.
//!
//! This module contains the core formatting logic that transforms
//! analysis results into format-agnostic structured data. All
//! functions in this module are pure - they take data as input
//! and return formatted data as output, with no side effects.
//!
//! # Pure Function Properties
//!
//! - No I/O operations
//! - No printing or logging
//! - Deterministic results
//! - No mutation of inputs
//! - Easily testable
//!
//! # Examples
//!
//! ```
//! use debtmap::format::{rules, FormattingConfig};
//!
//! let results = /* analysis results */;
//! let config = FormattingConfig::default();
//!
//! let formatted = rules::format_analysis_results(&results, &config);
//! ```
```

### Architecture Documentation

Update `ARCHITECTURE.md`:

```markdown
## Formatting Module

The formatting system is organized as follows:

```
src/format/
  ├─ types.rs     - Core data structures (no dependencies)
  ├─ style.rs     - Pure color/style functions
  ├─ rules.rs     - Pure formatting rules
  ├─ json.rs      - Pure JSON formatting
  ├─ yaml.rs      - Pure YAML formatting
  ├─ text.rs      - Pure text formatting
  ├─ markdown.rs  - Pure markdown formatting
  ├─ output.rs    - I/O operations (impure)
  └─ mod.rs       - Public API
```

### Separation of Concerns

**Pure Core (Still Water):**
- `types`, `style`, `rules`, format-specific modules
- All functions are deterministic
- No side effects
- Easy to test

**Imperative Shell (Streams):**
- `output.rs` - Writing to files/stdout
- Clear I/O boundary

This structure makes it easy to:
- Test formatting logic without I/O
- Add new output formats
- Modify formatting rules safely
- Compose formatting operations
```

## Implementation Notes

### Refactoring Steps

1. **Create directory structure**
   ```bash
   mkdir -p src/format
   touch src/format/{mod.rs,types.rs,style.rs,rules.rs,json.rs,yaml.rs,text.rs,markdown.rs,output.rs}
   ```

2. **Extract types.rs** (foundation first)
   - Move all data structures
   - No dependencies on other modules
   - Test compilation

3. **Extract style.rs** (simple, self-contained)
   - Move color/style functions
   - Test independently

4. **Extract format-specific modules** (parallel work possible)
   - json.rs, yaml.rs, text.rs, markdown.rs
   - Each can be extracted independently
   - Test each module

5. **Extract rules.rs** (uses types and styles)
   - Move core formatting logic
   - Depends on types and style
   - Test thoroughly

6. **Extract output.rs** (I/O boundary)
   - Move all I/O operations
   - Clear separation from pure code
   - Test with temp files

7. **Create mod.rs** (composition)
   - Public API
   - Re-exports
   - High-level functions

8. **Update imports** throughout codebase
   - Change `use formatter::X` to `use format::X`
   - Or use re-exports from mod.rs

9. **Delete original** formatter.rs
   - Only after all tests pass
   - Verify no remaining references

10. **Update tests**
    - May need to update imports
    - Verify all tests pass

### Common Pitfalls

1. **Circular dependencies** - Careful with module dependencies
2. **Lost functionality** - Ensure all functions moved
3. **Breaking imports** - Update all usage sites
4. **Test breakage** - Update test imports and mocks

### Pure Function Verification

For each module marked "pure", verify:
- [ ] No `std::fs::*` calls
- [ ] No `std::io::Write` usage
- [ ] No `println!` / `eprintln!`
- [ ] No environment variable access
- [ ] Returns same output for same input
- [ ] Can be unit tested without mocks

## Migration and Compatibility

### Breaking Changes

**None** - Public API preserved via re-exports in `mod.rs`.

### Internal Changes

Code using internal formatter functions may need import updates:

```rust
// Before
use crate::formatter::{format_json, FormattingConfig};

// After (if using re-exports)
use crate::format::{format_json, FormattingConfig};

// Or (if using specific modules)
use crate::format::json::to_json;
use crate::format::types::FormattingConfig;
```

### Migration Steps

1. No user action required (internal refactoring)
2. Developers update imports if using internal APIs
3. Tests may need import updates

## Success Metrics

- ✅ 9 modules created, each under 500 lines
- ✅ Pure functions separated from I/O
- ✅ Each module independently testable
- ✅ All existing tests pass
- ✅ No clippy warnings
- ✅ No performance regression
- ✅ Clear module boundaries
- ✅ Comprehensive documentation

## Follow-up Work

After this refactoring:
- Apply same pattern to other large files
- Consider adding more output formats (HTML, CSV)
- Extract common formatting patterns to shared utilities

## References

- **STILLWATER_EVALUATION.md** - Lines 689-695 (formatter.rs split recommendation)
- **Spec 181** - god_object_detector split (similar pattern)
- **Spec 183** - Analyzer I/O separation (Pure Core pattern)
- **CLAUDE.md** - Module boundary guidelines
