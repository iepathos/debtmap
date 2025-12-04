# Analyzer Types

## Analyzer Types

Debtmap supports multiple programming languages with varying levels of analysis capability.

### Supported Languages

**Rust** (Full Support)
- **Parser**: syn (native Rust AST)
- **Capabilities**:
  - Full complexity metrics (cyclomatic, cognitive, entropy)
  - Trait implementation tracking
  - Purity detection with confidence scoring
  - Call graph analysis (upstream callers, downstream callees)
  - Semantic function classification (entry points, business logic, data access, infrastructure, utilities, test code)
  - Enhanced call graph with transitive relationships
  - Macro expansion support for accurate complexity analysis
  - Pattern-based adjustments for macros and code generation
  - Visibility tracking (pub, pub(crate), private)
  - Test module detection (#[cfg(test)])

**Semantic Classification:**

Debtmap automatically identifies function roles in Rust code to apply appropriate role multipliers in unified scoring:

- **Entry Points**: Functions named `main`, `start`, or public functions in `bin/` modules
- **Business Logic**: Core domain functions with complex logic, algorithms, business rules
- **Data Access**: Functions performing database queries, file I/O, network operations
- **Infrastructure**: Logging, configuration, monitoring, error handling utilities
- **Utilities**: Helper functions, formatters, type converters, validation functions
- **Test Code**: Functions in `#[cfg(test)]` modules, functions with `#[test]` attribute

This classification feeds directly into the unified scoring system's role multiplier (see Risk Scoring section).

**Unsupported Languages:**

Debtmap currently supports only Rust. Files with unsupported extensions are filtered out during the file discovery phase and never reach the analysis stage.

Files with extensions like `.py` (Python), `.js`/`.ts` (JavaScript/TypeScript), `.cpp` (C++), `.java`, `.go`, `.rb` (Ruby), `.php`, `.cs` (C#), `.swift`, `.kt` (Kotlin), `.scala`, and others are silently filtered during discovery.

**File filtering behavior:**
- Discovery scans project for Rust source files (.rs extension)
- Non-Rust files are skipped silently (no warnings or errors)
- No analysis, metrics, or debt patterns are generated for filtered files

**Example:**
```bash
# Analyze all Rust files in current directory
debtmap analyze .

# Analyze specific Rust file
debtmap analyze src/main.rs
```

### Language Detection

Automatic detection by file extension:
```rust
// Detects .rs files as Rust
let language = Language::from_path(&path);
```

All `.rs` files are automatically analyzed. No language flag is needed since Debtmap is Rust-focused.

### Extensibility

Debtmap's architecture allows adding new languages:

1. **Implement Analyzer trait:**
```rust
pub trait Analyzer: Send + Sync {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast>;
    fn analyze(&self, ast: &Ast) -> FileMetrics;
    fn language(&self) -> Language;
}
```

2. **Register in get_analyzer():**
```rust
pub fn get_analyzer(language: Language) -> Box<dyn Analyzer> {
    match language {
        Language::Rust => Box::new(RustAnalyzer::new()),
        Language::YourLanguage => Box::new(YourAnalyzer::new()),
        // ...
    }
}
```

See `src/analyzers/rust.rs` for a complete implementation example.

