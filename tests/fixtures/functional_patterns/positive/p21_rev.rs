fn reverse_and_filter(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .rev()
        .filter(|&x| x > 0)
        .collect()
}
