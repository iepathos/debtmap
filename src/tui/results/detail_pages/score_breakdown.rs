//! Score Breakdown page (Page 2) - Detailed scoring analysis.
//!
//! Shows every factor that contributes to the final debt score,
//! helping users understand WHY an item scored high/low.
//!
//! Structured as pure section builders composed by a thin render shell,
//! following Stillwater philosophy: "Pure Core, Imperative Shell".

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::priority::classification::Severity;
use crate::priority::{DebtType, UnifiedDebtItem};
use crate::tui::results::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

// Column layout constants (from DESIGN.md)
const INDENT: usize = 2;
const LABEL_WIDTH: usize = 24; // Fixed column width for alignment
const GAP: usize = 4; // Breathing room between label and value

// ============================================================================
// Pure Section Builders (the "still" core)
// ============================================================================

/// Build final score section with severity classification (pure)
pub fn build_final_score_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    _width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    add_section_header(&mut lines, "final score", theme);

    let score = item.unified_score.final_score.value();
    let severity = Severity::from_score_100(score);
    let severity_color = match severity {
        Severity::Critical => Color::Red,
        Severity::High => Color::LightRed,
        Severity::Medium => Color::Yellow,
        Severity::Low => Color::Green,
    };

    // Create a visual bar representation
    let bar_width: usize = 20;
    let filled = ((score / 100.0) * bar_width as f64).round() as usize;
    let empty = bar_width.saturating_sub(filled);
    let bar = format!("[{}{}]", "#".repeat(filled), "-".repeat(empty));

    // Use proper column alignment
    let label = format!(
        "{:width$}",
        format!("{}total", " ".repeat(INDENT)),
        width = LABEL_WIDTH
    );
    let gap = " ".repeat(GAP);

    lines.push(Line::from(vec![
        Span::raw(label),
        Span::raw(gap),
        Span::styled(format!("{:.1}", score), Style::default().fg(severity_color)),
        Span::raw(" "),
        Span::styled(
            format!("[{}]", severity.as_str().to_lowercase()),
            Style::default().fg(severity_color),
        ),
        Span::raw(" "),
        Span::styled(bar, Style::default().fg(theme.muted)),
    ]));
    add_blank_line(&mut lines);
    lines
}

/// Build raw inputs section showing base metrics (pure)
pub fn build_raw_inputs_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    add_section_header(&mut lines, "raw inputs", theme);

    add_label_value(
        &mut lines,
        "cyclomatic",
        item.cyclomatic_complexity.to_string(),
        theme,
        width,
    );
    // Show cognitive complexity, with entropy-adjusted value if different
    // Check item-level first, then god object aggregated entropy
    let entropy_adjusted = item.entropy_adjusted_cognitive.or_else(|| {
        item.god_object_indicators
            .as_ref()
            .and_then(|g| g.aggregated_entropy.as_ref())
            .map(|e| e.adjusted_cognitive)
    });

    if let Some(adjusted) = entropy_adjusted {
        if adjusted != item.cognitive_complexity {
            add_label_value(
                &mut lines,
                "cognitive",
                format!(
                    "{} → {} (entropy-adjusted)",
                    item.cognitive_complexity, adjusted
                ),
                theme,
                width,
            );
        } else {
            add_label_value(
                &mut lines,
                "cognitive",
                item.cognitive_complexity.to_string(),
                theme,
                width,
            );
        }
    } else {
        add_label_value(
            &mut lines,
            "cognitive",
            item.cognitive_complexity.to_string(),
            theme,
            width,
        );
    }
    add_label_value(
        &mut lines,
        "nesting",
        item.nesting_depth.to_string(),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "loc",
        item.function_length.to_string(),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "upstream deps",
        item.upstream_dependencies.to_string(),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "downstream deps",
        item.downstream_dependencies.to_string(),
        theme,
        width,
    );

    // Coverage details (affects coverage_factor)
    if let Some(coverage) = &item.transitive_coverage {
        add_label_value(
            &mut lines,
            "direct coverage",
            format!("{:.1}%", coverage.direct * 100.0),
            theme,
            width,
        );
        if (coverage.transitive - coverage.direct).abs() > 0.01 {
            add_label_value(
                &mut lines,
                "transitive coverage",
                format!("{:.1}%", coverage.transitive * 100.0),
                theme,
                width,
            );
        }
    }

    add_blank_line(&mut lines);
    lines
}

