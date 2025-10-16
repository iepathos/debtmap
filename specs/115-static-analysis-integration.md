---
number: 115
title: Static Analysis Integration
category: foundation
priority: medium
status: draft
dependencies: []
created: 2025-10-16
---

# Specification 115: Static Analysis Integration

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap v0.2.8 focuses on complexity and dead code detection but does not integrate with existing static analysis tools (linters, type checkers) that can identify actual bugs in code. This leads to missed opportunities and misdiagnoses.

**Real-World Impact from Bug Report**:
- **Issue #5**: `ConversationPanel.on_message_added()` contains **undefined variable bug**
  ```python
  # Line 595 in conversation_panel.py
  if message is messages[index].message:  # ❌ 'messages' undefined, should be 'self.messages'
  ```
  - Debtmap flagged complexity but missed the actual bug
  - Would be caught by pylint, flake8, or mypy

- **Issue #9**: `DeliveryBoy.deliver()` references undefined `wx` module
  ```python
  wx.CallAfter(deliver, observers, message, index)  # ❌ 'wx' not imported in scope
  ```
  - Code is broken, not just unused
  - Would be caught by any Python linter

**Current Gaps**:
- No integration with pylint, flake8, mypy
- Cannot distinguish "broken code" from "dead code"
- No detection of undefined variables, missing imports
- No type checking integration
- Cannot validate if code even runs

