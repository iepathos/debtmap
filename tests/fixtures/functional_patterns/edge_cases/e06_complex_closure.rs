fn complex_filter(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .filter(|&x| {
            let doubled = x * 2;
            doubled > 0 && doubled < 100
        })
        .collect()
}
