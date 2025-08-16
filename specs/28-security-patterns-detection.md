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

Security vulnerabilities often arise from common anti-patterns that can be detected through static analysis. The current debtmap system focuses on complexity and structural debt but lacks specific security-focused detection capabilities. Many security issues can be identified by examining code patterns:

- **Unsafe Block Usage** - Rust's unsafe blocks bypass memory safety guarantees
- **Hardcoded Secrets** - Credentials and API keys embedded in source code
- **SQL Injection Risks** - String concatenation patterns in SQL construction
- **Cryptographic Misuse** - Weak algorithms or improper usage patterns
- **Input Validation Gaps** - Missing validation on external inputs

These patterns represent high-priority technical debt as they directly impact application security and should be flagged for immediate attention.

## Objective

Implement security-focused pattern detection that identifies common security anti-patterns and vulnerabilities by:

1. **Unsafe Block Detection**: Flag all unsafe blocks for manual security review
2. **Secret Detection**: Identify hardcoded credentials, API keys, and sensitive data
3. **SQL Injection Analysis**: Detect dangerous string concatenation in SQL contexts
4. **Cryptographic Pattern Analysis**: Find weak or deprecated cryptographic usage
5. **Input Validation Analysis**: Identify missing validation on external inputs

## Requirements

### Functional Requirements

1. **Unsafe Block Detection**
   - Detect all `unsafe` blocks in Rust code
   - Report location, size, and complexity of unsafe operations
   - Classify unsafe operations by risk level (memory access, FFI, transmute, etc.)
   - Track unsafe propagation through function call chains

2. **Hardcoded Secret Detection**
   - Pattern matching for common secret formats (API keys, passwords, tokens)
   - Variable name analysis for secret-like identifiers
   - String literal analysis for credential patterns
   - Base64/hex encoded secret detection
   - Configurable patterns for organization-specific secrets

3. **SQL Injection Risk Detection**
   - String concatenation patterns with SQL keywords
   - Format string usage in SQL contexts
   - Dynamic query construction without parameterization
   - User input flow analysis to SQL execution points

4. **Cryptographic Misuse Detection**
   - Deprecated cryptographic algorithms (MD5, SHA1, DES, etc.)
   - Weak key sizes and random number generation
   - Improper IV/salt usage patterns
   - Hardcoded cryptographic keys and initialization vectors

5. **Input Validation Gap Analysis**
   - External input sources (HTTP requests, file I/O, CLI arguments)
   - Validation bypass patterns
   - Unsafe deserialization patterns
   - Path traversal vulnerability patterns

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

- [ ] **Unsafe Block Detection**: All unsafe blocks flagged with detailed risk assessment
- [ ] **Secret Detection**: Common secret patterns identified with 90%+ accuracy
- [ ] **SQL Injection Detection**: Dynamic SQL construction patterns flagged
- [ ] **Crypto Misuse Detection**: Deprecated algorithms and weak patterns identified
- [ ] **Input Validation**: External input flows without validation detected
- [ ] **Risk Prioritization**: Security issues receive appropriate priority weighting
- [ ] **False Positive Management**: <10% false positive rate on real codebases
- [ ] **Performance**: Security analysis completes within 15% of baseline time

## Technical Details

### Implementation Approach

#### 1. Security Pattern Framework (`src/security/`)

```rust
/// Security vulnerability detection framework
pub mod security {
    use crate::core::ast::AstNode;
    use crate::core::{DebtItem, Priority};
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum SecurityVulnerability {
        UnsafeBlock {
            operation_type: UnsafeOperation,
            complexity: u32,
            risk_level: RiskLevel,
        },
        HardcodedSecret {
            secret_type: SecretType,
            confidence: f64,
            value_preview: String,
        },
        SqlInjection {
            injection_type: SqlInjectionType,
            user_input_flow: bool,
            severity: Severity,
        },
        CryptographicMisuse {
            algorithm: String,
            issue_type: CryptoIssue,
            recommendation: String,
        },
        InputValidationGap {
            input_source: InputSource,
            validation_missing: Vec<ValidationType>,
            exploitability: Exploitability,
        },
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum UnsafeOperation {
        MemoryAccess,
        ForeignFunctionInterface,
        Transmute,
        StaticMut,
        RawPointer,
        InlineAssembly,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum SecretType {
        ApiKey,
        Password,
        PrivateKey,
        DatabaseCredential,
        AuthToken,
        CryptoKey,
        Certificate,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum SqlInjectionType {
        StringConcatenation,
        FormatString,
        DynamicQuery,
        StoredProcedure,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum CryptoIssue {
        WeakAlgorithm,
        WeakKeySize,
        HardcodedKey,
        WeakRandom,
        ImproperIV,
        NoSalt,
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

#### 2. Unsafe Block Detector (`src/security/unsafe_detector.rs`)

```rust
pub struct UnsafeDetector {
    risk_weights: HashMap<UnsafeOperation, u32>,
}

