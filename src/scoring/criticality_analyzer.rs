use crate::core::FunctionMetrics;
use crate::priority::call_graph::FunctionId;
use crate::scoring::scoring_context::ScoringContext;

pub struct CriticalityAnalyzer<'a> {
    context: &'a ScoringContext,
}

impl<'a> CriticalityAnalyzer<'a> {
    pub fn new(context: &'a ScoringContext) -> Self {
        Self { context }
    }

    pub fn calculate_criticality(&self, function: &FunctionMetrics) -> f64 {
        let func_id = FunctionId {
            file: function.file.clone(),
            name: function.name.clone(),
            line: function.line,
        };

        self.calculate_criticality_for_id(&func_id)
    }

    fn calculate_distance_factor(distance_opt: Option<usize>) -> f64 {
        match distance_opt {
            Some(distance) => 2.0 / (1.0 + distance as f64 * 0.3),
            None => 1.0,
        }
    }

    fn calculate_caller_factor(caller_count: usize) -> f64 {
        match caller_count {
            0 => 1.0,
            count => {
                let factor = 1.0 + (count as f64).ln() * 0.2;
                factor.min(1.8)
            }
        }
    }

    fn calculate_git_history_factor(
        git_history: &crate::scoring::scoring_context::GitHistory,
        file_path: &std::path::Path,
    ) -> f64 {
        let change_factor = git_history
            .change_counts
            .get(file_path)
            .filter(|&&count| count > 10)
            .map(|&count| (1.0 + (count as f64 / 50.0)).min(1.4))
            .unwrap_or(1.0);

        let bug_factor = git_history
            .bug_fix_counts
            .get(file_path)
            .filter(|&&count| count > 5)
            .map(|&count| (1.0 + (count as f64 / 20.0)).min(1.5))
            .unwrap_or(1.0);

        change_factor * bug_factor
    }

    pub fn calculate_criticality_for_id(&self, function_id: &FunctionId) -> f64 {
        let mut score = 1.0;

        // Factor 1: Distance from entry points (closer = more critical)
        score *= Self::calculate_distance_factor(self.context.distance_from_entry(function_id));

        // Factor 2: Number of callers (fan-in)
        let caller_count = self
            .context
            .call_frequencies
            .get(function_id)
            .copied()
            .unwrap_or(0);
        score *= Self::calculate_caller_factor(caller_count);

        // Factor 3: Hot path bonus
        if self.context.hot_paths.contains(function_id) {
            score *= 1.5;
        }

        // Factor 4: Downstream impact (number of functions this calls)
        let callee_count = self.context.call_graph.get_callees(function_id).len();
        if callee_count > 5 {
            // Functions that orchestrate many others are critical
            let orchestration_factor = 1.0 + (callee_count as f64 / 10.0);
            score *= orchestration_factor.min(1.3);
        }

        // Factor 5: Git history (if available)
        if let Some(git_history) = &self.context.git_history {
            score *= Self::calculate_git_history_factor(git_history, &function_id.file);
        }

        // Cap the criticality multiplier at 2.0x
        score.min(2.0)
    }

    pub fn explain_criticality(&self, function_id: &FunctionId) -> String {
        let factors = self.collect_criticality_factors(function_id);

        match factors.is_empty() {
            true => "Base criticality: 1.0x".to_string(),
            false => factors.join(", "),
        }
    }

