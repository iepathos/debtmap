fn process_chars(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}