**Why This Matters**:
- Broken code is higher priority than complex code
- Users waste time on code that doesn't compile/run
- Static analysis tools are industry standard
- Combining analyses provides better insights
- Can explain WHY code might be dead (it's broken)

## Objective

Integrate static analysis tools (pylint, flake8, mypy for Python; clippy for Rust) to detect code errors, enhance findings with static analysis warnings, and distinguish between "broken code" and "dead code".

## Requirements

### Functional Requirements

1. **Python Static Analysis Integration**
   - Run pylint on analyzed Python files
   - Run flake8 for style and error checking
   - Run mypy for type checking (if type hints present)
   - Parse and correlate warnings with debtmap findings
   - Aggregate static analysis errors per file/function

2. **Rust Static Analysis Integration**
   - Run cargo clippy on Rust code
   - Parse clippy warnings and errors
   - Correlate with debtmap complexity findings
   - Integrate with existing debtmap Rust analysis

3. **Error Classification**
   - Classify errors by severity: error, warning, info
   - Group by category: undefined-variable, type-error, import-error, etc.
   - Prioritize errors over warnings
   - Link errors to specific functions/lines

4. **Finding Enhancement**
   - Add static analysis warnings to debtmap findings
   - Flag code with errors as "broken" not just "complex"
   - Show undefined variables in dead code warnings
   - Enhance confidence scoring with error data

5. **Caching and Performance**
   - Cache static analysis results per file hash
   - Skip static analysis for unchanged files
   - Run static analyzers in parallel
   - Support incremental analysis

6. **Configuration and Control**
   - Option to enable/disable static analysis per tool
   - Configure which tools to run (pylint, flake8, mypy)
   - Set severity thresholds
   - Custom analyzer configurations

### Non-Functional Requirements

1. **Performance**
   - Static analysis adds < 30% to total analysis time
   - Parallel execution of multiple analyzers
   - Efficient caching of results
   - Skip analysis for files without issues

2. **Reliability**
   - Graceful degradation if tools not installed
   - Handle analyzer crashes/timeouts
   - Validate analyzer output format
   - Continue analysis if one analyzer fails

3. **Usability**
   - Clear error messages if tools missing
   - Installation instructions for analyzers
   - Show analyzer versions in output
   - Configurable analyzer options

4. **Portability**
   - Works on Linux, macOS, Windows
   - Detects installed analyzers automatically
   - Supports different analyzer versions
   - Fallback to debtmap-only analysis

## Acceptance Criteria

- [ ] Pylint integration runs and parses output
- [ ] Flake8 integration runs and parses output
- [ ] Mypy integration runs and parses output (optional)
- [ ] Clippy integration runs for Rust code
- [ ] Static analysis warnings correlated with debtmap findings
- [ ] Undefined variable in issue #5 detected and reported
- [ ] Missing import in issue #9 detected and reported
- [ ] "Broken code" classification separate from "dead code"
- [ ] Findings enhanced with static analysis warnings
- [ ] Caching reduces repeated static analysis runs
- [ ] Performance overhead < 30%
- [ ] Graceful degradation if tools not installed
- [ ] Configuration options for enabling/disabling tools
- [ ] Documentation includes setup instructions
- [ ] Integration tests with mock analyzer output

## Technical Details

### Implementation Approach

**Phase 1: Python Analyzer Integration**
1. Detect installed Python analyzers (pylint, flake8, mypy)
2. Run analyzers on Python files
3. Parse JSON/text output
4. Correlate warnings with debtmap findings

**Phase 2: Rust Analyzer Integration**
1. Run cargo clippy with JSON output
2. Parse clippy warnings
3. Correlate with Rust complexity findings

**Phase 3: Result Enhancement**
1. Add static analysis warnings to findings
2. Classify findings (broken vs dead vs complex)
3. Update confidence scores
4. Generate enhanced output

**Phase 4: Caching and Optimization**
1. Cache static analysis results
2. Implement incremental analysis
3. Parallel analyzer execution
4. Performance profiling

### Architecture Changes

```rust
// src/analysis/static_analysis/mod.rs
pub mod pylint;
pub mod flake8;
pub mod mypy;
pub mod clippy;

pub struct StaticAnalyzer {
    config: StaticAnalysisConfig,
    cache: AnalysisCache,
}

#[derive(Debug, Clone)]
pub struct StaticAnalysisConfig {
    pub enable_python: bool,
    pub enable_rust: bool,
    pub pylint_enabled: bool,
    pub flake8_enabled: bool,
    pub mypy_enabled: bool,
    pub clippy_enabled: bool,
    pub severity_threshold: Severity,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct StaticAnalysisResult {
    pub tool: AnalyzerTool,
    pub file: PathBuf,
    pub warnings: Vec<AnalyzerWarning>,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct AnalyzerWarning {
    pub line: usize,
    pub column: Option<usize>,
    pub severity: Severity,
    pub code: String,
    pub category: ErrorCategory,
    pub message: String,
    pub function: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    UndefinedVariable,
    TypeError,
    ImportError,
    SyntaxError,
    NameError,
    AttributeError,
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnalyzerTool {
    Pylint,
    Flake8,
    Mypy,
    Clippy,
}

impl StaticAnalyzer {
    pub fn new(config: StaticAnalysisConfig) -> Self;
    pub fn detect_available_tools() -> Vec<AnalyzerTool>;
    pub fn analyze_file(&self, file: &Path, language: Language) -> Result<Vec<StaticAnalysisResult>>;
    pub fn analyze_project(&self, files: &[PathBuf]) -> Result<HashMap<PathBuf, Vec<StaticAnalysisResult>>>;
    pub fn correlate_with_findings(&self, findings: &mut [DebtFinding], results: &HashMap<PathBuf, Vec<StaticAnalysisResult>>);
}

// src/analysis/static_analysis/pylint.rs
pub struct PylintAnalyzer {
    config_file: Option<PathBuf>,
    timeout: Duration,
}

impl PylintAnalyzer {
    pub fn new(config_file: Option<PathBuf>, timeout: Duration) -> Self;
    pub fn is_available() -> bool;
    pub fn run(&self, file: &Path) -> Result<StaticAnalysisResult>;
    pub fn parse_output(&self, output: &str) -> Result<Vec<AnalyzerWarning>>;
}

// Example pylint JSON output parsing
impl PylintAnalyzer {
    fn parse_json_output(&self, json: &str) -> Result<Vec<AnalyzerWarning>> {
        let messages: Vec<PylintMessage> = serde_json::from_str(json)?;

        messages.into_iter().map(|msg| {
            Ok(AnalyzerWarning {
                line: msg.line,
                column: Some(msg.column),
                severity: self.map_severity(&msg.type_),
                code: msg.symbol,
                category: self.categorize_error(&msg.message_id),
                message: msg.message,
                function: None, // Extract from context if possible
            })
        }).collect()
    }

    fn map_severity(&self, pylint_type: &str) -> Severity {
        match pylint_type {
            "error" | "fatal" => Severity::Error,
            "warning" => Severity::Warning,
            "info" | "refactor" | "convention" => Severity::Info,
            _ => Severity::Warning,
        }
    }

    fn categorize_error(&self, message_id: &str) -> ErrorCategory {
        match message_id {
            "E0602" => ErrorCategory::UndefinedVariable,
            "E0401" => ErrorCategory::ImportError,
            "E1101" => ErrorCategory::AttributeError,
            code if code.starts_with("E") => ErrorCategory::Other(code.to_string()),
            _ => ErrorCategory::Other(message_id.to_string()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct PylintMessage {
    #[serde(rename = "type")]
    type_: String,
    message: String,
    #[serde(rename = "message-id")]
    message_id: String,
    symbol: String,
    line: usize,
    column: usize,
}

// src/debt/dead_code.rs (updated)
#[derive(Debug, Clone)]
pub struct DeadCodeFinding {
    pub function: FunctionDef,
    pub confidence: f32,
    pub reason: String,
    pub static_analysis_warnings: Vec<AnalyzerWarning>, // NEW
    pub is_broken: bool, // NEW
}

impl DeadCodeDetector {
    pub fn detect_with_static_analysis(
        &self,
        function: &FunctionDef,
        context: &ProjectContext,
        static_warnings: &[AnalyzerWarning],
    ) -> Option<DeadCodeFinding> {
        let has_errors = static_warnings.iter().any(|w| w.severity == Severity::Error);

        if !self.has_callers(function) {
            let mut finding = DeadCodeFinding {
                function: function.clone(),
                confidence: self.calculate_confidence(function),
                reason: self.generate_reason(function, static_warnings),
                static_analysis_warnings: static_warnings.to_vec(),
                is_broken: has_errors,
            };

            // Adjust reason if broken
            if has_errors {
                finding.reason = format!("Code contains errors and has no callers - likely broken: {}",
                    self.format_errors(static_warnings));
            }

            Some(finding)
        } else {
            None
        }
    }

    fn format_errors(&self, warnings: &[AnalyzerWarning]) -> String {
        warnings
            .iter()
            .filter(|w| w.severity == Severity::Error)
            .map(|w| format!("line {}: {}", w.line, w.message))
            .collect::<Vec<_>>()
            .join("; ")
    }
}
```

### Data Structures

```rust
// Integration with ProjectContext
#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub file_analyses: HashMap<PathBuf, FileAnalysis>,
    pub symbol_resolver: SymbolResolver,
    pub call_graph: CrossFileCallGraph,
    pub patterns: Vec<PatternInstance>,
    pub static_analysis: HashMap<PathBuf, Vec<StaticAnalysisResult>>, // NEW
}

// Cache structure
#[derive(Debug, Clone)]
pub struct AnalysisCache {
    // Map from (file_hash, analyzer) to results
    cache: HashMap<(String, AnalyzerTool), StaticAnalysisResult>,
}

impl AnalysisCache {
    pub fn get(&self, file_hash: &str, tool: AnalyzerTool) -> Option<&StaticAnalysisResult>;
    pub fn insert(&mut self, file_hash: String, tool: AnalyzerTool, result: StaticAnalysisResult);
    pub fn invalidate_file(&mut self, file_hash: &str);
}
```

### APIs and Interfaces

```rust
// Configuration in .debtmap.toml
[static_analysis]
enabled = true
timeout_seconds = 30

[static_analysis.python]
pylint_enabled = true
flake8_enabled = true
mypy_enabled = false  # Optional, slower

[static_analysis.rust]
clippy_enabled = true

[static_analysis.severity]
threshold = "warning"  # error, warning, info

// CLI options
Commands::Analyze {
    /// Enable static analysis integration
    #[arg(long = "static-analysis")]
    static_analysis: bool,

    /// Disable static analysis (default if not specified)
    #[arg(long = "no-static-analysis")]
    no_static_analysis: bool,

    /// Specify which analyzers to run
    #[arg(long = "analyzers")]
    analyzers: Option<Vec<String>>, // pylint,flake8,mypy,clippy
}

// Output format
{
  "file": "conversation_panel.py",
  "function": "on_message_added",
  "line": 595,
  "type": "dead_code",
  "severity": "medium",
  "is_broken": true,
  "reason": "Code contains errors and has no callers - likely broken",
  "static_analysis": [
    {
      "tool": "pylint",
      "line": 595,
      "severity": "error",
      "code": "E0602",
      "category": "undefined-variable",
      "message": "Undefined variable 'messages'"
    }
  ]
}
```

### Integration Points

1. **File Analysis Pipeline**
   - Run static analyzers after parsing
   - Cache results alongside file metrics
   - Pass to dead code detector

2. **Dead Code Detector** (`src/debt/dead_code.rs`)
   - Query static analysis results
   - Enhance findings with warnings
   - Classify broken vs dead code

3. **Output Formatters** (`src/io/output/`)
   - Include static analysis warnings in JSON
   - Show errors in terminal output
   - Generate separate error report

4. **Caching** (`src/cache/`)
   - Cache static analysis per file hash
   - Invalidate on file changes
   - Separate cache from debtmap metrics

## Dependencies

- **Prerequisites**: None (standalone feature)
- **Affected Components**:
  - `src/analysis/static_analysis/` - New module
  - `src/debt/dead_code.rs` - Integrate static warnings
  - `src/io/output/` - Display static analysis results
- **External Dependencies**:
  - Python: pylint, flake8, mypy (optional, detected at runtime)
  - Rust: cargo clippy (usually installed with Rust)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pylint_output_parsing() {
        let json_output = r#"[
            {
                "type": "error",
                "message": "Undefined variable 'messages'",
                "message-id": "E0602",
                "symbol": "undefined-variable",
                "line": 595,
                "column": 20
            }
        ]"#;

        let analyzer = PylintAnalyzer::new(None, Duration::from_secs(30));
        let warnings = analyzer.parse_json_output(json_output).unwrap();

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].severity, Severity::Error);
        assert_eq!(warnings[0].category, ErrorCategory::UndefinedVariable);
        assert_eq!(warnings[0].line, 595);
    }

    #[test]
    fn test_flake8_output_parsing() {
        let output = "conversation_panel.py:595:20: F821 undefined name 'messages'";

        let analyzer = Flake8Analyzer::new(Duration::from_secs(30));
        let warnings = analyzer.parse_output(output).unwrap();

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "F821");
        assert_eq!(warnings[0].line, 595);
    }

    #[test]
    fn test_broken_code_classification() {
        let function = FunctionDef {
            name: "on_message_added".to_string(),
            line: 583,
            ..Default::default()
        };

        let warnings = vec![
            AnalyzerWarning {
                line: 595,
                column: Some(20),
                severity: Severity::Error,
                code: "E0602".to_string(),
                category: ErrorCategory::UndefinedVariable,
                message: "Undefined variable 'messages'".to_string(),
                function: Some("on_message_added".to_string()),
            }
        ];

        let detector = DeadCodeDetector::new(Config::default());
        let finding = detector.detect_with_static_analysis(&function, &ProjectContext::default(), &warnings);

        assert!(finding.is_some());
        let finding = finding.unwrap();
        assert!(finding.is_broken);
        assert!(finding.reason.contains("broken"));
        assert_eq!(finding.static_analysis_warnings.len(), 1);
    }

    #[test]
    fn test_analyzer_tool_detection() {
        let available = StaticAnalyzer::detect_available_tools();

        // Should detect at least one tool on development machine
        assert!(!available.is_empty(), "No static analysis tools detected");
    }
}
```

### Integration Tests

**Test Case 1: Undefined Variable Detection**
```python
# tests/fixtures/static_analysis/undefined_var.py
def process_data():
    result = undefined_variable + 1  # ❌ Should be detected
    return result
