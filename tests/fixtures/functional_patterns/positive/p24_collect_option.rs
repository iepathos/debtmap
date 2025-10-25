fn try_parse_all(items: Vec<String>) -> Option<Vec<i32>> {
    items.iter()
        .map(|s| s.parse::<i32>().ok())
        .collect()
}
