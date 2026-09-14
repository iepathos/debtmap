//! Assert complete location-based outcomes, including absence of extra call sites.
use super::{
    Expectation, Outcome,
    oracle::{Location, Oracle},
};
use debtmap::priority::call_graph::{CallGraph, CallSite, FunctionId};
use std::collections::BTreeSet;

fn definition(id: &FunctionId) -> Option<Location> {
    Some(Location {
        file: id.file.clone(),
        line: id.line,
        column: id.column?,
    })
}
fn site(site: &CallSite) -> Option<Location> {
    Some(Location {
        file: site.file.clone(),
        line: site.line,
        column: site.column?,
    })
}

pub fn check(graph: &CallGraph, oracle: &Oracle, expected: &[Expectation]) -> Vec<String> {
    let mut failures = Vec::new();
    check_node_membership(graph, &mut failures);
    let actual: BTreeSet<_> = graph.get_all_functions().filter_map(definition).collect();
    let declared: BTreeSet<_> = oracle.definitions.values().cloned().collect();
    if graph.node_count() != declared.len() || actual != declared {
        failures.push(format!(
            "node identities: expected {declared:?}, actual {actual:?}, count {}",
            graph.node_count()
        ));
    }
    for (expectation, position) in expected.iter().zip(&oracle.sites) {
        check_site(graph, oracle, expectation, position, &mut failures);
    }
    let callers: BTreeSet<_> = expected
        .iter()
        .map(|e| &oracle.definitions[e.caller])
        .collect();
    for evidence in graph.edge_evidence().filter(|e| {
        definition(&e.call.caller)
            .as_ref()
            .is_some_and(|id| callers.contains(id))
    }) {
        if evidence
            .call_site
            .as_ref()
            .and_then(site)
            .is_none_or(|site| !oracle.sites.contains(&site))
        {
            failures.push(format!("unexpected resolved site: {evidence:?}"));
        }
    }
    for call in graph.uncertain_calls().filter(|call| {
        definition(&call.caller)
            .as_ref()
            .is_some_and(|id| callers.contains(id))
    }) {
        if site(&call.call_site).is_none_or(|site| !oracle.sites.contains(&site)) {
            failures.push(format!("unexpected uncertain site: {call:?}"));
        }
    }
    failures
}

fn check_node_membership(graph: &CallGraph, failures: &mut Vec<String>) {
    let nodes: BTreeSet<_> = graph.get_all_functions().cloned().collect();
    for call in graph.get_all_calls() {
        require_node(&call.caller, &nodes, "resolved caller", failures);
        require_node(&call.callee, &nodes, "resolved target", failures);
    }
    for call in graph.uncertain_calls() {
        require_node(&call.caller, &nodes, "uncertain caller", failures);
        for target in &call.candidates {
            require_node(target, &nodes, "possible target", failures);
        }
    }
}

fn require_node(
    id: &FunctionId,
    nodes: &BTreeSet<FunctionId>,
    role: &str,
    failures: &mut Vec<String>,
) {
    if !nodes.contains(id) {
        failures.push(format!("{role} is not a graph node: {id:?}"));
    }
}

fn check_site(
    graph: &CallGraph,
    oracle: &Oracle,
    expected: &Expectation,
    position: &Location,
    failures: &mut Vec<String>,
) {
    let caller = &oracle.definitions[expected.caller];
    let resolved: Vec<_> = graph
        .edge_evidence()
        .filter(|e| e.call_site.as_ref().and_then(site).as_ref() == Some(position))
        .collect();
    let uncertain: Vec<_> = graph
        .uncertain_calls()
        .filter(|call| site(&call.call_site).as_ref() == Some(position))
        .collect();
    let targets: BTreeSet<_> = expected
        .targets
        .iter()
        .map(|label| oracle.definitions[*label].clone())
        .collect();
    let right_callers = resolved
        .iter()
        .all(|e| definition(&e.call.caller).as_ref() == Some(caller))
        && uncertain
            .iter()
            .all(|call| definition(&call.caller).as_ref() == Some(caller));
    let valid = match expected.outcome {
        Outcome::Resolved => {
            uncertain.is_empty()
                && resolved.len() == targets.len()
                && resolved
                    .iter()
                    .filter_map(|e| definition(&e.call.callee))
                    .collect::<BTreeSet<_>>()
                    == targets
        }
        Outcome::Uncertain => {
            resolved.is_empty()
                && uncertain.len() == 1
                && uncertain[0].candidates.len() == targets.len()
                && uncertain[0]
                    .candidates
                    .iter()
                    .filter_map(definition)
                    .collect::<BTreeSet<_>>()
                    == targets
                && expected
                    .reason
                    .as_ref()
                    .is_none_or(|reason| &uncertain[0].reason == reason)
        }
        Outcome::Absent => resolved.is_empty() && uncertain.is_empty(),
    };
    if !valid || !right_callers {
        failures.push(format!("site {} expected {:?} targets {targets:?}; resolved={resolved:?}; uncertain={uncertain:?}", expected.site, expected.outcome));
    }
}
