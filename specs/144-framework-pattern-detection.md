---
number: 144
title: Framework Pattern Detection
category: optimization
priority: medium
status: draft
dependencies: [141, 142]
created: 2025-10-27
---

# Specification 144: Framework Pattern Detection

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 141 (I/O Detection), Spec 142 (Call Graph)

## Context

Many functions have responsibilities determined by **framework conventions** rather than their actual code. For example:

- A function decorated with `@pytest.fixture` is a "Test Setup" function
- A function named `handle_*` in an Axum/Actix web app is a "Request Handler"
- A function in a `tests/` directory is a "Test Function"
- A function implementing `clap::Parser` is "CLI Argument Parsing"

Current name-based heuristics partially capture these patterns (`test_*` → Testing), but miss framework-specific conventions. I/O detection (Spec 141) and call graph analysis (Spec 142) don't recognize framework semantics.

Framework pattern detection adds ~5% accuracy improvement by recognizing framework idioms, but more importantly, provides **better categorization** for framework-heavy code. Instead of generic "I/O Operation", we can classify as "HTTP Request Handler" or "Database Migration".

## Objective

Detect framework-specific patterns (decorators, traits, naming conventions, file locations) to enable accurate responsibility classification for framework-bound code. Support common frameworks in Rust (Axum, Actix, Diesel, Clap), Python (FastAPI, Flask, Pytest, Django), and JavaScript (Express, React, Jest).

## Requirements

### Functional Requirements

**Framework Detection**:
- Identify framework usage via imports and dependencies
- Detect framework-specific decorators and attributes
- Recognize framework trait implementations
- Track framework-specific naming conventions

**Pattern Categories**:
- **Web Handlers**: HTTP request/response handling (Axum, Flask, Express)
- **Test Functions**: Unit tests, fixtures, mocks (pytest, Jest, Rust tests)
- **CLI Commands**: Command-line argument parsing (Clap, argparse, Commander)
- **Database Operations**: Queries, migrations, models (Diesel, SQLAlchemy, Prisma)
- **Middleware**: Request/response transformation
- **Configuration**: Framework-specific config loading
- **Lifecycle Hooks**: Startup, shutdown, beforeEach, afterEach

**Multi-Language Support**:
- **Rust**: Axum, Actix-web, Diesel, Clap, Tokio, test attributes
- **Python**: FastAPI, Flask, Django, pytest, SQLAlchemy, Click
- **JavaScript/TypeScript**: Express, Fastify, React, Jest, Prisma

**Classification Integration**:
- Override generic I/O classification with framework-specific category
- Combine framework patterns with I/O and call graph signals
- Provide framework context in responsibility labels

### Non-Functional Requirements

- **Extensibility**: Support adding new frameworks without code changes
- **Accuracy**: Correctly identify >90% of framework patterns
- **Performance**: Framework detection adds <5% overhead
- **Configuration**: Framework patterns defined in TOML config files

## Acceptance Criteria

- [ ] Axum HTTP handlers are identified (`async fn handler(...)` with Axum types)
- [ ] Pytest fixtures are identified (`@pytest.fixture` decorator)
- [ ] Rust test functions are identified (`#[test]`, `#[cfg(test)]`)
- [ ] Clap CLI parsers are identified (`#[derive(Parser)]`)
- [ ] Express route handlers are identified (`app.get(...)`, `router.post(...)`)
- [ ] React components are identified (function returning JSX)
- [ ] Diesel queries are identified (uses `diesel::` API)
- [ ] Framework patterns override generic I/O classifications
- [ ] Configuration file supports adding new frameworks
- [ ] Performance overhead <5% on framework-heavy codebases
- [ ] Test suite includes debtmap examples and popular open-source projects

## Technical Details

### Implementation Approach

**Phase 1: Framework Pattern Configuration**

Create `src/analysis/framework_patterns/patterns.toml`:

