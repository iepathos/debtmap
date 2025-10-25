fn split_by_zero(items: &[i32]) -> Vec<Vec<i32>> {
    items.split(|&x| x == 0)
        .map(|slice| slice.to_vec())
        .collect()
}
