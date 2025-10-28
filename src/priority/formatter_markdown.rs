use crate::priority::{
    CategorizedDebt, CategorySummary, CrossCategoryDependency, DebtCategory, DebtItem, DebtType,
    DisplayGroup, FileDebtItem, ImpactLevel, Tier, UnifiedAnalysis, UnifiedDebtItem,
};
use std::fmt::Write;

/// Format priorities for markdown output without ANSI color codes
pub fn format_priorities_markdown(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    let mut output = String::new();

    let version = env!("CARGO_PKG_VERSION");
    writeln!(output, "# Debtmap v{}\n", version).unwrap();

    let top_items = analysis.get_top_mixed_priorities(limit);
    let count = top_items.len().min(limit);

    writeln!(output, "## Top {} Recommendations\n", count).unwrap();

    for (idx, item) in top_items.iter().enumerate() {
        format_mixed_priority_item_markdown(&mut output, idx + 1, item, verbosity);
        writeln!(output).unwrap();
    }

    // Add summary
    writeln!(output, "---\n").unwrap();
    writeln!(
        output,
        "**Total Debt Score:** {:.0}",
        analysis.total_debt_score
    )
    .unwrap();

    writeln!(
        output,
        "**Debt Density:** {:.1} per 1K LOC ({} total LOC)",
        analysis.debt_density, analysis.total_lines_of_code
    )
    .unwrap();

    if let Some(coverage) = analysis.overall_coverage {
        writeln!(output, "**Overall Coverage:** {:.2}%", coverage).unwrap();
    }

    output
}

