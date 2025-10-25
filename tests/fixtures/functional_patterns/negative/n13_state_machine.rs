fn state_based_process(items: Vec<i32>) -> Vec<i32> {
    let mut state = 0;
    let mut result = Vec::new();
    for item in items {
        state += item;
        if state > 100 {
            result.push(state);
            state = 0;
        }
    }
    result
}
