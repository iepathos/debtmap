fn find_extremes(items: Vec<i32>) -> (Option<i32>, Option<i32>) {
    let max = items.iter().max().copied();
    let min = items.iter().min().copied();
    (max, min)
}
