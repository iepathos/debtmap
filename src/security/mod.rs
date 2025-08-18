pub mod crypto_detector;
pub mod hardcoded_secret_detector;
pub mod input_validation_detector;
pub mod sql_injection_detector;
pub mod tool_integration;
pub mod types;
pub mod unsafe_detector;

use crate::core::{DebtItem, Priority};
use std::path::Path;
use syn::File;

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

    // Use data flow-based validation detector
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
