fn flatten_imperative(nested: Vec<Vec<i32>>) -> Vec<i32> {
    let mut result = Vec::new();
    for vec in nested {
        for item in vec {
            result.push(item);
        }
    }
    result
}
