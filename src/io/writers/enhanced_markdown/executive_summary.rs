use crate::core::AnalysisResults;
use crate::priority::{DebtType, UnifiedAnalysis, UnifiedDebtItem};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Health Dashboard Components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDashboard {
    pub overall_health: HealthStatus,
    pub trend: TrendIndicator,
    pub velocity_impact: VelocityImpact,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Good(u8),         // 80-100%
    ModerateRisk(u8), // 60-79%
    HighRisk(u8),     // 40-59%
    Critical(u8),     // 0-39%
}

impl HealthStatus {
    pub fn from_score(score: u32) -> Self {
        let score_u8 = score.min(100) as u8;
        match score {
            80..=100 => HealthStatus::Good(score_u8),
            60..=79 => HealthStatus::ModerateRisk(score_u8),
            40..=59 => HealthStatus::HighRisk(score_u8),
            _ => HealthStatus::Critical(score_u8),
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            HealthStatus::Good(score) => format!("{}% [GOOD] Good", score),
            HealthStatus::ModerateRisk(score) => format!("{}% [WARN] Moderate Risk", score),
            HealthStatus::HighRisk(score) => format!("{}% [HIGH] High Risk", score),
            HealthStatus::Critical(score) => format!("{}% [CRIT] Critical", score),
        }
    }

    pub fn interpretation(&self) -> &'static str {
        match self {
            HealthStatus::Good(_) => "Minimal technical debt, sustainable velocity",
            HealthStatus::ModerateRisk(_) => "Some debt accumulation, watch for trends",
            HealthStatus::HighRisk(_) => "Significant debt impact, allocate debt reduction",
            HealthStatus::Critical(_) => "Major architectural issues, immediate action needed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendIndicator {
    Improving,
    Stable,
    Declining,
}

impl TrendIndicator {
    pub fn as_emoji(&self) -> &'static str {
        match self {
            TrendIndicator::Improving => "/\\",
            TrendIndicator::Stable => "--",
            TrendIndicator::Declining => "\\/",
        }
    }