```toml
[rust.web.axum]
name = "Axum Web Framework"
category = "HTTP Request Handler"
patterns = [
    { type = "import", pattern = "axum::*" },
    { type = "parameter", pattern = "axum::extract::*" },
    { type = "return_type", pattern = "axum::response::*" },
]

[rust.testing.builtin]
name = "Rust Built-in Testing"
category = "Test Function"
patterns = [
    { type = "attribute", pattern = "#[test]" },
    { type = "attribute", pattern = "#[cfg(test)]" },
    { type = "name", pattern = "^test_.*" },
]

[rust.cli.clap]
name = "Clap CLI Parser"
category = "CLI Argument Parsing"
patterns = [
    { type = "derive", pattern = "Parser" },
    { type = "derive", pattern = "Args" },
    { type = "derive", pattern = "Subcommand" },
]

[python.web.fastapi]
name = "FastAPI"
category = "HTTP Request Handler"
patterns = [
    { type = "decorator", pattern = "@app\\.(get|post|put|delete|patch)" },
    { type = "import", pattern = "from fastapi import" },
    { type = "parameter", pattern = ": Request" },
]

[python.testing.pytest]
name = "Pytest"
category = "Test Function"
patterns = [
    { type = "decorator", pattern = "@pytest\\.fixture" },
    { type = "decorator", pattern = "@pytest\\.mark\\." },
    { type = "name", pattern = "^test_.*" },
    { type = "file_path", pattern = ".*/tests/.*" },
]

[javascript.web.express]
name = "Express.js"
category = "HTTP Request Handler"
patterns = [
    { type = "call", pattern = "app\\.(get|post|put|delete)" },
    { type = "call", pattern = "router\\.(get|post|put|delete)" },
    { type = "import", pattern = "require\\('express'\\)" },
]
```

