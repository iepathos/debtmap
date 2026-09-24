//! Owned effect assessments kept separate from reachability edges.

use super::{CallGraph, FunctionId};
use crate::analysis::effect_evidence::EffectAssessment;

impl CallGraph {
    /// Union source-backed evidence for one exact definition identity.
    pub fn record_effect_assessment(&mut self, id: FunctionId, assessment: EffectAssessment) {
        self.effect_assessments_propagated = false;
        self.effect_assessments
            .entry(id)
            .and_modify(|current| *current = current.join(&assessment))
            .or_insert(assessment);
    }

    /// Return evidence only for the exact definition; fuzzy identity is unsafe here.
    pub fn effect_assessment(&self, id: &FunctionId) -> Option<&EffectAssessment> {
        self.effect_assessments.get(id)
    }

    /// Iterate assessments in stable definition order.
    pub fn effect_assessments(&self) -> impl Iterator<Item = (&FunctionId, &EffectAssessment)> {
        self.effect_assessments.iter()
    }

    pub(crate) fn replace_with_propagated_effect_assessments(
        &mut self,
        assessments: impl IntoIterator<Item = (FunctionId, EffectAssessment)>,
    ) {
        self.effect_assessments = assessments.into_iter().collect();
        self.effect_assessments_propagated = true;
    }

    pub(crate) fn effect_assessments_are_propagated(&self) -> bool {
        self.effect_assessments_propagated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::effect_evidence::{
        EffectClassification, EffectDependency, EffectProvenance, ObservedEffect,
        ObservedEffectKind, UnresolvedBehavior, UnresolvedReason,
    };

    fn id() -> FunctionId {
        FunctionId::new("src/lib.rs".into(), "caller".into(), 3).with_column(Some(4))
    }

    #[test]
    fn repeated_and_reordered_merges_are_normalized() {
        let id = id();
        let uncertain = EffectAssessment::complete().with_unresolved(UnresolvedBehavior {
            reason: UnresolvedReason::UnresolvedCall,
            detail: "missing".into(),
            provenance: EffectProvenance::source(id.clone(), 4, Some(8)),
        });
        let mut left = CallGraph::new();
        left.record_effect_assessment(id.clone(), EffectAssessment::complete());
        left.record_effect_assessment(id.clone(), uncertain.clone());
        left.record_effect_assessment(id.clone(), uncertain.clone());
        let mut right = CallGraph::new();
        right.record_effect_assessment(id.clone(), uncertain);
        right.record_effect_assessment(id.clone(), EffectAssessment::complete());
        let mut merged_left = CallGraph::new();
        merged_left.merge(left.clone());
        merged_left.merge(right.clone());
        merged_left.merge(left);
        let mut merged_right = CallGraph::new();
        merged_right.merge(right);
        assert_eq!(
            merged_left.effect_assessment(&id),
            merged_right.effect_assessment(&id)
        );
        assert_eq!(
            merged_left
                .effect_assessment(&id)
                .map(EffectAssessment::classification),
            Some(EffectClassification::Unknown)
        );
        assert_eq!(
            merged_left
                .effect_assessment(&id)
                .unwrap()
                .unresolved()
                .count(),
            1
        );
    }

    #[test]
    fn legacy_json_without_assessments_loads_as_absent_evidence() {
        let graph = CallGraph::new();
        let mut value = serde_json::to_value(graph).unwrap();
        value.as_object_mut().unwrap().remove("effect_assessments");
        let restored: CallGraph = serde_json::from_value(value).unwrap();
        assert!(restored.effect_assessments().next().is_none());
    }

    #[test]
    fn json_round_trip_preserves_structured_assessment_identity() {
        let id = id();
        let target = FunctionId::new("src/lib.rs".into(), "callee".into(), 8);
        let mut graph = CallGraph::new();
        graph.add_function(id.clone(), true, false, 1, 3);
        graph.add_function(target.clone(), false, false, 1, 3);
        graph.add_call_parts(id.clone(), target.clone(), super::super::CallType::Direct);
        let evidence = EffectAssessment::complete()
            .with_effect(ObservedEffect {
                kind: ObservedEffectKind::Io,
                detail: "standard output".into(),
                provenance: EffectProvenance::source(id.clone(), 4, Some(8)),
            })
            .with_unresolved(UnresolvedBehavior {
                reason: UnresolvedReason::UnsupportedDispatch,
                detail: "formatting implementation".into(),
                provenance: EffectProvenance::source(id.clone(), 4, Some(10)),
            })
            .with_dependency(EffectDependency {
                target: target.clone(),
                provenance: EffectProvenance::source(id.clone(), 5, Some(8)),
            });
        graph.record_effect_assessment(id.clone(), evidence.clone());

        let encoded = serde_json::to_string(&graph).unwrap();
        let restored: CallGraph = serde_json::from_str(&encoded).unwrap();

        assert_eq!(restored.effect_assessment(&id), Some(&evidence));

        let encoded = postcard::to_allocvec(&graph).unwrap();
        let (restored, remaining): (CallGraph, _) = postcard::take_from_bytes(&encoded).unwrap();
        assert!(
            remaining.is_empty(),
            "all serialized indexes must be consumed"
        );
        assert_eq!(restored.effect_assessment(&id), Some(&evidence));
        assert_eq!(restored.get_callees(&id), vec![target.clone()]);
        assert_eq!(restored.get_callers(&target), vec![id]);
        assert_eq!(restored.nodes.len(), graph.nodes.len());
    }
}
