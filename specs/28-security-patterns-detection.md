---
number: 28
title: Security Patterns Detection
category: feature
priority: high
status: draft
dependencies: []
created: 2025-08-16
---

# Specification 28: Security Patterns Detection

**Category**: feature
**Priority**: high
**Status**: draft
**Dependencies**: []

## Context

While excellent security tools exist for individual languages (clippy for Rust, bandit for Python, ESLint security plugins for JavaScript), there are critical gaps in security analysis that debtmap is uniquely positioned to address:

1. **Security as Technical Debt**: Existing tools treat security issues in isolation, but security vulnerabilities are a form of technical debt that should be prioritized alongside complexity and structural issues.

2. **Cross-Language Consistency**: Organizations using multiple languages lack a unified view of security debt across their entire codebase.

3. **Gaps in Existing Tools**:
   - **Hardcoded Secrets** - Not comprehensively covered by language-specific linters
   - **SQL Injection in Rust** - Limited detection in existing Rust tooling
   - **Cross-Function Data Flow** - Input validation tracking across boundaries
   - **Custom Organization Patterns** - Company-specific security anti-patterns

4. **Integration Challenge**: Teams currently must run multiple security tools and manually correlate findings with other code quality metrics.

Debtmap will focus on detecting security patterns that are either not covered by existing tools or benefit from integration with technical debt scoring, while leveraging existing tools for well-solved problems.

## Objective

Create a security debt detection system that:

1. **Fills Critical Gaps**: Focuses on security patterns not adequately covered by existing tools
2. **Integrates Security with Technical Debt**: Provides unified scoring and prioritization
3. **Aggregates External Tool Findings**: Acts as a hub for security tool outputs
4. **Enables Custom Patterns**: Supports organization-specific security requirements

### Primary Focus Areas

1. **Hardcoded Secrets Detection** (High Priority)
   - Comprehensive pattern matching for credentials, API keys, tokens
   - Entropy-based detection for unknown secret formats
   - Variable name and context analysis

2. **SQL Injection Analysis** (High Priority)
   - String concatenation patterns in SQL contexts
   - Dynamic query construction detection
   - Cross-function taint analysis

3. **Input Validation Tracking** (Medium Priority)
   - Data flow analysis from external inputs to usage points
   - Missing sanitization detection
   - Cross-function validation gaps

4. **Security Tool Integration** (High Priority)
   - Import findings from clippy, bandit, ESLint security plugins
   - Normalize and score external security findings
   - Unified reporting with other debt metrics

### Explicitly Out of Scope

These are well-handled by existing tools and should not be reimplemented:
- Basic unsafe block detection (use clippy)
- Standard cryptographic algorithm deprecation (use dedicated crypto auditors)
- Dependency vulnerability scanning (use cargo-audit, npm audit, safety)
- Simple pattern matching covered by semgrep rules

## Requirements

### Functional Requirements

1. **Hardcoded Secret Detection**
   - Pattern matching for common secret formats (API keys, passwords, tokens)
   - Entropy-based detection (Shannon entropy > 4.5 for unknown patterns)
   - Context-aware analysis (variable names, comments, string literals)
   - Base64/hex encoded secret detection
   - Configurable patterns for organization-specific secrets
   - Allowlist support for false positive management

2. **SQL Injection Risk Detection**
   - String concatenation patterns with SQL keywords
   - Format string usage in SQL contexts
   - Dynamic query construction without parameterization
   - Taint analysis tracking user input to SQL execution
   - Support for ORM-specific patterns (diesel, sqlx, etc.)

3. **Input Validation Gap Analysis**
   - Data flow tracking from input sources to usage points
   - External input sources (HTTP requests, file I/O, CLI arguments)
   - Cross-function taint propagation
   - Missing sanitization before dangerous operations
   - Path traversal and command injection patterns

4. **Security Tool Integration**
   - Plugin architecture for external tool integration
   - Supported tools:
     - Rust: clippy security lints, cargo-audit
     - Python: bandit, safety
     - JavaScript/TypeScript: ESLint security plugins, npm audit
   - Finding normalization and deduplication
   - Unified severity scoring across tools

5. **Custom Security Patterns**
   - YAML/TOML-based custom rule definition
   - Organization-specific anti-pattern detection
   - Regex and AST-based pattern matching
   - Severity and remediation guidance configuration

### Non-Functional Requirements

