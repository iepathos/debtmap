fn remove_duplicates(mut items: Vec<i32>) -> Vec<i32> {
    items.sort();
    items.into_iter()
        .collect::<Vec<_>>()
        .into_iter()
        .collect()
}
