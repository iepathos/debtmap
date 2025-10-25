fn combine_lists(a: Vec<i32>, b: Vec<i32>) -> Vec<(i32, i32)> {
    a.into_iter()
        .zip(b.into_iter())
        .collect()
}