1. **Performance**
   - Security analysis adds <15% overhead to total analysis time
   - Efficient pattern matching using compiled regex and AST visitors
   - Incremental analysis support for large codebases

2. **Accuracy**
   - >90% precision for high-severity security patterns (low false positives)
   - >80% recall for critical security vulnerabilities
   - Configurable sensitivity levels to balance precision vs. recall

3. **Extensibility**
   - Plugin architecture for custom security patterns
   - Configuration support for organization-specific security rules
   - Integration with existing security scanning tools

## Acceptance Criteria

- [ ] **Secret Detection**: Hardcoded secrets identified with >90% precision, <5% false positive rate
- [ ] **SQL Injection Detection**: Dynamic SQL patterns detected with taint analysis
- [ ] **Input Validation**: Cross-function data flow tracking implemented
- [ ] **Tool Integration**: Successfully imports findings from at least 3 external tools
- [ ] **Custom Patterns**: Support for organization-specific rules via configuration
- [ ] **Unified Scoring**: Security findings integrated into overall debt score
- [ ] **Performance**: Security analysis adds <10% overhead to baseline analysis
- [ ] **Incremental Analysis**: Supports caching and incremental updates

## Technical Details

### Implementation Approach

The implementation will be structured in three phases:

**Phase 1**: Core detection capabilities (Secrets, SQL Injection)
**Phase 2**: Tool integration framework
**Phase 3**: Advanced analysis (Input validation tracking, custom patterns)

#### 1. Security Pattern Framework (`src/security/`)

```rust
/// Security vulnerability detection framework
pub mod security {
    use crate::core::ast::AstNode;
    use crate::core::{DebtItem, Priority};
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum SecurityVulnerability {
        HardcodedSecret {
            secret_type: SecretType,
            confidence: f64,
            value_preview: String,
            entropy: f64,
        },
        SqlInjection {
            injection_type: SqlInjectionType,
            taint_source: TaintSource,
            confidence: f64,
            severity: Severity,
        },
        InputValidationGap {
            input_source: InputSource,
            sink_operation: SinkOperation,
            taint_path: Vec<String>,
            severity: Severity,
        },
        ExternalToolFinding {
            tool: String,
            original_severity: String,
            normalized_severity: Severity,
            description: String,
            remediation: Option<String>,
        },
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum SecretType {
        ApiKey,
        Password,
        PrivateKey,
        DatabaseCredential,
        AuthToken,
        JwtSecret,
        WebhookSecret,
        EncryptionKey,
        Unknown, // For entropy-based detection
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum SqlInjectionType {
        StringConcatenation,
        FormatString,
        DynamicQuery,
        TemplateInjection,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum TaintSource {
        HttpRequest,
        CliArgument,
        Environment,
        FileInput,
        DatabaseQuery,
        UserControlled,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum SinkOperation {
        SqlQuery,
        FileSystem,
        ProcessExecution,
        NetworkRequest,
        Deserialization,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum RiskLevel {
        Critical,
        High,
        Medium,
        Low,
    }
    
    pub trait SecurityDetector {
        fn detect_vulnerabilities(&self, ast: &AstNode) -> Vec<SecurityVulnerability>;
        fn detector_name(&self) -> &'static str;
        fn supported_languages(&self) -> Vec<Language>;
    }
}
```