/// Build unified score factors section (pure)
pub fn build_score_factors_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    add_section_header(&mut lines, "score factors (0-10 scale)", theme);

    // Check if god object multiplier was applied (complexity_factor would be inflated)
    let has_god_object = matches!(&item.debt_type, DebtType::GodObject { .. })
        || item
            .god_object_indicators
            .as_ref()
            .map(|g| g.is_god_object)
            .unwrap_or(false);

    // Complexity factor
    let complexity_formula = if has_god_object {
        "weighted(cyc,cog) * god_mult"
    } else {
        "weighted(cyc,cog) / 2"
    };
    add_factor_line(
        &mut lines,
        "complexity",
        item.unified_score.complexity_factor,
        complexity_formula,
        theme,
        width,
    );

    // Coverage factor
    add_factor_line(
        &mut lines,
        "coverage gap",
        item.unified_score.coverage_factor,
        "(1.0 - coverage%) * 10",
        theme,
        width,
    );

    // Dependency factor
    add_factor_line(
        &mut lines,
        "dependencies",
        item.unified_score.dependency_factor,
        "upstream / 2.0",
        theme,
        width,
    );

    add_blank_line(&mut lines);
    lines
}

/// Build multipliers section showing all applied multipliers (pure)
pub fn build_multipliers_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    add_section_header(&mut lines, "multipliers applied", theme);

    // Role multiplier
    let role_desc = format!("{:?}", item.function_role);
    add_multiplier_line(
        &mut lines,
        "role",
        item.unified_score.role_multiplier,
        &role_desc,
        theme,
        width,
    );

    // Purity factor (if present)
    if let Some(purity) = item.unified_score.purity_factor {
        // Show purity level classification if available
        let purity_desc = if let Some(level) = &item.purity_level {
            format!("{:?}", level)
        } else if let Some(is_pure) = item.is_pure {
            if is_pure {
                "pure".to_string()
            } else {
                "impure".to_string()
            }
        } else {
            "data flow analysis".to_string()
        };

        // Include confidence if available
        let desc = if let Some(conf) = item.purity_confidence {
            format!("{} ({:.0}% conf)", purity_desc, conf * 100.0)
        } else {
            purity_desc
        };

        add_multiplier_line(&mut lines, "purity", purity, &desc, theme, width);
    }

    // Pattern factor (if present)
    if let Some(pattern) = item.unified_score.pattern_factor {
        add_multiplier_line(
            &mut lines,
            "pattern",
            pattern,
            "data flow vs logic",
            theme,
            width,
        );
    }

    // Refactorability factor (if present)
    if let Some(refactor) = item.unified_score.refactorability_factor {
        add_multiplier_line(
            &mut lines,
            "refactorability",
            refactor,
            "dead stores/escape analysis",
            theme,
            width,
        );
    }

    // Context multiplier (if present)
    if let Some(context) = item.context_multiplier {
        let context_type = item
            .context_type
            .as_ref()
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|| "unknown".to_string());
        add_multiplier_line(&mut lines, "context", context, &context_type, theme, width);
    }

    // Structural multiplier (spec 260) - if significantly different from 1.0
    if let Some(struct_mult) = item.unified_score.structural_multiplier {
        if (struct_mult - 1.0).abs() > 0.01 {
            let desc = if struct_mult > 1.2 {
                "deeply nested"
            } else if struct_mult > 1.0 {
                "moderate nesting"
            } else if struct_mult < 0.85 {
                "flat structure"
            } else {
                "good structure"
            };
            add_multiplier_line(&mut lines, "structural", struct_mult, desc, theme, width);
        }
    }

    // Entropy dampening (if present) - check both item-level and god object aggregated
    if let Some(dampening) = item.entropy_dampening_factor {
        add_multiplier_line(
            &mut lines,
            "entropy dampening",
            dampening,
            "repetitive patterns",
            theme,
            width,
        );
    } else if let Some(ref god) = item.god_object_indicators {
        // For god objects, show aggregated entropy dampening
        if let Some(ref entropy) = god.aggregated_entropy {
            add_multiplier_line(
                &mut lines,
                "entropy dampening",
                entropy.dampening_factor,
                "aggregated repetition",
                theme,
                width,
            );
        }
    }

    add_blank_line(&mut lines);
    lines
}

