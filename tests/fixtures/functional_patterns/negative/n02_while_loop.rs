fn while_loop_process(mut items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    while !items.is_empty() {
        if let Some(item) = items.pop() {
            result.push(item * 2);
        }
    }
    result
}
