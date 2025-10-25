fn safe_iteration(items: Vec<Option<i32>>) -> Vec<i32> {
    items.into_iter()
        .fuse()
        .filter_map(|x| x)
        .collect()
}
