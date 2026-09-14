//! Current map keys can contain a mixture of exact and legacy endpoint IDs.
use super::*;

#[test]
fn raw_current_pairs_preserve_mixed_columns_and_reject_ambiguous_endpoints() {
    let cases: Vec<_> = cases::cases()
        .into_iter()
        .filter(|case| matches!(case.name, "exact" | "unique_legacy" | "same_line_tie"))
        .collect();
    let mut failures = vec![];
    let mut cells = Cells::default();
    for left in &cases {
        for second in &cases {
            let mut flow = graph(left, second);
            sentinels(&mut flow);
            let mut json = serde_json::to_value(flow).unwrap();
            let pair = (&left.incoming, right(second.incoming.clone()));
            let key = format!("v2:{}", serde_json::to_string(&pair).unwrap());
            json["data_transformations"].as_object_mut().unwrap().insert(key, serde_json::json!({"input_vars": [], "output_vars": [], "transformation_type": "incoming"}));
            let flow: DataFlowGraph = serde_json::from_value(json).unwrap();
            let expected = (left.matches && second.matches).then_some("incoming");
            verify(
                &flow,
                (left, second),
                expected,
                &mut failures,
                "raw current JSON",
            );
            let restored: DataFlowGraph =
                serde_json::from_value(serde_json::to_value(flow).unwrap()).unwrap();
            verify(
                &restored,
                (left, second),
                expected,
                &mut failures,
                "raw current JSON roundtrip",
            );
            cells.record(format!("from={} to={}", left.name, second.name), &failures);
        }
    }
    assert_eq!(cells.total, 9);
    finish("raw current mixed transformation JSON", cells, failures);
}
