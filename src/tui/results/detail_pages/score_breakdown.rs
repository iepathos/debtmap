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

    let score = item.unified_score.final_score;
    let severity = Severity::from_score_100(score);
    let severity_color = match severity {
        Severity::Critical => Color::Red,
        Severity::High => Color::LightRed,
        Severity::Medium => Color::Yellow,
        Severity::Low => Color::Green,
    };

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
    // Check item-level entropy_analysis first, then god object aggregated entropy
    let entropy_adjusted = item
        .entropy_analysis
        .as_ref()
        .map(|e| e.adjusted_complexity)
        .or_else(|| {
            item.god_object_indicators
                .as_ref()
                .and_then(|g| g.aggregated_entropy.as_ref())
                .map(|e| e.adjusted_complexity)
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

    // Complexity factor - show actual calculation
    // The scorer may apply purity bonus to metrics before weighting
    // Formula: (cyc×0.4 + adj_cog×0.6) / 2 where adj_cog is entropy-adjusted
    let cyc = item.cyclomatic_complexity;
    let cog_adjusted = item
        .entropy_analysis
        .as_ref()
        .map(|e| e.adjusted_complexity)
        .unwrap_or(item.cognitive_complexity);

    // Check if purity bonus was applied (different from data flow purity_factor)
    // If purity_level is pure, complexity metrics were reduced before scoring
    let purity_bonus = item.purity_level.map(|level| {
        let conf = item.purity_confidence.unwrap_or(0.0);
        match level {
            crate::core::PurityLevel::StrictlyPure if conf > 0.8 => 0.70,
            crate::core::PurityLevel::StrictlyPure => 0.80,
            crate::core::PurityLevel::LocallyPure if conf > 0.8 => 0.75,
            crate::core::PurityLevel::LocallyPure => 0.85,
            crate::core::PurityLevel::ReadOnly if conf > 0.8 => 0.90,
            crate::core::PurityLevel::ReadOnly => 0.95,
            crate::core::PurityLevel::Impure => 1.0,
        }
    });

    let complexity_formula = if let Some(bonus) = purity_bonus {
        if (bonus - 1.0_f64).abs() > 0.01 {
            // Show purity-adjusted values
            let adj_cyc = (cyc as f64 * bonus) as u32;
            let adj_cog = (cog_adjusted as f64 * bonus) as u32;
            if has_god_object {
                format!(
                    "({}×0.4 + {}×0.6) / 2 × god [purity ×{:.2}]",
                    adj_cyc, adj_cog, bonus
                )
            } else {
                format!(
                    "({}×0.4 + {}×0.6) / 2 [purity ×{:.2}]",
                    adj_cyc, adj_cog, bonus
                )
            }
        } else if has_god_object {
            format!("({}×0.4 + {}×0.6) / 2 × god", cyc, cog_adjusted)
        } else {
            format!("({}×0.4 + {}×0.6) / 2", cyc, cog_adjusted)
        }
    } else if has_god_object {
        format!("({}×0.4 + {}×0.6) / 2 × god", cyc, cog_adjusted)
    } else {
        format!("({}×0.4 + {}×0.6) / 2", cyc, cog_adjusted)
    };
    add_factor_line(
        &mut lines,
        "complexity",
        item.unified_score.complexity_factor,
        &complexity_formula,
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

    // Entropy dampening - only show as multiplier for god objects where it's applied to final score
    // For regular functions, entropy is already applied to cognitive complexity (shown in raw inputs)
    let is_god_object = matches!(&item.debt_type, DebtType::GodObject { .. })
        || item
            .god_object_indicators
            .as_ref()
            .map(|g| g.is_god_object)
            .unwrap_or(false);

    if is_god_object {
        // For god objects, entropy dampening IS a score multiplier
        // Check item-level entropy_analysis first, then god object aggregated
        let dampening_factor = item
            .entropy_analysis
            .as_ref()
            .map(|e| e.dampening_factor)
            .or_else(|| {
                item.god_object_indicators
                    .as_ref()
                    .and_then(|g| g.aggregated_entropy.as_ref())
                    .map(|e| e.dampening_factor)
            });

        if let Some(dampening) = dampening_factor {
            let desc = if item.entropy_analysis.is_some() {
                "repetitive patterns"
            } else {
                "aggregated repetition"
            };
            add_multiplier_line(
                &mut lines,
                "entropy dampening",
                dampening,
                desc,
                theme,
                width,
            );
        }
    }
    // For regular functions: entropy is already reflected in cognitive complexity
    // (shown as "cognitive: X → Y (entropy-adjusted)" in raw inputs section)

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
        } => Some(*god_object_score),
        _ => item
            .god_object_indicators
            .as_ref()
            .filter(|g| g.is_god_object)
            .map(|g| g.god_object_score),
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

// ============================================================================
// Calculation Context and Supporting Types
// ============================================================================

/// Context for score calculation display, consolidating all extracted state upfront.
/// This eliminates repeated field access and makes the calculation logic clearer.
struct CalculationContext {
    /// Whether this is a DebtType::GodObject item
    is_god_object_item: bool,
    /// Whether the item has god object multiplier applied (from indicators)
    has_god_object: bool,
    /// God object multiplier if applicable
    god_multiplier: Option<f64>,
    /// Role multiplier
    role: f64,
    /// Purity factor
    purity: f64,
    /// Pattern factor
    pattern: f64,
    /// Refactorability factor
    refactor: f64,
    /// Context multiplier
    context: f64,
    /// Structural multiplier
    struct_mult: f64,
    /// Whether coverage data is available
    has_coverage_data: bool,
    /// Base score (before exponential scaling)
    base_score: f64,
    /// Exponential factor
    exponent: f64,
    /// Risk boost
    risk_boost: f64,
    /// Final score
    final_score: f64,
}

/// Collect all calculation context from a debt item (pure function).
fn collect_calculation_context(item: &UnifiedDebtItem) -> CalculationContext {
    // Check if god object multiplier was applied
    let has_god_object = matches!(&item.debt_type, DebtType::GodObject { .. })
        || item
            .god_object_indicators
            .as_ref()
            .map(|g| g.is_god_object)
            .unwrap_or(false);

    // Get god object multiplier if applicable
    let god_multiplier = if has_god_object {
        let go_score = match &item.debt_type {
            DebtType::GodObject {
                god_object_score, ..
            } => *god_object_score,
            _ => item
                .god_object_indicators
                .as_ref()
                .map(|g| g.god_object_score)
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

    // Use the stored structural multiplier from the actual scorer
    let struct_mult = item.unified_score.structural_multiplier.unwrap_or_else(|| {
        if item.cyclomatic_complexity == 0 {
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
        }
    });

    CalculationContext {
        is_god_object_item: matches!(item.debt_type, DebtType::GodObject { .. }),
        has_god_object,
        god_multiplier,
        role,
        purity,
        pattern,
        refactor,
        context,
        struct_mult,
        has_coverage_data: item.unified_score.has_coverage_data,
        base_score: item.unified_score.base_score.unwrap_or(0.0),
        exponent: item.unified_score.exponential_factor.unwrap_or(1.0),
        risk_boost: item.unified_score.risk_boost.unwrap_or(1.0),
        final_score: item.unified_score.final_score,
    }
}

// ============================================================================
// Formula Display Functions (Pure)
// ============================================================================

/// Build god object formula display lines (pure, static content).
fn build_god_object_formula_lines(theme: &Theme, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    add_label_value(
        &mut lines,
        "formula",
        "M × F × R × S × 20 × violations × adjustments".to_string(),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "where",
        "M=methods/20, F=fields/15, R=resp/3, S=lines/1000".to_string(),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "adjustments",
        "entropy dampening, complexity weight, functional bonus".to_string(),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "note",
        "factors capped at 3.0, uses weighted methods if available".to_string(),
        theme,
        width,
    );

    lines
}

/// Build regular function formula display lines (pure).
fn build_regular_formula_lines(
    ctx: &CalculationContext,
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Determine formula based on coverage and god object status
    let formula = if ctx.has_coverage_data {
        if ctx.has_god_object {
            "(C×5 + D×2.5) × cov × role × struct × god"
        } else {
            "(C×5 + D×2.5) × cov × role × struct"
        }
    } else if ctx.has_god_object {
        "(C×5 + D×2.5) × role × struct × god"
    } else {
        "(C×5 + D×2.5) × role × struct"
    };
    add_label_value(&mut lines, "formula", formula.to_string(), theme, width);

    // Show variable legend - use raw complexity for god objects
    let c_display = if let Some(gm) = ctx.god_multiplier {
        item.unified_score.complexity_factor / gm
    } else {
        item.unified_score.complexity_factor
    };

    if ctx.has_coverage_data {
        add_label_value(
            &mut lines,
            "where",
            format!(
                "C={:.1}, D={:.1}, cov={:.2}, role={:.2}, struct={:.2}",
                c_display,
                item.unified_score.dependency_factor,
                item.unified_score.coverage_factor / 10.0,
                ctx.role,
                ctx.struct_mult
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
                c_display, item.unified_score.dependency_factor, ctx.role, ctx.struct_mult
            ),
            theme,
            width,
        );
    }

    // Show other multipliers (purity, pattern, refactor, context)
    let other_mults: Vec<String> = [
        (ctx.purity, "purity"),
        (ctx.pattern, "pattern"),
        (ctx.refactor, "refactor"),
        (ctx.context, "context"),
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

    if let Some(gm) = ctx.god_multiplier {
        add_label_value(
            &mut lines,
            "god_mult",
            format!("{:.2} (3.0 + score/50)", gm),
            theme,
            width,
        );
    }

    lines
}

// ============================================================================
// God Object Calculation Types (Pure Data)
// ============================================================================

/// Raw metrics extracted from a god object item (pure data).
#[derive(Debug, Clone)]
struct GodObjectMetrics {
    methods: usize,
    fields: usize,
    responsibilities: usize,
    lines_of_code: usize,
    weighted_methods: Option<f64>,
    go_score: f64,
    entropy_damp: Option<f64>,
}

/// Thresholds for god object detection (pure data).
#[derive(Debug, Clone, Copy)]
struct GodObjectThresholds {
    max_methods: usize,
    max_fields: usize,
    max_lines: usize,
    max_responsibilities: usize,
}

impl Default for GodObjectThresholds {
    fn default() -> Self {
        Self {
            max_methods: 20,
            max_fields: 15,
            max_lines: 1000,
            max_responsibilities: 5,
        }
    }
}

/// Calculated factors for god object scoring (pure data).
#[derive(Debug, Clone, Copy)]
struct GodObjectFactors {
    method_factor: f64,
    field_factor: f64,
    resp_factor: f64,
    size_factor: f64,
}

impl GodObjectFactors {
    /// Calculate the base product of all factors (pure).
    fn base_product(&self) -> f64 {
        self.method_factor * self.field_factor * self.resp_factor * self.size_factor
    }
}

// ============================================================================
// God Object Extraction Functions (Pure)
// ============================================================================

/// Extract god object metrics from a debt item (pure).
fn extract_god_object_metrics(item: &UnifiedDebtItem) -> GodObjectMetrics {
    let stored_base = item.unified_score.base_score.unwrap_or(0.0);

    let go_score = match &item.debt_type {
        DebtType::GodObject {
            god_object_score, ..
        } => *god_object_score,
        _ => stored_base,
    };

    let (methods, fields, responsibilities, lines_of_code) = match &item.debt_type {
        DebtType::GodObject {
            methods,
            fields,
            responsibilities,
            lines: loc,
            ..
        } => (
            *methods as usize,
            fields.unwrap_or(0) as usize,
            *responsibilities as usize,
            *loc as usize,
        ),
        _ => item
            .god_object_indicators
            .as_ref()
            .map(|ind| {
                (
                    ind.method_count,
                    ind.field_count,
                    ind.responsibility_count,
                    ind.lines_of_code,
                )
            })
            .unwrap_or((0, 0, 0, 0)),
    };

    let weighted_methods = item
        .god_object_indicators
        .as_ref()
        .and_then(|g| g.weighted_method_count);

    let entropy_damp = item.entropy_analysis.as_ref().map(|e| e.dampening_factor);

    GodObjectMetrics {
        methods,
        fields,
        responsibilities,
        lines_of_code,
        weighted_methods,
        go_score,
        entropy_damp,
    }
}

/// Calculate god object factors from metrics (pure).
fn calculate_god_object_factors(
    metrics: &GodObjectMetrics,
    thresholds: &GodObjectThresholds,
) -> GodObjectFactors {
    let effective_methods = metrics.weighted_methods.unwrap_or(metrics.methods as f64);

    GodObjectFactors {
        method_factor: (effective_methods / thresholds.max_methods as f64).min(3.0),
        field_factor: (metrics.fields as f64 / thresholds.max_fields as f64).min(3.0),
        resp_factor: (metrics.responsibilities as f64 / 3.0).min(3.0),
        size_factor: (metrics.lines_of_code as f64 / thresholds.max_lines as f64).min(3.0),
    }
}

/// Build list of threshold violations (pure).
fn build_god_object_violations(
    metrics: &GodObjectMetrics,
    thresholds: &GodObjectThresholds,
) -> Vec<String> {
    let effective_methods = metrics.weighted_methods.unwrap_or(metrics.methods as f64);

    [
        (
            effective_methods > thresholds.max_methods as f64,
            format!(
                "methods {:.0} > {}",
                effective_methods, thresholds.max_methods
            ),
        ),
        (
            metrics.fields > thresholds.max_fields,
            format!("fields {} > {}", metrics.fields, thresholds.max_fields),
        ),
        (
            metrics.responsibilities > thresholds.max_responsibilities,
            format!(
                "responsibilities {} > {}",
                metrics.responsibilities, thresholds.max_responsibilities
            ),
        ),
        (
            metrics.lines_of_code > thresholds.max_lines,
            format!("lines {} > {}", metrics.lines_of_code, thresholds.max_lines),
        ),
    ]
    .into_iter()
    .filter(|(violated, _)| *violated)
    .map(|(_, msg)| msg)
    .collect()
}

// ============================================================================
// God Object Step Display Builders (Pure)
// ============================================================================

/// Build factor display lines for god object calculation (pure).
fn build_god_object_factor_lines(
    metrics: &GodObjectMetrics,
    factors: &GodObjectFactors,
    thresholds: &GodObjectThresholds,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Step 1: Method factor
    let method_display = match metrics.weighted_methods {
        Some(w) if (w - metrics.methods as f64).abs() > 0.1 => format!(
            "{:.2} = {:.1} / {} (raw {} → weighted)",
            factors.method_factor, w, thresholds.max_methods, metrics.methods
        ),
        _ => format!(
            "{:.2} = {} / {} (max 3.0)",
            factors.method_factor, metrics.methods, thresholds.max_methods
        ),
    };
    add_label_value(&mut lines, "1. method factor", method_display, theme, width);

    // Step 2: Field factor
    add_label_value(
        &mut lines,
        "2. field factor",
        format!(
            "{:.2} = {} / {} (max 3.0)",
            factors.field_factor, metrics.fields, thresholds.max_fields
        ),
        theme,
        width,
    );

    // Step 3: Responsibility factor
    add_label_value(
        &mut lines,
        "3. responsibility factor",
        format!(
            "{:.2} = {} / 3 (max 3.0)",
            factors.resp_factor, metrics.responsibilities
        ),
        theme,
        width,
    );

    // Step 4: Size factor
    add_label_value(
        &mut lines,
        "4. size factor",
        format!(
            "{:.2} = {} / {} (max 3.0)",
            factors.size_factor, metrics.lines_of_code, thresholds.max_lines
        ),
        theme,
        width,
    );

    // Step 5: Base product
    let base_product = factors.base_product();
    add_label_value(
        &mut lines,
        "5. base product",
        format!(
            "{:.2} = {:.2} × {:.2} × {:.2} × {:.2}",
            base_product,
            factors.method_factor,
            factors.field_factor,
            factors.resp_factor,
            factors.size_factor
        ),
        theme,
        width,
    );

    lines
}

/// Build violation and scaling lines (pure).
/// Returns (scaled_score, step_number_after, lines).
fn build_god_object_scaling_lines(
    factors: &GodObjectFactors,
    violations: &[String],
    theme: &Theme,
    width: u16,
) -> (f64, usize, Vec<Line<'static>>) {
    let mut lines = Vec::new();
    let base_product = factors.base_product();
    let violation_count = violations.len();

    // Step 6: Violations
    let violation_detail = if violations.is_empty() {
        "none".to_string()
    } else {
        violations.join(", ")
    };
    add_label_value(
        &mut lines,
        "6. violations",
        format!("{} ({})", violation_count, violation_detail),
        theme,
        width,
    );

    // Step 7: Scaling
    let scaled_score = if violation_count > 0 {
        base_product * 20.0 * violation_count as f64
    } else {
        base_product * 10.0
    };
    let scaling_detail = if violation_count > 0 {
        format!(
            "{:.2} × 20 × {} = {:.2}",
            base_product, violation_count, scaled_score
        )
    } else {
        format!("{:.2} × 10 = {:.2}", base_product, scaled_score)
    };
    add_label_value(&mut lines, "7. scaling", scaling_detail, theme, width);

    (scaled_score, 8, lines)
}

/// Build adjustment lines after scaling (pure).
/// Returns (final_step_num, final_value, lines).
fn build_god_object_adjustment_lines(
    metrics: &GodObjectMetrics,
    mut current: f64,
    mut step_num: usize,
    theme: &Theme,
    width: u16,
) -> (usize, f64, Vec<Line<'static>>) {
    let mut lines = Vec::new();

    // Entropy dampening
    if let Some(entropy_damp) = metrics.entropy_damp {
        if (entropy_damp - 1.0).abs() > 0.01 {
            let after_entropy = current * entropy_damp;
            add_label_value(
                &mut lines,
                &format!("{}. × entropy", step_num),
                format!(
                    "{:.2} × {:.2} = {:.2} (dampening)",
                    current, entropy_damp, after_entropy
                ),
                theme,
                width,
            );
            current = after_entropy;
            step_num += 1;
        }
    }

    // Combined adjustments for remaining gap
    let diff = (current - metrics.go_score).abs();
    if diff > 1.0 && current > 0.0 {
        let adjustment_factor = metrics.go_score / current;
        if (adjustment_factor - 1.0).abs() > 0.01 {
            let adjustment_desc = if adjustment_factor > 1.0 {
                "complexity weight"
            } else {
                "functional bonus"
            };
            add_label_value(
                &mut lines,
                &format!("{}. × {}", step_num, adjustment_desc),
                format!(
                    "{:.2} × {:.2} = {:.2}",
                    current, adjustment_factor, metrics.go_score
                ),
                theme,
                width,
            );
            step_num += 1;
        }
    }

    // Final god object score
    add_label_value(
        &mut lines,
        &format!("{}. god object score", step_num),
        format!("{:.2}", metrics.go_score),
        theme,
        width,
    );

    (step_num + 1, metrics.go_score, lines)
}

// ============================================================================
// Calculation Step Builders (Pure)
// ============================================================================

/// Build god object calculation steps (pure).
/// Returns (next_step_num, current_value, lines).
///
/// This function composes several pure helper functions:
/// - `extract_god_object_metrics`: Extract raw data
/// - `calculate_god_object_factors`: Compute scoring factors
/// - `build_god_object_violations`: Identify threshold violations
/// - `build_god_object_factor_lines`: Display factor steps
/// - `build_god_object_scaling_lines`: Display scaling steps
/// - `build_god_object_adjustment_lines`: Display final adjustments
fn build_god_object_calculation_steps(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> (usize, f64, Vec<Line<'static>>) {
    let metrics = extract_god_object_metrics(item);
    let thresholds = GodObjectThresholds::default();
    let factors = calculate_god_object_factors(&metrics, &thresholds);
    let violations = build_god_object_violations(&metrics, &thresholds);

    let mut lines = build_god_object_factor_lines(&metrics, &factors, &thresholds, theme, width);

    let (scaled_score, step_num, scaling_lines) =
        build_god_object_scaling_lines(&factors, &violations, theme, width);
    lines.extend(scaling_lines);

    let (final_step, final_value, adjustment_lines) =
        build_god_object_adjustment_lines(&metrics, scaled_score, step_num, theme, width);
    lines.extend(adjustment_lines);

    (final_step, final_value, lines)
}

/// Build regular function calculation steps (pure).
/// Returns (next_step_num, current_value, lines).
fn build_regular_calculation_steps(
    ctx: &CalculationContext,
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> (usize, f64, Vec<Line<'static>>) {
    let mut lines = Vec::new();

    let c = item.unified_score.complexity_factor;
    let d = item.unified_score.dependency_factor;

    // Both paths use the same weighted base: (C×5 + D×2.5)
    let weighted_base = (c * 5.0) + (d * 2.5);

    let (displayed_base, formula_detail) = if ctx.has_coverage_data {
        let cov_mult = item.unified_score.coverage_factor / 10.0;
        let base_with_cov = weighted_base * cov_mult;
        (
            base_with_cov,
            format!(
                "{:.2} = (C×5 + D×2.5) × cov = ({:.1}×5 + {:.1}×2.5) × {:.2}",
                base_with_cov, c, d, cov_mult
            ),
        )
    } else {
        (
            weighted_base,
            format!(
                "{:.2} = C×5 + D×2.5 = {:.1}×5 + {:.1}×2.5",
                weighted_base, c, d
            ),
        )
    };
    add_label_value(&mut lines, "1. weighted base", formula_detail, theme, width);

    // Step 2: After role adjustment
    let after_role = displayed_base * ctx.role;
    let mut next_step = 2_usize;
    if (ctx.role - 1.0).abs() > 0.01 {
        add_label_value(
            &mut lines,
            "2. × role",
            format!(
                "{:.2} × {:.2} = {:.2}",
                displayed_base, ctx.role, after_role
            ),
            theme,
            width,
        );
        next_step = 3;
    }

    // Step 3: After structural adjustment
    let after_struct = after_role * ctx.struct_mult;
    if (ctx.struct_mult - 1.0).abs() > 0.01 {
        add_label_value(
            &mut lines,
            &format!("{}. × struct", next_step),
            format!(
                "{:.2} × {:.2} = {:.2}",
                after_role, ctx.struct_mult, after_struct
            ),
            theme,
            width,
        );
        next_step += 1;
    }

    (next_step, after_struct, lines)
}

// ============================================================================
// Adjustment Step Builder (Pure)
// ============================================================================

/// Build post-calculation adjustment steps (pure).
fn build_adjustment_steps(
    item: &UnifiedDebtItem,
    ctx: &CalculationContext,
    mut step_num: usize,
    mut current_value: f64,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

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

    // Check for context multiplier (spec 191)
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

    // Show debt adjustment if applied (spec 260)
    if let Some(debt) = &item.unified_score.debt_adjustment {
        if debt.total.abs() > 0.01 {
            let after_debt = current_value + debt.total;
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

    // Show contextual risk multiplier if applied (spec 255, spec 260)
    if let Some(risk_mult) = item.unified_score.contextual_risk_multiplier {
        if (risk_mult - 1.0).abs() > 0.001 {
            let pre_ctx = item
                .unified_score
                .pre_contextual_score
                .unwrap_or(current_value);
            let after_risk = pre_ctx * risk_mult;
            add_label_value(
                &mut lines,
                &format!("{}. × risk", step_num),
                format!(
                    "{:.2} × {:.2} = {:.2} (git history risk)",
                    pre_ctx, risk_mult, after_risk
                ),
                theme,
                width,
            );
            current_value = after_risk;
            step_num += 1;
        }
    }

    // Handle any remaining gap between calculated value and stored base_score
    let gap_to_base = (ctx.base_score - current_value).abs();
    let gap_to_final = (ctx.final_score - current_value).abs();
    let base_close_to_final = (ctx.base_score - ctx.final_score).abs() < ctx.final_score * 0.05;

    if gap_to_base > 0.5 && base_close_to_final {
        let pre_norm = item.unified_score.pre_normalization_score;

        if ctx.base_score > current_value && current_value > 0.0 {
            let diff = ctx.base_score - current_value;
            let explanation = if let Some(pn) = pre_norm {
                format!("{:.2} → {:.2} → {:.2}", current_value, pn, ctx.base_score)
            } else {
                format!(
                    "{:.2} + {:.2} = {:.2} (combined adjustments)",
                    current_value, diff, ctx.base_score
                )
            };
            add_label_value(
                &mut lines,
                &format!("{}. adjusted", step_num),
                explanation,
                theme,
                width,
            );
        } else if ctx.base_score < current_value {
            add_label_value(
                &mut lines,
                &format!("{}. adjusted", step_num),
                format!("{:.2} → {:.2}", current_value, ctx.base_score),
                theme,
                width,
            );
        } else {
            add_label_value(
                &mut lines,
                &format!("{}. normalized", step_num),
                format!("{:.2} → {:.2}", current_value, ctx.base_score),
                theme,
                width,
            );
        }
    } else if gap_to_final > 0.5 && gap_to_base > 0.5 {
        add_label_value(
            &mut lines,
            &format!("{}. final adjustments", step_num),
            format!("{:.2} → {:.2}", current_value, ctx.final_score),
            theme,
            width,
        );
    }

    // Track running value for exponential and boost
    let mut current = ctx.base_score;

    // Show exponential scaling if applied
    if (ctx.exponent - 1.0).abs() > 0.01 {
        let after_exp = current.powf(ctx.exponent);
        add_label_value(
            &mut lines,
            "exponential",
            format!("{:.2}^{:.2} = {:.2}", current, ctx.exponent, after_exp),
            theme,
            width,
        );
        current = after_exp;
    }

    // Show risk boost if applied
    if (ctx.risk_boost - 1.0).abs() > 0.01 {
        let after_boost = current * ctx.risk_boost;
        add_label_value(
            &mut lines,
            "risk boost",
            format!(
                "{:.2} × {:.2} = {:.2}",
                current, ctx.risk_boost, after_boost
            ),
            theme,
            width,
        );
    }

    // Final score
    add_label_value(
        &mut lines,
        "final",
        format!("{:.1}", ctx.final_score),
        theme,
        width,
    );

    lines
}

/// Build score calculation summary (pure).
///
/// This function orchestrates the display of score calculation details by composing
/// smaller, focused helper functions. The refactored structure:
/// - `collect_calculation_context`: Extracts all state upfront
/// - `build_god_object_formula_lines` / `build_regular_formula_lines`: Formula display
/// - `build_god_object_calculation_steps` / `build_regular_calculation_steps`: Step details
/// - `build_adjustment_steps`: Post-calculation adjustments and final score
pub fn build_calculation_summary_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let ctx = collect_calculation_context(item);
    let mut lines = Vec::new();

    // Formula section
    add_section_header(&mut lines, "score formula (simplified)", theme);
    if ctx.is_god_object_item {
        lines.extend(build_god_object_formula_lines(theme, width));
    } else {
        lines.extend(build_regular_formula_lines(&ctx, item, theme, width));
    }
    add_blank_line(&mut lines);

    // Calculation steps section
    add_section_header(&mut lines, "calculation steps", theme);
    let (step_num, current_value, step_lines) = if ctx.is_god_object_item {
        build_god_object_calculation_steps(item, theme, width)
    } else {
        build_regular_calculation_steps(&ctx, item, theme, width)
    };
    lines.extend(step_lines);

    // Adjustments and final score
    lines.extend(build_adjustment_steps(
        item,
        &ctx,
        step_num,
        current_value,
        theme,
        width,
    ));

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
// Spec 267: Multi-item context builders
// ============================================================================

/// Build item indicator section (pure) - shown when multiple items at location
///
/// Displays "viewing item X of N: {DebtType}" to clarify which item's
/// breakdown is being shown.
pub fn build_item_indicator_section(
    current_index: usize,
    total_items: usize,
    debt_type: &DebtType,
    theme: &Theme,
    _width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if total_items > 1 {
        let debt_name = super::overview::format_debt_type_name(debt_type);
        lines.push(Line::from(vec![Span::styled(
            format!(
                "viewing item {} of {}: {}",
                current_index + 1,
                total_items,
                debt_name
            ),
            Style::default().fg(theme.muted),
        )]));
        lines.push(Line::from("")); // Blank line after indicator
    }

    lines
}

/// Build combined reference line (pure) - shown at end of calculation when multi-item
///
/// Shows how this item's score contributes to the location combined score.
pub fn build_combined_reference_line(
    item_score: f64,
    combined_score: f64,
    other_item_count: usize,
    theme: &Theme,
    _width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if other_item_count > 0 {
        let other_text = if other_item_count == 1 {
            "1 other item".to_string()
        } else {
            format!("{} other items", other_item_count)
        };

        // Use same column layout as other lines
        const INDENT: usize = 2;
        const LABEL_WIDTH: usize = 24;
        const GAP: usize = 4;

        let label = format!(
            "{:width$}",
            format!("{}location combined", " ".repeat(INDENT)),
            width = LABEL_WIDTH
        );
        let gap = " ".repeat(GAP);

        lines.push(Line::from(vec![
            Span::raw(label),
            Span::raw(gap),
            Span::styled(
                format!("{:.1}", combined_score),
                Style::default().fg(theme.primary),
            ),
            Span::raw(" "),
            Span::styled(
                format!("(this + {})", other_text),
                Style::default().fg(theme.muted),
            ),
        ]));
        lines.push(Line::from("")); // Blank line after

        // Note: Show difference from just this item
        let _ = item_score; // Used in the combined calculation context
    }

    lines
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

/// Build all page lines with multi-item context (pure) - spec 267
pub fn build_page_lines_with_context(
    item: &UnifiedDebtItem,
    location_items: &[&UnifiedDebtItem],
    current_item_index: usize,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut result = Vec::new();

    // Add item indicator at top if multiple items at location
    if location_items.len() > 1 {
        result.extend(build_item_indicator_section(
            current_item_index,
            location_items.len(),
            &item.debt_type,
            theme,
            width,
        ));
    }

    // Add standard sections
    result.extend(build_final_score_section(item, theme, width));
    result.extend(build_raw_inputs_section(item, theme, width));
    result.extend(build_score_factors_section(item, theme, width));
    result.extend(build_multipliers_section(item, theme, width));
    result.extend(build_scaling_pipeline_section(item, theme, width));
    result.extend(build_god_object_impact_section(item, theme, width));
    result.extend(build_orchestration_section(item, theme, width));
    result.extend(build_calculation_summary_section(item, theme, width));

    // Add combined reference at end if multiple items at location
    if location_items.len() > 1 {
        let combined_score: f64 = location_items
            .iter()
            .map(|i| i.unified_score.final_score)
            .sum();
        result.extend(build_combined_reference_line(
            item.unified_score.final_score,
            combined_score,
            location_items.len() - 1,
            theme,
            width,
        ));
    }

    result
}

// ============================================================================
// Render Shell (the "water" boundary)
// ============================================================================

/// Render score breakdown page showing detailed scoring analysis
///
/// Spec 267: When multiple items exist at the same location, shows
/// item indicator at top and combined score reference at bottom.
pub fn render(
    frame: &mut Frame,
    app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    // Get all items at this location for multi-item context
    let location_items = super::overview::get_items_at_location(app, item);

    // Always show first item (grouping is always on, no cycling)
    let lines = build_page_lines_with_context(item, &location_items, 0, theme, area.width);

    // I/O boundary: render the widget with scroll support
    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false })
        .scroll(app.detail_scroll_offset());

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

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
                final_score: final_score.max(0.0),
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
                pre_contextual_score: None,
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
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 3,
            function_length: 100,
            cyclomatic_complexity: 15,
            cognitive_complexity: 25,
            is_pure: Some(true),
            purity_confidence: Some(0.9),
            purity_level: None,
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
            context_suggestion: None,
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
                god_object_score: 75.0,
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