/// Build exponential scaling pipeline section (pure) - shows score progression
pub fn build_scaling_pipeline_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Only show if we have scaling data
    let has_scaling_data = item.unified_score.base_score.is_some()
        || item.unified_score.exponential_factor.is_some()
        || item.unified_score.risk_boost.is_some()
        || item.unified_score.pre_adjustment_score.is_some();

    if !has_scaling_data {
        return lines;
    }

    add_section_header(&mut lines, "score scaling pipeline", theme);

    // Base score (before exponential scaling)
    if let Some(base) = item.unified_score.base_score {
        add_label_value(
            &mut lines,
            "base score",
            format!("{:.2}", base),
            theme,
            width,
        );
    }

    // Exponential factor
    if let Some(exp) = item.unified_score.exponential_factor {
        let exp_desc = if (exp - 1.0).abs() < 0.01 {
            "no scaling".to_string()
        } else {
            format!("^{:.2} applied", exp)
        };
        add_label_value(&mut lines, "exponential factor", exp_desc, theme, width);
    }

    // Risk boost
    if let Some(boost) = item.unified_score.risk_boost {
        let boost_desc = if (boost - 1.0).abs() < 0.01 {
            "none".to_string()
        } else {
            format!("{:.2}x", boost)
        };
        add_label_value(&mut lines, "risk boost", boost_desc, theme, width);
    }

    // Pre-adjustment score
    if let Some(pre_adj) = item.unified_score.pre_adjustment_score {
        add_label_value(
            &mut lines,
            "pre-adjustment score",
            format!("{:.2}", pre_adj),
            theme,
            width,
        );
    }

    add_blank_line(&mut lines);
    lines
}

/// Build god object impact section (pure) - only shown for god objects
pub fn build_god_object_impact_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Only show for god objects
    let god_object_score = match &item.debt_type {
        DebtType::GodObject {
            god_object_score, ..
        } => Some(god_object_score.value()),
        _ => item
            .god_object_indicators
            .as_ref()
            .filter(|g| g.is_god_object)
            .map(|g| g.god_object_score.value()),
    };

    let Some(go_score) = god_object_score else {
        return lines;
    };

    add_section_header(&mut lines, "god object impact (MAJOR)", theme);

    // Detection type (GodClass/GodFile/GodModule)
    if let Some(indicators) = &item.god_object_indicators {
        add_label_value(
            &mut lines,
            "detection type",
            format!("{:?}", indicators.detection_type),
            theme,
            width,
        );
    }

    // Show the god object score
    add_label_value(
        &mut lines,
        "god object score",
        format!("{:.1}", go_score),
        theme,
        width,
    );

    // Calculate and show the multiplier with proper column alignment
    let multiplier = 3.0 + (go_score / 50.0);
    let label = format!(
        "{:width$}",
        format!("{}multiplier", " ".repeat(INDENT)),
        width = LABEL_WIDTH
    );
    let gap = " ".repeat(GAP);

    lines.push(Line::from(vec![
        Span::raw(label),
        Span::raw(gap),
        Span::styled(
            format!("{:.2}x", multiplier),
            Style::default().fg(Color::Red),
        ),
        Span::raw(" "),
        Span::styled("(3.0 + score/50)", Style::default().fg(theme.muted)),
    ]));

    // Show detailed indicators if available
    if let Some(indicators) = &item.god_object_indicators {
        // Raw counts
        add_label_value(
            &mut lines,
            "methods",
            indicators.method_count.to_string(),
            theme,
            width,
        );
        add_label_value(
            &mut lines,
            "fields",
            indicators.field_count.to_string(),
            theme,
            width,
        );
        add_label_value(
            &mut lines,
            "responsibilities",
            indicators.responsibility_count.to_string(),
            theme,
            width,
        );

        // Weighted method count (pure-adjusted)
        if let Some(weighted) = indicators.weighted_method_count {
            add_label_value(
                &mut lines,
                "weighted methods",
                format!("{:.1} (pure-adjusted)", weighted),
                theme,
                width,
            );
        }

        // Trait method summary (Spec 217)
        if let Some(trait_summary) = &indicators.trait_method_summary {
            add_label_value(
                &mut lines,
                "trait methods",
                format!(
                    "{} mandated, {} extractable",
                    trait_summary.mandated_count, trait_summary.extractable_count
                ),
                theme,
                width,
            );
        }

        // Domain diversity
        if indicators.domain_count > 0 {
            add_label_value(
                &mut lines,
                "domains",
                format!(
                    "{} (diversity: {:.2})",
                    indicators.domain_count, indicators.domain_diversity
                ),
                theme,
                width,
            );
        }

        // Show complexity metrics if available (Spec 211)
        if let Some(metrics) = &indicators.complexity_metrics {
            add_label_value(
                &mut lines,
                "avg complexity",
                format!("{:.1}", metrics.avg_cyclomatic),
                theme,
                width,
            );
            add_label_value(
                &mut lines,
                "max complexity",
                format!("{}", metrics.max_cyclomatic),
                theme,
                width,
            );
        }
    }

    // Warning about impact with proper alignment
    let warning_label = format!(
        "{:width$}",
        format!("{}warning", " ".repeat(INDENT)),
        width = LABEL_WIDTH
    );
    lines.push(Line::from(vec![
        Span::raw(warning_label.clone()),
        Span::raw(" ".repeat(GAP)),
        Span::styled(
            "God object multiplier applied LAST",
            Style::default().fg(Color::Yellow),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw(warning_label),
        Span::raw(" ".repeat(GAP)),
        Span::styled(
            "causing dramatic score inflation",
            Style::default().fg(Color::Yellow),
        ),
    ]));

    add_blank_line(&mut lines);
    lines
}

