use super::{GodObjectAnalysis, GodObjectConfidence};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Metrics tracking for god object detection over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectMetrics {
    /// Historical snapshots of god object detections
    pub snapshots: Vec<GodObjectSnapshot>,
    /// Summary statistics across all snapshots
    pub summary: MetricsSummary,
}

impl Default for GodObjectMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl GodObjectMetrics {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            summary: MetricsSummary::default(),
        }
    }

    /// Record a new snapshot of god object analysis
    pub fn record_snapshot(&mut self, file_path: PathBuf, analysis: GodObjectAnalysis) {
        let snapshot = GodObjectSnapshot {
            timestamp: Utc::now(),
            file_path,
            is_god_object: analysis.is_god_object,
            method_count: analysis.method_count,
            field_count: analysis.field_count,
            responsibility_count: analysis.responsibility_count,
            lines_of_code: analysis.lines_of_code,
            god_object_score: analysis.god_object_score.value(),
            confidence: analysis.confidence,
        };

        self.snapshots.push(snapshot);
        self.update_summary();
    }

    /// Update summary statistics based on current snapshots
    fn update_summary(&mut self) {
        if self.snapshots.is_empty() {
            self.summary = MetricsSummary::default();
            return;
        }

        let mut total_god_objects = 0;
        let mut total_methods = 0;
        let mut total_score = 0.0;
        let mut file_metrics: HashMap<PathBuf, FileMetricHistory> = HashMap::new();

        for snapshot in &self.snapshots {
            if snapshot.is_god_object {
                total_god_objects += 1;
            }
            total_methods += snapshot.method_count;
            total_score += snapshot.god_object_score;

            // Track per-file metrics
            let file_entry = file_metrics
                .entry(snapshot.file_path.clone())
                .or_insert_with(|| FileMetricHistory {
                    file_path: snapshot.file_path.clone(),
                    first_seen: snapshot.timestamp,
                    last_seen: snapshot.timestamp,
                    max_methods: 0,
                    max_score: 0.0,
                    current_is_god_object: false,
                });

            file_entry.last_seen = snapshot.timestamp;
            file_entry.max_methods = file_entry.max_methods.max(snapshot.method_count);
            file_entry.max_score = file_entry.max_score.max(snapshot.god_object_score);
            file_entry.current_is_god_object = snapshot.is_god_object;
        }

        let avg_methods = total_methods as f64 / self.snapshots.len() as f64;
        let avg_score = total_score / self.snapshots.len() as f64;

        self.summary = MetricsSummary {
            total_snapshots: self.snapshots.len(),
            total_god_objects_detected: total_god_objects,
            average_method_count: avg_methods,
            average_god_object_score: avg_score,
            files_tracked: file_metrics.len(),
            file_histories: file_metrics.into_values().collect(),
        };
    }

    /// Get trend for a specific file
    pub fn get_file_trend(&self, file_path: &PathBuf) -> Option<FileTrend> {
        let file_snapshots: Vec<&GodObjectSnapshot> = self
            .snapshots
            .iter()
            .filter(|s| &s.file_path == file_path)
            .collect();

        if file_snapshots.len() < 2 {
            return None;
        }

        let first = file_snapshots.first()?;
        let last = file_snapshots.last()?;

        let method_change = last.method_count as i32 - first.method_count as i32;
        let score_change = last.god_object_score - first.god_object_score;

        Some(FileTrend {
            file_path: file_path.clone(),
            method_count_change: method_change,
            score_change,
            trend_direction: determine_trend(score_change),
            improved: score_change < 0.0,
        })
    }

    /// Get all files that became god objects
    pub fn get_new_god_objects(&self) -> Vec<PathBuf> {
        let mut new_god_objects = Vec::new();
        let mut file_status: HashMap<PathBuf, bool> = HashMap::new();

        // Process snapshots chronologically
        for snapshot in &self.snapshots {
            let was_god_object = file_status.get(&snapshot.file_path).copied();
            let is_god_object = snapshot.is_god_object;

            if !was_god_object.unwrap_or(false) && is_god_object {
                new_god_objects.push(snapshot.file_path.clone());
            }

            file_status.insert(snapshot.file_path.clone(), is_god_object);
        }

        new_god_objects
    }

    /// Get all files that stopped being god objects
    pub fn get_resolved_god_objects(&self) -> Vec<PathBuf> {
        let mut resolved = Vec::new();
        let mut file_status: HashMap<PathBuf, bool> = HashMap::new();

        for snapshot in &self.snapshots {
            let was_god_object = file_status.get(&snapshot.file_path).copied();
            let is_god_object = snapshot.is_god_object;

            if was_god_object.unwrap_or(false) && !is_god_object {
                resolved.push(snapshot.file_path.clone());
            }

            file_status.insert(snapshot.file_path.clone(), is_god_object);
        }

        resolved
    }
}

/// A single snapshot of god object analysis at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectSnapshot {
    pub timestamp: DateTime<Utc>,
    pub file_path: PathBuf,
    pub is_god_object: bool,
    pub method_count: usize,
    pub field_count: usize,
    pub responsibility_count: usize,
    pub lines_of_code: usize,
    pub god_object_score: f64,
    pub confidence: GodObjectConfidence,
}

/// Summary statistics across all snapshots
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricsSummary {
    pub total_snapshots: usize,
    pub total_god_objects_detected: usize,
    pub average_method_count: f64,
    pub average_god_object_score: f64,
    pub files_tracked: usize,
    pub file_histories: Vec<FileMetricHistory>,
}

