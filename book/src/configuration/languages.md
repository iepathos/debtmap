# Language Configuration

Configure language-specific analysis behavior in debtmap.

## Overview

Debtmap analyzes source code for technical debt and complexity issues. Language configuration allows you to:

- Enable or disable specific languages
- Toggle analysis features per language
- Configure language-specific detection behavior

## Supported Languages

**Full Support (AST-Based Analysis):**
- **Rust** - Full AST parsing with `syn`
- **Python** - Full AST parsing with tree-sitter
- **JavaScript** - Tree-sitter parsing for modern JS, JSX, modules, callbacks, promises, and async workflows
- **TypeScript** - Tree-sitter parsing for TS/TSX syntax, type-oriented patterns, modules, and async workflows

**Source**: `src/core/mod.rs` (Language enum) and `src/config/languages.rs` (language-specific configuration fields)

The `Language` enum in the codebase currently supports:

```rust
// From src/core/mod.rs
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Unknown,
}
```

**Language Detection**: Debtmap determines file language by extension:
- `.rs` files are analyzed as Rust
- `.py` and `.pyw` files are analyzed as Python
- `.js`, `.mjs`, `.cjs`, and `.jsx` files are analyzed as JavaScript
- `.ts`, `.mts`, `.cts`, and `.tsx` files are analyzed as TypeScript

**Source**: `src/core/mod.rs`

```rust
pub fn from_extension(ext: &str) -> Self {
    match ext {
        "rs" => Language::Rust,
        "py" | "pyw" => Language::Python,
        "js" | "mjs" | "cjs" | "jsx" => Language::JavaScript,
        "ts" | "mts" | "cts" | "tsx" => Language::TypeScript,
        _ => Language::Unknown,
    }
}
```

## Configuration Structure

The `[languages]` section in your `debtmap.toml` configures language analysis.

**Source**: `src/config/languages.rs:4-22` (LanguagesConfig struct)

```toml
[languages]
enabled = ["rust", "python", "javascript", "typescript"]

[languages.rust]
detect_dead_code = false       # Disabled by default for Rust
detect_complexity = true
detect_duplication = true

[languages.python]
detect_dead_code = true
detect_complexity = true
detect_duplication = true

[languages.javascript]
detect_dead_code = true
detect_complexity = true
detect_duplication = true

[languages.typescript]
detect_dead_code = true
detect_complexity = true
detect_duplication = true
```

## Language-Specific Features

Each language supports three configurable feature toggles:

**Source**: `src/config/languages.rs:24-38` (LanguageFeatures struct)

| Feature | Description | Default |
|---------|-------------|---------|
| `detect_dead_code` | Identify unused code paths | `true` (except Rust) |
| `detect_complexity` | Calculate cyclomatic and cognitive complexity | `true` |
| `detect_duplication` | Find code duplication patterns | `true` |

### Rust Configuration

```toml
[languages.rust]
detect_dead_code = false        # Disabled: rustc already reports unused code
detect_complexity = true
detect_duplication = true
```

**Why dead code detection is disabled for Rust**: The Rust compiler (`rustc`) already provides excellent unused code warnings via `#[warn(dead_code)]`. Enabling debtmap's dead code detection would duplicate these warnings without adding value.

**Source**: `src/config/accessors.rs:104-111`

```rust
Language::Rust => {
    languages_config
        .and_then(|lc| lc.rust.clone())
        .unwrap_or(LanguageFeatures {
            detect_dead_code: false, // Rust dead code detection disabled by default
            detect_complexity: true,
            detect_duplication: true,
        })
}
```

### Python Configuration

```toml
[languages.python]
detect_dead_code = true
detect_complexity = true
detect_duplication = true
```

All features enabled by default for Python. Dead code detection is valuable since Python lacks a compiler phase that catches unused code.

**Source**: `src/config/accessors.rs:113-115`

### JavaScript Configuration

```toml
[languages.javascript]
detect_dead_code = true
detect_complexity = true
detect_duplication = true
```

JavaScript analysis covers modern ES syntax, JSX, callbacks, promises, and async workflow patterns.

### TypeScript Configuration

```toml
[languages.typescript]
detect_dead_code = true
detect_complexity = true
detect_duplication = true
```

TypeScript analysis covers TS/TSX syntax, modules, type-oriented patterns, and async workflow patterns.

## Enabling Languages

Specify which languages to analyze with the `enabled` array:

```toml
[languages]
enabled = ["rust", "python", "javascript", "typescript"]
```

The documented and implemented user-facing language set is Rust, Python, JavaScript, and TypeScript.

## Feature Defaults

**Source**: `src/config/languages.rs:40-61`

```rust
impl Default for LanguageFeatures {
    fn default() -> Self {
        Self {
            detect_dead_code: true,   // default_detect_dead_code()
            detect_complexity: true,  // default_detect_complexity()
            detect_duplication: true, // default_detect_duplication()
        }
    }
}
```

