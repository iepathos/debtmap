---
number: 145
title: Multi-Language God Object Support (Phase 3)
category: optimization
priority: low
status: draft
dependencies: [143, 144]
created: 2025-01-23
related: [143, 144]
---

# Specification 145: Multi-Language God Object Support (Phase 3)

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: Spec 143 (struct ownership), Spec 144 (cohesion scoring)
**Related**: Spec 143 (Phase 1), Spec 144 (Phase 2)

## Context

**Current State**: Specs 143 and 144 provide comprehensive god object analysis for Rust codebases using struct ownership tracking and call graph cohesion scoring.

**Problem**: Debtmap analyzes multiple languages (Python, JavaScript, TypeScript) but god object detection quality varies:
- **Python**: Has basic god object detection but no struct/class ownership tracking
- **JavaScript/TypeScript**: Minimal god object analysis support
- **Language-agnostic patterns**: Domain classification patterns optimized for Rust may not apply to other languages

**Real-World Examples**:

**Python God Class**:
```python
class UserManager:
    def __init__(self):
        self.users = []
        self.sessions = {}
        self.permissions = {}
        self.audit_log = []

    # User CRUD (should be UserRepository)
    def create_user(self, data): ...
    def delete_user(self, user_id): ...
    def get_user(self, user_id): ...

    # Session management (should be SessionManager)
    def create_session(self, user_id): ...
    def invalidate_session(self, session_id): ...

    # Permissions (should be PermissionChecker)
    def check_permission(self, user_id, resource): ...
    def grant_permission(self, user_id, permission): ...

    # Audit (should be AuditLogger)
    def log_action(self, user_id, action): ...
    def get_audit_trail(self, user_id): ...

    # Email (should be EmailService)
    def send_welcome_email(self, user_id): ...
    def send_password_reset(self, user_id): ...
```