/// Format priorities for markdown output with categorical grouping
pub fn format_priorities_categorical_markdown(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    let mut output = String::new();

    let version = env!("CARGO_PKG_VERSION");
    writeln!(output, "# Debtmap v{}\n", version).unwrap();

    let categorized = analysis.get_categorized_debt(limit);

    writeln!(output, "## Technical Debt Analysis - By Category\n").unwrap();

    // Sort categories by total score (highest first)
    let mut sorted_categories: Vec<(&DebtCategory, &CategorySummary)> =
        categorized.categories.iter().collect();
    sorted_categories.sort_by(|a, b| {
        b.1.total_score
            .partial_cmp(&a.1.total_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Format each category
    for (category, summary) in sorted_categories {
        format_category_section(&mut output, category, summary, verbosity);
    }

    // Add cross-category dependencies if any
    if !categorized.cross_category_dependencies.is_empty() {
        format_cross_category_dependencies(&mut output, &categorized.cross_category_dependencies);
    }

    // Add summary
    format_categorical_summary(&mut output, &categorized);

    writeln!(output).unwrap();
    writeln!(
        output,
        "**Total Debt Score:** {:.0}",
        analysis.total_debt_score
    )
    .unwrap();

    writeln!(
        output,
        "**Debt Density:** {:.1} per 1K LOC ({} total LOC)",
        analysis.debt_density, analysis.total_lines_of_code
    )
    .unwrap();

    if let Some(coverage) = analysis.overall_coverage {
        writeln!(output, "**Overall Coverage:** {:.2}%", coverage).unwrap();
    }

    output
}

fn format_category_section(
    output: &mut String,
    category: &DebtCategory,
    summary: &CategorySummary,
    verbosity: u8,
) {
    writeln!(
        output,
        "### {} {} ({} items)",
        category.icon(),
        category.name(),
        summary.item_count
    )
    .unwrap();

    writeln!(
        output,
        "**Total Score:** {:.1} | **Average Severity:** {:.1}",
        summary.total_score, summary.average_severity
    )
    .unwrap();

    writeln!(output).unwrap();
    writeln!(
        output,
        "{}",
        category.strategic_guidance(summary.item_count, summary.estimated_effort_hours)
    )
    .unwrap();
    writeln!(output).unwrap();

    // Show top items in this category
    if !summary.top_items.is_empty() {
        writeln!(output, "#### Top Priority Items").unwrap();
        writeln!(output).unwrap();

        for (idx, item) in summary.top_items.iter().take(3).enumerate() {
            format_categorized_debt_item(output, idx + 1, item, verbosity);
        }

        if summary.item_count > summary.top_items.len() {
            writeln!(
                output,
                "\n_... and {} more items in this category_",
                summary.item_count - summary.top_items.len()
            )
            .unwrap();
        }
    }

    writeln!(output).unwrap();
}

fn format_categorized_debt_item(output: &mut String, rank: usize, item: &DebtItem, verbosity: u8) {
    match item {
        DebtItem::Function(func) => {
            writeln!(
                output,
                "{}. **{}** - Score: {:.1}",
                rank, func.location.function, func.unified_score.final_score
            )
            .unwrap();
            writeln!(
                output,
                "   - Location: `{}:{}`",
                func.location.file.display(),
                func.location.line
            )
            .unwrap();
            writeln!(output, "   - Type: {}", format_debt_type(&func.debt_type)).unwrap();
            if verbosity >= 1 {
                writeln!(
                    output,
                    "   - Action: {}",
                    func.recommendation.primary_action
                )
                .unwrap();
            }
        }
        DebtItem::File(file) => {
            let file_name = file
                .metrics
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            writeln!(
                output,
                "{}. **{}** - Score: {:.1}",
                rank, file_name, file.score
            )
            .unwrap();
            writeln!(output, "   - Path: `{}`", file.metrics.path.display()).unwrap();
            writeln!(
                output,
                "   - Metrics: {} lines, {} functions",
                file.metrics.total_lines, file.metrics.function_count
            )
            .unwrap();
            if verbosity >= 1 {
                writeln!(output, "   - Action: {}", file.recommendation).unwrap();
            }
        }
    }
}

fn format_cross_category_dependencies(
    output: &mut String,
    dependencies: &[CrossCategoryDependency],
) {
    writeln!(output, "### [PERF] Cross-Category Dependencies\n").unwrap();
    writeln!(
        output,
        "These relationships affect how you should prioritize improvements:\n"
    )
    .unwrap();

    for dep in dependencies {
        let impact_symbol = match dep.impact_level {
            ImpactLevel::Critical => "[ERROR]",
            ImpactLevel::High => "[WARN]",
            ImpactLevel::Medium => "[WARN]",
            ImpactLevel::Low => "[OK]",
        };

        writeln!(
            output,
            "{} **{} → {}**: {}",
            impact_symbol,
            dep.source_category.name(),
            dep.target_category.name(),
            dep.description
        )
        .unwrap();
    }
    writeln!(output).unwrap();
}

fn format_categorical_summary(output: &mut String, categorized: &CategorizedDebt) {
    writeln!(output, "---\n").unwrap();
    writeln!(output, "## Summary by Category\n").unwrap();

    let total_items: usize = categorized.categories.values().map(|c| c.item_count).sum();
    let total_effort: u32 = categorized
        .categories
        .values()
        .map(|c| c.estimated_effort_hours)
        .sum();

    writeln!(output, "**Total Debt Items:** {}", total_items).unwrap();
    writeln!(output, "**Total Estimated Effort:** {} hours", total_effort).unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "| Category | Items | Total Score | Effort (hours) |"
    )
    .unwrap();
    writeln!(output, "|----------|-------|------------|----------------|").unwrap();

    for (category, summary) in &categorized.categories {
        writeln!(
            output,
            "| {} {} | {} | {:.1} | {} |",
            category.icon(),
            category.name(),
            summary.item_count,
            summary.total_score,
            summary.estimated_effort_hours
        )
        .unwrap();
    }
}

/// Format priorities for markdown output with tiered display
pub fn format_priorities_tiered_markdown(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    let mut output = String::new();

    let version = env!("CARGO_PKG_VERSION");
    writeln!(output, "# Debtmap v{}\n", version).unwrap();

    let tiered_display = analysis.get_tiered_display(limit);

    writeln!(output, "## Technical Debt Analysis - Priority Tiers\n").unwrap();

    // Format each tier
    format_tier_section(
        &mut output,
        &tiered_display.critical,
        Tier::Critical,
        verbosity,
    );
    format_tier_section(&mut output, &tiered_display.high, Tier::High, verbosity);
    format_tier_section(
        &mut output,
        &tiered_display.moderate,
        Tier::Moderate,
        verbosity,
    );
    format_tier_section(&mut output, &tiered_display.low, Tier::Low, verbosity);

    // Add summary
    writeln!(output, "---\n").unwrap();
    writeln!(output, "## Summary\n").unwrap();

    let critical_count: usize = tiered_display.critical.iter().map(|g| g.items.len()).sum();
    let high_count: usize = tiered_display.high.iter().map(|g| g.items.len()).sum();
    let moderate_count: usize = tiered_display.moderate.iter().map(|g| g.items.len()).sum();
    let low_count: usize = tiered_display.low.iter().map(|g| g.items.len()).sum();

    writeln!(
        output,
        "**Total Debt Items:** {}",
        critical_count + high_count + moderate_count + low_count
    )
    .unwrap();
    writeln!(output, "- [CRITICAL] Critical: {} items", critical_count).unwrap();
    writeln!(output, "- [WARN] High: {} items", high_count).unwrap();
    writeln!(output, "- Moderate: {} items", moderate_count).unwrap();
    writeln!(output, "- [INFO] Low: {} items", low_count).unwrap();

    writeln!(output).unwrap();
    writeln!(
        output,
        "**Total Debt Score:** {:.0}",
        analysis.total_debt_score
    )
    .unwrap();

    writeln!(
        output,
        "**Debt Density:** {:.1} per 1K LOC ({} total LOC)",
        analysis.debt_density, analysis.total_lines_of_code
    )
    .unwrap();

    if let Some(coverage) = analysis.overall_coverage {
        writeln!(output, "**Overall Coverage:** {:.2}%", coverage).unwrap();
    }

    output
}

fn format_tier_section(output: &mut String, groups: &[DisplayGroup], tier: Tier, verbosity: u8) {
    if groups.is_empty() {
        return;
    }

    writeln!(output, "### {}", tier.header()).unwrap();
    writeln!(output, "_Estimated effort: {}_\n", tier.effort_estimate()).unwrap();

    let max_items_per_tier = 5;
    let mut items_shown = 0;

    for group in groups {
        if items_shown >= max_items_per_tier && verbosity < 2 {
            let remaining: usize = groups.iter().skip(items_shown).map(|g| g.items.len()).sum();
            if remaining > 0 {
                writeln!(
                    output,
                    "\n_... and {} more items in this tier_\n",
                    remaining
                )
                .unwrap();
            }
            break;
        }

        format_display_group(output, group, verbosity);
        items_shown += group.items.len();
    }

    writeln!(output).unwrap();
}

fn format_display_group(output: &mut String, group: &DisplayGroup, verbosity: u8) {
    if group.items.len() > 1 && group.batch_action.is_some() {
        // Format as grouped items
        writeln!(
            output,
            "#### {} ({} items)",
            group.debt_type,
            group.items.len()
        )
        .unwrap();

        if let Some(action) = &group.batch_action {
            writeln!(output, "**Batch Action:** {}\n", action).unwrap();
        }

        if verbosity >= 1 {
            writeln!(output, "**Items:**").unwrap();
            for (idx, item) in group.items.iter().take(3).enumerate() {
                format_debt_item_brief(output, idx + 1, item);
            }
            if group.items.len() > 3 {
                writeln!(
                    output,
                    "- _... and {} more similar items_",
                    group.items.len() - 3
                )
                .unwrap();
            }
        } else {
            let total_score: f64 = group.items.iter().map(|i| i.score()).sum();
            writeln!(output, "- Combined Score: {:.1}", total_score).unwrap();
            writeln!(output, "- Count: {} items", group.items.len()).unwrap();
        }
    } else {
        // Format as individual item
        for item in &group.items {
            format_debt_item_detailed(output, item, verbosity);
        }
    }
    writeln!(output).unwrap();
}

fn format_debt_item_brief(output: &mut String, rank: usize, item: &DebtItem) {
    match item {
        DebtItem::Function(func) => {
            writeln!(
                output,
                "- #{} `{}` (Score: {:.1})",
                rank, func.location.function, func.unified_score.final_score
            )
            .unwrap();
        }
        DebtItem::File(file) => {
            writeln!(
                output,
                "- #{} `{}` (Score: {:.1})",
                rank,
                file.metrics.path.display(),
                file.score
            )
            .unwrap();
        }
    }
}

fn format_debt_item_detailed(output: &mut String, item: &DebtItem, verbosity: u8) {
    match item {
        DebtItem::Function(func) => {
            format_function_debt_item(output, func, verbosity);
        }
        DebtItem::File(file) => {
            format_file_debt_item(output, file, verbosity);
        }
    }
}

fn format_function_debt_item(output: &mut String, item: &UnifiedDebtItem, verbosity: u8) {
    let score = item.unified_score.final_score;
    writeln!(
        output,
        "#### {} - Score: {:.1}",
        item.location.function, score
    )
    .unwrap();

    writeln!(
        output,
        "**Location:** `{}:{}`",
        item.location.file.display(),
        item.location.line
    )
    .unwrap();

    writeln!(output, "**Type:** {}", format_debt_type(&item.debt_type)).unwrap();

    writeln!(output, "**Action:** {}", item.recommendation.primary_action).unwrap();

    if let Some(complexity) = extract_complexity_info(&item.debt_type) {
        writeln!(output, "**Complexity:** {}", complexity).unwrap();
    }

    if verbosity >= 1 {
        writeln!(
            output,
            "**Impact:** {}",
            format_impact(&item.expected_impact)
        )
        .unwrap();
        writeln!(output, "**Why:** {}", item.recommendation.rationale).unwrap();
    }
}

fn format_file_debt_item(output: &mut String, item: &FileDebtItem, verbosity: u8) {
    let score = item.score;
    let file_name = item
        .metrics
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    writeln!(output, "#### {} - Score: {:.1}", file_name, score).unwrap();

    writeln!(
        output,
        "**File:** `{}` ({} lines, {} functions)",
        item.metrics.path.display(),
        item.metrics.total_lines,
        item.metrics.function_count
    )
    .unwrap();

    if item.metrics.god_object_indicators.is_god_object {
        writeln!(output, "**Type:** GOD OBJECT").unwrap();
        writeln!(
            output,
            "**Metrics:** {} methods, {} fields, {} responsibilities",
            item.metrics.god_object_indicators.methods_count,
            item.metrics.god_object_indicators.fields_count,
            item.metrics.god_object_indicators.responsibilities
        )
        .unwrap();
    } else if item.metrics.total_lines > 500 {
        writeln!(output, "**Type:** LARGE FILE").unwrap();
    } else {
        writeln!(output, "**Type:** COMPLEX FILE").unwrap();
    }

    writeln!(output, "**Recommendation:** {}", item.recommendation).unwrap();

    if verbosity >= 1 {
        writeln!(output, "**Impact:** {}", format_file_impact(&item.impact)).unwrap();
    }
}

fn format_mixed_priority_item_markdown(
    output: &mut String,
    rank: usize,
    item: &DebtItem,
    verbosity: u8,
) {
    match item {
        DebtItem::Function(func_item) => {
            format_priority_item_markdown(output, rank, func_item, verbosity);
        }
        DebtItem::File(file_item) => {
            format_file_priority_item_markdown(output, rank, file_item, verbosity);
        }
    }
}

fn format_file_priority_item_markdown(
    output: &mut String,
    rank: usize,
    item: &FileDebtItem,
    verbosity: u8,
) {
    let severity = get_severity_label(item.score);

    // Determine file type
    let type_label = if item.metrics.god_object_indicators.is_god_object {
        "FILE - GOD OBJECT"
    } else if item.metrics.total_lines > 500 {
        "FILE - HIGH COMPLEXITY"
    } else {
        "FILE"
    };

    // File items (god objects) are always T1 Critical Architecture
    let tier_label = "[T1] ";

    // Header with rank, tier, and score
    writeln!(
        output,
        "### #{} {}Score: {:.1} [{}]",
        rank, tier_label, item.score, severity
    )
    .unwrap();

    writeln!(output, "**Type:** {}", type_label).unwrap();
    writeln!(
        output,
        "**File:** `{}` ({} lines, {} functions)",
        item.metrics.path.display(),
        item.metrics.total_lines,
        item.metrics.function_count
    )
    .unwrap();

    // God object details if applicable
    if item.metrics.god_object_indicators.is_god_object {
        writeln!(output, "**God Object Metrics:**").unwrap();
        writeln!(
            output,
            "- Methods: {}",
            item.metrics.god_object_indicators.methods_count
        )
        .unwrap();
        writeln!(
            output,
            "- Fields: {}",
            item.metrics.god_object_indicators.fields_count
        )
        .unwrap();
        writeln!(
            output,
            "- Responsibilities: {}",
            item.metrics.god_object_indicators.responsibilities
        )
        .unwrap();
        writeln!(
            output,
            "- God Object Score: {:.1}",
            item.metrics.god_object_indicators.god_object_score
        )
        .unwrap();

        // Show coverage data if available
        if item.metrics.coverage_percent > 0.0 {
            writeln!(
                output,
                "- Test Coverage: {:.1}% ({} uncovered lines)",
                item.metrics.coverage_percent, item.metrics.uncovered_lines
            )
            .unwrap();
        }
    }

    writeln!(output, "**Recommendation:** {}", item.recommendation).unwrap();

    writeln!(output, "**Impact:** {}", format_file_impact(&item.impact)).unwrap();

    if verbosity >= 1 {
        writeln!(output, "\n**Scoring Breakdown:**").unwrap();
        writeln!(
            output,
            "- File size: {}",
            score_category(item.metrics.total_lines)
        )
        .unwrap();
        writeln!(
            output,
            "- Functions: {}",
            function_category(item.metrics.function_count)
        )
        .unwrap();
        writeln!(
            output,
            "- Complexity: {}",
            complexity_category(item.metrics.avg_complexity)
        )
        .unwrap();
        if item.metrics.function_count > 0 {
            writeln!(
                output,
                "- Dependencies: {} functions may have complex interdependencies",
                item.metrics.function_count
            )
            .unwrap();
        }
    }
}

fn score_category(lines: usize) -> &'static str {
    match lines {
        0..=200 => "LOW",
        201..=500 => "MODERATE",
        501..=1000 => "HIGH",
        _ => "CRITICAL",
    }
}

fn function_category(count: usize) -> &'static str {
    match count {
        0..=10 => "LOW",
        11..=20 => "MODERATE",
        21..=50 => "HIGH",
        _ => "EXCESSIVE",
    }
}

