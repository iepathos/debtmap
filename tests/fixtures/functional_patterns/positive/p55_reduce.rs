fn find_max(items: Vec<i32>) -> Option<i32> {
    items.into_iter()
        .reduce(|a, b| if a > b { a } else { b })
}
