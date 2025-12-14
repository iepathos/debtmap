//! Anti-pattern output types for JSON serialization (spec 197)
//!
//! Provides output structures for detected anti-patterns including quality scores,
//! individual pattern items, and summary counts by severity.

use crate::organization::anti_pattern_detector::{
    AntiPattern, AntiPatternSeverity, AntiPatternType,
};
use serde::{Deserialize, Serialize};

/// Anti-pattern output for JSON serialization (spec 197)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPatternOutput {
    /// Quality score from anti-pattern analysis (0-100, higher = better)
    pub quality_score: f64,
    /// List of detected anti-patterns
    pub patterns: Vec<AntiPatternItem>,
    /// Summary counts by severity
    pub summary: AntiPatternSummary,
}

/// Individual anti-pattern item for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPatternItem {
    /// Type of anti-pattern detected
    pub pattern_type: AntiPatternType,
    /// Severity level
    pub severity: AntiPatternSeverity,
    /// Location where the anti-pattern was detected
    pub location: String,
    /// Human-readable description of the issue
    pub description: String,
    /// Recommended correction action
    pub recommendation: String,
    /// Affected methods (if applicable)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_methods: Vec<String>,
}

/// Summary counts of anti-patterns by severity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AntiPatternSummary {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

impl From<&AntiPattern> for AntiPatternItem {
    fn from(pattern: &AntiPattern) -> Self {
        Self {
            pattern_type: pattern.pattern_type.clone(),
            severity: pattern.severity.clone(),
            location: pattern.location.clone(),
            description: pattern.description.clone(),
            recommendation: pattern.correction.clone(),
            affected_methods: pattern.affected_methods.clone(),
        }
    }
}

/// Build AntiPatternOutput from FileDebtMetrics (spec 197)
pub fn build_anti_patterns(
    metrics: &crate::priority::FileDebtMetrics,
) -> Option<AntiPatternOutput> {
    // Get anti-pattern report from god object analysis
    let report = metrics
        .god_object_analysis
        .as_ref()
        .and_then(|a| a.anti_pattern_report.as_ref())?;

    // Convert patterns to output format
    let patterns: Vec<AntiPatternItem> = report.anti_patterns.iter().map(|p| p.into()).collect();

    // Build summary counts
    let mut summary = AntiPatternSummary::default();
    for pattern in &report.anti_patterns {
        match pattern.severity {
            AntiPatternSeverity::Critical => summary.critical += 1,
            AntiPatternSeverity::High => summary.high += 1,
            AntiPatternSeverity::Medium => summary.medium += 1,
            AntiPatternSeverity::Low => summary.low += 1,
        }
    }

    // Only return if there are patterns detected
    if patterns.is_empty() && summary.critical == 0 && summary.high == 0 {
        return None;
    }

    Some(AntiPatternOutput {
        quality_score: report.quality_score,
        patterns,
        summary,
    })
}
