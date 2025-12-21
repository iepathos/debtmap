//! Writer Effect for Analysis Telemetry
//!
//! This module provides the Writer Effect pattern for collecting analysis telemetry
//! without threading state through function parameters. Using this approach:
//!
//! - Function signatures stay clean (no telemetry receivers to pass)
//! - Testing is easier (no need to mock telemetry infrastructure)
//! - Business logic is decoupled from logging/metrics concerns
//! - Events accumulate automatically alongside computation
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::effects::telemetry::{tell_event, AnalysisEvent, AnalysisMetrics};
//! use stillwater::WriterEffectExt;
//!
//! fn analyze_file(path: &Path) -> impl WriterEffect<Output = FileMetrics, Writes = AnalysisMetrics> {
//!     let start = Instant::now();
//!     tell_event(AnalysisEvent::FileStarted { path: path.to_path_buf() })
//!         .and_then(|_| do_analysis(path))
//!         .tap_tell(|metrics| AnalysisMetrics::event(
//!             AnalysisEvent::ComplexityCalculated {
//!                 path: path.to_path_buf(),
//!                 cognitive: metrics.cognitive,
//!                 cyclomatic: metrics.cyclomatic,
//!             }
//!         ))
//! }
//!
//! // Execute and collect telemetry
//! let (result, metrics) = analyze_file(path).run_writer(&env).await;
//! let summary: AnalysisSummary = metrics.into();
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use stillwater::effect::writer::{tell, WriterEffect};
use stillwater::{Monoid, Semigroup};

use crate::core::types::Severity;

/// Analysis phases for phase-level telemetry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnalysisPhase {
    /// File discovery phase
    Discovery,
    /// Source code parsing phase
    Parsing,
    /// Complexity calculation phase
    Complexity,
    /// Debt detection phase
    DebtDetection,
    /// Risk assessment phase
    RiskAssessment,
    /// Report generation phase
    Reporting,
}

impl AnalysisPhase {
    /// Get a display name for the phase.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Discovery => "Discovery",
            Self::Parsing => "Parsing",
            Self::Complexity => "Complexity Analysis",
            Self::DebtDetection => "Debt Detection",
            Self::RiskAssessment => "Risk Assessment",
            Self::Reporting => "Report Generation",
        }
    }
}

/// Events emitted during analysis for telemetry collection.
///
/// These events capture key analysis milestones without coupling
/// the analysis logic to any specific logging/metrics infrastructure.
#[derive(Debug, Clone)]
pub enum AnalysisEvent {
    /// Emitted when file analysis begins.
    FileStarted {
        /// Path to the file being analyzed.
        path: PathBuf,
        /// Timestamp when analysis started.
        timestamp: Instant,
    },

    /// Emitted when file analysis completes successfully.
    FileCompleted {
        /// Path to the analyzed file.
        path: PathBuf,
        /// Duration of analysis in milliseconds.
        duration_ms: u64,
    },

    /// Emitted when file analysis fails.
    FileFailed {
        /// Path to the file that failed.
        path: PathBuf,
        /// Error message.
        error: String,
    },

    /// Emitted after successful parsing.
    ParseComplete {
        /// Path to the parsed file.
        path: PathBuf,
        /// Number of functions found.
        function_count: usize,
    },

    /// Emitted when complexity is calculated for a file.
    ComplexityCalculated {
        /// Path to the analyzed file.
        path: PathBuf,
        /// Cognitive complexity score.
        cognitive: u32,
        /// Cyclomatic complexity score.
        cyclomatic: u32,
    },

    /// Emitted when a debt item is detected.
    DebtItemDetected {
        /// Path where debt was found.
        path: PathBuf,
        /// Severity of the debt item.
        severity: Severity,
        /// Category of the debt (e.g., "complexity", "duplication").
        category: String,
    },

    /// Emitted when an analysis phase starts.
    PhaseStarted {
        /// The phase that started.
        phase: AnalysisPhase,
        /// Timestamp when the phase started.
        timestamp: Instant,
    },

    /// Emitted when an analysis phase completes.
    PhaseCompleted {
        /// The phase that completed.
        phase: AnalysisPhase,
        /// Duration of the phase in milliseconds.
        duration_ms: u64,
    },
}

impl AnalysisEvent {
    /// Create a file started event with current timestamp.
    pub fn file_started(path: PathBuf) -> Self {
        Self::FileStarted {
            path,
            timestamp: Instant::now(),
        }
    }