#### 2. Tool Integration Framework (`src/security/tool_integration.rs`)

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait SecurityToolAdapter {
    async fn run_tool(&self, path: &Path) -> Result<Vec<ToolFinding>, Error>;
    fn tool_name(&self) -> &str;
    fn supported_languages(&self) -> Vec<Language>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFinding {
    pub tool: String,
    pub severity: String,
    pub rule_id: String,
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
    pub message: String,
    pub remediation: Option<String>,
}

pub struct ClippyAdapter;

#[async_trait]
impl SecurityToolAdapter for ClippyAdapter {
    async fn run_tool(&self, path: &Path) -> Result<Vec<ToolFinding>, Error> {
        let output = Command::new("cargo")
            .args(&["clippy", "--message-format=json", "--", 
                   "-W", "clippy::unwrap_used",
                   "-W", "clippy::expect_used",
                   "-W", "clippy::panic"])
            .current_dir(path)
            .output()
            .await?;
        
        self.parse_clippy_output(&output.stdout)
    }
    
    fn tool_name(&self) -> &str { "clippy" }
    fn supported_languages(&self) -> Vec<Language> { vec![Language::Rust] }
}

pub struct BanditAdapter;

#[async_trait]
impl SecurityToolAdapter for BanditAdapter {
    async fn run_tool(&self, path: &Path) -> Result<Vec<ToolFinding>, Error> {
        let output = Command::new("bandit")
            .args(&["-r", path.to_str().unwrap(), "-f", "json"])
            .output()
            .await?;
        
        self.parse_bandit_output(&output.stdout)
    }
    
    fn tool_name(&self) -> &str { "bandit" }
    fn supported_languages(&self) -> Vec<Language> { vec![Language::Python] }
}

pub struct ToolIntegrationManager {
    adapters: Vec<Box<dyn SecurityToolAdapter>>,
    cache: HashMap<PathBuf, CachedFindings>,
}

impl ToolIntegrationManager {
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(ClippyAdapter),
                Box::new(BanditAdapter),
                Box::new(EslintSecurityAdapter),
            ],
            cache: HashMap::new(),
        }
    }
    
    pub async fn run_external_tools(&mut self, path: &Path, language: Language) 
        -> Result<Vec<SecurityVulnerability>, Error> {
        let mut all_findings = Vec::new();
        
        for adapter in &self.adapters {
            if adapter.supported_languages().contains(&language) {
                if let Ok(findings) = adapter.run_tool(path).await {
                    all_findings.extend(self.normalize_findings(findings));
                }
            }
        }
        
        self.deduplicate_findings(all_findings)
    }
    
    fn normalize_findings(&self, findings: Vec<ToolFinding>) -> Vec<SecurityVulnerability> {
        findings.into_iter().map(|finding| {
            SecurityVulnerability::ExternalToolFinding {
                tool: finding.tool,
                original_severity: finding.severity.clone(),
                normalized_severity: self.normalize_severity(&finding.severity),
                description: finding.message,
                remediation: finding.remediation,
            }
        }).collect()
    }
    
    fn normalize_severity(&self, severity: &str) -> Severity {
        match severity.to_lowercase().as_str() {
            "critical" | "high" => Severity::High,
            "medium" | "moderate" => Severity::Medium,
            "low" | "info" => Severity::Low,
            _ => Severity::Unknown,
        }
    }
}
```

#### 3. Secret Detector (`src/security/secret_detector.rs`)

```rust
pub struct SecretDetector {
    patterns: HashMap<SecretType, Vec<regex::Regex>>,
    entropy_threshold: f64,
}

impl SecurityDetector for SecretDetector {
    fn detect_vulnerabilities(&self, ast: &AstNode) -> Vec<SecurityVulnerability> {
        let mut vulnerabilities = Vec::new();
        
        // Check string literals
        let string_literals = self.extract_string_literals(ast);
        for literal in string_literals {
            if let Some(secret) = self.analyze_string_for_secrets(&literal) {
                vulnerabilities.push(secret);
            }
        }
        
        // Check variable names and assignments
        let assignments = self.extract_assignments(ast);
        for assignment in assignments {
            if let Some(secret) = self.analyze_assignment_for_secrets(&assignment) {
                vulnerabilities.push(secret);
            }
        }
        
        vulnerabilities
    }
}

impl SecretDetector {
    fn analyze_string_for_secrets(&self, literal: &StringLiteral) -> Option<SecurityVulnerability> {
        // Pattern matching against known secret formats
        for (secret_type, patterns) in &self.patterns {
            for pattern in patterns {
                if pattern.is_match(&literal.value) {
                    let confidence = self.calculate_confidence(&literal.value, pattern);
                    
                    if confidence > 0.7 {
                        return Some(SecurityVulnerability::HardcodedSecret {
                            secret_type: *secret_type,
                            confidence,
                            value_preview: self.create_preview(&literal.value),
                        });
                    }
                }
            }
        }
        
        // Entropy analysis for unknown secret formats
        if self.calculate_entropy(&literal.value) > self.entropy_threshold {
            return Some(SecurityVulnerability::HardcodedSecret {
                secret_type: SecretType::AuthToken,
                confidence: 0.6,
                value_preview: self.create_preview(&literal.value),
            });
        }
        
        None
    }
    
