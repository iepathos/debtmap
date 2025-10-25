fn extract_values(items: Vec<Option<Vec<i32>>>) -> Vec<i32> {
    items.into_iter()
        .flat_map(|opt| opt.into_iter().flatten())
        .collect()
}
