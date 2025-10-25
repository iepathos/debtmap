fn build_string_imperative(items: Vec<&str>) -> String {
    let mut result = String::new();
    for item in items {
        result.push_str(item);
        result.push(',');
    }
    result
}
