/// Debug and diagnostics for call graph resolution
///
/// This module provides comprehensive debug and diagnostic tools for the call graph system,
/// enabling developers and users to understand, validate, and troubleshoot call resolution issues.
use crate::priority::call_graph::FunctionId;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::time::Duration;

/// Resolution strategy used during call resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolutionStrategy {
    /// Exact name match
    Exact,
    /// Fuzzy matching with qualification
    Fuzzy,
    /// Name-only matching
    NameOnly,
}

/// Why a resolution attempt failed
#[derive(Debug, Clone)]
pub enum FailureReason {
    /// No candidates found
    NoCandidates,
    /// Multiple ambiguous candidates
    Ambiguous(Vec<FunctionId>),
    /// Candidates excluded by filters
    FilteredOut(String),
    /// Strategy not applicable
    NotApplicable,
}

/// Single strategy attempt details
#[derive(Debug, Clone)]
pub struct StrategyAttempt {
    /// Which strategy was tried
    pub strategy: ResolutionStrategy,
    /// Candidates found by this strategy
    pub candidates: Vec<FunctionId>,
    /// Why this attempt failed (if it did)
    pub failure_reason: Option<FailureReason>,
    /// Confidence score if successful
    pub confidence: Option<f32>,
}

/// Record of a single resolution attempt
#[derive(Debug, Clone)]
pub struct ResolutionAttempt {
    /// The caller function
    pub caller: FunctionId,
    /// The name being resolved
    pub callee_name: String,
    /// Resolution strategy attempts in order
    pub strategy_attempts: Vec<StrategyAttempt>,
    /// Final result (None if unresolved)
    pub result: Option<FunctionId>,
    /// Total time spent on resolution
    pub duration: Duration,
}

/// Statistics for a single resolution strategy
#[derive(Debug, Clone, Default)]
pub struct StrategyStats {
    /// Times this strategy was tried
    pub attempts: usize,
    /// Times this strategy succeeded
    pub successes: usize,
    /// Times this strategy failed
    pub failures: usize,
    /// Average confidence when successful
    pub avg_confidence: f32,
}

/// Time percentiles for resolution performance
#[derive(Debug, Clone, Default)]
pub struct Percentiles {
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

impl Percentiles {
    /// Calculate percentiles from a sorted list of durations
    fn from_sorted(durations: &[Duration]) -> Self {
        if durations.is_empty() {
            return Self::default();
        }

        let p50_idx = durations.len() / 2;
        let p95_idx = (durations.len() * 95) / 100;
        let p99_idx = (durations.len() * 99) / 100;

        Self {
            p50: durations.get(p50_idx).copied().unwrap_or_default(),
            p95: durations.get(p95_idx).copied().unwrap_or_default(),
            p99: durations.get(p99_idx).copied().unwrap_or_default(),
        }
    }
}

/// Statistics collected during resolution
#[derive(Debug, Clone, Default)]
pub struct ResolutionStatistics {
    /// Total calls attempted
    pub total_attempts: usize,
    /// Successfully resolved calls
    pub resolved: usize,
    /// Failed resolutions
    pub failed: usize,
    /// Breakdown by strategy
    pub by_strategy: HashMap<ResolutionStrategy, StrategyStats>,
    /// Resolution time distribution
    pub time_percentiles: Percentiles,
}

impl ResolutionStatistics {
    /// Calculate success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.0
        } else {
            (self.resolved as f64 / self.total_attempts as f64) * 100.0
        }
    }
}

/// Debug output format
#[derive(Debug, Clone, Copy)]
pub enum DebugFormat {
    Text,
    Json,
}

/// Configuration for debug output
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Include successful resolutions (not just failures)
    pub show_successes: bool,
    /// Include timing information
    pub show_timing: bool,
    /// Maximum candidates to show per attempt
    pub max_candidates_shown: usize,
    /// Output format (text or json)
    pub format: DebugFormat,
    /// Only show attempts for specific functions
    pub filter_functions: Option<HashSet<String>>,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            show_successes: false,
            show_timing: true,
            max_candidates_shown: 5,
            format: DebugFormat::Text,
            filter_functions: None,
        }
    }
}

/// Debug information collector for call graph resolution
pub struct CallGraphDebugger {
    /// All resolution attempts (successful and failed)
    attempts: Vec<ResolutionAttempt>,
    /// Functions to trace (if --trace-function specified)
    trace_functions: HashSet<String>,
    /// Statistics
    stats: ResolutionStatistics,
    /// Configuration
    config: DebugConfig,
}