    /// Create a file completed event.
    pub fn file_completed(path: PathBuf, duration_ms: u64) -> Self {
        Self::FileCompleted { path, duration_ms }
    }

    /// Create a file failed event.
    pub fn file_failed(path: PathBuf, error: impl Into<String>) -> Self {
        Self::FileFailed {
            path,
            error: error.into(),
        }
    }

    /// Create a parse complete event.
    pub fn parse_complete(path: PathBuf, function_count: usize) -> Self {
        Self::ParseComplete {
            path,
            function_count,
        }
    }

    /// Create a complexity calculated event.
    pub fn complexity_calculated(path: PathBuf, cognitive: u32, cyclomatic: u32) -> Self {
        Self::ComplexityCalculated {
            path,
            cognitive,
            cyclomatic,
        }
    }

    /// Create a debt item detected event.
    pub fn debt_detected(path: PathBuf, severity: Severity, category: impl Into<String>) -> Self {
        Self::DebtItemDetected {
            path,
            severity,
            category: category.into(),
        }
    }

    /// Create a phase started event with current timestamp.
    pub fn phase_started(phase: AnalysisPhase) -> Self {
        Self::PhaseStarted {
            phase,
            timestamp: Instant::now(),
        }
    }

    /// Create a phase completed event.
    pub fn phase_completed(phase: AnalysisPhase, duration_ms: u64) -> Self {
        Self::PhaseCompleted { phase, duration_ms }
    }
}

/// Collection of analysis events with Monoid-based accumulation.
///
/// This type implements `Semigroup` and `Monoid` to enable automatic
/// accumulation via the Writer Effect pattern. Events are stored in
/// a `Vec` for efficient appending.
#[derive(Debug, Clone, Default)]
pub struct AnalysisMetrics {
    /// Collected analysis events.
    pub events: Vec<AnalysisEvent>,
}

impl AnalysisMetrics {
    /// Create a new empty metrics collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create metrics containing a single event.
    pub fn event(event: AnalysisEvent) -> Self {
        Self {
            events: vec![event],
        }
    }

    /// Create metrics from multiple events.
    pub fn events(events: Vec<AnalysisEvent>) -> Self {
        Self { events }
    }

    /// Get the number of events collected.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if there are no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Iterate over events.
    pub fn iter(&self) -> impl Iterator<Item = &AnalysisEvent> {
        self.events.iter()
    }

    /// Filter events by predicate.
    pub fn filter<F>(&self, predicate: F) -> Self
    where
        F: Fn(&AnalysisEvent) -> bool,
    {
        Self {
            events: self
                .events
                .iter()
                .filter(|e| predicate(e))
                .cloned()
                .collect(),
        }
    }

    /// Get only file started events.
    pub fn file_started_events(&self) -> impl Iterator<Item = &AnalysisEvent> {
        self.events
            .iter()
            .filter(|e| matches!(e, AnalysisEvent::FileStarted { .. }))
    }

    /// Get only file completed events.
    pub fn file_completed_events(&self) -> impl Iterator<Item = &AnalysisEvent> {
        self.events
            .iter()
            .filter(|e| matches!(e, AnalysisEvent::FileCompleted { .. }))
    }

    /// Get only debt detected events.
    pub fn debt_detected_events(&self) -> impl Iterator<Item = &AnalysisEvent> {
        self.events
            .iter()
            .filter(|e| matches!(e, AnalysisEvent::DebtItemDetected { .. }))
    }
}

/// Semigroup implementation for AnalysisMetrics.
///
/// Combines two metrics by concatenating their event vectors.
impl Semigroup for AnalysisMetrics {
    fn combine(mut self, other: Self) -> Self {
        self.events.extend(other.events);
        self
    }
}

/// Monoid implementation for AnalysisMetrics.
///
/// The identity element is an empty metrics collection.
impl Monoid for AnalysisMetrics {
    fn empty() -> Self {
        Self::default()
    }
}

/// Summary statistics aggregated from analysis events.
///
/// This type provides a high-level view of analysis results,
/// derived from the collected events.
#[derive(Debug, Clone, Default)]
pub struct AnalysisSummary {
    /// Number of files processed.
    pub files_processed: usize,
    /// Number of files that failed processing.
    pub files_failed: usize,
    /// Total duration across all files (milliseconds).
    pub total_duration_ms: u64,
    /// Total number of functions parsed.
    pub total_functions: usize,
    /// Average cognitive complexity across files.
    pub avg_cognitive_complexity: f64,
    /// Average cyclomatic complexity across files.
    pub avg_cyclomatic_complexity: f64,
    /// Debt items grouped by severity.
    pub debt_items_by_severity: HashMap<Severity, usize>,
    /// Debt items grouped by category.
    pub debt_items_by_category: HashMap<String, usize>,
    /// Phase durations in milliseconds.
    pub phase_durations: HashMap<AnalysisPhase, u64>,
}

