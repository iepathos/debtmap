fn split_even_odd(items: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    items.into_iter()
        .partition(|&x| x % 2 == 0)
}
