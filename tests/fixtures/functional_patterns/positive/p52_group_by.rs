fn group_by_parity(items: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    items.into_iter()
        .fold((Vec::new(), Vec::new()), |(mut evens, mut odds), x| {
            if x % 2 == 0 {
                evens.push(x);
            } else {
                odds.push(x);
            }
            (evens, odds)
        })
}