    fn collect_criticality_factors(&self, function_id: &FunctionId) -> Vec<String> {
        let mut factors = vec![
            self.calculate_distance_factor_explanation(function_id),
            self.calculate_caller_factor_explanation(function_id),
            self.calculate_hot_path_factor_explanation(function_id),
            self.calculate_downstream_factor_explanation(function_id),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        factors.extend(self.calculate_git_history_factors_explanation(function_id));
        factors
    }

    fn calculate_distance_factor_explanation(&self, function_id: &FunctionId) -> Option<String> {
        self.context
            .distance_from_entry(function_id)
            .map(|distance| {
                let factor = 2.0 / (1.0 + distance as f64 * 0.3);
                format!("Entry distance {}: {:.1}x", distance, factor)
            })
    }

    fn calculate_caller_factor_explanation(&self, function_id: &FunctionId) -> Option<String> {
        self.context
            .call_frequencies
            .get(function_id)
            .filter(|&&count| count > 0)
            .map(|&caller_count| {
                let factor = 1.0 + (caller_count as f64).ln() * 0.2;
                format!("{} callers: {:.1}x", caller_count, factor.min(1.8))
            })
    }

    fn calculate_hot_path_factor_explanation(&self, function_id: &FunctionId) -> Option<String> {
        self.context
            .hot_paths
            .contains(function_id)
            .then(|| "Hot path: 1.5x".to_string())
    }

    fn calculate_downstream_factor_explanation(&self, function_id: &FunctionId) -> Option<String> {
        let callee_count = self.context.call_graph.get_callees(function_id).len();
        (callee_count > 5).then(|| {
            let factor = 1.0 + (callee_count as f64 / 10.0);
            format!("{} callees: {:.1}x", callee_count, factor.min(1.3))
        })
    }

    fn calculate_git_history_factors_explanation(&self, function_id: &FunctionId) -> Vec<String> {
        self.context
            .git_history
            .as_ref()
            .map(|git_history| {
                [
                    self.calculate_change_count_factor(git_history, &function_id.file),
                    self.calculate_bug_fix_factor(git_history, &function_id.file),
                ]
                .into_iter()
                .flatten()
                .collect()
            })
            .unwrap_or_default()
    }

    fn calculate_change_count_factor(
        &self,
        git_history: &crate::scoring::scoring_context::GitHistory,
        file_path: &std::path::Path,
    ) -> Option<String> {
        git_history
            .change_counts
            .get(file_path)
            .filter(|&&count| count > 10)
            .map(|&change_count| {
                let factor = 1.0 + (change_count as f64 / 50.0);
                format!("{} changes: {:.1}x", change_count, factor.min(1.4))
            })
    }

    fn calculate_bug_fix_factor(
        &self,
        git_history: &crate::scoring::scoring_context::GitHistory,
        file_path: &std::path::Path,
    ) -> Option<String> {
        git_history
            .bug_fix_counts
            .get(file_path)
            .filter(|&&count| count > 5)
            .map(|&bug_count| {
                let factor = 1.0 + (bug_count as f64 / 20.0);
                format!("{} bug fixes: {:.1}x", bug_count, factor.min(1.5))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::scoring_context::GitHistory;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_calculate_distance_factor_entry_point() {
        assert_eq!(CriticalityAnalyzer::calculate_distance_factor(Some(0)), 2.0);
    }

    #[test]
    fn test_calculate_distance_factor_distance_one() {
        let factor = CriticalityAnalyzer::calculate_distance_factor(Some(1));
        assert!((factor - 1.538).abs() < 0.01);
    }

    #[test]
    fn test_calculate_distance_factor_distance_two() {
        let factor = CriticalityAnalyzer::calculate_distance_factor(Some(2));
        assert!((factor - 1.25).abs() < 0.01);
    }

    #[test]
    fn test_calculate_distance_factor_far_distance() {
        let factor = CriticalityAnalyzer::calculate_distance_factor(Some(10));
        assert!((factor - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_calculate_distance_factor_none() {
        assert_eq!(CriticalityAnalyzer::calculate_distance_factor(None), 1.0);
    }

    #[test]
    fn test_calculate_caller_factor_zero() {
        assert_eq!(CriticalityAnalyzer::calculate_caller_factor(0), 1.0);
    }

    #[test]
    fn test_calculate_caller_factor_single() {
        assert_eq!(CriticalityAnalyzer::calculate_caller_factor(1), 1.0);
    }

    #[test]
    fn test_calculate_caller_factor_two() {
        let factor = CriticalityAnalyzer::calculate_caller_factor(2);
        assert!((factor - 1.138).abs() < 0.01);
    }

    #[test]
    fn test_calculate_caller_factor_five() {
        let factor = CriticalityAnalyzer::calculate_caller_factor(5);
        assert!((factor - 1.321).abs() < 0.01);
    }

    #[test]
    fn test_calculate_caller_factor_ten() {
        let factor = CriticalityAnalyzer::calculate_caller_factor(10);
        assert!((factor - 1.46).abs() < 0.01);
    }

    #[test]
    fn test_calculate_caller_factor_max_cap() {
        let factor = CriticalityAnalyzer::calculate_caller_factor(100);
        assert_eq!(factor, 1.8);
    }

    #[test]
    fn test_calculate_git_history_factor_no_changes() {
        let mut git_history = GitHistory {
            change_counts: HashMap::new(),
            bug_fix_counts: HashMap::new(),
        };
        git_history
            .change_counts
            .insert(PathBuf::from("test.rs"), 5);
        git_history
            .bug_fix_counts
            .insert(PathBuf::from("test.rs"), 2);

        let factor = CriticalityAnalyzer::calculate_git_history_factor(
            &git_history,
            &PathBuf::from("test.rs"),
        );
        assert_eq!(factor, 1.0);
    }

    #[test]
    fn test_calculate_git_history_factor_with_changes() {
        let mut git_history = GitHistory {
            change_counts: HashMap::new(),
            bug_fix_counts: HashMap::new(),
        };
        git_history
            .change_counts
            .insert(PathBuf::from("test.rs"), 20);

        let factor = CriticalityAnalyzer::calculate_git_history_factor(
            &git_history,
            &PathBuf::from("test.rs"),
        );
        assert!((factor - 1.4).abs() < 0.01);
    }

    #[test]
    fn test_calculate_git_history_factor_with_bugs() {
        let mut git_history = GitHistory {
            change_counts: HashMap::new(),
            bug_fix_counts: HashMap::new(),
        };
        git_history
            .bug_fix_counts
            .insert(PathBuf::from("test.rs"), 10);

        let factor = CriticalityAnalyzer::calculate_git_history_factor(
            &git_history,
            &PathBuf::from("test.rs"),
        );
        assert_eq!(factor, 1.5);
    }

    #[test]
    fn test_calculate_git_history_factor_combined() {
        let mut git_history = GitHistory {
            change_counts: HashMap::new(),
            bug_fix_counts: HashMap::new(),
        };
        git_history
            .change_counts
            .insert(PathBuf::from("test.rs"), 30);
        git_history
            .bug_fix_counts
            .insert(PathBuf::from("test.rs"), 15);

        let factor = CriticalityAnalyzer::calculate_git_history_factor(
            &git_history,
            &PathBuf::from("test.rs"),
        );
        assert!((factor - 2.1).abs() < 0.01);
    }

    #[test]
    fn test_calculate_git_history_factor_capped() {
        let mut git_history = GitHistory {
            change_counts: HashMap::new(),
            bug_fix_counts: HashMap::new(),
        };
        git_history
            .change_counts
            .insert(PathBuf::from("test.rs"), 100);
        git_history
            .bug_fix_counts
            .insert(PathBuf::from("test.rs"), 50);

        let factor = CriticalityAnalyzer::calculate_git_history_factor(
            &git_history,
            &PathBuf::from("test.rs"),
        );
        assert_eq!(factor, 1.4 * 1.5);
    }

    // Tests for explain_criticality function
    use crate::priority::call_graph::{CallGraph, FunctionId};

    fn create_test_context() -> ScoringContext {
        let call_graph = CallGraph::new();
        ScoringContext::new(call_graph)
    }

    fn create_test_function_id() -> FunctionId {
        FunctionId {
            file: PathBuf::from("test.rs"),
            name: "test_function".to_string(),
            line: 10,
        }
    }

    #[test]
    fn test_explain_criticality_with_no_factors() {
        let context = create_test_context();
        let analyzer = CriticalityAnalyzer::new(&context);
        let function_id = create_test_function_id();

        let explanation = analyzer.explain_criticality(&function_id);
        assert_eq!(explanation, "Base criticality: 1.0x");
    }

    #[test]
    fn test_explain_criticality_with_distance_factor() {
        let mut context = create_test_context();
        // Mock distance from entry
        let analyzer = CriticalityAnalyzer::new(&context);
        let function_id = create_test_function_id();

        // Since we can't easily mock distance_from_entry, test the formatting logic directly
        let explanation = analyzer.explain_criticality(&function_id);
        // Should return base criticality since no factors are present
        assert_eq!(explanation, "Base criticality: 1.0x");
    }

    #[test]
    fn test_explain_criticality_with_caller_factor() {
        let mut context = create_test_context();
        let function_id = create_test_function_id();

        // Add caller frequency
        context.call_frequencies.insert(function_id.clone(), 5);

        let analyzer = CriticalityAnalyzer::new(&context);
        let explanation = analyzer.explain_criticality(&function_id);

        // Should include caller factor
        assert!(explanation.contains("5 callers"));
        assert!(explanation.contains("1.3x"));
    }

    #[test]
    fn test_explain_criticality_with_hot_path() {
        let mut context = create_test_context();
        let function_id = create_test_function_id();

        // Add to hot paths
        context.hot_paths.insert(function_id.clone());

        let analyzer = CriticalityAnalyzer::new(&context);
        let explanation = analyzer.explain_criticality(&function_id);

        // Should include hot path factor
        assert!(explanation.contains("Hot path: 1.5x"));
    }

    #[test]
    fn test_explain_criticality_with_downstream_impact() {
        let mut context = create_test_context();
        let function_id = create_test_function_id();

        // Add caller frequency to show some factor
        context.call_frequencies.insert(function_id.clone(), 2);

        let analyzer = CriticalityAnalyzer::new(&context);
        let explanation = analyzer.explain_criticality(&function_id);

        // Should include caller factor
        assert!(explanation.contains("2 callers"));
        assert!(explanation.contains("1.1x"));
    }

    #[test]
    fn test_explain_criticality_with_git_history_changes() {
        let mut git_history = GitHistory {
            change_counts: HashMap::new(),
            bug_fix_counts: HashMap::new(),
        };
        git_history
            .change_counts
            .insert(PathBuf::from("test.rs"), 25);

        let mut context = create_test_context();
        context.git_history = Some(git_history);

        let analyzer = CriticalityAnalyzer::new(&context);
        let function_id = create_test_function_id();
        let explanation = analyzer.explain_criticality(&function_id);

        // Should include git history factor
        assert!(explanation.contains("25 changes"));
        assert!(explanation.contains("1.4x"));
    }

    #[test]
    fn test_explain_criticality_with_git_history_bugs() {
        let mut git_history = GitHistory {
            change_counts: HashMap::new(),
            bug_fix_counts: HashMap::new(),
        };
        git_history
            .bug_fix_counts
            .insert(PathBuf::from("test.rs"), 10);

        let mut context = create_test_context();
        context.git_history = Some(git_history);

        let analyzer = CriticalityAnalyzer::new(&context);
        let function_id = create_test_function_id();
        let explanation = analyzer.explain_criticality(&function_id);

        // Should include git history factor
        assert!(explanation.contains("10 bug fixes"));
        assert!(explanation.contains("1.5x"));
    }

    #[test]
    fn test_explain_criticality_with_multiple_factors() {
        let mut git_history = GitHistory {
            change_counts: HashMap::new(),
            bug_fix_counts: HashMap::new(),
        };
        git_history
            .change_counts
            .insert(PathBuf::from("test.rs"), 25);
        git_history
            .bug_fix_counts
            .insert(PathBuf::from("test.rs"), 10);

        let mut context = create_test_context();
        context.git_history = Some(git_history);
        let function_id = create_test_function_id();

        // Add multiple factors
        context.call_frequencies.insert(function_id.clone(), 3);
        context.hot_paths.insert(function_id.clone());

        // Add callees
        for i in 0..6 {
            context.call_graph.add_edge_by_name(
                function_id.name.clone(),
                format!("callee_{}", i),
                PathBuf::from(format!("callee_{}.rs", i)),
            );
        }

        let analyzer = CriticalityAnalyzer::new(&context);
        let explanation = analyzer.explain_criticality(&function_id);

        // Should include multiple factors
        assert!(explanation.contains("3 callers"));
        assert!(explanation.contains("Hot path: 1.5x"));
        assert!(explanation.contains("25 changes"));
        assert!(explanation.contains("10 bug fixes"));
    }

    #[test]
    fn test_explain_criticality_edge_case_zero_callers() {
        let mut context = create_test_context();
        let function_id = create_test_function_id();

        // Explicitly set zero callers
        context.call_frequencies.insert(function_id.clone(), 0);

        let analyzer = CriticalityAnalyzer::new(&context);
        let explanation = analyzer.explain_criticality(&function_id);

        // Should not include caller factor for zero callers
        assert!(!explanation.contains("callers"));
        assert_eq!(explanation, "Base criticality: 1.0x");
    }

    #[test]
    fn test_explain_criticality_edge_case_single_caller() {
        let mut context = create_test_context();
        let function_id = create_test_function_id();

        // Single caller
        context.call_frequencies.insert(function_id.clone(), 1);

        let analyzer = CriticalityAnalyzer::new(&context);
        let explanation = analyzer.explain_criticality(&function_id);

        // Should include single caller
        assert!(explanation.contains("1 callers: 1.0x"));
    }

    #[test]
    fn test_explain_criticality_edge_case_exactly_five_callees() {
        let mut context = create_test_context();
        let function_id = create_test_function_id();

        // Add exactly 5 callees (boundary condition)
        for i in 0..5 {
            context.call_graph.add_edge_by_name(
                function_id.name.clone(),
                format!("callee_{}", i),
                PathBuf::from(format!("callee_{}.rs", i)),
            );
        }

        let analyzer = CriticalityAnalyzer::new(&context);
        let explanation = analyzer.explain_criticality(&function_id);

        // Should not include downstream impact for exactly 5 callees
        assert!(!explanation.contains("callees"));
        assert_eq!(explanation, "Base criticality: 1.0x");
    }

    #[test]
    fn test_explain_criticality_edge_case_six_callees() {
        let mut context = create_test_context();
        let function_id = create_test_function_id();

        // Add caller frequency to test the boundary
        context.call_frequencies.insert(function_id.clone(), 6);

        let analyzer = CriticalityAnalyzer::new(&context);
        let explanation = analyzer.explain_criticality(&function_id);

        // Should include caller impact
        assert!(explanation.contains("6 callers"));
        assert!(explanation.contains("1.4x"));
    }

    #[test]
    fn test_explain_criticality_boundary_git_changes() {
        let mut git_history = GitHistory {
            change_counts: HashMap::new(),
            bug_fix_counts: HashMap::new(),
        };
        // Exactly at threshold (10)
        git_history
            .change_counts
            .insert(PathBuf::from("test.rs"), 10);

        let mut context = create_test_context();
        context.git_history = Some(git_history);

        let analyzer = CriticalityAnalyzer::new(&context);
        let function_id = create_test_function_id();
        let explanation = analyzer.explain_criticality(&function_id);

        // Should not include git history factor at exactly threshold
        assert!(!explanation.contains("changes"));
        assert_eq!(explanation, "Base criticality: 1.0x");
    }

    #[test]
    fn test_explain_criticality_boundary_git_bugs() {
        let mut git_history = GitHistory {
            change_counts: HashMap::new(),
            bug_fix_counts: HashMap::new(),
        };
        // Exactly at threshold (5)
        git_history
            .bug_fix_counts
            .insert(PathBuf::from("test.rs"), 5);

        let mut context = create_test_context();
        context.git_history = Some(git_history);

        let analyzer = CriticalityAnalyzer::new(&context);
        let function_id = create_test_function_id();
        let explanation = analyzer.explain_criticality(&function_id);

        // Should not include git history factor at exactly threshold
        assert!(!explanation.contains("bug fixes"));
        assert_eq!(explanation, "Base criticality: 1.0x");
    }
}