impl CallGraphDebugger {
    /// Create a new debugger with configuration
    pub fn new(config: DebugConfig) -> Self {
        Self {
            attempts: Vec::new(),
            trace_functions: HashSet::new(),
            stats: ResolutionStatistics::default(),
            config,
        }
    }

    /// Add a function name to trace
    pub fn add_trace_function(&mut self, name: String) {
        self.trace_functions.insert(name);
    }

    /// Check if a function should be traced
    pub fn should_trace(&self, function_name: &str) -> bool {
        if self.trace_functions.is_empty() {
            return true; // Trace all if no specific functions specified
        }
        self.trace_functions.iter().any(|trace_name| {
            function_name.contains(trace_name) || trace_name.contains(function_name)
        })
    }

    /// Record a resolution attempt
    pub fn record_attempt(&mut self, attempt: ResolutionAttempt) {
        // Update statistics
        self.stats.total_attempts += 1;
        if attempt.result.is_some() {
            self.stats.resolved += 1;
        } else {
            self.stats.failed += 1;
        }

        // Update strategy statistics
        for strategy_attempt in &attempt.strategy_attempts {
            let stats = self
                .stats
                .by_strategy
                .entry(strategy_attempt.strategy)
                .or_default();

            stats.attempts += 1;
            if strategy_attempt.failure_reason.is_none() && strategy_attempt.confidence.is_some() {
                stats.successes += 1;
                if let Some(confidence) = strategy_attempt.confidence {
                    stats.avg_confidence = (stats.avg_confidence * (stats.successes - 1) as f32
                        + confidence)
                        / stats.successes as f32;
                }
            } else {
                stats.failures += 1;
            }
        }

        // Store attempt if it should be traced
        if self.should_trace(&attempt.caller.name) || self.should_trace(&attempt.callee_name) {
            self.attempts.push(attempt);
        }
    }

    /// Get resolution statistics
    pub fn statistics(&self) -> &ResolutionStatistics {
        &self.stats
    }

    /// Get all failed resolutions
    pub fn failed_resolutions(&self) -> Vec<&ResolutionAttempt> {
        self.attempts
            .iter()
            .filter(|attempt| attempt.result.is_none())
            .collect()
    }

    /// Finalize statistics (calculate percentiles)
    pub fn finalize_statistics(&mut self) {
        let mut durations: Vec<Duration> = self.attempts.iter().map(|a| a.duration).collect();
        durations.sort();
        self.stats.time_percentiles = Percentiles::from_sorted(&durations);
    }

