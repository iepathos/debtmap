// Map and filter pipeline
fn process_numbers(items: Vec<i32>) -> Vec<i32> {
    items
        .iter()
        .filter(|&x| *x > 0)
        .map(|x| x * 2)
        .collect()
}