```

Expected: Static analysis detects undefined variable, finding marked as "broken".

**Test Case 2: Missing Import**
```python
# tests/fixtures/static_analysis/missing_import.py
def use_numpy():
    return numpy.array([1, 2, 3])  # ❌ numpy not imported
```

Expected: Import error detected, function flagged as broken.

**Test Case 3: Type Error (mypy)**
```python
# tests/fixtures/static_analysis/type_error.py
def add_numbers(a: int, b: int) -> int:
    return a + "wrong"  # ❌ Type error
```

Expected: Mypy detects type error, enhanced finding.

### Performance Tests

```rust
#[test]
fn test_static_analysis_performance() {
    let temp_dir = create_test_python_project(100); // 100 files
    let files = discover_python_files(&temp_dir);

    // Baseline: debtmap only
    let start = Instant::now();
    let baseline_analysis = analyze_project_without_static_analysis(files.clone()).unwrap();
    let baseline_time = start.elapsed();

    // With static analysis
    let start = Instant::now();
    let enhanced_analysis = analyze_project_with_static_analysis(files).unwrap();
    let enhanced_time = start.elapsed();

    let overhead_pct = ((enhanced_time.as_secs_f32() - baseline_time.as_secs_f32())
        / baseline_time.as_secs_f32()) * 100.0;

    assert!(overhead_pct < 30.0, "Static analysis overhead: {:.1}%", overhead_pct);
}
```

## Documentation Requirements

### Code Documentation

- Document static analyzer integrations
- Explain output parsing for each tool
- Document error categorization

### User Documentation

Add to user guide:

```markdown
## Static Analysis Integration