/// Historical metrics for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetricHistory {
    pub file_path: PathBuf,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub max_methods: usize,
    pub max_score: f64,
    pub current_is_god_object: bool,
}

/// Trend analysis for a specific file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTrend {
    pub file_path: PathBuf,
    pub method_count_change: i32,
    pub score_change: f64,
    pub trend_direction: TrendDirection,
    pub improved: bool,
}

/// Direction of trend
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Improving,
    Stable,
    Worsening,
}

fn determine_trend(score_change: f64) -> TrendDirection {
    if score_change < -10.0 {
        TrendDirection::Improving
    } else if score_change > 10.0 {
        TrendDirection::Worsening
    } else {
        TrendDirection::Stable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::score_types::Score0To100;

    fn create_test_analysis(
        is_god_object: bool,
        method_count: usize,
        score: f64,
    ) -> GodObjectAnalysis {
        GodObjectAnalysis {
            is_god_object,
            method_count,
            weighted_method_count: None,
            field_count: 5,
            responsibility_count: 3,
            lines_of_code: 500,
            complexity_sum: 100,
            god_object_score: Score0To100::new(score),
            recommended_splits: Vec::new(),
            confidence: if is_god_object {
                GodObjectConfidence::Probable
            } else {
                GodObjectConfidence::NotGodObject
            },
            responsibilities: Vec::new(),
            responsibility_method_counts: Default::default(),
            purity_distribution: None,
            module_structure: None,
            detection_type: crate::organization::DetectionType::GodClass,
            struct_name: None,
            struct_line: None,
            struct_location: None, // Spec 201: Added for per-struct analysis
            visibility_breakdown: None, // Spec 134: Added for test compatibility
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::organization::SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None, // Spec 152: Added for test compatibility
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,   // Spec 211
            trait_method_summary: None, // Spec 217
        }
    }

    #[test]
    fn test_record_snapshot() {
        let mut metrics = GodObjectMetrics::new();
        let analysis = create_test_analysis(true, 30, 150.0);

        metrics.record_snapshot(PathBuf::from("test.rs"), analysis);

        assert_eq!(metrics.snapshots.len(), 1);
        assert_eq!(metrics.summary.total_snapshots, 1);
        assert_eq!(metrics.summary.total_god_objects_detected, 1);
    }

    #[test]
    fn test_multiple_snapshots() {
        let mut metrics = GodObjectMetrics::new();

        metrics.record_snapshot(
            PathBuf::from("file1.rs"),
            create_test_analysis(true, 30, 150.0),
        );
        metrics.record_snapshot(
            PathBuf::from("file2.rs"),
            create_test_analysis(false, 10, 50.0),
        );
        metrics.record_snapshot(
            PathBuf::from("file3.rs"),
            create_test_analysis(true, 50, 250.0),
        );

        assert_eq!(metrics.snapshots.len(), 3);
        assert_eq!(metrics.summary.total_god_objects_detected, 2);
        assert_eq!(metrics.summary.files_tracked, 3);
        assert_eq!(metrics.summary.average_method_count, 30.0);
    }

    #[test]
    fn test_file_trend() {
        let mut metrics = GodObjectMetrics::new();
        let file_path = PathBuf::from("evolving.rs");

        // First snapshot - not a god object
        metrics.record_snapshot(file_path.clone(), create_test_analysis(false, 15, 75.0));

        // Second snapshot - became a god object
        metrics.record_snapshot(file_path.clone(), create_test_analysis(true, 35, 175.0));

        let trend = metrics.get_file_trend(&file_path).unwrap();
        assert_eq!(trend.method_count_change, 20);
        // Score goes from 75.0 to 100.0 (175.0 clamped), so change is 25.0
        assert_eq!(trend.score_change, 25.0);
        assert_eq!(trend.trend_direction, TrendDirection::Worsening);
        assert!(!trend.improved);
    }

    #[test]
    fn test_new_god_objects() {
        let mut metrics = GodObjectMetrics::new();

        metrics.record_snapshot(
            PathBuf::from("file1.rs"),
            create_test_analysis(false, 10, 50.0),
        );
        metrics.record_snapshot(
            PathBuf::from("file1.rs"),
            create_test_analysis(true, 30, 150.0),
        );
        metrics.record_snapshot(
            PathBuf::from("file2.rs"),
            create_test_analysis(true, 25, 125.0),
        );

        let new_god_objects = metrics.get_new_god_objects();
        assert_eq!(new_god_objects.len(), 2);
        assert!(new_god_objects.contains(&PathBuf::from("file1.rs")));
        assert!(new_god_objects.contains(&PathBuf::from("file2.rs")));
    }

    #[test]
    fn test_resolved_god_objects() {
        let mut metrics = GodObjectMetrics::new();

        metrics.record_snapshot(
            PathBuf::from("file1.rs"),
            create_test_analysis(true, 30, 150.0),
        );
        metrics.record_snapshot(
            PathBuf::from("file1.rs"),
            create_test_analysis(false, 15, 75.0),
        );

        let resolved = metrics.get_resolved_god_objects();
        assert_eq!(resolved.len(), 1);
        assert!(resolved.contains(&PathBuf::from("file1.rs")));
    }

    #[test]
    fn test_trend_direction() {
        assert_eq!(determine_trend(-20.0), TrendDirection::Improving);
        assert_eq!(determine_trend(0.0), TrendDirection::Stable);
        assert_eq!(determine_trend(5.0), TrendDirection::Stable);
        assert_eq!(determine_trend(20.0), TrendDirection::Worsening);
    }
}