**Phase 2: Framework Pattern Detector**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrameworkPattern {
    pub name: String,
    pub category: String,
    pub patterns: Vec<PatternMatcher>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum PatternMatcher {
    Import { pattern: String },
    Decorator { pattern: String },
    Attribute { pattern: String },
    Derive { pattern: String },
    Parameter { pattern: String },
    ReturnType { pattern: String },
    Name { pattern: String },
    Call { pattern: String },
    FilePath { pattern: String },
}

pub struct FrameworkDetector {
    patterns: HashMap<Language, Vec<FrameworkPattern>>,
    regex_cache: DashMap<String, Regex>,
}

impl FrameworkDetector {
    pub fn from_config(config_path: &Path) -> Result<Self> {
        let config_content = std::fs::read_to_string(config_path)?;
        let patterns: HashMap<String, HashMap<String, FrameworkPattern>> =
            toml::from_str(&config_content)?;

        // Convert config structure to indexed by language
        let mut by_language: HashMap<Language, Vec<FrameworkPattern>> = HashMap::new();

        for (lang_key, frameworks) in patterns {
            let language = Language::from_str(&lang_key)?;
            let framework_list: Vec<FrameworkPattern> =
                frameworks.into_values().collect();
            by_language.insert(language, framework_list);
        }

        Ok(FrameworkDetector {
            patterns: by_language,
            regex_cache: DashMap::new(),
        })
    }

    pub fn detect_framework_patterns(
        &self,
        function: &FunctionAst,
        file_context: &FileContext,
    ) -> Vec<FrameworkMatch> {
        let language = file_context.language;
        let framework_patterns = match self.patterns.get(&language) {
            Some(patterns) => patterns,
            None => return vec![],
        };

        let mut matches = Vec::new();

        for framework_pattern in framework_patterns {
            if self.matches_framework(function, file_context, framework_pattern) {
                matches.push(FrameworkMatch {
                    framework: framework_pattern.name.clone(),
                    category: framework_pattern.category.clone(),
                    confidence: self.calculate_confidence(function, framework_pattern),
                });
            }
        }

        matches
    }

    fn matches_framework(
        &self,
        function: &FunctionAst,
        file_context: &FileContext,
        pattern: &FrameworkPattern,
    ) -> bool {
        let mut matched_patterns = 0;
        let total_patterns = pattern.patterns.len();

        for matcher in &pattern.patterns {
            if self.matches_pattern(function, file_context, matcher) {
                matched_patterns += 1;
            }
        }

        // Require at least one pattern match
        matched_patterns > 0
    }

    fn matches_pattern(
        &self,
        function: &FunctionAst,
        file_context: &FileContext,
        matcher: &PatternMatcher,
    ) -> bool {
        match matcher {
            PatternMatcher::Import { pattern } => {
                file_context.imports.iter().any(|import| {
                    self.regex_match(pattern, import)
                })
            }
            PatternMatcher::Decorator { pattern } => {
                function.decorators.iter().any(|decorator| {
                    self.regex_match(pattern, &decorator.name)
                })
            }
            PatternMatcher::Attribute { pattern } => {
                function.attributes.iter().any(|attr| {
                    self.regex_match(pattern, &attr.to_string())
                })
            }
            PatternMatcher::Derive { pattern } => {
                function.derives.iter().any(|derive| {
                    self.regex_match(pattern, derive)
                })
            }
            PatternMatcher::Parameter { pattern } => {
                function.parameters.iter().any(|param| {
                    self.regex_match(pattern, &param.type_annotation)
                })
            }
            PatternMatcher::ReturnType { pattern } => {
                function.return_type
                    .as_ref()
                    .map(|rt| self.regex_match(pattern, rt))
                    .unwrap_or(false)
            }
            PatternMatcher::Name { pattern } => {
                self.regex_match(pattern, &function.name)
            }
            PatternMatcher::Call { pattern } => {
                function.calls.iter().any(|call| {
                    self.regex_match(pattern, &call.name)
                })
            }
            PatternMatcher::FilePath { pattern } => {
                self.regex_match(pattern, file_context.path.to_str().unwrap_or(""))
            }
        }
    }

    fn regex_match(&self, pattern: &str, text: &str) -> bool {
        let regex = self.regex_cache
            .entry(pattern.to_string())
            .or_insert_with(|| Regex::new(pattern).unwrap());

        regex.is_match(text)
    }

    fn calculate_confidence(&self, function: &FunctionAst, pattern: &FrameworkPattern) -> f64 {
        // Higher confidence if more patterns match
        let matched = pattern.patterns.iter()
            .filter(|p| self.matches_pattern(function, &Default::default(), p))
            .count();

        let total = pattern.patterns.len();

        (matched as f64 / total as f64).max(0.5)  // Minimum 0.5 confidence
    }
}
```

**Phase 3: Framework-Aware Classification**

```rust
#[derive(Debug, Clone)]
pub struct FrameworkMatch {
    pub framework: String,
    pub category: String,
    pub confidence: f64,
}

pub fn classify_with_framework_patterns(
    function: &FunctionAst,
    file_context: &FileContext,
    framework_detector: &FrameworkDetector,
    io_profile: &IoProfile,  // From Spec 141
) -> ResponsibilityClassification {
    // Check for framework patterns first
    let framework_matches = framework_detector.detect_framework_patterns(function, file_context);

    if let Some(framework_match) = framework_matches.first() {
        return ResponsibilityClassification {
            primary: framework_match.category.as_str(),
            confidence: framework_match.confidence,
            evidence: format!(
                "Matches {} framework pattern",
                framework_match.framework
            ),
            framework_context: Some(framework_match.clone()),
        };
    }

    // Fall back to I/O-based classification
    classify_from_io_profile(io_profile)
}
```

**Phase 4: Common Framework Patterns**

```rust
/// Pre-defined patterns for popular frameworks
pub struct CommonFrameworks;

impl CommonFrameworks {
    /// Detect Axum web handler
    pub fn is_axum_handler(function: &FunctionAst, file_context: &FileContext) -> bool {
        // Check for Axum-specific types in parameters or return
        let has_axum_imports = file_context.imports.iter().any(|i| i.contains("axum"));

        let has_axum_types = function.parameters.iter().any(|p| {
            p.type_annotation.contains("axum::")
        }) || function.return_type.as_ref().map(|rt| rt.contains("axum::")).unwrap_or(false);

        let is_async = function.is_async;

        has_axum_imports && has_axum_types && is_async
    }

    /// Detect pytest fixture
    pub fn is_pytest_fixture(function: &FunctionAst) -> bool {
        function.decorators.iter().any(|d| {
            d.name.contains("pytest.fixture") || d.name == "fixture"
        })
    }

    /// Detect Rust test function
    pub fn is_rust_test(function: &FunctionAst) -> bool {
        function.attributes.iter().any(|attr| {
            attr.to_string().contains("#[test]") || attr.to_string().contains("#[tokio::test]")
        })
    }

    /// Detect Clap CLI parser
    pub fn is_clap_parser(struct_ast: &StructAst) -> bool {
        struct_ast.derives.iter().any(|d| {
            d == "Parser" || d == "Args" || d == "Subcommand"
        })
    }

    /// Detect Express route handler
    pub fn is_express_handler(function: &FunctionAst, file_context: &FileContext) -> bool {
        let has_express_import = file_context.imports.iter().any(|i| {
            i.contains("express")
        });

        let has_req_res_params = function.parameters.len() >= 2 &&
            (function.parameters[0].name == "req" || function.parameters[0].name == "request") &&
            (function.parameters[1].name == "res" || function.parameters[1].name == "response");

        has_express_import && has_req_res_params
    }

    /// Detect React component
    pub fn is_react_component(function: &FunctionAst, file_context: &FileContext) -> bool {
        let has_react_import = file_context.imports.iter().any(|i| {
            i.contains("react") || i.contains("React")
        });

        let returns_jsx = function.return_type.as_ref().map(|rt| {
            rt.contains("JSX.Element") || rt.contains("ReactElement")
        }).unwrap_or(false);

        // Or check if function returns JSX (detected by parser)
        let has_jsx_return = function.body_contains_jsx;

        has_react_import && (returns_jsx || has_jsx_return)
    }
}
```

### Architecture Changes

**New Module**: `src/analysis/framework_patterns/`
- `detector.rs` - Main framework pattern detection
- `patterns.toml` - Configuration file with framework patterns
- `common.rs` - Hard-coded patterns for most popular frameworks
- `web.rs` - Web framework patterns (Axum, Flask, Express)
- `testing.rs` - Test framework patterns (pytest, Jest, Rust tests)
- `cli.rs` - CLI framework patterns (Clap, Click, Commander)
- `database.rs` - Database framework patterns (Diesel, SQLAlchemy, Prisma)

**Integration Point**: `src/organization/god_object_analysis.rs`
- Add framework detection as first classification step
- Override generic classifications with framework-specific categories
- Preserve framework context in output

**Configuration**: `framework_patterns.toml` in project root or config directory

## Dependencies

- **Prerequisites**: Spec 141 (I/O Detection), Spec 142 (Call Graph)
- **Affected Components**:
  - `src/organization/god_object_analysis.rs` - responsibility classification
  - `src/analysis/` - new framework_patterns module
- **External Dependencies**:
  - `regex` (already in use)
  - `toml` (already in use for config)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_axum_handler() {
        let code = r#"
        use axum::{extract::Path, response::Json};

        async fn get_user(Path(user_id): Path<u32>) -> Json<User> {
            // Handler logic
        }
        "#;

        let ast = parse_rust(code);
        let detector = FrameworkDetector::from_config("framework_patterns.toml").unwrap();

        let matches = detector.detect_framework_patterns(
            &ast.functions[0],
            &ast.file_context
        );

        assert_eq!(matches[0].category, "HTTP Request Handler");
        assert_eq!(matches[0].framework, "Axum Web Framework");
    }

    #[test]
    fn detect_pytest_fixture() {
        let code = r#"
        import pytest

        @pytest.fixture
        def database():
            return DatabaseConnection()
        "#;

        let ast = parse_python(code);
        let detector = FrameworkDetector::from_config("framework_patterns.toml").unwrap();

        let matches = detector.detect_framework_patterns(
            &ast.functions[0],
            &ast.file_context
        );

        assert_eq!(matches[0].category, "Test Fixture");
    }

    #[test]
    fn detect_rust_test() {
        let code = r#"
        #[test]
        fn test_addition() {
            assert_eq!(2 + 2, 4);
        }
        "#;

        let ast = parse_rust(code);
        let detector = FrameworkDetector::from_config("framework_patterns.toml").unwrap();

        let matches = detector.detect_framework_patterns(
            &ast.functions[0],
            &ast.file_context
        );

        assert_eq!(matches[0].category, "Test Function");
    }

    #[test]
    fn detect_clap_cli_parser() {
        let code = r#"
        use clap::Parser;

        #[derive(Parser)]
        struct Args {
            #[arg(short, long)]
            name: String,
        }
        "#;

        let ast = parse_rust(code);
        let detector = FrameworkDetector::from_config("framework_patterns.toml").unwrap();

        let matches = detector.detect_framework_patterns(
            &ast.structs[0],
            &ast.file_context
        );

        assert_eq!(matches[0].category, "CLI Argument Parsing");
    }
}
```

### Integration Tests

```rust
#[test]
fn framework_patterns_in_real_projects() {
    // Test on real open-source projects
    let test_cases = vec![
        ("axum/examples/hello-world/src/main.rs", "HTTP Request Handler"),
        ("pytest/src/_pytest/fixtures.py", "Test Fixture"),
        ("express/examples/hello-world.js", "HTTP Request Handler"),
    ];

    let detector = FrameworkDetector::from_config("framework_patterns.toml").unwrap();

    for (file_path, expected_category) in test_cases {
        let ast = parse_file(file_path);
        let matches = detector.detect_framework_patterns(&ast.functions[0], &ast.file_context);

        assert!(matches.iter().any(|m| m.category == expected_category));
    }
}
```

## Documentation Requirements

### User Documentation

Update README.md:
```markdown
## Framework Pattern Detection

Debtmap recognizes framework-specific patterns for accurate categorization:

**Supported Frameworks**:
- **Web**: Axum, Actix, FastAPI, Flask, Express, Fastify
- **Testing**: Rust tests, pytest, Jest, Mocha
- **CLI**: Clap, Click, Commander
- **Database**: Diesel, SQLAlchemy, Prisma

**Custom Frameworks**:
Add patterns to `framework_patterns.toml`:
```toml
[rust.your_framework]
name = "Your Framework"
category = "Your Category"
patterns = [
    { type = "import", pattern = "your_crate::*" }
]
```
```

## Implementation Notes

### Extensibility Design

The configuration-based approach allows users to add custom frameworks:

```toml
# User-defined framework
[rust.custom.my_web_framework]
name = "My Web Framework"
category = "Custom HTTP Handler"
patterns = [
    { type = "import", pattern = "my_framework::*" },
    { type = "attribute", pattern = "#\\[route\\(.*\\)\\]" },
]
```

### Performance Optimization

- Cache regex compilation in `DashMap`
- Lazy load framework patterns (only for detected languages)
- Short-circuit on first strong match (high confidence)

## Migration and Compatibility

### Gradual Rollout

1. **Phase 1**: Detect frameworks without changing classification
2. **Phase 2**: Add framework context to existing classifications
3. **Phase 3**: Override generic classifications with framework-specific

## Expected Impact

### Accuracy Improvement

- **Before**: Generic "I/O Operation" or "Function Call"
- **After**: Specific "HTTP Request Handler" or "Test Fixture"
- **Improvement**: +5% overall accuracy, +30% specificity for framework code

### Better Categorization

```rust
// Before
async fn create_user(...) -> Json<User> { ... }
// Classification: "I/O Operation" (generic)

// After
async fn create_user(...) -> Json<User> { ... }
// Classification: "HTTP Request Handler (Axum)" (specific)
```

### Foundation for Multi-Signal (Spec 145)

Framework patterns provide context-aware signals:
- Override generic I/O classification when framework detected
- Combine with call graph to detect framework misuse
- Enable framework-specific refactoring recommendations
