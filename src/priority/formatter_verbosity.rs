use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

pub fn format_priority_item_with_verbosity(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    let severity = crate::priority::formatter::get_severity_label(item.unified_score.final_score);
    let severity_color =
        crate::priority::formatter::get_severity_color(item.unified_score.final_score);

    // Base score line - add score breakdown for verbosity >= 1
    if verbosity >= 1 {
        // Get scoring weights for display
        let weights = crate::config::get_scoring_weights();

        // Calculate main contributing factors
        let mut factors = vec![];

        if item.unified_score.coverage_factor > 3.0 {
            factors.push(format!("Coverage gap ({:.0}%)", weights.coverage * 100.0));
        }
        if item.unified_score.dependency_factor > 5.0 {
            factors.push(format!(
                "Critical path ({:.0}%)",
                weights.dependency * 100.0
            ));
        }
        if item.unified_score.complexity_factor > 5.0 {
            factors.push(format!("Complexity ({:.0}%)", weights.complexity * 100.0));
        }

        // Add Security and Performance specific factors
        match &item.debt_type {
            crate::priority::DebtType::BasicSecurity {
                severity,
                vulnerability_type,
                ..
            } => {
                factors.push(format!("Security vulnerability ({})", severity));
                if !vulnerability_type.is_empty() && vulnerability_type != "Security Issue" {
                    factors.push(format!("{} detected", vulnerability_type));
                }
            }
            crate::priority::DebtType::HardcodedSecrets {
                severity,
                secret_type,
            } => {
                factors.push(format!("Security vulnerability ({})", severity));
                factors.push(format!("Hardcoded {} detected", secret_type));
            }
            crate::priority::DebtType::SqlInjectionRisk { risk_level, .. } => {
                factors.push(format!("Security vulnerability ({})", risk_level));
                factors.push("SQL injection risk detected".to_string());
            }
            crate::priority::DebtType::UnsafeCode { safety_concern, .. } => {
                factors.push("Security vulnerability (High)".to_string());
                factors.push(format!("Unsafe code: {}", safety_concern));
            }
            crate::priority::DebtType::WeakCryptography { algorithm, .. } => {
                factors.push("Security vulnerability (High)".to_string());
                factors.push(format!("Weak crypto: {}", algorithm));
            }
            crate::priority::DebtType::NestedLoops { depth, .. } => {
                factors.push("Performance impact (High)".to_string());
                factors.push(format!("{} level nested loops", depth));
            }
            crate::priority::DebtType::BlockingIO { operation, .. } => {
                factors.push("Performance impact (High)".to_string());
                factors.push(format!("Blocking {}", operation));
            }
            crate::priority::DebtType::AllocationInefficiency { pattern, .. } => {
                factors.push("Performance impact (Medium)".to_string());
                factors.push(format!("Allocation: {}", pattern));
            }
            _ => {} // No additional factors for other debt types
        }

        writeln!(
            output,
            "#{} {} [{}]",
            rank.to_string().bright_cyan().bold(),
            format!("SCORE: {:.1}", item.unified_score.final_score).bright_white(),
            severity.color(severity_color).bold()
        )
        .unwrap();

        if !factors.is_empty() {
            writeln!(output, "   ↳ Main factors: {}", factors.join(", ").dimmed()).unwrap();
        }
    } else {
        writeln!(
            output,
            "#{} {} [{}]",
            rank.to_string().bright_cyan().bold(),
            format!("SCORE: {:.1}", item.unified_score.final_score).bright_white(),
            severity.color(severity_color).bold()
        )
        .unwrap();
    }

    // Show detailed calculation for verbosity >= 2
    if verbosity >= 2 {
        let weights = crate::config::get_scoring_weights();
        writeln!(output, "├─ SCORE CALCULATION:").unwrap();
        writeln!(output, "│  ├─ Base Components (Weighted):").unwrap();

        // Show complexity with entropy adjustment if present
        if let Some(ref entropy) = item.entropy_details {
            writeln!(
                output,
                "│  │  ├─ Complexity:  {:.1} × {:.0}% = {:.2} (entropy-adjusted from {})",
                item.unified_score.complexity_factor,
                weights.complexity * 100.0,
                item.unified_score.complexity_factor * weights.complexity,
                entropy.original_complexity
            )
            .unwrap();
        } else {
            writeln!(
                output,
                "│  │  ├─ Complexity:  {:.1} × {:.0}% = {:.2}",
                item.unified_score.complexity_factor,
                weights.complexity * 100.0,
                item.unified_score.complexity_factor * weights.complexity
            )
            .unwrap();
        }
        writeln!(
            output,
            "│  │  ├─ Coverage:    {:.1} × {:.0}% = {:.2}",
            item.unified_score.coverage_factor,
            weights.coverage * 100.0,
            item.unified_score.coverage_factor * weights.coverage
        )
        .unwrap();
        
        writeln!(
            output,
            "│  │  ├─ Dependency:  {:.1} × {:.0}% = {:.2}",
            item.unified_score.dependency_factor,
            weights.dependency * 100.0,
            item.unified_score.dependency_factor * weights.dependency
        )
        .unwrap();

        // Always show security factor for consistency
        writeln!(
            output,
            "│  │  ├─ Security:    {:.1} × {:.0}% = {:.2}",
            item.unified_score.security_factor,
            weights.security * 100.0,
            item.unified_score.security_factor * weights.security
        )
        .unwrap();
        
        // Show semantic and organization with 0% weight for transparency
        // These were removed per spec 58 but keeping in display for clarity
        if weights.semantic > 0.0 || weights.organization > 0.0 {
            if weights.semantic > 0.0 {
                writeln!(
                    output,
                    "│  │  ├─ Semantic:    0.0 × {:.0}% = 0.00 (role multipliers used instead)",
                    weights.semantic * 100.0
                )
                .unwrap();
            }
            if weights.organization > 0.0 {
                writeln!(
                    output,
                    "│  │  ├─ Organization: 0.0 × {:.0}% = 0.00 (included in complexity)",
                    weights.organization * 100.0
                )
                .unwrap();
            }
        }

        // Calculate base score with actual weights from config
        // Note: semantic and organization are in config but not used in calculation (always 0)
        let base_score = item.unified_score.complexity_factor * weights.complexity
            + item.unified_score.coverage_factor * weights.coverage
            + item.unified_score.dependency_factor * weights.dependency
            + item.unified_score.security_factor * weights.security;

        writeln!(output, "│  ├─ Base Score: {:.2}", base_score).unwrap();

        // Show entropy impact if present
        if let Some(ref entropy) = item.entropy_details {
            writeln!(
                output,
                "│  ├─ Entropy Impact: {:.0}% dampening (entropy: {:.2}, repetition: {:.0}%)",
                (1.0 - entropy.dampening_factor) * 100.0,
                entropy.entropy_score,
                entropy.pattern_repetition * 100.0
            )
            .unwrap();
        }

        writeln!(
            output,
            "│  ├─ Role Adjustment: ×{:.2}",
            item.unified_score.role_multiplier
        )
        .unwrap();
        writeln!(
            output,
            "│  └─ Final Score: {:.2}",
            item.unified_score.final_score
        )
        .unwrap();
    }

    // Rest of the item formatting remains the same
    writeln!(
        output,
        "├─ {}: {}:{} {}()",
        crate::priority::formatter::format_debt_type(&item.debt_type).bright_yellow(),
        item.location.file.display(),
        item.location.line,
        item.location.function.bright_green()
    )
    .unwrap();

    writeln!(
        output,
        "├─ ACTION: {}",
        item.recommendation.primary_action.bright_white()
    )
    .unwrap();

    writeln!(
        output,
        "├─ IMPACT: {}",
        crate::priority::formatter::format_impact(&item.expected_impact).bright_cyan()
    )
    .unwrap();

    // Add complexity details
    let (cyclomatic, cognitive, branch_count, nesting, _length) =
        crate::priority::formatter::extract_complexity_info(item);
    if cyclomatic > 0 || cognitive > 0 {
        // Include entropy adjustment info if present
        if let Some(ref entropy) = item.entropy_details {
            writeln!(
                output,
                "├─ COMPLEXITY: cyclomatic={} (adj:{}), branches={}, cognitive={}, nesting={}, entropy={:.2}",
                cyclomatic.to_string().dimmed(),
                entropy.adjusted_complexity.to_string().dimmed(),
                branch_count.to_string().dimmed(),
                cognitive.to_string().dimmed(),
                nesting.to_string().dimmed(),
                entropy.entropy_score
            )
            .unwrap();
        } else {
            writeln!(
                output,
                "├─ COMPLEXITY: cyclomatic={}, branches={}, cognitive={}, nesting={}",
                cyclomatic.to_string().dimmed(),
                branch_count.to_string().dimmed(),
                cognitive.to_string().dimmed(),
                nesting.to_string().dimmed()
            )
            .unwrap();
        }
    }

    // Add dependency information
    let (upstream, downstream) = crate::priority::formatter::extract_dependency_info(item);
    if upstream > 0 || downstream > 0 {
        writeln!(
            output,
            "├─ DEPENDENCIES: {} upstream, {} downstream",
            upstream.to_string().dimmed(),
            downstream.to_string().dimmed()
        )
        .unwrap();

        // Add upstream callers if present
        if !item.upstream_callers.is_empty() {
            let callers_display = if item.upstream_callers.len() <= 3 {
                item.upstream_callers.join(", ")
            } else {
                format!(
                    "{}, ... ({} more)",
                    item.upstream_callers[..3].join(", "),
                    item.upstream_callers.len() - 3
                )
            };
            writeln!(output, "│  ├─ CALLERS: {}", callers_display.bright_blue()).unwrap();
        }

        // Add downstream callees if present
        if !item.downstream_callees.is_empty() {
            let callees_display = if item.downstream_callees.len() <= 3 {
                item.downstream_callees.join(", ")
            } else {
                format!(
                    "{}, ... ({} more)",
                    item.downstream_callees[..3].join(", "),
                    item.downstream_callees.len() - 3
                )
            };
            writeln!(output, "│  └─ CALLS: {}", callees_display.bright_magenta()).unwrap();
        }
    }

    // Add rationale
    writeln!(output, "└─ WHY: {}", item.recommendation.rationale.dimmed()).unwrap();
}
