fn generate_sequence(count: usize) -> Vec<i32> {
    std::iter::repeat_with(|| rand::random::<i32>())
        .take(count)
        .collect()
}
