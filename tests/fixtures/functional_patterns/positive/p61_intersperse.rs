fn join_with_separator(items: Vec<i32>, sep: i32) -> Vec<i32> {
    items.into_iter()
        .flat_map(|x| vec![x, sep])
        .collect()
}
