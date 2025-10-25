fn first_positive(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .filter(|&x| x > 0)
        .take(10)
        .collect()
}
