fn pairwise_sums(items: &[i32]) -> Vec<i32> {
    items.windows(2)
        .map(|w| w[0] + w[1])
        .collect()
}
