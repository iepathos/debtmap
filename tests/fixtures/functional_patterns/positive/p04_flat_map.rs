fn flatten_and_double(nested: Vec<Vec<i32>>) -> Vec<i32> {
    nested.into_iter()
        .flat_map(|v| v)
        .map(|x| x * 2)
        .collect()
}
