//! Evidence Formatter for Multi-Signal Classification Display
//!
//! Provides formatted output showing:
//! - Which signals contributed to each classification
//! - Individual signal confidences and weights
//! - Combined confidence scores
//! - Specific evidence from each signal
//! - Alternative classifications when ambiguous

use crate::analysis::multi_signal_aggregation::{
    AggregatedClassification, ResponsibilityCategory, SignalEvidence as AggregatedSignalEvidence,
    SignalType,
};

/// Verbosity level for evidence display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbosityLevel {
    /// Only show confidence and primary category
    Minimal,
    /// Show signal summary with top contributors (default)
    Standard,
    /// Show all details including technical evidence
    Verbose,
}

impl VerbosityLevel {
    /// Create verbosity level from count (0, 1, 2, 3+)
    pub fn from_count(count: u8) -> Self {
        match count {
            0 => Self::Minimal,
            1 => Self::Standard,
            _ => Self::Verbose,
        }
    }
}

/// Confidence band for classification quality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceBand {
    High,   // >= 0.80
    Medium, // 0.60 - 0.80
    Low,    // < 0.60
}

impl ConfidenceBand {
    /// Determine confidence band from score
    pub fn from_score(score: f64) -> Self {
        if score >= 0.80 {
            Self::High
        } else if score >= 0.60 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    /// Get display string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }
}

/// Signal filtering configuration
#[derive(Debug, Clone)]
pub struct SignalFilters {
    pub show_io_detection: bool,
    pub show_call_graph: bool,
    pub show_type_signatures: bool,
    pub show_purity: bool,
    pub show_framework: bool,
    pub show_name_heuristics: bool,
}

impl Default for SignalFilters {
    fn default() -> Self {
        Self {
            show_io_detection: true,
            show_call_graph: true,
            show_type_signatures: true,
            show_purity: true,
            show_framework: true,
            show_name_heuristics: false, // Low-weight fallback, hide by default
        }
    }
}

/// Evidence formatter for multi-signal classification
pub struct EvidenceFormatter {
    verbosity: VerbosityLevel,
    signal_filters: SignalFilters,
    show_all_signals: bool, // Override filters at -vvv
}

impl EvidenceFormatter {
    /// Create formatter from verbosity count
    /// - 0: Minimal (category + confidence only)
    /// - 1: Standard (signal summary)
    /// - 2: Verbose (detailed breakdown)
    /// - 3+: Very verbose (all signals including low-weight ones)
    pub fn new(verbose_count: u8) -> Self {
        let verbosity = VerbosityLevel::from_count(verbose_count);
        let show_all_signals = verbose_count >= 3;

        Self {
            verbosity,
            signal_filters: SignalFilters::default(),
            show_all_signals,
        }
    }

    /// Create formatter with custom signal filters
    pub fn with_filters(verbose_count: u8, filters: SignalFilters) -> Self {
        let verbosity = VerbosityLevel::from_count(verbose_count);
        let show_all_signals = verbose_count >= 3;

        Self {
            verbosity,
            signal_filters: filters,
            show_all_signals,
        }
    }

    /// Format evidence for display
    pub fn format_evidence(&self, evidence: &AggregatedClassification) -> String {
        match self.verbosity {
            VerbosityLevel::Minimal => self.format_minimal(evidence),
            VerbosityLevel::Standard => self.format_standard(evidence),
            VerbosityLevel::Verbose => self.format_verbose(evidence),
        }
    }

    /// Format minimal evidence (category + confidence only)
    fn format_minimal(&self, evidence: &AggregatedClassification) -> String {
        format!(
            "{} [{:.0}% confidence]",
            evidence.primary.as_str(),
            evidence.confidence * 100.0
        )
    }