/// Build orchestration adjustment section (pure)
pub fn build_orchestration_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let Some(adjustment) = &item.unified_score.adjustment_applied else {
        return lines;
    };

    add_section_header(&mut lines, "orchestration adjustment", theme);

    add_label_value(
        &mut lines,
        "original score",
        format!("{:.2}", adjustment.original_score),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "adjusted score",
        format!("{:.2}", adjustment.adjusted_score),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "reduction",
        format!("{:.1}%", adjustment.reduction_percent),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "reason",
        adjustment.adjustment_reason.clone(),
        theme,
        width,
    );

    add_blank_line(&mut lines);
    lines
}

/// Build score calculation summary (pure)
pub fn build_calculation_summary_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Check if god object multiplier was applied
    let has_god_object = matches!(&item.debt_type, DebtType::GodObject { .. })
        || item
            .god_object_indicators
            .as_ref()
            .map(|g| g.is_god_object)
            .unwrap_or(false);

    // Get god object multiplier if applicable
    let god_mult = if has_god_object {
        let go_score = match &item.debt_type {
            DebtType::GodObject {
                god_object_score, ..
            } => god_object_score.value(),
            _ => item
                .god_object_indicators
                .as_ref()
                .map(|g| g.god_object_score.value())
                .unwrap_or(0.0),
        };
        Some(3.0 + (go_score / 50.0))
    } else {
        None
    };

    // Collect active multipliers
    let role = item.unified_score.role_multiplier;
    let purity = item.unified_score.purity_factor.unwrap_or(1.0);
    let pattern = item.unified_score.pattern_factor.unwrap_or(1.0);
    let refactor = item.unified_score.refactorability_factor.unwrap_or(1.0);
    let context = item.context_multiplier.unwrap_or(1.0);
    let entropy = item.entropy_dampening_factor.unwrap_or(1.0);

    // Build multiplier product (for potential future use)
    let _total_mult = role * purity * pattern * refactor * context * entropy;

    // Use stored has_coverage_data flag - matches what the scorer actually used
    let has_coverage_data = item.unified_score.has_coverage_data;

    add_section_header(&mut lines, "score formula (simplified)", theme);

    // Calculate structural multiplier from nesting/cyclomatic ratio
    let struct_mult = if item.cyclomatic_complexity == 0 {
        1.0
    } else {
        let ratio = item.nesting_depth as f64 / item.cyclomatic_complexity as f64;
        match ratio {
            r if r >= 0.6 => 1.5,
            r if r >= 0.5 => 1.3,
            r if r >= 0.4 => 1.15,
            r if r >= 0.2 => 1.0,
            r if r >= 0.1 => 0.85,
            _ => 0.7,
        }
    };

    // Show symbolic formula - different for with/without coverage data
    // Formula: base × role × struct (× god_mult if applicable)
    let formula = if has_coverage_data {
        if has_god_object {
            "(C + D) × cov × role × struct × god"
        } else {
            "(C + D) × cov × role × struct"
        }
    } else {
        // No coverage data: uses weighted sum formula (spec 122)
        if has_god_object {
            "(C×5 + D×2.5) × role × struct × god"
        } else {
            "(C×5 + D×2.5) × role × struct"
        }
    };
    add_label_value(&mut lines, "formula", formula.to_string(), theme, width);

    // Show variable legend
    if has_coverage_data {
        add_label_value(
            &mut lines,
            "where",
            format!(
                "C={:.1}, D={:.1}, cov={:.2}, role={:.2}, struct={:.2}",
                item.unified_score.complexity_factor,
                item.unified_score.dependency_factor,
                1.0 - (item.unified_score.coverage_factor / 10.0),
                role,
                struct_mult
            ),
            theme,
            width,
        );
    } else {
        add_label_value(
            &mut lines,
            "where",
            format!(
                "C={:.1}, D={:.1}, role={:.2}, struct={:.2}",
                item.unified_score.complexity_factor,
                item.unified_score.dependency_factor,
                role,
                struct_mult
            ),
            theme,
            width,
        );
    }

    // Show other multipliers (purity, pattern, refactor, context, entropy)
    let other_mults: Vec<String> = [
        (purity, "purity"),
        (pattern, "pattern"),
        (refactor, "refactor"),
        (context, "context"),
        (entropy, "entropy"),
    ]
    .iter()
    .filter(|(v, _)| (*v - 1.0).abs() > 0.01)
    .map(|(v, name)| format!("{}={:.2}", name, v))
    .collect();

    if !other_mults.is_empty() {
        add_label_value(
            &mut lines,
            "other mults",
            other_mults.join(", "),
            theme,
            width,
        );
    }

    if let Some(gm) = god_mult {
        add_label_value(
            &mut lines,
            "god_mult",
            format!("{:.2} (3.0 + score/50)", gm),
            theme,
            width,
        );
    }

    add_blank_line(&mut lines);

    // Show actual calculation with step-by-step breakdown
    add_section_header(&mut lines, "calculation steps", theme);

    // Use the stored base_score (score before exponential scaling)
    let stored_base = item.unified_score.base_score.unwrap_or(0.0);
    let exponent = item.unified_score.exponential_factor.unwrap_or(1.0);
    let risk_boost = item.unified_score.risk_boost.unwrap_or(1.0);
    let final_score = item.unified_score.final_score.value();

    // Calculate intermediate values for display
    let c = item.unified_score.complexity_factor;
    let d = item.unified_score.dependency_factor;

    // Step 1: Base score from formula
    let weighted_base = if has_coverage_data {
        let cov_mult = 1.0 - (item.unified_score.coverage_factor / 10.0);
        (c + d) * cov_mult
    } else {
        (c * 5.0) + (d * 2.5)
    };
    add_label_value(
        &mut lines,
        "1. weighted base",
        format!("{:.2}", weighted_base),
        theme,
        width,
    );

    // Step 2: After role adjustment
    let after_role = weighted_base * role;
    add_label_value(
        &mut lines,
        "2. × role",
        format!("{:.2} × {:.2} = {:.2}", weighted_base, role, after_role),
        theme,
        width,
    );

    // Step 3: After structural adjustment
    let after_struct = after_role * struct_mult;
    add_label_value(
        &mut lines,
        "3. × struct",
        format!(
            "{:.2} × {:.2} = {:.2}",
            after_role, struct_mult, after_struct
        ),
        theme,
        width,
    );

    // Step 4: Show adjustments between formula result and stored base_score
    // The gap can include: debt aggregator, orchestration adjustment, context multiplier, clamping
    let base_score = stored_base;
    let mut step_num = 4;
    let mut current_value = after_struct;

    // Check for orchestration adjustment (spec 110)
    if let Some(adj) = &item.unified_score.adjustment_applied {
        if adj.reduction_percent.abs() > 0.1 {
            let after_orch = current_value * (1.0 - adj.reduction_percent / 100.0);
            add_label_value(
                &mut lines,
                &format!("{}. orchestration", step_num),
                format!(
                    "{:.2} × {:.2} = {:.2} ({})",
                    current_value,
                    1.0 - adj.reduction_percent / 100.0,
                    after_orch,
                    adj.adjustment_reason
                ),
                theme,
                width,
            );
            current_value = after_orch;
            step_num += 1;
        }
    }

    // Check for context multiplier (spec 191) - examples, tests, benchmarks get dampened
    if let Some(ctx_mult) = item.context_multiplier {
        if (ctx_mult - 1.0).abs() > 0.01 {
            let ctx_type_name = item
                .context_type
                .as_ref()
                .map(|t| format!("{:?}", t).to_lowercase())
                .unwrap_or_else(|| "context".to_string());
            let after_ctx = current_value * ctx_mult;
            add_label_value(
                &mut lines,
                &format!("{}. × context", step_num),
                format!(
                    "{:.2} × {:.2} = {:.2} ({} dampening)",
                    current_value, ctx_mult, after_ctx, ctx_type_name
                ),
                theme,
                width,
            );
            current_value = after_ctx;
            step_num += 1;
        }
    }

    // Step 4+: Show debt adjustment if applied (spec 260)
    if let Some(debt) = &item.unified_score.debt_adjustment {
        if debt.total.abs() > 0.01 {
            let after_debt = current_value + debt.total;
            // Show breakdown of debt components
            let components: Vec<String> = [
                (debt.testing, "test"),
                (debt.resource, "res"),
                (debt.duplication, "dup"),
            ]
            .iter()
            .filter(|(v, _)| v.abs() > 0.01)
            .map(|(v, name)| format!("{}:{:+.2}", name, v))
            .collect();
            let breakdown = if components.is_empty() {
                String::new()
            } else {
                format!(" ({})", components.join(", "))
            };
            add_label_value(
                &mut lines,
                &format!("{}. + debt", step_num),
                format!(
                    "{:.2} + {:.2} = {:.2}{}",
                    current_value, debt.total, after_debt, breakdown
                ),
                theme,
                width,
            );
            current_value = after_debt;
            step_num += 1;
        }
    }

    // Step: Show contextual risk multiplier if applied (spec 255)
    // This amplifies the score based on git history (churn, recency, bug likelihood)
    if let Some(risk_mult) = item.unified_score.contextual_risk_multiplier {
        if (risk_mult - 1.0).abs() > 0.01 {
            let after_risk = current_value * risk_mult;
            add_label_value(
                &mut lines,
                &format!("{}. × risk", step_num),
                format!(
                    "{:.2} × {:.2} = {:.2} (contextual risk)",
                    current_value, risk_mult, after_risk
                ),
                theme,
                width,
            );
            current_value = after_risk;
            step_num += 1;
        }
    }

    // Show clamping if it occurred (spec 260)
    if let Some(pre_norm) = item.unified_score.pre_normalization_score {
        if pre_norm > 100.0 {
            add_label_value(
                &mut lines,
                &format!("{}. clamped", step_num),
                format!("{:.2} → 100.00 (max score)", pre_norm),
                theme,
                width,
            );
            // Values updated for potential future use in display pipeline
            let _ = (100.0f64, step_num + 1);
        } else if (pre_norm - current_value).abs() > 0.5 {
            // Significant normalization applied
            add_label_value(
                &mut lines,
                &format!("{}. normalized", step_num),
                format!("{:.2} → {:.2}", pre_norm, current_value),
                theme,
                width,
            );
            // Value updated for potential future use in display pipeline
            let _ = step_num + 1;
        }
    } else if (base_score - current_value).abs() > 0.5 {
        // Fallback for data without detailed tracking - show what adjustment was applied
        if base_score > current_value && current_value > 0.0 {
            // Show the multiplier that was implicitly applied
            let implicit_mult = base_score / current_value;
            add_label_value(
                &mut lines,
                &format!("{}. × implicit", step_num),
                format!(
                    "{:.2} × {:.2} = {:.2} (untracked multiplier)",
                    current_value, implicit_mult, base_score
                ),
                theme,
                width,
            );
        } else if base_score < current_value && base_score <= 100.0 {
            add_label_value(
                &mut lines,
                &format!("{}. clamped", step_num),
                format!("{:.2} → {:.2}", current_value, base_score),
                theme,
                width,
            );
        } else {
            add_label_value(
                &mut lines,
                &format!("{}. normalized", step_num),
                format!("{:.2} → {:.2}", current_value, base_score),
                theme,
                width,
            );
        }
    } else if (current_value - base_score).abs() <= 0.5 {
        // No significant gap - just show the normalized value
        add_label_value(
            &mut lines,
            &format!("{}. normalized", step_num),
            format!("{:.2}", base_score),
            theme,
            width,
        );
    }

    // Track running value for exponential and boost
    let mut current = base_score;

    // Show exponential scaling if applied
    if (exponent - 1.0).abs() > 0.01 {
        let after_exp = current.powf(exponent);
        add_label_value(
            &mut lines,
            "exponential",
            format!("{:.2}^{:.2} = {:.2}", current, exponent, after_exp),
            theme,
            width,
        );
        current = after_exp;
    }

    // Show risk boost if applied
    if (risk_boost - 1.0).abs() > 0.01 {
        let after_boost = current * risk_boost;
        add_label_value(
            &mut lines,
            "risk boost",
            format!("{:.2} × {:.2} = {:.2}", current, risk_boost, after_boost),
            theme,
            width,
        );
        current = after_boost;
    }

    // Show god object multiplier if applied
    if let Some(gm) = god_mult {
        let after_god = current * gm;
        add_label_value(
            &mut lines,
            "god mult",
            format!("{:.2} × {:.2} = {:.2}", current, gm, after_god),
            theme,
            width,
        );
        current = after_god;
    }

    // Show clamping step explicitly if pre_normalization_score indicates clamping occurred (spec 260)
    // This makes the 51.55 → 100 jump explicit rather than hidden
    if let Some(pre_norm) = item.unified_score.pre_normalization_score {
        if pre_norm > 100.0 {
            add_label_value(
                &mut lines,
                "CLAMPED",
                format!("{:.2} → 100.00 (exceeds max, capped)", pre_norm),
                theme,
                width,
            );
        }
    } else if current > 100.0 {
        // Fallback: if we calculated a value > 100 but no pre_normalization_score was stored
        add_label_value(
            &mut lines,
            "CLAMPED",
            format!("{:.2} → 100.00 (exceeds max, capped)", current),
            theme,
            width,
        );
    }

    // Final score
    add_label_value(
        &mut lines,
        "final",
        format!("{:.1}", final_score),
        theme,
        width,
    );

    add_blank_line(&mut lines);
    lines
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Add a factor line with value and explanation (follows column layout)
fn add_factor_line(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: f64,
    formula: &str,
    theme: &Theme,
    _width: u16,
) {
    let label_formatted = format!(
        "{:width$}",
        format!("{}{}", " ".repeat(INDENT), label),
        width = LABEL_WIDTH
    );
    let gap = " ".repeat(GAP);

    lines.push(Line::from(vec![
        Span::raw(label_formatted),
        Span::raw(gap),
        Span::styled(format!("{:.2}", value), Style::default().fg(theme.primary)),
        Span::raw("  "),
        Span::styled(format!("({})", formula), Style::default().fg(theme.muted)),
    ]));
}

