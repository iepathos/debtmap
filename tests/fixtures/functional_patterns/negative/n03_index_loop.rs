fn index_based_process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for i in 0..items.len() {
        if items[i] > 0 {
            result.push(items[i] * 2);
        }
    }
    result
}