Debtmap can integrate with existing static analysis tools to enhance findings:

### Supported Tools

**Python**:
- **pylint**: Comprehensive linting and error detection
- **flake8**: Style checker and error detector
- **mypy**: Type checking (optional)

**Rust**:
- **clippy**: Rust linter and code quality tool

### Installation

Install Python analyzers:
```bash
pip install pylint flake8 mypy
```

Rust clippy (usually included with Rust):
```bash
rustup component add clippy
```

### Usage

Enable static analysis:
```bash
# Automatic detection and running
debtmap analyze src --static-analysis

# Specify analyzers
debtmap analyze src --analyzers pylint,flake8

# Disable (default)
debtmap analyze src  # No --static-analysis flag
```

### Configuration

```toml
# .debtmap.toml
[static_analysis]
enabled = true
timeout_seconds = 30

[static_analysis.python]
pylint_enabled = true
flake8_enabled = true
mypy_enabled = false  # Slower, optional

[static_analysis.severity]
threshold = "warning"  # error, warning, info
```

### Output

Enhanced findings show static analysis warnings:

```
#5 ConversationPanel.on_message_added [BROKEN CODE]
  Location: conversation_panel.py:595
  Reason: Code contains errors and has no callers - likely broken

  Static Analysis Errors:
    [pylint E0602] line 595: Undefined variable 'messages'

  Recommendation: Fix undefined variable before refactoring
```

