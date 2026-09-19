//! Translate propagation results and metric purity as one coherent fact.

use super::{PurityReason, PurityResult};
use crate::analysis::purity_analysis::PurityLevel;
use crate::core::{FunctionMetrics, PurityLevel as MetricPurityLevel};

pub(super) fn intrinsic_purity(metric: &FunctionMetrics) -> PurityResult {
    let Some(level) = intrinsic_level(metric) else {
        return PurityResult {
            level: PurityLevel::Impure,
            confidence: 0.3,
            reason: PurityReason::UnknownDeps { count: 0 },
        };
    };
    PurityResult {
        level,
        confidence: metric.purity_confidence.map(f64::from).unwrap_or(0.3),
        reason: PurityReason::Intrinsic,
    }
}

fn intrinsic_level(metric: &FunctionMetrics) -> Option<PurityLevel> {
    let level = metric.purity_level.map(analysis_level).or_else(|| {
        metric
            .is_pure
            .zip(metric.purity_confidence)
            .map(|(pure, _)| {
                if pure {
                    PurityLevel::StrictlyPure
                } else {
                    PurityLevel::Impure
                }
            })
    });
    // Explicitly contradictory legacy fields cannot establish strict purity.
    match (level, metric.is_pure) {
        (Some(PurityLevel::StrictlyPure), Some(false)) => Some(PurityLevel::Impure),
        (level, _) => level,
    }
}

impl PurityResult {
    pub(super) fn apply_to_metric(&self, metric: &FunctionMetrics) -> FunctionMetrics {
        FunctionMetrics {
            purity_level: Some(metric_level(&self.level)),
            is_pure: Some(self.level == PurityLevel::StrictlyPure),
            purity_confidence: Some(self.confidence as f32),
            purity_reason: Some(format!("{:?}", self.reason)),
            ..metric.clone()
        }
    }
}

fn analysis_level(level: MetricPurityLevel) -> PurityLevel {
    match level {
        MetricPurityLevel::StrictlyPure => PurityLevel::StrictlyPure,
        MetricPurityLevel::LocallyPure => PurityLevel::LocallyPure,
        MetricPurityLevel::ReadOnly => PurityLevel::ReadOnly,
        MetricPurityLevel::Impure => PurityLevel::Impure,
    }
}

fn metric_level(level: &PurityLevel) -> MetricPurityLevel {
    match level {
        PurityLevel::StrictlyPure => MetricPurityLevel::StrictlyPure,
        PurityLevel::LocallyPure => MetricPurityLevel::LocallyPure,
        PurityLevel::ReadOnly => MetricPurityLevel::ReadOnly,
        PurityLevel::Impure => MetricPurityLevel::Impure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metric() -> FunctionMetrics {
        FunctionMetrics::new("helper".into(), "helper.rs".into(), 7)
    }

    #[test]
    fn results_replace_all_purity_fields_together() {
        for (level, expected) in [
            (PurityLevel::StrictlyPure, MetricPurityLevel::StrictlyPure),
            (PurityLevel::LocallyPure, MetricPurityLevel::LocallyPure),
            (PurityLevel::ReadOnly, MetricPurityLevel::ReadOnly),
            (PurityLevel::Impure, MetricPurityLevel::Impure),
        ] {
            let mut original = metric();
            original.purity_level = Some(MetricPurityLevel::StrictlyPure);
            original.is_pure = Some(true);
            original.purity_confidence = Some(1.0);
            original.purity_reason = Some("stale".into());
            let result = PurityResult {
                level: level.clone(),
                confidence: 0.75,
                reason: PurityReason::Intrinsic,
            };
            let updated = result.apply_to_metric(&original);
            assert_eq!(updated.purity_level, Some(expected));
            assert_eq!(updated.is_pure, Some(level == PurityLevel::StrictlyPure));
            assert_eq!(updated.purity_confidence, Some(0.75));
            assert_eq!(updated.purity_reason.as_deref(), Some("Intrinsic"));
            assert_eq!(original.is_pure, Some(true));
            assert_eq!(intrinsic_purity(&updated).level, level);
        }
    }

    #[test]
    fn legacy_boolean_and_missing_evidence_keep_their_fallbacks() {
        for (pure, confidence, level) in [
            (Some(true), Some(0.9), PurityLevel::StrictlyPure),
            (Some(false), Some(0.9), PurityLevel::Impure),
            (Some(true), None, PurityLevel::Impure),
            (None, None, PurityLevel::Impure),
        ] {
            let mut metric = metric();
            metric.is_pure = pure;
            metric.purity_confidence = confidence;
            assert_eq!(intrinsic_purity(&metric).level, level);
        }
    }

    #[test]
    fn contradictory_strict_purity_is_conservative() {
        for (level, pure) in [
            (MetricPurityLevel::StrictlyPure, false),
            (MetricPurityLevel::Impure, true),
        ] {
            let mut metric = metric();
            metric.is_pure = Some(pure);
            metric.purity_level = Some(level);
            metric.purity_confidence = Some(0.95);
            assert_eq!(intrinsic_purity(&metric).level, PurityLevel::Impure);
        }
    }

    #[test]
    fn missing_evidence_cannot_gain_confidence_from_an_unmatched_field() {
        let mut metric = metric();
        metric.purity_confidence = Some(0.95);
        let result = intrinsic_purity(&metric);
        assert_eq!(result.level, PurityLevel::Impure);
        assert_eq!(result.confidence, 0.3);
        assert_eq!(result.reason, PurityReason::UnknownDeps { count: 0 });
    }
}
