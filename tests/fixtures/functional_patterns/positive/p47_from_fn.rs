fn fibonacci(n: usize) -> Vec<i32> {
    let mut a = 0;
    let mut b = 1;
    std::iter::from_fn(move || {
        let result = a;
        let next = a + b;
        a = b;
        b = next;
        Some(result)
    })
    .take(n)
    .collect()
}
