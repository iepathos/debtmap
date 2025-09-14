//! Pure formatting functions for markdown output
//!
//! Contains utility functions for formatting debt types, visibility levels,
//! and other markdown formatting helpers.

// Helper functions for formatting
pub fn format_debt_type(debt_type: &crate::priority::DebtType) -> &'static str {
    use crate::priority::DebtType;
    match debt_type {
        DebtType::TestingGap { .. } => "Testing Gap",
        DebtType::ComplexityHotspot { .. } => "Complexity",
        DebtType::DeadCode { .. } => "Dead Code",
        DebtType::Duplication { .. } => "Duplication",
        DebtType::Risk { .. } => "Risk",
        DebtType::TestComplexityHotspot { .. } => "Test Complexity",
        DebtType::TestTodo { .. } => "Test TODO",
        DebtType::TestDuplication { .. } => "Test Duplication",
        DebtType::ErrorSwallowing { .. } => "Error Swallowing",
        // Add wildcard for all new debt types
        _ => "Technical Debt",
    }
}

pub fn format_debt_issue(debt_type: &crate::priority::DebtType) -> String {
    use crate::priority::DebtType;
    match debt_type {
        DebtType::TestingGap {
            coverage,
            cyclomatic,
            ..
        } => {
            format!(
                "{:.0}% coverage, complexity {}",
                coverage * 100.0,
                cyclomatic
            )
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => {
            format!("Cyclomatic: {}, Cognitive: {}", cyclomatic, cognitive)
        }
        DebtType::DeadCode { visibility, .. } => {
            format!("Unused {:?} function", visibility)
        }
        DebtType::Duplication {
            instances,
            total_lines,
        } => {
            format!("{} instances, {} lines", instances, total_lines)
        }
        DebtType::Risk { risk_score, .. } => {
            format!("Risk score: {:.1}", risk_score)
        }
        DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive,
            ..
        } => {
            format!("Test complexity: {} / {}", cyclomatic, cognitive)
        }
        DebtType::TestTodo { priority, reason } => {
            let reason_str = reason.as_deref().unwrap_or("No reason provided");
            format!("{:?} priority: {}", priority, reason_str)
        }
        DebtType::TestDuplication {
            instances,
            similarity,
            ..
        } => {
            format!(
                "{} instances, {:.0}% similar",
                instances,
                similarity * 100.0
            )
        }
        DebtType::ErrorSwallowing { pattern, context } => match context {
            Some(ctx) => format!("{}: {}", pattern, ctx),
            None => pattern.to_string(),
        },
        // Add default formatting for all new debt types
        _ => "Technical debt pattern detected".to_string(),
    }
}

pub fn format_visibility(visibility: &crate::priority::FunctionVisibility) -> &'static str {
    use crate::priority::FunctionVisibility;
    match visibility {
        FunctionVisibility::Public => "public",
        FunctionVisibility::Private => "private",
        FunctionVisibility::Crate => "crate",
    }
}

pub fn get_dead_code_recommendation(
    visibility: &crate::priority::FunctionVisibility,
    complexity: u32,
) -> &'static str {
    use crate::priority::FunctionVisibility;
    match (visibility, complexity) {
        (FunctionVisibility::Private, c) if c < 5 => "Safe to remove",
        (FunctionVisibility::Private, _) => "Review and remove if unused",
        (FunctionVisibility::Crate, _) => "Check module usage",
        (FunctionVisibility::Public, _) => "Check external usage",
    }
}
