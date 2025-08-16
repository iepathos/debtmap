pub mod crypto_detector;
pub mod enhanced_secret_detector;
pub mod enhanced_sql_detector;
pub mod hardcoded_secret_detector;
pub mod input_validation_detector;
pub mod sql_injection_detector;
pub mod taint_analysis;
pub mod tool_integration;
pub mod types;
pub mod unsafe_detector;

use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;
use syn::File;

use self::enhanced_secret_detector::EnhancedSecretDetector;
use self::enhanced_sql_detector::EnhancedSqlInjectionDetector;
use self::taint_analysis::TaintAnalyzer;
use self::types::{SecurityDetector, SecurityVulnerability};

/// Analyzes security patterns in the given file
pub fn analyze_security_patterns(file: &File, path: &Path) -> Vec<DebtItem> {
    let mut debt_items = Vec::new();

    // Use existing detectors for backward compatibility
    debt_items.extend(unsafe_detector::detect_unsafe_blocks(file, path));
    debt_items.extend(hardcoded_secret_detector::detect_hardcoded_secrets(
        file, path,
    ));
    debt_items.extend(sql_injection_detector::detect_sql_injection(file, path));
    debt_items.extend(crypto_detector::detect_crypto_misuse(file, path));
    debt_items.extend(input_validation_detector::detect_validation_gaps(
        file, path,
    ));

    debt_items
}

/// Enhanced security analysis with new detectors
pub fn analyze_security_enhanced(file: &File, path: &Path) -> Vec<DebtItem> {
    let mut debt_items = Vec::new();

    // Run enhanced detectors
    let detectors: Vec<Box<dyn SecurityDetector>> = vec![
        Box::new(EnhancedSecretDetector::new()),
        Box::new(EnhancedSqlInjectionDetector::new()),
    ];

    for detector in detectors {
        let vulnerabilities = detector.detect_vulnerabilities(file, path);
        debt_items.extend(convert_vulnerabilities_to_debt_items(vulnerabilities));
    }

    // Run taint analysis
    let mut taint_analyzer = TaintAnalyzer::new();
    let taint_vulnerabilities = taint_analyzer.analyze_data_flow(file, path);
    debt_items.extend(convert_vulnerabilities_to_debt_items(taint_vulnerabilities));

    // Add existing detectors
    debt_items.extend(unsafe_detector::detect_unsafe_blocks(file, path));
    debt_items.extend(crypto_detector::detect_crypto_misuse(file, path));

    debt_items
}

/// Convert SecurityVulnerability to DebtItem
fn convert_vulnerabilities_to_debt_items(
    vulnerabilities: Vec<SecurityVulnerability>,
) -> Vec<DebtItem> {
    vulnerabilities
        .into_iter()
        .map(convert_vulnerability_to_debt_item)
        .collect()
}

fn convert_vulnerability_to_debt_item(vulnerability: SecurityVulnerability) -> DebtItem {
    use types::SecurityVulnerability::*;

    let (priority, message, context, line, file) = match vulnerability {
        HardcodedSecret {
            secret_type,
            confidence,
            entropy,
            line,
            file,
            ..
        } => (
            Priority::Critical,
            format!(
                "Hardcoded {:?} detected (confidence: {:.0}%, entropy: {:.2})",
                secret_type,
                confidence * 100.0,
                entropy
            ),
            Some("Move to environment variable or secure configuration".to_string()),
            line,
            file,
        ),
        SqlInjection {
            injection_type,
            taint_source,
            severity,
            line,
            file,
            ..
        } => (
            severity.to_priority(),
            format!(
                "SQL injection risk via {:?}{}",
                injection_type,
                taint_source
                    .map(|s| format!(" from {:?}", s))
                    .unwrap_or_default()
            ),
            Some("Use parameterized queries or prepared statements".to_string()),
            line,
            file,
        ),
        InputValidationGap {
            input_source,
            sink_operation,
            taint_path,
            severity,
            line,
            file,
        } => (
            severity.to_priority(),
            format!(
                "Unvalidated input from {:?} flows to {:?} ({} steps)",
                input_source,
                sink_operation,
                taint_path.len()
            ),
            Some("Add input validation before using external data".to_string()),
            line,
            file,
        ),
        ExternalToolFinding {
            tool,
            normalized_severity,
            description,
            remediation,
            line,
            file,
            ..
        } => (
            normalized_severity.to_priority(),
            format!("[{}] {}", tool, description),
            remediation,
            line,
            file,
        ),
        UnsafeUsage {
            description,
            severity,
            line,
            file,
        } => (
            severity.to_priority(),
            description,
            Some("Consider safe alternatives to unsafe code".to_string()),
            line,
            file,
        ),
        CryptoMisuse {
            issue_type,
            description,
            severity,
            line,
            file,
        } => (
            severity.to_priority(),
            format!("Cryptographic issue ({:?}): {}", issue_type, description),
            Some("Use secure cryptographic practices".to_string()),
            line,
            file,
        ),
    };

    DebtItem {
        id: format!("security-{}-{}", file.display(), line),
        debt_type: DebtType::Security,
        priority,
        file,
        line,
        column: None,
        message,
        context,
    }
}

/// Calculates priority for security issues
pub fn calculate_security_priority(message: &str) -> Priority {
    if message.contains("Critical")
        || message.contains("hardcoded")
        || message.contains("SQL injection")
    {
        Priority::Critical
    } else if message.contains("unsafe") || message.contains("weak") {
        Priority::High
    } else {
        Priority::Medium
    }
}
