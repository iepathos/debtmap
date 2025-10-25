use std::collections::HashMap;

fn build_map(items: Vec<(String, i32)>) -> HashMap<String, i32> {
    items.into_iter()
        .filter(|(_, v)| *v > 0)
        .collect()
}
