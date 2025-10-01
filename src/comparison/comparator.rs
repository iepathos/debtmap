use anyhow::Result;
use std::collections::{HashMap, HashSet};

use crate::comparison::types::*;
use crate::priority::{UnifiedAnalysis, UnifiedDebtItem};

pub struct Comparator {
    before: UnifiedAnalysis,
    after: UnifiedAnalysis,
    target_location: Option<String>,
}

impl Comparator {
    pub fn new(
        before: UnifiedAnalysis,
        after: UnifiedAnalysis,
        target_location: Option<String>,
    ) -> Self {
        Self {
            before,
            after,
            target_location,
        }
    }

    /// Perform full comparison
    pub fn compare(&self) -> Result<ComparisonResult> {
        let target_item = self
            .target_location
            .as_ref()
            .map(|loc| self.compare_target_item(loc))
            .transpose()?;

        let project_health = self.compare_project_health();
        let regressions = self.find_regressions();
        let improvements = self.find_improvements();
        let summary = self.generate_summary(&target_item, &regressions, &improvements);

        Ok(ComparisonResult {
            metadata: self.build_metadata(),
            target_item,
            project_health,
            regressions,
            improvements,
            summary,
        })
    }

    /// Compare specific target item
    fn compare_target_item(&self, location: &str) -> Result<TargetComparison> {
        let before_item = self.find_item_by_location(&self.before, location);
        let after_item = self.find_item_by_location(&self.after, location);

        let status = match (&before_item, &after_item) {
            (None, _) => TargetStatus::NotFoundBefore,
            (Some(_), None) => TargetStatus::Resolved,
            (Some(before), Some(after)) => self.classify_target_status(before, after),
        };

        let (before_metrics, after_metrics, improvements) = match (&before_item, &after_item) {
            (Some(before), Some(after)) => {
                let before_m = self.extract_metrics(before);
                let after_m = self.extract_metrics(after);
                let improvements = self.calculate_improvements(&before_m, &after_m);
                (before_m, Some(after_m), improvements)
            }
            (Some(before), None) => {
                let before_m = self.extract_metrics(before);
                let improvements = ImprovementMetrics {
                    score_reduction_pct: 100.0,
                    complexity_reduction_pct: 100.0,
                    coverage_improvement_pct: 100.0,
                };
                (before_m, None, improvements)
            }
            (None, _) => {
                return Err(anyhow::anyhow!(
                    "Target item not found in before analysis at location: {}",
                    location
                ));
            }
        };

        Ok(TargetComparison {
            location: location.to_string(),
            before: before_metrics,
            after: after_metrics,
            improvements,
            status,
        })
    }

    /// Find regressions (new critical items)
    fn find_regressions(&self) -> Vec<RegressionItem> {
        let before_critical: HashSet<String> = self
            .before
            .items
            .iter()
            .filter(|item| self.get_score(item) >= 60.0)
            .map(|item| self.item_key(item))
            .collect();

        let after_critical: Vec<&UnifiedDebtItem> = self
            .after
            .items
            .iter()
            .filter(|item| self.get_score(item) >= 60.0)
            .collect();

        after_critical
            .iter()
            .filter(|item| !before_critical.contains(&self.item_key(item)))
            .map(|item| self.build_regression_item(item))
            .collect()
    }

    /// Find improvements (resolved or significantly improved items)
    fn find_improvements(&self) -> Vec<ImprovementItem> {
        let before_items: HashMap<String, &UnifiedDebtItem> = self
            .before
            .items
            .iter()
            .map(|item| (self.item_key(item), item))
            .collect();

        let after_keys: HashSet<String> = self
            .after
            .items
            .iter()
            .map(|item| self.item_key(item))
            .collect();

        let mut improvements = Vec::new();

        // Find resolved items
        for (key, before_item) in before_items.iter() {
            if !after_keys.contains(key) && self.get_score(before_item) >= 40.0 {
                improvements.push(ImprovementItem {
                    location: self.format_location(before_item),
                    before_score: self.get_score(before_item),
                    after_score: None,
                    improvement_type: ImprovementType::Resolved,
                });
            }
        }

        // Find significantly improved items (>30% reduction)
        for before_item in before_items.values() {
            let key = self.item_key(before_item);
            if let Some(after_item) = self
                .after
                .items
                .iter()
                .find(|item| self.item_key(item) == key)
            {
                let before_score = self.get_score(before_item);
                let after_score = self.get_score(after_item);

                if before_score > 0.0 {
                    let reduction = (before_score - after_score) / before_score * 100.0;
                    if reduction >= 30.0 {
                        improvements.push(ImprovementItem {
                            location: self.format_location(before_item),
                            before_score,
                            after_score: Some(after_score),
                            improvement_type: ImprovementType::ScoreReduced,
                        });
                    }
                }
            }
        }

        improvements
    }

    /// Compare project-wide health metrics
    fn compare_project_health(&self) -> ProjectHealthComparison {
        let before_metrics = self.extract_project_metrics(&self.before);
        let after_metrics = self.extract_project_metrics(&self.after);
        let changes = self.calculate_project_changes(&before_metrics, &after_metrics);

        ProjectHealthComparison {
            before: before_metrics,
            after: after_metrics,
            changes,
        }
    }