impl SecurityDetector for UnsafeDetector {
    fn detect_vulnerabilities(&self, ast: &AstNode) -> Vec<SecurityVulnerability> {
        let mut vulnerabilities = Vec::new();
        let unsafe_blocks = self.find_unsafe_blocks(ast);
        
        for unsafe_block in unsafe_blocks {
            let operations = self.analyze_unsafe_operations(&unsafe_block);
            let complexity = self.calculate_unsafe_complexity(&operations);
            let risk_level = self.assess_risk_level(&operations, complexity);
            
            for operation in operations {
                vulnerabilities.push(SecurityVulnerability::UnsafeBlock {
                    operation_type: operation,
                    complexity,
                    risk_level,
                });
            }
        }
        
        vulnerabilities
    }
}

impl UnsafeDetector {
    fn find_unsafe_blocks(&self, ast: &AstNode) -> Vec<UnsafeBlock> {
        // AST traversal to find all unsafe blocks
        let mut blocks = Vec::new();
        
        ast.traverse_depth_first(|node| {
            if let AstNode::UnsafeBlock(block) = node {
                blocks.push(block.clone());
            }
        });
        
        blocks
    }
    
    fn analyze_unsafe_operations(&self, block: &UnsafeBlock) -> Vec<UnsafeOperation> {
        let mut operations = Vec::new();
        
        // Detect different types of unsafe operations
        if self.has_raw_pointer_dereference(block) {
            operations.push(UnsafeOperation::RawPointer);
        }
        
        if self.has_transmute(block) {
            operations.push(UnsafeOperation::Transmute);
        }
        
        if self.has_ffi_calls(block) {
            operations.push(UnsafeOperation::ForeignFunctionInterface);
        }
        
        if self.has_static_mut_access(block) {
            operations.push(UnsafeOperation::StaticMut);
        }
        
        if self.has_inline_assembly(block) {
            operations.push(UnsafeOperation::InlineAssembly);
        }
        
        operations
    }
    