    pub fn as_string(&self) -> &'static str {
        match self {
            TrendIndicator::Improving => "Improving",
            TrendIndicator::Stable => "Stable",
            TrendIndicator::Declining => "Declining",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityImpact {
    pub slowdown_percentage: f64,
    pub description: String,
}

impl VelocityImpact {
    pub fn from_debt_analysis(debt_count: usize, avg_complexity: f64) -> Self {
        // Empirical formula based on industry studies
        let complexity_factor = (avg_complexity / 10.0).min(2.0);
        let debt_factor = (debt_count as f64 / 50.0).min(2.0);
        let slowdown = (complexity_factor * 0.15 + debt_factor * 0.10) * 100.0;

        let description = match slowdown {
            x if x < 5.0 => "Minimal impact on delivery speed",
            x if x < 15.0 => "Moderate slowdown in feature delivery",
            x if x < 30.0 => "Significant impact on development velocity",
            _ => "Severe velocity reduction, impacting deadlines",
        };

        VelocityImpact {
            slowdown_percentage: slowdown,
            description: description.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Moderate,
    High,
    Critical,
}

impl RiskLevel {
    pub fn from_metrics(health_score: u32, debt_count: usize, avg_complexity: f64) -> Self {
        if health_score >= 80 && debt_count < 20 && avg_complexity < 10.0 {
            RiskLevel::Low
        } else if health_score >= 60 && debt_count < 50 && avg_complexity < 15.0 {
            RiskLevel::Moderate
        } else if health_score >= 40 && debt_count < 100 && avg_complexity < 20.0 {
            RiskLevel::High
        } else {
            RiskLevel::Critical
        }
    }

    pub fn as_string(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low Risk",
            RiskLevel::Moderate => "Moderate Risk",
            RiskLevel::High => "High Risk",
            RiskLevel::Critical => "Critical Risk",
        }
    }
}

// Quick Wins Analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickWins {
    pub count: usize,
    pub total_effort_hours: u32,
    pub expected_impact: ImpactSummary,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactSummary {
    pub health_improvement: f64,
    pub complexity_reduction: f64,
    pub coverage_increase: f64,
}

pub fn identify_quick_wins(items: &[UnifiedDebtItem]) -> QuickWins {
    let quick_items: Vec<_> = items
        .iter()
        .filter(|item| estimate_effort_hours(item) <= 8)
        .collect();

    let total_effort: u32 = quick_items
        .iter()
        .map(|item| estimate_effort_hours(item))
        .sum();

    let expected_impact = calculate_batch_impact(&quick_items);

    let mut recommendations = Vec::new();

    // Group quick wins by type for better recommendations
    let mut by_type: HashMap<String, Vec<&UnifiedDebtItem>> = HashMap::new();
    for item in &quick_items {
        let type_key = match &item.debt_type {
            DebtType::TestingGap { .. } => "Testing",
            DebtType::ComplexityHotspot { .. } => "Complexity",
            DebtType::DeadCode { .. } => "Dead Code",
            DebtType::Duplication { .. } => "Duplication",
            DebtType::Risk { .. } => "Risk",
            DebtType::GodObject { .. } => "God Object",
            DebtType::GodModule { .. } => "God Module",
            DebtType::TestComplexityHotspot { .. } => "Test Complexity",
            DebtType::TestTodo { .. } => "Test Todo",
            DebtType::TestDuplication { .. } => "Test Duplication",
            DebtType::ErrorSwallowing { .. } => "Error Handling",
            DebtType::AllocationInefficiency { .. } => "Allocation",
            DebtType::StringConcatenation { .. } => "String Operations",
            DebtType::NestedLoops { .. } => "Nested Loops",
            DebtType::BlockingIO { .. } => "Blocking IO",
            DebtType::SuboptimalDataStructure { .. } => "Data Structure",
            DebtType::FeatureEnvy { .. } => "Feature Envy",
            DebtType::PrimitiveObsession { .. } => "Primitive Obsession",
            DebtType::MagicValues { .. } => "Magic Values",
            DebtType::AssertionComplexity { .. } => "Complex Assertions",
            DebtType::FlakyTestPattern { .. } => "Flaky Tests",
            DebtType::AsyncMisuse { .. } => "Async Issues",
            DebtType::ResourceLeak { .. } => "Resource Leaks",
            DebtType::CollectionInefficiency { .. } => "Collection Issues",
        };
        by_type.entry(type_key.to_string()).or_default().push(item);
    }

    // Generate specific recommendations
    if let Some(testing_items) = by_type.get("Testing") {
        recommendations.push(format!(
            "Add tests for {} untested critical functions (est. {} hours)",
            testing_items.len(),
            testing_items
                .iter()
                .map(|i| estimate_effort_hours(i))
                .sum::<u32>()
        ));
    }

    if let Some(dead_code_items) = by_type.get("Dead Code") {
        recommendations.push(format!(
            "Remove {} unused functions to reduce maintenance burden",
            dead_code_items.len()
        ));
    }

    if let Some(duplication_items) = by_type.get("Duplication") {
        recommendations.push(format!(
            "Extract {} duplicated code blocks into shared functions",
            duplication_items.len()
        ));
    }

    // Limit to top 3 recommendations
    recommendations.truncate(3);

    QuickWins {
        count: quick_items.len(),
        total_effort_hours: total_effort,
        expected_impact,
        recommendations,
    }
}

fn calculate_batch_impact(items: &[&UnifiedDebtItem]) -> ImpactSummary {
    let total_complexity_reduction: f64 = items
        .iter()
        .map(|item| item.expected_impact.complexity_reduction)
        .sum();

    let total_coverage_improvement: f64 = items
        .iter()
        .map(|item| item.expected_impact.coverage_improvement)
        .sum();

    // Estimate health improvement based on fixes
    let health_improvement = (items.len() as f64 * 0.5).min(10.0);

    ImpactSummary {
        health_improvement,
        complexity_reduction: total_complexity_reduction,
        coverage_increase: total_coverage_improvement * 100.0,
    }
}

// Strategic Priorities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicPriority {
    pub title: String,
    pub description: String,
    pub effort_estimate: EffortEstimate,
    pub business_impact: String,
    pub blocking_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortEstimate {
    Hours(u32),
    Days(u32),
    Sprints(u32),
}

impl EffortEstimate {
    pub fn as_string(&self) -> String {
        match self {
            EffortEstimate::Hours(h) => format!("{} hours", h),
            EffortEstimate::Days(d) => format!("{} days", d),
            EffortEstimate::Sprints(s) => format!("{} sprints", s),
        }
    }
}

pub fn identify_strategic_priorities(
    items: &[UnifiedDebtItem],
    limit: usize,
) -> Vec<StrategicPriority> {
    let mut priorities = Vec::new();

    for item in items.iter().take(limit) {
        let effort_hours = estimate_effort_hours(item);
        let effort_estimate = if effort_hours <= 8 {
            EffortEstimate::Hours(effort_hours)
        } else if effort_hours <= 40 {
            EffortEstimate::Days(effort_hours / 8)
        } else {
            EffortEstimate::Sprints(effort_hours / 80)
        };

        let business_impact = generate_business_impact(item);

        // Calculate blocking factor based on dependencies
        let blocking_factor = calculate_blocking_factor(item);

        priorities.push(StrategicPriority {
            title: item.recommendation.primary_action.clone(),
            description: format!(
                "In {}: {}",
                item.location.file.display(),
                item.location.function
            ),
            effort_estimate,
            business_impact,
            blocking_factor,
        });
    }

    priorities
}

fn generate_business_impact(item: &UnifiedDebtItem) -> String {
    match &item.debt_type {
        DebtType::TestingGap { coverage, .. } => {
            format!(
                "Reduces production bug risk by ~{}%, improves deployment confidence",
                ((1.0 - coverage) * 30.0) as u32
            )
        }
        DebtType::ComplexityHotspot { cyclomatic, .. } => {
            format!(
                "Reduces feature development time by ~{}%, lowers bug introduction rate",
                (cyclomatic / 2).min(30)
            )
        }
        DebtType::GodObject {
            responsibilities, ..
        } => {
            format!(
                "Unblocks parallel development on {} related features, reduces merge conflicts",
                responsibilities / 3
            )
        }
        DebtType::DeadCode { .. } => {
            "Reduces maintenance overhead, improves code clarity".to_string()
        }
        DebtType::Duplication { instances, .. } => {
            format!(
                "Eliminates {} duplicate maintenance points, reduces inconsistency risk",
                instances
            )
        }
        DebtType::Risk { risk_score, .. } => {
            format!(
                "Reduces technical risk by {:.0}%, improves system stability",
                risk_score * 10.0
            )
        }
        DebtType::BlockingIO { operation, .. } => {
            format!(
                "Improves responsiveness by eliminating blocking {}",
                operation
            )
        }
        DebtType::NestedLoops {
            complexity_estimate,
            ..
        } => {
            format!(
                "Reduces algorithmic complexity ({}), improves performance",
                complexity_estimate
            )
        }
        DebtType::SuboptimalDataStructure {
            recommended_type, ..
        } => {
            format!("Improves performance by using {}", recommended_type)
        }
        DebtType::FeatureEnvy { external_class, .. } => {
            format!(
                "Improves cohesion, reduces coupling with {}",
                external_class
            )
        }
        DebtType::TestComplexityHotspot { .. } => {
            "Simplifies test maintenance, reduces test execution time".to_string()
        }
        DebtType::TestDuplication { .. } => {
            "Reduces test maintenance burden, improves consistency".to_string()
        }
        DebtType::ErrorSwallowing { .. } => {
            "Improves error visibility, aids debugging and monitoring".to_string()
        }
        DebtType::AllocationInefficiency { .. } => {
            "Reduces memory pressure, improves performance".to_string()
        }
        DebtType::StringConcatenation { .. } => {
            "Improves string building performance, reduces allocations".to_string()
        }
        DebtType::PrimitiveObsession { .. } => {
            "Improves type safety, reduces validation errors".to_string()
        }
        DebtType::MagicValues { .. } => {
            "Improves code readability, reduces maintenance errors".to_string()
        }
        DebtType::AssertionComplexity { .. } => {
            "Simplifies test understanding, improves test reliability".to_string()
        }
        DebtType::FlakyTestPattern { .. } => {
            "Improves test reliability, reduces false failures".to_string()
        }
        DebtType::AsyncMisuse { .. } => {
            "Prevents concurrency bugs, improves async performance".to_string()
        }
        DebtType::ResourceLeak { .. } => {
            "Prevents resource exhaustion, improves system stability".to_string()
        }
        DebtType::CollectionInefficiency { .. } => {
            "Improves collection performance, reduces memory usage".to_string()
        }
        _ => "Improves overall codebase health and maintainability".to_string(),
    }
}

fn calculate_blocking_factor(item: &UnifiedDebtItem) -> f64 {
    // Higher downstream dependencies = higher blocking factor
    let dependency_factor = (item.downstream_dependencies as f64 / 10.0).min(1.0);

    // Higher complexity = more blocking
    let complexity_factor = (item.cyclomatic_complexity as f64 / 20.0).min(1.0);

    // Combined blocking factor
    (dependency_factor * 0.6 + complexity_factor * 0.4) * 10.0
}

// Team Guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamGuidance {
    pub recommended_debt_allocation: u8,
    pub focus_areas: Vec<String>,
    pub process_improvements: Vec<String>,
}

impl TeamGuidance {
    pub fn from_analysis(health_score: u32, items: &[UnifiedDebtItem]) -> Self {
        let recommended_debt_allocation = match health_score {
            80..=100 => 5, // 5-10%
            60..=79 => 15, // 15-20%
            40..=59 => 25, // 25-30%
            _ => 40,       // 40-50%
        };

        let mut focus_areas = Vec::new();
        let mut process_improvements = Vec::new();

        // Analyze debt types to determine focus areas
        let mut type_counts: HashMap<&str, usize> = HashMap::new();
        for item in items {
            let type_key = match &item.debt_type {
                DebtType::TestingGap { .. } => "Testing",
                DebtType::ComplexityHotspot { .. } => "Complexity",
                DebtType::GodObject { .. } => "Architecture",
                _ => "Other",
            };
            *type_counts.entry(type_key).or_default() += 1;
        }

        // Generate focus areas based on debt distribution
        if type_counts.get("Testing").unwrap_or(&0) > &10 {
            focus_areas.push("Improve test coverage for critical paths".to_string());
            process_improvements.push("Implement test-first development practices".to_string());
        }

        if type_counts.get("Complexity").unwrap_or(&0) > &5 {
            focus_areas.push("Refactor complex functions into smaller units".to_string());
            process_improvements.push("Enforce complexity limits in code reviews".to_string());
        }

        if type_counts.get("Architecture").unwrap_or(&0) > &3 {
            focus_areas.push("Refactor god objects and improve module boundaries".to_string());
            process_improvements.push("Regular architecture review sessions".to_string());
        }

        TeamGuidance {
            recommended_debt_allocation,
            focus_areas,
            process_improvements,
        }
    }
}

// Success Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessMetrics {
    pub target_health_score: u8,
    pub target_coverage: f64,
    pub target_complexity_reduction: f64,
    pub timeline: String,
}

impl SuccessMetrics {
    pub fn from_current_state(health_score: u32, avg_complexity: f64) -> Self {
        let target_health_score = match health_score {
            80..=100 => 90,
            60..=79 => 80,
            40..=59 => 65,
            _ => 50,
        };

        let target_complexity_reduction = if avg_complexity > 15.0 {
            30.0
        } else if avg_complexity > 10.0 {
            20.0
        } else {
            10.0
        };

        let timeline = match health_score {
            80..=100 => "Next sprint",
            60..=79 => "Next 2 sprints",
            40..=59 => "Next quarter",
            _ => "Next 2 quarters",
        };

        SuccessMetrics {
            target_health_score,
            target_coverage: 0.8,
            target_complexity_reduction,
            timeline: timeline.to_string(),
        }
    }
}

// Complete Executive Summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    pub health_dashboard: HealthDashboard,
    pub quick_wins: QuickWins,
    pub strategic_priorities: Vec<StrategicPriority>,
    pub team_guidance: TeamGuidance,
    pub success_metrics: SuccessMetrics,
}

