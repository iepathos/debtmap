fn process_pipeline(items: Vec<i32>) -> i32 {
    items.into_iter()
        .filter(|&x| x > 0)
        .map(|x| x * 2)
        .filter(|&x| x < 100)
        .fold(0, |acc, x| acc + x)
}