fn complexity_category(avg: f64) -> &'static str {
    match avg as usize {
        0..=5 => "LOW",
        6..=10 => "MODERATE",
        11..=20 => "HIGH",
        _ => "VERY HIGH",
    }
}

fn format_file_impact(impact: &crate::priority::FileImpact) -> String {
    let mut parts = vec![];

    if impact.complexity_reduction > 0.0 {
        parts.push(format!(
            "Reduce complexity by {:.0}%",
            impact.complexity_reduction
        ));
    }
    if impact.test_effort > 0.0 {
        parts.push(format!("Test effort: {:.1}", impact.test_effort));
    }
    if impact.maintainability_improvement > 0.0 {
        parts.push("Enable parallel development".to_string());
    }

    if parts.is_empty() {
        "No measurable impact".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_priority_item_markdown(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    let severity = get_severity_label(item.unified_score.final_score);

    // Header with rank, tier, and score
    let tier_label = item
        .tier
        .as_ref()
        .map(|t| format!("[{}] ", t.short_label()))
        .unwrap_or_default();

    writeln!(
        output,
        "### #{} {}Score: {:.1} [{}]",
        rank, tier_label, item.unified_score.final_score, severity
    )
    .unwrap();

    // Show score breakdown for verbosity >= 2
    if verbosity >= 2 {
        output.push_str(&format_score_breakdown_with_coverage(
            &item.unified_score,
            item.transitive_coverage.as_ref(),
        ));
    } else if verbosity >= 1 {
        // Show main contributing factors for verbosity >= 1
        output.push_str(&format_main_factors_with_coverage(
            &item.unified_score,
            &item.debt_type,
            item.transitive_coverage.as_ref(),
        ));
    }

    // Location and type
    writeln!(
        output,
        "**Type:** {} | **Location:** `{}:{} {}()`",
        format_debt_type(&item.debt_type),
        item.location.file.display(),
        item.location.line,
        item.location.function
    )
    .unwrap();

    // Action and impact
    writeln!(output, "**Action:** {}", item.recommendation.primary_action).unwrap();
    writeln!(
        output,
        "**Impact:** {}",
        format_impact(&item.expected_impact)
    )
    .unwrap();

    // Complexity details
    if let Some(complexity) = extract_complexity_info(&item.debt_type) {
        writeln!(output, "**Complexity:** {}", complexity).unwrap();
    }

    // Dependencies
    if verbosity >= 1 {
        writeln!(output, "\n#### Dependencies").unwrap();
        writeln!(
            output,
            "- **Upstream:** {} | **Downstream:** {}",
            item.upstream_dependencies, item.downstream_dependencies
        )
        .unwrap();

        if !item.upstream_callers.is_empty() && verbosity >= 2 {
            let caller_info = format_dependency_list(&item.upstream_callers, 3, "Called by");
            if !caller_info.is_empty() {
                writeln!(output, "{}", caller_info).unwrap();
            }
        }

        if !item.downstream_callees.is_empty() && verbosity >= 2 {
            let callee_info = format_dependency_list(&item.downstream_callees, 3, "Calls");
            if !callee_info.is_empty() {
                writeln!(output, "{}", callee_info).unwrap();
            }
        }
    }

    // Rationale
    writeln!(output, "\n**Why:** {}", item.recommendation.rationale).unwrap();
}

fn get_severity_label(score: f64) -> &'static str {
    match score {
        s if s >= 9.0 => "CRITICAL",
        s if s >= 7.0 => "HIGH",
        s if s >= 5.0 => "MEDIUM",
        s if s >= 3.0 => "LOW",
        _ => "MINIMAL",
    }
}

fn format_debt_type(debt_type: &DebtType) -> &'static str {
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
        // Resource Management debt types
        DebtType::AllocationInefficiency { .. } => "Allocation Inefficiency",
        DebtType::StringConcatenation { .. } => "String Concatenation",
        DebtType::NestedLoops { .. } => "Nested Loops",
        DebtType::BlockingIO { .. } => "Blocking I/O",
        DebtType::SuboptimalDataStructure { .. } => "Suboptimal Data Structure",
        // Organization debt types
        DebtType::GodObject { .. } => "God Object",
        DebtType::GodModule { .. } => "God Module",
        DebtType::FeatureEnvy { .. } => "Feature Envy",
        DebtType::PrimitiveObsession { .. } => "Primitive Obsession",
        DebtType::MagicValues { .. } => "Magic Values",
        // Testing quality debt types
        DebtType::AssertionComplexity { .. } => "Assertion Complexity",
        DebtType::FlakyTestPattern { .. } => "Flaky Test Pattern",
        // Resource management debt types
        DebtType::AsyncMisuse { .. } => "Async Misuse",
        DebtType::ResourceLeak { .. } => "Resource Leak",
        DebtType::CollectionInefficiency { .. } => "Collection Inefficiency",
    }
}

