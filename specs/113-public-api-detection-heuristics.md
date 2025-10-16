---
number: 113
title: Public API Detection Heuristics
category: foundation
priority: high
status: draft
dependencies: [112]
created: 2025-10-16
---

# Specification 113: Public API Detection Heuristics

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 109 (Cross-File Dependency Analysis)

## Context

Debtmap v0.2.8 treats all functions without detected callers as "private" and flaggable as dead code. This causes **false positives for public API functions** - functions intentionally exposed for external use but not called within the analyzed codebase.

**Real-World Impact from Bug Report**:
- **False Positive #6**: `create_bots_from_list()` - Public API function flagged as dead
- **False Positive #7**: `Conversation.index_of()` - Public method with comprehensive docstring flagged as dead
- **False Positive #8**: `save_chat_history()` - Public utility paired with used `load_chat_history()` flagged as dead
- **Impact**: 30% of dead code recommendations were public API false positives

**Current Behavior**:
```python
# genai_utils.py
def create_bots_from_list(bot_files: list=None, bot_path=prompt_path, simple: bool=config["simple_chars"]):
    """Create bots from a list of bot configuration files."""
    # ❌ Flagged as dead code - but this is a PUBLIC API function!
    # - No underscore prefix
    # - Has default parameters (suggests library usage)
    # - Comprehensive docstring
    # - Type hints indicate public interface

def save_chat_history(bot_name, history, path=history_path):
    """Save chat history for a specific bot to a JSON file."""
    # ❌ Flagged as dead code - but paired with USED load_chat_history()
    # Symmetric load/save operations suggest public API
```

**Why This Matters**:
- Library modules expose functions for external callers
- Public APIs aren't called within the library itself
- Removing these breaks downstream consumers
- Users lose trust in dead code detection

**Current Gaps**:
- No distinction between internal and public functions
- Docstrings not used as public API signals
- Naming conventions (no underscore prefix) ignored
- Symmetric function pairs (load/save, get/set) not recognized

## Objective

Implement heuristics to detect public API functions and exclude them from dead code detection, reducing false positives from 30% to < 5% for library-style modules.

## Requirements

### Functional Requirements

1. **Naming Convention Heuristics**
   - Functions without underscore prefix at module level → likely public
   - Functions with leading underscore (`_private()`) → internal
   - Dunder methods (`__init__`, `__str__`) → special methods (not dead code)
   - Class methods without underscore → public methods

2. **Documentation Analysis**
   - Functions with comprehensive docstrings (> 50 chars) → likely public API
   - Google-style docstrings with Args/Returns sections → public API
   - Sphinx/reStructuredText docstrings → public API
   - Functions with type hints and docstrings → strong public API signal

3. **Type Annotation Analysis**
   - Functions with full type hints (params + return) → likely public
   - Use of generic types (`List[T]`, `Dict[K, V]`) → public API
   - Complex type annotations suggest documented interface

4. **Symmetric Function Detection**
   - Detect paired operations: `load`/`save`, `get`/`set`, `open`/`close`
   - If one function in pair is used, mark both as public
   - Common patterns: `create`/`destroy`, `start`/`stop`, `acquire`/`release`

5. **Module-Level Export Analysis**
   - Functions listed in `__all__` → definitely public API
   - Top-level module functions (not in classes) → likely public
   - Functions imported in `__init__.py` → package public API

6. **Interface Implementation Detection**
   - Abstract methods implemented from base classes → not dead code
   - Protocol/Interface implementations → public API
   - Methods matching abstract base class signatures → implementation requirement

7. **Decorator Detection**
   - `@property`, `@staticmethod`, `@classmethod` → API methods
   - `@abstractmethod` → interface definition
   - Framework decorators (`@app.route`, `@pytest.fixture`) → public/framework hooks

### Non-Functional Requirements

