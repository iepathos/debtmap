fn group_consecutive(items: Vec<i32>) -> Vec<Vec<i32>> {
    let mut result = Vec::new();
    let mut iter = items.into_iter().peekable();

    while iter.peek().is_some() {
        let current = iter.next().unwrap();
        let group: Vec<i32> = std::iter::once(current)
            .chain(iter.peeking_take_while(|&x| x == current))
            .collect();
        result.push(group);
    }
    result
}
