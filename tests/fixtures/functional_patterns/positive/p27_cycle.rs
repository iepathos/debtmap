fn repeat_pattern(items: Vec<i32>, count: usize) -> Vec<i32> {
    items.iter()
        .cycle()
        .take(count)
        .copied()
        .collect()
}
