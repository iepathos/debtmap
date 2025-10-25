fn process_empty() -> Vec<i32> {
    vec![].into_iter()
        .filter(|&x: &i32| x > 0)
        .collect()
}
