fn cross_product(a: Vec<i32>, b: Vec<i32>) -> Vec<(i32, i32)> {
    a.iter()
        .flat_map(|&x| b.iter().map(move |&y| (x, y)))
        .collect()
}
