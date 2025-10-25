fn process_with_logging(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .inspect(|x| println!("Processing: {}", x))
        .filter(|&x| x > 0)
        .collect()
}
