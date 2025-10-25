fn merge_lists(a: Vec<i32>, b: Vec<i32>) -> Vec<i32> {
    a.into_iter()
        .chain(b.into_iter())
        .collect()
}
