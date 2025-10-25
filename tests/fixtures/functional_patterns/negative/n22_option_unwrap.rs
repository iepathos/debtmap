fn extract_all_imperative(items: Vec<Option<i32>>) -> Vec<i32> {
    let mut result = Vec::new();
    for opt in items {
        if let Some(val) = opt {
            result.push(val);
        }
    }
    result
}
