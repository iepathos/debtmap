use super::cases::{self, Case, Cells, check, finish, target, tied, unrelated};
use debtmap::data_flow::{DataFlowGraph, DataTransformation};
use debtmap::priority::call_graph::{CallGraph, FunctionId};

#[path = "serialized_pairs.rs"]
mod serialized_pairs;

fn right(mut id: FunctionId) -> FunctionId {
    id.file = std::path::PathBuf::from("right").join(id.file);
    id.name = format!("right::{}", id.name);
    id.line += 100;
    id
}

fn graph(left: &Case, second: &Case) -> DataFlowGraph {
    let mut calls = CallGraph::new();
    for id in [target(), right(target()), unrelated()]
        .into_iter()
        .chain(left.tied.then(tied))
        .chain(second.tied.then(|| right(tied())))
    {
        calls.add_function(id, false, false, 1, 1);
    }
    DataFlowGraph::from_call_graph(calls)
}

fn write(flow: &mut DataFlowGraph, from: FunctionId, to: FunctionId, marker: &str) {
    flow.add_data_transformation(
        from,
        to,
        DataTransformation {
            input_vars: vec![],
            output_vars: vec![],
            transformation_type: marker.into(),
        },
    );
}

fn read(flow: &DataFlowGraph, from: &FunctionId, to: &FunctionId) -> Option<String> {
    flow.get_data_transformation(from, to)
        .map(|value| value.transformation_type.clone())
}

fn sentinels(flow: &mut DataFlowGraph) {
    write(flow, unrelated(), unrelated(), "unrelated");
    write(flow, right(target()), target(), "reverse");
}

fn verify(
    flow: &DataFlowGraph,
    cases: (&Case, &Case),
    expected: Option<&str>,
    failures: &mut Vec<String>,
    context: &str,
) {
    let (left, second) = cases;
    let label = format!("{context} / from={} to={}", left.name, second.name);
    for legacy_from in [false, true] {
        for legacy_to in [false, true] {
            let from = target().with_column((!legacy_from).then_some(7));
            let to = right(target()).with_column((!legacy_to).then_some(7));
            let value = if (left.tied && legacy_from) || (second.tied && legacy_to) {
                None
            } else {
                expected.map(str::to_owned)
            };
            check(
                failures,
                &format!("{label} query {from:?} -> {to:?}"),
                read(flow, &from, &to),
                value,
            );
        }
    }
    check(
        failures,
        &format!("{label} unrelated {:?}", unrelated()),
        read(flow, &unrelated(), &unrelated()),
        Some("unrelated".into()),
    );
    check(
        failures,
        &format!("{label} reverse {:?} -> {:?}", right(target()), target()),
        read(flow, &right(target()), &target()),
        Some("reverse".into()),
    );
}

#[test]
fn transformation_endpoint_product_preserves_unrelated_pairs() {
    let cases = cases::cases();
    let mut failures = vec![];
    let mut cells = Cells::default();
    for left in &cases {
        for second in &cases {
            for roundtrip in [false, true] {
                let mut flow = graph(left, second);
                sentinels(&mut flow);
                write(&mut flow, target(), right(target()), "target");
                let matches = left.matches && second.matches;
                if !matches {
                    check(
                        &mut failures,
                        &format!(
                            "roundtrip={roundtrip} / from={} to={} pre-write {:?} -> {:?}",
                            left.name,
                            second.name,
                            left.incoming,
                            right(second.incoming.clone())
                        ),
                        read(&flow, &left.incoming, &right(second.incoming.clone())),
                        None,
                    );
                }
                write(
                    &mut flow,
                    left.incoming.clone(),
                    right(second.incoming.clone()),
                    "incoming",
                );
                if roundtrip {
                    flow = serde_json::from_value(serde_json::to_value(flow).unwrap()).unwrap();
                }
                verify(
                    &flow,
                    (left, second),
                    Some(if matches { "incoming" } else { "target" }),
                    &mut failures,
                    &format!("roundtrip={roundtrip}"),
                );
                cells.record(
                    format!(
                        "from={} to={} roundtrip={roundtrip}",
                        left.name, second.name
                    ),
                    &failures,
                );
            }
        }
    }
    assert_eq!(cells.total, 162);
    finish(
        "transformation writes and current roundtrips",
        cells,
        failures,
    );
}

fn key(id: &FunctionId) -> String {
    format!("{}:{}:{}", id.file.display(), id.name, id.line)
}

#[test]
fn legacy_transformation_endpoint_product_requires_two_exact_matches() {
    let cases: Vec<_> = cases::cases()
        .into_iter()
        .filter(|case| case.incoming.column.is_none())
        .collect();
    let mut failures = vec![];
    let mut cells = Cells::default();
    for left in &cases {
        for second in &cases {
            let mut flow = graph(left, second);
            sentinels(&mut flow);
            let mut json = serde_json::to_value(flow).unwrap();
            let pair = format!(
                "{}|{}",
                key(&left.incoming),
                key(&right(second.incoming.clone()))
            );
            json["data_transformations"].as_object_mut().unwrap().insert(pair, serde_json::json!({"input_vars": [], "output_vars": [], "transformation_type": "incoming"}));
            let flow: DataFlowGraph = serde_json::from_value(json).unwrap();
            verify(
                &flow,
                (left, second),
                (left.matches && second.matches).then_some("incoming"),
                &mut failures,
                "legacy JSON",
            );
            let restored: DataFlowGraph =
                serde_json::from_value(serde_json::to_value(flow).unwrap()).unwrap();
            verify(
                &restored,
                (left, second),
                (left.matches && second.matches).then_some("incoming"),
                &mut failures,
                "legacy then current JSON",
            );
            cells.record(format!("from={} to={}", left.name, second.name), &failures);
        }
    }
    assert_eq!(cells.total, 49);
    finish("legacy transformation JSON", cells, failures);
}
