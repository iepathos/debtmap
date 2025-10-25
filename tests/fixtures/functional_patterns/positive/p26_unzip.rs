fn separate_pairs(items: Vec<(i32, i32)>) -> (Vec<i32>, Vec<i32>) {
    items.into_iter().unzip()
}