pub fn generate_executive_summary(
    results: &AnalysisResults,
    unified_analysis: Option<&UnifiedAnalysis>,
    health_score: u32,
    avg_complexity: f64,
) -> ExecutiveSummary {
    let debt_count = results.technical_debt.items.len();

    // Generate health dashboard
    let health_dashboard = HealthDashboard {
        overall_health: HealthStatus::from_score(health_score),
        trend: TrendIndicator::Stable, // Would need historical data for real trend
        velocity_impact: VelocityImpact::from_debt_analysis(debt_count, avg_complexity),
        risk_level: RiskLevel::from_metrics(health_score, debt_count, avg_complexity),
    };

    // Identify quick wins and strategic priorities
    let (quick_wins, strategic_priorities, team_guidance) = if let Some(analysis) = unified_analysis
    {
        let items: Vec<UnifiedDebtItem> = analysis.items.iter().cloned().collect();
        (
            identify_quick_wins(&items),
            identify_strategic_priorities(&items, 3),
            TeamGuidance::from_analysis(health_score, &items),
        )
    } else {
        (
            QuickWins {
                count: 0,
                total_effort_hours: 0,
                expected_impact: ImpactSummary {
                    health_improvement: 0.0,
                    complexity_reduction: 0.0,
                    coverage_increase: 0.0,
                },
                recommendations: vec![],
            },
            vec![],
            TeamGuidance {
                recommended_debt_allocation: 10,
                focus_areas: vec![],
                process_improvements: vec![],
            },
        )
    };

    let success_metrics = SuccessMetrics::from_current_state(health_score, avg_complexity);

    ExecutiveSummary {
        health_dashboard,
        quick_wins,
        strategic_priorities,
        team_guidance,
        success_metrics,
    }
}

