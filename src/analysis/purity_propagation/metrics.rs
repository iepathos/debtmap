//! Translate propagation results and metric purity as one coherent fact.

use super::PurityResult;
use crate::analysis::effect_evidence::{
    EffectAssessment, EffectClassification, EffectProvenance, ObservedEffect, ObservedEffectKind,
    UnresolvedReason,
};
use crate::analysis::purity_analysis::PurityLevel;
use crate::core::{FunctionMetrics, Language, PurityLevel as MetricPurityLevel};
use crate::priority::call_graph::FunctionId;

pub(super) fn intrinsic_purity(metric: &FunctionMetrics) -> PurityResult {
    let owner = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
        .with_column(metric.column);
    let confidence = metric.purity_confidence.map(f64::from).unwrap_or(0.3);
    let assessment = if Language::from_path(&metric.file) == Language::Rust {
        EffectAssessment::unknown_for(
            owner,
            UnresolvedReason::LegacyEvidence,
            "legacy Rust purity fields have no semantic effect evidence",
        )
    } else {
        intrinsic_level(metric)
            .map(|level| assessment_from_level(level, owner.clone()))
            .unwrap_or_else(|| {
                EffectAssessment::unknown_for(
                    owner,
                    UnresolvedReason::LegacyEvidence,
                    "purity evidence is absent",
                )
            })
    };
    super::result_from_assessment(assessment, confidence)
}

fn assessment_from_level(level: PurityLevel, owner: FunctionId) -> EffectAssessment {
    let kind = match level {
        PurityLevel::StrictlyPure => return EffectAssessment::complete(),
        PurityLevel::LocallyPure => ObservedEffectKind::LocalMutation,
        PurityLevel::ReadOnly => ObservedEffectKind::ExternalRead,
        PurityLevel::Impure => ObservedEffectKind::ExternalWrite,
    };
    EffectAssessment::complete().with_effect(ObservedEffect {
        kind,
        detail: format!("legacy non-Rust {level:?} classification"),
        provenance: EffectProvenance::source(owner.clone(), owner.line, owner.column),
    })
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
        let classification = self.assessment.classification();
        FunctionMetrics {
            purity_level: metric_level(classification),
            is_pure: match classification {
                EffectClassification::StrictlyPure => Some(true),
                EffectClassification::LocallyPure
                | EffectClassification::ReadOnly
                | EffectClassification::Impure => Some(false),
                EffectClassification::Unknown => None,
            },
            purity_confidence: Some(self.confidence as f32),
            purity_reason: Some(effect_reason(&self.assessment)),
            ..metric.clone()
        }
    }
}

fn effect_reason(assessment: &EffectAssessment) -> String {
    let observed = assessment
        .observed()
        .map(|effect| effect.detail.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let unresolved = assessment
        .unresolved()
        .map(|behavior| behavior.detail.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!("EffectEvidence: observed=[{observed}]; unresolved=[{unresolved}]")
}

fn analysis_level(level: MetricPurityLevel) -> PurityLevel {
    match level {
        MetricPurityLevel::StrictlyPure => PurityLevel::StrictlyPure,
        MetricPurityLevel::LocallyPure => PurityLevel::LocallyPure,
        MetricPurityLevel::ReadOnly => PurityLevel::ReadOnly,
        MetricPurityLevel::Impure => PurityLevel::Impure,
    }
}

fn metric_level(level: EffectClassification) -> Option<MetricPurityLevel> {
    match level {
        EffectClassification::StrictlyPure => Some(MetricPurityLevel::StrictlyPure),
        EffectClassification::LocallyPure => Some(MetricPurityLevel::LocallyPure),
        EffectClassification::ReadOnly => Some(MetricPurityLevel::ReadOnly),
        EffectClassification::Impure => Some(MetricPurityLevel::Impure),
        EffectClassification::Unknown => None,
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
                assessment: assessment_from_level(
                    level.clone(),
                    FunctionId::new(original.file.clone(), original.name.clone(), original.line),
                ),
                level: level.clone(),
                confidence: 0.75,
                reason: super::super::PurityReason::Intrinsic,
            };
            let updated = result.apply_to_metric(&original);
            assert_eq!(updated.purity_level, Some(expected));
            assert_eq!(updated.is_pure, Some(level == PurityLevel::StrictlyPure));
            assert_eq!(updated.purity_confidence, Some(0.75));
            assert!(
                updated
                    .purity_reason
                    .as_deref()
                    .unwrap()
                    .starts_with("EffectEvidence:")
            );
            assert_eq!(original.is_pure, Some(true));
        }
    }

    #[test]
    fn rust_legacy_fields_remain_unknown_even_with_high_confidence() {
        let mut metric = metric();
        metric.is_pure = Some(true);
        metric.purity_level = Some(MetricPurityLevel::StrictlyPure);
        metric.purity_confidence = Some(0.95);
        let result = intrinsic_purity(&metric);
        assert_eq!(result.level, PurityLevel::Impure);
        assert!((result.confidence - 0.95).abs() < 1e-6);
        assert_eq!(
            result.assessment.classification(),
            EffectClassification::Unknown
        );
        let updated = result.apply_to_metric(&metric);
        assert_eq!(updated.purity_level, None);
        assert_eq!(updated.is_pure, None);
    }
}
