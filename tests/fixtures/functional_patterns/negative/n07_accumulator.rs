fn accumulate_values(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    let mut acc = 0;
    for item in items {
        acc += item;
        result.push(acc);
    }
    result
}
