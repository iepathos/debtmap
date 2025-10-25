fn consume_partially(items: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    let mut iter = items.into_iter();
    let first_five: Vec<i32> = iter.by_ref().take(5).collect();
    let rest: Vec<i32> = iter.collect();
    (first_five, rest)
}
