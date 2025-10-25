fn count_positives(items: Vec<i32>) -> usize {
    let mut count = 0;
    for item in items {
        if item > 0 {
            count += 1;
        }
    }
    count
}
