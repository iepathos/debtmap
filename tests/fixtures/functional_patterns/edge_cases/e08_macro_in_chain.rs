fn with_macro(items: Vec<i32>) -> Vec<i32> {
    items.into_iter()
        .inspect(|x| println!("Value: {}", x))
        .collect()
}
