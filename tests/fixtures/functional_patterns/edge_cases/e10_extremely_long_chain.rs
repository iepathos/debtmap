fn very_long_pipeline(items: Vec<i32>) -> i32 {
    items.into_iter()
        .filter(|&x| x > 0)
        .map(|x| x * 2)
        .filter(|&x| x < 100)
        .map(|x| x + 1)
        .filter(|&x| x % 2 == 0)
        .map(|x| x / 2)
        .filter(|&x| x > 5)
        .map(|x| x - 1)
        .sum()
}
