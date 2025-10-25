fn find_position(items: Vec<i32>, target: i32) -> Option<usize> {
    items.iter()
        .position(|&x| x == target)
}
