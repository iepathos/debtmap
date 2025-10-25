fn safe_sum(items: Vec<Result<i32, String>>) -> Result<i32, String> {
    items.into_iter()
        .try_fold(0, |acc, r| r.map(|v| acc + v))
}
