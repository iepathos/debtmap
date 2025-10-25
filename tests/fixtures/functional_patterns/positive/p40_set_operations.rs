use std::collections::HashSet;

fn unique_positive(items: Vec<i32>) -> HashSet<i32> {
    items.into_iter()
        .filter(|&x| x > 0)
        .collect()
}