    /// Format standard evidence (signal summary)
    fn format_standard(&self, evidence: &AggregatedClassification) -> String {
        let parts = [
            self.format_header(evidence),
            self.format_signals(&evidence.evidence),
            self.format_alternatives_if_ambiguous(evidence),
        ];

        parts
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Format verbose evidence (detailed breakdown)
    fn format_verbose(&self, evidence: &AggregatedClassification) -> String {
        let parts = [
            self.format_standard(evidence),
            self.format_detailed_breakdown(&evidence.evidence),
        ];

        parts.join("\n")
    }

    /// Format header with primary classification and confidence
    fn format_header(&self, evidence: &AggregatedClassification) -> String {
        let band = ConfidenceBand::from_score(evidence.confidence);
        format!(
            "    Primary: {} [Confidence: {:.2}, {}]",
            evidence.primary.as_str(),
            evidence.confidence,
            band.as_str()
        )
    }

    /// Format contributing signals
    fn format_signals(&self, signals: &[AggregatedSignalEvidence]) -> String {
        let formatted_signals: Vec<String> = signals
            .iter()
            .filter(|s| self.should_show_signal(s))
            .map(|signal| self.format_signal(signal))
            .collect();

        if formatted_signals.is_empty() {
            String::new()
        } else {
            format!(
                "\n    Contributing Signals:\n{}",
                formatted_signals.join("\n")
            )
        }
    }

    /// Check if signal should be displayed
    fn should_show_signal(&self, signal: &AggregatedSignalEvidence) -> bool {
        // -vvv shows all signals, overriding config
        if self.show_all_signals {
            return true;
        }

        // Otherwise respect signal filters
        match signal.signal_type {
            SignalType::IoDetection => self.signal_filters.show_io_detection,
            SignalType::CallGraph => self.signal_filters.show_call_graph,
            SignalType::TypeSignatures => self.signal_filters.show_type_signatures,
            SignalType::Purity => self.signal_filters.show_purity,
            SignalType::Framework => self.signal_filters.show_framework,
            SignalType::Name => self.signal_filters.show_name_heuristics,
        }
    }

    /// Format individual signal contribution (spec 148 - includes weight display)
    fn format_signal(&self, signal: &AggregatedSignalEvidence) -> String {
        let indicator = match signal.contribution {
            c if c > 0.15 => "✓",
            c if c > 0.05 => "•",
            _ => "-",
        };

        // Explicitly format weight information for transparency (spec 148)
        format!(
            "      {} {:?} (weight: {:.2})\n\
             {}Confidence: {:.0}%, Contribution: {:.3}\n\
             {}Evidence: {}",
            indicator,
            signal.signal_type,
            signal.weight,
            " ".repeat(9),
            signal.confidence * 100.0,
            signal.contribution,
            " ".repeat(9), // Indent for evidence
            signal.description
        )
    }

    /// Format alternative classifications if ambiguous
    fn format_alternatives_if_ambiguous(&self, evidence: &AggregatedClassification) -> String {
        if evidence.alternatives.is_empty() {
            return String::new();
        }

        // Show alternatives within 0.10 of primary
        let close_alternatives: Vec<_> = evidence
            .alternatives
            .iter()
            .filter(|(_, score)| evidence.confidence - score < 0.10)
            .collect();

        if close_alternatives.is_empty() {
            return String::new();
        }

        let formatted_alts: Vec<String> = close_alternatives
            .iter()
            .map(|(category, score)| {
                format!(
                    "      - {} ({:.2} confidence, Δ{:.2})",
                    category.as_str(),
                    score,
                    evidence.confidence - score
                )
            })
            .collect();

        format!(
            "\n    Alternative Classifications:\n{}\n      ⚠  Classification is ambiguous - consider manual review",
            formatted_alts.join("\n")
        )
    }

    /// Format detailed breakdown (verbose mode)
    fn format_detailed_breakdown(&self, signals: &[AggregatedSignalEvidence]) -> String {
        let details: Vec<String> = signals
            .iter()
            .filter(|s| self.should_show_signal(s))
            .map(|signal| {
                format!(
                    "\n    {:?} Signal:\n\
                       Category: {}\n\
                       Confidence: {:.4}\n\
                       Weight: {:.4}\n\
                       Contribution: {:.4}\n\
                       Evidence: {}",
                    signal.signal_type,
                    signal.category.as_str(),
                    signal.confidence,
                    signal.weight,
                    signal.contribution,
                    signal.description
                )
            })
            .collect();

        if details.is_empty() {
            String::new()
        } else {
            format!("\n    DETAILED BREAKDOWN:{}", details.join(""))
        }
    }

    /// Explain why classification is "Unknown" or low confidence
    pub fn explain_low_confidence(
        &self,
        evidence: &AggregatedClassification,
        category: ResponsibilityCategory,
    ) -> String {
        if evidence.confidence >= 0.60 {
            return String::new(); // Only explain low confidence
        }

        let mut reasons = Vec::new();

        // Check if all signals are weak
        if evidence
            .evidence
            .iter()
            .all(|s| s.confidence < 0.50 || s.contribution < 0.05)
        {
            reasons.push("No strong signals detected - needs manual review");
        }

        // Check for I/O pattern clarity
        if let Some(io_signal) = evidence
            .evidence
            .iter()
            .find(|s| matches!(s.signal_type, SignalType::IoDetection))
        {
            if io_signal.confidence < 0.60 {
                reasons.push("I/O pattern unclear - mixed operations");
            }
        }

        // Check for call graph clarity
        if let Some(cg_signal) = evidence
            .evidence
            .iter()
            .find(|s| matches!(s.signal_type, SignalType::CallGraph))
        {
            if cg_signal.confidence < 0.60 {
                reasons.push("No clear structural pattern - neither orchestrator nor leaf");
            }
        }

        // Check for ambiguity
        if evidence.alternatives.len() >= 2 {
            let first_alt_diff = evidence.confidence - evidence.alternatives[0].1;
            if first_alt_diff < 0.15 {
                reasons.push("Multiple responsibilities detected - may need further splitting");
            }
        }

        if reasons.is_empty() {
            return String::new();
        }

        format!(
            "\n    ⚠  Classified as '{}' because:\n{}",
            category.as_str(),
            reasons
                .iter()
                .map(|r| format!("      - {}", r))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbosity_level_from_count() {
        assert_eq!(VerbosityLevel::from_count(0), VerbosityLevel::Minimal);
        assert_eq!(VerbosityLevel::from_count(1), VerbosityLevel::Standard);
        assert_eq!(VerbosityLevel::from_count(2), VerbosityLevel::Verbose);
        assert_eq!(VerbosityLevel::from_count(3), VerbosityLevel::Verbose);
    }

    #[test]
    fn confidence_band_from_score() {
        assert_eq!(ConfidenceBand::from_score(0.90), ConfidenceBand::High);
        assert_eq!(ConfidenceBand::from_score(0.70), ConfidenceBand::Medium);
        assert_eq!(ConfidenceBand::from_score(0.50), ConfidenceBand::Low);
    }

    #[test]
    fn format_minimal() {
        let evidence = AggregatedClassification {
            primary: ResponsibilityCategory::Parsing,
            confidence: 0.85,
            evidence: vec![],
            alternatives: vec![],
        };

        let formatter = EvidenceFormatter::new(0);
        let output = formatter.format_evidence(&evidence);

        assert!(output.contains("Parsing"));
        assert!(output.contains("85% confidence"));
    }

    #[test]
    fn format_standard() {
        let evidence = AggregatedClassification {
            primary: ResponsibilityCategory::FileIO,
            confidence: 0.82,
            evidence: vec![AggregatedSignalEvidence {
                signal_type: SignalType::IoDetection,
                category: ResponsibilityCategory::FileIO,
                confidence: 0.90,
                weight: 0.35,
                contribution: 0.315,
                description: "3 file ops".to_string(),
            }],
            alternatives: vec![],
        };

        let formatter = EvidenceFormatter::new(1);
        let output = formatter.format_evidence(&evidence);

        assert!(output.contains("Primary: File I/O"));
        assert!(output.contains("0.82"));
        assert!(output.contains("IoDetection"));
        assert!(output.contains("3 file ops"));
    }

    #[test]
    fn show_alternatives_when_ambiguous() {
        let evidence = AggregatedClassification {
            primary: ResponsibilityCategory::Validation,
            confidence: 0.65,
            evidence: vec![],
            alternatives: vec![
                (ResponsibilityCategory::Transformation, 0.62),
                (ResponsibilityCategory::PureComputation, 0.58),
            ],
        };

        let formatter = EvidenceFormatter::new(1);
        let output = formatter.format_evidence(&evidence);

        assert!(output.contains("Alternative Classifications"));
        assert!(output.contains("Transformation"));
        assert!(output.contains("ambiguous"));
    }
}
