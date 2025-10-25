use std::collections::HashSet;

fn deduplicate_imperative(items: Vec<i32>) -> Vec<i32> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for item in items {
        if !seen.contains(&item) {
            seen.insert(item);
            result.push(item);
        }
    }
    result
}