**Current Recommendation** (Python detector):
- Detects as god class (20 methods)
- No struct ownership (doesn't track which methods could be grouped)
- No domain classification

**Desired Recommendation** (This Spec):
```
Suggested Module Splits (4 modules):

├─ user_repository.py - data_access
│   → Class: UserRepository
│   → Methods: create_user, delete_user, get_user, update_user (4 methods)
│   → Cohesion: 0.92

├─ session_manager.py - session_management
│   → Class: SessionManager
│   → Methods: create_session, invalidate_session, get_session (3 methods)
│   → Cohesion: 0.88

├─ permission_checker.py - authorization
│   → Class: PermissionChecker
│   → Methods: check_permission, grant_permission, revoke_permission (3 methods)
│   → Cohesion: 0.85

├─ user_notifications.py - notifications
│   → Class: UserNotifications
│   → Methods: send_welcome_email, send_password_reset (2 methods)
│   → Cohesion: 0.75
```

## Objective

Extend struct-ownership-based god object analysis to Python, JavaScript, and TypeScript, providing the same quality recommendations as Rust analysis.

**Success Criteria**:
- Python class method tracking with ownership analysis
- JavaScript/TypeScript class method tracking
- Language-specific domain classification patterns
- Cohesion scoring for all supported languages
- Consistent recommendation quality across languages

**Phase 3 Scope**:
- ✅ Python class ownership tracking
- ✅ JavaScript/TypeScript class ownership tracking
- ✅ Language-specific domain patterns
- ✅ Call graph integration for Python (if available)
- ✅ Unified recommendation format across languages

## Requirements

### Functional Requirements

**FR1: Python Class Ownership Tracking**
- Parse Python classes and methods using AST
- Track which methods belong to which class
- Identify standalone functions vs class methods
- Handle inheritance (methods from parent classes)
- Exclude special methods (dunder methods like `__init__`, `__str__`)

**FR2: JavaScript/TypeScript Class Ownership Tracking**
- Parse JavaScript/TypeScript classes and methods
- Track class methods, static methods, and getters/setters
- Handle ES6 class syntax
- Support TypeScript interfaces and type annotations
- Exclude constructor methods from refactoring counts

**FR3: Language-Specific Domain Classification**
- Python domain patterns:
  - `Repository`, `DAO`, `DataAccess` → data_access
  - `Manager`, `Service` → service
  - `Controller`, `Handler` → controller
  - `Validator`, `Checker` → validation
  - `Serializer`, `Formatter` → formatting
- JavaScript/TypeScript patterns:
  - `Component`, `View` → ui
  - `Controller`, `Route` → routing
  - `Service` → service
  - `Model`, `Schema` → model
  - `Util`, `Helper` → utilities

**FR4: Multi-Language Call Graph Support**
- Leverage Python call graph if available
- Integrate JavaScript/TypeScript call graph when implemented
- Graceful degradation if call graph unavailable (use heuristics)
- Consistent cohesion calculation across languages

### Non-Functional Requirements

**NFR1: Performance**
- Python analysis should match Rust performance characteristics
- JavaScript/TypeScript parsing should be efficient
- Language detection should be fast (file extension based)

**NFR2: Consistency**
- Recommendation format should be consistent across languages
- Domain classification should be intuitive per language
- Cohesion scoring should use same algorithm (when call graph available)

**NFR3: Maintainability**
- Language-specific logic should be isolated in separate modules
- Domain patterns should be configurable (future: per-language config files)
- Easy to add new languages in the future

## Acceptance Criteria

### AC1: Python Class Ownership Tracking
- [ ] Parse Python classes using tree-sitter or rustpython_parser
- [ ] Track methods belonging to each class
- [ ] Handle class inheritance (track inherited methods separately)
- [ ] Exclude dunder methods from analysis
- [ ] Track standalone functions
- [ ] Unit tests for Python class parsing
- [ ] Integration test on real Python god class

### AC2: JavaScript/TypeScript Class Ownership Tracking
- [ ] Parse JavaScript/TypeScript classes using tree-sitter
- [ ] Track class methods, static methods, getters/setters
- [ ] Handle ES6 class syntax
- [ ] Support TypeScript type annotations
- [ ] Exclude constructors from method counts
- [ ] Unit tests for JS/TS class parsing
- [ ] Integration test on real JavaScript god class

### AC3: Language-Specific Domain Patterns
- [ ] Implement Python domain classifier
- [ ] Implement JavaScript/TypeScript domain classifier
- [ ] 10+ domain patterns per language
- [ ] Fallback to method-name-based classification
- [ ] Unit tests for each language's domain patterns

### AC4: Multi-Language Call Graph Integration
- [ ] Integrate with Python call graph (if available)
- [ ] Plan for JavaScript/TypeScript call graph (future work)
- [ ] Graceful degradation without call graph
- [ ] Consistent cohesion calculation
- [ ] Unit tests for call graph integration

### AC5: Unified Output Format
- [ ] Consistent recommendation format across languages
- [ ] Language-specific naming conventions (user_manager.py vs UserManager.ts)
- [ ] Display language in output
- [ ] Integration tests for each language

### AC6: Real-World Test Cases
- [ ] Test on Python god class (20+ methods)
- [ ] Test on JavaScript god class (React component with 15+ methods)
- [ ] Test on TypeScript god class (service class with 25+ methods)
- [ ] Validate recommendations align with best practices

## Technical Details

### Architecture

**Language Detection**:
```rust
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
}

impl Language {
    pub fn from_path(path: &Path) -> Option<Language> {
        match path.extension()?.to_str()? {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "js" | "jsx" => Some(Language::JavaScript),
            "ts" | "tsx" => Some(Language::TypeScript),
            _ => None,
        }
    }
}
```

**New Components**:
- `src/organization/python/class_ownership.rs` - Python class ownership
- `src/organization/python/domain_classifier.rs` - Python domain patterns
- `src/organization/javascript/class_ownership.rs` - JS/TS class ownership
- `src/organization/javascript/domain_classifier.rs` - JS/TS domain patterns
- `src/organization/language_dispatcher.rs` - Language-specific analysis routing

**Modified Components**:
- `src/organization/god_object_detector.rs` - Multi-language support
- `src/organization/god_object_analysis.rs` - Unified analysis interface

### Data Structures

```rust
// Language-agnostic ownership analyzer trait
pub trait ClassOwnershipAnalyzer {
    fn analyze_file(&self, content: &str) -> Result<ClassOwnership, String>;
    fn language(&self) -> Language;
}

// Unified class ownership structure
pub struct ClassOwnership {
    pub language: Language,
    pub class_to_methods: HashMap<String, Vec<String>>,
    pub method_to_class: HashMap<String, String>,
    pub standalone_functions: Vec<String>,
    pub class_locations: HashMap<String, (usize, usize)>,
}

// Python-specific implementation
pub struct PythonClassAnalyzer {
    // tree-sitter or rustpython_parser
}

impl ClassOwnershipAnalyzer for PythonClassAnalyzer {
    fn analyze_file(&self, content: &str) -> Result<ClassOwnership, String> {
        // Parse Python AST and extract class ownership
    }

    fn language(&self) -> Language {
        Language::Python
    }
}

// JavaScript/TypeScript-specific implementation
pub struct JavaScriptClassAnalyzer {
    language_variant: Language, // JavaScript or TypeScript
}

impl ClassOwnershipAnalyzer for JavaScriptClassAnalyzer {
    fn analyze_file(&self, content: &str) -> Result<ClassOwnership, String> {
        // Parse JS/TS AST using tree-sitter and extract class ownership
    }

    fn language(&self) -> Language {
        self.language_variant
    }
}
```

### Python Class Ownership Implementation

```rust
use rustpython_parser::{ast, parse};

pub struct PythonClassAnalyzer;

impl PythonClassAnalyzer {
    pub fn analyze_file(content: &str) -> Result<ClassOwnership, String> {
        let ast = parse(content, "input.py")
            .map_err(|e| format!("Failed to parse Python: {}", e))?;

        let mut ownership = ClassOwnership::new(Language::Python);

        for stmt in &ast.statements {
            match stmt {
                ast::Stmt::ClassDef(class_def) => {
                    Self::analyze_class(&class_def, &mut ownership);
                }
                ast::Stmt::FunctionDef(func_def) => {
                    // Standalone function
                    ownership.standalone_functions.push(func_def.name.clone());
                }
                _ => {}
            }
        }

        Ok(ownership)
    }

    fn analyze_class(class_def: &ast::StmtClassDef, ownership: &mut ClassOwnership) {
        let class_name = &class_def.name;
        let mut methods = Vec::new();

        for stmt in &class_def.body {
            if let ast::Stmt::FunctionDef(method_def) = stmt {
                let method_name = &method_def.name;

                // Skip dunder methods
                if method_name.starts_with("__") && method_name.ends_with("__") {
                    continue;
                }

                methods.push(method_name.clone());
                ownership.method_to_class.insert(
                    method_name.clone(),
                    class_name.clone(),
                );
            }
        }

        ownership.class_to_methods.insert(class_name.clone(), methods);

        // Track location (line numbers from AST)
        let start_line = class_def.lineno;
        let end_line = class_def.end_lineno.unwrap_or(start_line);
        ownership.class_locations.insert(
            class_name.clone(),
            (start_line, end_line),
        );
    }
}
```

### JavaScript/TypeScript Class Ownership Implementation

```rust
use tree_sitter::{Parser, Language as TSLanguage};

pub struct JavaScriptClassAnalyzer {
    parser: Parser,
    language_variant: Language,
}

impl JavaScriptClassAnalyzer {
    pub fn new(language: Language) -> Result<Self, String> {
        let mut parser = Parser::new();

        let ts_language = match language {
            Language::JavaScript => tree_sitter_javascript::language(),
            Language::TypeScript => tree_sitter_typescript::language_typescript(),
            _ => return Err("Invalid language for JavaScriptClassAnalyzer".to_string()),
        };

        parser.set_language(ts_language)
            .map_err(|e| format!("Failed to set parser language: {}", e))?;

        Ok(Self {
            parser,
            language_variant: language,
        })
    }

    pub fn analyze_file(&mut self, content: &str) -> Result<ClassOwnership, String> {
        let tree = self.parser.parse(content, None)
            .ok_or("Failed to parse file".to_string())?;

        let mut ownership = ClassOwnership::new(self.language_variant);
        let root = tree.root_node();

        Self::walk_tree(&root, content, &mut ownership);

        Ok(ownership)
    }

    fn walk_tree(node: &tree_sitter::Node, source: &str, ownership: &mut ClassOwnership) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "class_declaration" => {
                    Self::analyze_class(&child, source, ownership);
                }
                "function_declaration" => {
                    // Standalone function
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = name_node.utf8_text(source.as_bytes()).unwrap_or("");
                        ownership.standalone_functions.push(name.to_string());
                    }
                }
                _ => {
                    // Recurse into children
                    Self::walk_tree(&child, source, ownership);
                }
            }
        }
    }

    fn analyze_class(
        node: &tree_sitter::Node,
        source: &str,
        ownership: &mut ClassOwnership,
    ) {
        let class_name = if let Some(name_node) = node.child_by_field_name("name") {
            name_node.utf8_text(source.as_bytes()).unwrap_or("Unknown")
        } else {
            "Unknown"
        };

        let body_node = node.child_by_field_name("body");
        if body_node.is_none() {
            return;
        }

        let mut methods = Vec::new();
        let body = body_node.unwrap();

        for child in body.children(&mut body.walk()) {
            match child.kind() {
                "method_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let method_name = name_node.utf8_text(source.as_bytes())
                            .unwrap_or("");

                        // Skip constructor
                        if method_name == "constructor" {
                            continue;
                        }

                        methods.push(method_name.to_string());
                        ownership.method_to_class.insert(
                            method_name.to_string(),
                            class_name.to_string(),
                        );
                    }
                }
                _ => {}
            }
        }

        ownership.class_to_methods.insert(class_name.to_string(), methods);

        // Track location
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        ownership.class_locations.insert(
            class_name.to_string(),
            (start_line, end_line),
        );
    }
}
```

### Language-Specific Domain Classification

**Python Domain Classifier**:
```rust
pub fn classify_python_class_domain(class_name: &str, methods: &[String]) -> String {
    let lower = class_name.to_lowercase();

    // Python-specific patterns
    if lower.contains("repository") || lower.contains("dao") {
        return "data_access".to_string();
    }
    if lower.contains("manager") || lower.contains("service") {
        return "service".to_string();
    }
    if lower.contains("controller") || lower.contains("handler") {
        return "controller".to_string();
    }
    if lower.contains("validator") || lower.contains("checker") {
        return "validation".to_string();
    }
    if lower.contains("serializer") || lower.contains("formatter") {
        return "formatting".to_string();
    }
    if lower.contains("view") || lower.contains("template") {
        return "presentation".to_string();
    }
    if lower.contains("model") || lower.contains("entity") {
        return "model".to_string();
    }
    if lower.contains("test") {
        return "testing".to_string();
    }

    // Fallback to method-name-based classification
    infer_domain_from_methods(methods)
}
```

**JavaScript/TypeScript Domain Classifier**:
```rust
pub fn classify_javascript_class_domain(class_name: &str, methods: &[String]) -> String {
    let lower = class_name.to_lowercase();

    // JavaScript/TypeScript-specific patterns
    if lower.contains("component") || lower.contains("view") {
        return "ui".to_string();
    }
    if lower.contains("controller") || lower.contains("route") {
        return "routing".to_string();
    }
    if lower.contains("service") {
        return "service".to_string();
    }
    if lower.contains("model") || lower.contains("schema") {
        return "model".to_string();
    }
    if lower.contains("util") || lower.contains("helper") {
        return "utilities".to_string();
    }
    if lower.contains("store") || lower.contains("state") {
        return "state_management".to_string();
    }
    if lower.contains("api") || lower.contains("client") {
        return "api".to_string();
    }
    if lower.contains("hook") {
        return "hooks".to_string();
    }

    // Fallback to method-name-based classification
    infer_domain_from_methods(methods)
}
```

### Unified Analysis Interface

```rust
pub fn analyze_god_object_multi_language(
    path: &Path,
    content: &str,
) -> Result<EnhancedGodObjectAnalysis, String> {
    let language = Language::from_path(path)
        .ok_or("Unsupported file extension")?;

    match language {
        Language::Rust => {
            let parsed = syn::parse_file(content)
                .map_err(|e| format!("Rust parse error: {}", e))?;
            let detector = GodObjectDetector::with_source_content(content);
            Ok(detector.analyze_enhanced(path, &parsed))
        }
        Language::Python => {
            let ownership = PythonClassAnalyzer::analyze_file(content)?;
            analyze_with_ownership(path, content, ownership)
        }
        Language::JavaScript | Language::TypeScript => {
            let mut analyzer = JavaScriptClassAnalyzer::new(language)?;
            let ownership = analyzer.analyze_file(content)?;
            analyze_with_ownership(path, content, ownership)
        }
    }
}

fn analyze_with_ownership(
    path: &Path,
    content: &str,
    ownership: ClassOwnership,
) -> Result<EnhancedGodObjectAnalysis, String> {
    // Unified analysis using ClassOwnership
    // Similar to Rust analysis but language-agnostic
    // ...
}
```

## Testing Strategy

### Unit Tests

**Python Class Parsing**:
```rust
#[test]
fn test_python_class_ownership() {
    let code = r#"
class UserManager:
    def create_user(self, data):
        pass

    def delete_user(self, user_id):
        pass

    def __init__(self):
        pass

def standalone_function():
    pass
    "#;

    let ownership = PythonClassAnalyzer::analyze_file(code).unwrap();

    assert_eq!(ownership.class_to_methods.get("UserManager").unwrap().len(), 2);
    assert!(ownership.class_to_methods.get("UserManager").unwrap()
        .contains(&"create_user".to_string()));
    assert!(!ownership.class_to_methods.get("UserManager").unwrap()
        .contains(&"__init__".to_string())); // Excluded
    assert!(ownership.standalone_functions.contains(&"standalone_function".to_string()));
}
```

**JavaScript Class Parsing**:
```rust
#[test]
fn test_javascript_class_ownership() {
    let code = r#"
class UserController {
    constructor() {
        this.users = [];
    }

    createUser(data) {
        // ...
    }

    deleteUser(userId) {
        // ...
    }
}

function standaloneFunction() {
    // ...
}
    "#;

    let mut analyzer = JavaScriptClassAnalyzer::new(Language::JavaScript).unwrap();
    let ownership = analyzer.analyze_file(code).unwrap();

    assert_eq!(ownership.class_to_methods.get("UserController").unwrap().len(), 2);
    assert!(ownership.class_to_methods.get("UserController").unwrap()
        .contains(&"createUser".to_string()));
    assert!(!ownership.class_to_methods.get("UserController").unwrap()
        .contains(&"constructor".to_string())); // Excluded
    assert!(ownership.standalone_functions.contains(&"standaloneFunction".to_string()));
}
```

**Domain Classification**:
```rust
#[test]
fn test_python_domain_classification() {
    assert_eq!(classify_python_class_domain("UserRepository", &[]), "data_access");
    assert_eq!(classify_python_class_domain("EmailService", &[]), "service");
    assert_eq!(classify_python_class_domain("PasswordValidator", &[]), "validation");
}

#[test]
fn test_javascript_domain_classification() {
    assert_eq!(classify_javascript_class_domain("UserComponent", &[]), "ui");
    assert_eq!(classify_javascript_class_domain("ApiService", &[]), "service");
    assert_eq!(classify_javascript_class_domain("UserModel", &[]), "model");
}
```

### Integration Tests

**Python God Class**:
```rust
#[test]
fn test_python_god_class_recommendations() {
    let code = r#"
class UserManager:
    def create_user(self, data): pass
    def delete_user(self, user_id): pass
    def get_user(self, user_id): pass
    def update_user(self, user_id, data): pass

    def create_session(self, user_id): pass
    def invalidate_session(self, session_id): pass

    def check_permission(self, user_id, resource): pass
    def grant_permission(self, user_id, permission): pass

    def send_welcome_email(self, user_id): pass
    def send_password_reset(self, user_id): pass
    "#;

    let analysis = analyze_god_object_multi_language(
        Path::new("user_manager.py"),
        code,
    ).unwrap();

    // Should recommend multiple splits
    match analysis.classification {
        GodObjectType::GodClass { .. } => {
            // Should have domain-based recommendations
            assert!(analysis.file_metrics.recommended_splits.len() >= 3);
        }
        _ => panic!("Expected GodClass classification"),
    }
}
```

## Output Format

### Language-Specific Naming

**Python Output**:
```
RECOMMENDED REFACTORING STRATEGY (Python):

Suggested Module Splits (4 modules):

├─ ⭐⭐⭐ user_repository.py - data_access
│   → Class: UserRepository
│   → Methods: create_user, delete_user, get_user, update_user (4 methods)
│   → Cohesion: 0.92 (Excellent)
```

**JavaScript Output**:
```
RECOMMENDED REFACTORING STRATEGY (JavaScript):

Suggested Module Splits (3 modules):

├─ ⭐⭐⭐ UserController.js - routing
│   → Class: UserController
│   → Methods: handleCreate, handleDelete, handleUpdate (3 methods)
│   → Cohesion: 0.88 (Excellent)
```

**TypeScript Output**:
```
RECOMMENDED REFACTORING STRATEGY (TypeScript):

Suggested Module Splits (3 modules):

├─ ⭐⭐⭐ UserService.ts - service
│   → Class: UserService
│   → Methods: createUser, deleteUser, updateUser (3 methods)
│   → Cohesion: 0.85 (Good)
```

## Dependencies

**External Crates**:
- `rustpython-parser` or `tree-sitter-python` for Python parsing
- `tree-sitter-javascript` for JavaScript parsing
- `tree-sitter-typescript` for TypeScript parsing
- `tree-sitter` for unified parsing interface

**Cargo.toml**:
```toml
[dependencies]
tree-sitter = "0.20"
tree-sitter-python = "0.20"
tree-sitter-javascript = "0.20"
tree-sitter-typescript = "0.20"
# OR
rustpython-parser = "0.3"
```

## Success Metrics

### Quantitative Metrics

1. **Cross-Language Consistency**:
   - ✅ Python recommendations have similar quality to Rust
   - ✅ JavaScript/TypeScript recommendations align with best practices
   - ✅ Domain classification accuracy >80% per language

2. **Performance**:
   - ✅ Python analysis time comparable to Rust
   - ✅ JavaScript/TypeScript parsing efficient (<200ms for 1000 LOC)

### Qualitative Metrics

1. **User Trust**:
   - Recommendations align with language-specific best practices
   - Domain classifications make sense for each language ecosystem
   - Users can follow recommendations without language-specific adaptation

## Migration and Compatibility

### Backward Compatibility

- Existing Rust analysis unchanged
- Python analysis enhanced (no breaking changes)
- JavaScript/TypeScript analysis new functionality

### Rollout Strategy

1. **v0.4.0**: Introduce Python support (this spec)
2. **v0.4.1**: Introduce JavaScript support
3. **v0.4.2**: Introduce TypeScript support
4. **v0.5.0**: Unified multi-language interface

## Implementation Plan

### Week 1: Python Support
- [ ] Implement Python class ownership analyzer
- [ ] Python domain classifier
- [ ] Unit tests

### Week 2: JavaScript Support
- [ ] Implement JavaScript class ownership analyzer
- [ ] JavaScript domain classifier
- [ ] Unit tests

### Week 3: TypeScript Support
- [ ] Implement TypeScript class ownership analyzer
- [ ] TypeScript domain classifier
- [ ] Integration tests

### Week 4: Polish & Integration
- [ ] Unified interface
- [ ] Performance optimization
- [ ] Documentation
- [ ] Real-world test cases

## Related Specifications

- **Spec 143**: Struct Ownership Foundation (prerequisite)
- **Spec 144**: Call Graph Cohesion Scoring (prerequisite)

## Notes

- Tree-sitter provides better error recovery than language-specific parsers
- Python call graph support depends on existing Python analyzer capabilities
- JavaScript/TypeScript call graph may require separate implementation
- Consider supporting Python type hints for better domain classification
- React components may need special handling (JSX/TSX)
