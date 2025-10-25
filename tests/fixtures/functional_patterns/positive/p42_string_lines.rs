fn non_empty_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect()
}
