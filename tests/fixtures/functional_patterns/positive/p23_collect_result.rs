fn parse_all(items: Vec<String>) -> Result<Vec<i32>, std::num::ParseIntError> {
    items.iter()
        .map(|s| s.parse::<i32>())
        .collect()
}
