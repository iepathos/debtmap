use std::collections::BTreeMap;

fn sorted_values(map: BTreeMap<String, i32>) -> Vec<i32> {
    map.into_values()
        .filter(|&v| v > 0)
        .collect()
}
