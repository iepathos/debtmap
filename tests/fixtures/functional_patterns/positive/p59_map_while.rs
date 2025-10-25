fn parse_until_error(items: Vec<String>) -> Vec<i32> {
    items.into_iter()
        .map_while(|s| s.parse::<i32>().ok())
        .collect()
}
