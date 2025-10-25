use std::collections::HashMap;

fn process_map(map: HashMap<String, i32>) -> Vec<i32> {
    map.into_iter()
        .map(|(_, v)| v)
        .filter(|&v| v > 0)
        .collect()
}
