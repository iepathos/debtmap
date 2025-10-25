fn parse_all_imperative(items: Vec<String>) -> Vec<i32> {
    let mut result = Vec::new();
    for s in items {
        if let Ok(val) = s.parse::<i32>() {
            result.push(val);
        }
    }
    result
}
