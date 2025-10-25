fn process_nested(items: Vec<Vec<i32>>) -> Vec<Vec<i32>> {
    items.into_iter()
        .map(|v| v.into_iter()
            .filter(|&x| x > 0)
            .collect())
        .collect()
}
