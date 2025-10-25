fn with_indices(items: Vec<String>) -> Vec<(usize, String)> {
    items.into_iter()
        .enumerate()
        .collect()
}