    fn calculate_entropy(&self, value: &str) -> f64 {
        let mut char_counts = HashMap::new();
        for c in value.chars() {
            *char_counts.entry(c).or_insert(0) += 1;
        }
        
        let length = value.len() as f64;
        let entropy = char_counts.values()
            .map(|&count| {
                let probability = count as f64 / length;
                -probability * probability.log2()
            })
            .sum();
            
        entropy
    }
    
    fn create_preview(&self, value: &str) -> String {
        let preview_length = 8.min(value.len());
        let visible = &value[..preview_length];
        let hidden_count = value.len() - preview_length;
        
        if hidden_count > 0 {
            format!("{}***({} chars hidden)", visible, hidden_count)
        } else {
            visible.to_string()
        }
    }
}
```

#### 4. SQL Injection Detector (`src/security/sql_injection_detector.rs`)

```rust
pub struct SqlInjectionDetector {
    sql_keywords: HashSet<String>,
    dangerous_patterns: Vec<regex::Regex>,
}

impl SecurityDetector for SqlInjectionDetector {
    fn detect_vulnerabilities(&self, ast: &AstNode) -> Vec<SecurityVulnerability> {
        let mut vulnerabilities = Vec::new();
        
        // Find string concatenations that might be SQL
        let concatenations = self.find_string_concatenations(ast);
        for concat in concatenations {
            if let Some(vulnerability) = self.analyze_sql_concatenation(&concat) {
                vulnerabilities.push(vulnerability);
            }
        }
        
        // Find format string usage in SQL contexts
        let format_calls = self.find_format_calls(ast);
        for format_call in format_calls {
            if let Some(vulnerability) = self.analyze_sql_format(&format_call) {
                vulnerabilities.push(vulnerability);
            }
        }
        
        vulnerabilities
    }
}

impl SqlInjectionDetector {
    fn analyze_sql_concatenation(&self, concat: &StringConcatenation) -> Option<SecurityVulnerability> {
        // Check if any part contains SQL keywords
        let has_sql = concat.parts.iter().any(|part| {
            self.contains_sql_keywords(&part.value)
        });
        
        if !has_sql {
            return None;
        }
        
        // Check if any part comes from user input
        let user_input_flow = concat.parts.iter().any(|part| {
            self.traces_to_user_input(part)
        });
        
        let severity = if user_input_flow {
            Severity::High
        } else {
            Severity::Medium
        };
        
        Some(SecurityVulnerability::SqlInjection {
            injection_type: SqlInjectionType::StringConcatenation,
            user_input_flow,
            severity,
        })
    }
    
    fn contains_sql_keywords(&self, text: &str) -> bool {
        let text_upper = text.to_uppercase();
        self.sql_keywords.iter().any(|keyword| {
            text_upper.contains(keyword)
        })
    }
    
    fn traces_to_user_input(&self, part: &StringPart) -> bool {
        // Data flow analysis to determine if this value originated from user input
        // This would involve tracking variable assignments and function parameters
        // Simplified implementation for specification purposes
        match &part.source {
            StringSource::Parameter => true,
            StringSource::FunctionCall(call) if self.is_input_function(call) => true,
            StringSource::Variable(var) => self.variable_traces_to_input(var),
            _ => false,
        }
    }
    
    fn is_input_function(&self, call: &FunctionCall) -> bool {
        const INPUT_FUNCTIONS: &[&str] = &[
            "read_line", "args", "env", "request_body", "query_param",
            "header", "form_data", "json_body", "path_param"
        ];
        
        INPUT_FUNCTIONS.contains(&call.function_name.as_str())
    }
}
```

#### 5. Taint Analysis for Input Validation (`src/security/taint_analysis.rs`)

```rust
pub struct TaintAnalyzer {
    taint_sources: HashSet<String>,
    taint_sinks: HashSet<String>,
    taint_graph: DiGraph<TaintNode, TaintEdge>,
}

#[derive(Debug, Clone)]
pub struct TaintNode {
    pub id: String,
    pub node_type: TaintNodeType,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub enum TaintNodeType {
    Source(TaintSource),
    Sink(SinkOperation),
    Propagator(String), // Function or variable name
    Sanitizer(String),  // Validation function
}

impl TaintAnalyzer {
    pub fn analyze_data_flow(&mut self, ast: &AstNode) -> Vec<SecurityVulnerability> {
        // Build taint propagation graph
        self.build_taint_graph(ast);
        
        // Find paths from sources to sinks
        let vulnerable_paths = self.find_taint_paths();
        
        // Convert paths to vulnerabilities
        vulnerable_paths.into_iter().map(|path| {
            SecurityVulnerability::InputValidationGap {
                input_source: path.source,
                sink_operation: path.sink,
                taint_path: path.nodes,
                severity: self.assess_path_severity(&path),
            }
        }).collect()
    }
    