    /// Generate text format debug report
    fn generate_text_report(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str("🔍 Call Graph Debug Report\n");
        output.push_str("════════════════════════════════════════\n\n");

        // Statistics
        output.push_str("📊 RESOLUTION STATISTICS\n");
        output.push_str(&format!(
            "  Total Attempts:    {}\n",
            self.stats.total_attempts
        ));
        output.push_str(&format!(
            "  Resolved:          {} ({:.1}%)\n",
            self.stats.resolved,
            self.stats.success_rate()
        ));
        output.push_str(&format!(
            "  Failed:            {} ({:.1}%)\n",
            self.stats.failed,
            100.0 - self.stats.success_rate()
        ));
        output.push('\n');

        // Strategy breakdown
        if !self.stats.by_strategy.is_empty() {
            output.push_str("  By Strategy:\n");
            for (strategy, stats) in &self.stats.by_strategy {
                let success_rate = if stats.attempts > 0 {
                    (stats.successes as f64 / stats.attempts as f64) * 100.0
                } else {
                    0.0
                };
                output.push_str(&format!(
                    "    {:?}: {} attempts ({:.1}% success)\n",
                    strategy, stats.attempts, success_rate
                ));
            }
            output.push('\n');
        }

        // Timing information
        if self.config.show_timing {
            output.push_str("  Resolution Time:\n");
            output.push_str(&format!(
                "    p50: {:.2}ms\n",
                self.stats.time_percentiles.p50.as_secs_f64() * 1000.0
            ));
            output.push_str(&format!(
                "    p95: {:.2}ms\n",
                self.stats.time_percentiles.p95.as_secs_f64() * 1000.0
            ));
            output.push_str(&format!(
                "    p99: {:.2}ms\n",
                self.stats.time_percentiles.p99.as_secs_f64() * 1000.0
            ));
            output.push('\n');
        }

        // Failed resolutions
        let failures = self.failed_resolutions();
        if !failures.is_empty() {
            output.push_str(&format!(
                "❌ FAILED RESOLUTIONS ({} total)\n\n",
                failures.len()
            ));

            for (idx, attempt) in failures.iter().enumerate().take(20) {
                // Limit to 20 for readability
                output.push_str(&format!("  {}. {}\n", idx + 1, attempt.callee_name));
                output.push_str(&format!("     Called from: {}\n", attempt.caller.name));
                output.push_str(&format!(
                    "     Location: {}:{}\n",
                    attempt.caller.file.display(),
                    attempt.caller.line
                ));
                output.push_str("\n     Strategy Attempts:\n");

                for (strategy_idx, strategy_attempt) in attempt.strategy_attempts.iter().enumerate()
                {
                    output.push_str(&format!(
                        "       {}. {:?} → ",
                        strategy_idx + 1,
                        strategy_attempt.strategy
                    ));

                    if let Some(reason) = &strategy_attempt.failure_reason {
                        match reason {
                            FailureReason::NoCandidates => {
                                output.push_str("No candidates\n");
                            }
                            FailureReason::Ambiguous(candidates) => {
                                output.push_str(&format!(
                                    "Found {} candidates (ambiguous)\n",
                                    candidates.len()
                                ));
                                for candidate in
                                    candidates.iter().take(self.config.max_candidates_shown)
                                {
                                    output.push_str(&format!(
                                        "          • {} ({}:{})\n",
                                        candidate.name,
                                        candidate.file.display(),
                                        candidate.line
                                    ));
                                }
                            }
                            FailureReason::FilteredOut(reason) => {
                                output.push_str(&format!("Filtered out: {}\n", reason));
                            }
                            FailureReason::NotApplicable => {
                                output.push_str("Not applicable\n");
                            }
                        }
                    } else {
                        output.push_str(&format!(
                            "Found {} candidates\n",
                            strategy_attempt.candidates.len()
                        ));
                    }
                }

                output.push('\n');
            }

            if failures.len() > 20 {
                output.push_str(&format!(
                    "  ... and {} more failed resolutions\n\n",
                    failures.len() - 20
                ));
            }
        }

        // Recommendations
        output.push_str("📈 RECOMMENDATIONS\n");
        let success_rate = self.stats.success_rate();
        if success_rate >= 95.0 {
            output.push_str(&format!(
                "  • {:.1}% resolution rate is excellent (target: >95%)\n",
                success_rate
            ));
        } else if success_rate >= 85.0 {
            output.push_str(&format!(
                "  • {:.1}% resolution rate is good (target: >95%)\n",
                success_rate
            ));
        } else {
            output.push_str(&format!(
                "  • {:.1}% resolution rate needs improvement (target: >95%)\n",
                success_rate
            ));
        }

        if self.stats.failed > 0 {
            output.push_str(&format!(
                "  • Investigate {} failed resolutions for patterns\n",
                self.stats.failed
            ));
        }

        output
    }

    /// Generate JSON format debug report
    fn generate_json_report(&self) -> String {
        use serde_json::json;

        let failed_resolutions: Vec<_> = self
            .failed_resolutions()
            .iter()
            .map(|attempt| {
                json!({
                    "caller": {
                        "function": attempt.caller.name,
                        "file": attempt.caller.file.display().to_string(),
                        "line": attempt.caller.line
                    },
                    "callee_name": attempt.callee_name,
                    "attempts": attempt.strategy_attempts.iter().map(|sa| {
                        let mut obj = json!({
                            "strategy": format!("{:?}", sa.strategy),
                            "candidates": sa.candidates.iter().map(|c| {
                                json!({
                                    "name": c.name,
                                    "file": c.file.display().to_string(),
                                    "line": c.line
                                })
                            }).collect::<Vec<_>>()
                        });

                        if let Some(reason) = &sa.failure_reason {
                            obj["failure_reason"] = match reason {
                                FailureReason::NoCandidates => json!("NoCandidates"),
                                FailureReason::Ambiguous(ids) => json!({
                                    "Ambiguous": ids.iter().map(|id| id.name.clone()).collect::<Vec<_>>()
                                }),
                                FailureReason::FilteredOut(reason) => json!({
                                    "FilteredOut": reason
                                }),
                                FailureReason::NotApplicable => json!("NotApplicable"),
                            };
                        }

                        obj
                    }).collect::<Vec<_>>()
                })
            })
            .collect();

        let report = json!({
            "statistics": {
                "total_attempts": self.stats.total_attempts,
                "resolved": self.stats.resolved,
                "failed": self.stats.failed,
                "success_rate": self.stats.success_rate() / 100.0,
                "by_strategy": self.stats.by_strategy.iter().map(|(strategy, stats)| {
                    (format!("{:?}", strategy), json!({
                        "attempts": stats.attempts,
                        "successes": stats.successes,
                        "failures": stats.failures,
                        "avg_confidence": stats.avg_confidence
                    }))
                }).collect::<serde_json::Map<String, serde_json::Value>>()
            },
            "failed_resolutions": failed_resolutions
        });

        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    }

