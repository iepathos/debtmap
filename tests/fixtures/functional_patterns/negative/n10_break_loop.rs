fn process_until_condition(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item < 0 {
            break;
        }
        result.push(item * 2);
    }
    result
}