1. **Accuracy**
   - False positive reduction: 30% → < 5%
   - No false negatives (don't mark truly dead code as public)
   - Configurable confidence thresholds

2. **Performance**
   - Heuristic evaluation adds < 5% to analysis time
   - Docstring parsing scales to large files
   - Efficient pattern matching for function pairs

3. **Configurability**
   - Users can adjust heuristic weights
   - Option to disable specific heuristics
   - Custom patterns for project-specific APIs

4. **Language Support**
   - Python (primary)
   - Rust (underscore convention, public/pub keyword)
   - JavaScript/TypeScript (export keyword)

## Acceptance Criteria

- [ ] Functions without underscore prefix marked as "potentially public"
- [ ] Functions with comprehensive docstrings (> 50 chars) marked as public API
- [ ] Functions with full type hints + docstring marked as high-confidence public API
- [ ] Symmetric function pairs detected (load/save, get/set)
- [ ] If one function in pair is used, both marked as public
- [ ] `__all__` exports identified as definite public API
- [ ] Abstract method implementations excluded from dead code
- [ ] Decorator-annotated functions handled appropriately
- [ ] `create_bots_from_list()` example no longer flagged as dead code
- [ ] `save_chat_history()` example no longer flagged (paired with `load_chat_history()`)
- [ ] False positive rate < 5% on promptconstruct-frontend
- [ ] Configuration option to adjust heuristic sensitivity
- [ ] Performance overhead < 5%
- [ ] Documentation includes heuristic explanations and examples

## Technical Details

### Implementation Approach

**Phase 1: Basic Heuristics**
1. Implement naming convention checker
2. Add docstring length and quality analysis
3. Detect `__all__` exports

**Phase 2: Advanced Heuristics**
1. Implement symmetric function pair detection
2. Add type annotation analysis
3. Detect abstract method implementations

**Phase 3: Configuration and Tuning**
1. Add configuration options for heuristic weights
2. Implement confidence scoring
3. Allow custom patterns for project-specific APIs

### Architecture Changes

```rust
// src/debt/public_api_detector.rs
pub struct PublicApiDetector {
    config: PublicApiConfig,
    heuristics: Vec<Box<dyn ApiHeuristic>>,
}

#[derive(Debug, Clone)]
pub struct PublicApiConfig {
    // Weight for each heuristic (0.0 - 1.0)
    pub naming_convention_weight: f32,
    pub docstring_weight: f32,
    pub type_annotation_weight: f32,
    pub symmetric_pair_weight: f32,
    pub module_export_weight: f32,

    // Confidence threshold for marking as public
    pub public_api_threshold: f32,

    // Custom patterns
    pub custom_public_prefixes: Vec<String>,
    pub custom_symmetric_pairs: Vec<(String, String)>,
}

impl Default for PublicApiConfig {
    fn default() -> Self {
        Self {
            naming_convention_weight: 0.3,
            docstring_weight: 0.25,
            type_annotation_weight: 0.15,
            symmetric_pair_weight: 0.2,
            module_export_weight: 0.1,
            public_api_threshold: 0.6,
            custom_public_prefixes: vec![],
            custom_symmetric_pairs: vec![],
        }
    }
}

pub trait ApiHeuristic: Send + Sync {
    fn name(&self) -> &str;
    fn evaluate(&self, function: &FunctionDef, context: &FileContext) -> f32;
    fn explain(&self, function: &FunctionDef) -> String;
}

#[derive(Debug, Clone)]
pub struct PublicApiScore {
    pub is_public: bool,
    pub confidence: f32,
    pub heuristic_scores: HashMap<String, f32>,
    pub reasoning: Vec<String>,
}

impl PublicApiDetector {
    pub fn new(config: PublicApiConfig) -> Self;
    pub fn is_public_api(&self, function: &FunctionDef, context: &FileContext) -> PublicApiScore;
    pub fn find_symmetric_pair(&self, function: &FunctionDef, module: &Module) -> Option<FunctionDef>;
}

// Individual heuristic implementations
pub struct NamingConventionHeuristic;
pub struct DocstringHeuristic;
pub struct TypeAnnotationHeuristic;
pub struct SymmetricPairHeuristic;
pub struct ModuleExportHeuristic;
pub struct DecoratorHeuristic;
pub struct AbstractMethodHeuristic;

// src/debt/dead_code.rs (updated)
impl DeadCodeDetector {
    pub fn detect_with_public_api_analysis(
        &self,
        function: &FunctionDef,
        context: &FileContext,
    ) -> Option<DeadCodeFinding> {
        // Check if function is public API
        let public_api_score = self.public_api_detector.is_public_api(function, context);

        if public_api_score.is_public {
            return None; // Public API, not dead code
        }

        // Continue with dead code detection
        if !self.has_callers(function) {
            Some(DeadCodeFinding {
                function: function.clone(),
                confidence: self.calculate_confidence(function),
                reason: "No callers detected - private function".to_string(),
                public_api_score: Some(public_api_score),
            })
        } else {
            None
        }
    }
}
```

### Heuristic Implementations

```rust
// Naming Convention Heuristic
impl ApiHeuristic for NamingConventionHeuristic {
    fn evaluate(&self, function: &FunctionDef, context: &FileContext) -> f32 {
        let name = &function.name;

        // Dunder methods (not dead code, but not public API either)
        if name.starts_with("__") && name.ends_with("__") {
            return 0.5;
        }

        // Leading underscore → internal
        if name.starts_with('_') {
            return 0.0;
        }

        // Module-level function without underscore → likely public
        if context.is_module_level(function) {
            return 1.0;
        }

        // Class method without underscore → public
        if context.is_class_method(function) && !name.starts_with('_') {
            return 0.8;
        }

        0.5 // Neutral
    }

    fn explain(&self, function: &FunctionDef) -> String {
        if function.name.starts_with('_') {
            "Function has leading underscore (private convention)".to_string()
        } else {
            "Function has no underscore prefix (public convention)".to_string()
        }
    }
}

// Docstring Heuristic
impl ApiHeuristic for DocstringHeuristic {
    fn evaluate(&self, function: &FunctionDef, _context: &FileContext) -> f32 {
        let docstring = match &function.docstring {
            Some(doc) => doc,
            None => return 0.0, // No docstring
        };

        let length = docstring.len();

        // Very short docstrings (< 20 chars) → minimal signal
        if length < 20 {
            return 0.2;
        }

        // Medium docstrings (20-50 chars) → some signal
        if length < 50 {
            return 0.5;
        }

        // Long docstrings (50-100 chars) → strong signal
        if length < 100 {
            return 0.8;
        }

        // Check for structured docstring (Google/Sphinx style)
        if self.is_structured_docstring(docstring) {
            return 1.0; // Very strong signal
        }

        // Very long docstrings → likely public API
        0.9
    }

    fn is_structured_docstring(&self, doc: &str) -> bool {
        let markers = ["Args:", "Returns:", "Raises:", "Yields:", "Parameters:", ":param", ":return"];
        markers.iter().any(|marker| doc.contains(marker))
    }
}

// Type Annotation Heuristic
impl ApiHeuristic for TypeAnnotationHeuristic {
    fn evaluate(&self, function: &FunctionDef, _context: &FileContext) -> f32 {
        let param_annotations = function.parameters.iter()
            .filter(|p| p.type_annotation.is_some())
            .count();

        let total_params = function.parameters.len();

        // No parameters → neutral
        if total_params == 0 {
            return 0.5;
        }

        // Calculate annotation completeness
        let annotation_ratio = param_annotations as f32 / total_params as f32;

        // Has return type annotation?
        let has_return_type = function.return_type.is_some();

        // Fully annotated (all params + return) → strong public API signal
        if annotation_ratio >= 1.0 && has_return_type {
            return 1.0;
        }

        // Partially annotated
        if has_return_type {
            return 0.5 + (annotation_ratio * 0.3);
        }

        annotation_ratio * 0.7
    }
}

// Symmetric Pair Heuristic
impl ApiHeuristic for SymmetricPairHeuristic {
    fn evaluate(&self, function: &FunctionDef, context: &FileContext) -> f32 {
        let pairs = vec![
            ("load", "save"),
            ("get", "set"),
            ("open", "close"),
            ("create", "destroy"),
            ("start", "stop"),
            ("acquire", "release"),
            ("add", "remove"),
            ("push", "pop"),
            ("read", "write"),
        ];

        let func_name = &function.name;

        for (first, second) in pairs {
            // Check if this function matches either side of pair
            if func_name.contains(first) || func_name.contains(second) {
                // Find the symmetric pair
                let pair_name = if func_name.contains(first) {
                    func_name.replace(first, second)
                } else {
                    func_name.replace(second, first)
                };

                // Check if pair exists in module
                if let Some(pair_func) = context.find_function(&pair_name) {
                    // If pair is used, mark this as public (symmetric API)
                    if context.is_function_used(&pair_func) {
                        return 1.0;
                    }

                    // Pair exists but not used → moderate signal
                    return 0.7;
                }
            }
        }

        0.0 // No symmetric pair found
    }

    fn explain(&self, function: &FunctionDef) -> String {
        format!("Function '{}' may be part of a symmetric API pair", function.name)
    }
}

// Module Export Heuristic
impl ApiHeuristic for ModuleExportHeuristic {
    fn evaluate(&self, function: &FunctionDef, context: &FileContext) -> f32 {
        // Check if function is in __all__
        if context.is_in_module_all(&function.name) {
            return 1.0; // Definite public API
        }

        // Check if imported in __init__.py
        if context.is_exported_in_init(&function.name) {
            return 1.0; // Package public API
        }

        0.0
    }
}
```

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct FileContext {
    pub file_path: PathBuf,
    pub module_all: Option<Vec<String>>,
    pub functions: HashMap<String, FunctionDef>,
    pub classes: HashMap<String, ClassDef>,
    pub used_functions: HashSet<String>,
    pub init_exports: Vec<String>,
}

impl FileContext {
    pub fn is_module_level(&self, function: &FunctionDef) -> bool;
    pub fn is_class_method(&self, function: &FunctionDef) -> bool;
    pub fn is_in_module_all(&self, name: &str) -> bool;
    pub fn is_exported_in_init(&self, name: &str) -> bool;
    pub fn find_function(&self, name: &str) -> Option<&FunctionDef>;
    pub fn is_function_used(&self, function: &FunctionDef) -> bool;
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub docstring: Option<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub decorators: Vec<String>,
    pub is_method: bool,
    pub class_name: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
}
```

### APIs and Interfaces

```rust
// Configuration in .debtmap.toml
[dead_code]
public_api_detection = true
public_api_threshold = 0.6

[dead_code.heuristics]
naming_convention_weight = 0.3
docstring_weight = 0.25
type_annotation_weight = 0.15
symmetric_pair_weight = 0.2
module_export_weight = 0.1

[dead_code.custom]
public_prefixes = ["api_", "public_"]
symmetric_pairs = [["fetch", "submit"], ["init", "cleanup"]]

// CLI options
Commands::Analyze {
    /// Disable public API detection
    #[arg(long = "no-public-api-detection")]
    no_public_api_detection: bool,

    /// Public API confidence threshold (0.0-1.0)
    #[arg(long = "public-api-threshold")]
    public_api_threshold: Option<f32>,
}
```

### Integration Points

1. **Dead Code Detector** (`src/debt/dead_code.rs`)
   - Query public API detector before marking as dead
   - Include public API score in findings

2. **Output Formatters** (`src/io/output/`)
   - Show public API reasoning in verbose mode
   - Include heuristic scores in JSON output

3. **Configuration** (`src/config.rs`)
   - Load public API detection settings
   - Validate heuristic weights sum to 1.0

## Dependencies

- **Prerequisites**:
  - Spec 109 (Cross-File Dependency Analysis) - Provides usage data
- **Affected Components**:
  - `src/debt/dead_code.rs` - Add public API check
  - `src/analyzers/python/` - Extract docstrings and annotations
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naming_convention_public() {
        let function = FunctionDef {
            name: "create_bots".to_string(),
            ..Default::default()
        };

        let heuristic = NamingConventionHeuristic;
        let score = heuristic.evaluate(&function, &FileContext::module_level());

        assert!(score >= 0.8, "Public function should score high");
    }

    #[test]
    fn test_naming_convention_private() {
        let function = FunctionDef {
            name: "_internal_helper".to_string(),
            ..Default::default()
        };

        let heuristic = NamingConventionHeuristic;
        let score = heuristic.evaluate(&function, &FileContext::module_level());

        assert_eq!(score, 0.0, "Private function should score 0");
    }

    #[test]
    fn test_docstring_structured() {
        let function = FunctionDef {
            name: "process_data".to_string(),
            docstring: Some(r#"
                Process input data and return results.

                Args:
                    data: Input data to process

                Returns:
                    Processed data
            "#.to_string()),
            ..Default::default()
        };

        let heuristic = DocstringHeuristic;
        let score = heuristic.evaluate(&function, &FileContext::default());

        assert!(score >= 0.9, "Structured docstring should score very high");
    }

    #[test]
    fn test_symmetric_pair_detection() {
        let save_func = FunctionDef {
            name: "save_chat_history".to_string(),
            ..Default::default()
        };

        let load_func = FunctionDef {
            name: "load_chat_history".to_string(),
            ..Default::default()
        };

        let mut context = FileContext::default();
        context.functions.insert("load_chat_history".to_string(), load_func.clone());
        context.used_functions.insert("load_chat_history".to_string());

        let heuristic = SymmetricPairHeuristic;
        let score = heuristic.evaluate(&save_func, &context);

        assert!(score >= 0.8, "Function with used symmetric pair should score high");
    }

    #[test]
    fn test_full_type_annotations() {
        let function = FunctionDef {
            name: "calculate".to_string(),
            parameters: vec![
                Parameter {
                    name: "x".to_string(),
                    type_annotation: Some("int".to_string()),
                    default_value: None,
                },
                Parameter {
                    name: "y".to_string(),
                    type_annotation: Some("int".to_string()),
                    default_value: None,
                },
            ],
            return_type: Some("int".to_string()),
            ..Default::default()
        };

        let heuristic = TypeAnnotationHeuristic;
        let score = heuristic.evaluate(&function, &FileContext::default());

        assert!(score >= 0.9, "Fully annotated function should score high");
    }

    #[test]
    fn test_module_all_export() {
        let function = FunctionDef {
            name: "exported_func".to_string(),
            ..Default::default()
        };

        let mut context = FileContext::default();
        context.module_all = Some(vec!["exported_func".to_string()]);

        let heuristic = ModuleExportHeuristic;
        let score = heuristic.evaluate(&function, &context);

        assert_eq!(score, 1.0, "Function in __all__ should score 1.0");
    }

    #[test]
    fn test_public_api_detector_integration() {
        let function = FunctionDef {
            name: "create_bots_from_list".to_string(),
            docstring: Some("Create bots from a list of bot configuration files.".to_string()),
            parameters: vec![
                Parameter {
                    name: "bot_files".to_string(),
                    type_annotation: Some("list".to_string()),
                    default_value: Some("None".to_string()),
                },
            ],
            ..Default::default()
        };

        let context = FileContext::module_level();
        let detector = PublicApiDetector::new(PublicApiConfig::default());
        let score = detector.is_public_api(&function, &context);

        assert!(score.is_public, "Function should be detected as public API");
        assert!(score.confidence >= 0.6, "Confidence should exceed threshold");
    }
}
```

### Integration Tests

**Test Case 1: Public API Function**
```python
# tests/fixtures/public_api/utils.py
def create_bots_from_list(bot_files: list = None, bot_path=None, simple: bool = False):
    """Create bots from a list of bot configuration files."""
    pass
```

Expected: NOT flagged as dead code (public API detected).

**Test Case 2: Private Helper**
```python
# tests/fixtures/public_api/utils.py
def _internal_parse(data):
    """Internal parsing helper."""
    pass
```

Expected: Flagged as dead code (private convention).

**Test Case 3: Symmetric Pair**
```python
# tests/fixtures/public_api/storage.py
def load_chat_history(bot_name):
    """Load chat history."""
    pass

def save_chat_history(bot_name, history):
    """Save chat history."""
    pass

# tests/fixtures/public_api/app.py
from storage import load_chat_history
load_chat_history("bot1")
```

Expected: Both `load_chat_history` AND `save_chat_history` NOT flagged as dead code.

**Test Case 4: Module Exports**
```python
# tests/fixtures/public_api/api.py
__all__ = ["public_function", "PublicClass"]

def public_function():
    pass

def _private_function():
    pass
```

Expected: `public_function` NOT flagged, `_private_function` flagged.

## Documentation Requirements

### Code Documentation

- Document each heuristic's algorithm and scoring
- Explain heuristic weight tuning
- Provide examples of public API patterns

### User Documentation

Add to user guide:

```markdown
## Public API Detection

Debtmap uses heuristics to avoid flagging public API functions as dead code:

### Detection Heuristics

1. **Naming Conventions**
   - Functions without `_` prefix → likely public
   - Functions with `_` prefix → internal/private
   - `__dunder__` methods → special methods (not dead code)

2. **Documentation Quality**
   - Comprehensive docstrings (> 50 chars) → public API signal
   - Structured docstrings (Args/Returns sections) → strong signal
   - Type hints + docstrings → very strong signal

3. **Symmetric Function Pairs**
   - `load`/`save`, `get`/`set`, `open`/`close`
   - If one function is used, both marked as public API

4. **Module Exports**
   - Functions in `__all__` → definite public API
   - Functions imported in `__init__.py` → package API

### Configuration

Adjust heuristic sensitivity:

```toml
# .debtmap.toml
[dead_code]
public_api_detection = true
public_api_threshold = 0.6  # 0.0-1.0

[dead_code.heuristics]
naming_convention_weight = 0.3
docstring_weight = 0.25
type_annotation_weight = 0.15
symmetric_pair_weight = 0.2
module_export_weight = 0.1
```

### Custom Patterns

Define project-specific patterns:

```toml
[dead_code.custom]
public_prefixes = ["api_", "public_", "handler_"]
symmetric_pairs = [["fetch", "submit"], ["init", "cleanup"]]
```

### Disabling Public API Detection

```bash
debtmap analyze src --no-public-api-detection
```

### Interpreting Results

Output shows public API reasoning:

```
#6 save_chat_history [PUBLIC API - NOT DEAD CODE]
  Location: genai_utils.py:51
  Confidence: 0.85
  Reasons:
    - No underscore prefix (0.8)
    - Has comprehensive docstring (0.7)
    - Symmetric pair with used load_chat_history (1.0)
```
```

### Architecture Documentation

Update ARCHITECTURE.md with public API detection pipeline.

## Implementation Notes

### Heuristic Tuning

Default weights were chosen based on:
- **Naming convention** (0.3): Strong signal in Python community
- **Docstring** (0.25): Good signal, but can be absent on public APIs
- **Type annotations** (0.15): Growing adoption, moderate signal
- **Symmetric pairs** (0.2): Very strong signal when detected
- **Module exports** (0.1): Definitive but uncommon

Threshold of 0.6 means function needs:
- Good naming + docstring, OR
- Good naming + symmetric pair, OR
- Module export + any other signal

### Performance Considerations

- **Docstring parsing**: Cache docstring structure analysis
- **Symmetric pair matching**: Build function name index once
- **Type annotation parsing**: Already available from AST

### Edge Cases

1. **Inconsistent conventions**: Project mixes public/private styles
2. **Generated code**: May lack docstrings but be public
3. **Test fixtures**: Public in test context, not production API
4. **Monkey-patched functions**: Added at runtime, not in source

## Migration and Compatibility

### Backward Compatibility

- **Opt-in feature**: Enabled by default but can be disabled
- **No breaking changes**: Reduces false positives only
- **Gradual rollout**: Test on library-style projects first

### Migration Path

For existing users:
1. **Automatic activation**: Public API detection runs by default
2. **Review flagged items**: Check if previously detected dead code is now marked public
3. **Tune configuration**: Adjust weights if needed for project conventions

## Future Enhancements

1. **Machine learning**: Train model on labeled public/private functions
2. **Usage frequency**: Track how often functions are imported externally
3. **Documentation generation**: Auto-generate API docs from detected public functions
4. **Language-specific patterns**: Adapt heuristics per language (Rust `pub`, JS `export`)
5. **Community conventions**: Learn from popular open-source project patterns

## Success Metrics

- **False positive reduction**: 30% → < 5%
- **Precision**: 95% of flagged dead code is actually dead
- **Recall**: Detect 90% of truly dead code
- **User satisfaction**: Zero complaints about "removed my public API"
- **Configuration adoption**: 20% of users customize heuristic weights

## Related Specifications

- Spec 112: Cross-File Dependency Analysis (provides usage data)
- Spec 113: Confidence Scoring System (uses public API scores)