### Performance

- Static analysis adds ~20-30% to total analysis time
- Results are cached per file hash
- Analyzers run in parallel
- Graceful degradation if tools not installed
```

### Architecture Documentation

Update ARCHITECTURE.md with static analysis integration pipeline.

## Implementation Notes

### Analyzer Detection

Detect tools at runtime:
```rust
impl StaticAnalyzer {
    pub fn detect_available_tools() -> Vec<AnalyzerTool> {
        let mut tools = Vec::new();

        if Command::new("pylint").arg("--version").output().is_ok() {
            tools.push(AnalyzerTool::Pylint);
        }

        if Command::new("flake8").arg("--version").output().is_ok() {
            tools.push(AnalyzerTool::Flake8);
        }

        if Command::new("mypy").arg("--version").output().is_ok() {
            tools.push(AnalyzerTool::Mypy);
        }

        if Command::new("cargo").arg("clippy").arg("--version").output().is_ok() {
            tools.push(AnalyzerTool::Clippy);
        }

        tools
    }
}
```

### Running Analyzers

```rust
impl PylintAnalyzer {
    pub fn run(&self, file: &Path) -> Result<StaticAnalysisResult> {
        let output = Command::new("pylint")
            .arg("--output-format=json")
            .arg(file)
            .timeout(self.timeout)
            .output()
            .context("Failed to run pylint")?;

        let warnings = self.parse_json_output(&String::from_utf8_lossy(&output.stdout))?;

        Ok(StaticAnalysisResult {
            tool: AnalyzerTool::Pylint,
            file: file.to_path_buf(),
            warnings,
            duration: Duration::default(),
        })
    }
}
```

### Parallel Execution

```rust
impl StaticAnalyzer {
    pub fn analyze_project(&self, files: &[PathBuf]) -> Result<HashMap<PathBuf, Vec<StaticAnalysisResult>>> {
        files
            .par_iter()
            .map(|file| {
                let results = self.analyze_file(file, self.detect_language(file))?;
                Ok((file.clone(), results))
            })
            .collect()
    }
}
```

### Edge Cases

1. **Tool not installed**: Graceful skip, warn user
2. **Tool timeout**: Skip file, log warning
3. **Parse errors**: Log error, continue analysis
4. **Tool crashes**: Catch error, continue with remaining tools
5. **Large files**: Apply timeout to prevent hangs

## Migration and Compatibility

### Backward Compatibility

- **Opt-in feature**: Requires `--static-analysis` flag
- **No breaking changes**: Enhances existing findings
- **Graceful degradation**: Works without tools installed

### Migration Path

For existing users:
1. **Install analyzers**: `pip install pylint flake8`
2. **Enable feature**: Add `--static-analysis` to commands
3. **Review enhanced findings**: Check for broken code classification
4. **Configure**: Tune analyzer settings in `.debtmap.toml`

## Future Enhancements

1. **Additional analyzers**: bandit (security), radon (complexity), vulture (dead code)
2. **Custom analyzers**: User-defined analyzer integration
3. **Fix suggestions**: Auto-fix common issues (undefined vars, imports)
4. **IDE integration**: Real-time analysis in editors
5. **Continuous integration**: GitHub Actions, GitLab CI integration
6. **Historical tracking**: Track error trends over time

## Success Metrics

- **Error detection**: Catch 95% of undefined variables and import errors
- **Broken code identification**: Flag broken code with 90% accuracy
- **Performance**: < 30% overhead
- **Adoption**: 40% of users enable static analysis
- **User satisfaction**: Positive feedback on enhanced findings

## Related Specifications

- Spec 112: Cross-File Dependency Analysis (complementary analysis)
- Spec 116: Confidence Scoring System (uses static analysis for confidence)
