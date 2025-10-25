fn parse_numbers(items: Vec<String>) -> Vec<i32> {
    items.iter()
        .filter_map(|s| s.parse::<i32>().ok())
        .collect()
}
