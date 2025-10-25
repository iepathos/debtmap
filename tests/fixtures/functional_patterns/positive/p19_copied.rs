fn copy_and_transform(items: &[i32]) -> Vec<i32> {
    items.iter()
        .copied()
        .map(|x| x * 2)
        .collect()
}