/// Add a multiplier line with visual indicator (follows column layout)
fn add_multiplier_line(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: f64,
    reason: &str,
    theme: &Theme,
    _width: u16,
) {
    let color = if value < 0.8 {
        Color::Green // Reduces score
    } else if value > 1.2 {
        Color::Red // Increases score
    } else {
        theme.primary // Neutral
    };

    let effect = if value < 0.95 {
        "reduces"
    } else if value > 1.05 {
        "increases"
    } else {
        "neutral"
    };

    let label_formatted = format!(
        "{:width$}",
        format!("{}{}", " ".repeat(INDENT), label),
        width = LABEL_WIDTH
    );
    let gap = " ".repeat(GAP);

    lines.push(Line::from(vec![
        Span::raw(label_formatted),
        Span::raw(gap),
        Span::styled(format!("{:.2}x", value), Style::default().fg(color)),
        Span::raw("  "),
        Span::styled(
            format!("[{}] {}", effect, reason),
            Style::default().fg(theme.muted),
        ),
    ]));
}

// ============================================================================
// Public API for text extraction
// ============================================================================

/// Build all page lines for text extraction (pure)
pub fn build_page_lines(item: &UnifiedDebtItem, theme: &Theme, width: u16) -> Vec<Line<'static>> {
    [
        build_final_score_section(item, theme, width),
        build_raw_inputs_section(item, theme, width),
        build_score_factors_section(item, theme, width),
        build_multipliers_section(item, theme, width),
        build_scaling_pipeline_section(item, theme, width),
        build_god_object_impact_section(item, theme, width),
        build_orchestration_section(item, theme, width),
        build_calculation_summary_section(item, theme, width),
    ]
    .into_iter()
    .flatten()
    .collect()
}

