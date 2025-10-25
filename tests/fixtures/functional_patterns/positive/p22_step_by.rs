fn every_nth(items: Vec<i32>, n: usize) -> Vec<i32> {
    items.into_iter()
        .step_by(n)
        .collect()
}
