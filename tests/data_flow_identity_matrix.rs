//! Desired exact migration contract; failures are active repair specifications.
//! Out-of-graph legacy reads are deliberately unspecified. They may not alias a
//! different file/name/line. `module_path` metadata policy is also left open;
//! module qualification here means the qualified source definition name.
#[path = "data_flow_identity_matrix/cases.rs"]
mod cases;
#[path = "data_flow_identity_matrix/facts.rs"]
mod facts;
#[path = "data_flow_identity_matrix/transformations.rs"]
mod transformations;

use cases::{Cells, check, finish, graph, target, tied, unrelated};
use debtmap::data_flow::DataFlowGraph;
use facts::{Fact, marker};

fn write_cell(case: &cases::Case, fact: Fact, roundtrip: bool, failures: &mut Vec<String>) {
    let label = format!("{} / {fact:?} / roundtrip={roundtrip}", case.name);
    let mut flow = graph(case);
    fact.write(&mut flow, target(), "target");
    fact.write(&mut flow, unrelated(), "unrelated");
    if case.tied {
        fact.write(&mut flow, tied(), "tie");
    }
    if !case.matches {
        check(
            failures,
            &format!("{label} pre-write incoming lookup {:?}", case.incoming),
            fact.read(&flow, &case.incoming),
            None,
        );
    }
    fact.write(&mut flow, case.incoming.clone(), "incoming");
    if roundtrip {
        flow = serde_json::from_value(serde_json::to_value(flow).unwrap()).unwrap();
    }
    let expected = if case.matches {
        fact.after_matching_write()
    } else {
        marker("target")
    };
    check(
        failures,
        &format!("{label} exact target {:?}", target()),
        fact.read(&flow, &target()),
        expected.clone(),
    );
    check(
        failures,
        &format!("{label} unrelated {:?}", unrelated()),
        fact.read(&flow, &unrelated()),
        marker("unrelated"),
    );
    let legacy = target().with_column(None);
    check(
        failures,
        &format!("{label} legacy target {legacy:?}"),
        fact.read(&flow, &legacy),
        if case.tied { None } else { expected },
    );
    if case.tied {
        check(
            failures,
            &format!("{label} other same-line definition {:?}", tied()),
            fact.read(&flow, &tied()),
            marker("tie"),
        );
    }
}

#[test]
fn writes_and_current_roundtrips_preserve_every_definition_identity() {
    let mut failures = vec![];
    let mut cells = Cells::default();
    for case in cases::cases() {
        for fact in facts::ALL {
            write_cell(&case, fact, false, &mut failures);
            cells.record(format!("{} / {fact:?} / direct", case.name), &failures);
        }
        for fact in facts::SERIALIZED {
            write_cell(&case, fact, true, &mut failures);
            cells.record(
                format!("{} / {fact:?} / current roundtrip", case.name),
                &failures,
            );
        }
    }
    assert_eq!(cells.total, 90);
    finish("fact writes and current roundtrips", cells, failures);
}

fn import_legacy(case: &cases::Case, fact: Fact) -> DataFlowGraph {
    let mut flow = graph(case);
    fact.write(&mut flow, unrelated(), "unrelated");
    let mut json = serde_json::to_value(flow).unwrap();
    let mut incoming = DataFlowGraph::new();
    fact.write(&mut incoming, case.incoming.clone(), "incoming");
    let payload = serde_json::to_value(incoming).unwrap()[fact.field()]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .clone();
    let id = &case.incoming;
    let key = format!("{}:{}:{}", id.file.display(), id.name, id.line);
    json[fact.field()]
        .as_object_mut()
        .unwrap()
        .insert(key, payload);
    serde_json::from_value(json).unwrap()
}

#[test]
fn legacy_json_facts_require_a_matching_unique_declaration() {
    let mut failures = vec![];
    let mut cells = Cells::default();
    for case in cases::cases()
        .into_iter()
        .filter(|case| case.incoming.column.is_none())
    {
        for fact in facts::SERIALIZED {
            let flow = import_legacy(&case, fact);
            let label = format!("{} / {fact:?} / legacy JSON", case.name);
            check(
                &mut failures,
                &format!("{label} target {:?}", target()),
                fact.read(&flow, &target()),
                if case.matches {
                    marker("incoming")
                } else {
                    None
                },
            );
            check(
                &mut failures,
                &format!("{label} unrelated {:?}", unrelated()),
                fact.read(&flow, &unrelated()),
                marker("unrelated"),
            );
            if case.matches || case.tied {
                check(
                    &mut failures,
                    &format!("{label} legacy query {:?}", case.incoming),
                    fact.read(&flow, &case.incoming),
                    if case.matches {
                        marker("incoming")
                    } else {
                        None
                    },
                );
            }
            let mut restored: DataFlowGraph =
                serde_json::from_value(serde_json::to_value(flow).unwrap()).unwrap();
            check(
                &mut failures,
                &format!("{label} current roundtrip"),
                fact.read(&restored, &target()),
                if case.matches {
                    marker("incoming")
                } else {
                    None
                },
            );
            if matches!(fact, Fact::Io) {
                fact.write(&mut restored, target(), "appended");
                let expected = if case.matches {
                    Some(vec!["incoming".into(), "appended".into()])
                } else {
                    marker("appended")
                };
                check(
                    &mut failures,
                    &format!("{label} append after import"),
                    fact.read(&restored, &target()),
                    expected,
                );
                check(
                    &mut failures,
                    &format!("{label} unrelated after append {:?}", unrelated()),
                    fact.read(&restored, &unrelated()),
                    marker("unrelated"),
                );
            }
            cells.record(label, &failures);
        }
    }
    assert_eq!(cells.total, 28);
    finish("legacy fact JSON", cells, failures);
}