    /// Output report to writer
    pub fn write_report<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let report = match self.config.format {
            DebugFormat::Text => self.generate_text_report(),
            DebugFormat::Json => self.generate_json_report(),
        };

        write!(writer, "{}", report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_debugger_creation() {
        let config = DebugConfig::default();
        let debugger = CallGraphDebugger::new(config);
        assert_eq!(debugger.statistics().total_attempts, 0);
        assert_eq!(debugger.statistics().resolved, 0);
        assert_eq!(debugger.statistics().failed, 0);
    }

    #[test]
    fn test_record_successful_attempt() {
        let mut debugger = CallGraphDebugger::new(DebugConfig::default());

        let caller = FunctionId::new(PathBuf::from("test.rs"), "caller".to_string(), 10);
        let callee = FunctionId::new(PathBuf::from("test.rs"), "callee".to_string(), 20);

        let attempt = ResolutionAttempt {
            caller: caller.clone(),
            callee_name: "callee".to_string(),
            strategy_attempts: vec![StrategyAttempt {
                strategy: ResolutionStrategy::Exact,
                candidates: vec![callee.clone()],
                failure_reason: None,
                confidence: Some(1.0),
            }],
            result: Some(callee),
            duration: Duration::from_millis(1),
        };

        debugger.record_attempt(attempt);

        assert_eq!(debugger.statistics().total_attempts, 1);
        assert_eq!(debugger.statistics().resolved, 1);
        assert_eq!(debugger.statistics().failed, 0);
    }

    #[test]
    fn test_record_failed_attempt() {
        let mut debugger = CallGraphDebugger::new(DebugConfig::default());

        let caller = FunctionId::new(PathBuf::from("test.rs"), "caller".to_string(), 10);

        let attempt = ResolutionAttempt {
            caller: caller.clone(),
            callee_name: "unknown".to_string(),
            strategy_attempts: vec![StrategyAttempt {
                strategy: ResolutionStrategy::Exact,
                candidates: vec![],
                failure_reason: Some(FailureReason::NoCandidates),
                confidence: None,
            }],
            result: None,
            duration: Duration::from_millis(2),
        };

        debugger.record_attempt(attempt);

        assert_eq!(debugger.statistics().total_attempts, 1);
        assert_eq!(debugger.statistics().resolved, 0);
        assert_eq!(debugger.statistics().failed, 1);
    }

    #[test]
    fn test_success_rate_calculation() {
        let mut stats = ResolutionStatistics::default();
        assert_eq!(stats.success_rate(), 0.0);

        stats.total_attempts = 100;
        stats.resolved = 95;
        stats.failed = 5;
        assert!((stats.success_rate() - 95.0).abs() < 0.01);
    }

    #[test]
    fn test_trace_function_filtering() {
        let mut debugger = CallGraphDebugger::new(DebugConfig::default());
        debugger.add_trace_function("specific_function".to_string());

        assert!(debugger.should_trace("specific_function"));
        assert!(debugger.should_trace("module::specific_function"));
        assert!(!debugger.should_trace("other_function"));
    }

    #[test]
    fn test_percentiles_calculation() {
        let durations = vec![
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(3),
            Duration::from_millis(4),
            Duration::from_millis(100),
        ];

        let mut sorted = durations.clone();
        sorted.sort();
        let percentiles = Percentiles::from_sorted(&sorted);

        // p50 should be median (3ms)
        assert_eq!(percentiles.p50, Duration::from_millis(3));
        // p95 and p99 should be the highest value for small samples
        assert!(percentiles.p95.as_millis() >= 3);
    }
}