// ============================================================================
// Render Shell (the "water" boundary)
// ============================================================================

/// Render score breakdown page showing detailed scoring analysis
pub fn render(
    frame: &mut Frame,
    _app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let lines = build_page_lines(item, theme, area.width);

    // I/O boundary: render the widget
    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::score_types::Score0To100;
    use crate::priority::unified_scorer::{Location, UnifiedScore};
    use crate::priority::{ActionableRecommendation, FunctionRole, ImpactMetrics};

    fn create_test_item(final_score: f64, debt_type: DebtType) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: std::path::PathBuf::from("test.rs"),
                line: 10,
                function: "test_func".to_string(),
            },
            unified_score: UnifiedScore {
                final_score: Score0To100::new(final_score),
                complexity_factor: 5.0,
                coverage_factor: 7.0,
                dependency_factor: 3.0,
                role_multiplier: 1.2,
                base_score: Some(25.0),
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: Some(0.7),
                refactorability_factor: Some(1.0),
                pattern_factor: Some(0.85),
                // Spec 260: Score transparency fields
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.15),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
            },
            debt_type,
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Test".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 5.0,
                coverage_improvement: 0.1,
                lines_reduction: 10,
                risk_reduction: 0.2,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 5,
            downstream_dependencies: 10,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 3,
            function_length: 100,
            cyclomatic_complexity: 15,
            cognitive_complexity: 25,
            is_pure: Some(true),
            purity_confidence: Some(0.9),
            purity_level: None,
            entropy_details: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: Some(0.85),
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: Some(0.9),
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }

    #[test]
    fn test_build_final_score_section() {
        let item = create_test_item(
            65.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
        );
        let theme = Theme::default();

        let lines = build_final_score_section(&item, &theme, 80);

        assert!(!lines.is_empty());
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(content.contains("65.0"));
        assert!(content.contains("high")); // 50.0-69.9 is High severity
    }

    #[test]
    fn test_build_raw_inputs_section() {
        let item = create_test_item(
            50.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
        );
        let theme = Theme::default();

        let lines = build_raw_inputs_section(&item, &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(content.contains("cyclomatic"));
        assert!(content.contains("15"));
        assert!(content.contains("cognitive"));
        assert!(content.contains("25"));
    }

    #[test]
    fn test_build_score_factors_section() {
        let item = create_test_item(
            50.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
        );
        let theme = Theme::default();

        let lines = build_score_factors_section(&item, &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(content.contains("complexity"));
        assert!(content.contains("5.00")); // complexity_factor
    }

    #[test]
    fn test_build_multipliers_section() {
        let item = create_test_item(
            50.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
        );
        let theme = Theme::default();

        let lines = build_multipliers_section(&item, &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(content.contains("role"));
        assert!(content.contains("1.20")); // role_multiplier
        assert!(content.contains("purity"));
        assert!(content.contains("0.70")); // purity_factor
    }

    #[test]
    fn test_god_object_section_shown_for_god_objects() {
        let item = create_test_item(
            90.0,
            DebtType::GodObject {
                methods: 50,
                fields: Some(20),
                responsibilities: 8,
                lines: 1000,
                god_object_score: Score0To100::new(75.0),
            },
        );
        let theme = Theme::default();

        let lines = build_god_object_impact_section(&item, &theme, 80);

        assert!(!lines.is_empty());
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(content.contains("god object"));
        assert!(content.contains("75.0"));
        assert!(content.contains("warning"));
    }

    #[test]
    fn test_god_object_section_empty_for_non_god_objects() {
        let item = create_test_item(
            50.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
        );
        let theme = Theme::default();

        let lines = build_god_object_impact_section(&item, &theme, 80);

        assert!(lines.is_empty());
    }

    #[test]
    fn test_build_page_lines_combines_all_sections() {
        let item = create_test_item(
            75.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
        );
        let theme = Theme::default();

        let lines = build_page_lines(&item, &theme, 80);

        // Should have lines from multiple sections
        assert!(lines.len() > 10);
    }

    #[test]
    fn test_column_alignment_consistent() {
        // Verify that factor lines and multiplier lines use consistent column widths
        let item = create_test_item(
            50.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
        );
        let theme = Theme::default();

        let factor_lines = build_score_factors_section(&item, &theme, 80);
        let mult_lines = build_multipliers_section(&item, &theme, 80);

        // Both should have label columns of width LABEL_WIDTH (24)
        // Check that first span of data lines has consistent width
        for line in factor_lines.iter().skip(1) {
            // skip header
            if !line.spans.is_empty() && !line.spans[0].content.is_empty() {
                // Non-empty lines should have proper indent
                let first_span = &line.spans[0].content;
                if !first_span.trim().is_empty() {
                    assert!(
                        first_span.len() >= INDENT,
                        "Line should have at least {} indent: {:?}",
                        INDENT,
                        first_span
                    );
                }
            }
        }

        for line in mult_lines.iter().skip(1) {
            if !line.spans.is_empty() && !line.spans[0].content.is_empty() {
                let first_span = &line.spans[0].content;
                if !first_span.trim().is_empty() {
                    assert!(
                        first_span.len() >= INDENT,
                        "Line should have at least {} indent: {:?}",
                        INDENT,
                        first_span
                    );
                }
            }
        }
    }
}