impl AnalysisSummary {
    /// Total number of debt items detected.
    pub fn total_debt_items(&self) -> usize {
        self.debt_items_by_severity.values().sum()
    }

    /// Average file processing time in milliseconds.
    pub fn avg_file_duration_ms(&self) -> f64 {
        if self.files_processed == 0 {
            0.0
        } else {
            self.total_duration_ms as f64 / self.files_processed as f64
        }
    }
}

impl From<AnalysisMetrics> for AnalysisSummary {
    fn from(metrics: AnalysisMetrics) -> Self {
        let mut summary = AnalysisSummary::default();
        let mut cognitive_sum: u64 = 0;
        let mut cyclomatic_sum: u64 = 0;
        let mut complexity_count: usize = 0;

        for event in metrics.events {
            match event {
                AnalysisEvent::FileStarted { .. } => {
                    // Counted via FileCompleted
                }
                AnalysisEvent::FileCompleted { duration_ms, .. } => {
                    summary.files_processed += 1;
                    summary.total_duration_ms += duration_ms;
                }
                AnalysisEvent::FileFailed { .. } => {
                    summary.files_failed += 1;
                }
                AnalysisEvent::ParseComplete { function_count, .. } => {
                    summary.total_functions += function_count;
                }
                AnalysisEvent::ComplexityCalculated {
                    cognitive,
                    cyclomatic,
                    ..
                } => {
                    cognitive_sum += cognitive as u64;
                    cyclomatic_sum += cyclomatic as u64;
                    complexity_count += 1;
                }
                AnalysisEvent::DebtItemDetected {
                    severity, category, ..
                } => {
                    *summary.debt_items_by_severity.entry(severity).or_insert(0) += 1;
                    *summary.debt_items_by_category.entry(category).or_insert(0) += 1;
                }
                AnalysisEvent::PhaseStarted { .. } => {
                    // Timing captured via PhaseCompleted
                }
                AnalysisEvent::PhaseCompleted { phase, duration_ms } => {
                    summary.phase_durations.insert(phase, duration_ms);
                }
            }
        }

        // Calculate averages
        if complexity_count > 0 {
            summary.avg_cognitive_complexity = cognitive_sum as f64 / complexity_count as f64;
            summary.avg_cyclomatic_complexity = cyclomatic_sum as f64 / complexity_count as f64;
        }

        summary
    }
}

/// Emit a single analysis event to be accumulated.
///
/// This is the primary helper for emitting telemetry events
/// within a Writer Effect context.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::telemetry::{tell_event, AnalysisEvent};
///
/// let effect = tell_event(AnalysisEvent::file_started(path.to_path_buf()));
/// ```
pub fn tell_event<E, Env>(
    event: AnalysisEvent,
) -> impl WriterEffect<Output = (), Error = E, Env = Env, Writes = AnalysisMetrics>
where
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
{
    tell(AnalysisMetrics::event(event))
}

