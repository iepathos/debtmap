fn mixed_style(items: Vec<i32>) -> Vec<i32> {
    let mut result: Vec<i32> = items.into_iter()
        .filter(|&x| x > 0)
        .collect();
    result.sort();
    result
}