// Helper function to estimate effort
pub fn estimate_effort_hours(item: &UnifiedDebtItem) -> u32 {
    match &item.debt_type {
        DebtType::TestingGap { cyclomatic, .. } => {
            // Testing effort scales with complexity
            (cyclomatic / 2).clamp(2, 16)
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
            ..
        } => {
            // Refactoring effort based on both metrics
            ((cyclomatic + cognitive) / 3).clamp(4, 40)
        }
        DebtType::DeadCode { .. } => {
            // Dead code removal is usually quick
            1
        }
        DebtType::Duplication { instances, .. } => {
            // Extraction effort scales with instances
            (instances * 2).min(24)
        }
        DebtType::GodObject {
            responsibilities, ..
        } => {
            // God object refactoring is complex
            (responsibilities * 4).clamp(16, 80)
        }
        DebtType::BlockingIO { .. } => {
            // Async refactoring
            12
        }
        DebtType::NestedLoops { depth, .. } => {
            // Complexity scales with depth
            (depth * 4).min(24)
        }
        DebtType::SuboptimalDataStructure { .. } => {
            // Data structure refactoring
            8
        }
        DebtType::FeatureEnvy { .. } => {
            // Moving methods between classes
            6
        }
        DebtType::TestComplexityHotspot { .. } => {
            // Test refactoring
            4
        }
        DebtType::TestDuplication { instances, .. } => {
            // Test deduplication
            (*instances).min(12)
        }
        DebtType::TestTodo { .. } => {
            // Completing TODO test
            2
        }
        DebtType::ErrorSwallowing { .. } => {
            // Error handling fix
            3
        }
        DebtType::AllocationInefficiency { .. } => {
            // Memory optimization
            6
        }
        DebtType::StringConcatenation { .. } => {
            // String builder refactor
            3
        }
        DebtType::PrimitiveObsession { .. } => {
            // Type creation
            8
        }
        DebtType::MagicValues { .. } => {
            // Constants extraction
            2
        }
        DebtType::AssertionComplexity { .. } => {
            // Assertion simplification
            3
        }
        DebtType::FlakyTestPattern { .. } => {
            // Test stabilization
            6
        }
        DebtType::AsyncMisuse { .. } => {
            // Async refactoring
            10
        }
        DebtType::ResourceLeak { .. } => {
            // Resource management fix
            5
        }
        DebtType::CollectionInefficiency { .. } => {
            // Collection optimization
            4
        }
        _ => 8, // Default estimate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        ActionableRecommendation, FunctionRole, ImpactMetrics, Location, UnifiedScore,
    };
    use std::path::PathBuf;

    fn create_test_debt_item(debt_type: DebtType, complexity: u32) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_function".to_string(),
                line: 10,
            },
            debt_type,
            recommendation: ActionableRecommendation {
                primary_action: "Test action".to_string(),
                rationale: "Test rationale".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.5,
                complexity_reduction: 0.3,
                coverage_improvement: 0.2,
                lines_reduction: 10,
            },
            unified_score: UnifiedScore {
                final_score: 5.0,
                pre_adjustment_score: None,
                adjustment_applied: None,
                coverage_factor: 1.0,
                complexity_factor: 1.0,
                dependency_factor: 1.0,
                role_multiplier: 1.0,
            },
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            cyclomatic_complexity: complexity,
            cognitive_complexity: complexity - 2,
            nesting_depth: 3,
            function_length: 50,
            function_role: FunctionRole::PureLogic,
            transitive_coverage: None,
            upstream_callers: vec![],
            downstream_callees: vec![],
            entropy_details: None,
            is_pure: None,
            purity_confidence: None,
            god_object_indicators: None,
            tier: None,
        }
    }

    #[test]
    fn test_health_status_classification() {
        assert!(matches!(
            HealthStatus::from_score(90),
            HealthStatus::Good(_)
        ));
        assert!(matches!(
            HealthStatus::from_score(70),
            HealthStatus::ModerateRisk(_)
        ));
        assert!(matches!(
            HealthStatus::from_score(50),
            HealthStatus::HighRisk(_)
        ));
        assert!(matches!(
            HealthStatus::from_score(30),
            HealthStatus::Critical(_)
        ));
    }

    #[test]
    fn test_velocity_impact_calculation() {
        let impact = VelocityImpact::from_debt_analysis(10, 5.0);
        assert!(impact.slowdown_percentage < 10.0);

        let high_impact = VelocityImpact::from_debt_analysis(100, 20.0);
        assert!(high_impact.slowdown_percentage > 20.0);
    }

    #[test]
    fn test_quick_wins_identification() {
        let items = vec![
            create_test_debt_item(
                DebtType::DeadCode {
                    visibility: crate::priority::FunctionVisibility::Private,
                    cyclomatic: 5,
                    cognitive: 3,
                    usage_hints: vec![],
                },
                5,
            ),
            create_test_debt_item(
                DebtType::TestingGap {
                    coverage: 0.0,
                    cyclomatic: 10,
                    cognitive: 8,
                },
                10,
            ),
            create_test_debt_item(
                DebtType::ComplexityHotspot {
                    cyclomatic: 30,
                    cognitive: 25,
                },
                30,
            ),
        ];

        let quick_wins = identify_quick_wins(&items);
        assert!(quick_wins.count > 0);
        assert!(quick_wins.total_effort_hours <= quick_wins.count as u32 * 8);
    }

    #[test]
    fn test_effort_estimation() {
        let test_gap = create_test_debt_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 8,
            },
            10,
        );
        assert!(estimate_effort_hours(&test_gap) <= 16);

        let dead_code = create_test_debt_item(
            DebtType::DeadCode {
                visibility: crate::priority::FunctionVisibility::Private,
                cyclomatic: 5,
                cognitive: 3,
                usage_hints: vec![],
            },
            5,
        );
        assert_eq!(estimate_effort_hours(&dead_code), 1);
    }

    #[test]
    fn test_team_guidance_generation() {
        let items = vec![
            create_test_debt_item(
                DebtType::TestingGap {
                    coverage: 0.0,
                    cyclomatic: 10,
                    cognitive: 8,
                },
                10,
            );
            15
        ];

        let guidance = TeamGuidance::from_analysis(75, &items);
        assert_eq!(guidance.recommended_debt_allocation, 15);
        assert!(!guidance.focus_areas.is_empty());
    }

    #[test]
    fn test_success_metrics_generation() {
        let metrics = SuccessMetrics::from_current_state(65, 12.0);
        assert_eq!(metrics.target_health_score, 80);
        assert_eq!(metrics.target_coverage, 0.8);
        assert_eq!(metrics.target_complexity_reduction, 20.0);
    }
}
