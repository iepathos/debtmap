use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::scoring::scoring_context::ScoringContext;
use std::collections::HashSet;

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

    pub fn calculate_criticality_for_id(&self, function_id: &FunctionId) -> f64 {
        let mut score = 1.0;
        
        // Factor 1: Distance from entry points (closer = more critical)
        if let Some(distance) = self.context.distance_from_entry(function_id) {
            // Exponential decay based on distance
            // Distance 0 (entry point) = 2.0x
            // Distance 1 = 1.7x
            // Distance 2 = 1.4x
            // Distance 3+ = 1.2x to 1.0x
            score *= 2.0 / (1.0 + distance as f64 * 0.3);
        }
        
        // Factor 2: Number of callers (fan-in)
        let caller_count = self.context.call_frequencies
            .get(function_id)
            .copied()
            .unwrap_or(0);
        
        if caller_count > 0 {
            // Logarithmic scaling for caller count
            // 1 caller = 1.0x
            // 2 callers = 1.15x
            // 5 callers = 1.4x
            // 10 callers = 1.6x
            // 20+ callers = 1.8x
            let caller_factor = 1.0 + (caller_count as f64).ln() * 0.2;
            score *= caller_factor.min(1.8);
        }
        
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
            if let Some(change_count) = git_history.change_counts.get(&function_id.file) {
                // Frequently changed files are more critical
                if *change_count > 10 {
                    let change_factor = 1.0 + (*change_count as f64 / 50.0);
                    score *= change_factor.min(1.4);
                }
            }
            
            if let Some(bug_count) = git_history.bug_fix_counts.get(&function_id.file) {
                // Files with many bug fixes are critical
                if *bug_count > 5 {
                    let bug_factor = 1.0 + (*bug_count as f64 / 20.0);
                    score *= bug_factor.min(1.5);
                }
            }
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