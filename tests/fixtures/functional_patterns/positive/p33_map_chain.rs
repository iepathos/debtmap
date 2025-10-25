fn transform_chain(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .map(|x| x * 2)
        .map(|x| x + 1)
        .map(|x| x / 2)
        .collect()
}
