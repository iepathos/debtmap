fn take_until_negative(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .take_while(|&x| x >= 0)
        .collect()
}