    // Helper methods

    fn find_item_by_location<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        location: &str,
    ) -> Option<&'a UnifiedDebtItem> {
        let parts: Vec<&str> = location.split(':').collect();
        if parts.len() != 3 {
            return None;
        }

        let (file, function, line_str) = (parts[0], parts[1], parts[2]);
        let line: usize = line_str.parse().ok()?;

        analysis.items.iter().find(|item| {
            // Normalize paths for comparison (strip leading ./)
            let item_file = item
                .location
                .file
                .to_string_lossy()
                .strip_prefix("./")
                .unwrap_or(&item.location.file.to_string_lossy())
                .to_string();
            let target_file = file.strip_prefix("./").unwrap_or(file);

            item_file == target_file
                && item.location.function == function
                && item.location.line == line
        })
    }

    fn item_key(&self, item: &UnifiedDebtItem) -> String {
        let file_str = item.location.file.to_string_lossy();
        let normalized_file = file_str.strip_prefix("./").unwrap_or(&file_str);
        format!(
            "{}:{}:{}",
            normalized_file, item.location.function, item.location.line
        )
    }

    fn get_score(&self, item: &UnifiedDebtItem) -> f64 {
        item.unified_score.final_score
    }

    fn format_location(&self, item: &UnifiedDebtItem) -> String {
        self.item_key(item)
    }

    fn extract_metrics(&self, item: &UnifiedDebtItem) -> TargetMetrics {
        let coverage = item
            .transitive_coverage
            .as_ref()
            .map(|tc| tc.transitive)
            .unwrap_or(0.0);

        TargetMetrics {
            score: self.get_score(item),
            cyclomatic_complexity: item.cyclomatic_complexity,
            cognitive_complexity: item.cognitive_complexity,
            coverage,
            function_length: item.function_length,
            nesting_depth: item.nesting_depth,
        }
    }

    fn calculate_improvements(
        &self,
        before: &TargetMetrics,
        after: &TargetMetrics,
    ) -> ImprovementMetrics {
        let score_reduction_pct = if before.score > 0.0 {
            ((before.score - after.score) / before.score * 100.0).max(0.0)
        } else {
            0.0
        };

        let before_complexity = before.cyclomatic_complexity + before.cognitive_complexity;
        let after_complexity = after.cyclomatic_complexity + after.cognitive_complexity;
        let complexity_reduction_pct = if before_complexity > 0 {
            ((before_complexity - after_complexity) as f64 / before_complexity as f64 * 100.0)
                .max(0.0)
        } else {
            0.0
        };

        let coverage_improvement_pct = (after.coverage - before.coverage).max(0.0);

        ImprovementMetrics {
            score_reduction_pct,
            complexity_reduction_pct,
            coverage_improvement_pct,
        }
    }

    fn classify_target_status(
        &self,
        before: &UnifiedDebtItem,
        after: &UnifiedDebtItem,
    ) -> TargetStatus {
        let before_score = self.get_score(before);
        let after_score = self.get_score(after);

        if after_score < before_score * 0.7 {
            TargetStatus::Improved
        } else if after_score > before_score * 1.1 {
            TargetStatus::Regressed
        } else {
            TargetStatus::Unchanged
        }
    }

    fn extract_project_metrics(&self, analysis: &UnifiedAnalysis) -> ProjectMetrics {
        let total_items = analysis.items.len();
        let critical_items = analysis
            .items
            .iter()
            .filter(|item| self.get_score(item) >= 60.0)
            .count();
        let high_priority_items = analysis
            .items
            .iter()
            .filter(|item| self.get_score(item) >= 40.0)
            .count();

        let average_score = if total_items > 0 {
            analysis
                .items
                .iter()
                .map(|item| self.get_score(item))
                .sum::<f64>()
                / total_items as f64
        } else {
            0.0
        };

        ProjectMetrics {
            total_debt_score: analysis.total_debt_score,
            total_items,
            critical_items,
            high_priority_items,
            average_score,
        }
    }

    fn calculate_project_changes(
        &self,
        before: &ProjectMetrics,
        after: &ProjectMetrics,
    ) -> ProjectChanges {
        let debt_score_change = after.total_debt_score - before.total_debt_score;
        let debt_score_change_pct = if before.total_debt_score > 0.0 {
            debt_score_change / before.total_debt_score * 100.0
        } else {
            0.0
        };

        ProjectChanges {
            debt_score_change,
            debt_score_change_pct,
            items_change: after.total_items as i32 - before.total_items as i32,
            critical_items_change: after.critical_items as i32 - before.critical_items as i32,
        }
    }

    fn build_regression_item(&self, item: &UnifiedDebtItem) -> RegressionItem {
        RegressionItem {
            location: self.format_location(item),
            score: self.get_score(item),
            debt_type: format!("{:?}", item.debt_type),
            description: format!(
                "New critical debt item with score {:.1}",
                self.get_score(item)
            ),
        }
    }

    fn generate_summary(
        &self,
        target: &Option<TargetComparison>,
        regressions: &[RegressionItem],
        improvements: &[ImprovementItem],
    ) -> ComparisonSummary {
        let target_improved = target
            .as_ref()
            .map(|t| matches!(t.status, TargetStatus::Improved | TargetStatus::Resolved))
            .unwrap_or(false);

        let overall_debt_trend =
            if self.after.total_debt_score < self.before.total_debt_score * 0.95 {
                DebtTrend::Improving
            } else if self.after.total_debt_score > self.before.total_debt_score * 1.05 {
                DebtTrend::Regressing
            } else {
                DebtTrend::Stable
            };

        ComparisonSummary {
            target_improved,
            new_critical_count: regressions.len(),
            resolved_count: improvements
                .iter()
                .filter(|i| matches!(i.improvement_type, ImprovementType::Resolved))
                .count(),
            overall_debt_trend,
        }
    }

    fn build_metadata(&self) -> ComparisonMetadata {
        ComparisonMetadata {
            comparison_date: chrono::Utc::now().to_rfc3339(),
            before_file: "before.json".to_string(),
            after_file: "after.json".to_string(),
            target_location: self.target_location.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        unified_scorer::{Location, UnifiedScore},
        DebtType, FunctionRole, ImpactMetrics,
    };
    use im::Vector;
    use std::path::PathBuf;

    fn create_test_item(file: &str, function: &str, line: usize, score: f64) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from(file),
                function: function.to_string(),
                line,
            },
            debt_type: DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 20,
            },
            unified_score: UnifiedScore {
                complexity_factor: score / 10.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                final_score: score,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: crate::priority::ActionableRecommendation {
                primary_action: "Test".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 2,
            function_length: 50,
            cyclomatic_complexity: 10,
            cognitive_complexity: 20,
            entropy_details: None,
            is_pure: None,
            purity_confidence: None,
            god_object_indicators: None,
        }
    }

    fn create_test_analysis(items: Vec<UnifiedDebtItem>, total_score: f64) -> UnifiedAnalysis {
        UnifiedAnalysis {
            items: Vector::from(items),
            file_items: Vector::new(),
            total_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: total_score,
            debt_density: 0.0,
            total_lines_of_code: 1000,
            call_graph: crate::priority::CallGraph::new(),
            data_flow_graph: crate::data_flow::DataFlowGraph::new(),
            overall_coverage: None,
        }
    }

    #[test]
    fn test_compare_target_improved() {
        let before = create_test_analysis(
            vec![create_test_item("src/main.rs", "func", 42, 81.9)],
            81.9,
        );
        let after = create_test_analysis(
            vec![create_test_item("src/main.rs", "func", 42, 15.2)],
            15.2,
        );

        let comparator = Comparator::new(before, after, Some("src/main.rs:func:42".to_string()));
        let result = comparator.compare().unwrap();

        assert!(result.target_item.is_some());
        let target = result.target_item.unwrap();
        assert_eq!(target.status, TargetStatus::Improved);
        assert!(target.improvements.score_reduction_pct > 80.0);
    }

    #[test]
    fn test_compare_target_resolved() {
        let before = create_test_analysis(
            vec![create_test_item("src/main.rs", "func", 42, 81.9)],
            81.9,
        );
        let after = create_test_analysis(vec![], 0.0);

        let comparator = Comparator::new(before, after, Some("src/main.rs:func:42".to_string()));
        let result = comparator.compare().unwrap();

        let target = result.target_item.unwrap();
        assert_eq!(target.status, TargetStatus::Resolved);
        assert_eq!(target.after, None);
        assert_eq!(target.improvements.score_reduction_pct, 100.0);
    }

    #[test]
    fn test_detect_regressions() {
        let before = create_test_analysis(
            vec![create_test_item("src/main.rs", "old_func", 42, 81.9)],
            81.9,
        );
        let after = create_test_analysis(
            vec![
                create_test_item("src/main.rs", "old_func", 42, 15.2),
                create_test_item("src/main.rs", "new_func1", 156, 65.3),
                create_test_item("src/main.rs", "new_func2", 189, 58.7),
            ],
            139.2,
        );

        let comparator = Comparator::new(before, after, None);
        let result = comparator.compare().unwrap();

        assert_eq!(result.regressions.len(), 1); // Only 65.3 >= 60
        assert_eq!(result.summary.overall_debt_trend, DebtTrend::Regressing);
    }

    #[test]
    fn test_project_health_improving() {
        let before = create_test_analysis(
            vec![
                create_test_item("src/main.rs", "func1", 10, 50.0),
                create_test_item("src/main.rs", "func2", 20, 50.0),
            ],
            100.0,
        );
        let after = create_test_analysis(
            vec![
                create_test_item("src/main.rs", "func1", 10, 20.0),
                create_test_item("src/main.rs", "func2", 20, 20.0),
            ],
            40.0,
        );

        let comparator = Comparator::new(before, after, None);
        let result = comparator.compare().unwrap();

        assert_eq!(result.project_health.changes.debt_score_change, -60.0);
        assert_eq!(result.summary.overall_debt_trend, DebtTrend::Improving);
    }
}
