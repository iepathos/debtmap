fn running_sum(items: Vec<i32>) -> Vec<i32> {
    items.iter()
        .scan(0, |state, x| {
            *state += x;
            Some(*state)
        })
        .collect()
}
