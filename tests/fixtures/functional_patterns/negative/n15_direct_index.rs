fn reverse_imperative(mut items: Vec<i32>) -> Vec<i32> {
    let len = items.len();
    for i in 0..len/2 {
        let temp = items[i];
        items[i] = items[len - 1 - i];
        items[len - 1 - i] = temp;
    }
    items
}
