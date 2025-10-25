fn single_map(x: i32) -> Vec<i32> {
    vec![x].into_iter()
        .map(|n| n * 2)
        .collect()
}