    fn build_taint_graph(&mut self, ast: &AstNode) {
        // Identify taint sources (user inputs)
        let sources = self.find_taint_sources(ast);
        for source in sources {
            self.add_source_node(source);
        }
        
        // Track data flow through variables and functions
        let assignments = self.find_assignments(ast);
        for assignment in assignments {
            self.track_assignment_flow(assignment);
        }
        
        // Identify taint sinks (dangerous operations)
        let sinks = self.find_taint_sinks(ast);
        for sink in sinks {
            self.add_sink_node(sink);
        }
    }
    
    fn find_taint_paths(&self) -> Vec<TaintPath> {
        let mut vulnerable_paths = Vec::new();
        
        // Use graph algorithms to find paths from sources to sinks
        for source in self.get_source_nodes() {
            for sink in self.get_sink_nodes() {
                if let Some(path) = self.find_path(source, sink) {
                    // Check if path has sanitizers
                    if !self.path_has_sanitizer(&path) {
                        vulnerable_paths.push(path);
                    }
                }
            }
        }
        
        vulnerable_paths
    }
}
```

#### 6. Integration with Existing System (`src/analyzers/rust.rs`)

```rust
use crate::security::{
    SecurityDetector, SecretDetector, SqlInjectionDetector,
    TaintAnalyzer, ToolIntegrationManager
};

// Add to analyze_rust_file function
fn analyze_rust_file(ast: &RustAst, threshold: u32) -> FileMetrics {
    // ... existing analysis ...
    
    // Security analysis
    let security_items = analyze_security_patterns(&ast.file, &ast.path);
    debt_items.extend(security_items);
    
    // ... rest of analysis ...
}

async fn analyze_security_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    let mut security_items = Vec::new();
    
    // Run native detectors
    let detectors: Vec<Box<dyn SecurityDetector>> = vec![
        Box::new(SecretDetector::new()),
        Box::new(SqlInjectionDetector::new()),
    ];
    
    let ast_node = convert_syn_to_ast_node(file);
    
    for detector in detectors {
        let vulnerabilities = detector.detect_vulnerabilities(&ast_node);
        
        for vulnerability in vulnerabilities {
            let debt_item = convert_vulnerability_to_debt_item(vulnerability, path);
            security_items.push(debt_item);
        }
    }
    
    // Run taint analysis
    let mut taint_analyzer = TaintAnalyzer::new();
    let taint_vulnerabilities = taint_analyzer.analyze_data_flow(&ast_node);
    for vulnerability in taint_vulnerabilities {
        security_items.push(convert_vulnerability_to_debt_item(vulnerability, path));
    }
    
    // Integrate external tool findings
    let mut tool_manager = ToolIntegrationManager::new();
    if let Ok(tool_findings) = tool_manager.run_external_tools(path, Language::Rust).await {
        for finding in tool_findings {
            security_items.push(convert_vulnerability_to_debt_item(finding, path));
        }
    }
    
    security_items
}

