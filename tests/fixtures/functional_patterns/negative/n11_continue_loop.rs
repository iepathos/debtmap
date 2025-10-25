fn skip_zeros(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item == 0 {
            continue;
        }
        result.push(item);
    }
    result
}
