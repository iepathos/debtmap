use std::collections::HashMap;

fn build_map_imperative(items: Vec<(String, i32)>) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    for (k, v) in items {
        map.insert(k, v);
    }
    map
}
