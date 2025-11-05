use crate::priority::unified_scorer::EntropyDetails;
use crate::priority::{DebtType, FunctionVisibility, UnifiedDebtItem};

// Pure function to create formatting context
pub(crate) fn create_format_context(
    rank: usize,
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
) -> FormatContext {
    FormatContext {
        rank,
        score: item.unified_score.final_score,
        severity_info: SeverityInfo::from_score(item.unified_score.final_score),
        location_info: LocationInfo::from_item(item),
        action: item.recommendation.primary_action.clone(),
        impact: item.expected_impact.clone(),
        complexity_info: ComplexityInfo::from_item(item),
        dependency_info: DependencyInfo::from_item(item),
        debt_specific_info: DebtSpecificInfo::from_item(item),
        coverage_info: CoverageInfo::from_item(item, has_coverage_data),
        rationale: item.recommendation.rationale.clone(),
    }
}

// Data structures for formatted content
pub(crate) struct FormatContext {
    pub rank: usize,
    pub score: f64,
    pub severity_info: SeverityInfo,
    pub location_info: LocationInfo,
    pub action: String,
    pub impact: crate::priority::ImpactMetrics,
    pub complexity_info: ComplexityInfo,
    pub dependency_info: DependencyInfo,
    pub debt_specific_info: DebtSpecificInfo,
    pub coverage_info: Option<CoverageInfo>,
    pub rationale: String,
}

pub(crate) struct SeverityInfo {
    pub label: String,
    pub color: colored::Color,
}

impl SeverityInfo {
    fn from_score(score: f64) -> Self {
        Self {
            label: super::get_severity_label(score).to_string(),
            color: super::get_severity_color(score),
        }
    }
}

pub(crate) struct LocationInfo {
    pub file: std::path::PathBuf,
    pub line: u32,
    pub function: String,
}

impl LocationInfo {
    fn from_item(item: &UnifiedDebtItem) -> Self {
        Self {
            file: item.location.file.clone(),
            line: item.location.line as u32,
            function: item.location.function.clone(),
        }
    }
}

pub(crate) struct ComplexityInfo {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub branch_count: u32,
    pub nesting: u32,
    pub has_complexity: bool,
    pub entropy_details: Option<EntropyDetails>,
}

impl ComplexityInfo {
    fn from_item(item: &UnifiedDebtItem) -> Self {
        let (cyclomatic, cognitive, branch_count, nesting, _length) =
            super::extract_complexity_info(item);
        Self {
            cyclomatic,
            cognitive,
            branch_count,
            nesting,
            has_complexity: cyclomatic > 0 || cognitive > 0,
            entropy_details: item.entropy_details.clone(),
        }
    }
}

pub(crate) struct DependencyInfo {
    #[allow(dead_code)]
    pub upstream: usize,
    #[allow(dead_code)]
    pub downstream: usize,
    pub upstream_callers: Vec<String>,
    pub downstream_callees: Vec<String>,
    #[allow(dead_code)]
    pub has_dependencies: bool,
}

impl DependencyInfo {
    fn from_item(item: &UnifiedDebtItem) -> Self {
        let (upstream, downstream) = super::extract_dependency_info(item);
        Self {
            upstream,
            downstream,
            upstream_callers: item.upstream_callers.clone(),
            downstream_callees: item.downstream_callees.clone(),
            has_dependencies: upstream > 0 || downstream > 0,
        }
    }
}

pub(crate) enum DebtSpecificInfo {
    DeadCode {
        visibility: String,
        usage_hints: Vec<String>,
    },
    Other,
}

impl DebtSpecificInfo {
    fn from_item(item: &UnifiedDebtItem) -> Self {
        match &item.debt_type {
            DebtType::DeadCode {
                visibility,
                usage_hints,
                ..
            } => Self::DeadCode {
                visibility: format_visibility(visibility).to_string(),
                usage_hints: usage_hints.clone(),
            },
            _ => Self::Other,
        }
    }
}

fn format_visibility(visibility: &FunctionVisibility) -> &'static str {
    match visibility {
        FunctionVisibility::Private => "private",
        FunctionVisibility::Crate => "crate-public",
        FunctionVisibility::Public => "public",
    }
}

pub(crate) struct CoverageInfo {
    pub tag: String,
    pub color: colored::Color,
}

impl CoverageInfo {
    fn from_item(item: &UnifiedDebtItem, has_coverage_data: bool) -> Option<Self> {
        if !has_coverage_data {
            return None;
        }

        if let Some(ref trans_cov) = item.transitive_coverage {
            let coverage_pct = trans_cov.direct * 100.0;
            Some(Self::from_coverage_percentage(coverage_pct))
        } else if item.unified_score.coverage_factor >= 10.0 {
            Some(Self {
                tag: "[ERROR UNTESTED]".to_string(),
                color: colored::Color::BrightRed,
            })
        } else {
            None
        }
    }

    fn from_coverage_percentage(coverage_pct: f64) -> Self {
        match coverage_pct {
            0.0 => Self {
                tag: "[ERROR UNTESTED]".to_string(),
                color: colored::Color::BrightRed,
            },
            c if c < 20.0 => Self {
                tag: "[WARN LOW]".to_string(),
                color: colored::Color::Yellow,
            },
            c if c < 50.0 => Self {
                tag: "[WARN PARTIAL]".to_string(),
                color: colored::Color::Yellow,
            },
            c if c < 80.0 => Self {
                tag: "[INFO MODERATE]".to_string(),
                color: colored::Color::Cyan,
            },
            c if c < 95.0 => Self {
                tag: "[OK GOOD]".to_string(),
                color: colored::Color::Green,
            },
            _ => Self {
                tag: "[OK EXCELLENT]".to_string(),
                color: colored::Color::BrightGreen,
            },
        }
    }
}
