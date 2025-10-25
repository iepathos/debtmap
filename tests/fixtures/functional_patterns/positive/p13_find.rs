fn find_first_even(items: Vec<i32>) -> Option<i32> {
    items.into_iter()
        .find(|&x| x % 2 == 0)
}
