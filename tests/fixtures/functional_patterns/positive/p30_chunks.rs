fn batch_process(items: &[i32], batch_size: usize) -> Vec<i32> {
    items.chunks(batch_size)
        .map(|chunk| chunk.iter().sum())
        .collect()
}
