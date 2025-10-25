fn filter_bytes(data: &[u8]) -> Vec<u8> {
    data.iter()
        .copied()
        .filter(|&b| b > 32)
        .collect()
}
