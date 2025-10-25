fn clone_and_process(items: &[i32]) -> Vec<i32> {
    items.iter()
        .cloned()
        .filter(|&x| x > 0)
        .collect()
}