fn convert_vulnerability_to_debt_item(
    vulnerability: SecurityVulnerability, 
    path: &Path
) -> DebtItem {
    let (priority, message) = match vulnerability {
        SecurityVulnerability::HardcodedSecret { secret_type, confidence, entropy, .. } => {
            (Priority::Critical, format!("Hardcoded {:?} detected (confidence: {:.0}%, entropy: {:.2})", 
                secret_type, confidence * 100.0, entropy))
        }
        SecurityVulnerability::SqlInjection { injection_type, taint_source, severity, .. } => {
            let priority = match severity {
                Severity::High => Priority::Critical,
                Severity::Medium => Priority::High,
                Severity::Low => Priority::Medium,
                _ => Priority::Low,
            };
            (priority, format!("SQL injection risk via {:?} from {:?}", injection_type, taint_source))
        }
        SecurityVulnerability::InputValidationGap { input_source, sink_operation, taint_path, .. } => {
            (Priority::High, format!("Unvalidated input from {:?} flows to {:?} ({} steps)", 
                input_source, sink_operation, taint_path.len()))
        }
        SecurityVulnerability::ExternalToolFinding { tool, normalized_severity, description, .. } => {
            let priority = match normalized_severity {
                Severity::High => Priority::High,
                Severity::Medium => Priority::Medium,
                Severity::Low => Priority::Low,
                _ => Priority::Low,
            };
            (priority, format!("[{}] {}", tool, description))
        }
    };
    
    DebtItem {
        id: format!("security-{}-{}", path.display(), line_number_from_vulnerability(&vulnerability)),
        debt_type: DebtType::Security, // New debt type to be added
        priority,
        file: path.to_path_buf(),
        line: line_number_from_vulnerability(&vulnerability),
        message,
        context: Some(vulnerability_context(&vulnerability)),
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_integration() {
        let findings = vec![
            ToolFinding {
                tool: "clippy".to_string(),
                severity: "warning".to_string(),
                rule_id: "clippy::unwrap_used".to_string(),
                file: PathBuf::from("src/main.rs"),
                line: 42,
                column: Some(8),
                message: "used unwrap on Result".to_string(),
                remediation: Some("Use ? operator or match".to_string()),
            }
        ];
        
        let manager = ToolIntegrationManager::new();
        let normalized = manager.normalize_findings(findings);
        
        assert_eq!(normalized.len(), 1);
        if let SecurityVulnerability::ExternalToolFinding { tool, normalized_severity, .. } = &normalized[0] {
            assert_eq!(tool, "clippy");
            assert_eq!(*normalized_severity, Severity::Low);
        } else {
            panic!("Expected external tool finding");
        }
    }
    
    #[test]
    fn test_hardcoded_secret_detection() {
        let source = r#"
            const API_KEY: &str = "sk-1234567890abcdef1234567890abcdef";
            const DATABASE_PASSWORD: &str = "super_secret_password123";
        "#;
        
        let ast = parse_rust_source(source);
        let detector = SecretDetector::new();
        let vulnerabilities = detector.detect_vulnerabilities(&ast);
        
        assert!(!vulnerabilities.is_empty());
        
        // Check for API key detection
        let api_key_found = vulnerabilities.iter().any(|v| {
            if let SecurityVulnerability::HardcodedSecret { secret_type, .. } = v {
                *secret_type == SecretType::ApiKey
            } else {
                false
            }
        });
        assert!(api_key_found);
    }
    
    #[test]
    fn test_sql_injection_detection() {
        let source = r#"
            fn build_query(user_input: &str) -> String {
                format!("SELECT * FROM users WHERE name = '{}'", user_input)
            }
            
            fn safe_query(user_input: &str) -> String {
                "SELECT * FROM users WHERE name = ?".to_string() // Parameterized query
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = SqlInjectionDetector::new();
        let vulnerabilities = detector.detect_vulnerabilities(&ast);
        
        // Should detect vulnerability in build_query but not safe_query
        assert_eq!(vulnerabilities.len(), 1);
        if let SecurityVulnerability::SqlInjection { injection_type, user_input_flow, .. } = &vulnerabilities[0] {
            assert_eq!(*injection_type, SqlInjectionType::FormatString);
            assert!(*user_input_flow);
        } else {
            panic!("Expected SQL injection vulnerability");
        }
    }
    
    #[test]
    fn test_taint_analysis() {
        let source = r#"
            fn process_input(user_input: String) {
                let query = format!("SELECT * FROM users WHERE name = '{}'", user_input);
                execute_sql(&query);
            }
            
            fn safe_process(user_input: String) {
                let sanitized = sanitize_input(&user_input);
                let query = format!("SELECT * FROM users WHERE name = '{}'", sanitized);
                execute_sql(&query);
            }
        "#;
        
        let ast = parse_rust_source(source);
        let mut analyzer = TaintAnalyzer::new();
        let vulnerabilities = analyzer.analyze_data_flow(&ast);
        
        // Should detect vulnerability in process_input but not safe_process
        assert_eq!(vulnerabilities.len(), 1);
        
        if let SecurityVulnerability::InputValidationGap { sink_operation, .. } = &vulnerabilities[0] {
            assert_eq!(*sink_operation, SinkOperation::SqlQuery);
        } else {
            panic!("Expected input validation gap");
        }
    }
}
```

### Integration Tests

```rust
// tests/security_integration.rs
use std::process::Command;

#[test]
fn test_security_analysis_end_to_end() {
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/security_vulnerable", "--security"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Verify security vulnerabilities are detected
    assert!(stdout.contains("Security"));
    assert!(stdout.contains("unsafe"));
    assert!(stdout.contains("Hardcoded"));
}

#[test]
fn test_security_json_output() {
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/crypto_example", "--security", "--format", "json"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    // Verify security section in JSON output
    assert!(json["analysis"]["security"]["vulnerabilities"].is_array());
}
```

## Configuration

### Security Analysis Configuration

```toml
[security]
enabled = true
# Focus on gaps not covered by existing tools
detectors = ["secrets", "sql_injection", "taint_analysis"]
# External tools to integrate
external_tools = ["clippy", "bandit", "eslint-security"]

[security.secrets]
enabled = true
entropy_threshold = 4.5
# Common secret patterns
patterns = [
    { type = "api_key", pattern = "sk-[a-zA-Z0-9]{32,}" },
    { type = "aws_key", pattern = "AKIA[0-9A-Z]{16}" },
    { type = "github_token", pattern = "ghp_[a-zA-Z0-9]{36}" },
    { type = "jwt", pattern = "eyJ[a-zA-Z0-9_-]+\\.[a-zA-Z0-9_-]+\\.[a-zA-Z0-9_-]+" },
]
# Allowlist for false positives
allowlist = [
    "example_api_key",
    "test_token",
    "mock_secret",
]

[security.sql_injection]
enabled = true
sql_keywords = ["SELECT", "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER"]
# ORM-specific patterns
orm_patterns = {
    diesel = ["query_dsl", "sql_query"],
    sqlx = ["query", "query_as"],
    rusqlite = ["execute", "prepare"],
}

[security.taint_analysis]
enabled = true
# Sources of tainted data
taint_sources = [
    "std::env::args",
    "std::io::stdin",
    "reqwest::Request",
    "actix_web::HttpRequest",
    "rocket::Request",
]
# Dangerous sinks
taint_sinks = [
    "std::process::Command",
    "std::fs::File",
    "diesel::sql_query",
    "sqlx::query",
]
# Sanitization functions
sanitizers = [
    "html_escape",
    "sql_escape",
    "shell_escape",
    "validate_input",
]

[security.tool_integration]
# Normalize severity across tools
severity_mapping = {
    clippy = { error = "high", warning = "medium", note = "low" },
    bandit = { HIGH = "high", MEDIUM = "medium", LOW = "low" },
    eslint = { error = "high", warning = "medium", info = "low" },
}
# Cache external tool results
cache_duration = 3600  # seconds
# Run tools in parallel
parallel_execution = true

[security.custom_patterns]
# Organization-specific patterns
patterns_file = ".debtmap/security_patterns.yaml"
# Enable/disable custom patterns
enabled = false
```

## Expected Impact

After implementation:

1. **Unified Security Debt View**: Single source of truth for security issues across all languages and tools
2. **Gap Coverage**: Detection of security patterns missed by language-specific tools (secrets, SQL injection in Rust, cross-function taint tracking)
3. **Tool Integration**: Leverage existing security tools while providing unified scoring and reporting
4. **Reduced Tool Fatigue**: Teams run one command instead of multiple security tools separately
5. **Prioritized Remediation**: Security issues ranked alongside other technical debt for better resource allocation
6. **Incremental Adoption**: Can start with core detectors and gradually integrate more external tools

This focused approach maximizes value by addressing real gaps in existing security tooling while avoiding redundant implementation of well-solved problems.

## Dependencies and Integration

### Modified Components
- `src/core/mod.rs`: Add SecurityVulnerability and Security debt type
- `src/analyzers/`: Integrate security analysis into language analyzers
- Configuration system for security detector settings and tool integration
- CLI options for enabling/disabling security analysis and external tools

### New Dependencies
- `async-trait` for async tool integration
- `tokio` for async external tool execution (if not already present)
- `serde_json` for parsing tool outputs (already in use)
- `petgraph` for taint analysis graph algorithms
- `regex` crate for pattern matching (already in use)

### Implementation Phases
1. **Phase 1 (4-6 weeks)**: Core secret detection and SQL injection analysis
2. **Phase 2 (3-4 weeks)**: Tool integration framework with clippy, bandit, ESLint
3. **Phase 3 (6-8 weeks)**: Taint analysis and cross-function tracking

This phased approach allows for early value delivery while building toward comprehensive security debt analysis capabilities.