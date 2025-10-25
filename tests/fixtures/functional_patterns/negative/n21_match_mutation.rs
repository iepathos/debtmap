fn categorize_imperative(items: Vec<i32>) -> (Vec<i32>, Vec<i32>, Vec<i32>) {
    let mut neg = Vec::new();
    let mut zero = Vec::new();
    let mut pos = Vec::new();

    for item in items {
        match item {
            x if x < 0 => neg.push(x),
            0 => zero.push(0),
            x => pos.push(x),
        }
    }
    (neg, zero, pos)
}