fn format_impact(impact: &crate::priority::ImpactMetrics) -> String {
    let mut parts = vec![];

    if impact.complexity_reduction > 0.0 {
        parts.push(format!("-{:.1} complexity", impact.complexity_reduction));
    }
    if impact.risk_reduction > 0.1 {
        parts.push(format!("-{:.1} risk", impact.risk_reduction));
    }
    if impact.coverage_improvement > 0.01 {
        parts.push(format!("+{:.0}% coverage", impact.coverage_improvement));
    }
    if impact.lines_reduction > 0 {
        parts.push(format!("-{} lines", impact.lines_reduction));
    }

    if parts.is_empty() {
        "No measurable impact".to_string()
    } else {
        parts.join(", ")
    }
}

fn extract_complexity_info(debt_type: &DebtType) -> Option<String> {
    match debt_type {
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        }
        | DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive,
            ..
        } => Some(format!(
            "cyclomatic={}, cognitive={}",
            cyclomatic, cognitive
        )),
        DebtType::TestingGap {
            cyclomatic,
            cognitive,
            ..
        } => Some(format!(
            "cyclomatic={}, cognitive={}",
            cyclomatic, cognitive
        )),
        DebtType::Risk { .. } => None,
        DebtType::DeadCode {
            cyclomatic,
            cognitive,
            ..
        } => Some(format!(
            "cyclomatic={}, cognitive={}",
            cyclomatic, cognitive
        )),
        _ => None,
    }
}

fn format_score_breakdown_with_coverage(
    unified_score: &crate::priority::UnifiedScore,
    transitive_coverage: Option<&crate::priority::coverage_propagation::TransitiveCoverage>,
) -> String {
    let weights = crate::config::get_scoring_weights();
    let mut output = String::new();

    writeln!(&mut output, "\n#### Score Calculation\n").unwrap();
    writeln!(
        &mut output,
        "| Component | Value | Weight | Contribution | Details |"
    )
    .unwrap();
    writeln!(
        &mut output,
        "|-----------|-------|--------|--------------|----------|"
    )
    .unwrap();
    writeln!(
        &mut output,
        "| Complexity | {:.1} | {:.0}% | {:.2} | |",
        unified_score.complexity_factor,
        weights.complexity * 100.0,
        unified_score.complexity_factor * weights.complexity
    )
    .unwrap();

    // Add coverage details if available
    let coverage_details = if let Some(trans_cov) = transitive_coverage {
        format!("Line: {:.2}%", trans_cov.direct * 100.0)
    } else {
        "No data".to_string()
    };
    writeln!(
        &mut output,
        "| Coverage | {:.1} | {:.0}% | {:.2} | {} |",
        unified_score.coverage_factor,
        weights.coverage * 100.0,
        unified_score.coverage_factor * weights.coverage,
        coverage_details
    )
    .unwrap();
    // Semantic and ROI factors removed per spec 55 and 58
    writeln!(
        &mut output,
        "| Dependency | {:.1} | {:.0}% | {:.2} | |",
        unified_score.dependency_factor,
        weights.dependency * 100.0,
        unified_score.dependency_factor * weights.dependency
    )
    .unwrap();

    // Organization factor removed per spec 58 - redundant with complexity factor

    // New weights after removing security: complexity, coverage, dependency
    let base_score = unified_score.complexity_factor * weights.complexity
        + unified_score.coverage_factor * weights.coverage
        + unified_score.dependency_factor * weights.dependency;

    writeln!(&mut output).unwrap();
    writeln!(&mut output, "- **Base Score:** {:.2}", base_score).unwrap();
    writeln!(
        &mut output,
        "- **Role Adjustment:** ×{:.2}",
        unified_score.role_multiplier
    )
    .unwrap();
    writeln!(
        &mut output,
        "- **Final Score:** {:.2}",
        unified_score.final_score
    )
    .unwrap();
    writeln!(&mut output).unwrap();

    output
}

fn format_main_factors_with_coverage(
    unified_score: &crate::priority::UnifiedScore,
    debt_type: &crate::priority::DebtType,
    transitive_coverage: Option<&crate::priority::coverage_propagation::TransitiveCoverage>,
) -> String {
    let weights = crate::config::get_scoring_weights();
    let mut factors = vec![];

    // Show coverage info - both good and bad coverage are important factors
    if let Some(trans_cov) = transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        if coverage_pct >= 95.0 {
            factors.push(format!("Excellent coverage {:.1}%", coverage_pct));
        } else if coverage_pct >= 80.0 {
            factors.push(format!("Good coverage {:.1}%", coverage_pct));
        } else if unified_score.coverage_factor > 3.0 {
            factors.push(format!(
                "Line coverage {:.1}% (weight: {:.0}%)",
                coverage_pct,
                weights.coverage * 100.0
            ));
        }
    } else if unified_score.coverage_factor > 3.0 {
        factors.push(format!(
            "No coverage data (weight: {:.0}%)",
            weights.coverage * 100.0
        ));
    }
    if unified_score.complexity_factor > 5.0 {
        factors.push(format!(
            "Complexity (weight: {:.0}%)",
            weights.complexity * 100.0
        ));
    } else if unified_score.complexity_factor > 3.0 {
        factors.push("Moderate complexity".to_string());
    }

    if unified_score.dependency_factor > 5.0 {
        factors.push(format!(
            "Critical path (weight: {:.0}%)",
            weights.dependency * 100.0
        ));
    }
    // Organization factor removed per spec 58 - redundant with complexity factor

    // Add specific factors for various debt types
    match debt_type {
        crate::priority::DebtType::NestedLoops { depth, .. } => {
            factors.push("Complexity impact (High)".to_string());
            factors.push(format!("{} level nested loops", depth));
        }
        crate::priority::DebtType::BlockingIO { operation, .. } => {
            factors.push("Resource management issue".to_string());
            factors.push(format!("Blocking {}", operation));
        }
        crate::priority::DebtType::AllocationInefficiency { pattern, .. } => {
            factors.push("Resource management issue".to_string());
            factors.push(format!("Allocation: {}", pattern));
        }
        _ => {} // No additional factors for other debt types
    }

    if !factors.is_empty() {
        format!("*Main factors: {}*\n", factors.join(", "))
    } else {
        String::new()
    }
}