/// Emit multiple analysis events to be accumulated.
///
/// Use this when you have several events to emit at once.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::telemetry::{tell_events, AnalysisEvent};
///
/// let effect = tell_events(vec![
///     AnalysisEvent::parse_complete(path.clone(), 5),
///     AnalysisEvent::complexity_calculated(path, 10, 8),
/// ]);
/// ```
pub fn tell_events<E, Env>(
    events: Vec<AnalysisEvent>,
) -> impl WriterEffect<Output = (), Error = E, Env = Env, Writes = AnalysisMetrics>
where
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
{
    tell(AnalysisMetrics::events(events))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_metrics_monoid_empty() {
        let empty = AnalysisMetrics::empty();
        assert!(empty.is_empty());
    }

    #[test]
    fn analysis_metrics_monoid_identity() {
        let metrics = AnalysisMetrics::event(AnalysisEvent::file_started(PathBuf::from("test.rs")));
        let empty = AnalysisMetrics::empty();

        // Right identity: a.combine(empty) == a
        let combined = metrics.clone().combine(empty.clone());
        assert_eq!(combined.len(), 1);

        // Left identity: empty.combine(a) == a
        let combined = empty.combine(metrics.clone());
        assert_eq!(combined.len(), 1);
    }

    #[test]
    fn analysis_metrics_semigroup_combine() {
        let m1 = AnalysisMetrics::event(AnalysisEvent::file_started(PathBuf::from("a.rs")));
        let m2 = AnalysisMetrics::event(AnalysisEvent::file_started(PathBuf::from("b.rs")));
        let m3 = AnalysisMetrics::event(AnalysisEvent::file_started(PathBuf::from("c.rs")));

        // Verify combination adds events
        let combined = m1.clone().combine(m2.clone());
        assert_eq!(combined.len(), 2);

        // Verify associativity: (a.combine(b)).combine(c) == a.combine(b.combine(c))
        let left = m1.clone().combine(m2.clone()).combine(m3.clone());
        let right = m1.combine(m2.combine(m3));
        assert_eq!(left.len(), right.len());
        assert_eq!(left.len(), 3);
    }

    #[test]
    fn analysis_summary_from_metrics() {
        let metrics = AnalysisMetrics::events(vec![
            AnalysisEvent::file_completed(PathBuf::from("a.rs"), 100),
            AnalysisEvent::file_completed(PathBuf::from("b.rs"), 200),
            AnalysisEvent::file_failed(PathBuf::from("c.rs"), "parse error"),
            AnalysisEvent::parse_complete(PathBuf::from("a.rs"), 5),
            AnalysisEvent::parse_complete(PathBuf::from("b.rs"), 10),
            AnalysisEvent::complexity_calculated(PathBuf::from("a.rs"), 10, 8),
            AnalysisEvent::complexity_calculated(PathBuf::from("b.rs"), 20, 16),
            AnalysisEvent::debt_detected(PathBuf::from("a.rs"), Severity::Warning, "complexity"),
            AnalysisEvent::debt_detected(PathBuf::from("b.rs"), Severity::Critical, "security"),
        ]);

        let summary: AnalysisSummary = metrics.into();

        assert_eq!(summary.files_processed, 2);
        assert_eq!(summary.files_failed, 1);
        assert_eq!(summary.total_duration_ms, 300);
        assert_eq!(summary.total_functions, 15);
        assert!((summary.avg_cognitive_complexity - 15.0).abs() < 0.01); // (10+20)/2
        assert!((summary.avg_cyclomatic_complexity - 12.0).abs() < 0.01); // (8+16)/2
        assert_eq!(summary.total_debt_items(), 2);
        assert_eq!(
            summary.debt_items_by_severity.get(&Severity::Warning),
            Some(&1)
        );
        assert_eq!(
            summary.debt_items_by_severity.get(&Severity::Critical),
            Some(&1)
        );
        assert_eq!(summary.debt_items_by_category.get("complexity"), Some(&1));
        assert_eq!(summary.debt_items_by_category.get("security"), Some(&1));
    }

    #[test]
    fn analysis_summary_avg_file_duration() {
        let summary = AnalysisSummary {
            files_processed: 4,
            total_duration_ms: 400,
            ..Default::default()
        };
        assert!((summary.avg_file_duration_ms() - 100.0).abs() < 0.01);
    }

    #[test]
    fn analysis_summary_avg_file_duration_empty() {
        let summary = AnalysisSummary::default();
        assert!((summary.avg_file_duration_ms()).abs() < 0.01);
    }

    #[test]
    fn analysis_event_constructors() {
        let path = PathBuf::from("test.rs");

        let event = AnalysisEvent::file_started(path.clone());
        assert!(matches!(event, AnalysisEvent::FileStarted { .. }));

        let event = AnalysisEvent::file_completed(path.clone(), 100);
        assert!(matches!(
            event,
            AnalysisEvent::FileCompleted {
                duration_ms: 100,
                ..
            }
        ));

        let event = AnalysisEvent::file_failed(path.clone(), "error");
        assert!(matches!(event, AnalysisEvent::FileFailed { .. }));

        let event = AnalysisEvent::parse_complete(path.clone(), 5);
        assert!(matches!(
            event,
            AnalysisEvent::ParseComplete {
                function_count: 5,
                ..
            }
        ));

        let event = AnalysisEvent::complexity_calculated(path.clone(), 10, 8);
        assert!(matches!(
            event,
            AnalysisEvent::ComplexityCalculated {
                cognitive: 10,
                cyclomatic: 8,
                ..
            }
        ));

        let event = AnalysisEvent::debt_detected(path.clone(), Severity::Warning, "complexity");
        assert!(matches!(
            event,
            AnalysisEvent::DebtItemDetected {
                severity: Severity::Warning,
                ..
            }
        ));

        let event = AnalysisEvent::phase_started(AnalysisPhase::Parsing);
        assert!(matches!(
            event,
            AnalysisEvent::PhaseStarted {
                phase: AnalysisPhase::Parsing,
                ..
            }
        ));

        let event = AnalysisEvent::phase_completed(AnalysisPhase::Complexity, 50);
        assert!(matches!(
            event,
            AnalysisEvent::PhaseCompleted {
                phase: AnalysisPhase::Complexity,
                duration_ms: 50
            }
        ));
    }

    #[test]
    fn analysis_metrics_filter() {
        let metrics = AnalysisMetrics::events(vec![
            AnalysisEvent::file_started(PathBuf::from("a.rs")),
            AnalysisEvent::file_completed(PathBuf::from("a.rs"), 100),
            AnalysisEvent::file_started(PathBuf::from("b.rs")),
            AnalysisEvent::file_completed(PathBuf::from("b.rs"), 200),
        ]);

        let started_only = metrics.filter(|e| matches!(e, AnalysisEvent::FileStarted { .. }));
        assert_eq!(started_only.len(), 2);

        let completed_only = metrics.filter(|e| matches!(e, AnalysisEvent::FileCompleted { .. }));
        assert_eq!(completed_only.len(), 2);
    }

    #[test]
    fn analysis_phase_display_name() {
        assert_eq!(AnalysisPhase::Discovery.display_name(), "Discovery");
        assert_eq!(AnalysisPhase::Parsing.display_name(), "Parsing");
        assert_eq!(
            AnalysisPhase::Complexity.display_name(),
            "Complexity Analysis"
        );
        assert_eq!(
            AnalysisPhase::DebtDetection.display_name(),
            "Debt Detection"
        );
        assert_eq!(
            AnalysisPhase::RiskAssessment.display_name(),
            "Risk Assessment"
        );
        assert_eq!(AnalysisPhase::Reporting.display_name(), "Report Generation");
    }

    #[tokio::test]
    async fn writer_effect_collects_single_event() {
        // Emit a single event and collect it
        let effect = tell_event::<(), ()>(AnalysisEvent::file_started(PathBuf::from("test.rs")));
        let (_, metrics) = effect.run_writer(&()).await;

        assert_eq!(metrics.len(), 1);
        assert!(metrics
            .iter()
            .any(|e| matches!(e, AnalysisEvent::FileStarted { .. })));
    }

    #[tokio::test]
    async fn writer_effect_with_chained_events() {
        use stillwater::EffectExt;

        // Use writer-specific and_then to chain two tell operations
        let effect1 = tell_event::<(), ()>(AnalysisEvent::file_started(PathBuf::from("test.rs")));
        let effect2 =
            tell_event::<(), ()>(AnalysisEvent::file_completed(PathBuf::from("test.rs"), 50));

        // Chain using EffectExt::and_then which composes effects
        let effect = effect1.and_then(|_| effect2);

        let (_, metrics) = effect.run_writer(&()).await;
        assert_eq!(metrics.len(), 2);

        let summary: AnalysisSummary = metrics.into();
        assert_eq!(summary.files_processed, 1);
        assert_eq!(summary.total_duration_ms, 50);
    }

    #[tokio::test]
    async fn writer_effect_tap_tell_accumulates() {
        use stillwater::effect::writer::WriterEffectExt;

        // Use tap_tell to add events based on intermediate values
        let effect = tell_event::<(), ()>(AnalysisEvent::phase_started(AnalysisPhase::Complexity))
            .tap_tell(|_| {
                AnalysisMetrics::event(AnalysisEvent::phase_completed(
                    AnalysisPhase::Complexity,
                    10,
                ))
            });

        let (_, metrics) = effect.run_writer(&()).await;
        assert_eq!(metrics.len(), 2);

        // Verify we have both phase events
        let has_started = metrics.iter().any(|e| {
            matches!(
                e,
                AnalysisEvent::PhaseStarted {
                    phase: AnalysisPhase::Complexity,
                    ..
                }
            )
        });
        let has_completed = metrics.iter().any(|e| {
            matches!(
                e,
                AnalysisEvent::PhaseCompleted {
                    phase: AnalysisPhase::Complexity,
                    ..
                }
            )
        });
        assert!(has_started);
        assert!(has_completed);
    }
}
