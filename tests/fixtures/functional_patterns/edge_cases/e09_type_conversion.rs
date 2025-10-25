fn convert_types(items: Vec<i32>) -> Vec<String> {
    items.into_iter()
        .map(|x| x.to_string())
        .collect()
}