fn format_dependency_list(items: &[String], max_shown: usize, list_type: &str) -> String {
    if items.is_empty() {
        return String::new();
    }

    let list = if items.len() > max_shown {
        format!(
            "{}, ... ({} more)",
            items
                .iter()
                .take(max_shown)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            items.len() - max_shown
        )
    } else {
        items.to_vec().join(", ")
    };

    format!("- **{}:** {}", list_type, list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        ActionableRecommendation, FunctionRole, FunctionVisibility, ImpactMetrics, Location,
        TransitiveCoverage, UnifiedDebtItem, UnifiedScore,
    };

    #[test]
    fn test_get_severity_label() {
        assert_eq!(get_severity_label(10.0), "CRITICAL");
        assert_eq!(get_severity_label(9.5), "CRITICAL");
        assert_eq!(get_severity_label(9.0), "CRITICAL");
        assert_eq!(get_severity_label(8.0), "HIGH");
        assert_eq!(get_severity_label(7.0), "HIGH");
        assert_eq!(get_severity_label(6.0), "MEDIUM");
        assert_eq!(get_severity_label(5.0), "MEDIUM");
        assert_eq!(get_severity_label(4.0), "LOW");
        assert_eq!(get_severity_label(3.0), "LOW");
        assert_eq!(get_severity_label(2.0), "MINIMAL");
        assert_eq!(get_severity_label(0.5), "MINIMAL");
    }

    #[test]
    fn test_format_debt_type() {
        let test_gap = DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 20,
        };
        assert_eq!(format_debt_type(&test_gap), "Testing Gap");

        let complexity = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 30,
        };
        assert_eq!(format_debt_type(&complexity), "Complexity");

        let dead_code = DebtType::DeadCode {
            visibility: FunctionVisibility::Private,
            cyclomatic: 5,
            cognitive: 10,
            usage_hints: vec![],
        };
        assert_eq!(format_debt_type(&dead_code), "Dead Code");
    }

    #[test]
    fn test_format_impact_with_all_metrics() {
        let impact = ImpactMetrics {
            complexity_reduction: 5.5,
            risk_reduction: 0.3,
            coverage_improvement: 15.5,
            lines_reduction: 25,
        };

        let result = format_impact(&impact);
        assert!(result.contains("-5.5 complexity"));
        assert!(result.contains("-0.3 risk"));
        // 15.5 rounds to 16 with {:.0} formatting
        assert!(result.contains("+16% coverage"));
        assert!(result.contains("-25 lines"));
    }

    #[test]
    fn test_format_impact_with_no_metrics() {
        let impact = ImpactMetrics {
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
            coverage_improvement: 0.0,
            lines_reduction: 0,
        };

        let result = format_impact(&impact);
        assert_eq!(result, "No measurable impact");
    }

    #[test]
    fn test_format_impact_with_partial_metrics() {
        let impact = ImpactMetrics {
            complexity_reduction: 3.0,
            risk_reduction: 0.05,        // Below threshold
            coverage_improvement: 0.005, // Below threshold
            lines_reduction: 10,
        };

        let result = format_impact(&impact);
        assert!(result.contains("-3.0 complexity"));
        assert!(!result.contains("risk"));
        assert!(!result.contains("coverage"));
        assert!(result.contains("-10 lines"));
    }

    #[test]
    fn test_extract_complexity_info() {
        let complexity_hotspot = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 30,
        };
        assert_eq!(
            extract_complexity_info(&complexity_hotspot),
            Some("cyclomatic=15, cognitive=30".to_string())
        );

        let test_gap = DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 20,
        };
        assert_eq!(
            extract_complexity_info(&test_gap),
            Some("cyclomatic=10, cognitive=20".to_string())
        );

        let dead_code = DebtType::DeadCode {
            visibility: FunctionVisibility::Private,
            cyclomatic: 5,
            cognitive: 10,
            usage_hints: vec![],
        };
        assert_eq!(
            extract_complexity_info(&dead_code),
            Some("cyclomatic=5, cognitive=10".to_string())
        );

        let risk = DebtType::Risk {
            risk_score: 8.5,
            factors: vec!["complex".to_string()],
        };
        assert_eq!(extract_complexity_info(&risk), None);
    }

    #[test]
    fn test_format_score_breakdown() {
        let score = UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 8.0,
            dependency_factor: 4.0,
            role_multiplier: 1.2,
            final_score: 8.5,
            pre_adjustment_score: None,
            adjustment_applied: None,
        };

        let result = format_score_breakdown_with_coverage(&score, None);

        // Check for table headers
        assert!(result.contains("Score Calculation"));
        assert!(result.contains("| Component | Value | Weight | Contribution |"));

        // Check for component rows
        assert!(result.contains("| Complexity | 5.0"));
        assert!(result.contains("| Coverage | 8.0"));
        // ROI and Semantic removed from scoring per spec 55 and 58
        assert!(result.contains("| Dependency | 4.0"));

        // Check for summary lines (with markdown formatting)
        assert!(result.contains("**Base Score:**"));
        assert!(result.contains("**Role Adjustment:** ×1.20"));
        assert!(result.contains("**Final Score:** 8.50"));
    }

    #[test]
    fn test_format_main_factors_with_multiple_factors() {
        let score = UnifiedScore {
            complexity_factor: 6.0, // Above threshold
            coverage_factor: 4.0,   // Above threshold
            dependency_factor: 6.0, // Above threshold
            role_multiplier: 1.0,
            final_score: 7.0,
            pre_adjustment_score: None,
            adjustment_applied: None,
        };

        let debt_type = DebtType::Risk {
            risk_score: 5.0,
            factors: vec!["Test factor".to_string()],
        };

        let result = format_main_factors_with_coverage(&score, &debt_type, None);

        assert!(result.contains("Main factors:"));
        assert!(result.contains("No coverage data") || result.contains("Line coverage"));
        // ROI removed from scoring per spec 55 and 58
        assert!(result.contains("Critical path"));
        assert!(result.contains("Complexity"));
    }

    #[test]
    fn test_format_main_factors_with_no_factors() {
        let score = UnifiedScore {
            complexity_factor: 2.0, // Below all thresholds
            coverage_factor: 2.0,
            dependency_factor: 2.0,
            role_multiplier: 1.0,
            final_score: 2.0,
            pre_adjustment_score: None,
            adjustment_applied: None,
        };

        let debt_type = DebtType::Risk {
            risk_score: 1.0,
            factors: vec!["Test factor".to_string()],
        };

        let result = format_main_factors_with_coverage(&score, &debt_type, None);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_dependency_list_empty() {
        let items: Vec<String> = vec![];
        let result = format_dependency_list(&items, 3, "Called by");
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_dependency_list_few_items() {
        let items = vec!["func1".to_string(), "func2".to_string()];
        let result = format_dependency_list(&items, 3, "Called by");
        assert_eq!(result, "- **Called by:** func1, func2");
    }

    #[test]
    fn test_format_dependency_list_many_items() {
        let items = vec![
            "func1".to_string(),
            "func2".to_string(),
            "func3".to_string(),
            "func4".to_string(),
            "func5".to_string(),
        ];
        let result = format_dependency_list(&items, 3, "Calls");
        assert_eq!(result, "- **Calls:** func1, func2, func3, ... (2 more)");
    }

    #[test]
    fn test_format_dependency_list_exactly_max() {
        let items = vec![
            "func1".to_string(),
            "func2".to_string(),
            "func3".to_string(),
        ];
        let result = format_dependency_list(&items, 3, "Dependencies");
        assert_eq!(result, "- **Dependencies:** func1, func2, func3");
    }

    // Helper function to create test UnifiedDebtItem
    fn create_test_debt_item() -> UnifiedDebtItem {
        use std::path::PathBuf;

        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                line: 100,
                function: "test_function".to_string(),
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
            unified_score: UnifiedScore {
                complexity_factor: 7.0,
                coverage_factor: 8.0,
                dependency_factor: 6.0,
                role_multiplier: 1.2,
                final_score: 8.5,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor complex function".to_string(),
                rationale: "High complexity makes it hard to maintain".to_string(),
                implementation_steps: vec![
                    "Extract helper functions".to_string(),
                    "Add unit tests".to_string(),
                ],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 5.0,
                risk_reduction: 0.2,
                coverage_improvement: 25.0,
                lines_reduction: 30,
            },
            transitive_coverage: Some(TransitiveCoverage {
                direct: 0.45,
                transitive: 0.55,
                propagated_from: vec![],
                uncovered_lines: vec![101, 102, 103],
            }),
            upstream_dependencies: 3,
            downstream_dependencies: 5,
            upstream_callers: vec![
                "caller1".to_string(),
                "caller2".to_string(),
                "caller3".to_string(),
            ],
            downstream_callees: vec!["callee1".to_string(), "callee2".to_string()],
            nesting_depth: 3,
            function_length: 150,
            cyclomatic_complexity: 15,
            cognitive_complexity: 25,
            entropy_details: None,
            is_pure: None,
            purity_confidence: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
        }
    }

    #[test]
    fn test_format_priority_item_markdown_minimal_verbosity() {
        let mut output = String::new();
        let item = create_test_debt_item();

        format_priority_item_markdown(&mut output, 1, &item, 0);

        // Check basic elements are present
        assert!(output.contains("### #1 Score: 8.5 [HIGH]"));
        assert!(output.contains("**Type:** Complexity"));
        assert!(output.contains("**Location:** `test.rs:100 test_function()`"));
        assert!(output.contains("**Action:** Refactor complex function"));
        assert!(output.contains("**Impact:**"));
        assert!(output.contains("**Complexity:** cyclomatic=15, cognitive=25"));
        assert!(output.contains("**Why:** High complexity makes it hard to maintain"));

        // Should NOT include score breakdown or dependencies at verbosity 0
        assert!(!output.contains("#### Dependencies"));
        assert!(!output.contains("Coverage Gap"));
    }

    #[test]
    fn test_format_priority_item_markdown_verbosity_1() {
        let mut output = String::new();
        let item = create_test_debt_item();

        format_priority_item_markdown(&mut output, 2, &item, 1);

        // Should include main factors but not full breakdown
        assert!(output.contains("### #2 Score: 8.5 [HIGH]"));
        assert!(output.contains("Main factors"));

        // Should include dependencies section
        assert!(output.contains("#### Dependencies"));
        assert!(output.contains("**Upstream:** 3 | **Downstream:** 5"));

        // Should NOT include caller/callee lists at verbosity 1
        assert!(!output.contains("Called by"));
        assert!(!output.contains("Calls"));
    }

    #[test]
    fn test_format_priority_item_markdown_verbosity_2() {
        let mut output = String::new();
        let item = create_test_debt_item();

        format_priority_item_markdown(&mut output, 3, &item, 2);

        // Should include full score breakdown
        assert!(output.contains("Score Calculation"));
        assert!(output.contains("Component"));
        assert!(output.contains("Complexity"));

        // Should include dependencies with detailed lists
        assert!(output.contains("#### Dependencies"));
        assert!(output.contains("**Upstream:** 3 | **Downstream:** 5"));
        assert!(output.contains("Called by"));
        assert!(output.contains("caller1, caller2, caller3"));
        assert!(output.contains("Calls"));
        assert!(output.contains("callee1, callee2"));
    }

    #[test]
    fn test_format_priority_item_markdown_critical_score() {
        let mut output = String::new();
        let mut item = create_test_debt_item();
        item.unified_score.final_score = 9.5;

        format_priority_item_markdown(&mut output, 1, &item, 0);

        assert!(output.contains("[CRITICAL]"));
    }

    #[test]
    fn test_format_priority_item_markdown_low_score() {
        let mut output = String::new();
        let mut item = create_test_debt_item();
        item.unified_score.final_score = 3.5;

        format_priority_item_markdown(&mut output, 1, &item, 0);

        assert!(output.contains("[LOW]"));
    }

    #[test]
    fn test_format_priority_item_markdown_no_complexity() {
        let mut output = String::new();
        let mut item = create_test_debt_item();
        item.debt_type = DebtType::Risk {
            risk_score: 7.5,
            factors: vec!["Factor1".to_string()],
        };

        format_priority_item_markdown(&mut output, 1, &item, 0);

        // Should not have complexity section for Risk type
        assert!(!output.contains("**Complexity:**"));
        assert!(output.contains("**Type:** Risk"));
    }

    #[test]
    fn test_format_priority_item_markdown_empty_dependencies() {
        let mut output = String::new();
        let mut item = create_test_debt_item();
        item.upstream_callers.clear();
        item.downstream_callees.clear();

        format_priority_item_markdown(&mut output, 1, &item, 2);

        // Should still show dependency counts but no lists
        assert!(output.contains("**Upstream:** 3 | **Downstream:** 5"));
        assert!(!output.contains("Called by"));
        assert!(!output.contains("Calls"));
    }

    #[test]
    fn test_format_priority_item_markdown_large_rank() {
        let mut output = String::new();
        let item = create_test_debt_item();

        format_priority_item_markdown(&mut output, 999, &item, 0);

        assert!(output.contains("### #999 Score:"));
    }

    #[test]
    fn test_format_priority_item_markdown_no_transitive_coverage() {
        let mut output = String::new();
        let mut item = create_test_debt_item();
        item.transitive_coverage = None;

        format_priority_item_markdown(&mut output, 1, &item, 2);

        // Should still work without transitive coverage
        assert!(output.contains("### #1 Score: 8.5"));
        // Coverage information should be omitted in breakdown
    }

    #[test]
    fn test_format_file_priority_item_markdown_basic() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/main.rs"),
                total_lines: 250,
                function_count: 10,
                class_count: 2,
                avg_complexity: 5.5,
                max_complexity: 12,
                total_complexity: 55,
                coverage_percent: 0.75,
                uncovered_lines: 25,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 5,
                    fields_count: 3,
                    responsibilities: 2,
                    is_god_object: false,
                    god_object_score: 0.0,
                    responsibility_names: Vec::new(),
                    recommended_splits: Vec::new(),
                    module_structure: None,
                
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,},
                function_scores: vec![10.0, 8.0, 6.0],
                god_object_type: None,
            },
            score: 45.2,
            priority_rank: 1,
            recommendation: "Refactor complex functions".to_string(),
            impact: FileImpact {
                complexity_reduction: 15.0,
                maintainability_improvement: 20.0,
                test_effort: 10.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 1, &item, 0);

        assert!(output.contains("### #1 [T1] Score: 45.2"));
        assert!(output.contains("**Type:** FILE"));
        assert!(output.contains("src/main.rs"));
        assert!(output.contains("250 lines, 10 functions"));
        assert!(output.contains("**Recommendation:** Refactor complex functions"));
        assert!(!output.contains("**God Object Metrics:**"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_god_object() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/god_class.rs"),
                total_lines: 800,
                function_count: 50,
                class_count: 1,
                avg_complexity: 8.5,
                max_complexity: 25,
                total_complexity: 425,
                coverage_percent: 0.60,
                uncovered_lines: 320,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 45,
                    fields_count: 20,
                    responsibilities: 8,
                    is_god_object: true,
                    god_object_score: 3.5,
                    responsibility_names: Vec::new(),
                    recommended_splits: Vec::new(),
                    module_structure: None,
                
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,},
                function_scores: vec![],
                god_object_type: None,
            },
            score: 125.8,
            priority_rank: 1,
            recommendation: "Split into multiple focused modules".to_string(),
            impact: FileImpact {
                complexity_reduction: 50.0,
                maintainability_improvement: 60.0,
                test_effort: 30.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 1, &item, 0);

        assert!(output.contains("### #1 [T1] Score: 125.8"));
        assert!(output.contains("**Type:** FILE - GOD OBJECT"));
        assert!(output.contains("**God Object Metrics:**"));
        assert!(output.contains("- Methods: 45"));
        assert!(output.contains("- Fields: 20"));
        assert!(output.contains("- Responsibilities: 8"));
        assert!(output.contains("- God Object Score: 3.5"));
        assert!(output.contains("**Recommendation:** Split into multiple focused modules"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_high_complexity() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/complex.rs"),
                total_lines: 600,
                function_count: 15,
                class_count: 3,
                avg_complexity: 12.0,
                max_complexity: 30,
                total_complexity: 180,
                coverage_percent: 0.50,
                uncovered_lines: 300,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 12,
                    fields_count: 8,
                    responsibilities: 4,
                    is_god_object: false,
                    god_object_score: 0.0,
                    responsibility_names: Vec::new(),
                    recommended_splits: Vec::new(),
                    module_structure: None,
                
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,},
                function_scores: vec![],
                god_object_type: None,
            },
            score: 85.3,
            priority_rank: 2,
            recommendation: "Reduce complexity and improve test coverage".to_string(),
            impact: FileImpact {
                complexity_reduction: 35.0,
                maintainability_improvement: 40.0,
                test_effort: 25.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 2, &item, 0);

        assert!(output.contains("### #2 [T1] Score: 85.3"));
        assert!(output.contains("**Type:** FILE - HIGH COMPLEXITY"));
        assert!(output.contains("600 lines"));
        assert!(!output.contains("**God Object Metrics:**"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_with_verbosity() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/verbose.rs"),
                total_lines: 350,
                function_count: 12,
                class_count: 2,
                avg_complexity: 7.5,
                max_complexity: 18,
                total_complexity: 90,
                coverage_percent: 0.65,
                uncovered_lines: 122,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 10,
                    fields_count: 5,
                    responsibilities: 3,
                    is_god_object: false,
                    god_object_score: 0.0,
                    responsibility_names: Vec::new(),
                    recommended_splits: Vec::new(),
                    module_structure: None,
                
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,},
                function_scores: vec![],
                god_object_type: None,
            },
            score: 55.7,
            priority_rank: 3,
            recommendation: "Consider refactoring".to_string(),
            impact: FileImpact {
                complexity_reduction: 20.0,
                maintainability_improvement: 25.0,
                test_effort: 15.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 3, &item, 1);

        assert!(output.contains("### #3 [T1] Score: 55.7"));
        assert!(output.contains("**Scoring Breakdown:**"));
        assert!(output.contains("- File size:"));
        assert!(output.contains("- Functions:"));
        assert!(output.contains("- Complexity:"));
        assert!(output.contains("- Dependencies: 12 functions may have complex interdependencies"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_zero_functions() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/empty.rs"),
                total_lines: 100,
                function_count: 0,
                class_count: 0,
                avg_complexity: 0.0,
                max_complexity: 0,
                total_complexity: 0,
                coverage_percent: 1.0,
                uncovered_lines: 0,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 0,
                    fields_count: 0,
                    responsibilities: 0,
                    is_god_object: false,
                    god_object_score: 0.0,
                    responsibility_names: Vec::new(),
                    recommended_splits: Vec::new(),
                    module_structure: None,
                
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,},
                function_scores: vec![],
                god_object_type: None,
            },
            score: 5.0,
            priority_rank: 10,
            recommendation: "No action needed".to_string(),
            impact: FileImpact {
                complexity_reduction: 0.0,
                maintainability_improvement: 0.0,
                test_effort: 0.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 10, &item, 1);

        assert!(output.contains("0 functions"));
        assert!(!output.contains("- Dependencies:"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_critical_severity() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/critical.rs"),
                total_lines: 1000,
                function_count: 60,
                class_count: 5,
                avg_complexity: 15.0,
                max_complexity: 40,
                total_complexity: 900,
                coverage_percent: 0.30,
                uncovered_lines: 700,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 55,
                    fields_count: 30,
                    responsibilities: 12,
                    is_god_object: true,
                    god_object_score: 5.0,
                    responsibility_names: Vec::new(),
                    recommended_splits: Vec::new(),
                    module_structure: None,
                
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,},
                function_scores: vec![],
                god_object_type: None,
            },
            score: 150.0,
            priority_rank: 1,
            recommendation: "Urgent refactoring required".to_string(),
            impact: FileImpact {
                complexity_reduction: 70.0,
                maintainability_improvement: 80.0,
                test_effort: 50.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 1, &item, 0);

        assert!(output.contains("[CRITICAL]"));
        assert!(output.contains("**Type:** FILE - GOD OBJECT"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_low_severity() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/simple.rs"),
                total_lines: 150,
                function_count: 5,
                class_count: 1,
                avg_complexity: 3.0,
                max_complexity: 5,
                total_complexity: 15,
                coverage_percent: 0.90,
                uncovered_lines: 15,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 4,
                    fields_count: 2,
                    responsibilities: 1,
                    is_god_object: false,
                    god_object_score: 0.0,
                    responsibility_names: Vec::new(),
                    recommended_splits: Vec::new(),
                    module_structure: None,
                
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,},
                function_scores: vec![],
                god_object_type: None,
            },
            score: 18.5,
            priority_rank: 15,
            recommendation: "Good state, minor improvements possible".to_string(),
            impact: FileImpact {
                complexity_reduction: 5.0,
                maintainability_improvement: 8.0,
                test_effort: 3.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 15, &item, 0);

        assert!(output.contains("[CRITICAL]")); // Score 18.5 is CRITICAL (>=9.0)
        assert!(output.contains("**Type:** FILE"));
    }

    #[test]
    fn test_format_priorities_tiered_markdown() {
        use crate::priority::CallGraph;
        use std::path::PathBuf;

        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Add items with various scores to test tiering
        let critical_item = UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("critical.rs"),
                line: 10,
                function: "critical_func".to_string(),
            },
            debt_type: DebtType::GodObject {
                methods: 10,
                fields: 5,
                responsibilities: 10,
                god_object_score: 95.0,
            },
            unified_score: UnifiedScore {
                complexity_factor: 10.0,
                coverage_factor: 10.0,
                dependency_factor: 10.0,
                role_multiplier: 1.0,
                final_score: 95.0,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::Unknown,
            recommendation: ActionableRecommendation {
                primary_action: "Split into multiple classes".to_string(),
                rationale: "God object detected".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 50.0,
                risk_reduction: 5.0,
                coverage_improvement: 20.0,
                lines_reduction: 500,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 20,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 1000,
            cyclomatic_complexity: 50,
            cognitive_complexity: 100,
            entropy_details: None,
            is_pure: Some(false),
            purity_confidence: Some(1.0),
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
        };

        let high_item = UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("high.rs"),
                line: 20,
                function: "high_func".to_string(),
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 30,
                cognitive: 40,
            },
            unified_score: UnifiedScore {
                complexity_factor: 8.0,
                coverage_factor: 7.0,
                dependency_factor: 6.0,
                role_multiplier: 1.0,
                final_score: 75.0,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor complex function".to_string(),
                rationale: "High complexity".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 20.0,
                risk_reduction: 2.0,
                coverage_improvement: 10.0,
                lines_reduction: 100,
            },
            transitive_coverage: None,
            upstream_dependencies: 2,
            downstream_dependencies: 5,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 3,
            function_length: 200,
            cyclomatic_complexity: 30,
            cognitive_complexity: 40,
            entropy_details: None,
            is_pure: Some(false),
            purity_confidence: Some(0.8),
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
        };

        let moderate_item = UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("moderate.rs"),
                line: 30,
                function: "moderate_func".to_string(),
            },
            debt_type: DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 15,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 6.0,
                dependency_factor: 4.0,
                role_multiplier: 1.0,
                final_score: 55.0,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Add unit tests".to_string(),
                rationale: "No test coverage".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 0.0,
                risk_reduction: 1.0,
                coverage_improvement: 50.0,
                lines_reduction: 0,
            },
            transitive_coverage: None,
            upstream_dependencies: 1,
            downstream_dependencies: 2,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 50,
            cyclomatic_complexity: 10,
            cognitive_complexity: 15,
            entropy_details: None,
            is_pure: Some(true),
            purity_confidence: Some(0.9),
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
        };

        let low_item = UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("low.rs"),
                line: 40,
                function: "low_func".to_string(),
            },
            debt_type: DebtType::DeadCode {
                visibility: FunctionVisibility::Private,
                cyclomatic: 5,
                cognitive: 8,
                usage_hints: vec![],
            },
            unified_score: UnifiedScore {
                complexity_factor: 2.0,
                coverage_factor: 3.0,
                dependency_factor: 2.0,
                role_multiplier: 1.0,
                final_score: 25.0,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::Unknown,
            recommendation: ActionableRecommendation {
                primary_action: "Remove dead code".to_string(),
                rationale: "Function is not used".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 5.0,
                risk_reduction: 0.1,
                coverage_improvement: 0.0,
                lines_reduction: 30,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 30,
            cyclomatic_complexity: 5,
            cognitive_complexity: 8,
            entropy_details: None,
            is_pure: Some(false),
            purity_confidence: Some(0.5),
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
        };

        analysis.add_item(critical_item);
        analysis.add_item(high_item);
        analysis.add_item(moderate_item);
        analysis.add_item(low_item);
        analysis.sort_by_priority();
        analysis.calculate_total_impact();

        let output = format_priorities_tiered_markdown(&analysis, 10, 0);

        // Check that all tier headers are present (if not empty)
        assert!(output.contains("[CRITICAL] CRITICAL"));
        assert!(output.contains("[WARN] HIGH"));
        assert!(output.contains("MODERATE"));
        assert!(output.contains("[INFO] LOW"));

        // Check that items are in the right sections
        assert!(output.contains("critical_func"));
        assert!(output.contains("high_func"));
        assert!(output.contains("moderate_func"));
        assert!(output.contains("low_func"));

        // Check effort estimates
        assert!(output.contains("1-2 days per item"));
        assert!(output.contains("2-4 hours per item"));

        // Check summary section
        assert!(output.contains("## Summary"));
        assert!(output.contains("Total Debt Items:"));
    }

    #[test]
    fn test_tier_classification() {
        assert_eq!(Tier::from_score(95.0), Tier::Critical);
        assert_eq!(Tier::from_score(90.0), Tier::Critical);
        assert_eq!(Tier::from_score(89.9), Tier::High);
        assert_eq!(Tier::from_score(75.0), Tier::High);
        assert_eq!(Tier::from_score(70.0), Tier::High);
        assert_eq!(Tier::from_score(69.9), Tier::Moderate);
        assert_eq!(Tier::from_score(55.0), Tier::Moderate);
        assert_eq!(Tier::from_score(50.0), Tier::Moderate);
        assert_eq!(Tier::from_score(49.9), Tier::Low);
        assert_eq!(Tier::from_score(25.0), Tier::Low);
        assert_eq!(Tier::from_score(0.0), Tier::Low);
    }

    #[test]
    fn test_tier_headers() {
        assert_eq!(
            Tier::Critical.header(),
            "[CRITICAL] CRITICAL - Immediate Action Required"
        );
        assert_eq!(Tier::High.header(), "[WARN] HIGH - Current Sprint Priority");
        assert_eq!(Tier::Moderate.header(), "MODERATE - Next Sprint Planning");
        assert_eq!(Tier::Low.header(), "[INFO] LOW - Backlog Consideration");
    }

    #[test]
    fn test_format_file_priority_item_markdown_extreme_verbosity() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/detailed.rs"),
                total_lines: 750,
                function_count: 25,
                class_count: 4,
                avg_complexity: 9.2,
                max_complexity: 22,
                total_complexity: 230,
                coverage_percent: 0.55,
                uncovered_lines: 337,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 20,
                    fields_count: 12,
                    responsibilities: 5,
                    is_god_object: false,
                    god_object_score: 0.0,
                    responsibility_names: Vec::new(),
                    recommended_splits: Vec::new(),
                    module_structure: None,
                
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,},
                function_scores: vec![],
                god_object_type: None,
            },
            score: 72.4,
            priority_rank: 4,
            recommendation: "Significant refactoring recommended".to_string(),
            impact: FileImpact {
                complexity_reduction: 40.0,
                maintainability_improvement: 45.0,
                test_effort: 28.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 4, &item, 2);

        // With verbosity 2, should include all details
        assert!(output.contains("**Scoring Breakdown:**"));
        assert!(output.contains("- File size:"));
        assert!(output.contains("HIGH")); // 750 lines is HIGH category
        assert!(output.contains("- Functions:"));
        assert!(output.contains("HIGH")); // 25 functions is HIGH category
        assert!(output.contains("- Complexity:"));
        assert!(output.contains("MODERATE")); // avg 9.2 is MODERATE category
    }
}
