fn process_triplets(items: &[i32]) -> Vec<i32> {
    items.chunks(3)
        .map(|chunk| chunk.iter().sum())
        .collect()
}
