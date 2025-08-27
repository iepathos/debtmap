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
        let mut factors = Vec::new();

        // Distance from entry
        if let Some(distance) = self.context.distance_from_entry(function_id) {
            let factor = 2.0 / (1.0 + distance as f64 * 0.3);
            factors.push(format!("Entry distance {}: {:.1}x", distance, factor));
        }

        // Caller count
        if let Some(caller_count) = self.context.call_frequencies.get(function_id) {
            if *caller_count > 0 {
                let factor = 1.0 + (*caller_count as f64).ln() * 0.2;
                factors.push(format!("{} callers: {:.1}x", caller_count, factor.min(1.8)));
            }
        }

        // Hot path
        if self.context.hot_paths.contains(function_id) {
            factors.push("Hot path: 1.5x".to_string());
        }

        // Downstream impact
        let callee_count = self.context.call_graph.get_callees(function_id).len();
        if callee_count > 5 {
            let factor = 1.0 + (callee_count as f64 / 10.0);
            factors.push(format!("{} callees: {:.1}x", callee_count, factor.min(1.3)));
        }

        // Git history
        if let Some(git_history) = &self.context.git_history {
            if let Some(change_count) = git_history.change_counts.get(&function_id.file) {
                if *change_count > 10 {
                    let factor = 1.0 + (*change_count as f64 / 50.0);
                    factors.push(format!("{} changes: {:.1}x", change_count, factor.min(1.4)));
                }
            }

            if let Some(bug_count) = git_history.bug_fix_counts.get(&function_id.file) {
                if *bug_count > 5 {
                    let factor = 1.0 + (*bug_count as f64 / 20.0);
                    factors.push(format!("{} bug fixes: {:.1}x", bug_count, factor.min(1.5)));
                }
            }
        }

        if factors.is_empty() {
            "Base criticality: 1.0x".to_string()
        } else {
            factors.join(", ")
        }
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
}
