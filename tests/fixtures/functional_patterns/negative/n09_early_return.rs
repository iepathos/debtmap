fn find_first_imperative(items: Vec<i32>) -> Option<i32> {
    for item in items {
        if item > 10 {
            return Some(item);
        }
    }
    None
}
