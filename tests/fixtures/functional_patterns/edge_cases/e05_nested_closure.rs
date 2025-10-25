fn nested_map(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .map(|x| {
            let y = x * 2;
            let z = y + 1;
            z
        })
        .collect()
}
