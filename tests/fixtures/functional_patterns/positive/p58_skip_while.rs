fn skip_leading_zeros(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .skip_while(|&x| x == 0)
        .collect()
}
