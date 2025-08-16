pub mod crypto_detector;
pub mod hardcoded_secret_detector;
pub mod input_validation_detector;
pub mod sql_injection_detector;
pub mod unsafe_detector;

use crate::core::{DebtItem, Priority};
use std::path::Path;
use syn::File;

/// Analyzes security patterns in the given file
pub fn analyze_security_patterns(file: &File, path: &Path) -> Vec<DebtItem> {
    let mut debt_items = Vec::new();

    // Analyze unsafe blocks
    debt_items.extend(unsafe_detector::detect_unsafe_blocks(file, path));

    // Detect hardcoded secrets
    debt_items.extend(hardcoded_secret_detector::detect_hardcoded_secrets(
        file, path,
    ));

    // Detect SQL injection vulnerabilities
    debt_items.extend(sql_injection_detector::detect_sql_injection(file, path));

    // Detect cryptographic misuse
    debt_items.extend(crypto_detector::detect_crypto_misuse(file, path));

    // Detect input validation gaps
    debt_items.extend(input_validation_detector::detect_validation_gaps(
        file, path,
    ));

    debt_items
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
