//! Context section formatting with pure classification functions.
//!
//! Handles context dampening (spec 191) and file context (spec 181) display.
//! Follows Stillwater philosophy: pure functions for classification,
//! section formatters compose these for output.

use crate::analysis::FileContext;
use crate::context::FileType;
use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

// ============================================================================
// Pure Classification Functions (Stillwater "still" core)
// ============================================================================

/// Pure function to get context description from file type
pub fn get_context_description(file_type: FileType) -> &'static str {
    match file_type {
        FileType::Example => "Example/demonstration code (pedagogical patterns accepted)",
        FileType::Test => "Test code (test helper complexity accepted)",
        FileType::Benchmark => "Benchmark code (performance test patterns accepted)",
        FileType::BuildScript => "Build script (build-time complexity accepted)",
        FileType::Documentation => "Documentation code (code example patterns accepted)",
        FileType::Production | FileType::Configuration => "Production code",
    }
}

/// Pure function to calculate dampening percentage from multiplier
pub fn calculate_dampening_percentage(multiplier: f64) -> i32 {
    ((1.0 - multiplier) * 100.0) as i32
}

/// Pure function to determine if context should be shown
pub fn should_show_context_dampening(multiplier: Option<f64>) -> bool {
    multiplier.is_some_and(|m| m < 1.0)
}

/// Pure function to get file context explanation
pub fn get_file_context_explanation(factor: f64) -> &'static str {
    if factor >= 1.0 {
        "no score adjustment"
    } else if factor >= 0.6 {
        "40% score reduction"
    } else if factor >= 0.2 {
        "80% score reduction"
    } else {
        "90% score reduction"
    }
}

/// Pure function to check if file context is non-production
pub fn is_non_production_context(context: &FileContext) -> bool {
    !matches!(context, FileContext::Production)
}

// ============================================================================
// Section Formatters (Stillwater "water" shell - I/O at boundaries)
// ============================================================================

/// Format context dampening section (spec 191)
pub fn format_context_dampening_section(output: &mut String, item: &UnifiedDebtItem) {
    if let (Some(multiplier), Some(file_type)) = (item.context_multiplier, item.context_type) {
        if should_show_context_dampening(Some(multiplier)) {
            let description = get_context_description(file_type);
            let dampening_pct = calculate_dampening_percentage(multiplier);

            writeln!(
                output,
                "├─ {} {} ({}% dampening applied)",
                "CONTEXT:".bright_blue(),
                description.bright_cyan(),
                dampening_pct
            )
            .unwrap();
        }
    }
}

/// Format file context section for default verbosity (spec 181)
pub fn format_file_context_section(output: &mut String, item: &UnifiedDebtItem, verbosity: u8) {
    // Only show non-production contexts in default mode
    if verbosity != 0 {
        return;
    }

    if let Some(ref context) = item.file_context {
        use crate::priority::scoring::file_context_scoring::{
            context_label, context_reduction_factor,
        };

        if is_non_production_context(context) {
            let factor = context_reduction_factor(context);
            let reduction_pct = ((1.0 - factor) * 100.0) as u32;

            writeln!(
                output,
                "├─ {} {} ({}% score reduction)",
                "FILE CONTEXT:".bright_blue(),
                context_label(context).bright_magenta(),
                reduction_pct
            )
            .unwrap();
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_context_description() {
        assert!(get_context_description(FileType::Example).contains("Example"));
        assert!(get_context_description(FileType::Test).contains("Test"));
        assert!(get_context_description(FileType::Benchmark).contains("Benchmark"));
        assert!(get_context_description(FileType::BuildScript).contains("Build"));
        assert!(get_context_description(FileType::Documentation).contains("Documentation"));
        assert!(get_context_description(FileType::Production).contains("Production"));
        assert!(get_context_description(FileType::Configuration).contains("Production"));
    }

    #[test]
    fn test_calculate_dampening_percentage() {
        assert_eq!(calculate_dampening_percentage(1.0), 0);
        // 0.8 can have floating-point precision issues, so test nearby values
        let pct_80 = calculate_dampening_percentage(0.8);
        assert!(
            pct_80 == 19 || pct_80 == 20,
            "Expected 19 or 20, got {}",
            pct_80
        );
        assert_eq!(calculate_dampening_percentage(0.5), 50);
        assert_eq!(calculate_dampening_percentage(0.0), 100);
    }

    #[test]
    fn test_should_show_context_dampening() {
        assert!(!should_show_context_dampening(None));
        assert!(!should_show_context_dampening(Some(1.0)));
        assert!(!should_show_context_dampening(Some(1.5)));
        assert!(should_show_context_dampening(Some(0.99)));
        assert!(should_show_context_dampening(Some(0.5)));
        assert!(should_show_context_dampening(Some(0.0)));
    }

    #[test]
    fn test_get_file_context_explanation() {
        assert_eq!(get_file_context_explanation(1.0), "no score adjustment");
        assert_eq!(get_file_context_explanation(1.5), "no score adjustment");
        assert_eq!(get_file_context_explanation(0.6), "40% score reduction");
        assert_eq!(get_file_context_explanation(0.7), "40% score reduction");
        assert_eq!(get_file_context_explanation(0.2), "80% score reduction");
        assert_eq!(get_file_context_explanation(0.3), "80% score reduction");
        assert_eq!(get_file_context_explanation(0.1), "90% score reduction");
        assert_eq!(get_file_context_explanation(0.0), "90% score reduction");
    }

    #[test]
    fn test_is_non_production_context() {
        assert!(!is_non_production_context(&FileContext::Production));
        assert!(is_non_production_context(&FileContext::Test {
            confidence: 1.0,
            test_framework: None,
            test_count: 1,
        }));
        assert!(is_non_production_context(&FileContext::Generated {
            generator: "test".to_string(),
        }));
        assert!(is_non_production_context(&FileContext::Configuration));
        assert!(is_non_production_context(&FileContext::Documentation));
    }
}