When no language-specific configuration is provided, debtmap uses these defaults (with the Rust dead code exception handled at the accessor level).

## Using Language Configuration

### Analyze Only Rust Code

```toml
[languages]
enabled = ["rust"]

[languages.rust]
detect_dead_code = false
detect_complexity = true
detect_duplication = true
```

### Full Python Analysis

```toml
[languages]
enabled = ["python"]

[languages.python]
detect_dead_code = true
detect_complexity = true
detect_duplication = true
```

### Multi-Language Projects

```toml
[languages]
enabled = ["rust", "python", "javascript", "typescript"]

# Different settings per language
[languages.rust]
detect_dead_code = false        # Trust rustc
detect_complexity = true
detect_duplication = true

[languages.python]
detect_dead_code = true         # Python needs this
detect_complexity = true
detect_duplication = false      # Skip if not needed

[languages.javascript]
detect_dead_code = true
detect_complexity = true
detect_duplication = true

[languages.typescript]
detect_dead_code = true
detect_complexity = true
detect_duplication = true
```

## Complexity Calculations by Language

Debtmap uses language-specific complexity analyzers:

**Source**: `src/complexity/languages/rust.rs` and `src/analyzers/implementations.rs`

### Rust Complexity

- Uses `syn` for AST parsing
- Calculates cyclomatic complexity from control flow
- Tracks cognitive complexity with nesting depth penalties
- Detects pattern match complexity with entropy analysis

### Python Complexity

- Uses tree-sitter for AST parsing
- Handles Python-specific constructs (decorators, comprehensions)
- Tracks class method complexity
- Analyzes exception handling patterns

### JavaScript Complexity

- Uses tree-sitter for AST parsing
- Handles ES modules, JSX, callbacks, promises, and async workflows
- Tracks function and class method complexity
- Detects JavaScript-specific functional and async patterns

### TypeScript Complexity

- Uses tree-sitter for TS/TSX parsing
- Handles modern module syntax and frontend/server patterns
- Tracks type-oriented complexity and async workflows
- Detects broad type-safety and promise-related patterns

## Integration with Other Configuration

Language configuration interacts with other debtmap settings:

### Ignore Patterns

File exclusions in `[ignore]` apply before language detection:

```toml
[ignore]
patterns = [
    "target/**",       # Rust build output
    "venv/**",         # Python virtual environment
    "*.min.js",        # Minified frontend artifacts
]
```

See [Thresholds Configuration](thresholds.md) for complexity thresholds that can be language-aware.

### Classification

Semantic classification (function roles like "orchestrator", "pure_logic") operates independently of language but uses language-specific patterns for detection.

See [Advanced Options](advanced.md) for classification configuration.

## API Reference

### LanguagesConfig

**Source**: `src/config/languages.rs:4-22`

| Field | Type | Description |
|-------|------|-------------|
| `enabled` | `Vec<String>` | Languages to analyze |
| `rust` | `Option<LanguageFeatures>` | Rust-specific settings |
| `python` | `Option<LanguageFeatures>` | Python-specific settings |
| `javascript` | `Option<LanguageFeatures>` | JavaScript-specific settings |
| `typescript` | `Option<LanguageFeatures>` | TypeScript-specific settings |

### LanguageFeatures

**Source**: `src/config/languages.rs:24-38`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detect_dead_code` | `bool` | `true` | Enable dead code detection |
| `detect_complexity` | `bool` | `true` | Enable complexity analysis |
| `detect_duplication` | `bool` | `true` | Enable duplication detection |

## Troubleshooting

### Language Not Detected

If files aren't being analyzed:

1. Check the file extension is recognized (`.rs`, `.py`, `.pyw`, `.js`, `.mjs`, `.cjs`, `.jsx`, `.ts`, `.mts`, `.cts`, `.tsx`)
2. Verify the language is in `enabled` array
3. Check ignore patterns aren't excluding the files

### Missing Dead Code Warnings

For Rust: Enable dead code detection explicitly if you want debtmap to report it:

```toml
[languages.rust]
detect_dead_code = true  # Override default
```

For Python, JavaScript, and TypeScript: Ensure `detect_dead_code = true` (the default).

### Unexpected Complexity Scores

Language-specific complexity varies due to:
- Different control flow constructs
- Pattern matching complexity (Rust)
- Exception handling (Python)
- Async and callback flow (JavaScript/TypeScript)

See [Entropy Analysis](../entropy-analysis.md) for how entropy affects complexity scoring.

## See Also

- [Scoring Configuration](scoring.md) - Configure how complexity translates to debt scores
- [Thresholds Configuration](thresholds.md) - Set complexity thresholds
- [Advanced Options](advanced.md) - Classification and detection settings
