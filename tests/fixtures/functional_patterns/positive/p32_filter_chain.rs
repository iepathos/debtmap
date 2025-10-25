fn complex_filter(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .filter(|&x| x > 0)
        .filter(|&x| x < 100)
        .filter(|&x| x % 2 == 0)
        .collect()
}