    fn assess_risk_level(&self, operations: &[UnsafeOperation], complexity: u32) -> RiskLevel {
        let base_risk: u32 = operations.iter()
            .map(|op| self.risk_weights.get(op).copied().unwrap_or(1))
            .sum();
            
        let complexity_factor = complexity / 10;
        let total_risk = base_risk + complexity_factor;
        
        match total_risk {
            0..=2 => RiskLevel::Low,
            3..=5 => RiskLevel::Medium,
            6..=10 => RiskLevel::High,
            _ => RiskLevel::Critical,
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

#### 5. Cryptographic Misuse Detector (`src/security/crypto_detector.rs`)

```rust
pub struct CryptoDetector {
    weak_algorithms: HashMap<String, CryptoIssue>,
    minimum_key_sizes: HashMap<String, u32>,
}

impl SecurityDetector for CryptoDetector {
    fn detect_vulnerabilities(&self, ast: &AstNode) -> Vec<SecurityVulnerability> {
        let mut vulnerabilities = Vec::new();
        
        // Check for weak cryptographic algorithms
        let crypto_calls = self.find_crypto_function_calls(ast);
        for call in crypto_calls {
            if let Some(vulnerability) = self.analyze_crypto_call(&call) {
                vulnerabilities.push(vulnerability);
            }
        }
        
        // Check for hardcoded cryptographic keys
        let key_assignments = self.find_key_assignments(ast);
        for assignment in key_assignments {
            if let Some(vulnerability) = self.analyze_key_assignment(&assignment) {
                vulnerabilities.push(vulnerability);
            }
        }
        
        vulnerabilities
    }
}

impl CryptoDetector {
    fn analyze_crypto_call(&self, call: &FunctionCall) -> Option<SecurityVulnerability> {
        // Check algorithm name against weak algorithm list
        if let Some(issue) = self.weak_algorithms.get(&call.function_name) {
            let recommendation = self.get_algorithm_recommendation(&call.function_name);
            
            return Some(SecurityVulnerability::CryptographicMisuse {
                algorithm: call.function_name.clone(),
                issue_type: *issue,
                recommendation,
            });
        }
        
        // Check key size for algorithms that support variable key sizes
        if let Some(key_size) = self.extract_key_size(call) {
            if let Some(&minimum) = self.minimum_key_sizes.get(&call.function_name) {
                if key_size < minimum {
                    return Some(SecurityVulnerability::CryptographicMisuse {
                        algorithm: call.function_name.clone(),
                        issue_type: CryptoIssue::WeakKeySize,
                        recommendation: format!(
                            "Use key size >= {} bits for {}",
                            minimum, call.function_name
                        ),
                    });
                }
            }
        }
        
        None
    }
    
    fn get_algorithm_recommendation(&self, algorithm: &str) -> String {
        match algorithm.to_lowercase().as_str() {
            "md5" => "Use SHA-256 or SHA-3 instead of MD5".to_string(),
            "sha1" => "Use SHA-256 or SHA-3 instead of SHA-1".to_string(),
            "des" => "Use AES instead of DES".to_string(),
            "3des" => "Use AES instead of 3DES".to_string(),
            "rc4" => "Use AES or ChaCha20 instead of RC4".to_string(),
            _ => format!("Consider using a stronger alternative to {}", algorithm),
        }
    }
}
```

#### 6. Integration with Existing System (`src/analyzers/rust.rs`)

```rust
use crate::security::{
    SecurityDetector, UnsafeDetector, SecretDetector, 
    SqlInjectionDetector, CryptoDetector, InputValidationDetector
};

// Add to analyze_rust_file function
fn analyze_rust_file(ast: &RustAst, threshold: u32) -> FileMetrics {
    // ... existing analysis ...
    
    // Security analysis
    let security_items = analyze_security_patterns(&ast.file, &ast.path);
    debt_items.extend(security_items);
    
    // ... rest of analysis ...
}

fn analyze_security_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    let detectors: Vec<Box<dyn SecurityDetector>> = vec![
        Box::new(UnsafeDetector::new()),
        Box::new(SecretDetector::new()),
        Box::new(SqlInjectionDetector::new()),
        Box::new(CryptoDetector::new()),
        Box::new(InputValidationDetector::new()),
    ];
    
    let ast_node = convert_syn_to_ast_node(file);
    let mut security_items = Vec::new();
    
    for detector in detectors {
        let vulnerabilities = detector.detect_vulnerabilities(&ast_node);
        
        for vulnerability in vulnerabilities {
            let debt_item = convert_vulnerability_to_debt_item(vulnerability, path);
            security_items.push(debt_item);
        }
    }
    
    security_items
}

fn convert_vulnerability_to_debt_item(
    vulnerability: SecurityVulnerability, 
    path: &Path
) -> DebtItem {
    let (priority, message) = match vulnerability {
        SecurityVulnerability::UnsafeBlock { risk_level, operation_type, .. } => {
            let priority = match risk_level {
                RiskLevel::Critical => Priority::Critical,
                RiskLevel::High => Priority::High,
                RiskLevel::Medium => Priority::Medium,
                RiskLevel::Low => Priority::Low,
            };
            (priority, format!("Unsafe block with {:?} operation", operation_type))
        }
        SecurityVulnerability::HardcodedSecret { secret_type, confidence, .. } => {
            (Priority::Critical, format!("Hardcoded {:?} detected ({}% confidence)", 
                secret_type, (confidence * 100.0) as u32))
        }
        SecurityVulnerability::SqlInjection { injection_type, user_input_flow, .. } => {
            let priority = if user_input_flow { Priority::Critical } else { Priority::High };
            (priority, format!("Potential SQL injection via {:?}", injection_type))
        }
        SecurityVulnerability::CryptographicMisuse { algorithm, issue_type, .. } => {
            (Priority::High, format!("Cryptographic misuse: {:?} in {}", issue_type, algorithm))
        }
        SecurityVulnerability::InputValidationGap { input_source, validation_missing, .. } => {
            (Priority::High, format!("Missing validation for {:?} input: {:?}", 
                input_source, validation_missing))
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
    fn test_unsafe_block_detection() {
        let source = r#"
            fn dangerous_operation() {
                unsafe {
                    let ptr = std::ptr::null_mut::<i32>();
                    *ptr = 42; // Raw pointer dereference
                }
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = UnsafeDetector::new();
        let vulnerabilities = detector.detect_vulnerabilities(&ast);
        
        assert_eq!(vulnerabilities.len(), 1);
        if let SecurityVulnerability::UnsafeBlock { operation_type, risk_level, .. } = &vulnerabilities[0] {
            assert_eq!(*operation_type, UnsafeOperation::RawPointer);
            assert_eq!(*risk_level, RiskLevel::High);
        } else {
            panic!("Expected unsafe block vulnerability");
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
    fn test_crypto_misuse_detection() {
        let source = r#"
            use md5::Md5;
            use sha1::Sha1;
            
            fn weak_hash(data: &[u8]) -> Vec<u8> {
                use md5::Digest;
                let mut hasher = Md5::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = CryptoDetector::new();
        let vulnerabilities = detector.detect_vulnerabilities(&ast);
        
        assert!(!vulnerabilities.is_empty());
        
        let md5_found = vulnerabilities.iter().any(|v| {
            if let SecurityVulnerability::CryptographicMisuse { algorithm, issue_type, .. } = v {
                algorithm.contains("md5") && *issue_type == CryptoIssue::WeakAlgorithm
            } else {
                false
            }
        });
        assert!(md5_found);
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
detectors = ["unsafe", "secrets", "sql_injection", "crypto", "input_validation"]

[security.unsafe]
enabled = true
risk_weights = { raw_pointer = 3, transmute = 4, ffi = 2, static_mut = 3, inline_asm = 5 }

[security.secrets]
enabled = true
entropy_threshold = 4.5
patterns = [
    { type = "api_key", pattern = "sk-[a-zA-Z0-9]{32,}" },
    { type = "password", pattern = "password\\s*=\\s*['\"][^'\"]{8,}['\"]" },
]

[security.sql_injection]
enabled = true
sql_keywords = ["SELECT", "INSERT", "UPDATE", "DELETE", "DROP", "CREATE"]
track_user_input = true

[security.crypto]
enabled = true
weak_algorithms = ["md5", "sha1", "des", "3des", "rc4"]
minimum_key_sizes = { rsa = 2048, aes = 128, dsa = 2048 }

[security.input_validation]
enabled = true
input_sources = ["http_request", "file_input", "cli_args", "env_vars"]
validation_types = ["sanitization", "type_checking", "bounds_checking"]
```

## Expected Impact

After implementation:

1. **Enhanced Security Posture**: Automatic detection of common security vulnerabilities during development
2. **Risk Prioritization**: Security issues receive appropriate high priority in technical debt reports
3. **Developer Education**: Security patterns help educate developers about common pitfalls
4. **Compliance Support**: Automated security scanning helps meet security compliance requirements
5. **False Positive Management**: Configurable sensitivity allows tuning for specific organizational needs

This specification provides comprehensive security pattern detection capabilities that complement existing complexity and structural analysis, creating a more complete technical debt assessment system.

## Dependencies and Integration

### Modified Components
- `src/core/mod.rs`: Add SecurityVulnerability and Security debt type
- `src/analyzers/rust.rs`: Integrate security analysis into main analysis pipeline
- Configuration system for security detector settings
- CLI options for enabling/disabling security analysis

### New Dependencies
- `regex` crate for pattern matching (already in use)
- `entropy` crate for entropy calculations (optional)
- No major external dependencies required

This security-focused detection capability significantly enhances the technical debt assessment by identifying high-priority security vulnerabilities that require immediate attention.